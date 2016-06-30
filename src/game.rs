use ::input::{Input, PlayerInput, KeyInput};
use ::fighter::Fighter;
use ::package::Package;
use ::player::Player;
use ::rules::Rules;
use ::stage::Stage;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use glium::glutin::VirtualKeyCode;

enum GameState {
    Running,
    Paused,
    Results,
}

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
    frames:            u64,
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
            frames:            0,
        }
    }

    pub fn run(&mut self, input: &mut Input, key_input: &Arc<Mutex<KeyInput>>) {
        loop {
            let player_input = input.read(self.frames);
            let key_input = key_input.lock().unwrap();
            match self.state {
                GameState::Running => { self.step_game(player_input); },
                GameState::Paused  => { self.step_pause(player_input, &key_input); },
                GameState::Results => { self.step_results(); },
            }

            thread::sleep(Duration::from_millis(16));
            //TODO: when finished results screen, return, without aborting
        }
    }

    fn step_game(&mut self, player_input: &Vec<PlayerInput>) {
        // lock resources
        let mut players = self.players.lock().unwrap();
        let fighters = self.fighters.lock().unwrap();
        let stages = self.stages.lock().unwrap();
        let stage = &stages[self.selected_stage];
        
        // step each player
        for (i, player) in (&mut *players).iter_mut().enumerate() {
            if true { // TODO: check not netplay
                if player_input[i].start.press {
                    self.state = GameState::Paused;
                }
            }

            let fighter = &fighters[self.selected_fighters[i]];
            //println!("player: {}", i);
            player.step(&player_input[i], fighter, stage);
        }

        // handle timer
        self.frames += 1;
        if self.frames / 60 > self.rules.time_limit {
            self.state = GameState::Results;
        }
    }

    fn step_pause(&mut self, player_input: &Vec<PlayerInput>, key_input: &KeyInput) {
        if key_input.pressed(VirtualKeyCode::Space) {
            self.step_game(player_input);
        }

        // lock resources
        let players = self.players.lock().unwrap();

        // allow players to resume game
        for (i, _) in (&players).iter().enumerate() {
            if player_input[i].start.press {
                self.state = GameState::Running;
            }
        }

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
