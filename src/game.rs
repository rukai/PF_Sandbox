use ::input::{Input, KeyInput};
use ::fighter::Fighter;
use ::package::Package;
use ::player::Player;
use ::rules::Rules;
use ::stage::Stage;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use glium::glutin::VirtualKeyCode;

#[derive(Debug)]
enum GameState {
    Local,
    ReplayForwards,
    ReplayBackwards,
    Netplay,
    Paused,  // Only Local, ReplayForwards and ReplayBackwards can be paused
    Results, // Both Local and Netplay end at Results
}

pub struct Game {
    // package data
    rules:    Rules,
    fighters: Arc<Mutex<Vec<Fighter>>>,
    stages:   Arc<Mutex<Vec<Stage>>>,

    // variables
    player_history:    Vec<Vec<Player>>,
    current_frame:     usize,
    saved_frame:       usize,
    pub players:       Arc<Mutex<Vec<Player>>>,
    selected_fighters: Vec<usize>,
    selected_stage:    usize,
    edit_player:       usize,
    debug_outputs:     Vec<DebugOutput>,
    state:             GameState,
}

impl Game {
    pub fn new(package: &Package, selected_fighters: Vec<usize>, selected_stage: usize, netplay: bool) -> Game {
        let mut players: Vec<Player> = Vec::new();
        {
            let stages = package.stages.lock().unwrap();
            for i in 0..selected_fighters.len() {
                let spawn_points = &stages[selected_stage].spawn_points;
                // Spawn points are reused when there are none left
                let spawn = spawn_points[i % spawn_points.len()].clone();
                players.push(Player::new(spawn, package.rules.stock_count));
            }
        }

        Game {
            state:    if netplay { GameState::Netplay } else { GameState::Local },
            rules:    package.rules.clone(),
            fighters: package.fighters.clone(),
            stages:   package.stages.clone(),

            player_history:    vec!(),
            current_frame:     0,
            saved_frame:       0,
            players:           Arc::new(Mutex::new(players)),
            selected_fighters: selected_fighters,
            selected_stage:    selected_stage,
            edit_player:       0,
            debug_outputs:     vec!(),
        }
    }

    pub fn run(&mut self, input: &mut Input, key_input: &Arc<Mutex<KeyInput>>) {
        loop {
            let frame_start = Instant::now();
            {
                input.update();
                let key_input = key_input.lock().unwrap();
                match self.state {
                    GameState::Local           => { self.step_local(input); },
                    GameState::Netplay         => { self.step_netplay(input); },
                    GameState::Results         => { self.step_results(); },
                    GameState::ReplayForwards  => { self.step_replay_forwards(); },
                    GameState::ReplayBackwards => { self.step_replay_backwards(); },
                    GameState::Paused          => { self.step_pause(input, &key_input); },
                }
            }

            let frame_duration = Duration::from_secs(1) / 60;
            let frame_duration_actual = frame_start.elapsed();
            if frame_duration_actual < frame_duration {
                thread::sleep(frame_duration - frame_start.elapsed());
            }
            // TODO: when finished results screen, return, without aborting
        }
    }

    fn step_local(&mut self, input: &mut Input) {
        if input.start_pressed() {
            self.state = GameState::Paused;
        }

        self.step_game(input);
    }

    fn step_netplay(&mut self, input: &mut Input) {
        self.step_game(input);
    }

