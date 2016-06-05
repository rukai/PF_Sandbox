use ::input::{Input, PlayerInput};
use ::fighter::Fighter;
use ::package::Package;
use ::player::Player;
use ::rules::Rules;
use ::stage::Stage;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

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
    state:             GameState,
    timer:             u64,
}

impl Game {
    pub fn new(package: &Package, selected_fighters: Vec<usize>, selected_stage: usize) -> Game {
        let mut players: Vec<Player> = Vec::new();

        {
            let stages = package.stages.lock().unwrap();
            for i in 0..selected_fighters.len() {
                let spawn = stages[selected_stage].spawn_points[i].clone();
                players.push(Player::new(spawn.clone(), package.rules.stock_count));
            }
        }

        Game {
            state:    GameState::Running,
            rules:    package.rules.clone(),
            fighters: package.fighters.clone(),
            stages:   package.stages.clone(),

            selected_fighters: selected_fighters,
            selected_stage:    selected_stage,
            players:           Arc::new(Mutex::new(players)),
            timer:             0,
        }
    }

    pub fn run(&mut self, input: &mut Input) {
        loop {
            let player_input = input.read(); //TODO: need to be able to dermine which player owns which player_input (atm out of order port usage will break things)
            match self.state {
                GameState::Running => { self.step_game(&player_input); },
                GameState::Results => { self.step_results();   },
                GameState::Paused  => { self.step_pause();     },
            }
            
            thread::sleep(Duration::from_millis(16));
            //TODO: when finished results screen, return, without aborting
        }
    }

    fn step_game(&mut self, player_input: &Vec<PlayerInput>) {
        let mut players = self.players.lock().unwrap();
        let fighters = self.fighters.lock().unwrap();
        for (i, player) in (&mut *players).iter_mut().enumerate() {
            if player_input[i].start {
                self.state = GameState::Paused; //TODO: on press
            }

            let fighter = &fighters[self.selected_fighters[i]];
            player.step(&player_input[i], fighter);
        }
        self.timer += 1;
        if self.timer / 60 > self.rules.time_limit {
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

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct Point {
    pub x: f64,
    pub y: f64
}
