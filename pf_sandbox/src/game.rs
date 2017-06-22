use ::camera::Camera;
use ::collision::collision_check;
use ::config::Config;
use ::fighter::{ActionFrame, CollisionBox, LinkType};
use ::graphics::{GraphicsMessage, Render, RenderRect};
use ::graphics;
use ::input::{Input, PlayerInput, ControllerInput};
use ::os_input::OsInput;
use ::package::Package;
use ::player::{Player, RenderPlayer, DebugPlayer, RenderFighter};
use ::records::{GameResult, PlayerResult};
use ::replays;
use ::rules::Goal;
use ::stage::{Area, Stage};
use ::rand::{StdRng, SeedableRng};

use std::cmp::Ordering;
use std::collections::HashSet;
use std::iter;
use chrono::Local;

use winit::VirtualKeyCode;
use treeflection::{Node, NodeRunner, NodeToken};

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct Game {
    pub package:                Package,
    pub config:                 Config,
    pub init_seed:              Vec<usize>,
    pub state:                  GameState,
    pub player_history:         Vec<Vec<Player>>,
    pub stage_history:          Vec<Stage>,
    pub current_frame:          usize,
    pub saved_frame:            usize,
    pub stage:                  Stage,
    pub players:                Vec<Player>,
    pub debug_players:          Vec<DebugPlayer>,
    pub selected_controllers:   Vec<usize>,
    pub selected_stage:         String,
    pub edit:                   Edit,
    pub debug_output_this_step: bool,
    pub selector:               Selector,
    copied_frame:               Option<ActionFrame>,
    pub camera:                 Camera,
    pub tas:                    Vec<ControllerInput>
}

/// Frame 0 refers to the initial state of the game.
/// Any changes occur in the proceeding frames i.e. frames 1, 2, 3 ...

/// All previous frame state is used to calculate the next frame then the current_frame is incremented

impl Game {
    pub fn new(package: Package, config: Config, setup: GameSetup) -> Game {
        let stage = package.stages[setup.stage.as_ref()].clone();

        // generate players
        let mut players:       Vec<Player>      = vec!();
        let mut debug_players: Vec<DebugPlayer> = vec!();
        {
            for (i, _) in setup.controllers.iter().enumerate() {
                // Stage can have less spawn points then players
                let spawn = stage.spawn_points[i % stage.spawn_points.len()].clone();
                let respawn = stage.respawn_points[i % stage.respawn_points.len()].clone();
                // The CLI allows for selected_fighters to be shorter then players
                let fighter = setup.fighters[i % setup.fighters.len()].clone();
                players.push(Player::new(fighter, spawn, respawn, package.rules.stock_count));
                debug_players.push(Default::default());
            }
        }

        Game {
            package:                package,
            config:                 config,
            init_seed:              setup.init_seed,
            state:                  setup.state,
            player_history:         setup.player_history,
            stage_history:          setup.stage_history,
            current_frame:          0,
            saved_frame:            0,
            players:                players,
            stage:                  stage,
            debug_players:          debug_players,
            selected_controllers:   setup.controllers,
            selected_stage:         setup.stage,
            edit:                   Edit::Stage,
            debug_output_this_step: false,
            selector:               Default::default(),
            copied_frame:           None,
            camera:                 Camera::new(),
            tas:                    vec!()
        }
    }

    pub fn step(&mut self, input: &mut Input, os_input: &OsInput) -> GameState {
        {
            match self.state.clone() {
                GameState::Local           => { self.step_local(input, os_input); }
                GameState::Netplay         => { self.step_netplay(input); }
                GameState::ReplayForwards  => { self.step_replay_forwards(input, os_input); }
                GameState::ReplayBackwards => { self.step_replay_backwards(input, os_input); }
                GameState::Paused          => { self.step_pause(input, os_input); }
                GameState::ToResults (_)   => { unreachable!(); }
                GameState::ToCSS           => { unreachable!(); }
            }
            self.camera.update(os_input, &self.players, &self.package.fighters, &self.stage);

            if self.debug_output_this_step {
                self.debug_output_this_step = false;
                self.debug_output(input);
            }
        }

        self.set_context();

        self.state.clone()
    }

