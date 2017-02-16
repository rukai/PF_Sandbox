use ::fighter::*;
use ::input::{PlayerInput};
use ::stage::{Stage, Platform, Area, SpawnPoint};
use ::collision::CollisionResult;

use std::f32;
use num::FromPrimitive;
use std::collections::HashSet;
use treeflection::{Node, NodeRunner, NodeToken};

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct Player {
    pub action:           u64,
    action_new:           u64,
    action_set:           bool,
    pub frame:            u64,
    pub stocks:           u64,
    pub damage:           f32,
    pub bps_x:            f32,
    pub bps_y:            f32,
    pub respawn:          SpawnPoint,
    pub x_vel:            f32,
    pub y_vel:            f32,
    pub kb_x_vel:         f32,
    pub kb_y_vel:         f32,
    pub kb_x_dec:         f32,
    pub kb_y_dec:         f32,
    pub face_right:       bool,
    pub airbourne:        bool,
    pub pass_through:     bool,
    pub fastfalled:       bool,
    pub air_jumps_left:   u64,
    pub jumpsquat_button: bool,
    pub turn_dash_buffer: bool,
    pub ecb:              ECB,
}


impl Player {
    pub fn new(spawn: SpawnPoint, respawn: SpawnPoint, stocks: u64) -> Player {
        Player {
            action:           Action::Spawn as u64,
            action_new:       Action::Spawn as u64,
            action_set:       false,
            frame:            0,
            stocks:           stocks,
            damage:           0.0,
            bps_x:            spawn.x,
            bps_y:            spawn.y,
            respawn:          respawn,
            x_vel:            0.0,
            y_vel:            0.0,
            kb_x_vel:         0.0,
            kb_y_vel:         0.0,
            kb_x_dec:         0.0,
            kb_y_dec:         0.0,
            ecb:              ECB::default(),
            face_right:       spawn.face_right,
            airbourne:        true,
            pass_through:     false,
            fastfalled:       false,
            air_jumps_left:   0,
            jumpsquat_button: false,
            turn_dash_buffer: false,
        }
    }

    // always change self.action through this method
    fn set_action(&mut self, action: Action) {
        self.action_new = action as u64;
        self.action_set = true;
    }

    // TODO: I could hook in a turbo mode here
    fn interruptible(&self, fighter: &Fighter) -> bool {
        self.frame >= fighter.actions[self.action as usize].iasa
    }

    pub fn step_collision(&mut self, fighter: &Fighter, col_results: &[CollisionResult]) {
        for col_result in col_results {
            match col_result {
                &CollisionResult::HitDef (ref hitbox, ref hurtbox) => {
                    let damage_done = hitbox.damage * hurtbox.damage_mult; // TODO: apply staling
                    self.damage += damage_done;

                    let damage_launch = 0.05 * (hitbox.damage * (damage_done + self.damage.floor())) + (damage_done + self.damage) * 0.1;
                    let weight = 2.0 - (2.0 * fighter.weight) / (1.0 + fighter.weight);
                    let kbg = hitbox.kbg + hurtbox.kbg_add;
                    let bkb = hitbox.bkb + hurtbox.bkb_add;

                    let mut kb_vel = (bkb + kbg * (damage_launch * weight * 1.4 + 18.0)).min(2500.0); // 96

                    if let Some(action) = Action::from_u64(self.action) {
                        match action {
                            Action::Crouch => {
                                kb_vel *= 0.67;
                            }
                            _ => { }
                        }
                    }

                    let angle_deg = if hitbox.angle == 361.0 {
                        if kb_vel < 32.1 {
                            0.0
                        }
                        else {
                            44.0
                        }
                    } else if hitbox.angle == 180.0 - 361.0 {
                        if kb_vel < 32.1 {
                            180.0
                        }
                        else {
                            180.0 - 44.0
                        }
                    } else {
                        hitbox.angle
                    };
                    let angle = angle_deg * f32::consts::PI / 180.0;
                    let (sin, cos) = angle.sin_cos();
                    self.kb_x_vel = cos * kb_vel * 0.03;
                    self.kb_y_vel = sin * kb_vel * 0.03;
                    self.kb_x_dec = cos * 0.051;
                    self.kb_y_dec = sin * 0.051;

                    if self.kb_y_vel == 0.0 {
                        if kb_vel >= 80.0 {
                            self.airbourne = true;
                            self.bps_y += 0.0001;
                        }
                    }
                    else if self.kb_y_vel > 0.0 {
                        self.airbourne = true;
                    }

                    if true { // airbourne
                        self.set_action(Action::DamageFly);
                    }
                    else {
                        self.set_action(Action::Damage);
                    }
                }
                _ => { }
            }
        }
    }

    pub fn step(&mut self, input: &PlayerInput, fighter: &Fighter, stage: &Stage) {
        self.input_step(input, fighter);
        self.physics_step(fighter, stage);
        self.action_step();
    }

    fn action_step(&mut self) {
        if self.action_set {
            self.frame = 0;
            self.action = self.action_new;
            self.action_set = false;
        }
        else {
            self.frame += 1;
        }
    }

    /*
     *  Begin input section
     */

