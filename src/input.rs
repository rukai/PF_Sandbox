use std::time::Duration;
use libusb::{Context, Device, DeviceHandle};
use std::collections::VecDeque;

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
            let device_desc = device.device_descriptor().unwrap();
            if device_desc.product_id() == 0x0337 {
                let mut handle = device.open().unwrap();
                if handle.kernel_driver_active(0).unwrap() {
                    handle.detach_kernel_driver(0).unwrap();
                }
                
                // Tell adapter to start reading
                let payload = [0x13];
                handle.write_interrupt(0x2, &payload, Duration::new(1, 0)).unwrap();
                
                adapter_handles.push(handle);
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

    #[allow(dead_code)]
    fn debug_print(device: &mut Device) {
        for interface in device.config_descriptor(0).unwrap().interfaces() {
            println!("interface: {}", interface.number());
            for setting in interface.descriptors() {
                for endpoint in setting.endpoint_descriptors() {
                    println!("endpoint: {}, {}", endpoint.number(), endpoint.address());
                }
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

        stick_x:   Stick { value: 0, diff: 0 },
        stick_y:   Stick { value: 0, diff: 0 },
        c_stick_x: Stick { value: 0, diff: 0 },
        c_stick_y: Stick { value: 0, diff: 0 },
        l_analog:  Trigger { value: 0, diff: 0 },
        r_analog:  Trigger { value: 0, diff: 0 },
    }
}

/// Add 4 GC adapter controllers to inputs
fn read_gc_adapter(handle: &mut DeviceHandle, inputs: &mut Vec<PlayerInput>, prev_inputs: &Vec<PlayerInput>) {
    let mut data: [u8; 37] = [0; 37];
    handle.read_interrupt(0x81, &mut data, Duration::new(1, 0)).unwrap();

    for port in 0..4 {
        let prev_input = &prev_inputs[inputs.len()];

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

        let stick_x   = stick_filter(data[9*port+4]);
        let stick_y   = stick_filter(data[9*port+5]);
        let c_stick_x = stick_filter(data[9*port+6]);
        let c_stick_y = stick_filter(data[9*port+7]);
        let l_analog  = data[9*port+8];
        let r_analog  = data[9*port+9];

        if plugged_in {
            inputs.push(PlayerInput {
                plugged_in: true,

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

                l_analog:  Trigger { value: l_analog, diff: (l_analog as i16) - (prev_input.l_analog.value as i16) },
                r_analog:  Trigger { value: r_analog, diff: (r_analog as i16) - (prev_input.r_analog.value as i16) },
            });
        } else {
            inputs.push(empty_player_input());
        }
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

fn stick_filter(stick: u8) -> i16{
    let signed = stick.wrapping_sub(128) as i8;

    if signed < 22 && signed > -22 {
        return 0;
    }

    signed as i16
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
    pub r_analog:  Trigger,
    pub l_analog:  Trigger,
}

pub struct Button {
    pub value: bool, // on
    pub press: bool, // off->on this frame
}

// must use i16 instead of i8 for Stick.value as u8::min_value().abs() causes overflow.
pub struct Stick {
    pub value: i16, // current.value
    pub diff:  i16, // current.value - previous.value
}

pub struct Trigger {
    pub value: u8, // current.value
    pub diff:  i16, // current.value - previous.value
}
