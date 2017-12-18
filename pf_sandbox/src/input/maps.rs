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
    pub os:           OS,
    pub id:           usize,
    pub name:         String,
    pub analog_maps:  Vec<AnalogMap>,
    pub digital_maps: Vec<DigitalMap>,
}

impl ControllerMap {
    pub fn get_digital_maps(&self, dest: DigitalDest) -> Vec<(usize, DigitalMap)> {
        let mut result = vec!();
        for (index, map) in self.digital_maps.iter().enumerate() {
            if dest == map.dest {
                result.push((index, map.clone()));
            }
        }
        result
    }

    pub fn get_analog_maps(&self, dest: AnalogDest) -> Vec<(usize, AnalogMap)> {
        let mut result = vec!();
        for (index, map) in self.analog_maps.iter().enumerate() {
            if dest == map.dest {
                result.push((index, map.clone()));
            }
        }
        result
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum OS {
    Windows,
    Linux,
}

impl OS {
    pub fn get_current() -> OS {
        if cfg!(target_os = "linux") {
            OS::Linux
        }
        else if cfg!(target_os = "windows") {
            OS::Windows
        }
        else {
            panic!("OS not supported");
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AnalogMap {
    pub source: usize,
    pub dest:   AnalogDest,
    pub filter: AnalogFilter,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DigitalMap {
    pub source: usize,
    pub dest:   DigitalDest,
    pub filter: DigitalFilter,
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
        AnalogFilter::FromAnalog { min: -1.0, max: 1.0, flip: false }
    }

    pub fn is_digital_source(&self) -> bool {
        match self {
            &AnalogFilter::FromDigital { .. } => true,
            _ => false
        }
    }

    pub fn set_min(&mut self, new_min: f32) {
        match self {
            &mut AnalogFilter::FromAnalog { ref mut min, .. } => { *min = new_min }
            &mut AnalogFilter::FromDigital { .. } => unreachable!()
        }
    }

    pub fn set_max(&mut self, new_max: f32) {
        match self {
            &mut AnalogFilter::FromAnalog { ref mut max, .. } => { *max = new_max }
            &mut AnalogFilter::FromDigital { .. } => unreachable!()
        }
    }

    pub fn set_flip(&mut self, new_flip: bool) {
        match self {
            &mut AnalogFilter::FromAnalog { ref mut flip, .. } => { *flip = new_flip }
            &mut AnalogFilter::FromDigital { .. } => unreachable!()
        }
    }

    pub fn set_value(&mut self, new_value: f32) {
        match self {
            &mut AnalogFilter::FromDigital { ref mut value, .. } => { *value = new_value }
            &mut AnalogFilter::FromAnalog { .. } => unreachable!()
        }
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

    pub fn is_digital_source(&self) -> bool {
        match self {
            &DigitalFilter::FromDigital => true,
            _ => false
        }
    }

    pub fn set_min(&mut self, new_min: f32) {
        match self {
            &mut DigitalFilter::FromAnalog { ref mut min, .. } => { *min = new_min }
            &mut DigitalFilter::FromDigital => unreachable!()
        }
    }

    pub fn set_max(&mut self, new_max: f32) {
        match self {
            &mut DigitalFilter::FromAnalog { ref mut max, .. } => { *max = new_max }
            &mut DigitalFilter::FromDigital => unreachable!()
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum AnalogDest {
    StickX,
    StickY,
    CStickX,
    CStickY,
    RTrigger,
    LTrigger,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
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

#[test]
pub fn controller_maps_file_is_valid() {
    let maps = include_str!("controller_maps.json");
    let _controller_maps: ControllerMaps = serde_json::from_str(maps).unwrap();
}
