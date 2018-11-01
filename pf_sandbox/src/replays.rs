use std::fs;
use std::path::PathBuf;
use std::cmp::Ordering;

use chrono::{Local, DateTime};

use pf_sandbox_lib::files;
use pf_sandbox_lib::input::ControllerInput;
use pf_sandbox_lib::package::Package;
use pf_sandbox_lib::stage::Stage;
use crate::game::{Game, PlayerSetup};
use crate::input::Input;
use crate::player::Player;

pub fn get_replay_names(package: &Package) -> Vec<String> {
    let mut result: Vec<String> = vec!();
    
    if let Ok(files) = fs::read_dir(get_replays_dir_path(package)) {
        for file in files {
            if let Ok(file) = file {
                let file_name = file.file_name().into_string().unwrap();
                if let Some(split_point) = file_name.rfind(".") {
                    let (name, ext) = file_name.split_at(split_point);
                    if ext.to_lowercase() == ".zip" {
                        result.push(name.to_string());
                    }
                }
            }
        }
    }

    // Most recent dates come first
    // Dates come before non-dates
    // Non-dates are sorted alphabetically
    result.sort_by(
        |a, b| {
            let a_dt = DateTime::parse_from_rfc2822(a);
            let b_dt = DateTime::parse_from_rfc2822(b);
            if a_dt.is_err() && b_dt.is_err() {
                a.cmp(b)
            } else {
                if let Ok(a_dt) = a_dt {
                    if let Ok(b_dt) = b_dt {
                        a_dt.cmp(&b_dt).reverse()
                    } else {
                        Ordering::Less
                    }
                } else {
                    Ordering::Greater
                }
            }
        }
    );
    result
}

fn get_replays_dir_path(package: &Package) -> PathBuf {
    let mut replays_path = files::get_path();
    replays_path.push("replays");
    replays_path.push(package.file_name());
    replays_path
}

fn get_replay_path(package: &Package, name: &str) -> PathBuf {
    let mut replay_path = get_replays_dir_path(package);
    replay_path.push(format!("{}.zip", name));
    replay_path
}

pub fn load_replay(name: &str, package: &Package) -> Result<Replay, String> {
    let replay_path = get_replay_path(package, name);
    files::load_struct_compressed(replay_path)
}

pub fn save_replay(replay: &Replay, package: &Package) {
    let replay_path = get_replay_path(package, replay.timestamp.to_rfc2822().as_ref()); // TODO: could still collide under strange circumstances: check and handle
    files::save_struct_compressed(replay_path, &replay);
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Replay {
    pub init_seed:            u64,
    pub timestamp:            DateTime<Local>,
    pub input_history:        Vec<Vec<ControllerInput>>,
    pub player_history:       Vec<Vec<Player>>,
    pub stage_history:        Vec<Stage>,
    pub selected_controllers: Vec<usize>,
    pub selected_players:     Vec<PlayerSetup>,
    pub selected_ais:         Vec<usize>,
    pub selected_stage:       String,
}

impl Replay {
    pub fn new(game: &Game, input: &Input) -> Replay {
        let selected_players = game.players.iter().map(|x| PlayerSetup {
            fighter: x.fighter.clone(),
            team:    x.team,
        }).collect();

        Replay {
            init_seed:            game.init_seed.clone(),
            timestamp:            Local::now(),
            input_history:        input.get_history(),
            player_history:       game.player_history.clone(),
            stage_history:        game.stage_history.clone(),
            selected_controllers: game.selected_controllers.clone(),
            selected_ais:         game.selected_ais.clone(),
            selected_stage:       game.selected_stage.clone(),
            selected_players
        }
    }
}
