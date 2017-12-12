use json_upgrade;
use files;

use std::path::PathBuf;

use serde_json;

#[derive(Serialize, Deserialize)]
pub struct ControllerMaps {
    pub engine_version: u64,
    pub maps:           Vec<ControllerMap>,
}

impl ControllerMaps {
    fn get_path() -> PathBuf {
        let mut path = files::get_path();
        path.push("controller_maps.json");
        path
    }

    pub fn load() -> ControllerMaps {
        if let Ok (json) = files::load_json(ControllerMaps::get_path()) {
            if let Ok (mut maps) = serde_json::from_value::<ControllerMaps>(json) {
                return maps;
            }
        }

        warn!("{:?} is invalid or does not exist, loading default values", ControllerMaps::get_path());
        let maps = include_str!("controller_maps.json");
        serde_json::from_str(maps).unwrap()
    }

    pub fn save(&self) {
        files::save_struct(ControllerMaps::get_path(), self);
    }
}

impl Default for ControllerMaps {
    fn default() -> ControllerMaps {
        ControllerMaps {
            engine_version: json_upgrade::engine_version(),
            maps:           vec!()
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ControllerMap {
    os:          OS,
    id:          usize,
    name:        String,
    analog_maps: Vec<AnalogMap>,
    button_maps: Vec<DigitalMap>,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum OS {
    Windows,
    Linux,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AnalogMap {
    source: usize,
    dest:   AnalogDest,
    filter: AnalogFilter,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DigitalMap {
    source: usize,
    dest:   DigitalDest,
    filter: DigitalFilter,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum AnalogFilter {
    FromDigital { value: f32 }, // is value when true otherwise unchanged, starting at 0 (can stack multiple AnalogMap's in this way)
    FromAnalog  { min: f32, max: f32, flip: bool } // map the analog value from [min, max] to [-1.0, 1.0], flipping if enabled.
}

impl AnalogFilter {
    pub fn default_digital() -> AnalogFilter {
        AnalogFilter::FromDigital { value: 1.0 }
    }
    pub fn default_analog() -> AnalogFilter {
        AnalogFilter::FromAnalog { min: 1.0, max: 1.0, flip: false }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum DigitalFilter {
    FromDigital,
    FromAnalog { min: f32, max: f32 } // true if between min and max false otherwise
}

impl DigitalFilter {
    pub fn default_digital() -> DigitalFilter {
        DigitalFilter::FromDigital
    }

    pub fn default_analog() -> DigitalFilter {
        DigitalFilter::FromAnalog { min: 0.5, max: 1.0 }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum AnalogDest {
    StickX,
    StickY,
    CStickX,
    CStickY,
    RTrigger,
    LTrigger,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum DigitalDest {
    A,
    B,
    X,
    Y,
    Left,
    Right,
    Down,
    Up,
    Start,
    Z,
    R,
    L,
}
