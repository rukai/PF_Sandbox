use ::fighter::*;
use ::input::{PlayerInput};
use ::game::Point;

use num::FromPrimitive;

#[derive(Clone)]
pub struct Player {
    action:         u64,
    action_count:   u64,
    pub stocks:     u64,
    pub damage:     u64,
    pub bps:        Point,
    pub x_vel:      f64,
    pub y_vel:      f64,
    pub ecb_w:      f64,
    pub ecb_y:      f64, // relative to bps.y. when 0, the bottom of the ecb touches the bps
    pub ecb_top:    f64, // Relative to ecb_y
    pub ecb_bottom: f64, // Relative to ecb_y
    pub face_right: bool,
    pub airbourne:  bool,
    pub jumps_left: u64,
}

impl Player {
    pub fn new(spawn: Point, stocks: u64) -> Player {
        Player {
            action:       Action::Spawn as u64,
            action_count: 0,
            stocks:       stocks,
            damage:       0,
            bps:          spawn,
            x_vel:        0.0,
            y_vel:        0.0,
            ecb_w:        0.0,
            ecb_y:        0.0,
            ecb_top:      0.0,
            ecb_bottom:   0.0,
            face_right:   true,
            jumps_left:   0,
            airbourne:    true,
        }
    }

    pub fn step(&mut self, input: &PlayerInput, fighter: &Fighter) {
        if input.plugged_in {
            self.input_step(input, fighter);
        }
        self.physics_step(fighter);
    }

    fn input_step(&mut self, input: &PlayerInput, fighter: &Fighter) {
        let action_frames = fighter.action_defs[self.action as usize].frames.len() as u64;
        if self.action_count == action_frames - 1 {
            self.action_expired();
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
            Some(Action::Land) | Some(Action::UairLand) | Some(Action::DairLand) |
            Some(Action::FairLand) | Some(Action::NairLand) => { self.land_action(fighter); },
            Some(Action::SpawnIdle) | Some(Action::Fall) | Some(Action::AerialFall) |
            Some(Action::JumpF) | Some(Action::JumpB) | Some(Action::JumpAerialF) | 
            Some(Action::JumpAerialB)                 => { self.aerial_action(input, fighter); },
            Some(Action::Idle) | Some(Action::Crouch) => { self.ground_idle_action(input); },
            Some(Action::Dash)                        => { self.dash_action(input); },
            Some(Action::Run)                         => { self.run_action(input); },
            _                                         => { },
        }

        println!("\naction_count: {}", self.action_count);
        println!("action: {:?}", Action::from_u64(self.action).unwrap());

        self.action_count += 1;
    }

    fn physics_step(&mut self, fighter: &Fighter) {
        self.airbourne = true; // TODO: Run a collision check

        if self.airbourne {
            self.y_vel += fighter.gravity;
            if self.y_vel < fighter.terminal_vel {
                self.y_vel = fighter.terminal_vel;
            }

            self.bps.x += self.x_vel;
            self.bps.y += self.y_vel;
        }
        else {
            self.bps.x += self.x_vel;
        }

        if self.bps.x > 200.0 || self.bps.x < -200.0 || self.bps.y > 200.0 || self.bps.y < -200.0 {
            self.die(fighter);
        }
    }

    fn aerial_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        if input.a || input.z {
            self.set_action(Action::Nair);
        }
        else if input.b {
            // special attack
        }
        else if (input.x || input.y) && self.jumps_left > 0 {
            self.jumps_left -= 1;
            self.y_vel = fighter.jump_y_init_vel;

            if self.relative_stick(input.stick_x) < -30 {
                self.set_action(Action::JumpAerialB);
            }
            else {
                self.set_action(Action::JumpAerialF);
            }
        }
        else if input.l || input.r {
            self.set_action(Action::AerialDodge);
        }
        else {
            self.x_vel = (input.stick_x as f64) / 50.0;
        }
    }

    fn ground_idle_action(&mut self, input: &PlayerInput) {
        if input.y || input.x {
            self.set_action(Action::JumpSquat);
        }
        else if input.a {
            self.set_action(Action::Jab);
        }
        else if input.b {
            // special attack
        }
        else if input.r {
            self.set_action(Action::Grab);
        }
    }

    fn dash_action(&mut self, input: &PlayerInput) {
        if input.y || input.x {
            self.set_action(Action::JumpSquat);
        }
        if self.relative_stick(input.stick_x) < -100 {
            self.face_right = !self.face_right;
            self.set_action(Action::Dash);
        }
    }
    fn run_action(&mut self, input: &PlayerInput) {
        if input.y || input.x {
            self.set_action(Action::JumpSquat);
        }
        if input.a {
            self.set_action(Action::DashAttack);
        }
        if input.z {
            self.set_action(Action::DashGrab);
        }
    }

    fn land_action(&mut self, fighter: &Fighter) {
        if self.action_count == 0 {
            self.jumps_left = fighter.jumps;
        }
    }

    fn die(&mut self, fighter: &Fighter) {
        self.stocks -= 1;
        self.bps.x = 0.0;
        self.bps.y = 0.0;
        self.y_vel = 0.0;
        self.x_vel = 0.0;
        self.jumps_left = fighter.jumps;
        self.set_action(Action::Spawn);
    }

    fn action_expired(&mut self) {
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
            Some(Action::JumpSquat)   => { self.set_action(Action::JumpF);      }, //TODO: Or JumpB
            Some(Action::JumpF)       => { self.set_action(Action::Fall);       },
            Some(Action::JumpB)       => { self.set_action(Action::Fall);       },
            Some(Action::JumpAerialF) => { self.set_action(Action::AerialFall); },
            Some(Action::JumpAerialB) => { self.set_action(Action::AerialFall); },
            Some(Action::Turn)        => { self.set_action(Action::Turn);       }, //TODO: I guess this is highly dependent on some other state
            Some(Action::Dash)        => { self.set_action(Action::Dash);       },
            Some(Action::Run)         => { self.set_action(Action::RunEnd);     },
            Some(Action::RunEnd)      => { self.set_action(Action::Idle);       },

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
        };
    }
    
    //always change self.action through this method
    fn set_action(&mut self, action: Action) {
        self.action = action as u64;
        self.action_count = 0;
    }

    fn relative_stick(&self, input: i8) -> i8 {
        if self.face_right {
            input
        }
        else {
            input * -1
        }
    }
}
