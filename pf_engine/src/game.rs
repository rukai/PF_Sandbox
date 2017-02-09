use ::input::{Input, PlayerInput, ControllerInput};
use ::os_input::OsInput;
use ::package::Package;
use ::player::{Player, RenderPlayer, DebugPlayer, RenderFighter};
use ::fighter::{ActionFrame, CollisionBox, LinkType};
use ::camera::Camera;
use ::stage::Area;
use ::graphics::{GraphicsMessage, Render};
use ::collision::collision_check;

use ::std::collections::HashSet;

use winit::VirtualKeyCode;
use treeflection::{Node, NodeRunner, NodeToken};

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct Game {
    pub package:                Package,
    pub state:                  GameState,
    pub player_history:         Vec<Vec<Player>>,
    pub current_frame:          usize,
    pub saved_frame:            usize,
    pub players:                Vec<Player>,
    pub debug_players:          Vec<DebugPlayer>,
    pub selected_controllers:   Vec<usize>,
    pub selected_fighters:      Vec<usize>,
    pub selected_stage:         usize,
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
    pub fn new(package: Package, selected_fighters: Vec<usize>, selected_stage: usize, netplay: bool, selected_controllers: Vec<usize>) -> Game {
        // generate players
        let mut players:       Vec<Player>      = vec!();
        let mut debug_players: Vec<DebugPlayer> = vec!();
        {
            let spawn_points = &package.stages[selected_stage].spawn_points;
            let respawn_points = &package.stages[selected_stage].respawn_points;
            for (i, _) in selected_controllers.iter().enumerate() {
                // Stages can have less spawn points then players
                let spawn = spawn_points[i % spawn_points.len()].clone();
                let respawn = respawn_points[i % respawn_points.len()].clone();
                players.push(Player::new(spawn, respawn, package.rules.stock_count));
                debug_players.push(Default::default());
            }
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
            package:                package,
            state:                  if netplay { GameState::Netplay } else { GameState::Local },
            player_history:         vec!(),
            current_frame:          0,
            saved_frame:            0,
            players:                players,
            debug_players:          debug_players,
            selected_controllers:   selected_controllers,
            selected_fighters:      filled_fighters,
            selected_stage:         selected_stage,
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
                GameState::Paused          => { self.step_pause(input, &os_input); }
                GameState::Results         => { panic!("No more steps should occur after state is set to Results") }
            }
            {
                let stage = &self.package.stages[self.selected_stage];
                self.camera.update(os_input, &self.players, stage);
            }

            if self.debug_output_this_step {
                self.debug_output_this_step = false;
                self.debug_output(input);
            }
        }

        // set treeflection context
        match self.edit {
            Edit::Fighter (player) => {
                let player_fighter = self.selected_fighters[player];
                let player_action = self.players[player].action as usize;
                let player_frame  = self.players[player].frame as usize;
                let player_colboxes = self.selector.colboxes_vec();

                let fighters = &mut self.package.fighters;
                fighters.set_context(player_fighter);

                let actions = &mut fighters[player_fighter].actions;
                actions.set_context(player_action);

                let frames = &mut actions[player_action].frames;
                frames.set_context(player_frame);

                let colboxes = &mut frames[player_frame].colboxes;
                colboxes.set_context_vec(player_colboxes);
            }
            _ => { }
        }

        self.package.stages.set_context(self.selected_stage);

        self.state.clone()
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
        }
        else if os_input.key_pressed(VirtualKeyCode::Key2) && players_len >= 2 {
            if os_input.held_shift() {
                self.edit = Edit::Player (1);
            }
            else {
                self.edit = Edit::Fighter (1);
            }
        }
        else if os_input.key_pressed(VirtualKeyCode::Key3) && players_len >= 3 {
            if os_input.held_shift() {
                self.edit = Edit::Player (2);
            }
            else {
                self.edit = Edit::Fighter (2);
            }
        }
        else if os_input.key_pressed(VirtualKeyCode::Key4) && players_len >= 4 {
            if os_input.held_shift() {
                self.edit = Edit::Player (3);
            }
            else {
                self.edit = Edit::Fighter (3);
            }
        }

        // modify package
        if os_input.key_pressed(VirtualKeyCode::E) {
            self.package.save();
        }
        if os_input.key_pressed(VirtualKeyCode::R) {
            self.package.load();
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
        else if input.start_pressed() || os_input.key_pressed(VirtualKeyCode::Return) {
            self.state = GameState::Local;
        }

        match self.edit {
            Edit::Fighter (player) => {
                let fighter = self.selected_fighters[player];
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
                            for (i, any_player) in (&mut *self.players).iter_mut().enumerate() {
                                if self.selected_fighters[i] == fighter && any_player.action as usize == action
                                    && any_player.frame as usize == self.package.fighters[fighter].actions[action].frames.len() {
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
                                let p_x = player.bps_x;
                                let p_y = player.bps_y;

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
                            let player_x = self.players[player].bps_x;
                            let player_y = self.players[player].bps_y;

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
                                let player_x = self.players[player].bps_x;
                                let player_y = self.players[player].bps_y;
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
                match debug.fighter {
                    RenderFighter::Normal => {
                    }
                    RenderFighter::Debug => {
                    }
                    RenderFighter::None => {
                    }
                }
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

    fn step_game(&mut self, player_input: &Vec<PlayerInput>) {
        {
            let stage = &self.package.stages[self.selected_stage];

            // step each player
            for (i, player) in (&mut *self.players).iter_mut().enumerate() {
                let fighter = &self.package.fighters[self.selected_fighters[i]];
                let input = &player_input[self.selected_controllers[i]];
                player.step(input, fighter, stage);
            }

            // check collisions
            let collision_results = collision_check(&self.players, &self.package.fighters, &self.selected_fighters);
            for (i, player) in (&mut *self.players).iter_mut().enumerate() {
                let fighter = &self.package.fighters[self.selected_fighters[i]];
                player.step_collision(fighter, &collision_results[i]);
            }
        }

        // handle timer
        if (self.current_frame / 60) as u64 > self.package.rules.time_limit {
            self.state = GameState::Results;
        }

        // handle no stocks left
        for player in &self.players {
            if player.stocks <= 0 {
                self.state = GameState::Results;
            }
        }

        self.update_frame();
    }

    fn debug_output(&mut self, input: &Input) {
        let frame = self.current_frame;
        let player_inputs = &input.players(frame);

        println!("\n-------------------------------------------");
        println!("Frame: {}    state: {:?}", frame, self.state);

        for (i, player) in self.players.iter().enumerate() {
            let fighter = &self.package.fighters[self.selected_fighters[i]];
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
                let cam_area = &player.cam_area(&self.package.stages[self.selected_stage].camera);
                entities.push(RenderEntity::Area(area_to_render(cam_area)));
            }
            let player_colors: Vec<[f32; 4]> = vec!(
                [0.0, 90.0/255.0, 224.0/255.0, 1.0],
                [239.0/255.0, 100.0/255.0, 0.0, 1.0],
                [1.0, 0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0, 1.0],
            );

            entities.push(RenderEntity::Player(player.render(player_colors[i], self.selected_fighters[i], selected_colboxes, fighter_selected, player_selected, debug)));
        }

        // stage areas
        let stage = &self.package.stages[self.selected_stage];
        entities.push(RenderEntity::Area(area_to_render(&stage.camera)));
        entities.push(RenderEntity::Area(area_to_render(&stage.blast)));

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
    Paused,  // Only Local, ReplayForwards and ReplayBackwards can be paused
    Results, // Both Local and Netplay end at Results
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
    pub entities: Vec<RenderEntity>,
    pub state:    GameState,
    pub camera:   Camera,
}

pub enum RenderEntity {
    Player   (RenderPlayer),
    Selector (RenderRect),
    Area     (RenderRect),
}

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct RenderRect {
    pub p1: (f32, f32),
    pub p2: (f32, f32),
}