    fn input_step(&mut self, input: &PlayerInput, fighter: &Fighter) {
        let action_frames = fighter.actions[self.action as usize].frames.len() as u64;

        // handles a frame index that no longer exists by jumping to the last existing frame
        if self.frame >= action_frames - 1 {
            self.frame = action_frames - 1;
        }

        if self.frame == action_frames - 1 {
            self.action_expired(input, fighter);
        }

        let fighter_frame = &fighter.actions[self.action as usize].frames[self.frame as usize];
        let action = Action::from_u64(self.action);

        // update ecb
        let prev_bot_y = self.ecb.bot_y;
        self.ecb = fighter_frame.ecb.clone();
        match action {
            //TODO: Err does this if apply to all Some()?
            Some(Action::JumpF) | Some(Action::JumpB) | Some(Action::JumpAerialF) | Some(Action::JumpAerialB) if self.frame < 10
                => { self.ecb.bot_y = prev_bot_y }
            _   => { }
        }

        if let Some(action) = action {
            match action {
                Action::SpawnIdle  | Action::Fall |
                Action::AerialFall | Action::JumpAerialF |
                Action::JumpF      | Action::JumpB |
                Action::Fair       | Action::Bair |
                Action::Dair       | Action::Uair |
                Action::Nair       | Action::JumpAerialB
                => { self.aerial_action(input, fighter) }

                Action::Jab       | Action::Jab2 |
                Action::Jab3      | Action::Utilt |
                Action::Ftilt     | Action::DashAttack |
                Action::Dsmash    | Action::Fsmash |
                Action::Usmash    | Action::Idle |
                Action::Grab      | Action::DashGrab |
                Action::CrouchEnd | Action::CrouchStart |
                Action::FairLand  | Action::BairLand |
                Action::UairLand  | Action::DairLand |
                Action::Land      | Action::SpecialLand
                => { self.ground_idle_action(input, fighter) }

                Action::AerialDodge => { self.aerialdodge_action(input, fighter) }
                Action::SpecialFall => { self.specialfall_action(input, fighter) }
                Action::Dtilt       => { self.dtilt_action(input, fighter) }
                Action::Crouch      => { self.crouch_action(input, fighter) }
                Action::Walk        => { self.walk_action(input, fighter) }
                Action::Dash        => { self.dash_action(input, fighter) }
                Action::Run         => { self.run_action(input, fighter) }
                Action::Turn        => { self.turn_action(input, fighter) }
                _ => { },
            }
        }
    }

    fn aerial_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        if self.interruptible(fighter) {
            if self.check_attacks_aerial(input) { }
            else if input.b.press {
                // special attack
            }
            else if self.jump_input(input).jump() && self.air_jumps_left > 0 {
                self.air_jumps_left -= 1;
                self.y_vel = fighter.jump_y_init_vel;
                self.x_vel = fighter.jump_x_init_vel * input[0].stick_x;
                self.fastfalled = false;

                if self.relative_f(input.stick_x.value) < -0.1 { // TODO: refine
                    self.set_action(Action::JumpAerialB);
                }
                else {
                    self.set_action(Action::JumpAerialF);
                }
            }
            else if input.l.press || input.r.press {
                self.aerialdodge(input, fighter);
            }
        }

