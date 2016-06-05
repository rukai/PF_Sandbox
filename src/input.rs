use std::time::Duration;
use libusb::{Context, Device, DeviceHandle};

pub struct Input<'a> {
    handle: DeviceHandle<'a>,
}

impl<'a> Input<'a> {
    pub fn new(context: &'a mut  Context) -> Result<Input<'a>, &'static str> {
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

                return Ok(Input { handle: handle} );
            }
        }
        return Err("No GC adapter found");
    }

    pub fn read(&mut self) -> Vec<PlayerInput> {
        let mut data: [u8; 37] = [0; 37];
        self.handle.read_interrupt(0x81, &mut data, Duration::new(1, 0)).unwrap();

        let mut inputs: Vec<PlayerInput> = Vec::new();
        for port in 0..4 {
            inputs.push(PlayerInput {
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

                stick_x:   stick_filter(data[9*port+4]),
                stick_y:   stick_filter(data[9*port+5]),
                c_stick_x: stick_filter(data[9*port+6]),
                c_stick_y: stick_filter(data[9*port+7]),
                l_analog:  data[9*port+8],
                r_analog:  data[9*port+9],
            });
        };
        inputs
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

fn stick_filter(stick: u8) -> i8{
    let signed = stick.wrapping_sub(128) as i8;

    if signed < 22 && signed > -22 {
        return 0;
    }

    signed
}

pub struct PlayerInput {
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