    fn set_context(&mut self) {
        match self.edit {
            Edit::Fighter (player) => {
                let player_fighter = self.players[player].fighter.as_ref();
                let player_action = self.players[player].action as usize;
                let player_frame  = self.players[player].frame as usize;
                let player_colboxes = self.selector.colboxes_vec();

                let fighters = &mut self.package.fighters;
                if let Some(fighter_index) = fighters.key_to_index(player_fighter) {
                    fighters.set_context(fighter_index);
                }
                else {
                    return;
                }

                let actions = &mut fighters[player_fighter].actions;
                if player_action >= actions.len() {
                    return;
                }
                actions.set_context(player_action);

                let frames = &mut actions[player_action].frames;
                if player_frame >= frames.len() {
                    return;
                }
                frames.set_context(player_frame);

                let colboxes = &mut frames[player_frame].colboxes;
                colboxes.set_context_vec(player_colboxes);
            }
            _ => { }
        }
        let index = self.package.stages.key_to_index(self.selected_stage.as_ref()).unwrap();
        self.package.stages.set_context(index);
    }

    fn step_local(&mut self, input: &mut Input, os_input: &OsInput) {
        self.player_history.push(self.players.clone());
        self.current_frame += 1;

        // erase any future history
        for _ in self.current_frame..self.player_history.len() {
            self.player_history.pop();
        }

        // run game loop
        input.game_update(self.current_frame);
        let player_inputs = &input.players(self.current_frame);
        self.step_game(player_inputs);

        // pause game
        if input.start_pressed() || os_input.key_pressed(VirtualKeyCode::Space) || os_input.key_pressed(VirtualKeyCode::Return) {
            self.state = GameState::Paused;
        }
    }

    fn step_netplay(&mut self, input: &mut Input) {
        self.player_history.push(self.players.clone());
        self.current_frame += 1;

        input.game_update(self.current_frame);
        let player_inputs = &input.players(self.current_frame);
        self.step_game(player_inputs);
    }

