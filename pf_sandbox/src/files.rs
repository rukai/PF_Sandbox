use std::env;
use std::fs::{DirBuilder, File};
use std::fs;
use std::io::{Cursor, Read, Write, Seek};
use std::path::{PathBuf, Path};

use reqwest::Url;
use reqwest;
use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use serde_json::Value;
use serde_json;
use zip::{ZipArchive, ZipWriter};
use zip::write::FileOptions;

pub fn write_to_zip<TObject: Serialize, TWriter: Write + Seek>(zip: &mut ZipWriter<TWriter>, path: &str, object: &TObject) {
    zip.start_file(path, FileOptions::default()).unwrap();
    let json = serde_json::to_string_pretty(object).unwrap();
    zip.write_all(json.as_bytes()).unwrap();
}

pub fn save_struct<T: Serialize>(filename: PathBuf, object: &T) {
    // ensure parent directories exists
    DirBuilder::new().recursive(true).create(filename.parent().unwrap()).unwrap();

    // save
    let json = serde_json::to_string_pretty(object).unwrap();
    File::create(filename).unwrap().write_all(&json.as_bytes()).unwrap();
}

pub fn save_struct_compressed<T: Serialize>(filename: PathBuf, object: &T) {
    // ensure parent directories exists
    DirBuilder::new().recursive(true).create(filename.parent().unwrap()).unwrap();

    // save
    let json = serde_json::to_string_pretty(object).unwrap();
    let mut zip = ZipWriter::new(File::create(filename).unwrap());
    zip.start_file("data.json", FileOptions::default()).unwrap();
    zip.write_all(&json.as_bytes()).unwrap();
    zip.finish().unwrap();
}

pub fn load_struct<T: DeserializeOwned>(filename: PathBuf) -> Result<T, String> {
    let json = load_file(filename)?;
    serde_json::from_str(&json).map_err(|x| format!("{:?}", x))
}

pub fn load_struct_compressed<T: DeserializeOwned>(filename: PathBuf) -> Result<T, String> {
    let mut zip = ZipArchive::new(File::open(filename).unwrap()).map_err(|x| format!("{:?}", x))?;
    let zip_file = zip.by_name("data.json").map_err(|x| format!("{:?}", x))?;
    serde_json::from_reader(zip_file).map_err(|x| format!("{:?}", x))
}

pub fn load_json(filename: PathBuf) -> Result<Value, String> {
    let json = load_file(filename)?;
    serde_json::from_str(&json).map_err(|x| format!("{:?}", x))
}

pub fn load_file(filename: PathBuf) -> Result<String, String> {
    let mut file = match File::open(&filename) {
        Ok(file) => file,
        Err(err) => return Err(format!("Failed to open file: {} because: {}", filename.to_str().unwrap(), err))
    };

    let mut contents = String::new();
    if let Err(err) = file.read_to_string(&mut contents) {
        return Err(format!("Failed to read file {} because: {}", filename.to_str().unwrap(), err))
    };
    Ok(contents)
}

/// Load the json file at the passed URL directly into a struct
pub fn load_struct_from_url<T: DeserializeOwned>(url: Url) -> Option<T> {
    if let Ok(mut response) = reqwest::get(url) {
        if response.status().is_success() {
            return response.json().ok();
        }
    }
    None
}

/// Returns the bytes of the file stored at the url
pub fn load_bin_from_url(url: Url) -> Option<Vec<u8>> {
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


/// deletes all files in the passed directory
/// if the directory does not exist it is created
pub fn nuke_dir(path: &Path) {
    fs::remove_dir_all(path).ok();
    fs::create_dir_all(path).unwrap();
}

/// Delete contents of destination directory
/// Extract contents of zip into destination
pub fn extract_zip(zip: &[u8], destination: &Path) {
    nuke_dir(destination);

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
                compile_error!("macos is unimplemented");
            }
        }
        None => {
            panic!("could not get path of home");
        }
    }
}