        self.air_drift(input, fighter);
        self.fall_action(input, fighter);
        self.pass_through = input.stick_y.value < -0.2; // TODO: refine
    }

    fn air_drift(&mut self, input: &PlayerInput, fighter: &Fighter) {
        let term_vel = fighter.air_x_term_vel * input[0].stick_x;
        let drift = input[0].stick_x.abs() >= 0.3;
        if !drift ||
           (term_vel < 0.0 && self.x_vel < term_vel) ||
           (term_vel > 0.0 && self.x_vel > term_vel) {
            if self.x_vel > 0.0 {
                self.x_vel -= fighter.air_friction;
                if self.x_vel < 0.0 {
                    self.x_vel = 0.0;
                }
            }
            else if self.x_vel < 0.0 {
                self.x_vel += fighter.air_friction;
                if self.x_vel > 0.0 {
                    self.x_vel = 0.0;
                }
            }
        }

        if drift {
            if (term_vel < 0.0 && self.x_vel > term_vel) ||
               (term_vel > 0.0 && self.x_vel < term_vel) {
                self.x_vel += fighter.air_mobility_a * input[0].stick_x + fighter.air_mobility_b * input[0].stick_x.signum();
            }
        }
    }

    fn turn_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        if self.frame == 0 && self.dash_input(input) {
            self.set_action(Action::Dash);
        }
        if self.check_jump(input) { }
        else if self.check_special(input) { } // TODO: No neutral special
        else if self.check_smash(input) { }
        else if self.check_attacks(input) { }
        else if self.check_taunt(input) { }
        else {
            self.apply_friction(fighter);
        }
    }

    fn crouch_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        self.apply_friction(fighter);
        if input.stick_y.value > -0.3 {
            self.set_action(Action::CrouchEnd);
        }
        if self.interruptible(fighter) {
            if self.check_jump(input) { }
            else if self.check_special(input) { } // TODO: no neutral/side special
            else if self.check_smash(input) { }
            else if self.check_attacks(input) { }
            else if self.check_dash(input, fighter) { }
            else if self.check_turn(input) { }
            else if self.check_walk(input, fighter) { }
            else if self.check_taunt(input) { }
        }
        self.apply_friction(fighter);
    }

    fn dtilt_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        self.apply_friction(fighter);
        if self.interruptible(fighter) {
            if self.check_jump(input) { }
            else if self.check_special(input) { } // TODO: no neutral/side special
            else if self.check_smash(input) { }
            else if self.check_attacks(input) { }
            else if self.check_dash(input, fighter) { }
            else if self.check_turn(input) { }
            else if self.check_walk(input, fighter) { }
            else if self.check_taunt(input) { }
        }
    }

    fn ground_idle_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        self.apply_friction(fighter);
        if self.interruptible(fighter) {
            if self.check_jump(input) { }
            else if self.check_special(input) { }
            else if self.check_smash(input) { }
            else if self.check_attacks(input) { }
            else if self.check_crouch(input) { }
            else if self.check_dash(input, fighter) { }
            else if self.check_turn(input) { }
            else if self.check_walk(input, fighter) { }
            else if self.check_taunt(input) { }
        }

        self.pass_through = input.stick_y.diff < -0.1; // TODO: refine
    }

    fn walk_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        if input[0].stick_x == 0.0 {
            self.set_action(Action::Idle);
        }
        else if self.check_jump(input) { }
        else if self.check_special(input) { }
        else if self.check_smash(input) { }
        else if self.check_attacks(input) { }
        else if self.check_crouch(input) { }
        else if self.check_dash(input, fighter) { }
        else if self.check_turn(input) { }
        else if self.check_taunt(input) { }
        else {
            let vel_max = fighter.walk_max_vel * input[0].stick_x;

            if self.x_vel.abs() > vel_max.abs() {
                self.apply_friction(fighter);
            }
            else {
                let acc = (vel_max - self.x_vel) * (2.0/fighter.walk_max_vel) * (fighter.walk_init_vel + fighter.walk_acc);
                self.x_vel += acc;
                if self.relative_f(self.x_vel) > self.relative_f(vel_max) {
                    self.x_vel = acc;
                }
            }
        }
    }

    fn dash_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        if self.frame == 2 {
            self.x_vel = self.relative_f(fighter.dash_init_vel);
            if self.x_vel.abs() > fighter.dash_run_term_vel {
                self.x_vel = self.relative_f(fighter.dash_run_term_vel);
            }
        }

        if self.frame > 1 {
            if input[0].stick_x.abs() < 0.3 {
                self.apply_friction(fighter);
            }
            else {
                let vel_max = input[0].stick_x * fighter.dash_run_term_vel;
                let acc     = input[0].stick_x * fighter.dash_run_acc_a;

                self.x_vel += acc;
                if (vel_max > 0.0 && self.x_vel > vel_max) || (vel_max < 0.0 && self.x_vel < vel_max) {
                    self.apply_friction(fighter);
                    if (vel_max > 0.0 && self.x_vel < vel_max) || (vel_max < 0.0 && self.x_vel > vel_max) {
                        self.x_vel = vel_max;
                    }
                }
                else {
                    self.x_vel += acc;
                    if (vel_max > 0.0 && self.x_vel > vel_max) || (vel_max < 0.0 && self.x_vel < vel_max) {
                        self.x_vel = vel_max;
                    }
                }
            }
        }
        if self.relative_f(input.stick_x.value) < -0.35 { // TODO: refine
            self.turn();
        }
        else if input.a.press {
            self.set_action(Action::DashAttack);
        }
        else if input.z.press {
            self.set_action(Action::DashGrab);
        }
        self.check_jump(input);
    }

    fn run_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        self.check_jump(input);
        if self.relative_f(input.stick_x.value) <= -0.3 {
            self.set_action(Action::TurnRun);
        }
        // verified horizontally stick_x <= 0.61250 ends run
        else if self.relative_f(input.stick_x.value) < 0.62 {
            self.set_action(Action::RunEnd);
        }
        else if input.a.press {
            self.set_action(Action::DashAttack);
        }
        else if input.z.press {
            self.set_action(Action::DashGrab);
        }
        else {
            let vel_max = input[0].stick_x * fighter.dash_run_term_vel;
            let acc = (vel_max - self.x_vel)
                    * (fighter.dash_run_acc_a + (fighter.dash_run_acc_b / input[0].stick_x.abs()))
                    / (fighter.dash_run_term_vel * 2.5);

            self.x_vel += acc;
            if self.relative_f(self.x_vel) > self.relative_f(vel_max) {
                self.x_vel = vel_max;
            }
        }
        self.check_jump(input);
    }

    fn aerialdodge(&mut self, input: &PlayerInput, fighter: &Fighter) {
        self.set_action(Action::AerialDodge);
        match input[0].stick_angle() {
            Some(angle) => {
                self.x_vel = angle.cos() * fighter.aerialdodge_mult;
                self.y_vel = angle.sin() * fighter.aerialdodge_mult;
            }
            None => {
                self.x_vel = 0.0;
                self.y_vel = 0.0;
            }
        }
        self.fastfalled = false;
    }

    fn aerialdodge_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        if self.frame < fighter.aerialdodge_drift_frame {
            self.x_vel *= 0.9;
            self.y_vel *= 0.9;
        }
        else {
            self.air_drift(input, fighter);
        }
    }

    fn check_crouch(&mut self, input: &PlayerInput) -> bool {
        if input[0].stick_y < -0.69 {
            if let Some(action) = Action::from_u64(self.action) {
                match action {
                    Action::CrouchStart | Action::Crouch | Action::CrouchEnd => {
                    }
                    _ => {
                        self.set_action(Action::CrouchStart);
                    }
                }
            }
            true
        }
        else {
            false
        }
    }

    fn check_walk(&mut self, input: &PlayerInput, fighter: &Fighter) -> bool {
        if input[0].stick_x.abs() > 0.3 {
            self.walk(fighter);
            true
        }
        else {
            false
        }
    }

    fn check_dash(&mut self, input: &PlayerInput, fighter: &Fighter) -> bool {
        if self.dash_input(input) {
            let stick_face_right = input.stick_x.value > 0.0;
            if stick_face_right == self.face_right {
                self.dash(fighter);
            }
            else {
                self.turn_dash();
            }
            true
        }
        else {
            false
        }
    }

    fn check_turn(&mut self, input: &PlayerInput) -> bool {
        let turn = self.relative_f(input[0].stick_x) < -0.3;
        if turn {
            self.turn();
        }
        self.turn_dash_buffer = self.relative_f(input[1].stick_x) > -0.3;
        turn
    }

    fn check_jump(&mut self, input: &PlayerInput) -> bool{
        match self.jump_input(input) {
            JumpResult::Button => {
                self.jumpsquat_button = true;
                self.set_action(Action::JumpSquat);
                true
            }
            JumpResult::Stick => {
                self.jumpsquat_button = false;
                self.set_action(Action::JumpSquat);
                true
            }
            JumpResult::None => {
                false
            }
        }
    }

    fn check_attacks_aerial(&mut self, input: &PlayerInput) -> bool {
        if input.a.press || input.z.press {
            if self.relative_f(input[0].stick_x) > 0.3 && input[0].stick_x.abs() > input[0].stick_y.abs() - 0.1 {
                self.set_action(Action::Fair);
            }
            else if self.relative_f(input[0].stick_x) < -0.3 && input[0].stick_x.abs() > input[0].stick_y.abs() - 0.1 {
                self.set_action(Action::Bair);
            }
            else if input[0].stick_y < -0.3 {
                self.set_action(Action::Dair);
            }
            else if input[0].stick_y > 0.3 {
                self.set_action(Action::Uair);
            }
            else {
                self.set_action(Action::Nair);
            }
            true
        }
        else if self.relative_f(input[0].c_stick_x) >= 0.3 && self.relative_f(input[1].c_stick_x) < 0.3 
            && input[0].c_stick_x.abs() > input[0].c_stick_y.abs() - 0.1
        {
            self.set_action(Action::Fair);
            true
        }
        else if self.relative_f(input[0].c_stick_x) <= -0.3 && self.relative_f(input[1].c_stick_x) > -0.3
            && input[0].c_stick_x.abs() > input[0].c_stick_y.abs() - 0.1
        {
            self.set_action(Action::Bair);
            true
        }
        else if input[0].c_stick_y < -0.3 && input[1].c_stick_y > -0.3 {
            self.set_action(Action::Dair);
            true
        }
        else if input[0].c_stick_y >= 0.3 && input[1].c_stick_y < 0.3 {
            self.set_action(Action::Uair);
            true
        }
        else {
            false
        }
    }

    fn check_attacks(&mut self, input: &PlayerInput) -> bool {
        if input.a.press {
            if self.relative_f(input[0].stick_x) > 0.3 && input[0].stick_x.abs() - input[0].stick_y.abs() > -0.05 {
                self.set_action(Action::Ftilt);
            }
            else if input[0].stick_y < -0.3 {
                self.set_action(Action::Dtilt);
            }
            else if input[0].stick_y > 0.3 {
                self.set_action(Action::Utilt);
            }
            else {
                self.set_action(Action::Jab);
            }
            true
        }
        else {
            false
        }
    }

    fn check_special(&mut self, input: &PlayerInput) -> bool {
        if input.b.press {
            // special attack
            true
        }
        else {
            false
        }
    }

    fn check_smash(&mut self, input: &PlayerInput) -> bool {
        if input.a.press {
            if (input[0].stick_x >=  0.79 && input[2].stick_x < 0.3) ||
               (input[0].stick_x <= -0.79 && input[2].stick_x > 0.3) {
                self.face_right = input.c_stick_x.value > 0.0;
                self.set_action(Action::Fsmash);
                return true;
            }
            else if input[0].stick_y >= 0.66 && input[2].stick_y < 0.3 {
                self.set_action(Action::Usmash);
                return true;
            }
            else if input[0].stick_y <= -0.66 && input[2].stick_y > 0.3 {
                self.set_action(Action::Dsmash);
                return true;
            }
        }
        else if input[0].c_stick_x.abs() >= 0.79 && input[1].c_stick_x.abs() < 0.79 {
            self.face_right = input.c_stick_x.value > 0.0;
            self.set_action(Action::Fsmash);
            return true;
        }
        else if input[0].c_stick_y >= 0.66 && input[1].c_stick_y < 0.66 {
            self.set_action(Action::Usmash);
            return true;
        }
        else if input[0].c_stick_y <= -0.66 && input[1].c_stick_y > -0.66 {
            self.set_action(Action::Dsmash);
            return true;
        }
        false
    }

    fn check_taunt(&mut self, input: &PlayerInput) -> bool {
        if input.up.press {
            self.set_action(Action::TauntUp);
            true
        }
        else if input.down.press {
            self.set_action(Action::TauntDown);
            true
        }
        else if input.left.press {
            self.set_action(Action::TauntLeft);
            true
        }
        else if input.right.press {
            self.set_action(Action::TauntRight);
            true
        }
        else {
            false
        }
    }

    fn jump_input(&self, input: &PlayerInput) -> JumpResult {
        if input.x.press || input.y.press {
            JumpResult::Button
        }
        else if input[0].stick_y > 0.66 && input[3].stick_y < 0.2 {
            JumpResult::Stick
        }
        else {
            JumpResult::None
        }
    }

    fn dash_input(&self, input: &PlayerInput) -> bool {
        input[0].stick_x.abs() > 0.79 && input[2].stick_x.abs() < 0.3
    }

    fn action_expired(&mut self, input: &PlayerInput, fighter: &Fighter) {
        match Action::from_u64(self.action) {
            None => { panic!("Custom defined action expirations have not been implemented"); },

            // Idle
            Some(Action::Spawn)     => { self.set_action(Action::SpawnIdle); },
            Some(Action::SpawnIdle) => { self.set_action(Action::SpawnIdle); },
            Some(Action::Idle)      => { self.set_action(Action::Idle);      },

            // crouch
            Some(Action::CrouchStart) => { self.set_action(Action::Crouch); },
            Some(Action::Crouch)      => { self.set_action(Action::Crouch); },
            Some(Action::CrouchEnd)   => { self.set_action(Action::Idle);   },

            // Movement
            Some(Action::Fall)         => { self.set_action(Action::Fall);       },
            Some(Action::AerialFall)   => { self.set_action(Action::AerialFall); },
            Some(Action::Land)         => { self.set_action(Action::Idle);       },
            Some(Action::JumpF)        => { self.set_action(Action::Fall);       },
            Some(Action::JumpB)        => { self.set_action(Action::Fall);       },
            Some(Action::JumpAerialF)  => { self.set_action(Action::AerialFall); },
            Some(Action::JumpAerialB)  => { self.set_action(Action::AerialFall); },
            Some(Action::TurnDash)     => { self.set_action(Action::Dash);       },
            Some(Action::TurnRun)      => { self.set_action(Action::Idle);       },
            Some(Action::Dash)         => { self.set_action(Action::Run);        },
            Some(Action::Run)          => { self.set_action(Action::Run);        },
            Some(Action::RunEnd)       => { self.set_action(Action::Idle);       },
            Some(Action::Walk)         => { self.set_action(Action::Walk);       },
            Some(Action::PassPlatform) => { self.set_action(Action::AerialFall); },
            Some(Action::Damage)       => { self.set_action(Action::Idle);       },
            Some(Action::DamageFly)    => { self.set_action(Action::DamageFall); },
            Some(Action::DamageFall)   => { self.set_action(Action::DamageFall); },
            Some(Action::Turn) => {
                let new_action = if self.relative_f(input[0].stick_x) > 0.79 && self.turn_dash_buffer {
                    Action::Dash
                }
                else {
                    Action::Idle
                };
                self.set_action(new_action);
            },
            Some(Action::JumpSquat) => {
                self.airbourne = true;

                let shorthop = if self.jumpsquat_button {
                    !input[0].x && !input[0].y
                }
                else {
                    input[0].stick_y < 0.67
                };

                if shorthop {
                    self.y_vel = fighter.jump_y_init_vel_short;
                }
                else {
                    self.y_vel = fighter.jump_y_init_vel;
                }

                if self.relative_f(input[2].stick_x) >= -0.3 {
                    self.set_action(Action::JumpF);
                }
                else {
                    self.set_action(Action::JumpB);
                }
            },

            // Defense
            Some(Action::ShieldOn)    => { self.set_action(Action::Shield);      },
            Some(Action::Shield)      => { self.set_action(Action::Shield);      },
            Some(Action::ShieldOff)   => { self.set_action(Action::Idle);        },
            Some(Action::RollF)       => { self.set_action(Action::Idle);        },
            Some(Action::RollB)       => { self.set_action(Action::Idle);        },
            Some(Action::AerialDodge) => { self.set_action(Action::SpecialFall); },
            Some(Action::SpecialFall) => { self.set_action(Action::SpecialFall); },
            Some(Action::SpecialLand) => { self.set_action(Action::Idle);        },
            Some(Action::TechF)       => { self.set_action(Action::Idle);        },
            Some(Action::TechS)       => { self.set_action(Action::Idle);        },
            Some(Action::TechB)       => { self.set_action(Action::Idle);        },
            Some(Action::Rebound)     => { self.set_action(Action::Idle);        },

            // Attack
            Some(Action::Jab)        => { self.set_action(Action::Idle); },
            Some(Action::Jab2)       => { self.set_action(Action::Idle); },
            Some(Action::Jab3)       => { self.set_action(Action::Idle); },
            Some(Action::Utilt)      => { self.set_action(Action::Idle); },
            Some(Action::Dtilt)      => { self.set_action(Action::Crouch); },
            Some(Action::Ftilt)      => { self.set_action(Action::Idle); },
            Some(Action::DashAttack) => { self.set_action(Action::Idle); },
            Some(Action::Usmash)     => { self.set_action(Action::Idle); },
            Some(Action::Dsmash)     => { self.set_action(Action::Idle); },
            Some(Action::Fsmash)     => { self.set_action(Action::Idle); },
            Some(Action::Grab)       => { self.set_action(Action::Idle); },
            Some(Action::DashGrab)   => { self.set_action(Action::Idle); },

            // Aerials
            Some(Action::Uair)     => { self.set_action(Action::Fall); },
            Some(Action::Dair)     => { self.set_action(Action::Fall); },
            Some(Action::Fair)     => { self.set_action(Action::Fall); },
            Some(Action::Bair)     => { self.set_action(Action::Fall); },
            Some(Action::Nair)     => { self.set_action(Action::Fall); },
            Some(Action::UairLand) => { self.set_action(Action::Idle); },
            Some(Action::DairLand) => { self.set_action(Action::Idle); },
            Some(Action::FairLand) => { self.set_action(Action::Idle); },
            Some(Action::BairLand) => { self.set_action(Action::Idle); },
            Some(Action::NairLand) => { self.set_action(Action::Idle); },

            // Taunts
            Some(Action::TauntUp)    => { self.set_action(Action::Idle); },
            Some(Action::TauntDown)  => { self.set_action(Action::Idle); },
            Some(Action::TauntLeft)  => { self.set_action(Action::Idle); },
            Some(Action::TauntRight) => { self.set_action(Action::Idle); },
        };
    }

    pub fn relative_f(&self, input: f32) -> f32 {
        if self.face_right {
            input
        }
        else {
            input * -1.0
        }
    }

    pub fn relative_frame(&self, frame: &ActionFrame) -> ActionFrame {
        let mut frame = frame.clone();

        // fix hitboxes
        for mut colbox in &mut frame.colboxes[..] {
            let (x, y) = colbox.point;
            colbox.point = (self.relative_f(x), y);
            if let &mut CollisionBoxRole::Hit (ref mut hitbox) = &mut colbox.role {
                if !self.face_right {
                    hitbox.angle = 180.0 - hitbox.angle
                };
            }
        }

        // fix effects
        let mut relative_effects: Vec<FrameEffect> = vec!();
        for effect in frame.effects {
            relative_effects.push(
                match effect {
                    FrameEffect::Velocity     { x, y } => { FrameEffect::Velocity     { x: self.relative_f(x), y: y } },
                    FrameEffect::Acceleration { x, y } => { FrameEffect::Acceleration { x: self.relative_f(x), y: y } },
                    //_                                  => { effect }, // When the time comes, uncomment this
                }
            );
        }
        frame.effects = relative_effects;
        frame
    }

    fn specialfall_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        self.fall_action(input, fighter);
        self.air_drift(input, fighter);
    }

    fn fall_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        if !self.fastfalled {
            if input[0].stick_y < -0.65 && input[3].stick_y > -0.1 && self.y_vel < 0.0 {
                self.fastfalled = true;
                self.y_vel = fighter.fastfall_terminal_vel;
            }
            else {
                self.y_vel += fighter.gravity;
                if self.y_vel < fighter.terminal_vel {
                    self.y_vel = fighter.terminal_vel;
                }
            }
        }
    }

    /*
     *  Begin physics section
     */

    fn physics_step(&mut self, fighter: &Fighter, stage: &Stage) {
        if self.airbourne {
            self.bps_x += self.x_vel + self.kb_x_vel;
            self.bps_y += match self.land_stage_collision(stage, self.y_vel + self.kb_y_vel) {
                None => { self.y_vel + self.kb_y_vel},
                Some(platform) => {
                    self.land(fighter);
                    let self_y = self.bps_y + self.ecb.bot_y;
                    let plat_y = platform.y + platform.h / 2.0;
                    plat_y - self_y
                },
            };
        }
        else {
            self.bps_x += self.x_vel + self.kb_x_vel;
            self.bps_y += self.y_vel + self.kb_y_vel;
        }

        if self.kb_x_vel.abs() > 0.0 {
            let vel_dir = self.kb_x_vel.signum();
            if self.airbourne {
                self.kb_x_vel -= self.kb_x_dec;
            } else {
                self.kb_x_vel -= vel_dir * fighter.friction;
            }
            if vel_dir != self.kb_x_vel.signum() {
                self.kb_x_vel = 0.0;
            }
        }

        if self.kb_y_vel.abs() > 0.0 {
            if self.airbourne {
                let vel_dir = self.kb_y_vel.signum();
                self.kb_y_vel -= self.kb_y_dec;
                if vel_dir != self.kb_y_vel.signum() {
                    self.kb_y_vel = 0.0;
                }
            }
            else {
                self.kb_y_vel = 0.0;
            }
        }

        // are we on a platform?
        match self.land_stage_collision(stage, -0.001) {
            Some(_) if self.airbourne && self.frame > 2 => { // TODO: I dunno what I want to do instead of checking self.frame ...
                self.land(fighter);
            },
            None if !self.airbourne => {
                self.fall();
            },
            _ => { },
        }

        // death
        let blast = &stage.blast;
        if self.bps_x < blast.left || self.bps_x > blast.right || self.bps_y < blast.bot || self.bps_y > blast.top {
            self.die(fighter);
        }
    }

    fn apply_friction(&mut self, fighter: &Fighter) {
        if self.x_vel > 0.0 {
            self.x_vel -= fighter.friction;
            if self.x_vel < 0.0 {
                self.x_vel = 0.0;
            }
        }
        else {
            self.x_vel += fighter.friction;
            if self.x_vel > 0.0 {
                self.x_vel = 0.0;
            }
        }
    }

    /// return the platform that the player would land on if moved by y_offset
    fn land_stage_collision<'a> (&self, stage: &'a Stage, y_offset: f32) -> Option<&'a Platform> {
        if self.y_vel > 0.0 {
            return None;
        }

        for platform in &stage.platforms[..] {
            if platform.pass_through && self.pass_through {
                continue;
            }

            let self_x = self.bps_x;
            let self_y = self.bps_y + self.ecb.bot_y + y_offset;

            let plat_x1 = platform.x - platform.w / 2.0;
            let plat_x2 = platform.x + platform.w / 2.0;
            let plat_y1 = platform.y - platform.h / 2.0;
            let plat_y2 = platform.y + platform.h / 2.0;

            if self_x > plat_x1 && self_x < plat_x2 && self_y > plat_y1 && self_y < plat_y2 {
                return Some(platform)
                // TODO: GAH, need to refactor to set PassPlatform state
            }
        }
        None
    }

    /// Returns the area sorounding the player that the camera must include
    pub fn cam_area(&self, cam_max: &Area) -> Area {
        let mut left  = self.bps_x;
        let mut right = self.bps_x;
        let mut bot   = self.bps_y - 5.0;
        let mut top   = self.bps_y + 25.0;

        if self.face_right {
            left  -= 7.0;
            right += 40.0;
        }
        else {
            left  -= 40.0;
            right += 7.0;
        }

        if left < cam_max.left {
            let diff = left - cam_max.left;
            left  -= diff;
            right -= diff;
        }
        else if right > cam_max.right {
            let diff = right - cam_max.right;
            left  -= diff;
            right -= diff;
        }

        if bot < cam_max.bot {
            let diff = bot - cam_max.bot;
            bot -= diff;
            top -= diff;
        }
        else if top > cam_max.top {
            let diff = top - cam_max.top;
            bot -= diff;
            top -= diff;
        }

        Area {
            left:  left,
            right: right,
            bot:   bot,
            top:   top,
        }
    }

    fn land(&mut self, fighter: &Fighter) {
        match Action::from_u64(self.action) {
            Some(Action::Uair)      => { self.set_action(Action::UairLand) },
            Some(Action::Dair)      => { self.set_action(Action::DairLand) },
            Some(Action::Fair)      => { self.set_action(Action::FairLand) },
            Some(Action::Bair)      => { self.set_action(Action::BairLand) },
            Some(Action::Nair)      => { self.set_action(Action::NairLand) },
            _ if self.y_vel >= -1.0 => { self.set_action(Action::Idle) }, // no impact land

            Some(Action::SpecialFall) |
            Some(Action::AerialDodge) |
            None => { self.set_action(Action::SpecialLand) },

            Some(_) => { self.set_action(Action::Land) },
        }

        self.y_vel = 0.0;
        self.airbourne = false;
        self.fastfalled = false;
        self.air_jumps_left = fighter.air_jumps;
    }

    fn walk(&mut self, fighter: &Fighter) {
        let walk_init_vel = self.relative_f(fighter.walk_init_vel);
        if (walk_init_vel > 0.0 && self.x_vel < walk_init_vel) ||
           (walk_init_vel < 0.0 && self.x_vel > walk_init_vel) {
            self.x_vel += walk_init_vel;
        }
        self.set_action(Action::Walk);
    }

    fn dash(&mut self, fighter: &Fighter) {
        self.x_vel = self.relative_f(fighter.dash_init_vel);
        self.set_action(Action::Dash);
    }

    fn turn(&mut self) {
        self.face_right = !self.face_right;
        self.set_action(Action::Turn);
    }

    fn turn_dash(&mut self) {
        self.face_right = !self.face_right;
        self.set_action(Action::TurnDash);
    }

    fn fall(&mut self) {
        self.airbourne = true;
        self.fastfalled = false;
        self.set_action(Action::Fall);
    }

    fn die(&mut self, fighter: &Fighter) {
        self.stocks -= 1;
        self.damage = 0.0;
        self.bps_x = self.respawn.x;
        self.bps_y = self.respawn.y;
        self.face_right = self.respawn.face_right;
        self.x_vel = 0.0;
        self.y_vel = 0.0;
        self.kb_x_vel = 0.0;
        self.kb_y_vel = 0.0;
        self.air_jumps_left = fighter.air_jumps;
        self.fastfalled = false;
        self.set_action(Action::Spawn);
    }

    pub fn debug_print(&self, fighter: &Fighter, player_input: &PlayerInput, debug: &DebugPlayer, index: usize) {
        if debug.physics {
            println!("Player: {}    x: {}    y: {}    x_vel: {:.5}    y_vel: {:.5}    kb_x_vel: {:.5}    kb_y_vel: {:.5} ",
                index, self.bps_x, self.bps_y, self.x_vel, self.y_vel, self.kb_x_vel, self.kb_y_vel);
        }

        if debug.input {
            let stick_x   = player_input.stick_x.value;
            let stick_y   = player_input.stick_y.value;
            let c_stick_x = player_input.c_stick_x.value;
            let c_stick_y = player_input.c_stick_y.value;
            let l_trigger = player_input.l_trigger.value;
            let r_trigger = player_input.r_trigger.value;

            println!("Player: {}    VALUE    stick_x: {:.5}    stick_y: {:.5}    c_stick_x: {:.5}    c_stick_y: {:.5}    l_trigger: {:.5}    r_trigger: {:.5}",
                index, stick_x, stick_y, c_stick_x, c_stick_y, l_trigger, r_trigger);
        }

        if debug.input_diff {
            let stick_x   = player_input.stick_x.diff;
            let stick_y   = player_input.stick_y.diff;
            let c_stick_x = player_input.c_stick_x.diff;
            let c_stick_y = player_input.c_stick_y.diff;
            let l_trigger = player_input.l_trigger.diff;
            let r_trigger = player_input.r_trigger.diff;

            println!("Player: {}    DIFF    stick_x: {:.5}    stick_y: {:.5}    c_stick_x: {:.5}    c_stick_y: {:.5}    l_trigger: {:.5}    r_trigger: {:.5}",
                index, stick_x, stick_y, c_stick_x, c_stick_y, l_trigger, r_trigger);
        }

        if debug.action {
            let action = Action::from_u64(self.action).unwrap();
            let action_frames = fighter.actions[self.action as usize].frames.len() as u64 - 1;
            let iasa = fighter.actions[self.action as usize].iasa;

            println!("Player: {}    action: {:?}    airbourne: {}    frame: {}/{}    IASA: {}",
                index, action, self.airbourne, self.frame, action_frames, iasa);
        }

        if debug.frame {
            let frames = &fighter.actions[self.action as usize].frames;
            if frames.len() > self.frame as usize {
                let frame = &frames[self.frame as usize];
                let hitbox_count = frame.colboxes.len();
                let effects_count = frame.effects.len();
                println!("Player: {}    colboxes: {}    effects: {}",
                    index, hitbox_count, effects_count);
            }
            else {
                println!("Player: {}    frame {} does not exist.", index, self.frame);
            }
        }
    }

    pub fn render(&self, fighter_color: [f32; 4], fighter: usize, selected_colboxes: HashSet<usize>, fighter_selected: bool, player_selected: bool, debug: DebugPlayer) -> RenderPlayer {
        RenderPlayer {
            debug:             debug,
            bps:               (self.bps_x, self.bps_y),
            ecb:               self.ecb.clone(),
            frame:             self.frame as usize,
            action:            self.action as usize,
            fighter:           fighter,
            face_right:        self.face_right,
            fighter_color:     fighter_color,
            fighter_selected:  fighter_selected,
            player_selected:   player_selected,
            selected_colboxes: selected_colboxes,
        }
    }
}