    fn step_pause(&mut self, input: &mut Input, os_input: &OsInput) {
        let players_len = self.players.len();

        // set current edit state
        if os_input.key_pressed(VirtualKeyCode::Grave) {
            self.edit = Edit::Stage;
        }
        else if os_input.key_pressed(VirtualKeyCode::Key1) && players_len >= 1 {
            if os_input.held_shift() {
                self.edit = Edit::Player (0);
            }
            else {
                self.edit = Edit::Fighter (0);
            }
            self.update_frame();
        }
        else if os_input.key_pressed(VirtualKeyCode::Key2) && players_len >= 2 {
            if os_input.held_shift() {
                self.edit = Edit::Player (1);
            }
            else {
                self.edit = Edit::Fighter (1);
            }
            self.update_frame();
        }
        else if os_input.key_pressed(VirtualKeyCode::Key3) && players_len >= 3 {
            if os_input.held_shift() {
                self.edit = Edit::Player (2);
            }
            else {
                self.edit = Edit::Fighter (2);
            }
            self.update_frame();
        }
        else if os_input.key_pressed(VirtualKeyCode::Key4) && players_len >= 4 {
            if os_input.held_shift() {
                self.edit = Edit::Player (3);
            }
            else {
                self.edit = Edit::Fighter (3);
            }
            self.update_frame();
        }

        // modify package
        if os_input.key_pressed(VirtualKeyCode::W) {
            replays::save_replay(self, input, &self.package);
        }
        if os_input.key_pressed(VirtualKeyCode::E) {
            self.package.save();
        }
        if os_input.key_pressed(VirtualKeyCode::R) {
            //self.package.load(); // Currently disabled to easy to bump, need some sort of UI confirmation
        }

        // game flow control
        if os_input.key_pressed(VirtualKeyCode::J) {
            self.step_replay_backwards(input, os_input);
        }
        else if os_input.key_pressed(VirtualKeyCode::K) {
            self.step_replay_forwards(input, os_input);
        }
        else if os_input.key_pressed(VirtualKeyCode::H) {
            self.state = GameState::ReplayBackwards;
        }
        else if os_input.key_pressed(VirtualKeyCode::L) {
            self.state = GameState::ReplayForwards;
        }
        else if os_input.key_pressed(VirtualKeyCode::Space) {
            self.step_local(input, os_input);
        }
        else if os_input.key_pressed(VirtualKeyCode::U) {
            // TODO: Invalidate saved_frame when the frame it refers to is deleted.
            self.saved_frame = self.current_frame;
        }
        else if os_input.key_pressed(VirtualKeyCode::I) {
            self.jump_frame();
        }
        else if input.game_quit_held() {
            self.state = GameState::ToCSS;
        }
        else if input.start_pressed() || os_input.key_pressed(VirtualKeyCode::Return) {
            self.state = GameState::Local;
        }

        match self.edit {
            Edit::Fighter (player) => {
                let fighter_string = self.players[player].fighter.clone();
                let fighter = fighter_string.as_ref();
                let action = self.players[player].action as usize;
                let frame  = self.players[player].frame as usize;
                self.set_debug(os_input, player);

                // move collisionboxes
                if self.selector.moving {
                    let (d_x, d_y) = os_input.game_mouse_diff(&self.camera);
                    let distance = (self.players[player].relative_f(d_x), d_y);
                    self.package.move_fighter_colboxes(fighter, action, frame, &self.selector.colboxes, distance);

                    // end move
                    if os_input.mouse_pressed(0) {
                        self.update_frame();
                    }
                }
                else {
                    // copy frame
                    if os_input.key_pressed(VirtualKeyCode::V) {
                        let frame = self.package.fighters[fighter].actions[action].frames[frame].clone();
                        self.copied_frame = Some(frame);
                    }
                    // paste over current frame
                    if os_input.key_pressed(VirtualKeyCode::B) {
                        let action_frame = self.copied_frame.clone();
                        if let Some(action_frame) = action_frame {
                            self.package.insert_fighter_frame(fighter, action, frame, action_frame);
                            self.package.delete_fighter_frame(fighter, action, frame+1);
                        }
                    }

                    // new frame
                    if os_input.key_pressed(VirtualKeyCode::M) {
                        self.package.new_fighter_frame(fighter, action, frame);
                        // We want to step just the players current frame to simplify the animation work flow
                        // However we need to do a proper full step so that the history doesn't get mucked up.
                        self.step_local(input, os_input);
                    }
                    // delete frame
                    if os_input.key_pressed(VirtualKeyCode::N) {
                        if self.package.delete_fighter_frame(fighter, action, frame) {
                            // Correct any players that are now on a nonexistent frame due to the frame deletion.
                            // This is purely to stay on the same action for usability.
                            // The player itself must handle being on a frame that has been deleted in order for replays to work.
                            for any_player in &mut self.players {
                                if any_player.fighter == fighter && any_player.action as usize == action
                                    && any_player.frame as usize == self.package.fighters[fighter].actions[action].frames.len()
                                {
                                    any_player.frame -= 1;
                                }
                            }
                            self.update_frame();
                        }
                    }

                    // start move collisionbox
                    if os_input.key_pressed(VirtualKeyCode::A) {
                        if self.selector.colboxes.len() > 0 {
                            self.selector.moving = true;
                        }
                    }
                    // enter pivot mode
                    if os_input.key_pressed(VirtualKeyCode::S) {
                        // TODO
                    }
                    // delete collisionbox
                    if os_input.key_pressed(VirtualKeyCode::D) {
                        self.package.delete_fighter_colboxes(fighter, action, frame, &self.selector.colboxes);
                        self.update_frame();
                    }
                    // add collisionbox
                    if os_input.key_pressed(VirtualKeyCode::F) {
                        if let Some((m_x, m_y)) = os_input.game_mouse(&self.camera) {
                            let selected = {
                                let player = &self.players[player];
                                let (p_x, p_y) = player.bps_xy(&self.players, &self.package.fighters, &self.stage.platforms);

                                let point = (player.relative_f(m_x - p_x), m_y - p_y);
                                let new_colbox = CollisionBox::new(point);
                                let link_type = match os_input.held_shift() {
                                    true  => { LinkType::Simple },
                                    false => { LinkType::MeldFirst }
                                };

                                self.package.append_fighter_colbox(fighter, action, frame, new_colbox, &self.selector.colboxes, link_type)
                            };
                            self.update_frame();
                            self.selector.colboxes.insert(selected);
                        }
                    }
                    // resize collisionbox
                    if os_input.key_pressed(VirtualKeyCode::LBracket) {
                        self.package.resize_fighter_colboxes(fighter, action, frame, &self.selector.colboxes, -0.1);
                    }
                    if os_input.key_pressed(VirtualKeyCode::RBracket) {
                        self.package.resize_fighter_colboxes(fighter, action, frame, &self.selector.colboxes, 0.1);
                    }
                    // meld link collisionboxes
                    if os_input.key_pressed(VirtualKeyCode::Z) {
                        // TODO
                    }
                    // simple link collisionboxes
                    if os_input.key_pressed(VirtualKeyCode::X) {
                        // TODO
                    }
                    // unlink collisionboxes
                    if os_input.key_pressed(VirtualKeyCode::C) {
                        // TODO
                    }
                    if os_input.key_pressed(VirtualKeyCode::Comma) {
                        self.package.fighter_colboxes_send_to_front(fighter, action, frame, &self.selector.colboxes)
                    }
                    if os_input.key_pressed(VirtualKeyCode::Period) {
                        self.package.fighter_colboxes_send_to_back(fighter, action, frame, &self.selector.colboxes)
                    }

                    // single click collisionbox selection
                    if os_input.mouse_pressed(0) {
                        if let Some((m_x, m_y)) = os_input.game_mouse(&self.camera) {
                            let (player_x, player_y) = self.players[player].bps_xy(&self.players, &self.package.fighters, &self.stage.platforms);

                            if !(os_input.held_shift() || os_input.held_alt()) {
                                self.selector.colboxes = HashSet::new();
                            }
                            let frame = &self.package.fighters[fighter].actions[action].frames[frame];
                            let frame = self.players[player].relative_frame(frame);
                            for (i, colbox) in frame.colboxes.iter().enumerate() {
                                let hit_x = colbox.point.0 + player_x;
                                let hit_y = colbox.point.1 + player_y;

                                let distance = ((m_x - hit_x).powi(2) + (m_y - hit_y).powi(2)).sqrt();
                                if distance < colbox.radius {
                                    self.selector.colboxes.remove(&i);
                                    if !os_input.held_alt() {
                                        self.selector.colboxes.insert(i);
                                    }
                                }
                            }
                        }

                        // Select topmost colbox
                        if os_input.held_control() {
                            let mut selector_vec: Vec<usize> = self.selector.colboxes.iter().cloned().collect();
                            selector_vec.sort();
                            selector_vec.reverse();
                            selector_vec.truncate(1);
                            self.selector.colboxes = selector_vec.into_iter().collect();
                        }
                    }

                    // begin multiple collisionbox selection
                    if os_input.mouse_pressed(1) {
                        if let Some(mouse) = os_input.game_mouse(&self.camera) {
                            self.selector.start(mouse);
                        }
                    }

                    // complete multiple collisionbox selection
                    if let Some(selection) = self.selector.point {
                        let (x1, y1) = selection;
                        if os_input.mouse_released(1) {
                            if let Some((x2, y2)) = os_input.game_mouse(&self.camera) {
                                if !(os_input.held_shift() || os_input.held_alt()) {
                                    self.selector.colboxes = HashSet::new();
                                }
                                let (player_x, player_y) = self.players[player].bps_xy(&self.players, &self.package.fighters, &self.stage.platforms);
                                let frame = &self.package.fighters[fighter].actions[action].frames[frame];
                                let frame = self.players[player].relative_frame(frame);

                                for (i, colbox) in frame.colboxes.iter().enumerate() {
                                    let hit_x = colbox.point.0 + player_x;
                                    let hit_y = colbox.point.1 + player_y;

                                    let x_check = (hit_x > x1 && hit_x < x2) || (hit_x > x2 && hit_x < x1);
                                    let y_check = (hit_y > y1 && hit_y < y2) || (hit_y > y2 && hit_y < y1);
                                    if x_check && y_check {
                                        self.selector.colboxes.remove(&i);
                                        if !os_input.held_alt() {
                                            self.selector.colboxes.insert(i);
                                        }
                                    }
                                }
                                self.selector.point = None;
                            }
                        }
                    }
                }
                self.selector.mouse = os_input.game_mouse(&self.camera); // hack to access mouse during render call, dont use this otherwise
            },
            Edit::Player (player) => {
                self.set_debug(os_input, player);
            },
            Edit::Stage => { },
        }
    }


