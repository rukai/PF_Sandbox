use std::path::PathBuf;

use serde_json;
use treeflection::{Node, NodeRunner, NodeToken};

use ::files;
use ::package;

#[derive(Clone, Serialize, Deserialize, Node)]
pub struct Config {
    pub current_package: Option<String>,
}

impl Config {
    fn get_path() -> PathBuf{
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
        Config::default()
    }

    pub fn save(&self) {
        files::save_struct(Config::get_path(), self);
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            current_package: None
        }
    }
}