enum JumpResult {
    Button,
    Stick,
    None,
}

impl JumpResult {
    fn jump(&self) -> bool {
        match *self {
            JumpResult::Button | JumpResult::Stick => true,
            JumpResult::None => false
        }
    }
}

pub struct RenderPlayer {
    pub debug:             DebugPlayer,
    pub bps:               (f32, f32),
    pub ecb:               ECB,
    pub frame:             usize,
    pub action:            usize,
    pub fighter:           usize,
    pub face_right:        bool,
    pub fighter_color:     [f32; 4],
    pub fighter_selected:  bool,
    pub player_selected:   bool,
    pub selected_colboxes: HashSet<usize>,
}

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct DebugPlayer {
    pub physics:        bool,
    pub input:          bool,
    pub input_diff:     bool,
    pub action:         bool,
    pub frame:          bool,
    pub stick_vector:   bool,
    pub c_stick_vector: bool,
    pub di_vector:      bool,
    pub ecb:            bool,
    pub fighter:        RenderFighter,
    pub cam_area:       bool,
}

#[derive(Clone, Serialize, Deserialize, Node)]
pub enum RenderFighter {
    Normal,
    Debug,
    None,
}

impl Default for RenderFighter {
    fn default() -> RenderFighter {
        RenderFighter::Normal
    }
}
