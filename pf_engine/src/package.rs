use std::fs::{File, DirBuilder, self};
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::collections::HashSet;
use serde::Serialize;
use serde_json::Value;
use serde_json;
use treeflection::{Node, NodeRunner, NodeToken, ContextVec};
use std::env;

use ::fighter::{Fighter, ActionFrame, CollisionBox, CollisionBoxLink, LinkType, RenderOrder};
use ::rules::Rules;
use ::stage::Stage;
use ::json_upgrade::{engine_version, upgrade_to_latest};

fn get_packages_path() -> PathBuf {
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
                home.push("PF_ENGINE/packages");
                home
            }
            #[cfg(windows)]
            {
                home.push("AppData\\Local\\PF_ENGINE\\packages");
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
        let mut path = get_packages_path();
        path.push(name);

        let meta = PackageMeta {
            engine_version:  engine_version(),
            save_version:    0,
            title:           "".to_string(),
            source:          "".to_string(),
            signature:       "".to_string(),
            read_only:       false,
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

    // TODO: Eventually we will just ship with some nice example package
    // This function will be deleted and we can just load the example package everywhere this is used.
    fn generate_base(name: &str) -> Package {
        let mut path = get_packages_path();
        path.push(name);

        let meta = PackageMeta {
            engine_version:  engine_version(),
            save_version:    0,
            title:           "Base Package".to_string(),
            source:          "example.com/base_package".to_string(),
            signature:       "".to_string(),
            read_only:       false,
        };

        let mut package = Package {
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
        let package_path = get_packages_path().join(package_name);

        // if a package does not already exist create a new one
        let mut package = match fs::metadata(package_path) {
            Ok(_)  => Package::open(package_name),
            Err(_) => Package::generate_base(package_name),
        };

        let package_update = PackageUpdate::Package(package.clone());
        package.package_updates.push(package_update);
        package
    }

    pub fn save(&mut self) {
        self.meta.save_version += 1;
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
        let mut meta = Package::load_struct(self.path.join("package_meta.json"));
        let mut rules = Package::load_struct(self.path.join("rules.json"));

        let mut fighters: Vec<Value> = vec!();
        for path in fs::read_dir(self.path.join("Fighters")).unwrap() {
            let full_path = path.unwrap().path();
            self.fighters_filenames.push(full_path.to_str().unwrap().to_string());

            fighters.push(Package::load_struct(full_path));
        }

        let mut stages: Vec<Value> = vec!();
        for path in fs::read_dir(self.path.join("Stages")).unwrap() {
            let full_path = path.unwrap().path();
            self.stages_filenames.push(full_path.to_str().unwrap().to_string());

            stages.push(Package::load_struct(full_path));
        }

        // the upgraded json is loaded into this package
        // the user can then save the package to make the upgrade permanent
        // some nice side effects:
        // *    the package cannot be saved if it wont load
        // *    the user can choose to not save, if they find issues with the upgrade
        upgrade_to_latest(&mut meta, &mut rules, &mut fighters, &mut stages);
        self.load_from_json(meta, rules, fighters, stages);
    }

    pub fn load_from_json(&mut self, meta: Value, rules: Value, fighters: Vec<Value>, stages: Vec<Value>) {
        self.meta = serde_json::from_value(meta).unwrap();
        self.rules = serde_json::from_value(rules).unwrap();
    
        for fighter in fighters {
            self.fighters.push(serde_json::from_value(fighter).unwrap());
        }

        for stage in stages {
            self.stages.push(serde_json::from_value(stage).unwrap());
        }
    }

    // Save a struct to the given file name
    fn save_struct<T: Serialize>(filename: PathBuf, object: &T) {
        let json = serde_json::to_string_pretty(object).unwrap();
        File::create(filename).unwrap().write_all(&json.as_bytes()).unwrap();
    }

    // Load a struct from the given file name
    fn load_struct(filename: PathBuf) -> Value {
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
            let new_link_index = fighter_frame.colbox_links.len();
            fighter_frame.colbox_links.push(CollisionBoxLink {
                one:       *colbox_index,
                two:       new_colbox_index,
                link_type: link_type.clone(),
            });
            fighter_frame.render_order.push(RenderOrder::Link(new_link_index));
        }

        if link_to.len() == 0 {
            fighter_frame.render_order.push(RenderOrder::Colbox(new_colbox_index));
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

    pub fn delete_fighter_colboxes(&mut self, fighter: usize, action: usize, frame: usize, colboxes_to_delete: &HashSet<usize>) {
        let mut fighter_frame = &mut self.fighters[fighter].actions[action].frames[frame];
        {
            let mut colboxes = &mut fighter_frame.colboxes;

            // ensure that collisionboxes are deleted in an order in which the indexes continue to refer to the same element.
            let mut colboxes_to_delete = colboxes_to_delete.iter().collect::<Vec<_>>();
            colboxes_to_delete.sort();
            colboxes_to_delete.reverse();

            for delete_colbox_i in colboxes_to_delete {
                // delete colboxes
                let delete_colbox_i = *delete_colbox_i;
                colboxes.remove(delete_colbox_i);

                // construct a new RenderOrder vec that is valid after the colbox deletion
                let mut new_render_order: Vec<RenderOrder> = vec!();
                for order in &fighter_frame.render_order {
                    match order {
                        &RenderOrder::Colbox (order_colbox_i) => {
                            if order_colbox_i != delete_colbox_i {
                                new_render_order.push(order.dec_greater_than(delete_colbox_i));
                            }
                        }
                        &RenderOrder::Link (_) => {
                            new_render_order.push(order.clone());
                        }
                    }
                }
                fighter_frame.render_order = new_render_order;

                // construct a new links vec that is valid after the colbox deletion
                let mut new_links: Vec<CollisionBoxLink> = vec!();
                let mut deleted_links: Vec<usize> = vec!();
                for (link_i, link) in fighter_frame.colbox_links.iter().enumerate() {
                    if link.contains(delete_colbox_i) {
                        deleted_links.push(link_i);
                    }
                    else {
                        new_links.push(link.dec_greater_than(delete_colbox_i));
                    }
                }
                fighter_frame.colbox_links = new_links;

                // construct a new RendrerOrder vec that is valid after the link deletion
                deleted_links.sort();
                deleted_links.reverse();
                for delete_link_i in deleted_links {
                    let mut new_render_order: Vec<RenderOrder> = vec!();
                    for order in &fighter_frame.render_order {
                        match order {
                            &RenderOrder::Colbox (_) => {
                                new_render_order.push(order.clone());
                            }
                            &RenderOrder::Link (order_link_i) => {
                                if order_link_i != delete_link_i {
                                    new_render_order.push(order.dec_greater_than(delete_link_i));
                                }
                            }
                        }
                    }
                    fighter_frame.render_order = new_render_order;
                }
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

    pub fn resize_fighter_colboxes(&mut self, fighter: usize, action: usize, frame: usize, resized_colboxes: &HashSet<usize>, size_diff: f32) {
        let mut fighter_frame = &mut self.fighters[fighter].actions[action].frames[frame];
        {
            let mut colboxes = &mut fighter_frame.colboxes;

            for i in resized_colboxes {
                let mut colbox = &mut colboxes[*i];
                colbox.radius += size_diff;
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

    /// All colboxes or links containing colboxes from reordered_colboxes are sent to the back
    pub fn fighter_colboxes_send_to_back(&mut self, fighter: usize, action: usize, frame: usize, reordered_colboxes: &HashSet<usize>) {
        let mut fighter_frame = &mut self.fighters[fighter].actions[action].frames[frame];
        {
            for reorder_colbox_i in reordered_colboxes {
                let links = fighter_frame.get_links_containing_colbox(*reorder_colbox_i);
                let colbox_links_clone = fighter_frame.colbox_links.clone();
                let mut render_order = &mut fighter_frame.render_order;

                // delete pre-existing value
                render_order.retain(|x| -> bool {
                    match x {
                        &RenderOrder::Colbox (ref colbox_i) => {
                            colbox_i != reorder_colbox_i
                        }
                        &RenderOrder::Link (link_i) => {
                            !colbox_links_clone[link_i].contains(*reorder_colbox_i)
                        }
                    }
                });

                // reinsert value
                if links.len() == 0 {
                    render_order.insert(0, RenderOrder::Colbox (*reorder_colbox_i));
                }
                else {
                    for link_i in links {
                        render_order.insert(0, RenderOrder::Link (link_i));
                    }
                }
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

    /// All colboxes or links containing colboxes from reordered_colboxes are sent to the front
    pub fn fighter_colboxes_send_to_front(&mut self, fighter: usize, action: usize, frame: usize, reordered_colboxes: &HashSet<usize>) {
        let mut fighter_frame = &mut self.fighters[fighter].actions[action].frames[frame];
        {
            for reorder_i in reordered_colboxes {
                let links = fighter_frame.get_links_containing_colbox(*reorder_i);
                let mut render_order = &mut fighter_frame.render_order;

                // delete pre-existing value
                render_order.retain(|x| -> bool {
                    match x {
                        &RenderOrder::Colbox (ref colbox_i) => {
                            colbox_i != reorder_i
                        }
                        &RenderOrder::Link (_) => {
                            true
                        }
                    }
                });

                // reinsert value
                if links.len() == 0 {
                    render_order.push(RenderOrder::Colbox (*reorder_i));
                }
                else {
                    for link_i in links {
                        render_order.push(RenderOrder::Link (link_i));
                    }
                }
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
    pub engine_version:  u64,    // compared with a value incremented by pf engine when there are breaking changes to data structures
    pub save_version:    u64,    // incremented every time the package is saved
    pub title:           String, // User readable title
    pub source:          String, // check "https://"+source+str(release+1)+".zip" for the next update
    pub signature:       String, // package validity + title + version will be boldly declared on the CSS screen
    pub read_only:       bool,   // read only packages must be copied before being modified
    // TODO: will need to store public keys somewhere too
}
