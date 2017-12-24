use std::collections::HashMap;

use gilrs::Gilrs;
use uuid::Uuid;

use pf_sandbox::input::maps::{ControllerMaps, ControllerMap, OS};

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
    pub min:        f32,
    pub max:        f32,
    pub last_value: f32,
}

impl AnalogHistory {
    pub fn new(value: f32) -> AnalogHistory {
        AnalogHistory {
            min:        value,
            max:        value,
            last_value: value,
        }
    }
}

impl State {
    pub fn new() -> State {
        let gilrs = Gilrs::new();
        let mut controller_maps = ControllerMaps::load();

        // add gamepads that dont have an existing mapping
        for (_, gamepad) in gilrs.gamepads() {
            let name = gamepad.name().to_string();

            let mut new = true;
            for controller_map in controller_maps.maps.iter() {
                if controller_map.name == name && controller_map.os == OS::get_current() {
                    new = false;
                }
            }

            if new {
                controller_maps.maps.push(ControllerMap {
                    os:           OS::get_current(),
                    id:           0,
                    analog_maps:  vec!(),
                    digital_maps: vec!(),
                    name
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

