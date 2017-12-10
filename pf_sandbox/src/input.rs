use std::ops::Index;
use std::time::Duration;
use std::f32;

use gilrs::{Gilrs, Gamepad};
use gilrs;
use libusb::{Context, Device, DeviceHandle, Error};
use treeflection::{Node, NodeRunner, NodeToken};

use network::{Netplay, NetplayState};

enum InputSource<'a> {
    GCAdapter { handle: DeviceHandle<'a>, deadzones: [Deadzone; 4] },
    GenericController { index: usize, deadzone: Deadzone }
}

/// Stores the first value returned from an input source
pub struct Deadzone {
    plugged_in: bool,
    stick_x:    u8,
    stick_y:    u8,
    c_stick_x:  u8,
    c_stick_y:  u8,
    l_trigger:  u8,
    r_trigger:  u8,
}

impl Deadzone {
    pub fn empty() -> Self {
        Deadzone {
            plugged_in: false,
            stick_x:    0,
            stick_y:    0,
            c_stick_x:  0,
            c_stick_y:  0,
            l_trigger:  0,
            r_trigger:  0,
        }
    }

    pub fn empty4() -> [Self; 4] {
        [
            Deadzone::empty(),
            Deadzone::empty(),
            Deadzone::empty(),
            Deadzone::empty()
        ]
    }
}

pub struct Input<'a> {
    // game past and (potentially) future inputs, frame 0 has index 2
    // structure: frames Vec<controllers Vec<ControllerInput>>
    game_inputs:    Vec<Vec<ControllerInput>>,
    current_inputs: Vec<ControllerInput>, // inputs for this frame
    prev_start:     bool,
    input_sources:  Vec<InputSource<'a>>,
    gilrs:          Gilrs,
}

// In/Out is from perspective of computer
// Out means: computer->adapter
// In means:  adapter->computer

