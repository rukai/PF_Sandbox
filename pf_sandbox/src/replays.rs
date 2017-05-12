use files;
use std::fs;
use std::path::PathBuf;
use std::cmp::Ordering;

use chrono::datetime::DateTime;
use chrono::offset::local::Local;

use player::Player;
use game::Game;
use input::{Input, ControllerInput};
use package::Package;

pub fn get_replay_names(package: &Package) -> Vec<String> {
    let mut result: Vec<String> = vec!();
    
    if let Ok(files) = fs::read_dir(get_replays_path(package)) {
        for file in files {
            if let Ok(file) = file {
                result.push(file.file_name().into_string().unwrap());
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
                        a_dt.cmp(&b_dt)
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

pub fn get_replays_path(package: &Package) -> PathBuf {
    let mut replays_path = files::get_path();
    replays_path.push("replays");
    replays_path.push(package.file_name());
    replays_path
}

pub fn load_replay(name: &str, package: &Package) -> Option<Replay> {
    let mut replay_path = get_replays_path(package);
    replay_path.push(name);
    files::load_struct_compressed(replay_path)
}

pub fn save_replay(game: &Game, input: &Input, package: &Package) {
    let mut replay_path = get_replays_path(package);
    let replay = Replay::new(game, input);
    replay_path.push(replay.timestamp.to_rfc2822()); // TODO: could still collide under strange circumstances: check and handle
    files::save_struct_compressed(replay_path, &replay);
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Replay {
    pub timestamp:            DateTime<Local>,
    pub input_history:        Vec<Vec<ControllerInput>>,
    pub player_history:       Vec<Vec<Player>>,
    pub selected_controllers: Vec<usize>,
    pub selected_fighters:    Vec<String>,
    pub selected_stage:       String,
}

impl Replay {
    pub fn new(game: &Game, input: &Input) -> Replay {
        Replay {
            timestamp:            Local::now(),
            input_history:        input.get_history(),
            player_history:       game.player_history.clone(),
            selected_controllers: game.selected_controllers.clone(),
            selected_fighters:    game.selected_fighters.clone(),
            selected_stage:       game.selected_stage.clone(),
        }
    }
}
