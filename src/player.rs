use ::fighter::*;
use ::game::Point;
use ::input::{PlayerInput};
use ::stage::{Stage, Platform};

use num::FromPrimitive;

#[derive(Clone)]
pub struct Player {
    pub action:         u64,
    action_new:         u64,
    action_set:         bool,
    pub frame:          u64,
    pub stocks:         u64,
    pub damage:         u64,
    pub bps:            Point,
    pub spawn:          Point,
    pub x_vel:          f32,
    pub y_vel:          f32,
    pub x_acc:          f32,
    pub y_acc:          f32,
    pub ecb_w:          f32,
    pub ecb_y:          f32, // relative to bps.y. when 0, the bottom of the ecb touches the bps
    pub ecb_top:        f32, // Relative to ecb_y
    pub ecb_bottom:     f32, // Relative to ecb_y
    pub face_right:     bool,
    pub airbourne:      bool,
    pub pass_through:   bool,
    pub air_jumps_left: u64,
}

impl Player {
    pub fn new(spawn: Point, stocks: u64) -> Player {
        Player {
            action:         Action::Spawn as u64,
            action_new:     Action::Spawn as u64,
            action_set:     false,
            frame:          0,
            stocks:         stocks,
            damage:         0,
            bps:            spawn.clone(),
            spawn:          spawn,
            x_vel:          0.0,
            y_vel:          0.0,
            x_acc:          0.0,
            y_acc:          0.0,
            ecb_w:          0.0,
            ecb_y:          0.0,
            ecb_top:        0.0,
            ecb_bottom:     0.0,
            face_right:     true,
            airbourne:      true,
            pass_through:   false,
            air_jumps_left: 0,
        }
    }

    // always change self.action through this method
    fn set_action(&mut self, action: Action) {
        self.action_new = action as u64;
        self.action_set = true;
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
        let action_frames = fighter.action_defs[self.action as usize].frames.len() as u64;

        // handles a frame index that no longer exists by jumping to the last existing frame
        if self.frame >= action_frames - 1 {
            self.frame = action_frames - 1;
        }

        if self.frame == action_frames - 1 {
            self.action_expired(input, fighter);
        }

        let fighter_frame = &fighter.action_defs[self.action as usize].frames[self.frame as usize];
        let action = Action::from_u64(self.action);

        // update ecb
        self.ecb_w = fighter_frame.ecb_w;
        self.ecb_y = fighter_frame.ecb_y;
        self.ecb_top = fighter_frame.ecb_h / 2.0;
        self.ecb_bottom = match action {
            //TODO: Err does this if apply to all Some()?
            Some(Action::JumpF) | Some(Action::JumpB) | Some(Action::JumpAerialF) | Some(Action::JumpAerialB) if self.frame < 10
                => self.ecb_bottom,
            _   => -fighter_frame.ecb_h / 2.0,
        };

        match action {
            Some(Action::SpawnIdle) | Some(Action::Fall) | Some(Action::AerialFall) |
            Some(Action::JumpF) | Some(Action::JumpB) | Some(Action::JumpAerialF) | 
            Some(Action::JumpAerialB)                 => { self.aerial_action(input, fighter); },
            Some(Action::Idle) | Some(Action::Crouch) => { self.ground_idle_action(input, fighter); },
            Some(Action::Dash)                        => { self.dash_action(input, fighter); },
            Some(Action::Run)                         => { self.run_action(input, fighter); },
            _                                         => { },
        }
    }

    fn aerial_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        if input.a.press || input.z.press {
            self.set_action(Action::Nair);
        }
        else if input.b.press {
            // special attack
        }
        else if self.check_jump(input) && self.air_jumps_left > 0 {
            self.air_jumps_left -= 1;
            self.y_vel = fighter.jump_y_init_vel;

            if self.relative_f(input.stick_x.value) < -0.1 { // TODO: refine
                self.set_action(Action::JumpAerialB);
            }
            else {
                self.set_action(Action::JumpAerialF);
            }
        }
        else if input.l.press || input.r.press {
            self.set_action(Action::AerialDodge);
        }
        else {
            self.x_vel = input.stick_x.value * 2.5
        }

