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
    pub players:       Arc<Mutex<Vec<Player>>>,
    selected_fighters: Vec<usize>,
    selected_stage:    usize,
    current_stage:     u64,
    state:             GameState,
    timer:             u64,
}

impl Game {
    pub fn new(package: &Package, selected_fighters: Vec<usize>, selected_stage: usize) -> Game {
        let mut players: Vec<Player> = Vec::new();

        for _ in &selected_fighters {
            players.push(Player::new());
        }

        Game {
            state:    GameState::Running,
            rules:    package.rules.clone(),
            fighters: package.fighters.clone(),
            stages:   package.stages.clone(),

            selected_fighters: selected_fighters,
            selected_stage:    selected_stage,
            current_stage:     0,
            players:           Arc::new(Mutex::new(players)),
            timer:             0,
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
        let fighters = self.fighters.lock().unwrap();
        for (i, player) in (&mut *players).iter_mut().enumerate() {
            if control.start {
                self.state = GameState::Paused; //TODO: on press
            }
            let fighter = &fighters[self.selected_fighters[i]];
            player.step(&control, fighter);
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