    fn step_pause(&mut self, input: &mut Input, key_input: &KeyInput) {
        let players_len = self.players.lock().unwrap().len();

        // set current player to direct character edits to
        if key_input.pressed(VirtualKeyCode::Key1) && players_len >= 1 {
            self.edit_player = 0;
        }
        else if key_input.pressed(VirtualKeyCode::Key2) && players_len >= 2 {
            self.edit_player = 1;
        }
        else if key_input.pressed(VirtualKeyCode::Key3) && players_len >= 3 {
            self.edit_player = 2;
        }
        else if key_input.pressed(VirtualKeyCode::Key4) && players_len >= 4 {
            self.edit_player = 3;
        }

        // add debug outputs
        if key_input.pressed(VirtualKeyCode::F1) {
            self.debug_outputs.push(DebugOutput::Physics{ player: self.edit_player });
        }
        if key_input.pressed(VirtualKeyCode::F2) {
            if key_input.held_shift() {
                self.debug_outputs.push(DebugOutput::InputDiff{ player: self.edit_player });
            }
            else {
                self.debug_outputs.push(DebugOutput::Input{ player: self.edit_player });
            }
        }
        if key_input.pressed(VirtualKeyCode::F3) {
            self.debug_outputs.push(DebugOutput::Action{ player: self.edit_player });
        }
        if key_input.pressed(VirtualKeyCode::F4) {
            self.debug_outputs.push(DebugOutput::Frame{ player: self.edit_player });
        }
        if key_input.pressed(VirtualKeyCode::F5) {
            self.debug_outputs = vec!();
        }

        // game flow
        if key_input.pressed(VirtualKeyCode::Comma) {
            if key_input.held_shift() {
                self.step_replay_forwards();
            }
            else {
                self.state = GameState::ReplayForwards;
            }
        }
        else if key_input.pressed(VirtualKeyCode::Colon) {
            if key_input.held_shift() {
                self.step_replay_backwards();
            }
            else {
                self.state = GameState::ReplayBackwards;
            }
        }
        else if key_input.pressed(VirtualKeyCode::Space) {
            self.step_game(input);
        }
        else if key_input.pressed(VirtualKeyCode::K) {
            // TODO: Invalidate saved_frame when the frame it refers to is deleted.
            self.saved_frame = self.current_frame;
        }
        else if key_input.pressed(VirtualKeyCode::L) {
            let frame = self.saved_frame;
            self.jump_frame(frame);
        }

        // allow players to resume game
        if input.start_pressed() {
            self.state = GameState::Local;
        }

        // TODO: Handle character/stage edits here
    }

    fn step_replay_forwards(&mut self) {
    }

    fn step_replay_backwards(&mut self) {
    }

    fn jump_frame(&mut self, frame: usize) {
    }

    fn step_game(&mut self, input: &mut Input) {
        // acquire resources
        let mut players = self.players.lock().unwrap();
        let fighters = self.fighters.lock().unwrap();
        let stages = self.stages.lock().unwrap();
        let stage = &stages[self.selected_stage];

        // input
        input.game_update();
        let player_input = input.player_inputs();

        // step each player
        for (i, player) in (&mut *players).iter_mut().enumerate() {
            let fighter = &fighters[self.selected_fighters[i]];
            player.step(&player_input[i], fighter, stage);
        }

        // handle timer
        self.current_frame += 1;
        if (self.current_frame / 60) as u64 > self.rules.time_limit {
            self.state = GameState::Results;
        }

        println!("\n-------------------------------------------");
        println!("Frame {} ", self.current_frame);

        for debug_output in &self.debug_outputs {
            match debug_output {
                &DebugOutput::Physics{ player } => {
                    print!("Player: {}    ", player);
                    players[player].debug_physics();
                },
                &DebugOutput::Input{ player } => {
                    print!("Player: {}    ", player);
                    players[player].debug_input(&player_input[player]);
                },
                &DebugOutput::InputDiff{ player } => {
                    print!("Player: {}    ", player);
                    players[player].debug_input_diff(&player_input[player]);
                },
                &DebugOutput::Action{ player } => {
                    print!("Player: {}    ", player);
                    players[player].debug_action(&fighters[self.selected_fighters[player]]);
                },
                &DebugOutput::Frame{ player } => {
                    print!("Player: {}    ", player);
                    players[player].debug_frame(&fighters[self.selected_fighters[player]]);
                },
            }
        }
    }

    fn step_results(&mut self) {
    }
}

enum DebugOutput {
    Physics   {player: usize},
    Input     {player: usize},
    InputDiff {player: usize},
    Action    {player: usize},
    Frame     {player: usize},
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct Point {
    pub x: f64,
    pub y: f64
}
