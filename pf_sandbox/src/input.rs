/// The contents of this module should really be in pf_sandbox_lib::input
/// However that would mean adding libusb as a dep to pf_sandbox_lib
/// So I wont move this code into pf_sandbox_lib until libusb has no external dependencies to setup.

use pf_sandbox_lib::input as pf_sandbox_lib_input;
use pf_sandbox_lib::input::{
    maps::ControllerMaps,
    ControllerInput,
    Deadzone,
    PlayerInput,
    Button,
    Stick,
    Trigger
};

use std::time::Duration;

use gilrs_core::{Gilrs, Event};
use libusb::{Context, Device, DeviceHandle, Error};

use pf_sandbox_lib::network::{Netplay, NetplayState};

enum InputSource<'a> {
    GCAdapter { handle: DeviceHandle<'a>, deadzones: [Deadzone; 4] },
    GenericController { index: usize, state: ControllerInput, deadzone: Deadzone }
}

pub struct Input<'a> {
    // game past and (potentially) future inputs, frame 0 has index 2
    // structure: frames Vec<controllers Vec<ControllerInput>>
    game_inputs:     Vec<Vec<ControllerInput>>,
    current_inputs:  Vec<ControllerInput>, // inputs for this frame
    prev_start:      bool,
    input_sources:   Vec<InputSource<'a>>,
    gilrs:           Gilrs,
    controller_maps: ControllerMaps,
    pub events:      Vec<Event>,
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
                                }
                                Err(e) => println!("GC adapter: Failed to claim interface: {}", e)
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

        let gilrs = Gilrs::new().unwrap();

        let controller_maps = ControllerMaps::load();

        Input {
            game_inputs:    vec!(),
            current_inputs: vec!(),
            events:         vec!(),
            prev_start:     false,
            input_sources,
            gilrs,
            controller_maps,
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

        self.events.clear();
        while let Some(ev) = self.gilrs.next_event() {
            self.events.push(ev);
        }
        self.events.sort_by_key(|x| x.time);

        // find new generic controllers
        for index in 0..self.gilrs.last_gamepad_hint() {
            let gamepad = self.gilrs.gamepad(index).unwrap();
            if gamepad.is_connected() {
                let mut exists = false;
                for source in &self.input_sources {
                    if let &InputSource::GenericController { index: check_index, .. } = source {
                        if index == check_index {
                            exists = true;
                        }
                    }
                }

                // Force users to use native GC->Wii U input
                if !exists && gamepad.name() != "mayflash limited MAYFLASH GameCube Controller Adapter" {
                    let state = ControllerInput { plugged_in: true, .. ControllerInput::default() };
                    self.input_sources.push(InputSource::GenericController { index, state, deadzone: Deadzone::empty() });
                }
            }
        }

        // read input from controllers
        let mut inputs: Vec<ControllerInput> = Vec::new();
        for source in &mut self.input_sources {
            match source {
                &mut InputSource::GCAdapter { ref mut handle, ref mut deadzones }
                    => read_gc_adapter(handle, deadzones, &mut inputs),

                &mut InputSource::GenericController { index, ref mut state, ref mut deadzone } => {
                    let events = self.events.iter().filter(|x| x.id == index).map(|x| &x.event).cloned().collect();
                    let gamepad = &self.gilrs.gamepad(index).unwrap(); // Old gamepads stick around forever so its fine to unwrap.
                    let maps = &self.controller_maps.maps;
                    inputs.push(pf_sandbox_lib_input::read_generic(maps, state, events, gamepad, deadzone));
                }
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

        self.prev_start = self.current_inputs.iter().any(|x| x.start);
        self.current_inputs = inputs;

        debug!("step");
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
        for _ in frame..=self.game_inputs.len() {
            self.game_inputs.pop();
        }

        self.game_inputs.push(self.current_inputs.clone());
    }

    /// Call this once from netplay game/menu update logic only (instead of game_update)
    pub fn netplay_update(&mut self) {
        self.game_inputs.push(self.current_inputs.clone());
    }

    /// Return game inputs at specified index into history
    pub fn players_no_log(&self, frame: usize, netplay: &Netplay) -> Vec<PlayerInput> {
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
                let num_controllers = peer_inputs.last().map_or(0, |x| x.len());
                for i in 0..num_controllers {
                    let inputs = self.get_8frames_of_input(&peer_inputs[..], i, netplay.frame() as i64);
                    result_inputs.push(Input::controller_inputs_to_player_input(inputs));
                }
            }
        }

        result_inputs
    }

    /// Return game inputs at specified index into history
    pub fn players(&self, frame: usize, netplay: &Netplay) -> Vec<PlayerInput> {
        let result_inputs = self.players_no_log(frame, netplay);

        debug!("players()");
        for (i, input) in result_inputs.iter().enumerate() {
            debug!("    #{} a: {} b: {} input.stick_x: {} input.stick_y: {}", i, input.a.value, input.b.value, input.stick_x.value, input.stick_y.value);
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

        assert!(result.len() == 8, "get_8frames_of_input needs to return a vector of size 8 but it was {}", result.len());
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
            let (stick_x, stick_y)     = pf_sandbox_lib_input::stick_filter(pf_sandbox_lib_input::stick_deadzone(raw_stick_x,   deadzone.stick_x),  
                                                                            pf_sandbox_lib_input::stick_deadzone(raw_stick_y,   deadzone.stick_y));
            let (c_stick_x, c_stick_y) = pf_sandbox_lib_input::stick_filter(pf_sandbox_lib_input::stick_deadzone(raw_c_stick_x, deadzone.c_stick_x),
                                                                            pf_sandbox_lib_input::stick_deadzone(raw_c_stick_y, deadzone.c_stick_y));
            let l_trigger = pf_sandbox_lib_input::trigger_filter(raw_l_trigger.saturating_sub(deadzone.l_trigger));
            let r_trigger = pf_sandbox_lib_input::trigger_filter(raw_r_trigger.saturating_sub(deadzone.r_trigger));

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
