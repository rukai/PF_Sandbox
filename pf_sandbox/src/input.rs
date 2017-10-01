use libusb::{Context, Device, DeviceHandle, Error};
use std::ops::Index;
use std::time::Duration;
use std::f32;

use treeflection::{Node, NodeRunner, NodeToken};

enum InputSource<'a> {
    GCAdapter { handle: DeviceHandle<'a>, deadzones: [Deadzone; 4] },
    #[allow(dead_code)]
    GenericController { handle: usize, deadzone: Deadzone }
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
    input_sources:  Vec<InputSource<'a>>,
    current_inputs: Vec<ControllerInput>,      // inputs for this frame
    game_inputs:    Vec<Vec<ControllerInput>>, // game past and (potentially) future inputs, frame 0 has index 2
    prev_start:     bool,
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

        Input {
            input_sources,
            game_inputs:     vec!(),
            current_inputs:  vec!(),
            prev_start:      false,
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
    pub fn update(&mut self, tas_inputs: &[ControllerInput], ai_inputs: &[ControllerInput], reset_deadzones: bool) {
        // clear deadzones so they will be set at next read
        if reset_deadzones {
            for source in &mut self.input_sources {
                match source {
                    &mut InputSource::GCAdapter         { ref mut deadzones, .. } => { *deadzones = Deadzone::empty4() }
                    &mut InputSource::GenericController { ref mut deadzone,  .. } => { *deadzone  = Deadzone::empty() }
                }
            }
        }

        // read input from controllers
        let mut inputs: Vec<ControllerInput> = Vec::new();
        for source in &mut self.input_sources {
            match source {
                &mut InputSource::GCAdapter { ref mut handle, ref mut deadzones } => read_gc_adapter(handle, deadzones, &mut inputs),
                &mut InputSource::GenericController { .. }                        => unimplemented!()
            }
        }

        // append AI inputs
        inputs.extend_from_slice(ai_inputs);

        // replace tas inputs
        for i in 0..tas_inputs.len().min(inputs.len()) {
            inputs[i] = tas_inputs[i].clone();
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

    /// Return game inputs at current index into history
    pub fn players(&self, frame: usize) -> Vec<PlayerInput> {
        let mut result_inputs: Vec<PlayerInput> = vec!();

        for (i, _) in self.current_inputs.iter().enumerate() {
            let inputs = self.get_player_inputs(i, frame as i64);
            if inputs[0].plugged_in {
                result_inputs.push(PlayerInput {
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
                });
            }
            else {
                result_inputs.push(PlayerInput::empty());
            }
        }
        result_inputs
    }

    fn get_player_inputs(&self, player: usize, frame: i64) -> Vec<ControllerInput> {
        let mut result: Vec<ControllerInput> = vec!();

        for i in (frame-8..frame).rev() {
            result.push(
                if i < 0 {
                    ControllerInput::empty()
                }
                else {
                    match self.game_inputs[i as usize].get(player) {
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
    handle.read_interrupt(0x81, &mut data, Duration::new(1, 0)).unwrap();

    for port in 0..4 {
        let plugged_in    = data[9*port+1] == 20 || data[9*port+1] == 16;
        let raw_stick_x   = data[9*port+4];
        let raw_stick_y   = data[9*port+5];
        let raw_c_stick_x = data[9*port+6];
        let raw_c_stick_y = data[9*port+7];
        let raw_l_trigger = data[9*port+8];
        let raw_r_trigger = data[9*port+9];

        if plugged_in && !deadzones[port].plugged_in // Only reset deadzone if a controller was just plugged in
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

        inputs.push(ControllerInput {
            plugged_in,
            up   : data[9*port+2] & 0b10000000 != 0,
            down : data[9*port+2] & 0b01000000 != 0,
            right: data[9*port+2] & 0b00100000 != 0,
            left : data[9*port+2] & 0b00010000 != 0,
            y    : data[9*port+2] & 0b00001000 != 0,
            x    : data[9*port+2] & 0b00000100 != 0,
            b    : data[9*port+2] & 0b00000010 != 0,
            a    : data[9*port+2] & 0b00000001 != 0,
            l    : data[9*port+3] & 0b00001000 != 0,
            r    : data[9*port+3] & 0b00000100 != 0,
            z    : data[9*port+3] & 0b00000010 != 0,
            start: data[9*port+3] & 0b00000001 != 0,

            l_trigger: trigger_filter(raw_l_trigger.saturating_sub(deadzone.l_trigger)),
            r_trigger: trigger_filter(raw_r_trigger.saturating_sub(deadzone.r_trigger)),

            stick_x:   stick_x,
            stick_y:   stick_y,
            c_stick_x: c_stick_x,
            c_stick_y: c_stick_y,
        });
    }
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
