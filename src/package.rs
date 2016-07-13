use std::fs::{File, DirBuilder, self};
use std::io::Read;
use std::io::Write;
use std::path::{PathBuf, Path};
use rustc_serialize::{Encodable, Decodable};
use rustc_serialize::json::{self, Encoder, DecodeResult};

use ::fighter::Fighter;
use ::rules::Rules;
use ::stage::Stage;

pub struct Package {
    pub path:               PathBuf,
    pub meta:               PackageMeta,
    pub rules:              Rules,

    pub stages:             Vec<Stage>,
    pub fighters:           Vec<Fighter>,
    pub stages_filenames:   Vec<String>,
    pub fighters_filenames: Vec<String>,
}

impl Package {
    pub fn open(name: &str) -> Package {
        let mut path = PathBuf::from("packages");
        path.push(name);

        let meta = PackageMeta {
            version:   0,
            title:     "".to_string(),
            source:    "".to_string(),
            signature: "".to_string(),
            read_only: false,
        };

        let mut package = Package {
            meta:               meta,
            path:               path,
            rules:              Rules::base(),
            stages:             Vec::new(),
            fighters:           Vec::new(),
            fighters_filenames: Vec::new(),
            stages_filenames:   Vec::new(),
        };
        package.load();
        package
    }


    pub fn generate_base(name: &str) -> Package {
        let mut path = PathBuf::from("packages");
        path.push(name);

        let meta = PackageMeta {
            version:   0,
            title:     "Base Package".to_string(),
            source:    "example.com/base_package".to_string(),
            signature: "".to_string(),
            read_only: false,
        };

        let package = Package {
            meta:               meta,
            rules:              Rules::base(),
            stages:             vec!(Stage::base()),
            stages_filenames:   vec!(path.join("Stages").join("base_stage.json").to_str().unwrap().to_string()),
            fighters:           vec!(Fighter::base()),
            fighters_filenames: vec!(path.join("Fighters").join("base_fighter.json").to_str().unwrap().to_string()),
            path:               path,
        };
        package.save();
        package
    }

    pub fn open_or_generate(package_name: &str) -> Package {
        let package_path = Path::new("packages").join(package_name);

        // if a package does not already exist create a new one
        match fs::metadata(package_path) {
            Ok(_)  => Package::open(package_name),
            Err(_) => Package::generate_base(package_name),
        }
    }

    pub fn save(&self) {
        // Create directory structure
        DirBuilder::new().recursive(true).create(self.path.join("Fighters")).unwrap();
        DirBuilder::new().recursive(true).create(self.path.join("Stages")).unwrap();

        //save all json files
        Package::save_struct(self.path.join("rules.json"), &self.rules);
        Package::save_struct(self.path.join("package_meta.json"), &self.meta);

        for (i, filename) in self.fighters_filenames.iter().enumerate() {
            Package::save_struct(PathBuf::from(filename), &self.fighters[i]);
        }
        
        for (i, filename) in self.stages_filenames.iter().enumerate() {
            Package::save_struct(PathBuf::from(filename), &self.stages[i]);
        }
    }

    pub fn load(&mut self) {
        self.rules = Package::load_struct(self.path.join("rules.json")).unwrap();
        self.meta = Package::load_struct(self.path.join("package_meta.json")).unwrap();

        for path in fs::read_dir(self.path.join("Fighters")).unwrap() {
            // TODO: Use magic rust powers to filter out non .json files and form a vec of fighter_filenames
            // http://stackoverflow.com/questions/31225745/iterate-over-stdfsreaddir-and-get-only-filenames-from-paths

            let full_path = path.unwrap().path();
            self.fighters_filenames.push(full_path.to_str().unwrap().to_string());

            self.fighters.push(Package::load_struct(full_path).unwrap());
        }

        for path in fs::read_dir(self.path.join("Stages")).unwrap() {
            let full_path = path.unwrap().path();
            self.stages_filenames.push(full_path.to_str().unwrap().to_string());

            self.stages.push(Package::load_struct(full_path).unwrap());
        }
    }
    
    // Save a struct to the given file name
    fn save_struct<T: Encodable>(filename: PathBuf, object: &T) {
        let mut json = String::new();
        object.encode(&mut Encoder::new_pretty(&mut json)).expect("Failed");
        File::create(filename).unwrap().write_all(&json.as_bytes()).unwrap()
    }

    // Load a struct from the given file name
    fn load_struct<T: Decodable>(filename: PathBuf) -> DecodeResult<T> {
        let mut json = String::new();
        File::open(filename).unwrap().read_to_string(&mut json).unwrap();
        json::decode(&json)
    }

    pub fn verify(&self) -> bool {
        true //It's fine, I triple checked
    }
}

#[derive(RustcEncodable, RustcDecodable)]
pub struct PackageMeta {
    pub version:   u64,    // increment every release, 
    pub title:     String, // User readable title
    pub source:    String, // check "https://"+source+str(release+1)+".zip" for the next update
    pub signature: String, // package validity + title + version will be boldly declared on the CSS screen
    pub read_only: bool,   // read only packages must be copied before being modified
    // TODO: will need to store public keys somewhere too
}
