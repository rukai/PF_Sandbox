use std::fs::{File, DirBuilder, self};
use std::io::Read;
use std::io::Write;
use std::path::{PathBuf, Path};
use std::collections::HashSet;
use serde::{Serialize, Deserialize};
use serde_json;
use treeflection::{Node, NodeRunner, NodeToken, ContextVec};

use ::fighter::{Fighter, ActionFrame, CollisionBox, CollisionBoxLink, LinkType};
use ::rules::Rules;
use ::stage::Stage;

#[derive(Clone, Serialize, Deserialize)]
pub struct Package {
        path:               PathBuf,
    pub meta:               PackageMeta,
    pub rules:              Rules,

    pub stages:             ContextVec<Stage>,
    pub fighters:           ContextVec<Fighter>,
        stages_filenames:   Vec<String>,
        fighters_filenames: Vec<String>,
        package_updates:    Vec<PackageUpdate>,
}

impl Default for Package {
    fn default() -> Package {
        Package::open_or_generate("base_package")
    }
}

impl Package {
    fn open(name: &str) -> Package {
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
            stages:             ContextVec::new(),
            fighters:           ContextVec::new(),
            fighters_filenames: vec!(),
            stages_filenames:   vec!(),
            package_updates:    vec!(),
        };
        package.load();
        package
    }

    fn generate_base(name: &str) -> Package {
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
            stages:             ContextVec::from_vec(vec!(Stage::base())),
            stages_filenames:   vec!(path.join("Stages").join("base_stage.json").to_str().unwrap().to_string()),
            fighters:           ContextVec::from_vec(vec!(Fighter::base())),
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
        let mut package = match fs::metadata(package_path) {
            Ok(_)  => Package::open(package_name),
            Err(_) => Package::generate_base(package_name),
        };

        let package_update = PackageUpdate::Package(package.clone());
        package.package_updates.push(package_update);
        package
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
        self.rules = Package::load_struct(self.path.join("rules.json"));
        self.meta = Package::load_struct(self.path.join("package_meta.json"));

        for path in fs::read_dir(self.path.join("Fighters")).unwrap() {
            // TODO: Use magic rust powers to filter out non .json files and form a vec of fighter_filenames
            // http://stackoverflow.com/questions/31225745/iterate-over-stdfsreaddir-and-get-only-filenames-from-paths

            let full_path = path.unwrap().path();
            self.fighters_filenames.push(full_path.to_str().unwrap().to_string());

            self.fighters.push(Package::load_struct(full_path));
        }

        for path in fs::read_dir(self.path.join("Stages")).unwrap() {
            let full_path = path.unwrap().path();
            self.stages_filenames.push(full_path.to_str().unwrap().to_string());

            self.stages.push(Package::load_struct(full_path));
        }
    }
    
    // Save a struct to the given file name
    fn save_struct<T: Serialize>(filename: PathBuf, object: &T) {
        let json = serde_json::to_string_pretty(object).unwrap();
        File::create(filename).unwrap().write_all(&json.as_bytes()).unwrap();
    }

    // Load a struct from the given file name
    fn load_struct<T: Deserialize>(filename: PathBuf) -> T {
        let mut json = String::new();
        File::open(filename).unwrap().read_to_string(&mut json).unwrap();
        serde_json::from_str(&json).unwrap()
    }

    pub fn verify(&self) -> bool {
        true // It's fine, I triple checked
    }

    pub fn new_fighter_frame(&mut self, fighter: usize, action: usize, frame: usize) {
        let new_frame = {
            let action_frames = &self.fighters[fighter].actions[action].frames;
            action_frames[frame].clone()
        };
        self.insert_fighter_frame(fighter, action, frame, new_frame);
    }

    pub fn insert_fighter_frame(&mut self, fighter: usize, action: usize, frame: usize, action_frame: ActionFrame) {
        let mut action_frames = &mut self.fighters[fighter].actions[action].frames;

        action_frames.insert(frame, action_frame.clone());

        self.package_updates.push(PackageUpdate::InsertFighterFrame {
            fighter:     fighter,
            action:      action,
            frame_index: frame,
            frame:       action_frame,
        });
    }

    pub fn delete_fighter_frame(&mut self, fighter: usize, action: usize, frame: usize) -> bool {
        let mut action_frames = &mut self.fighters[fighter].actions[action].frames;

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

    /// add the passed collisionbox to the specified fighter frame
    /// the added collisionbox is linked to the specified collisionboxes
    /// returns the index the collisionbox was added to.
    pub fn append_fighter_colbox(
        &mut self, fighter: usize, action: usize, frame: usize,
        new_colbox: CollisionBox, link_to: &HashSet<usize>, link_type: LinkType
    ) -> usize {
        let mut fighter_frame = &mut self.fighters[fighter].actions[action].frames[frame];
        let new_colbox_index = fighter_frame.colboxes.len();
        fighter_frame.colboxes.push(new_colbox);

        for colbox_index in link_to {
            fighter_frame.colbox_links.push(CollisionBoxLink {
                one:       *colbox_index,
                two:       new_colbox_index,
                link_type: link_type.clone(),
            });
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

        new_colbox_index
    }

    pub fn delete_fighter_colboxes(&mut self, fighter: usize, action: usize, frame: usize, delete_boxes: &HashSet<usize>) {
        let mut fighter_frame = &mut self.fighters[fighter].actions[action].frames[frame];
        {
            let mut colboxes = &mut fighter_frame.colboxes;

            // ensure that collisionboxes are deleted in an order in which the indexes continue to refer to the same element.
            let mut delete_boxes = delete_boxes.iter().collect::<Vec<_>>();
            delete_boxes.sort();
            delete_boxes.reverse();

            for i in delete_boxes {
                let delete = *i;
                colboxes.remove(delete);

                // construct a new list of links that is valid after the deletion
                let mut new_links = vec!();
                for link in &fighter_frame.colbox_links {
                    if !link.contains(delete) {
                        new_links.push(link.dec_greater_than(delete));
                    }
                }
                fighter_frame.colbox_links = new_links;
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
        let mut fighter_frame = &mut self.fighters[fighter].actions[action].frames[frame];
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

    // TODO: Refactor to use a reference would be way faster
    pub fn force_update_entire_package(&mut self) {
        let package_update = PackageUpdate::Package(self.clone());
        self.package_updates.push(package_update);
    }

    // TODO: Swaparino
    pub fn updates(&mut self) -> Vec<PackageUpdate> {
        let package_updates = self.package_updates.clone();
        self.package_updates = vec!();
        return package_updates;
    }
}

impl Node for Package {
    fn node_step(&mut self, mut runner: NodeRunner) -> String {
        let result = match runner.step() {
            NodeToken::ChainProperty (property) => {
                match property.as_str() {
                    "fighters" => { self.fighters.node_step(runner) }
                    "stages"   => { self.stages.node_step(runner) }
                    "meta"     => { self.meta.node_step(runner) }
                    "rules"    => { self.rules.node_step(runner) }
                    prop       => format!("Package does not have a property '{}'", prop)
                }
            }
            action => { format!("Package cannot '{:?}'", action) }
        };

        self.force_update_entire_package();
        result
    }
}

// Finer grained changes are used when speed is needed
#[derive(Clone, Serialize, Deserialize)]
pub enum PackageUpdate {
    Package (Package),
    DeleteFighterFrame { fighter: usize, action: usize, frame_index: usize },
    InsertFighterFrame { fighter: usize, action: usize, frame_index: usize, frame: ActionFrame },
    DeleteStage { stage_index: usize },
    InsertStage { stage_index: usize, stage: Stage },
}

// TODO: Why the seperate struct?
#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct PackageMeta {
    pub version:   u64,    // increment every release, 
    pub title:     String, // User readable title
    pub source:    String, // check "https://"+source+str(release+1)+".zip" for the next update
    pub signature: String, // package validity + title + version will be boldly declared on the CSS screen
    pub read_only: bool,   // read only packages must be copied before being modified
    // TODO: will need to store public keys somewhere too
}