    // TODO: Shift to apply to all players
    // TODO: F09 - load preset from player profile
    // TODO: F10 - save preset to player profile
    fn set_debug(&mut self, os_input: &OsInput, player: usize) {
        {
            let debug = &mut self.debug_players[player];

            if os_input.key_pressed(VirtualKeyCode::F1) {
                debug.physics = !debug.physics;
            }
            if os_input.key_pressed(VirtualKeyCode::F2) {
                if os_input.held_shift() {
                    debug.input_diff = !debug.input_diff;
                }
                else {
                    debug.input = !debug.input;
                }
            }
            if os_input.key_pressed(VirtualKeyCode::F3) {
                debug.action = !debug.action;
            }
            if os_input.key_pressed(VirtualKeyCode::F4) {
                debug.frame = !debug.frame;
            }
            if os_input.key_pressed(VirtualKeyCode::F5) {
                debug.stick_vector = !debug.stick_vector;
            }
            if os_input.key_pressed(VirtualKeyCode::F6) {
                debug.c_stick_vector = !debug.c_stick_vector;
            }
            if os_input.key_pressed(VirtualKeyCode::F7) {
                debug.di_vector = !debug.di_vector;
            }
            if os_input.key_pressed(VirtualKeyCode::F8) {
                debug.ecb = !debug.ecb;
            }
            if os_input.key_pressed(VirtualKeyCode::F9) {
                debug.fighter = match debug.fighter {
                    RenderFighter::Normal => {
                        RenderFighter::Debug
                    }
                    RenderFighter::Debug => {
                        RenderFighter::None
                    }
                    RenderFighter::None => {
                        RenderFighter::Normal
                    }
                };
            }
            if os_input.key_pressed(VirtualKeyCode::F10) {
                debug.cam_area = !debug.cam_area;
            }
        }
        if os_input.key_pressed(VirtualKeyCode::F11) {
            self.debug_players[player] = DebugPlayer {
                physics:        true,
                input:          true,
                input_diff:     true,
                action:         true,
                frame:          true,
                stick_vector:   true,
                c_stick_vector: true,
                di_vector:      true,
                ecb:            true,
                fighter:        RenderFighter::Debug,
                cam_area:       true,
            }
        }
        if os_input.key_pressed(VirtualKeyCode::F12) {
            self.debug_players[player] = DebugPlayer::default();
        }
    }

