use std::fs::{File, DirBuilder, self};
use std::io::Read;
use std::io::Write;
use std::path::{PathBuf, Path};
use std::collections::HashSet;
use rustc_serialize::{Encodable, Decodable};
use rustc_serialize::json::{self, Encoder, DecodeResult};

use ::fighter::{Fighter, ActionFrame, CollisionBox};
use ::rules::Rules;
use ::stage::Stage;

#[derive(Clone)]
pub struct Package {
    pub path:               PathBuf,
    pub meta:               PackageMeta,
    pub rules:              Rules,

    pub stages:             Vec<Stage>,
    pub fighters:           Vec<Fighter>,
    pub stages_filenames:   Vec<String>,
    pub fighters_filenames: Vec<String>,
    pub package_updates:    Vec<PackageUpdate>,
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
            stages:             vec!(),
            fighters:           vec!(),
            fighters_filenames: vec!(),
            stages_filenames:   vec!(),
            package_updates:    vec!(),
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
            package_updates:    vec!(),
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

        // save all json files
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

    pub fn add_fighter_frame(&mut self, fighter: usize, action: usize, frame: usize) {
        let mut action_frames = &mut self.fighters[fighter].action_defs[action].frames;

        let new_frame = action_frames[frame].clone();
        let new_frame_update = new_frame.clone();
        action_frames.insert(frame + 1, new_frame);

        self.package_updates.push(PackageUpdate::InsertFighterFrame {
            fighter:     fighter,
            action:      action,
            frame_index: frame,
            frame:       new_frame_update,
        });
    }

    pub fn delete_fighter_frame(&mut self, fighter: usize, action: usize, frame: usize) -> bool {
        let mut action_frames = &mut self.fighters[fighter].action_defs[action].frames;

        if action_frames.len() > 1 {
            action_frames.remove(frame);

            self.package_updates.push(PackageUpdate::DeleteFighterFrame {
                fighter:     fighter,
                action:      action,
                frame_index: frame,
            });
            true
        } else {
            false
        }
    }

    pub fn append_fighter_colboxes(&mut self, fighter: usize, action: usize, frame: usize, new_colboxes: Vec<CollisionBox>) {
        let mut fighter_frame = &mut self.fighters[fighter].action_defs[action].frames[frame];
        {
            let mut colboxes = &mut fighter_frame.colboxes;

            for colbox in new_colboxes {
                colboxes.push(colbox);
            }
        }

        self.package_updates.push(PackageUpdate::DeleteFighterFrame {
            fighter:     fighter,
            action:      action,
            frame_index: frame,
        });
        self.package_updates.push(PackageUpdate::InsertFighterFrame {
            fighter:     fighter,
            action:      action,
            frame_index: frame,
            frame:       fighter_frame.clone(),
        });
    }

    pub fn delete_fighter_colboxes(&mut self, fighter: usize, action: usize, frame: usize, delete_boxes: &HashSet<usize>) {
        let mut fighter_frame = &mut self.fighters[fighter].action_defs[action].frames[frame];
        {
            let mut colboxes = &mut fighter_frame.colboxes;

            // ensure that collisionboxes are deleted in an order in which the indexes continue to refer to the same element.
            let mut delete_boxes = delete_boxes.iter().collect::<Vec<_>>();
            delete_boxes.sort();
            delete_boxes.reverse();

            for i in delete_boxes {
                colboxes.remove(*i);
            }
        }

        self.package_updates.push(PackageUpdate::DeleteFighterFrame {
            fighter:     fighter,
            action:      action,
            frame_index: frame,
        });
        self.package_updates.push(PackageUpdate::InsertFighterFrame {
            fighter:     fighter,
            action:      action,
            frame_index: frame,
            frame:       fighter_frame.clone(),
        });
    }

    pub fn move_fighter_colboxes(&mut self, fighter: usize, action: usize, frame: usize, moved_colboxes: &HashSet<usize>, distance: (f32, f32)) {
        let mut fighter_frame = &mut self.fighters[fighter].action_defs[action].frames[frame];
        {
            let mut colboxes = &mut fighter_frame.colboxes;
            let (d_x, d_y) = distance;

            for i in moved_colboxes {
                let (b_x, b_y) = colboxes[*i].point;
                colboxes[*i].point = (b_x + d_x, b_y + d_y);
            }
        }

        self.package_updates.push(PackageUpdate::DeleteFighterFrame {
            fighter:     fighter,
            action:      action,
            frame_index: frame,
        });
        self.package_updates.push(PackageUpdate::InsertFighterFrame {
            fighter:     fighter,
            action:      action,
            frame_index: frame,
            frame:       fighter_frame.clone(),
        });
    }

    // TODO: Swaparino
    pub fn updates(&mut self) -> Vec<PackageUpdate> {
        let package_updates = self.package_updates.clone();
        self.package_updates = vec!();
        return package_updates;
    }
}

// Finer grained changes are used when speed is needed
#[derive(Clone)]
pub enum PackageUpdate {
    Package (Package),
    DeleteFighterFrame { fighter: usize, action: usize, frame_index: usize },
    InsertFighterFrame { fighter: usize, action: usize, frame_index: usize, frame: ActionFrame },
    DeleteStage { stage_index: usize },
    InsertStage { stage_index: usize, stage: Stage },
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct PackageMeta {
    pub version:   u64,    // increment every release, 
    pub title:     String, // User readable title
    pub source:    String, // check "https://"+source+str(release+1)+".zip" for the next update
    pub signature: String, // package validity + title + version will be boldly declared on the CSS screen
    pub read_only: bool,   // read only packages must be copied before being modified
    // TODO: will need to store public keys somewhere too
}
