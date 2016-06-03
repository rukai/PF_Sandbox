use std::thread;
use std::time::Duration;
use libusb::{Context, Device, DeviceHandle};

#[derive(Default)]
pub struct Input {
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

    pub stick_x:   i8,
    pub stick_y:   i8,
    pub c_stick_x: i8,
    pub c_stick_y: i8,
    pub r_analog:  u8,
    pub l_analog:  u8,
}

impl Input {
    pub fn from_adapter(data: &[u8; 37]) -> Vec<Input> {
        let mut inputs: Vec<Input> = Vec::new();
        for port in 0..4 {
            inputs.push(Input {
                plugged_in: data[9*port+1] == 20,

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

                stick_x:   (data[9*port+4].wrapping_sub(128)) as i8,
                stick_y:   (data[9*port+5].wrapping_sub(128)) as i8,
                c_stick_x: (data[9*port+6].wrapping_sub(128)) as i8,
                c_stick_y: (data[9*port+7].wrapping_sub(128)) as i8,
                l_analog:  data[9*port+8],
                r_analog:  data[9*port+9],
            });
        };
        inputs
    }
}

pub fn input_setup() {
    let mut context = Context::new().unwrap();
    for mut device in context.devices().unwrap().iter() {
        let device_desc = device.device_descriptor().unwrap();
        if device_desc.product_id() == 0x0337 {
            let mut handle = device.open().unwrap();
            if handle.kernel_driver_active(0).unwrap() {
                handle.detach_kernel_driver(0).unwrap();
            }
            
            // Tell adapter to start reading
            let payload = [0x13];
            handle.write_interrupt(0x2, &payload, Duration::new(1, 0)).unwrap();

            read_loop(&mut handle);
        }
    }
}

fn read_loop(handle: &mut DeviceHandle) {
    loop {
        let mut data: [u8; 37] = [0; 37];
        handle.read_interrupt(0x81, &mut data, Duration::new(1, 0)).unwrap();
        Input::from_adapter(&data);

        thread::sleep(Duration::from_millis(16));
    }
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
