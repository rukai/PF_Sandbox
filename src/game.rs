use ::controller::Control;
use ::fighter::Fighter;
use ::package::Package;
use ::player::Player;
use ::rules::Rules;
use ::stage::Stage;

use std::sync::{Arc, Mutex};

enum GameState {
    Running,
    Paused,
    Results,
}

#[allow(dead_code)]
pub struct Game {
    // package data
    rules:    Rules,
    fighters: Arc<Mutex<Vec<Fighter>>>,
    stages:   Arc<Mutex<Vec<Stage>>>,

    // variables
    current_stage: u64,
    state:         GameState,
    pub players:   Arc<Mutex<Vec<Player>>>,
    timer:         u64,
}

impl Game {
    pub fn new(package: &Package, selected_fighters: Vec<String>, stage: String) -> Game {
        let mut players: Vec<Player> = Vec::new();
        for fighter in selected_fighters {
            players.push(Player::new()); // TODO: set or otherwise handle a player knowing who its fighter is
        }

        Game {
            state:    GameState::Running,
            rules:    package.rules.clone(),
            fighters: package.fighters.clone(),
            stages:   package.stages.clone(),

            current_stage: 0,
            players:       Arc::new(Mutex::new(players)),
            timer:         0,
        }
    }

    pub fn run(&mut self) {
        loop {
            match self.state {
                GameState::Running => { self.step_game();    },
                GameState::Results => { self.step_results(); },
                GameState::Paused  => { self.step_pause();   },
            }
            
            //TODO: when finished results screen, return, without aborting
        }
    }

    fn step_game(&mut self) {
        let control: Control = Default::default();
        let mut players = self.players.lock().unwrap();
        for player in &mut *players {
            if control.start {
                self.state = GameState::Paused; //TODO: on press
            }
            player.step(&control);
        }
        self.timer += 1;
        if self.timer > self.rules.time_limit {
            self.state = GameState::Results;
        }
    }

    fn step_pause(&mut self) {
        self.state = GameState::Running;
        //TODO: Handle character/stage edits here
    }

    fn step_results(&mut self) {
    }
}