    /// next frame is advanced by using the input history on the current frame
    // TODO: Allow choice between using input history and game history
    fn step_replay_forwards(&mut self, input: &mut Input, os_input: &OsInput) {
        if self.current_frame < input.last_frame() {
            let player_inputs = &input.players(self.current_frame);
            self.step_game(player_inputs);

            // flow controls
            if os_input.key_pressed(VirtualKeyCode::H) {
                self.state = GameState::ReplayBackwards;
            }
            if input.start_pressed() || os_input.key_pressed(VirtualKeyCode::Space) || os_input.key_pressed(VirtualKeyCode::Return) {
                self.state = GameState::Paused;
            }
            self.current_frame += 1;
            self.update_frame();
        }
        else {
            self.state = GameState::Paused;
        }
    }

    /// Immediately jumps to the previous frame in history
    fn step_replay_backwards(&mut self, input: &mut Input, os_input: &OsInput) {
        if self.current_frame > 0 {
            let jump_to = self.current_frame - 1;
            self.players = self.player_history.get(jump_to).unwrap().clone();
            self.current_frame = jump_to;
            self.update_frame();
        }
        else {
            self.state = GameState::Paused;
        }

        // flow controls
        if os_input.key_pressed(VirtualKeyCode::L) {
            self.state = GameState::ReplayForwards;
        }
        else if input.start_pressed() || os_input.key_pressed(VirtualKeyCode::Space) || os_input.key_pressed(VirtualKeyCode::Return) {
            self.state = GameState::Paused;
            self.update_frame();
        }
    }

