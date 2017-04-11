use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use serde_json::Value;
use serde_json;
use treeflection::{Node, NodeRunner, NodeToken, ContextVec};
use crypto::digest::Digest;
use crypto::sha2::Sha256;

use ::files;
use ::fighter::{Fighter, ActionFrame, CollisionBox, CollisionBoxLink, LinkType, RenderOrder};
use ::rules::Rules;
use ::stage::Stage;
use ::json_upgrade::{engine_version, upgrade_to_latest};

fn get_packages_path() -> PathBuf {
    let mut path = files::get_path();
    path.push("packages");
    path
}

/// If PF_Sandbox packages path does not exist then generate a stub 'Example' package.
/// Does not otherwise regenerate this package because the user may wish to delete it.
pub fn generate_example_stub() {
    if !get_packages_path().exists() {
        let mut path = get_packages_path();
        path.push("example");
        path.push("package_meta.json");

        let meta = PackageMeta {
            engine_version: engine_version(),
            save_version:   0,
            title:          "Example Package".to_string(),
            source:         "lucaskent.me/example_package".to_string(),
            hash:           "".to_string(),
        };
        files::save_struct(path, &meta);
    }
}


pub fn print_list() {
    for path in fs::read_dir(get_packages_path()).unwrap() {
        println!("{}", path.unwrap().file_name().to_str().unwrap());
    }
}

pub fn get_package_names() -> Vec<String> {
    fs::read_dir(get_packages_path()).unwrap().map(
        |x| x.unwrap().file_name().into_string().unwrap()
    ).collect()
}

pub fn exists(name: &str) -> bool {
    for path in fs::read_dir(get_packages_path()).unwrap() {
        if path.unwrap().file_name().to_str().unwrap() == name {
            return true;
        }
    }
    false
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
        panic!("Why would you do that >.>");
    }
}

impl Package {
    // TODO: Actually handle failures to load package
    pub fn open(name: &str) -> Package {
        let mut path = get_packages_path();
        path.push(name);

        let meta = PackageMeta {
            engine_version:  engine_version(),
            save_version:    0,
            title:           "".to_string(),
            source:          "".to_string(),
            hash:            "".to_string(),
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
        let mut path = get_packages_path();
        path.push(name);

        let meta = PackageMeta {
            engine_version:  engine_version(),
            save_version:    0,
            title:           "New Package".to_string(),
            source:          "DELET THIS".to_string(), // TODO: include option to disable source functionality and use it here
            hash:            "".to_string(),
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
        package.load();
        package
    }

    /// Opens a package if it exists
    /// Creates and opens it if it doesnt
    /// However if it does exist but is broken in some way it returns None
    pub fn open_or_generate(package_name: &str) -> Option<Package> {
        let package_path = get_packages_path().join(package_name);

        // if a package does not already exist create a new one
        match fs::metadata(package_path) {
            Ok(_)  => Some(Package::open(package_name)), // TODO: Actually handle failures to load package
            Err(_) => Some(Package::generate_base(package_name)),
        }
    }

    pub fn save(&mut self) {
        self.meta.save_version += 1;
        self.meta.hash = self.compute_hash();

        // save all json files
        files::save_struct(self.path.join("rules.json"), &self.rules);
        files::save_struct(self.path.join("package_meta.json"), &self.meta);

        for (i, filename) in self.fighters_filenames.iter().enumerate() {
            files::save_struct(PathBuf::from(filename), &self.fighters[i]);
        }
        
        for (i, filename) in self.stages_filenames.iter().enumerate() {
            files::save_struct(PathBuf::from(filename), &self.stages[i]);
        }
    }

    pub fn load(&mut self) {
        let mut meta = files::load_json(self.path.join("package_meta.json"));
        let mut rules = files::load_json(self.path.join("rules.json"));

        let mut fighters: Vec<Option<Value>> = vec!();
        if let Ok (dir) = fs::read_dir(self.path.join("Fighters")) {
            for path in dir {
                let full_path = path.unwrap().path();
                self.fighters_filenames.push(full_path.to_str().unwrap().to_string());

                fighters.push(files::load_json(full_path));
            }
        }

        let mut stages: Vec<Option<Value>> = vec!();
        if let Ok (dir) = fs::read_dir(self.path.join("Stages")) {
            for path in dir {
                let full_path = path.unwrap().path();
                self.stages_filenames.push(full_path.to_str().unwrap().to_string());

                stages.push(files::load_json(full_path));
            }
        }

        // the upgraded json is loaded into this package
        // the user can then save the package to make the upgrade permanent
        // some nice side effects:
        // *    the package cannot be saved if it wont load
        // *    the user can choose to not save, if they find issues with the upgrade
        upgrade_to_latest(&mut meta, &mut rules, &mut fighters, &mut stages);
        self.json_into_structs(meta, rules, fighters, stages);

        self.force_update_entire_package();
    }

    pub fn json_into_structs(&mut self, meta: Option<Value>, rules: Option<Value>, fighters: Vec<Option<Value>>, stages: Vec<Option<Value>>) {
        if let Some (meta) = meta {
            self.meta = serde_json::from_value(meta).unwrap();
        }
        else {
            self.meta = PackageMeta::default();
        }

        if let Some (rules) = rules {
            self.rules = serde_json::from_value(rules).unwrap();
        }
        else {
            self.rules = Rules::default();
        }
    
        for fighter in fighters {
            if let Some (fighter) = fighter {
                self.fighters.push(serde_json::from_value(fighter).unwrap());
            }
            else {
                self.fighters.push(Fighter::default());
            }
        }

        for stage in stages {
            if let Some (stage) = stage {
                self.stages.push(serde_json::from_value(stage).unwrap());
            }
            else {
                self.stages.push(Stage::default());
            }
        }
    }

    pub fn download_latest_meta(&self) -> Option<PackageMeta> {
        files::load_struct_from_url(format!("{}/package_meta.json", self.meta.source).as_str())
    }

    pub fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.input_str(serde_json::to_string(&self.rules).unwrap().as_str());

        for stage in self.stages.iter() {
            hasher.input_str(serde_json::to_string(stage).unwrap().as_str());
        }

        for fighter in self.fighters.iter() {
            hasher.input_str(serde_json::to_string(fighter).unwrap().as_str());
        }

        hasher.result_str().to_string()
    }

    pub fn verify(&self) -> Verify {
        if let Some(latest_meta) = self.download_latest_meta() {
            let hash = self.compute_hash();
            if self.meta.save_version >= latest_meta.save_version {
                if hash == latest_meta.hash {
                    Verify::Ok
                }
                else {
                    Verify::IncorrectHash
                }
            }
            else {
                Verify::UpdateAvailable
            }
        }
        else {
            Verify::CannotConnect
        }
    }

    pub fn update(&mut self) {
        if let Some(latest_meta) = self.download_latest_meta() {
            if self.meta.save_version < latest_meta.save_version {
                let zip = files::load_bin_from_url(format!("{}/package{}.zip", self.meta.source, latest_meta.save_version).as_str());
                if let Some(zip) = zip {
                    files::extract_zip(&zip, &self.path);
                    self.load();
                }
            }
        }
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

#[derive(Clone, Serialize, Deserialize)]
pub enum Verify {
    Ok,
    None,
    IncorrectHash,
    UpdateAvailable,
    CannotConnect,
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

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct PackageMeta {
    pub engine_version:  u64, // compared with a value incremented by pf engine when there are breaking changes to data structures
    pub save_version:    u64, // incremented every time the package is saved
    pub title:           String,
    pub source:          String,
    pub hash:            String,
}
