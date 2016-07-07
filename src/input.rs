use glium::glutin::VirtualKeyCode;
use libusb::{Context, Device, DeviceHandle};
use std::collections::VecDeque;
use std::time::Duration;

pub struct Input<'a> {
    adapter_handles: Vec<DeviceHandle<'a>>,
    history: VecDeque<Vec<PlayerInput>>,
    last_confirmed_frame: u64,
}

impl<'a> Input<'a> {
    pub fn new(context: &'a mut  Context) -> Input<'a> {
        let mut adapter_handles: Vec<DeviceHandle> = Vec::new();
        let devices = context.devices();
        for mut device in devices.unwrap().iter() {
            if let Ok(device_desc) = device.device_descriptor() {
                if device_desc.product_id() == 0x0337 {
                    if let Ok(mut handle) = device.open() {
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
                }
            }
        }
        let mut input = Input {
            adapter_handles: adapter_handles,
            history: VecDeque::new(),
            last_confirmed_frame: 0,
        };
        input.new_history();
        input
    }
    
    /// Generate a new history starting with empty inputs for all controllers
    fn new_history(&mut self) {
        let mut history: VecDeque<Vec<PlayerInput>> = VecDeque::new();
        let mut empty_inputs: Vec<PlayerInput> = Vec::new();

        // create empty inputs
        for _ in &mut self.adapter_handles {
            for _ in 0..4 {
                empty_inputs.push(empty_player_input());
            }
        }
        for _ in 0..4 {
            empty_inputs.push(empty_player_input());
        }

        history.push_front(empty_inputs);
        self.history = history;
    }

    pub fn reset_history(&mut self) {
        self.last_confirmed_frame = 0;
        self.new_history();
    }
    
    /// return the latest inputs
    pub fn read(&mut self, confirmed_frame: u64) -> &Vec<PlayerInput> {
        // add current frame
        let mut inputs: Vec<PlayerInput> = Vec::new();
        {
            let prev_inputs = &self.history.front().unwrap();
            for handle in &mut self.adapter_handles {
                read_gc_adapter(handle, &mut inputs, prev_inputs);
            }
            read_usb_controllers(&mut inputs, prev_inputs);
        }
        self.history.push_front(inputs);

        // delete confirmed frames
        if self.history.len() > 2 {
            for _ in self.last_confirmed_frame..confirmed_frame {
                self.history.pop_back();
            }
        }
        self.last_confirmed_frame = confirmed_frame;
        
        // return current frame
        self.history.front().unwrap()
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

fn empty_player_input() -> PlayerInput {
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
    }
}

/// Add 4 GC adapter controllers to inputs
fn read_gc_adapter(handle: &mut DeviceHandle, inputs: &mut Vec<PlayerInput>, prev_inputs: &Vec<PlayerInput>) {
    let mut data: [u8; 37] = [0; 37];
    handle.read_interrupt(0x81, &mut data, Duration::new(1, 0)).unwrap();

    for port in 0..4 {
        let prev_input = &prev_inputs[inputs.len()];

        // Returns false when rumble usb is not plugged in making this essentially useless
        let plugged_in = data[9*port+1] == 20;

        let up    = data[9*port+2] & 0b10000000 != 0;
        let down  = data[9*port+2] & 0b01000000 != 0;
        let right = data[9*port+2] & 0b00100000 != 0;
        let left  = data[9*port+2] & 0b00010000 != 0;
        let y     = data[9*port+2] & 0b00001000 != 0;
        let x     = data[9*port+2] & 0b00000100 != 0;
        let b     = data[9*port+2] & 0b00000010 != 0;
        let a     = data[9*port+2] & 0b00000001 != 0;
        let l     = data[9*port+3] & 0b00001000 != 0;
        let r     = data[9*port+3] & 0b00000100 != 0;
        let z     = data[9*port+3] & 0b00000010 != 0;
        let start = data[9*port+3] & 0b00000001 != 0;

        let (stick_x, stick_y)     = stick_filter(data[9*port+4], data[9*port+5]);
        let (c_stick_x, c_stick_y) = stick_filter(data[9*port+6], data[9*port+7]);

        let l_trigger  = trigger_filter(data[9*port+8]);
        let r_trigger  = trigger_filter(data[9*port+9]);

        inputs.push(PlayerInput {
            plugged_in: plugged_in,

            up:    Button { value: up,    press: up    && !prev_input.up.value },
            down:  Button { value: down,  press: down  && !prev_input.down.value },
            right: Button { value: right, press: right && !prev_input.right.value },
            left:  Button { value: left,  press: left  && !prev_input.left.value },
            y:     Button { value: y,     press: y     && !prev_input.y.value },
            x:     Button { value: x,     press: x     && !prev_input.x.value },
            b:     Button { value: b,     press: b     && !prev_input.b.value },
            a:     Button { value: a,     press: a     && !prev_input.a.value },
            l:     Button { value: l,     press: l     && !prev_input.l.value },
            r:     Button { value: r,     press: r     && !prev_input.r.value },
            z:     Button { value: z,     press: z     && !prev_input.z.value },
            start: Button { value: start, press: start && !prev_input.start.value },

            stick_x:   Stick   { value: stick_x,   diff: stick_x   - prev_input.stick_x.value },
            stick_y:   Stick   { value: stick_y,   diff: stick_y   - prev_input.stick_y.value },
            c_stick_x: Stick   { value: c_stick_x, diff: c_stick_x - prev_input.c_stick_x.value },
            c_stick_y: Stick   { value: c_stick_y, diff: c_stick_y - prev_input.c_stick_y.value },

            l_trigger:  Trigger { value: l_trigger, diff: (l_trigger) - (prev_input.l_trigger.value) },
            r_trigger:  Trigger { value: r_trigger, diff: (r_trigger) - (prev_input.r_trigger.value) },
        });
    };
}

//TODO: implement
/// Add 4 controllers from usb to inputs
#[allow(unused_variables)]
fn read_usb_controllers(inputs: &mut Vec<PlayerInput>, prev_inputs: &Vec<PlayerInput>) {
    for _ in 0..4 {
        inputs.push(empty_player_input());
    }
}

fn abs_min(a: f64, b: f64) -> f64 {
    if (a >= 0.0 && a > b) || (a <= 0.0 && a < b) {
        b
    } else {
        a
    }
}
fn stick_filter(in_stick_x: u8, in_stick_y: u8) -> (f64, f64) {
    let raw_stick_x = in_stick_x as f64 - 128.0;
    let raw_stick_y = in_stick_y as f64 - 128.0;
    let angle = (raw_stick_y).atan2(raw_stick_x);

    let max = (angle.cos() * 80.0).trunc();
    let stick_x = abs_min(raw_stick_x, max) / 80.0;

    let max = (angle.sin() * 80.0).trunc();
    let stick_y = abs_min(raw_stick_y, max) / 80.0;

    (stick_x, stick_y)
}

fn trigger_filter(trigger: u8) -> f64 {
    let value = (trigger as f64) / 140.0;
    if value > 1.0
    {
        1.0
    }
    else {
        value
    }

}

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
}

pub struct Button {
    pub value: bool, // on
    pub press: bool, // off->on this frame
}

// must use i16 instead of i8 for Stick.value as u8::min_value().abs() causes overflow.
pub struct Stick {
    pub value: f64, // current.value
    pub diff:  f64, // current.value - previous.value
}

pub struct Trigger {
    pub value: f64, // current.value
    pub diff:  f64, // current.value - previous.value
}

pub struct KeyInput {
    current_actions: Vec<KeyAction>,
    held: [bool; 148], // number of VirtualKeyCode's
}

impl KeyInput {
    pub fn new() -> KeyInput {
        KeyInput {
            current_actions: vec!(),
            held: [false; 148],
        }
    }

    /// Called every frame to set the new keyboard inputs
    pub fn set_actions(&mut self, actions: Vec<KeyAction>) {
        for action in &actions {
            match action {
                &KeyAction::Pressed(key_code)  => { self.held[key_code as usize] = true; },
                &KeyAction::Released(key_code) => { self.held[key_code as usize] = false; }
            }
        }

        self.current_actions = actions;
    }

    /// off->on
    pub fn pressed(&self, check_key_code: VirtualKeyCode) -> bool {
        for action in &self.current_actions {
            if let &KeyAction::Pressed(key_code) = action {
                if key_code == check_key_code {
                    return true;
                }
            }
        }
        false
    }

    /// on->off
    pub fn released(&self, check_key_code: VirtualKeyCode) -> bool {
        for action in &self.current_actions {
            if let &KeyAction::Released(key_code) = action {
                if key_code == check_key_code {
                    return true;
                }
            }
        }
        false
    }

    /// on
    pub fn held(&self, key_code: VirtualKeyCode) -> bool {
        return self.held[key_code as usize];
    }
}

pub enum KeyAction {
    Pressed  (VirtualKeyCode),
    Released (VirtualKeyCode),
}
