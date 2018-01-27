use camera::Camera;
use collision::collision_check;
use command_line::CommandLine;
use config::Config;
use fighter::{ActionFrame, CollisionBox, LinkType, Action};
use geometry::Rect;
use graphics::{GraphicsMessage, Render, RenderType};
use input::{Input, PlayerInput, ControllerInput};
use menu::ResumeMenu;
use network::Netplay;
use os_input::OsInput;
use package::Package;
use player::{Player, RenderPlayer, DebugPlayer, StepContext};
use replays::Replay;
use replays;
use results::{GameResults, RawPlayerResult, PlayerResult};
use rules::Goal;
use stage::{Stage, DebugStage, SpawnPoint, Surface, Floor};

use rand::{StdRng, SeedableRng};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt;
use std::time::Duration;
use chrono::Local;
use enum_traits::{FromIndex, ToIndex};

use treeflection::{Node, NodeRunner, NodeToken};
use winit::VirtualKeyCode;

#[NodeActions(
    NodeAction(function="save_replay", return_string),
    NodeAction(function="reset_deadzones", return_string),
    NodeAction(function="copy_stage_to_package", return_string),
    NodeAction(function="copy_package_to_stage", return_string),
)]
#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct Game {
    pub package:                Package,
    pub config:                 Config,
    pub init_seed:              u64,
    pub state:                  GameState,
    pub player_history:         Vec<Vec<Player>>,
    pub stage_history:          Vec<Stage>,
    pub current_frame:          usize,
    pub saved_frame:            usize,
    pub stage:                  Stage,
    pub players:                Vec<Player>,
    pub debug_stage:            DebugStage,
    pub debug_players:          Vec<DebugPlayer>,
    pub selected_controllers:   Vec<usize>,
    pub selected_ais:           Vec<usize>,
    pub selected_stage:         String,
    pub edit:                   Edit,
    pub debug_output_this_step: bool,
    pub debug_lines:            Vec<String>,
    pub selector:               Selector,
    copied_frame:               Option<ActionFrame>,
    pub camera:                 Camera,
    pub tas:                    Vec<ControllerInput>,
    save_replay:                bool,
    reset_deadzones:            bool,
}

/// Frame 0 refers to the initial state of the game.
/// Any changes occur in the proceeding frames i.e. frames 1, 2, 3 ...

/// All previous frame state is used to calculate the next frame, then the current_frame is incremented.

