use ::fighter::*;
use ::game::Point;
use ::input::{PlayerInput};
use ::stage::{Stage, Platform};

use num::FromPrimitive;

#[derive(Clone)]
pub struct Player {
    action:             u64,
    action_count:       u64,
    pub stocks:         u64,
    pub damage:         u64,
    pub bps:            Point,
    pub spawn:          Point,
    pub x_vel:          f64,
    pub y_vel:          f64,
    pub ecb_w:          f64,
    pub ecb_y:          f64, // relative to bps.y. when 0, the bottom of the ecb touches the bps
    pub ecb_top:        f64, // Relative to ecb_y
    pub ecb_bottom:     f64, // Relative to ecb_y
    pub face_right:     bool,
    pub airbourne:      bool,
    pub pass_through:   bool,
    pub air_jumps_left: u64,
}

impl Player {
    pub fn new(spawn: Point, stocks: u64) -> Player {
        Player {
            action:         Action::Spawn as u64,
            action_count:   0,
            stocks:         stocks,
            damage:         0,
            bps:            spawn.clone(),
            spawn:          spawn,
            x_vel:          0.0,
            y_vel:          0.0,
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

    //always change self.action through this method
    fn set_action(&mut self, action: Action) {
        self.action = action as u64;
        self.action_count = 0;
    }

    pub fn step(&mut self, input: &PlayerInput, fighter: &Fighter, stage: &Stage) {
        self.input_step(input, fighter);
        self.physics_step(fighter, stage);
    }

    /*
     *  Begin input section
     */

    fn input_step(&mut self, input: &PlayerInput, fighter: &Fighter) {
        let action_frames = fighter.action_defs[self.action as usize].frames.len() as u64;
        if self.action_count == action_frames - 1 {
            self.action_expired(input, fighter);
        }

        let frame = &fighter.action_defs[self.action as usize].frames[self.action_count as usize];
        let action = Action::from_u64(self.action);

        // update ecb
        self.ecb_w = frame.ecb_w;
        self.ecb_y = frame.ecb_y;
        self.ecb_top = frame.ecb_h / 2.0;
        self.ecb_bottom = match action {
            Some(Action::JumpF) | Some(Action::JumpB) | Some(Action::JumpAerialF) | Some(Action::JumpAerialB) if self.action_count < 10
                => self.ecb_bottom,
            _   => -frame.ecb_h / 2.0,
        };

        match action {
            Some(Action::SpawnIdle) | Some(Action::Fall) | Some(Action::AerialFall) |
            Some(Action::JumpF) | Some(Action::JumpB) | Some(Action::JumpAerialF) | 
            Some(Action::JumpAerialB)                 => { self.aerial_action(input, fighter); },
            Some(Action::Idle) | Some(Action::Crouch) => { self.ground_idle_action(input); },
            Some(Action::Dash)                        => { self.dash_action(input); },
            Some(Action::Run)                         => { self.run_action(input); },
            _                                         => { },
        }

        if input.plugged_in {
            println!("\naction_count: {}", self.action_count);
            println!("airbourne: {}", self.airbourne);
            println!("action: {:?}", Action::from_u64(self.action).unwrap());
        }

        self.action_count += 1;
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

            if self.relative_stick(input.stick_x.value) < -30 { // TODO: refine
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
            self.x_vel = (input.stick_x.value as f64) / 50.0;
        }

        self.pass_through = input.stick_y.value < -70; // TODO: refine
    }

    fn ground_idle_action(&mut self, input: &PlayerInput) {
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
        else if input.c_stick_x.value.abs() > 70 { //TODO: hmmmm how do I want to stop smashes from auto-spamming
            self.face_right = input.c_stick_x.value > 0;
            self.set_action(Action::Fsmash);
        }
        else if input.c_stick_y.value > 70 {
            self.set_action(Action::Usmash);
        }
        else if input.c_stick_y.value < -70 {
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
        else {
            self.x_vel = (input.stick_x.value as f64) / 50.0;
        }

        self.pass_through = input.stick_y.diff < -30; // TODO: refine
    }

    fn dash_action(&mut self, input: &PlayerInput) {
        if input.y.press || input.x.press {
            self.set_action(Action::JumpSquat);
        }
        if self.relative_stick(input.stick_x.value) < -100 { //TODO: refine
            self.face_right = !self.face_right;
            self.set_action(Action::Dash);
        }
    }

    fn run_action(&mut self, input: &PlayerInput) {
        if self.check_jump(input) {
            self.set_action(Action::JumpSquat);
        }
        if input.a.press {
            self.set_action(Action::DashAttack);
        }
        if input.z.press {
            self.set_action(Action::DashGrab);
        }
    }

    fn check_jump(&self, input: &PlayerInput) -> bool {
        input.x.press || input.y.press || input.stick_y.diff > 30 // TODO: refine
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
            Some(Action::Turn)        => { self.set_action(Action::Turn);       }, //TODO: I guess this is highly dependent on some other state
            Some(Action::Dash)        => { self.set_action(Action::Dash);       },
            Some(Action::Run)         => { self.set_action(Action::RunEnd);     },
            Some(Action::RunEnd)      => { self.set_action(Action::Idle);       },
            Some(Action::JumpSquat)   => {
                self.airbourne = true;

                let shorthop = input.x.value || input.y.value || input.stick_y.value > 50; // TODO: refine

                if shorthop {
                    self.y_vel = fighter.jump_y_init_vel;
                } else {
                    self.y_vel = fighter.jump_y_init_vel_short;
                }

                if self.relative_stick(input.stick_x.value) < -30 { // TODO: refine
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

    fn relative_stick(&self, input: i16) -> i16 {
        if self.face_right {
            input
        }
        else {
            input * -1
        }
    }

    /*
     *  Begin physics section
     */

    fn physics_step(&mut self, fighter: &Fighter, stage: &Stage) {

        // are we on a platform?
        match self.land_stage_collision(stage, -0.001) {
            Some(_) if self.airbourne && self.action_count > 2 => { // TODO: I dunno what I want to do instead of checking self.action_count ...
                self.land(fighter);
            },
            None if !self.airbourne => {
                self.fall();
            },
            _ => { },
        }

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
            self.bps.x += self.x_vel;

            if fighter.friction > self.x_vel.abs() {
                self.x_vel = 0.0;
            } else if self.x_vel > 0.0 {
                self.x_vel -= fighter.friction;
            } else {
                self.x_vel += fighter.friction;
            }
        }

        // death
        if self.bps.x < stage.lower_bounds.x || self.bps.x > stage.higher_bounds.x || self.bps.y < stage.lower_bounds.y || self.bps.y > stage.higher_bounds.y {
            self.die(fighter);
        }
    }

    /// return the platform that the player would land on if moved by y_offset
    fn land_stage_collision<'a> (&self, stage: &'a Stage, y_offset: f64) -> Option<&'a Platform> {
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
            Some(Action::Uair) => { self.set_action(Action::UairLand) },
            Some(Action::Dair) => { self.set_action(Action::DairLand) },
            Some(Action::Fair) => { self.set_action(Action::FairLand) },
            Some(Action::Nair) => { self.set_action(Action::NairLand) },
            Some(_) | None     => { self.set_action(Action::Land);    },
        }
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
}
