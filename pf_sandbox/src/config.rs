use files;
use package;

use std::path::PathBuf;

use serde_json;
use treeflection::{Node, NodeRunner, NodeToken};

#[derive(Clone, Serialize, Deserialize, Node)]
pub struct Config {
    pub current_package:       Option<String>,
    pub netplay_region:        Option<String>,
    pub auto_save_replay:      bool,
    pub verify_package_hashes: bool,
    pub fullscreen:            bool,
    pub physical_device_name:  Option<String>,
}

impl Config {
    fn get_path() -> PathBuf {
        let mut path = files::get_path();
        path.push("config.json");
        path
    }

    pub fn load() -> Config {
        if let Ok (json) = files::load_json(Config::get_path()) {
            if let Ok (mut config) = serde_json::from_value::<Config>(json) {
                // current_package may have been deleted since config was last saved
                if let Some (ref current_package) = config.current_package.clone() {
                    if !package::exists(current_package.as_str()) {
                        config.current_package = None;
                    }
                }
                return config;
            }
        }
        warn!("{:?} is invalid or does not exist, loading default values", Config::get_path());
        Config::default()
    }

    pub fn save(&self) {
        files::save_struct(Config::get_path(), self);
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            current_package:       None,
            netplay_region:        None,
            auto_save_replay:      false,
            verify_package_hashes: true,
            fullscreen:            false,
            physical_device_name:  None,
        }
    }
}
