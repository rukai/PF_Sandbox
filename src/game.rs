use ::input::{Input, KeyInput, PlayerInput};
use ::package::Package;
use ::player::{Player, RenderPlayer};

use glium::glutin::VirtualKeyCode;

pub struct Game {
    player_history:       Vec<Vec<Player>>,
    current_frame:        usize,
    saved_frame:          usize,
    players:              Vec<Player>,
    selected_controllers: Vec<usize>,
    selected_fighters:    Vec<usize>,
    selected_stage:       usize,
    edit_player:          usize,
    debug_outputs:        Vec<DebugOutput>,
    state:                GameState,
}

impl Game {
    pub fn new(package: &Package, selected_fighters: Vec<usize>, selected_stage: usize, netplay: bool, selected_controllers: Vec<usize>) -> Game {
        let mut players: Vec<Player> = vec!();
        let spawn_points = &package.stages[selected_stage].spawn_points;
        for (i, _) in selected_controllers.iter().enumerate() {
            // Stages can have less spawn points then players
            let spawn = spawn_points[i % spawn_points.len()].clone();
            players.push(Player::new(spawn, package.rules.stock_count));
        }

        // The CLI allows for selected_fighters to be shorter then players
        let mut filled_fighters = selected_fighters.clone();
        let wrap = selected_fighters.len();
        if players.len() > selected_fighters.len() {
            let extra = players.len() - selected_fighters.len();
            for i in 0..extra {
                filled_fighters.push(selected_fighters[i % wrap]);
            }
        }

        Game {
            state:                if netplay { GameState::Netplay } else { GameState::Local },
            player_history:       vec!(),
            current_frame:        0,
            saved_frame:          0,
            players:              players,
            selected_controllers: selected_controllers,
            selected_fighters:    filled_fighters,
            selected_stage:       selected_stage,
            edit_player:          0,
            debug_outputs:        vec!(),
        }
    }

    pub fn step(&mut self, package: &mut Package, input: &mut Input, key_input: &KeyInput) {
        match self.state {
            GameState::Local           => { self.step_local(package, input, key_input); },
            GameState::Netplay         => { self.step_netplay(package, input); },
            GameState::Results         => { self.step_results(); },
            GameState::ReplayForwards  => { self.step_replay_forwards(package, input, key_input); },
            GameState::ReplayBackwards => { self.step_replay_backwards(package, input, key_input); },
            GameState::Paused          => { self.step_pause(package, input, &key_input); },
        }
    }

    fn step_local(&mut self, package: &Package, input: &mut Input, key_input: &KeyInput) {
        // erase any future history
        for _ in (self.current_frame+1)..(self.player_history.len()) {
            self.player_history.pop();
        }

        // run game loop
        input.game_update(self.current_frame);
        let player_inputs = &input.players(self.current_frame);
        self.step_game(package, player_inputs);

        self.player_history.push(self.players.clone());
        if input.start_pressed() || key_input.pressed(VirtualKeyCode::Space) {
            self.state = GameState::Paused;
        }
        else {
            self.current_frame += 1;
        }
    }

    fn step_netplay(&mut self, package: &Package, input: &mut Input) {
        input.game_update(self.current_frame);
        let player_inputs = &input.players(self.current_frame);
        self.step_game(package, player_inputs);

        self.player_history.push(self.players.clone());
        self.current_frame += 1;
    }

    fn step_pause(&mut self, package: &mut Package, input: &mut Input, key_input: &KeyInput) {
        let players_len = self.players.len();

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

        // game flow control
        if key_input.pressed(VirtualKeyCode::J) {
            self.step_replay_backwards(package, input, key_input);
        }
        else if key_input.pressed(VirtualKeyCode::K) {
            self.step_replay_forwards(package, input, key_input);
        }
        else if key_input.pressed(VirtualKeyCode::H) {
            self.state = GameState::ReplayBackwards;
        }
        else if key_input.pressed(VirtualKeyCode::L) {
            self.state = GameState::ReplayForwards;
        }
        else if key_input.pressed(VirtualKeyCode::Space) {
            self.current_frame += 1;
            self.step_local(package, input, key_input);
        }
        else if key_input.pressed(VirtualKeyCode::U) {
            // TODO: Invalidate saved_frame when the frame it refers to is deleted.
            self.saved_frame = self.current_frame;
        }
        else if key_input.pressed(VirtualKeyCode::I) {
            self.jump_frame(package, input);
        }
        else if input.start_pressed() {
            self.state = GameState::Local;
        }

        // TODO: Handle character/stage edits here
    }

