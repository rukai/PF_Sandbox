use std::path::PathBuf;
use std::env;
use std::fs::File;
use std::io::Read;
use std::io::Write;

use serde::Serialize;
use serde_json::Value;
use serde_json;

pub fn save_struct<T: Serialize>(filename: PathBuf, object: &T) {
    let json = serde_json::to_string_pretty(object).unwrap();
    File::create(filename).unwrap().write_all(&json.as_bytes()).unwrap();
}

pub fn load_json(filename: PathBuf) -> Option<Value> {
    if let Ok(mut file) = File::open(filename) {
        let mut json = String::new();
        if let Ok(_) = file.read_to_string(&mut json) {
            Some(serde_json::from_str(&json).unwrap())
        }
        else {
            None
        }
    }
    else {
        None
    }
}

pub fn get_path() -> PathBuf {
    match env::home_dir() {
        Some (mut home) => {
            #[cfg(unix)]
            {
                let share = match env::var("XDG_DATA_HOME") {
                    Ok(share) => {
                        if share == "" {
                            String::from(".local/share")
                        } else {
                            share
                        }
                    }
                    Err(_) => {
                        String::from(".local/share")
                    }
                };
                home.push(&share);
                home.push("PF_Sandbox");
                home
            }
            #[cfg(windows)]
            {
                home.push("AppData\\Local\\PF_Sandbox");
                home
            }
            #[cfg(macos)]
            {
                panic!("macos is unimplemented");
            }
        }
        None => {
            panic!("could not get path of home");
        }
    }
}
