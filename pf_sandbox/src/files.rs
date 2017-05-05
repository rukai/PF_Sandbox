use std::path::{PathBuf, Path};
use std::env;
use std::fs;
use std::fs::File;
use std::fs::DirBuilder;
use std::io::Read;
use std::io::Write;
use std::io::Cursor;

use reqwest;
use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use serde_json::Value;
use serde_json;
use zip::ZipArchive;

pub fn save_struct<T: Serialize>(filename: PathBuf, object: &T) {
    DirBuilder::new().recursive(true).create(filename.parent().unwrap()).unwrap();

    let json = serde_json::to_string_pretty(object).unwrap();
    File::create(filename).unwrap().write_all(&json.as_bytes()).unwrap();
}

pub fn load_struct<T: DeserializeOwned>(filename: PathBuf) -> Option<T> {
    if let Ok(mut file) = File::open(filename) {
        let mut json = String::new();
        if file.read_to_string(&mut json).is_ok() {
            return serde_json::from_str(&json).ok();
        }
    }
    None
}

pub fn load_json(filename: PathBuf) -> Option<Value> {
    if let Ok(mut file) = File::open(filename) {
        let mut json = String::new();
        if let Ok(_) = file.read_to_string(&mut json) {
            return Some(serde_json::from_str(&json).unwrap());
        }
    }
    None
}

/// Load the json file at the passed URL directly into a struct
pub fn load_struct_from_url<T: DeserializeOwned>(url: &str) -> Option<T> {
    if let Some(json_bytes) = load_bin_from_url(url) {
        if let Ok(json) = String::from_utf8(json_bytes) {
            if let Ok(object) = serde_json::from_str(&json) {
                return Some(object);
            }
        }
    }
    None
    // TODO: waiting on upgrade to serde 1.0 https://github.com/seanmonstar/reqwest/pull/79
    //if let Ok(mut response) = reqwest::get(url) {
    //    if response.status().is_success() {
    //        return response.json().ok();
    //    }
    //}
    //None
}

/// Returns the bytes of the file stored at the url
pub fn load_bin_from_url(url: &str) -> Option<Vec<u8>> {
    if let Ok(mut response) = reqwest::get(url) {
        if response.status().is_success() {
            let mut buf: Vec<u8> = vec!();
            if let Ok(_) = response.read_to_end(&mut buf) {
                return Some(buf);
            }
        }
    }
    None
}

/// Delete contents of destination directory
/// Extract contents of zip into destination
pub fn extract_zip(zip: &[u8], destination: &Path) {
    // nuke destination
    fs::remove_dir_all(destination).unwrap();
    fs::create_dir_all(destination).unwrap();

    let mut zip = ZipArchive::new(Cursor::new(zip)).unwrap();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        let mut path = PathBuf::from(destination);
        path.push(file.name());

        if file.name().ends_with("/") { // TODO: Is this cross platform?
            fs::create_dir_all(path).unwrap();
        }
        else {
            let mut buf = Vec::<u8>::new();
            file.read_to_end(&mut buf).unwrap();
            File::create(path).unwrap().write_all(&buf).unwrap();
        }
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