    /// Jump to the saved frame in history
    fn jump_frame(&mut self) {
        let frame = self.saved_frame;
        if (frame+1) < self.player_history.len() {
            self.players = self.player_history.get(frame).unwrap().clone();

            self.current_frame = frame;
            self.update_frame();
        }
    }

    fn get_seed(&self) -> Vec<usize> {
        let mut seed = self.init_seed.clone();
        seed.push(self.current_frame);
        seed
    }

    fn step_game(&mut self, player_input: &Vec<PlayerInput>) {
        {
            let mut rng = StdRng::from_seed(&self.get_seed());

            // To synchronize player stepping, we step through player logic in stages (action logic, physics logic, collision logic)
            // Modified players are copied from the previous stage so that every player perceives themselves as being stepped first, within that stage.

            // step each player action
            let mut action_players: Vec<Player> = vec!();
            for (i, player) in self.players.iter().enumerate() {
                let mut player = player.clone();
                let input = &player_input[self.selected_controllers[i]];
                player.action_hitlag_step(input, &self.players, &self.package.fighters, &self.stage.platforms, &mut rng);
                action_players.push(player);
            }

            // step each player physics
            let mut physics_players: Vec<Player> = vec!();
            for (i, player) in action_players.iter().enumerate() {
                let mut player = player.clone();
                let input = &player_input[self.selected_controllers[i]];
                player.physics_step(input, &action_players, &self.package.fighters, &self.stage, self.current_frame, self.package.rules.goal.clone());
                physics_players.push(player);
            }

            // check for hits and run hit logic
            let mut collision_players: Vec<Player> = vec!();
            let collision_results = collision_check(&physics_players, &self.package.fighters, &self.stage.platforms);
            for (i, player) in physics_players.iter().enumerate() {
                let mut player = player.clone();
                player.step_collision(&physics_players, &self.package.fighters, &self.stage.platforms, &collision_results[i]);
                collision_players.push(player);
            }

            self.players = collision_players;
        }

        match self.package.rules.goal {
            Goal::Time => {
                if (self.current_frame / 60) as u64 > self.package.rules.time_limit {
                    self.state = self.generate_game_results();
                }
            }
            Goal::Stock => {
                if (self.current_frame / 60) as u64 > self.package.rules.time_limit
                || self.players.iter().filter(|x| x.stocks > 0).count() == 1 {
                    self.state = self.generate_game_results();
                }
            }
            Goal::Training => { }
        }

        self.update_frame();
    }

