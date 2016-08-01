use ::input::{Input, PlayerInput};
use ::os_input::OsInput;
use ::package::Package;
use ::player::{Player, RenderPlayer, DebugPlayer};
use ::fighter::{ActionFrame, CollisionBox, LinkType};
use ::camera::Camera;
use ::stage::{Area};

use ::std::collections::HashSet;

use glium::glutin::VirtualKeyCode;

pub struct Game {
    state:                  GameState,
    player_history:         Vec<Vec<Player>>,
    current_frame:          usize,
    saved_frame:            usize,
    players:                Vec<Player>,
    selected_controllers:   Vec<usize>,
    selected_fighters:      Vec<usize>,
    selected_stage:         usize,
    edit:                   Edit,
    debug_output_this_step: Option<usize>,
    selector:               Selector,
    copied_frame:           Option<ActionFrame>,
    camera:                 Camera,
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
            state:                  if netplay { GameState::Netplay } else { GameState::Local },
            player_history:         vec!(),
            current_frame:          0,
            saved_frame:            0,
            players:                players,
            selected_controllers:   selected_controllers,
            selected_fighters:      filled_fighters,
            selected_stage:         selected_stage,
            edit:                   Edit::Stage,
            debug_output_this_step: None,
            selector:               Default::default(),
            copied_frame:           None,
            camera:                 Camera::new(),
        }
    }

    pub fn step(&mut self, package: &mut Package, input: &mut Input, os_input: &OsInput) {
        match self.state.clone() {
            GameState::Local           => { self.step_local(package, input, os_input); },
            GameState::Netplay         => { self.step_netplay(package, input); },
            GameState::Results         => { self.step_results(); },
            GameState::ReplayForwards  => { self.step_replay_forwards(package, input, os_input); },
            GameState::ReplayBackwards => { self.step_replay_backwards(input, os_input); },
            GameState::Paused          => { self.step_pause(package, input, &os_input); },
        }

        let stage = &package.stages[self.selected_stage];
        self.camera.update(os_input, &self.players, stage);

        if let Some(frame) = self.debug_output_this_step {
            self.debug_output_this_step = None;
            self.debug_output(package, input, frame);
        }
    }

    fn step_local(&mut self, package: &Package, input: &mut Input, os_input: &OsInput) {
        // erase any future history
        for _ in (self.current_frame+1)..(self.player_history.len()) {
            self.player_history.pop();
        }

        // run game loop
        input.game_update(self.current_frame);
        let player_inputs = &input.players(self.current_frame);
        self.step_game(package, player_inputs);

        self.player_history.push(self.players.clone());
        if input.start_pressed() || os_input.key_pressed(VirtualKeyCode::Space) {
            self.set_paused();
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

    fn step_pause(&mut self, package: &mut Package, input: &mut Input, os_input: &OsInput) {
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
            package.save();
        }
        if os_input.key_pressed(VirtualKeyCode::R) {
            package.load();
        }

        // game flow control
        if os_input.key_pressed(VirtualKeyCode::J) {
            self.step_replay_backwards(input, os_input);
        }
        else if os_input.key_pressed(VirtualKeyCode::K) {
            self.step_replay_forwards(package, input, os_input);
        }
        else if os_input.key_pressed(VirtualKeyCode::H) {
            self.state = GameState::ReplayBackwards;
        }
        else if os_input.key_pressed(VirtualKeyCode::L) {
            self.state = GameState::ReplayForwards;
        }
        else if os_input.key_pressed(VirtualKeyCode::Space) {
            self.current_frame += 1;
            self.step_local(package, input, os_input);
        }
        else if os_input.key_pressed(VirtualKeyCode::U) {
            // TODO: Invalidate saved_frame when the frame it refers to is deleted.
            self.saved_frame = self.current_frame;
        }
        else if os_input.key_pressed(VirtualKeyCode::I) {
            self.jump_frame();
        }
        else if input.start_pressed() {
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
                    package.move_fighter_colboxes(fighter, action, frame, &self.selector.colboxes, distance);

                    if os_input.mouse_pressed(0) {
                        self.selector = Default::default();
                    }
                }
                else {
                    // copy frame
                    if os_input.key_pressed(VirtualKeyCode::V) {
                        let frame = package.fighters[fighter].action_defs[action].frames[frame].clone();
                        self.copied_frame = Some(frame);
                    }
                    // paste frame
                    if os_input.key_pressed(VirtualKeyCode::B) {
                        if let Some(ref action_frame) = self.copied_frame {
                            package.insert_fighter_frame(fighter, action, frame, action_frame.clone());
                            self.debug_output_this_step = Some(self.current_frame);
                        }
                    }

                    // new frame
                    if os_input.key_pressed(VirtualKeyCode::M) {
                        package.new_fighter_frame(fighter, action, frame);
                        self.players[player].frame += 1;
                        self.debug_output_this_step = Some(self.current_frame);
                    }
                    // delete frame
                    if os_input.key_pressed(VirtualKeyCode::N) {
                        if package.delete_fighter_frame(fighter, action, frame) {
                            // Correct any players that are now on a nonexistent frame due to the frame deletion.
                            // This is purely to stay on the same action for usability.
                            // The player itself must handle being on a frame that has been deleted in order for replays to work.
                            for (i, any_player) in (&mut *self.players).iter_mut().enumerate() {
                                if self.selected_fighters[i] == fighter && any_player.action as usize == action
                                    && any_player.frame as usize == package.fighters[fighter].action_defs[action].frames.len() {
                                    any_player.frame -= 1;
                                }
                            }
                            self.debug_output_this_step = Some(self.current_frame);
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
                        package.delete_fighter_colboxes(fighter, action, frame, &self.selector.colboxes);

                        self.selector = Default::default();
                    }
                    // add collisionbox
                    if os_input.key_pressed(VirtualKeyCode::F) {
                        if let Some((m_x, m_y)) = os_input.game_mouse(&self.camera) {
                            let player = &self.players[player];
                            let p_x = player.bps_x;
                            let p_y = player.bps_y;

                            let point = (player.relative_f(m_x - p_x), m_y - p_y);
                            let new_colbox = CollisionBox::new(point);
                            let link_type = match os_input.held_shift() {
                                true  => { LinkType::Simple },
                                false => { LinkType::Meld }
                            };

                            let selected = package.append_fighter_colbox(fighter, action, frame, new_colbox, &self.selector.colboxes, link_type);
                            self.selector.colboxes = HashSet::new();
                            self.selector.colboxes.insert(selected);
                        }
                    }
                    // resize collisionbox
                    if os_input.key_pressed(VirtualKeyCode::G) {
                        // TODO
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

                    // single collisionbox selection
                    if os_input.mouse_pressed(0) {
                        if let Some((m_x, m_y)) = os_input.game_mouse(&self.camera) {
                            let player_x = self.players[player].bps_x;
                            let player_y = self.players[player].bps_y;

                            if !os_input.held_shift() {
                                self.selector.colboxes = HashSet::new();
                            }
                            let frame = &package.fighters[fighter].action_defs[action].frames[frame];
                            let frame = self.players[player].relative_frame(frame);
                            for (i, colbox) in frame.colboxes.iter().enumerate() {
                                let hit_x = colbox.point.0 + player_x;
                                let hit_y = colbox.point.1 + player_y;

                                let distance = ((m_x - hit_x).powi(2) + (m_y - hit_y).powi(2)).sqrt();
                                if distance < colbox.radius {
                                    self.selector.colboxes.insert(i);
                                    break;
                                }
                            }
                        }
                    }

                    // begin multiple collisionbox selection
                    if os_input.mouse_pressed(1) {
                        self.selector = Default::default();
                        if let Some(mouse) = os_input.game_mouse(&self.camera) {
                            self.selector.point = Some(mouse);
                        }
                    }

                    // complete multiple collisionbox selection
                    if let Some(selection) = self.selector.point {
                        let (x1, y1) = selection;
                        if os_input.mouse_released(1) {
                            if let Some((x2, y2)) = os_input.game_mouse(&self.camera) {
                                if !os_input.held_shift() {
                                    self.selector.colboxes = HashSet::new();
                                }
                                let player_x = self.players[player].bps_x;
                                let player_y = self.players[player].bps_y;
                                let frame = &package.fighters[fighter].action_defs[action].frames[frame];
                                let frame = self.players[player].relative_frame(frame);

                                for (i, colbox) in frame.colboxes.iter().enumerate() {
                                    let hit_x = colbox.point.0 + player_x;
                                    let hit_y = colbox.point.1 + player_y;

                                    let x_check = (hit_x > x1 && hit_x < x2) || (hit_x > x2 && hit_x < x1);
                                    let y_check = (hit_y > y1 && hit_y < y2) || (hit_y > y2 && hit_y < y1);
                                    if x_check && y_check {
                                        self.selector.colboxes.insert(i);
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
            let debug = &mut self.players[player].debug;

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
                debug.player = !debug.player;
            }
            if os_input.key_pressed(VirtualKeyCode::F9) {
                debug.no_fighter = !debug.no_fighter;
            }
            if os_input.key_pressed(VirtualKeyCode::F10) {
                debug.cam_area = !debug.cam_area;
            }
        }
        if os_input.key_pressed(VirtualKeyCode::F11) {
            self.players[player].debug = DebugPlayer {
                physics:        true,
                input:          true,
                input_diff:     true,
                action:         true,
                frame:          true,
                stick_vector:   true,
                c_stick_vector: true,
                di_vector:      true,
                player:         true,
                no_fighter:     true,
                cam_area: true,
            }
        }
        if os_input.key_pressed(VirtualKeyCode::F12) {
            self.players[player].debug = DebugPlayer::default();
        }
    }

    /// next frame is advanced by using the input history on the current frame
    // TODO: Allow choice between using input history and game history
    fn step_replay_forwards(&mut self, package: &Package, input: &mut Input, os_input: &OsInput) {
        if self.current_frame < input.last_frame() {
            let player_inputs = &input.players(self.current_frame);
            self.step_game(package, player_inputs);

            // flow controls
            if os_input.key_pressed(VirtualKeyCode::H) {
                self.state = GameState::ReplayBackwards;
            }
            if input.start_pressed() || os_input.key_pressed(VirtualKeyCode::Space) {
                self.set_paused();
            }
            self.current_frame += 1;
        }
        else {
            self.set_paused();
        }
    }

    /// Immediately jumps to the previous frame in history
    fn step_replay_backwards(&mut self, input: &mut Input, os_input: &OsInput) {
        if self.current_frame > 0 {
            let jump_to = self.current_frame - 1;
            self.players = self.player_history.get(jump_to).unwrap().clone();

            self.current_frame = jump_to;
            self.debug_output_this_step = Some(jump_to);
        }
        else {
            self.set_paused();
        }

        // flow controls
        if os_input.key_pressed(VirtualKeyCode::L) {
            self.state = GameState::ReplayForwards;
        }
        else if input.start_pressed() || os_input.key_pressed(VirtualKeyCode::Space) {
            self.set_paused();
        }
    }

    /// Jump to the saved frame in history
    fn jump_frame(&mut self) {
        let frame = self.saved_frame;
        if (frame+1) < self.player_history.len() {
            self.players = self.player_history.get(frame).unwrap().clone();

            self.current_frame = frame;
            self.debug_output_this_step = Some(frame);
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

        self.debug_output_this_step = Some(self.current_frame);
    }

    fn debug_output(&mut self, package: &Package, input: &Input, frame: usize) {
        let player_inputs = &input.players(frame);

        println!("\n-------------------------------------------");
        println!("Frame: {}    state: {:?}", frame, self.state);

        for (i, player) in self.players.iter().enumerate() {
            let fighter = &package.fighters[self.selected_fighters[i]];
            let player_input = &player_inputs[i];
            player.debug_print(fighter, player_input, i);
        }
    }

    fn step_results(&mut self) {
    }

    fn set_paused(&mut self) {
        self.state = GameState::Paused;
        self.selector = Default::default();
    }

    pub fn render(&self, package: &Package) -> RenderGame {
        let mut entities = vec!();
        for (i, player) in self.players.iter().enumerate() {

            let mut selected_colboxes = HashSet::new();
            let mut selected = false;
            if let GameState::Paused = self.state {
                match self.edit {
                    Edit::Fighter (player) => {
                        if i == player {
                            selected_colboxes = self.selector.colboxes.clone();
                            // TODO: color outline green
                        }
                    },
                    Edit::Player (player) => {
                        selected = player == i;
                    },
                    _ => { },
                }
            }
            entities.push(RenderEntity::Player(player.render(self.selected_fighters[i], selected_colboxes, selected)));

            if player.debug.cam_area {
                let cam_area = &player.cam_area(&package.stages[self.selected_stage].camera);
                entities.push(RenderEntity::Area(area_to_render(cam_area)));
            }
        }

        // stage areas
        let stage = &package.stages[self.selected_stage];
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
}

fn area_to_render(area: &Area) -> RenderRect {
    RenderRect {
        p1: (area.left,  area.bot),
        p2: (area.right, area.top)
    }
}

#[derive(Debug, Clone)]
pub enum GameState {
    Local,
    ReplayForwards,
    ReplayBackwards,
    Netplay,
    Paused,  // Only Local, ReplayForwards and ReplayBackwards can be paused
    Results, // Both Local and Netplay end at Results
}

pub enum Edit {
    Fighter (usize), // index to player
    Player  (usize),
    Stage
}

#[derive(Debug, Clone, Default)]
pub struct Selector {
    colboxes: HashSet<usize>,
    moving:   bool,
    point:    Option<(f32, f32)>, // selector starting point
    mouse:    Option<(f32, f32)>, // used to know mouse point during render
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

pub struct RenderRect {
    pub p1: (f32, f32),
    pub p2: (f32, f32),
}
