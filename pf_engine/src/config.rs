use std::path::PathBuf;

use serde_json;
use treeflection::{Node, NodeRunner, NodeToken};

use ::files;

#[derive(Clone, Serialize, Deserialize, Node)]
pub struct Config {
    pub current_package: String,
}

impl Config {
    fn get_path() -> PathBuf{
        let mut path = files::get_path();
        path.push("config.json");
        path
    }

    pub fn load() -> Config {
        if let Some(json) = files::load_json(Config::get_path()) {
            // TODO: handle upgrades here

            serde_json::from_value(json).unwrap()
        } else {
            Config::default()
        }
    }

    pub fn save(&self) {
        files::save_struct(Config::get_path(), self);
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            current_package: String::from("base_backage")
        }
    }
}
