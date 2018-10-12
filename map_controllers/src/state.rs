use std::collections::HashMap;

use gilrs_core::Gilrs;
use uuid::Uuid;

use pf_sandbox_lib::input::maps::{ControllerMaps, ControllerMap, OS};

pub struct State {
    pub gilrs:             Gilrs,
    pub controller_maps:   ControllerMaps,
    pub controller:        Option<usize>,
    pub ui_to_analog_map:  HashMap<Uuid, usize>,
    pub ui_to_digital_map: HashMap<Uuid, usize>,
    pub analog_history:    HashMap<usize, AnalogHistory>,
    pub last_code:         Code,
}

#[derive(Clone)]
pub enum Code {
    Analog  (usize, AnalogHistory),
    Digital (usize),
    None,
}

#[derive(Clone)]
pub struct AnalogHistory {
    pub min:        i32,
    pub max:        i32,
    pub last_value: i32,
}

impl AnalogHistory {
    pub fn new(value: i32) -> AnalogHistory {
        AnalogHistory {
            min:        value,
            max:        value,
            last_value: value,
        }
    }
}

impl State {
    pub fn new() -> State {
        let gilrs = Gilrs::new().unwrap();
        let mut controller_maps = ControllerMaps::load();

        // add gamepads that dont have an existing mapping
        for i in 0..gilrs.last_gamepad_hint() {
            let gamepad = gilrs.gamepad(i).unwrap();
            let name = gamepad.name().to_string();
            let uuid = Uuid::from_bytes(gamepad.uuid());
            let os = OS::get_current();

            let mut new = true;
            for controller_map in controller_maps.maps.iter() {
                if controller_map.name == name && controller_map.uuid == uuid && controller_map.os == os {
                    new = false;
                }
            }

            if new && gamepad.is_connected() {
                controller_maps.maps.push(ControllerMap {
                    analog_maps:  vec!(),
                    digital_maps: vec!(),
                    os,
                    name,
                    uuid
                });
            }
        }

        State {
            controller:        None,
            ui_to_analog_map:  HashMap::new(),
            ui_to_digital_map: HashMap::new(),
            analog_history:    HashMap::new(),
            last_code:         Code::None,
            gilrs,
            controller_maps,
        }
    }
}