impl<'a> Input<'a> {
    pub fn new(context: &'a mut  Context) -> Input<'a> {
        let mut adapter_handles: Vec<DeviceHandle> = Vec::new();
        let devices = context.devices();
        for device in devices.unwrap().iter() {
            if let Ok(device_desc) = device.device_descriptor() {
                if device_desc.vendor_id() == 0x057E && device_desc.product_id() == 0x0337 {
                    match device.open() {
                        Ok(mut handle) => {
                            if let Ok(true) = handle.kernel_driver_active(0) {
                                handle.detach_kernel_driver(0).unwrap();
                            }
                            match handle.claim_interface(0) {
                                Ok(_) => {
                                    // Tell adapter to start reading
                                    let payload = [0x13];
                                    if let Ok(_) = handle.write_interrupt(0x2, &payload, Duration::new(1, 0)) {
                                        adapter_handles.push(handle);
                                        println!("GC adapter: Setup complete");
                                    }
                                },
                                Err(e) => { println!("GC adapter: Failed to claim interface: {}", e) }
                            }
                        }
                        Err(e) => {
                            Input::handle_open_error(e);
                        }
                    }
                }
            }
        }

        let mut input_sources = vec!();
        for handle in adapter_handles {
            input_sources.push(InputSource::GCAdapter { handle, deadzones: Deadzone::empty4() });
        }

        let gilrs = Gilrs::new();

        Input {
            game_inputs:    vec!(),
            current_inputs: vec!(),
            prev_start:     false,
            input_sources,
            gilrs,
        }
    }

    fn handle_open_error(e: Error) {
        let access_solution = if cfg!(target_os = "linux") { r#":
    You need to set a udev rule so that the adapter can be accessed.
    To fix this on most Linux distributions, run the following command and then restart your computer.
    echo 'SUBSYSTEM=="usb", ENV{DEVTYPE}=="usb_device", ATTRS{idVendor}=="057e", ATTRS{idProduct}=="0337", TAG+="uaccess"' | sudo tee /etc/udev/rules.d/51-gcadapter.rules"#
        } else { "" };

        let driver_solution = if cfg!(target_os = "windows") { r#":
    To use your GC adapter you must:
    1. Download and run Zadig: http://zadig.akeo.ie/
    2. Options -> List all devices
    3. In the pulldown menu, Select WUP-028
    4. On the right ensure WinUSB is selected
    5. Select Replace Driver
    6. Select yes in the dialog box
    7. Restart PF Sandbox"#
        } else { "" };

        match e {
            Error::Access => {
                println!("GC adapter: Permissions error{}", access_solution);
            }
            Error::NotSupported => {
                println!("GC adapter: Not supported error{}", driver_solution);
            }
            _ => { println!("GC adapter: Failed to open handle: {:?}", e); }
        }
    }

    /// Call this once every frame
    pub fn step(&mut self, tas_inputs: &[ControllerInput], ai_inputs: &[ControllerInput], netplay: &mut Netplay, reset_deadzones: bool) {
        // clear deadzones so they will be set at next read
        if reset_deadzones {
            for source in &mut self.input_sources {
                match source {
                    &mut InputSource::GCAdapter         { ref mut deadzones, .. } => { *deadzones = Deadzone::empty4() }
                    &mut InputSource::GenericController { ref mut deadzone,  .. } => { *deadzone  = Deadzone::empty() }
                }
            }
        }

        while let Some(ev) = self.gilrs.next_event() {
            self.gilrs.update(&ev);
        }

        // find new generic controllers
        for (index, _) in self.gilrs.gamepads() {
            let mut exists = false;
            for source in &self.input_sources {
                if let &InputSource::GenericController { index: check_index, .. } = source {
                    if index == check_index {
                        exists = true;
                    }
                }
            }

            // Force users to use native GC->Wii U input
            if !exists && self.gilrs.gamepad(index).name() != "mayflash limited MAYFLASH GameCube Controller Adapter" {
                self.input_sources.push(InputSource::GenericController { index, deadzone: Deadzone::empty() });
            }
        }

        // read input from controllers
        let mut inputs: Vec<ControllerInput> = Vec::new();
        for source in &mut self.input_sources {
            match source {
                &mut InputSource::GCAdapter { ref mut handle, ref mut deadzones }
                    => read_gc_adapter(handle, deadzones, &mut inputs),

                &mut InputSource::GenericController { index, ref mut deadzone }
                    => inputs.push(read_generic(self.gilrs.gamepad(index), deadzone)),
            }
        }

        if netplay.skip_frame() {
            // TODO: combine the skipped frames input with the next frame:
            // * average float values
            // * detect dropped presses and include the press
        }
        else {
            netplay.send_controller_inputs(inputs.clone());
        }

        // append AI inputs
        inputs.extend_from_slice(ai_inputs);

        if let NetplayState::Offline = netplay.state() {
            // replace tas inputs
            for i in 0..tas_inputs.len().min(inputs.len()) {
                inputs[i] = tas_inputs[i].clone();
            }
        }

        self.prev_start     = self.current_inputs.iter().any(|x| x.start);
        self.current_inputs = inputs;
    }

    /// Reset the game input history
    pub fn reset_history(&mut self) {
        self.game_inputs.clear();
        self.prev_start = false;
    }

    /// Set the game input history
    pub fn set_history(&mut self, history: Vec<Vec<ControllerInput>>) {
        self.game_inputs = history;
    }
    
    /// Get the game input history
    pub fn get_history(&self) -> Vec<Vec<ControllerInput>> {
        self.game_inputs.clone()
    }

    /// Call this once from the game update logic only
    /// Throws out all future history that may exist
    pub fn game_update(&mut self, frame: usize) {
        for _ in frame..(self.game_inputs.len()+1) {
            self.game_inputs.pop();
        }

        self.game_inputs.push(self.current_inputs.clone());
    }

    /// Call this once from netplay game/menu update logic only (instead of game_update)
    pub fn netplay_update(&mut self) {
        self.game_inputs.push(self.current_inputs.clone());
    }

    /// Return game inputs at specified index into history
    pub fn players(&self, frame: usize, netplay: &Netplay) -> Vec<PlayerInput> {
        let mut result_inputs: Vec<PlayerInput> = vec!();

        let local_index = netplay.local_index();
        let mut peer_offset = 0;
        let peers_inputs = &netplay.confirmed_inputs;
        for i in 0..netplay.number_of_peers() {
            if i == local_index {
                peer_offset = 1;

                for i in 0..self.current_inputs.len() {
                    let inputs = self.get_8frames_of_input(&self.game_inputs, i, frame as i64);
                    result_inputs.push(Input::controller_inputs_to_player_input(inputs));
                }
            }
            else {
                let peer_inputs = &peers_inputs[i - peer_offset];
                let num_inputs = peer_inputs.last().map_or(0, |x| x.len());
                for i in 0..num_inputs {
                    let inputs = self.get_8frames_of_input(&peer_inputs[..], i, netplay.frame() as i64);
                    result_inputs.push(Input::controller_inputs_to_player_input(inputs));
                }
            }
        }

        // netplay input logging
        println!("");
        println!("netplay.number_of_peers: {}", netplay.number_of_peers());
        println!("netplay.local_index: {}",  netplay.local_index());
        println!("netplay.frame: {}", netplay.frame());
        println!("netplay.frames_to_step: {}", netplay.frames_to_step());
        println!("netplay.skip_frame : {}", netplay.skip_frame());
        println!("game/menu.current_frame: {}", frame);
        for (i, input) in result_inputs.iter().enumerate() {
            println!("#{} a: {} b: {} input.stick_x: {} input.stick_y: {}", i, input.a.value, input.b.value, input.stick_x.value, input.stick_y.value);
        }

        result_inputs
    }

    fn controller_inputs_to_player_input(inputs: Vec<ControllerInput>) -> PlayerInput {
        if inputs[0].plugged_in {
            PlayerInput {
                plugged_in: true,

                up:    Button { value: inputs[0].up,    press: inputs[0].up    && !inputs[1].up },
                down:  Button { value: inputs[0].down,  press: inputs[0].down  && !inputs[1].down },
                right: Button { value: inputs[0].right, press: inputs[0].right && !inputs[1].right },
                left:  Button { value: inputs[0].left,  press: inputs[0].left  && !inputs[1].left },
                y:     Button { value: inputs[0].y,     press: inputs[0].y     && !inputs[1].y },
                x:     Button { value: inputs[0].x,     press: inputs[0].x     && !inputs[1].x },
                b:     Button { value: inputs[0].b,     press: inputs[0].b     && !inputs[1].b },
                a:     Button { value: inputs[0].a,     press: inputs[0].a     && !inputs[1].a },
                l:     Button { value: inputs[0].l,     press: inputs[0].l     && !inputs[1].l },
                r:     Button { value: inputs[0].r,     press: inputs[0].r     && !inputs[1].r },
                z:     Button { value: inputs[0].z,     press: inputs[0].z     && !inputs[1].z },
                start: Button { value: inputs[0].start, press: inputs[0].start && !inputs[1].start },

                stick_x:   Stick { value: inputs[0].stick_x,   diff: inputs[0].stick_x   - inputs[1].stick_x },
                stick_y:   Stick { value: inputs[0].stick_y,   diff: inputs[0].stick_y   - inputs[1].stick_y },
                c_stick_x: Stick { value: inputs[0].c_stick_x, diff: inputs[0].c_stick_x - inputs[1].c_stick_x },
                c_stick_y: Stick { value: inputs[0].c_stick_y, diff: inputs[0].c_stick_y - inputs[1].c_stick_y },

                l_trigger:  Trigger { value: inputs[0].l_trigger, diff: inputs[0].l_trigger - inputs[1].l_trigger },
                r_trigger:  Trigger { value: inputs[0].r_trigger, diff: inputs[0].r_trigger - inputs[1].r_trigger },
                history: inputs,
            }
        }
        else {
            PlayerInput::empty()
        }
    }

    /// converts frames Vec<controllers Vec<ControllerInput>> into frames Vec<ControllerInput> for the specified controller_i
    /// Output must be 8 frames long, any missing frames due to either netplay lag or the game just starting are filled in
    fn get_8frames_of_input(&self, game_inputs: &[Vec<ControllerInput>], controller_i: usize, frame: i64) -> Vec<ControllerInput> {
        let mut result: Vec<ControllerInput> = vec!();
        let empty_vec = vec!();

        for frame_i in (frame-8..frame).rev() {
            result.push(
                if frame_i < 0 {
                    ControllerInput::empty()
                }
                else {
                    let controllers = match game_inputs.get(frame_i as usize) {
                        Some(controllers) => controllers,
                        None              => game_inputs.last().unwrap_or(&empty_vec)
                    };
                    match controllers.get(controller_i) {
                        Some(value) => value.clone(),
                        None        => ControllerInput::empty()
                    }
                }
            );
        }

        assert!(result.len() == 8, "get_player_inputs needs to return a vector of size 8 but it was {}", result.len());
        result
    }

    /// Returns the index to the last frame in history
    pub fn last_frame(&self) -> usize {
        self.game_inputs.len() - 1
    }

    /// The player input history system cannot be used when the game is paused (or it would create bogus entries into the history)
    /// Instead we need to create custom functions for handling input when paused.

    /// Check for start button press
    pub fn start_pressed(&mut self) -> bool {
        !self.prev_start && self.current_inputs.iter().any(|x| x.start)
    }

    /// button combination for quiting the game
    pub fn game_quit_held(&mut self) -> bool {
        self.current_inputs.iter().any(|x| x.a && x.l && x.r && x.start) && self.start_pressed()
    }
}

#[allow(dead_code)]
fn display_endpoints(device: &mut Device) {
    for interface in device.config_descriptor(0).unwrap().interfaces() {
        println!("interface: {}", interface.number());
        for setting in interface.descriptors() {
            for endpoint in setting.endpoint_descriptors() {
                println!("    endpoint - number: {}, address: {}", endpoint.number(), endpoint.address());
            }
        }
    }
}

impl ControllerInput {
    fn empty() -> ControllerInput {
        ControllerInput {
            plugged_in: false,

            up:    false,
            down:  false,
            right: false,
            left:  false,
            y:     false,
            x:     false,
            b:     false,
            a:     false,
            l:     false,
            r:     false,
            z:     false,
            start: false,

            stick_x:   0.0,
            stick_y:   0.0,
            c_stick_x: 0.0,
            c_stick_y: 0.0,
            l_trigger: 0.0,
            r_trigger: 0.0,
        }
    }