        self.pass_through = input.stick_y.value < -0.2; // TODO: refine
    }

    fn ground_idle_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        if input.stick_y.value < -0.3 { // TODO: refine
            self.set_action(Action::Crouch);
        }
        else if input.stick_x.diff > 0.1 && input.stick_x.value > 0.1 {
            self.face_right = true;
            self.dash(fighter);
        }
        else if input.stick_x.diff < -0.1 && input.stick_x.value < -0.1 {
            self.face_right = false;
            self.dash(fighter);
        }

        if self.check_jump(input) {
            self.set_action(Action::JumpSquat);
        }
        else if input.a.press {
            self.set_action(Action::Jab);
        }
        else if input.b.press {
            // special attack
        }
        else if input.z.press {
            self.set_action(Action::Grab);
        }
        else if input.c_stick_x.value.abs() > 0.2 { // TODO: hmmmm how do I want to stop smashes from auto-spamming
            self.face_right = input.c_stick_x.value > 0.0;
            self.set_action(Action::Fsmash);
        }
        else if input.c_stick_y.value > 0.2 {
            self.set_action(Action::Usmash);
        }
        else if input.c_stick_y.value < -0.2 {
            self.set_action(Action::Dsmash);
        }
        else if input.up.press {
            self.set_action(Action::TauntUp);
        }
        else if input.down.press {
            self.set_action(Action::TauntDown);
        }
        else if input.left.press {
            self.set_action(Action::TauntLeft);
        }
        else if input.right.press {
            self.set_action(Action::TauntRight);
        }

        self.pass_through = input.stick_y.diff < -0.1; // TODO: refine
    }

    fn dash_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        let stick_x = self.relative_f(input.stick_x.value);
        if stick_x >= 0.8 || stick_x <= 0.4 { // verified horizontally only
            // TODO: Implement terminal velocity
            let dash_acc = fighter.dash_run_acc_a * stick_x.abs() + fighter.dash_run_acc_b;
            self.x_acc = self.relative_f(dash_acc) * if stick_x > 0.0 { 1.0 } else { -1.0 };
        }

        if input.y.press || input.x.press {
            self.set_action(Action::JumpSquat);
        }
        if self.relative_f(input.stick_x.value) < -0.35 { // TODO: refine
            self.face_right = !self.face_right;
            self.dash(fighter);
        }
    }

    fn run_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        if self.check_jump(input) {
            self.set_action(Action::JumpSquat);
        }
        // verified horizontally stick_x <= 0.61250 ends dash
        else if self.relative_f(input.stick_x.value) <= 0.61300 {
            self.set_action(Action::RunEnd);
        }
        else if input.a.press {
            self.set_action(Action::DashAttack);
        }
        else if input.z.press {
            self.set_action(Action::DashGrab);
        }
        else {
            // Placeholder logic, needs research
            let acc = self.relative_f(fighter.dash_run_acc_b);
            if self.x_vel + acc <= fighter.dash_run_term_vel {
                self.x_acc = acc;
            }
        }
    }

    fn check_jump(&self, input: &PlayerInput) -> bool {
        input.x.press || input.y.press || (input.stick_y.diff > 0.4 && input.stick_y.value > 0.1) // TODO: refine
    }

    fn action_expired(&mut self, input: &PlayerInput, fighter: &Fighter) {
        match Action::from_u64(self.action) {
            None => { panic!("Custom defined action expirations have not been implemented"); },

            // Idle
            Some(Action::Spawn)     => { self.set_action(Action::SpawnIdle); },
            Some(Action::SpawnIdle) => { self.set_action(Action::SpawnIdle); },
            Some(Action::Idle)      => { self.set_action(Action::Idle);      },
            Some(Action::Crouch)    => { self.set_action(Action::Crouch);    },

            // Movement
            Some(Action::Fall)        => { self.set_action(Action::Fall);       },
            Some(Action::AerialFall)  => { self.set_action(Action::AerialFall); },
            Some(Action::Land)        => { self.set_action(Action::Idle);       },
            Some(Action::JumpF)       => { self.set_action(Action::Fall);       },
            Some(Action::JumpB)       => { self.set_action(Action::Fall);       },
            Some(Action::JumpAerialF) => { self.set_action(Action::AerialFall); },
            Some(Action::JumpAerialB) => { self.set_action(Action::AerialFall); },
            Some(Action::Turn)        => { self.set_action(Action::Turn);       }, // TODO: I guess this is highly dependent on some other state
            Some(Action::Dash)        => { self.set_action(Action::Run);        },
            Some(Action::Run)         => { self.set_action(Action::Run);        },
            Some(Action::RunEnd)      => { self.set_action(Action::Idle);       },
            Some(Action::JumpSquat)   => {

                self.airbourne = true;

                let shorthop = input.x.value || input.y.value || input.stick_y.value > 0.15; // TODO: refine

                if shorthop {
                    self.y_vel = fighter.jump_y_init_vel;
                } else {
                    self.y_vel = fighter.jump_y_init_vel_short;
                }

                if self.relative_f(input.stick_x.value) < -0.1 { // TODO: refine
                    self.set_action(Action::JumpB);
                }
                else {
                    self.set_action(Action::JumpF);
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

            // Attack
            Some(Action::Jab)        => { self.set_action(Action::Idle); },
            Some(Action::Jab2)       => { self.set_action(Action::Idle); },
            Some(Action::Jab3)       => { self.set_action(Action::Idle); },
            Some(Action::Utilt)      => { self.set_action(Action::Idle); },
            Some(Action::Dtilt)      => { self.set_action(Action::Idle); },
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
            Some(Action::Nair)     => { self.set_action(Action::Fall); },
            Some(Action::UairLand) => { self.set_action(Action::Idle); },
            Some(Action::DairLand) => { self.set_action(Action::Idle); },
            Some(Action::FairLand) => { self.set_action(Action::Idle); },
            Some(Action::NairLand) => { self.set_action(Action::Idle); },

            // Taunts
            Some(Action::TauntUp)    => { self.set_action(Action::Idle); },
            Some(Action::TauntDown)  => { self.set_action(Action::Idle); },
            Some(Action::TauntLeft)  => { self.set_action(Action::Idle); },
            Some(Action::TauntRight) => { self.set_action(Action::Idle); },
        };
    }

    fn relative_f(&self, input: f32) -> f32 {
        if self.face_right {
            input
        }
        else {
            input * -1.0
        }
    }

    /*
     *  Begin physics section
     */

    fn physics_step(&mut self, fighter: &Fighter, stage: &Stage) {
        // movement
        if self.airbourne {
            self.y_vel += fighter.gravity;
            if self.y_vel < fighter.terminal_vel {
                self.y_vel = fighter.terminal_vel;
            }

            self.bps.x += self.x_vel;
            self.bps.y += match self.land_stage_collision(stage, self.y_vel) {
                None => { self.y_vel },
                Some(platform) => {
                    self.land(fighter);
                    let ecb_y = self.bps.y + self.ecb_y + self.ecb_bottom;
                    let plat_y = platform.y + platform.h / 2.0;
                    plat_y - ecb_y
                },
            };
        }
        else {
            if self.x_acc == 0.0 { // Careful, float equality >:/
                if fighter.friction > self.x_vel.abs() {
                    self.x_vel = 0.0;
                } else if self.x_vel > 0.0 {
                    self.x_vel -= fighter.friction;
                } else {
                    self.x_vel += fighter.friction;
                }
            }
            else {
                self.x_vel += self.x_acc;
            }

            self.bps.x += self.x_vel;
            self.x_acc = 0.0;
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
        if self.bps.x < stage.lower_bounds.x || self.bps.x > stage.higher_bounds.x || self.bps.y < stage.lower_bounds.y || self.bps.y > stage.higher_bounds.y {
            self.die(fighter);
        }
    }

    /// return the platform that the player would land on if moved by y_offset
    fn land_stage_collision<'a> (&self, stage: &'a Stage, y_offset: f32) -> Option<&'a Platform> {
        if self.y_vel > 0.0 {
            return None;
        }

        for platform in &stage.platforms {
            if platform.pass_through && self.pass_through {
                continue;
            }

            let ecb_x = self.bps.x;
            let ecb_y = self.bps.y + self.ecb_y + self.ecb_bottom + y_offset;

            let plat_x1 = platform.x - platform.w / 2.0;
            let plat_x2 = platform.x + platform.w / 2.0;
            let plat_y1 = platform.y - platform.h / 2.0;
            let plat_y2 = platform.y + platform.h / 2.0;

            if ecb_x > plat_x1 && ecb_x < plat_x2 && ecb_y > plat_y1 && ecb_y < plat_y2 {
                return Some(platform)
            }
        }
        None
    }

    fn land(&mut self, fighter: &Fighter) {
        self.airbourne = false;
        self.air_jumps_left = fighter.air_jumps;

        match Action::from_u64(self.action) {
            Some(Action::Uair)      => { self.set_action(Action::UairLand) },
            Some(Action::Dair)      => { self.set_action(Action::DairLand) },
            Some(Action::Fair)      => { self.set_action(Action::FairLand) },
            Some(Action::Nair)      => { self.set_action(Action::NairLand) },
            _ if self.y_vel >= -1.0 => { self.set_action(Action::Idle)     }, // no impact land
            Some(_) | None          => { self.set_action(Action::Land)     },
        }
    }

    fn dash(&mut self, fighter: &Fighter) {
        self.x_acc = self.relative_f(fighter.dash_init_vel);
        self.set_action(Action::Dash);
    }

    fn fall(&mut self) {
        self.y_vel = 0.0;
        self.airbourne = true;
        self.set_action(Action::Fall);
    }

    fn die(&mut self, fighter: &Fighter) {
        self.stocks -= 1;
        self.bps = self.spawn.clone();
        self.y_vel = 0.0;
        self.x_vel = 0.0;
        self.air_jumps_left = fighter.air_jumps;
        self.set_action(Action::Spawn);
    }

    pub fn debug_physics(&self) {
        println!("x_vel: {:.5}    y_vel: {:.5}    x_acc {:.5}", self.x_vel, self.y_vel, self.x_acc);
    }

    pub fn debug_input(&self, input: &PlayerInput) {
        let stick_x   = input.stick_x.value;
        let stick_y   = input.stick_y.value;
        let c_stick_x = input.c_stick_x.value;
        let c_stick_y = input.c_stick_y.value;
        let l_trigger = input.l_trigger.value;
        let r_trigger = input.r_trigger.value;

        println!("VALUE    stick_x: {:.5}    stick_y: {:.5}    c_stick_x: {:.5}    c_stick_y: {:.5}    l_trigger: {:.5}    r_trigger: {:.5}",
            stick_x, stick_y, c_stick_x, c_stick_y, l_trigger, r_trigger);
    }

    pub fn debug_input_diff(&self, input: &PlayerInput) {
        let stick_x   = input.stick_x.diff;
        let stick_y   = input.stick_y.diff;
        let c_stick_x = input.c_stick_x.diff;
        let c_stick_y = input.c_stick_y.diff;
        let l_trigger = input.l_trigger.diff;
        let r_trigger = input.r_trigger.diff;

        println!("DIFF    stick_x: {:.5}    stick_y: {:.5}    c_stick_x: {:.5}    c_stick_y: {:.5}    l_trigger: {:.5}    r_trigger: {:.5}",
            stick_x, stick_y, c_stick_x, c_stick_y, l_trigger, r_trigger);
    }

    pub fn debug_action(&self, fighter: &Fighter) {
        let action = Action::from_u64(self.action).unwrap();
        let action_frames = fighter.action_defs[self.action as usize].frames.len() as u64 - 1;
        let iasa = fighter.action_defs[self.action as usize].iasa;

        println!("action: {:?}    airbourne: {}    frame: {}/{}    IASA: {}",
            action, self.airbourne, self.frame, action_frames, iasa);
    }

    pub fn debug_frame(&self, fighter: &Fighter) {
        let frame = &fighter.action_defs[self.action as usize].frames[self.frame as usize];
        let hitbox_count = frame.hitboxes.len();
        let effects_count = frame.effects.len();
        let ecb_w = frame.ecb_w;
        let ecb_h = frame.ecb_h;
        let ecb_y = frame.ecb_y;
        println!("hitboxes: {}    effects: {}    ecb_w: {:.5}    ecb_h: {:.5}    ecb_y: {:.5}",
            hitbox_count, effects_count, ecb_w, ecb_h, ecb_y);
    }

    pub fn render(&self, fighter: usize) -> RenderPlayer {
        RenderPlayer {
            bps:        self.bps.clone(),
            ecb_enable: true,
            ecb_w:      self.ecb_w,
            ecb_y:      self.ecb_y,
            ecb_top:    self.ecb_top,
            ecb_bottom: self.ecb_bottom,
            frame:      self.frame as usize,
            action:     self.action as usize,
            fighter:    fighter,
        }
    }
}

pub struct RenderPlayer {
    pub bps:        Point,
    pub ecb_enable: bool,
    pub ecb_w:      f32,
    pub ecb_y:      f32,
    pub ecb_top:    f32,
    pub ecb_bottom: f32,
    pub frame:      usize,
    pub action:     usize,
    pub fighter:    usize,
}
