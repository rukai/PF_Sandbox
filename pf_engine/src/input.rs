use libusb::{Context, Device, DeviceHandle, Error};
use std::ops::Index;
use std::time::Duration;
use std::f32;

use treeflection::{Node, NodeRunner, NodeToken};

pub struct Input<'a> {
    adapter_handles: Vec<DeviceHandle<'a>>,
    current_inputs:  Vec<ControllerInput>,      // inputs for this frame
    game_inputs:     Vec<Vec<ControllerInput>>, // game past and (potentially) future inputs, frame 0 has index 2
    prev_start:      bool,
}

// In/Out is from perspective of computer
// Out means: computer->adapter
// In means:  adapter->computer

impl<'a> Input<'a> {
    pub fn new(context: &'a mut  Context) -> Input<'a> {
        let mut adapter_handles: Vec<DeviceHandle> = Vec::new();
        let devices = context.devices();
        for mut device in devices.unwrap().iter() {
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

        let input = Input {
            adapter_handles: adapter_handles,
            game_inputs:     vec!(),
            current_inputs:  vec!(),
            prev_start:      false,
        };
        input
    }

    fn handle_open_error(e: Error) {
        let access_solution = if cfg!(target_os = "linux") { r#":
    You need to set a udev rule so that the adapter can be accessed.
    To fix this on most Linux distributions, run the following command and then restart your computer.
    echo 'SUBSYSTEM=="usb", ENV{DEVTYPE}=="usb_device", ATTRS{idVendor}=="057e", ATTRS{idProduct}=="0337", TAG+="uaccess"' | sudo tee /etc/udev/rules.d/51-gcadapter.rules"#
        } else { "" };

        match e {
            Error::Access => {
                println!("GC adapter: Permissions error{}", access_solution);
            },
            _ => { println!("GC adapter: Failed to open handle: {:?}", e); },
        }
    }

    /// Call this once every frame
    pub fn update(&mut self, tas_inputs: &[ControllerInput]) {
        let mut inputs: Vec<ControllerInput> = Vec::new();

        for input in tas_inputs.iter().rev() {
            inputs.insert(0, input.clone());
        }

        for handle in &mut self.adapter_handles {
            read_gc_adapter(handle, &mut inputs);
        }
        read_usb_controllers(&mut inputs);


        self.current_inputs = inputs;
    }

    /// Reset the game input history
    pub fn reset_history(&mut self) {
        self.game_inputs = vec!();
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

        for i in 0..4 { // TODO: retrieve number of controllers from frame history
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

    /// Check for start button press
    /// Uses a seperate state from the game inputs
    /// TODO: Maybe this should be extended to include all menu controller interaction? 
    /// TODO: Does not distinguish between start presses from different players, should it?
    pub fn start_pressed(&mut self) -> bool {
        let held = self.start_held();
        let pressed = !self.prev_start && held;
        self.prev_start = held;
        pressed
    }

    fn start_held(&mut self) -> bool {
        for player in &self.current_inputs {
            if player.start {
                return true;
            }
        }
        false
    }

    /// Returns the index to the last frame in history
    pub fn last_frame(&self) -> usize {
        self.game_inputs.len() - 1
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
fn read_gc_adapter(handle: &mut DeviceHandle, inputs: &mut Vec<ControllerInput>) {
    let mut data: [u8; 37] = [0; 37];
    handle.read_interrupt(0x81, &mut data, Duration::new(1, 0)).unwrap();

    for port in 0..4 {
        let (stick_x, stick_y)     = stick_filter(data[9*port+4], data[9*port+5]);
        let (c_stick_x, c_stick_y) = stick_filter(data[9*port+6], data[9*port+7]);

        inputs.push(ControllerInput {
            plugged_in: data[9*port+1] == 20 || data[9*port+1] == 16,

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

            l_trigger: trigger_filter(data[9*port+8]),
            r_trigger: trigger_filter(data[9*port+9]),

            stick_x:   stick_x,
            stick_y:   stick_y,
            c_stick_x: c_stick_x,
            c_stick_y: c_stick_y,

        });
    };
}

// TODO: implement
/// Add 4 controllers from usb to inputs
fn read_usb_controllers(inputs: &mut Vec<ControllerInput>) {
    for _ in 0..0 {
        inputs.push(ControllerInput::empty());
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

    let max = (angle.cos() * 80.0).trunc();
    let mut stick_x = abs_min(raw_stick_x, max) / 80.0;

    let max = (angle.sin() * 80.0).trunc();
    let mut stick_y = abs_min(raw_stick_y, max) / 80.0;

    let deadzone = 0.28;
    if stick_x.abs() < deadzone {
        stick_x = 0.0;
    }
    if stick_y.abs() < deadzone {
        stick_y = 0.0;
    }

    (stick_x, stick_y)
}

fn trigger_filter(trigger: u8) -> f32 {
    let value = (trigger as f32) / 140.0;
    if value > 1.0
    {
        1.0
    }
    else {
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