    pub fn generate_game_results(&self) -> GameState {
        let player_results: Vec<PlayerResult> = self.players.iter().map(|x| x.result()).collect();
        let places: Vec<usize> = match self.package.rules.goal {
            Goal::Training => {
                iter::repeat(0).take(self.players.len()).collect()
            }
            Goal::Stock => {
                // most stocks remaining wins
                // tie-breaker:
                //  * if both eliminated: who lost their last stock last wins
                //  * if both alive:      lowest percentage wins
                let mut player_results_i: Vec<(usize, &PlayerResult)> = player_results.iter().enumerate().collect();
                player_results_i.sort_by(
                    |a_set, b_set| {
                        let a = a_set.1;
                        let b = b_set.1;
                        let a_deaths = a.deaths.len();
                        let b_deaths = b.deaths.len();
                        a_deaths.cmp(&b_deaths).then(
                            if a_deaths == 0 {
                                if let Some(death_a) = a.deaths.last() {
                                    if let Some(death_b) = b.deaths.last() {
                                        death_a.frame.cmp(&death_b.frame)
                                    }
                                    else {
                                        Ordering::Equal
                                    }
                                }
                                else {
                                    Ordering::Equal
                                }
                            }
                            else {
                                a.final_damage.unwrap().partial_cmp(&b.final_damage.unwrap()).unwrap_or(Ordering::Equal)
                            }
                        )
                    }
                );
                player_results_i.iter().map(|x| x.0).collect()
            }
            Goal::Time => {
                // highest kills wins
                // tie breaker: least deaths wins
                let mut player_results_i: Vec<(usize, &PlayerResult)> = player_results.iter().enumerate().collect();
                player_results_i.sort_by(
                    |a_set, b_set| {
                        // Repopulating kill lists every frame shouldnt be too bad
                        let a_kills: Vec<usize> = vec!(); // TODO: populate
                        let b_kills: Vec<usize> = vec!(); // TODO: populate
                        let a = a_set.1;
                        let b = b_set.1;
                        let a_kills = a_kills.len();
                        let b_kills = b_kills.len();
                        let a_deaths = a.deaths.len();
                        let b_deaths = b.deaths.len();
                        b_kills.cmp(&a_kills).then(a_deaths.cmp(&b_deaths))
                    }
                );
                player_results_i.iter().map(|x| x.0).collect()
            }
        };

        let mut game_results: Vec<GameResult> = vec!();
        for (i, player_result) in player_results.iter().enumerate() {
            let lcancel_percent = if player_result.lcancel_attempts == 0 {
                100.0
            }
            else {
                player_result.lcancel_success as f32 / player_result.lcancel_attempts as f32
            };
            game_results.push(GameResult {
                fighter:         player_result.ended_as_fighter.clone().unwrap(),
                controller:      self.selected_controllers[i],
                place:           places[i],
                kills:           vec!(), // TODO
                deaths:          player_result.deaths.clone(),
                lcancel_percent: lcancel_percent,
            });
        }
        game_results.sort_by_key(|x| x.place);
        GameState::ToResults (game_results)
    }

    fn debug_output(&mut self, input: &Input) {
        let frame = self.current_frame;
        let player_inputs = &input.players(frame);

        println!("\n-------------------------------------------");
        println!("Frame: {}    state: {:?}", frame, self.state);

        for (i, player) in self.players.iter().enumerate() {
            let fighter = &self.package.fighters[self.players[i].fighter.as_ref()];
            let player_input = &player_inputs[i];
            let debug_player = &self.debug_players[i];
            player.debug_print(fighter, player_input, debug_player, i);
        }
    }

    /// Call this whenever a player's frame is changed, this can be from:
    /// *   the fighter's frame data is changed
    /// *   the player now refers to a different frame.
    fn update_frame(&mut self) {
        self.selector = Default::default();
        self.debug_output_this_step = true;
    }