    pub fn stick_angle(&self) -> Option<f32> {
        if self.stick_x == 0.0 && self.stick_y == 0.0 {
            None
        } else {
            Some(self.stick_y.atan2(self.stick_x))
        }
    }

    #[allow(dead_code)]
    pub fn c_stick_angle(&self) -> Option<f32> {
        if self.stick_x == 0.0 && self.stick_y == 0.0 {
            None
        } else {
            Some(self.c_stick_y.atan2(self.c_stick_x))
        }
    }
}

impl PlayerInput {
    pub fn empty() -> PlayerInput {
        PlayerInput {
            plugged_in: false,

            up:    Button { value: false, press: false },
            down:  Button { value: false, press: false },
            right: Button { value: false, press: false },
            left:  Button { value: false, press: false },
            y:     Button { value: false, press: false },
            x:     Button { value: false, press: false },
            b:     Button { value: false, press: false },
            a:     Button { value: false, press: false },
            l:     Button { value: false, press: false },
            r:     Button { value: false, press: false },
            z:     Button { value: false, press: false },
            start: Button { value: false, press: false },

            stick_x:   Stick { value: 0.0, diff: 0.0 },
            stick_y:   Stick { value: 0.0, diff: 0.0 },
            c_stick_x: Stick { value: 0.0, diff: 0.0 },
            c_stick_y: Stick { value: 0.0, diff: 0.0 },

            l_trigger:  Trigger { value: 0.0, diff: 0.0 },
            r_trigger:  Trigger { value: 0.0, diff: 0.0 },
            history: vec!(ControllerInput::empty(); 8),
        }
    }
}

/// Add 4 GC adapter controllers to inputs
fn read_gc_adapter(handle: &mut DeviceHandle, deadzones: &mut [Deadzone], inputs: &mut Vec<ControllerInput>) {
    let mut data: [u8; 37] = [0; 37];
    if let Ok(_) = handle.read_interrupt(0x81, &mut data, Duration::new(1, 0)) {
        for port in 0..4 {
            let plugged_in    = data[9*port+1] == 20 || data[9*port+1] == 16;
            let raw_stick_x   = data[9*port+4];
            let raw_stick_y   = data[9*port+5];
            let raw_c_stick_x = data[9*port+6];
            let raw_c_stick_y = data[9*port+7];
            let raw_l_trigger = data[9*port+8];
            let raw_r_trigger = data[9*port+9];

            if plugged_in && !deadzones[port].plugged_in // Only reset deadzone if controller was just plugged in
                && raw_stick_x != 0 // first response seems to give garbage data
            {
                deadzones[port] = Deadzone {
                    plugged_in: true,
                    stick_x:    raw_stick_x,
                    stick_y:    raw_stick_y,
                    c_stick_x:  raw_c_stick_x,
                    c_stick_y:  raw_c_stick_y,
                    l_trigger:  raw_l_trigger,
                    r_trigger:  raw_r_trigger,
                };
            }
            if !plugged_in {
                deadzones[port] = Deadzone::empty();
            }

            let deadzone = &deadzones[port];
            let (stick_x, stick_y)     = stick_filter(stick_deadzone(raw_stick_x,   deadzone.stick_x),   stick_deadzone(raw_stick_y,   deadzone.stick_y));
            let (c_stick_x, c_stick_y) = stick_filter(stick_deadzone(raw_c_stick_x, deadzone.c_stick_x), stick_deadzone(raw_c_stick_y, deadzone.c_stick_y));
            let l_trigger = trigger_filter(raw_l_trigger.saturating_sub(deadzone.l_trigger));
            let r_trigger = trigger_filter(raw_r_trigger.saturating_sub(deadzone.r_trigger));

            inputs.push(ControllerInput {
                up:    data[9*port+2] & 0b10000000 != 0,
                down:  data[9*port+2] & 0b01000000 != 0,
                right: data[9*port+2] & 0b00100000 != 0,
                left:  data[9*port+2] & 0b00010000 != 0,
                y:     data[9*port+2] & 0b00001000 != 0,
                x:     data[9*port+2] & 0b00000100 != 0,
                b:     data[9*port+2] & 0b00000010 != 0,
                a:     data[9*port+2] & 0b00000001 != 0,
                l:     data[9*port+3] & 0b00001000 != 0,
                r:     data[9*port+3] & 0b00000100 != 0,
                z:     data[9*port+3] & 0b00000010 != 0,
                start: data[9*port+3] & 0b00000001 != 0,
                stick_x,
                stick_y,
                c_stick_x,
                c_stick_y,
                l_trigger,
                r_trigger,
                plugged_in,
            });
        }
    }
    else {
        inputs.push(ControllerInput::empty());
        inputs.push(ControllerInput::empty());
        inputs.push(ControllerInput::empty());
        inputs.push(ControllerInput::empty());
    }
}

/// Add a single controller to inputs, reading from the passed gamepad
fn read_generic(gamepad: &Gamepad, deadzone: &mut Deadzone) -> ControllerInput {
    let plugged_in = gamepad.is_connected();
    let raw_stick_x   = generic_to_byte(gamepad.value(gilrs::Axis::LeftStickX));
    let raw_stick_y   = generic_to_byte(gamepad.value(gilrs::Axis::LeftStickY));
    let raw_c_stick_x = generic_to_byte(gamepad.value(gilrs::Axis::RightStickX));
    let raw_c_stick_y = generic_to_byte(gamepad.value(gilrs::Axis::RightStickY));

    if plugged_in && !deadzone.plugged_in { // Only reset deadzone if controller was just plugged in
        *deadzone = Deadzone {
            plugged_in: true,
            stick_x:    raw_stick_x,
            stick_y:    raw_stick_y,
            c_stick_x:  raw_c_stick_x,
            c_stick_y:  raw_c_stick_y,
            l_trigger:  0, // gilrs doesnt give sensible values until the trigger has been used so just use a dummy value here
            r_trigger:  0,
        };
    }
    if !plugged_in {
        *deadzone = Deadzone::empty();
    }

    let (stick_x, stick_y)     = stick_filter(stick_deadzone(raw_stick_x,   deadzone.stick_x),   stick_deadzone(raw_stick_y,   deadzone.stick_y));
    let (c_stick_x, c_stick_y) = stick_filter(stick_deadzone(raw_c_stick_x, deadzone.c_stick_x), stick_deadzone(raw_c_stick_y, deadzone.c_stick_y));
    let l_trigger = generic_trigger_bounds(gamepad.value(gilrs::Axis::LeftTrigger2));
    let r_trigger = generic_trigger_bounds(gamepad.value(gilrs::Axis::RightTrigger2));

    ControllerInput {
        up:    gamepad.is_pressed(gilrs::Button::DPadUp),
        down:  gamepad.is_pressed(gilrs::Button::DPadDown),
        right: gamepad.is_pressed(gilrs::Button::DPadLeft),
        left:  gamepad.is_pressed(gilrs::Button::DPadRight),
        y:     gamepad.is_pressed(gilrs::Button::North),
        x:     gamepad.is_pressed(gilrs::Button::East),
        b:     gamepad.is_pressed(gilrs::Button::West),
        a:     gamepad.is_pressed(gilrs::Button::South),
        z:     gamepad.is_pressed(gilrs::Button::LeftTrigger) || gamepad.is_pressed(gilrs::Button::RightTrigger),
        start: gamepad.is_pressed(gilrs::Button::Start),
        l:     l_trigger > 0.9,
        r:     r_trigger > 0.9,
        stick_x,
        stick_y,
        c_stick_x,
        c_stick_y,
        l_trigger,
        r_trigger,
        plugged_in,
    }
}

fn generic_to_byte(value: f32) -> u8 {
    (value.min(1.0).max(-1.0) * 127.0 + 127.0) as u8
}

fn generic_trigger_bounds(value: f32) -> f32 {
    (value.min(1.0).max(-1.0) + 1.0) / 2.0
}

/// use the first received stick value to reposition the current stick value around 128
pub fn stick_deadzone(current: u8, first: u8) -> u8 {
    if current > first {
        128u8.saturating_add(current - first)
    } else {
        128u8.saturating_sub(first - current)
    }
}

fn abs_min(a: f32, b: f32) -> f32 {
    if (a >= 0.0 && a > b) || (a <= 0.0 && a < b) {
        b
    } else {
        a
    }
}

fn stick_filter(in_stick_x: u8, in_stick_y: u8) -> (f32, f32) {
    let raw_stick_x = in_stick_x as f32 - 128.0;
    let raw_stick_y = in_stick_y as f32 - 128.0;
    let angle = (raw_stick_y).atan2(raw_stick_x);

    let max_x = (angle.cos() * 80.0).trunc();
    let max_y = (angle.sin() * 80.0).trunc();
    let stick_x = if in_stick_x == 128 { // avoid raw_stick_x = 0 and thus division by zero in the atan2)
        0.0
    } else {
        abs_min(raw_stick_x, max_x) / 80.0
    };
    let stick_y = abs_min(raw_stick_y, max_y) / 80.0;

    let deadzone = 0.28;
    (
        if stick_x.abs() < deadzone { 0.0 } else { stick_x },
        if stick_y.abs() < deadzone { 0.0 } else { stick_y }
    )
}

fn trigger_filter(trigger: u8) -> f32 {
    let value = (trigger as f32) / 140.0;
    if value > 1.0 {
        1.0
    } else {
        value
    }
}

/// Internal input storage
#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct ControllerInput {
    pub plugged_in: bool,

