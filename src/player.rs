use ::fighter::*;
use ::controller::Control;

use num::FromPrimitive;

#[derive(Clone)]
pub struct Player {
    action:         u64,
    action_count:   u64,
    pub stocks:     u64,
    pub damage:     u64,
    pub bps_x:      f64,
    pub bps_y:      f64,
    pub ecb_w:      f64,
    pub ecb_top:    f64, // Relative to bps_y
    pub ecb_bottom: f64, // Relative to bps_y
    pub face_right: bool,
}

impl Player {
    pub fn new() -> Player {
        Player {
            action:       Action::Spawn as u64,
            action_count: 0,
            stocks:       0,
            damage:       0,
            bps_x:        0.0,
            bps_y:        0.0,
            ecb_w:        0.0,
            ecb_top:      0.0,
            ecb_bottom:   0.0,
            face_right:   true,
        }
    }

    pub fn step(&mut self, controls: &Control, fighter: &Fighter) {
        let action_frames = fighter.action_defs[self.action as usize].frames.len() as u64;
        if self.action_count == action_frames - 1 {
            self.action_expired();
        }

        let frame = &fighter.action_defs[self.action as usize].frames[self.action_count as usize];
        let action = Action::from_u64(self.action);

        self.ecb_w = frame.ecb_w;
        self.ecb_top = frame.ecb_top;
        self.ecb_bottom = match action {
            Some(Action::JumpF) | Some(Action::JumpB) | Some(Action::JumpAerialF) | Some(Action::JumpAerialB) if self.action_count < 10
                => self.ecb_bottom,
            _   => frame.ecb_bottom,
        };

        match action {
            Some(Action::Idle) | Some(Action::Run) => {
                if controls.y || controls.x {
                    self.set_action(Action::JumpSquat);
                }
            },
            _ => { },
        }

        println!("\nFighter: {}", fighter.name);
        println!("action_count: {}", self.action_count);
        println!("action: {:?}", Action::from_u64(self.action).unwrap());

        self.action_count += 1;
    }

    fn action_expired(&mut self) {
        match Action::from_u64(self.action) {
            None => { panic!("Custom defined action expirations have not been implemented"); },

            // Idle
            Some(Action::Spawn)     => { self.set_action(Action::SpawnIdle); },
            Some(Action::SpawnIdle) => { self.set_action(Action::Fall);      },
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
}