    pub fn render(&self) -> RenderGame {
        let mut entities = vec!();

        // stage areas
        entities.push(RenderEntity::Area(area_to_render(&self.stage.camera)));
        entities.push(RenderEntity::Area(area_to_render(&self.stage.blast)));

        for (i, player) in self.players.iter().enumerate() {
            let mut selected_colboxes = HashSet::new();
            let mut fighter_selected = false;
            let mut player_selected = false;
            if let GameState::Paused = self.state {
                match self.edit {
                    Edit::Fighter (player) => {
                        if i == player {
                            selected_colboxes = self.selector.colboxes.clone();
                            fighter_selected = true;
                        }
                    },
                    Edit::Player (player) => {
                        player_selected = player == i;
                    },
                    _ => { },
                }
            }

            let debug = self.debug_players[i].clone();
            if debug.cam_area {
                let cam_area = &player.cam_area(&self.stage.camera, &self.players, &self.package.fighters, &self.stage.platforms);
                entities.push(RenderEntity::Area(area_to_render(cam_area)));
            }

            let color = graphics::get_controller_color(self.selected_controllers[i]);
            let player_render = player.render(color, selected_colboxes, fighter_selected, player_selected, debug, &self.players, &self.package.fighters, &self.stage.platforms);
            entities.push(RenderEntity::Player(player_render));
        }

        // render selector box
        if let Some(point) = self.selector.point {
            if let Some(mouse) = self.selector.mouse {
                let render_box = RenderRect {
                    p1: point,
                    p2: mouse,
                };
                entities.push(RenderEntity::Selector(render_box));
            }
        }

        RenderGame {
            stage:    self.selected_stage.clone(),
            entities: entities,
            state:    self.state.clone(),
            camera:   self.camera.clone(),
        }
    }

    pub fn graphics_message(&mut self) -> GraphicsMessage {
        GraphicsMessage {
            package_updates: self.package.updates(),
            render: Render::Game (self.render())
        }
    }

    pub fn reclaim(self) -> (Package, Config) {
        (self.package, self.config)
    }
}

fn area_to_render(area: &Area) -> RenderRect {
    RenderRect {
        p1: (area.left,  area.bot),
        p2: (area.right, area.top)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
pub enum GameState {
    Local,
    ReplayForwards,
    ReplayBackwards,
    Netplay,
    Paused, // Only Local, ReplayForwards and ReplayBackwards can be paused
    ToResults (Vec<GameResult>), // Both Local and Netplay end at ToResults
    ToCSS,
}

impl Default for GameState {
    fn default() -> GameState {
        GameState::Paused
    }
}

#[derive(Clone, Serialize, Deserialize, Node)]
pub enum Edit {
    Fighter (usize), // index to player
    Player  (usize),
    Stage
}

impl Default for Edit {
    fn default() -> Edit {
        Edit::Stage
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Node)]
pub struct Selector {
    colboxes: HashSet<usize>,
    moving:   bool,
    point:    Option<(f32, f32)>, // selector starting point
    mouse:    Option<(f32, f32)>, // used to know mouse point during render
}

impl Selector {
    fn colboxes_vec(&self) -> Vec<usize> { // TODO: LOL
        let mut result:Vec<usize> = vec!();
        for value in &self.colboxes {
            result.push(*value);
        }
        result
    }

    pub fn start(&mut self, mouse: (f32, f32)) {
        self.point  = Some(mouse);
        self.moving = false;
        self.mouse  = None;
    }
}

pub struct RenderGame {
    pub stage:    String,
    pub entities: Vec<RenderEntity>,
    pub state:    GameState,
    pub camera:   Camera,
}

pub enum RenderEntity {
    Player    (RenderPlayer),
    Selector  (RenderRect),
    Area      (RenderRect),
}

#[derive(Clone)]
pub struct GameSetup {
    pub init_seed:      Vec<usize>,
    pub input_history:  Vec<Vec<ControllerInput>>,
    pub player_history: Vec<Vec<Player>>,
    pub stage_history:  Vec<Stage>,
    pub controllers:    Vec<usize>,
    pub fighters:       Vec<String>,
    pub stage:          String,
    pub state:          GameState,
}

impl GameSetup {
    pub fn gen_seed() -> Vec<usize> {
        vec!(Local::now().timestamp() as usize)
    }
}