    /// next frame is advanced by using the input history on the current frame
    fn step_replay_forwards(&mut self, package: &Package, input: &mut Input, key_input: &KeyInput) {
        if self.current_frame < input.last_frame() {
            let player_inputs = &input.players(self.current_frame);
            self.step_game(package, player_inputs);

            // flow controls
            if key_input.pressed(VirtualKeyCode::H) {
                self.state = GameState::ReplayBackwards;
            }
            if input.start_pressed() || key_input.pressed(VirtualKeyCode::Space) {
                self.state = GameState::Paused;
            }
            self.current_frame += 1;
        }
        else {
            self.state = GameState::Paused;
        }
    }

    /// Immediately jumps to the previous frame in history
    fn step_replay_backwards(&mut self, package: &Package, input: &mut Input, key_input: &KeyInput) {
        if self.current_frame > 0 {
            let jump_to = self.current_frame - 1;
            self.players = self.player_history.get(jump_to).unwrap().clone();

            let player_inputs = &input.players(jump_to);
            self.current_frame = jump_to;
            self.debug_output(package, player_inputs);
        }
        else {
            self.state = GameState::Paused;
        }

        // flow controls
        if key_input.pressed(VirtualKeyCode::L) {
            self.state = GameState::ReplayForwards;
        }
        else if input.start_pressed() || key_input.pressed(VirtualKeyCode::Space) {
            self.state = GameState::Paused;
        }
    }

    /// Jump to the specified frame in history
    fn jump_frame(&mut self, package: &Package, input: &mut Input) {
        let frame = self.saved_frame;
        if (frame+1) < self.player_history.len() {
            self.players = self.player_history.get(frame).unwrap().clone();

            let player_inputs = &input.players(frame);
            self.current_frame = frame;
            self.debug_output(package, player_inputs);
        }
    }

    fn step_game(&mut self, package: &Package, player_input: &Vec<PlayerInput>) {
        let stage = &package.stages[self.selected_stage];

        // step each player
        for (i, player) in (&mut *self.players).iter_mut().enumerate() {
            let fighter = &package.fighters[self.selected_fighters[i]];
            let input = &player_input[self.selected_controllers[i]];
            player.step(input, fighter, stage);
        }

        // handle timer
        if (self.current_frame / 60) as u64 > package.rules.time_limit {
            self.state = GameState::Results;
        }

        self.debug_output(package, player_input);
    }

    fn debug_output(&mut self, package: &Package, player_input: &Vec<PlayerInput>) {
        println!("\n-------------------------------------------");
        println!("Frame: {}    state: {:?}", self.current_frame, self.state);

        for debug_output in &self.debug_outputs {
            match debug_output {
                &DebugOutput::Physics{ player } => {
                    print!("Player: {}    ", player);
                    self.players[player].debug_physics();
                },
                &DebugOutput::Input{ player } => {
                    print!("Player: {}    ", player);
                    self.players[player].debug_input(&player_input[player]);
                },
                &DebugOutput::InputDiff{ player } => {
                    print!("Player: {}    ", player);
                    self.players[player].debug_input_diff(&player_input[player]);
                },
                &DebugOutput::Action{ player } => {
                    print!("Player: {}    ", player);
                    self.players[player].debug_action(&package.fighters[self.selected_fighters[player]]);
                },
                &DebugOutput::Frame{ player } => {
                    print!("Player: {}    ", player);
                    self.players[player].debug_frame(&package.fighters[self.selected_fighters[player]]);
                },
            }
        }
    }

    fn step_results(&mut self) {
    }

    pub fn render(&self) -> RenderGame {
        let mut entities = vec!();
        for (i, player) in self.players.iter().enumerate() {
            entities.push(RenderEntity::Player(player.render(self.selected_fighters[i])));
        }

        RenderGame {
            entities: entities,
            state:    self.state.clone(),
            pan_x:    0.0,
            pan_y:    0.0,
            zoom:     0.0,
        }
    }
}

enum DebugOutput {
    Physics   {player: usize},
    Input     {player: usize},
    InputDiff {player: usize},
    Action    {player: usize},
    Frame     {player: usize},
}

#[derive(Debug)]
#[derive(Clone)]
pub enum GameState {
    Local,
    ReplayForwards,
    ReplayBackwards,
    Netplay,
    Paused,  // Only Local, ReplayForwards and ReplayBackwards can be paused
    Results, // Both Local and Netplay end at Results
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub struct RenderGame {
    pub entities: Vec<RenderEntity>,
    pub state:    GameState,

    // camera modifiers
    pub pan_x: f64,
    pub pan_y: f64,
    pub zoom:  f64,
}

pub enum RenderEntity {
    Player (RenderPlayer),
}