impl Game {
    pub fn new(package: Package, config: Config, setup: GameSetup) -> Game {
        let stage = package.stages[setup.stage.as_ref()].clone();

        // generate players
        let mut players:       Vec<Player>      = vec!();
        let mut debug_players: Vec<DebugPlayer> = vec!();
        {
            for (i, player) in setup.players.iter().enumerate() {
                // Stage can have less spawn points then players
                let fighter = player.fighter.clone();
                let team = player.team;
                let spawn = stage.spawn_points[i % stage.spawn_points.len()].clone();
                players.push(Player::new(fighter, team, spawn, &package));
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
            stage:                  stage,
            players:                players,
            debug_stage:            Default::default(),
            debug_players:          debug_players,
            selected_controllers:   setup.controllers,
            selected_ais:           setup.ais,
            selected_stage:         setup.stage,
            edit:                   Edit::Stage,
            debug_output_this_step: false,
            debug_lines:            vec!(),
            selector:               Default::default(),
            copied_frame:           None,
            camera:                 Camera::new(),
            tas:                    vec!(),
            save_replay:            false,
            reset_deadzones:        false,
        }
    }

    pub fn step(&mut self, input: &mut Input, os_input: &OsInput, os_input_blocked: bool, netplay: &Netplay) -> GameState {
        if os_input.held_alt() && os_input.key_pressed(VirtualKeyCode::Return) {
            self.config.fullscreen = !self.config.fullscreen;
            self.config.save();
        }

        if self.save_replay {
            replays::save_replay(&Replay::new(self, input), &self.package);
            self.save_replay = false;
        }

        {
            let state = self.state.clone();
            match state {
                GameState::Local                 => { self.step_local(input, netplay); }
                GameState::Netplay               => { self.step_netplay(input, netplay); }
                GameState::ReplayForwards        => { self.step_replay_forwards(input, netplay); }
                GameState::ReplayBackwards       => { self.step_replay_backwards(input); }
                GameState::StepThenPause         => { self.step_local(input, netplay); self.state = GameState::Paused; }
                GameState::StepForwardThenPause  => { self.step_replay_forwards(input, netplay); self.state = GameState::Paused; }
                GameState::StepBackwardThenPause => { self.step_replay_backwards(input); self.state = GameState::Paused; }
                GameState::Paused                => { self.step_pause(input); }
                GameState::Quit (_)              => { unreachable!(); }
            }

            if !os_input_blocked {
                match state {
                    GameState::Local           => { self.step_local_os_input(os_input); }
                    GameState::ReplayForwards  => { self.step_replay_forwards_os_input(os_input); }
                    GameState::ReplayBackwards => { self.step_replay_backwards_os_input(os_input); }
                    GameState::Paused          => { self.step_pause_os_input(input, os_input, netplay); }
                    GameState::Quit (_)        => { unreachable!(); }

                    GameState::Netplay              | GameState::StepThenPause |
                    GameState::StepForwardThenPause | GameState::StepBackwardThenPause => { }
                }
                self.camera.update_os_input(os_input);
            }
            self.camera.update(os_input, &self.players, &self.package.fighters, &self.stage);

            self.generate_debug(input, netplay);
        }

        self.set_context();

        debug!("current_frame: {}", self.current_frame);
        self.state.clone()
    }

    pub fn save_replay(&mut self) -> String {
        self.save_replay = true;
        String::from("Save replay completed")
    }

    pub fn reset_deadzones(&mut self) -> String {
        self.reset_deadzones = true;
        String::from("Deadzones reset")
    }

    pub fn copy_stage_to_package(&mut self) -> String {
        self.package.stages[self.selected_stage.as_ref()] = self.stage.clone();
        String::from("Current stage state copied to package")
    }

    pub fn copy_package_to_stage(&mut self) -> String {
        self.stage = self.package.stages[self.selected_stage.as_ref()].clone();
        String::from("Package copied to current stage state")
    }

    pub fn check_reset_deadzones(&mut self) -> bool {
        let value = self.reset_deadzones;
        self.reset_deadzones = false;
        value
    }

    fn set_context(&mut self) {
        match self.edit {
            Edit::Fighter (player) => {
                let player_fighter  = self.players[player].fighter.as_ref();
                let player_action   = self.players[player].action as usize;
                let player_frame    = self.players[player].frame as usize;
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
            Edit::Stage => {
                self.stage.surfaces.set_context_vec(self.selector.surfaces_vec());
                self.stage.spawn_points.set_context_vec(self.selector.spawn_points.iter().cloned().collect());
                self.stage.respawn_points.set_context_vec(self.selector.respawn_points.iter().cloned().collect());
            }
            _ => { }
        }
    }

    fn step_local(&mut self, input: &mut Input, netplay: &Netplay) {
        self.player_history.push(self.players.clone());
        self.stage_history .push(self.stage.clone());
        self.current_frame += 1;

        // erase any future history
        for _ in self.current_frame..self.player_history.len() {
            self.player_history.pop();
        }
        for _ in self.current_frame..self.stage_history.len() {
            self.stage_history.pop();
        }

        // run game loop
        input.game_update(self.current_frame);
        let player_inputs = &input.players(self.current_frame, netplay);
        self.step_game(input, player_inputs);

        // pause game
        if input.start_pressed() {
            self.state = GameState::Paused;
        }
    }

    fn step_local_os_input(&mut self, os_input: &OsInput) {
        if os_input.key_pressed(VirtualKeyCode::Space) || os_input.key_pressed(VirtualKeyCode::Return) {
            self.state = GameState::Paused;
        }
    }

    fn step_netplay(&mut self, input: &mut Input, netplay: &Netplay) {
        if !netplay.skip_frame() {
            self.current_frame += 1;

            let start = self.current_frame - netplay.frames_to_step();
            let end = self.current_frame;

            self.player_history.truncate(start);
            self.stage_history.truncate(start);
            if start != 0 {
                self.players = self.player_history.get(start-1).unwrap().clone();
                self.stage   = self.stage_history.get(start-1).unwrap().clone();
            }

            input.netplay_update();

            for frame in start..end {
                let player_inputs = &input.players(frame, netplay);
                self.step_game(input, player_inputs);

                self.player_history.push(self.players.clone());
                self.stage_history.push(self.stage.clone());
            }
        }
    }

    fn step_pause(&mut self, input: &mut Input) {
        if input.game_quit_held() {
            self.state = GameState::Quit (ResumeMenu::Unchanged);
        }
        else if input.start_pressed() {
            self.state = GameState::Local;
        }
    }

    fn step_pause_os_input(&mut self, input: &mut Input, os_input: &OsInput, netplay: &Netplay) {
        let players_len = self.players.len();

        // set current edit state
        if os_input.key_pressed(VirtualKeyCode::Key0) {
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

        // game flow control
        if os_input.key_pressed(VirtualKeyCode::J) {
            self.step_replay_backwards(input);
        }
        else if os_input.key_pressed(VirtualKeyCode::K) {
            self.step_replay_forwards(input, netplay);
        }
        else if os_input.key_pressed(VirtualKeyCode::H) {
            self.state = GameState::ReplayBackwards;
        }
        else if os_input.key_pressed(VirtualKeyCode::L) {
            self.state = GameState::ReplayForwards;
        }
        else if os_input.key_pressed(VirtualKeyCode::Space) {
            self.step_local(input, netplay);
        }
        else if os_input.key_pressed(VirtualKeyCode::U) {
            self.saved_frame = self.current_frame;
        }
        else if os_input.key_pressed(VirtualKeyCode::I) {
            //self.jump_frame(); // TODO: Fix
        }
        else if os_input.key_pressed(VirtualKeyCode::Return) {
            self.state = GameState::Local;
        }

        match self.edit {
            Edit::Fighter (player) => {
                let fighter_string = self.players[player].fighter.clone();
                let fighter = fighter_string.as_ref();
                let action = self.players[player].action as usize;
                let action_enum = Action::from_index(self.players[player].action);
                let frame  = self.players[player].frame as usize;
                let land_frame_skip  = self.players[player].land_frame_skip;
                self.debug_players[player].step(os_input);

                // by adding the same amount of frames that are skipped in the player logic,
                // the user continues to see the same frames as they step through the action
                let repeat_frames = if action_enum.as_ref().map_or(false, |x| x.is_land()) {
                     land_frame_skip + 1
                } else {
                    1
                };

                // move collisionboxes
                if self.selector.moving {
                    // undo the operations used to render the player
                    let (raw_d_x, raw_d_y) = os_input.game_mouse_diff(&self.camera);
                    let angle = -self.players[player].angle(&self.package.fighters[fighter], &self.stage.surfaces); // rotate by the inverse of the angle
                    let d_x = raw_d_x * angle.cos() - raw_d_y * angle.sin();
                    let d_y = raw_d_x * angle.sin() + raw_d_y * angle.cos();
                    let distance = (self.players[player].relative_f(d_x), d_y); // *= -1 is its own inverse
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
                        for i in 0..repeat_frames {
                            self.package.new_fighter_frame(fighter, action, frame + i as usize);
                        }
                        // We want to step just the players current frame to simplify the animation work flow
                        // However we need to do a proper full step so that the history doesn't get mucked up.
                        self.step_local(input, netplay);
                    }
                    // delete frame
                    if os_input.key_pressed(VirtualKeyCode::N) {
                        let i = 0; //for i in 0..repeat_frames { // TODO: Panic
                            if self.package.delete_fighter_frame(fighter, action, frame - i as usize) {
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
                        //}
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
                                let (p_x, p_y) = player.public_bps_xy(&self.players, &self.package.fighters, &self.stage.surfaces);

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
                    // set hitbox angle
                    if os_input.key_pressed(VirtualKeyCode::Q) {
                        if let Some((m_x, m_y)) = os_input.game_mouse(&self.camera) {
                            let player = &self.players[player];
                            let (p_x, p_y) = player.public_bps_xy(&self.players, &self.package.fighters, &self.stage.surfaces);

                            let x = player.relative_f(m_x - p_x);
                            let y = m_y - p_y;
                            self.package.point_hitbox_angles_to(fighter, action, frame, &self.selector.colboxes, x, y);
                        }
                    }

                    // handle single selection
                    if let Some((m_x, m_y)) = self.selector.step_single_selection(os_input, &self.camera) {
                        let (player_x, player_y) = self.players[player].public_bps_xy(&self.players, &self.package.fighters, &self.stage.surfaces);
                        let frame = self.players[player].relative_frame(&self.package.fighters[fighter], &self.stage.surfaces);

                        for (i, colbox) in frame.colboxes.iter().enumerate() {
                            let hit_x = colbox.point.0 + player_x;
                            let hit_y = colbox.point.1 + player_y;

                            let distance = ((m_x - hit_x).powi(2) + (m_y - hit_y).powi(2)).sqrt();
                            if distance < colbox.radius {
                                if os_input.held_alt() {
                                    self.selector.colboxes.remove(&i);
                                } else {
                                    self.selector.colboxes.insert(i);
                                }
                            }
                        }

                        // Select topmost colbox
                        // TODO: Broken by the addition of ActionFrame.render_order, fix by taking it into account
                        if os_input.held_control() {
                            let mut selector_vec = self.selector.colboxes_vec();
                            selector_vec.sort();
                            selector_vec.reverse();
                            selector_vec.truncate(1);
                            self.selector.colboxes = selector_vec.into_iter().collect();
                        }
                    }

                    // handle multiple selection
                    if let Some(rect) = self.selector.step_multiple_selection(os_input, &self.camera) {
                        let (player_x, player_y) = self.players[player].public_bps_xy(&self.players, &self.package.fighters, &self.stage.surfaces);
                        let frame = self.players[player].relative_frame(&self.package.fighters[fighter], &self.stage.surfaces);

                        for (i, colbox) in frame.colboxes.iter().enumerate() {
                            let hit_x = colbox.point.0 + player_x;
                            let hit_y = colbox.point.1 + player_y;

                            if rect.contains_point(hit_x, hit_y) {
                                if os_input.held_alt() {
                                    self.selector.colboxes.remove(&i);
                                } else {
                                    self.selector.colboxes.insert(i);
                                }
                            }
                        }
                        self.selector.point = None;
                    }
                }
            }
            Edit::Player (player) => {
                self.debug_players[player].step(os_input);
            }
            Edit::Stage => {
                self.debug_stage.step(os_input);
                if self.selector.moving {
                    let (d_x, d_y) = os_input.game_mouse_diff(&self.camera);
                    for (i, spawn) in self.stage.spawn_points.iter_mut().enumerate() {
                        if self.selector.spawn_points.contains(&i) {
                            spawn.x += d_x;
                            spawn.y += d_y;
                        }
                    }

                    for (i, respawn) in self.stage.respawn_points.iter_mut().enumerate() {
                        if self.selector.respawn_points.contains(&i) {
                            respawn.x += d_x;
                            respawn.y += d_y;
                        }
                    }

                    for (i, surface) in self.stage.surfaces.iter_mut().enumerate() {
                        if self.selector.surfaces.contains(&SurfaceSelection::P1(i)) {
                            surface.x1 += d_x;
                            surface.y1 += d_y;
                        }
                        if self.selector.surfaces.contains(&SurfaceSelection::P2(i)) {
                            surface.x2 += d_x;
                            surface.y2 += d_y;
                        }
                    }

                    // end move
                    if os_input.mouse_pressed(0) {
                        self.update_frame();
                    }
                }
                else {
                    // start move elements
                    if os_input.key_pressed(VirtualKeyCode::A) {
                        if self.selector.surfaces.len() + self.selector.spawn_points.len() + self.selector.respawn_points.len() > 0 {
                            self.selector.moving = true;
                        }
                    }
                    // delete elements
                    if os_input.key_pressed(VirtualKeyCode::D) {
                        // the indexes are sorted in reverse order to preserve index order while deleting.
                        let mut spawns_to_delete: Vec<usize> = self.selector.spawn_points.iter().cloned().collect();
                        spawns_to_delete.sort();
                        spawns_to_delete.reverse();
                        for spawn_i in spawns_to_delete {
                            self.stage.spawn_points.remove(spawn_i);
                        }

                        let mut respawns_to_delete: Vec<usize> = self.selector.respawn_points.iter().cloned().collect();
                        respawns_to_delete.sort();
                        respawns_to_delete.reverse();
                        for respawn_i in respawns_to_delete {
                            self.stage.respawn_points.remove(respawn_i);
                        }

                        let mut surfaces_to_delete: Vec<usize> = self.selector.surfaces_vec();
                        surfaces_to_delete.sort();
                        surfaces_to_delete.reverse();
                        let players = self.players.clone();
                        for surface_i in surfaces_to_delete {
                            for player in self.players.iter_mut() {
                                player.platform_deleted(&players, &self.package.fighters, &self.stage.surfaces, surface_i);
                            }
                            self.stage.surfaces.remove(surface_i);
                        }

                        self.update_frame();
                    }
                    // add decorative surface
                    if os_input.key_pressed(VirtualKeyCode::Q) {
                        self.add_surface(Surface::default(), os_input);
                    }
                    // add ceiling surface
                    if os_input.key_pressed(VirtualKeyCode::W) {
                        let surface = Surface { ceiling: true, .. Surface::default() };
                        self.add_surface(surface, os_input);
                    }
                    // add wall surface
                    if os_input.key_pressed(VirtualKeyCode::E) {
                        let surface = Surface { wall: true, .. Surface::default() };
                        self.add_surface(surface, os_input);
                    }
                    // add stage surface
                    if os_input.key_pressed(VirtualKeyCode::R) {
                        let surface = Surface { floor: Some(Floor { traction: 1.0, pass_through: false }), .. Surface::default() };
                        self.add_surface(surface, os_input);
                    }
                    // add platform surface
                    if os_input.key_pressed(VirtualKeyCode::F) {
                        let surface = Surface { floor: Some(Floor { traction: 1.0, pass_through: true }), .. Surface::default() };
                        self.add_surface(surface, os_input);
                    }
                    // add spawn point
                    if os_input.key_pressed(VirtualKeyCode::Z) {
                        if let Some((m_x, m_y)) = os_input.game_mouse(&self.camera) {
                            self.stage.spawn_points.push(SpawnPoint::new(m_x, m_y));
                            self.update_frame();
                        }
                    }
                    // add respawn point
                    if os_input.key_pressed(VirtualKeyCode::X) {
                        if let Some((m_x, m_y)) = os_input.game_mouse(&self.camera) {
                            self.stage.respawn_points.push(SpawnPoint::new(m_x, m_y));
                            self.update_frame();
                        }
                    }
                    if os_input.key_pressed(VirtualKeyCode::S) {
                        let mut join = false;
                        let mut points: Vec<(f32, f32)> = vec!();
                        for selection in self.selector.surfaces.iter() {
                            match selection {
                                &SurfaceSelection::P1 (i) => {
                                    let surface = &self.stage.surfaces[i];
                                    if let Some((prev_x, prev_y)) = points.last().cloned() {
                                        if surface.x1 != prev_x || surface.y1 != prev_y {
                                            join = true;
                                        }
                                    }
                                    points.push((surface.x1, surface.y1));
                                }
                                &SurfaceSelection::P2 (i) => {
                                    let surface = &self.stage.surfaces[i];
                                    if let Some((prev_x, prev_y)) = points.last().cloned() {
                                        if surface.x2 != prev_x || surface.y2 != prev_y {
                                            join = true;
                                        }
                                    }
                                    points.push((surface.x2, surface.y2));
                                }
                            }
                        }

                        let mut average_x = 0.0;
                        let mut average_y = 0.0;
                        for (x, y) in points.iter().cloned() {
                            average_x += x;
                            average_y += y;
                        }
                        average_x /= points.len() as f32;
                        average_y /= points.len() as f32;

                        if join {
                            for selection in self.selector.surfaces.iter() {
                                match selection {
                                    &SurfaceSelection::P1 (i) => {
                                        let surface = &mut self.stage.surfaces[i];
                                        surface.x1 = average_x;
                                        surface.y1 = average_y;
                                    }
                                    &SurfaceSelection::P2 (i) => {
                                        let surface = &mut self.stage.surfaces[i];
                                        surface.x2 = average_x;
                                        surface.y2 = average_y;
                                    }
                                }
                            }
                        } else { // split
                            for selection in self.selector.surfaces.iter() {
                                match selection {
                                    &SurfaceSelection::P1 (i) => {
                                        let surface = &mut self.stage.surfaces[i];
                                        surface.x1 = average_x + (surface.x2 - average_x) / 5.0;
                                        surface.y1 = average_y + (surface.y2 - average_y) / 5.0;
                                    }
                                    &SurfaceSelection::P2 (i) => {
                                        let surface = &mut self.stage.surfaces[i];
                                        surface.x2 = average_x + (surface.x1 - average_x) / 5.0;
                                        surface.y2 = average_y + (surface.y1 - average_y) / 5.0;
                                    }
                                }
                            }
                        }
                    }
                }

                // handle single selection
                if let Some((m_x, m_y)) = self.selector.step_single_selection(os_input, &self.camera) {
                    if self.debug_stage.spawn_points {
                        for (i, point) in self.stage.spawn_points.iter().enumerate() {
                            let distance = ((m_x - point.x).powi(2) + (m_y - point.y).powi(2)).sqrt();
                            if distance < 4.0 {
                                if os_input.held_alt() {
                                    self.selector.spawn_points.remove(&i);
                                } else {
                                    self.selector.spawn_points.insert(i);
                                }
                            }
                        }
                    }
                    if self.debug_stage.respawn_points {
                        for (i, point) in self.stage.respawn_points.iter().enumerate() {
                            let distance = ((m_x - point.x).powi(2) + (m_y - point.y).powi(2)).sqrt();
                            if distance < 4.0 {
                                if os_input.held_alt() {
                                    self.selector.respawn_points.remove(&i);
                                } else {
                                    self.selector.respawn_points.insert(i);
                                }
                            }
                        }
                    }
                    for (i, surface) in self.stage.surfaces.iter().enumerate() {
                        let distance1 = ((m_x - surface.x1).powi(2) + (m_y - surface.y1).powi(2)).sqrt();
                        if distance1 < 3.0 { // TODO: check entire half of surface, not just the edge
                            if os_input.held_alt() {
                                self.selector.surfaces.remove(&SurfaceSelection::P1(i));
                            } else {
                                self.selector.surfaces.insert(SurfaceSelection::P1(i));
                            }
                        }
                        let distance2 = ((m_x - surface.x2).powi(2) + (m_y - surface.y2).powi(2)).sqrt();
                        if distance2 < 3.0 {
                            if os_input.held_alt() {
                                self.selector.surfaces.remove(&SurfaceSelection::P2(i));
                            } else {
                                self.selector.surfaces.insert(SurfaceSelection::P2(i));
                            }
                        }
                    }
                }

                // handle multiple selection
                if let Some(rect) = self.selector.step_multiple_selection(os_input, &self.camera) {
                    if self.debug_stage.spawn_points {
                        for (i, point) in self.stage.spawn_points.iter().enumerate() {
                            if rect.contains_point(point.x, point.y) { // TODO: check entire half of surface, not just the edge
                                if os_input.held_alt() {
                                    self.selector.spawn_points.remove(&i);
                                } else {
                                    self.selector.spawn_points.insert(i);
                                }
                            }
                        }
                    }
                    if self.debug_stage.respawn_points {
                        for (i, point) in self.stage.respawn_points.iter().enumerate() {
                            if rect.contains_point(point.x, point.y) {
                                if os_input.held_alt() {
                                    self.selector.respawn_points.remove(&i);
                                } else {
                                    self.selector.respawn_points.insert(i);
                                }
                            }
                        }
                    }
                    for (i, surface) in self.stage.surfaces.iter().enumerate() {
                        if rect.contains_point(surface.x1, surface.y1) {
                            if os_input.held_alt() {
                                self.selector.surfaces.remove(&SurfaceSelection::P1(i));
                            } else {
                                self.selector.surfaces.insert(SurfaceSelection::P1(i));
                            }
                        }
                        if rect.contains_point(surface.x2, surface.y2) {
                            if os_input.held_alt() {
                                self.selector.surfaces.remove(&SurfaceSelection::P2(i));
                            } else {
                                self.selector.surfaces.insert(SurfaceSelection::P2(i));
                            }
                        }
                    }
                    self.selector.point = None;
                }
            }
        }
        self.selector.mouse = os_input.game_mouse(&self.camera); // hack to access mouse during render call, dont use this otherwise
    }

    fn add_surface(&mut self, surface: Surface, os_input: &OsInput) {
        if let Some((m_x, m_y)) = os_input.game_mouse(&self.camera) {
            if self.selector.surfaces.len() == 1 {
                // create new surface, p1 is selected surface, p2 is current mouse
                let (x1, y1) = match self.selector.surfaces.iter().next().unwrap() {
                    &SurfaceSelection::P1 (i) => (self.stage.surfaces[i].x1, self.stage.surfaces[i].y1),
                    &SurfaceSelection::P2 (i) => (self.stage.surfaces[i].x2, self.stage.surfaces[i].y2)
                };

                self.selector.clear();
                self.selector.surfaces.insert(SurfaceSelection::P2(self.stage.surfaces.len()));
                self.stage.surfaces.push(Surface { x1, y1, x2: m_x, y2: m_y, .. surface });
            }
            else if self.selector.surfaces.len() == 0 {
                // create new surface, p1 is current mouse, p2 is moving
                self.selector.clear();
                self.selector.surfaces.insert(SurfaceSelection::P2(self.stage.surfaces.len()));
                self.selector.moving = true;
                self.stage.surfaces.push(Surface { x1: m_x, y1: m_y, x2: m_x, y2: m_y, .. surface } );
            }
        }
    }

    /// next frame is advanced by using the input history on the current frame
    // TODO: Activate by shift+K/L
    fn step_replay_forwards(&mut self, input: &mut Input, netplay: &Netplay) { // TODO: rename: step_replay_forwards_from_input
        if self.current_frame <= input.last_frame() {
            self.current_frame += 1;
            let player_inputs = &input.players(self.current_frame, netplay);
            self.step_game(input, player_inputs);

            self.update_frame();
        }
        else {
            self.state = GameState::Paused;
        }

        if input.start_pressed() {
            self.state = GameState::Paused;
        }
    }

    // TODO: Activate by K/L
    // fn step_replay_forwards_from_history() {
    //     TODO
    // }

    fn step_replay_forwards_os_input(&mut self, os_input: &OsInput) {
        if os_input.key_pressed(VirtualKeyCode::H) {
            self.state = GameState::ReplayBackwards;
        }
        if os_input.key_pressed(VirtualKeyCode::Space) || os_input.key_pressed(VirtualKeyCode::Return) {
            self.state = GameState::Paused;
        }
    }

    /// Immediately jumps to the previous frame in history
    fn step_replay_backwards(&mut self, input: &mut Input) {
        if self.current_frame > 0 {
            self.current_frame -= 1;
            self.players = self.player_history.get(self.current_frame).unwrap().clone();
            self.stage   = self.stage_history .get(self.current_frame).unwrap().clone();
            self.update_frame();
        }
        else {
            self.state = GameState::Paused;
        }

        if input.start_pressed() {
            self.state = GameState::Paused;
            self.update_frame();
        }
    }

    fn step_replay_backwards_os_input(&mut self, os_input: &OsInput) {
        if os_input.key_pressed(VirtualKeyCode::L) {
            self.state = GameState::ReplayForwards;
        }
        else if os_input.key_pressed(VirtualKeyCode::Space) || os_input.key_pressed(VirtualKeyCode::Return) {
            self.state = GameState::Paused;
            self.update_frame();
        }
    }

    /// Jump to the saved frame in history
    // TODO: FIX
    //fn jump_frame(&mut self) {
    //    let frame = self.saved_frame;
    //    if (frame+1) < self.player_history.len() {
    //        self.players = self.player_history.get(frame).unwrap().clone();

    //        self.current_frame = frame;
    //        self.update_frame();
    //    }
    //}

    /// TODO: Weird that StdRng takes usize, I thought usize was only for indexing.
    fn get_seed(&self) -> [usize; 2] {
        [self.init_seed as usize, self.current_frame as usize]
    }

    fn step_game(&mut self, input: &Input, player_input: &Vec<PlayerInput>) {
        {
            let mut rng = StdRng::from_seed(&self.get_seed());

            // To synchronize player stepping, we step through player logic in stages (action logic, physics logic, collision logic)
            // Modified players are copied from the previous stage so that every player perceives themselves as being stepped first, within that stage.

            // step each player action
            let mut action_players: Vec<Player> = vec!();
            for (i, player) in self.players.iter().enumerate() {
                let mut player = player.clone();
                let input = &player_input[self.selected_controllers[i]];
                let mut context = StepContext {
                    players:  &self.players,
                    fighters: &self.package.fighters,
                    fighter:  &self.package.fighters[player.fighter.as_ref()],
                    stage:    &self.stage,
                    surfaces: &self.stage.surfaces,
                    rng:      &mut rng,
                    input,
                };
                player.action_hitlag_step(&mut context);
                action_players.push(player);
            }

            // step each player physics
            let mut physics_players: Vec<Player> = vec!();
            for (i, player) in action_players.iter().enumerate() {
                let mut player = player.clone();
                let input = &player_input[self.selected_controllers[i]];
                let mut context = StepContext {
                    players:  &self.players,
                    fighters: &self.package.fighters,
                    fighter:  &self.package.fighters[player.fighter.as_ref()],
                    stage:    &self.stage,
                    surfaces: &self.stage.surfaces,
                    rng:      &mut rng,
                    input,
                };
                player.physics_step(&mut context, i, self.current_frame, self.package.rules.goal.clone());
                physics_players.push(player);
            }

            // check for hits and run hit logic
            let mut collision_players: Vec<Player> = vec!();
            let collision_results = collision_check(&physics_players, &self.package.fighters, &self.stage.surfaces);
            for (i, player) in physics_players.iter().enumerate() {
                let mut player = player.clone();
                let input = &player_input[self.selected_controllers[i]];
                let mut context = StepContext {
                    players:  &self.players,
                    fighters: &self.package.fighters,
                    fighter:  &self.package.fighters[player.fighter.as_ref()],
                    stage:    &self.stage,
                    surfaces: &self.stage.surfaces,
                    rng:      &mut rng,
                    input,
                };
                player.step_collision(&mut context, &collision_results[i]);
                collision_players.push(player);
            }

            self.players = collision_players;
        }

        if self.time_out() ||
           (self.players.len() == 1 && self.players.iter().filter(|x| x.action != Action::Eliminated.index()).count() == 0) ||
           (self.players.len() >  1 && self.players.iter().filter(|x| x.action != Action::Eliminated.index()).count() == 1)
        {
            self.state = self.generate_game_results(input);
        }

        self.update_frame();
    }

    pub fn time_out(&self) -> bool {
        if let Some(time_limit_frames) = self.package.rules.time_limit_frames() {
            self.current_frame as u64 > time_limit_frames
        } else {
            false
        }
    }

    pub fn generate_game_results(&self, input: &Input) -> GameState {
        let raw_player_results: Vec<RawPlayerResult> = self.players.iter().map(|x| x.result()).collect();
        // TODO: Players on the same team score to the same pool, and share their place.
        let places: Vec<usize> = match self.package.rules.goal {
            Goal::LastManStanding => {
                // most stocks remaining wins
                // tie-breaker:
                //  * if both eliminated: who lost their last stock last wins
                //  * if both alive:      lowest percentage wins
                let mut raw_player_results_i: Vec<(usize, &RawPlayerResult)> = raw_player_results.iter().enumerate().collect();
                raw_player_results_i.sort_by(
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
                raw_player_results_i.iter().map(|x| x.0).collect()
            }
            Goal::KillDeathScore => {
                // highest kills wins
                // tie breaker: least deaths wins
                let mut raw_player_results_i: Vec<(usize, &RawPlayerResult)> = raw_player_results.iter().enumerate().collect();
                raw_player_results_i.sort_by(
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
                raw_player_results_i.iter().map(|x| x.0).collect()
            }
        };

        let mut player_results: Vec<PlayerResult> = vec!();
        for (i, raw_player_result) in raw_player_results.iter().enumerate() {
            let lcancel_percent = if raw_player_result.lcancel_attempts == 0 {
                100.0
            }
            else {
                raw_player_result.lcancel_success as f32 / raw_player_result.lcancel_attempts as f32
            };
            player_results.push(PlayerResult {
                fighter:         raw_player_result.ended_as_fighter.clone().unwrap(),
                team:            raw_player_result.team,
                controller:      self.selected_controllers[i],
                place:           places[i],
                kills:           vec!(), // TODO
                deaths:          raw_player_result.deaths.clone(),
                lcancel_percent: lcancel_percent,
            });
        }
        player_results.sort_by_key(|x| x.place);

        let replay = Replay::new(self, input);

        GameState::Quit (
            ResumeMenu::Results (
                GameResults {
                    player_results,
                    replay,
                }
            )
        )
    }

    fn generate_debug(&mut self, input: &Input, netplay: &Netplay) {
        let frame = self.current_frame;
        let player_inputs = &input.players_no_log(frame, netplay);

        self.debug_lines = vec!(format!("Frame: {}    state: {}", frame, self.state));
        for (i, player) in self.players.iter().enumerate() {
            let fighter = &self.package.fighters[self.players[i].fighter.as_ref()];
            let player_input = &player_inputs[self.selected_controllers[i]];
            let debug_player = &self.debug_players[i];
            self.debug_lines.extend(player.debug_print(fighter, player_input, debug_player, i));
        }

        if self.debug_output_this_step {
            self.debug_output_this_step = false;
            for i in 1..self.debug_lines.len() {
                debug!("{}", self.debug_lines[i]);
            }
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
                let cam_area = player.cam_area(&self.stage.camera, &self.players, &self.package.fighters, &self.stage.surfaces);
                entities.push(RenderEntity::rect_outline(cam_area, 0.0, 0.0, 1.0));
            }

            let fighters = &self.package.fighters;
            let surfaces = &self.stage.surfaces;
            let player_render = player.render(selected_colboxes, fighter_selected, player_selected, debug, &self.players, fighters, surfaces);
            entities.push(RenderEntity::Player(player_render));
        }

        // render stage debug entities
        if self.debug_stage.blast {
            entities.push(RenderEntity::rect_outline(self.stage.blast.clone(),  1.0, 0.0, 0.0));
        }
        if self.debug_stage.camera {
            entities.push(RenderEntity::rect_outline(self.stage.camera.clone(), 0.0, 0.0, 1.0));
        }
        if self.debug_stage.spawn_points {
            for (i, point) in self.stage.spawn_points.iter().enumerate() {
                if self.selector.spawn_points.contains(&i) {
                    entities.push(RenderEntity::spawn_point(point.clone(), 0.0, 1.0, 0.0));
                } else {
                    entities.push(RenderEntity::spawn_point(point.clone(), 1.0, 0.0, 1.0));
                }
            }
        }
        if self.debug_stage.respawn_points {
            for (i, point) in self.stage.respawn_points.iter().enumerate() {
                if self.selector.respawn_points.contains(&i) {
                    entities.push(RenderEntity::spawn_point(point.clone(), 0.0, 1.0, 0.0));
                } else {
                    entities.push(RenderEntity::spawn_point(point.clone(), 1.0, 1.0, 0.0));
                }
            }
        }

        // render selector box
        if let Some(point) = self.selector.point {
            if let Some(mouse) = self.selector.mouse {
                let render_box = Rect::from_tuples(point, mouse);
                entities.push(RenderEntity::rect_outline(render_box, 0.0, 1.0, 0.0));
            }
        }

        let timer = if let Some(time_limit_frames) = self.package.rules.time_limit_frames() {
            let frames_remaining = time_limit_frames.saturating_sub(self.current_frame as u64);
            let frame_duration = Duration::new(1, 0) / 60;
            Some(frame_duration * frames_remaining as u32)
        } else {
            None
        };

        RenderGame {
            seed:              self.get_seed(),
            surfaces:          self.stage.surfaces.to_vec(),
            selected_surfaces: self.selector.surfaces.clone(),
            entities:          entities,
            state:             self.state.clone(),
            camera:            self.camera.clone(),
            debug_lines:       self.debug_lines.clone(),
            timer:             timer,
        }
    }

    pub fn graphics_message(&mut self, command_line: &CommandLine) -> GraphicsMessage {
        let render = Render {
            command_output: command_line.output(),
            render_type:    RenderType::Game(self.render()),
            fullscreen:     self.config.fullscreen
        };

        GraphicsMessage {
            package_updates: self.package.updates(),
            render:          render,
        }
    }

    pub fn reclaim(self) -> (Package, Config) {
        (self.package, self.config)
    }
}

#[derive(Clone, Serialize, Deserialize, Node)]
pub enum GameState {
    Local,
    ReplayForwards,
    ReplayBackwards,
    Netplay,
    Paused, // Only Local, ReplayForwards and ReplayBackwards can be paused
    Quit (ResumeMenu), // Both Local and Netplay end at Quit

    // Used for TAS, in game these are run during pause state
    StepThenPause,
    StepForwardThenPause,
    StepBackwardThenPause,
}

impl fmt::Display for GameState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &GameState::Local                 => write!(f, "Local"),
            &GameState::ReplayForwards        => write!(f, "ReplayForwards"),
            &GameState::ReplayBackwards       => write!(f, "ReplayBackwards"),
            &GameState::Netplay               => write!(f, "Netplay"),
            &GameState::Paused                => write!(f, "Paused"),
            &GameState::Quit (_)              => write!(f, "Quit"),
            &GameState::StepThenPause         => write!(f, "StepThenPause"),
            &GameState::StepForwardThenPause  => write!(f, "StepForwardThenPause"),
            &GameState::StepBackwardThenPause => write!(f, "StepBackwardThenPause)"),
        }
    }
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
    colboxes:       HashSet<usize>,
    surfaces:      HashSet<SurfaceSelection>,
    spawn_points:   HashSet<usize>,
    respawn_points: HashSet<usize>,
    moving:         bool,
    point:          Option<(f32, f32)>, // selector starting point
    mouse:          Option<(f32, f32)>, // used to know mouse point during render
}

impl Selector {
    fn colboxes_vec(&self) -> Vec<usize> {
        self.colboxes.iter().cloned().collect()
    }

    fn surfaces_vec(&self) -> Vec<usize> {
        let mut result = vec!();
        let mut prev_i: Option<usize> = None;
        let mut surfaces: Vec<usize> = self.surfaces.iter().map(|x| x.index()).collect();
        surfaces.sort();

        for surface_i in surfaces {
            if let Some(prev_i) = prev_i {
                if prev_i != surface_i {
                    result.push(surface_i)
                }
            }
            else {
                result.push(surface_i)
            }
            prev_i = Some(surface_i);
        }
        result
    }

    fn start(&mut self, mouse: (f32, f32)) {
        self.point  = Some(mouse);
        self.moving = false;
        self.mouse  = None;
    }

    fn clear(&mut self) {
        self.colboxes.clear();
        self.surfaces.clear();
        self.spawn_points.clear();
        self.respawn_points.clear();
    }

    /// Returns a selection rect iff a multiple selection is finished.
    fn step_multiple_selection(&mut self, os_input: &OsInput, camera: &Camera) -> Option<Rect> {
        // start selection
        if os_input.mouse_pressed(1) {
            if let Some(mouse) = os_input.game_mouse(camera) {
                self.start(mouse);
            }
        }

        // finish selection
        if let (Some(p1), Some(p2)) = (self.point, os_input.game_mouse(camera)) {
            if os_input.mouse_released(1) {
                if !(os_input.held_shift() || os_input.held_alt()) {
                    self.clear();
                }
                return Some(Rect::from_tuples(p1, p2))
            }
        }
        None
    }

    /// Returns a selection point iff a single selection is made.
    fn step_single_selection(&mut self, os_input: &OsInput, camera: &Camera) -> Option<(f32, f32)> {
        if os_input.mouse_pressed(0) {
            if let point @ Some(_) = os_input.game_mouse(camera) {
                if !(os_input.held_shift() || os_input.held_alt()) {
                    self.clear();
                }
                return point;
            }
        }
        None
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, Node)]
pub enum SurfaceSelection {
    P1 (usize),
    P2 (usize)
}

impl SurfaceSelection {
    fn index(&self) -> usize {
        match self {
            &SurfaceSelection::P1 (index) |
            &SurfaceSelection::P2 (index) => index
        }
    }
}

impl Default for SurfaceSelection {
    fn default() -> SurfaceSelection {
        SurfaceSelection::P1 (0)
    }
}

pub struct RenderGame {
    pub seed:              [usize; 2],
    pub surfaces:          Vec<Surface>,
    pub selected_surfaces: HashSet<SurfaceSelection>,
    pub entities:          Vec<RenderEntity>,
    pub state:             GameState,
    pub camera:            Camera,
    pub debug_lines:       Vec<String>,
    pub timer:             Option<Duration>,
}

pub enum RenderEntity {
    Player      (RenderPlayer),
    RectOutline (RenderRect),
    SpawnPoint  (RenderSpawnPoint),
}

impl RenderEntity {
    pub fn rect_outline(rect: Rect, r: f32, g: f32, b: f32) -> RenderEntity {
        RenderEntity::RectOutline (
            RenderRect {
                rect,
                color: [r, g, b, 1.0]
            }
        )
    }
    pub fn spawn_point(point: SpawnPoint, r: f32, g: f32, b: f32) -> RenderEntity {
        RenderEntity::SpawnPoint (
            RenderSpawnPoint {
                point,
                color: [r, g, b, 1.0]
            }
        )
    }
}

pub struct RenderRect {
    pub rect:  Rect,
    pub color: [f32; 4]
}

pub struct RenderSpawnPoint {
    pub point: SpawnPoint,
    pub color: [f32; 4]
}

#[derive(Clone)]
pub struct GameSetup {
    pub init_seed:      u64,
    pub input_history:  Vec<Vec<ControllerInput>>,
    pub player_history: Vec<Vec<Player>>,
    pub stage_history:  Vec<Stage>,
    pub controllers:    Vec<usize>,
    pub players:        Vec<PlayerSetup>,
    pub ais:            Vec<usize>,
    pub stage:          String,
    pub state:          GameState,
}

impl GameSetup {
    pub fn gen_seed() -> u64 {
        Local::now().timestamp() as u64
    }
}

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct PlayerSetup {
    pub fighter: String,
    pub team:    usize,
}