    pub a:     bool,
    pub b:     bool,
    pub x:     bool,
    pub y:     bool,
    pub left:  bool,
    pub right: bool,
    pub down:  bool,
    pub up:    bool,
    pub start: bool,
    pub z:     bool,
    pub r:     bool,
    pub l:     bool,

    pub stick_x:   f32,
    pub stick_y:   f32,
    pub c_stick_x: f32,
    pub c_stick_y: f32,
    pub r_trigger: f32,
    pub l_trigger: f32,
}

/// External data access
pub struct PlayerInput {
    pub plugged_in: bool,

    pub a:     Button,
    pub b:     Button,
    pub x:     Button,
    pub y:     Button,
    pub left:  Button,
    pub right: Button,
    pub down:  Button,
    pub up:    Button,
    pub start: Button,
    pub z:     Button,
    pub r:     Button,
    pub l:     Button,

    pub stick_x:   Stick,
    pub stick_y:   Stick,
    pub c_stick_x: Stick,
    pub c_stick_y: Stick,
    pub r_trigger:  Trigger,
    pub l_trigger:  Trigger,
    history: Vec<ControllerInput>, // guaranteed to contain 8 elements
}

impl Index<usize> for PlayerInput {
    type Output = ControllerInput;

    fn index(&self, index: usize) -> &ControllerInput {
        &self.history[index]
    }
}

// TODO: now we that we have history we could remove the value from these, turning them into primitive values

pub struct Button {
    pub value: bool, // on
    pub press: bool, // off->on this frame
}

pub struct Stick {
    pub value: f32, // current.value
    pub diff:  f32, // current.value - previous.value
}

pub struct Trigger {
    pub value: f32, // current.value
    pub diff:  f32, // current.value - previous.value
}
