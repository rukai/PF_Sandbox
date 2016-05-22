use ::fighter::*;
use ::controller::Control;

use num::FromPrimitive;

#[derive(Clone)]
pub struct Player {
    pub fighter:    Fighter,
    action:         u64,
    action_count:   u64,
    pub stocks:     u64,
    pub damage:     u64,
    pub x:          f32,
    pub y:          f32,
    pub w:          f32,
    pub h:          f32,
    pub face_right: bool,
}

impl Player {
    pub fn new() -> Player {
        Player {
            fighter: Fighter::base(),
            action: Action::Spawn as u64,
            action_count: 0,
            stocks: 0,
            damage: 0,
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
            face_right: true,
        }
    }

    pub fn step(&mut self, controls: &Control) {
        let action_frames = self.fighter.action_defs[self.action as usize].frames.len() as u64;
        if self.action_count == action_frames - 1 {
            self.action_expired();
        }
        
        // PLAN: A series of match's to handle cases that can occur on multiple states
        
        match Action::from_u64(self.action) {
            Some(Action::Idle) | Some(Action::Run) => {
                if controls.y || controls.x {
                    self.set_action(Action::JumpSquat);
                }
            },
            _ => { },
        }

        println!("\nFighter: {}", self.fighter.name);
        println!("action_count: {}", self.action_count);
        println!("action: {:?}", Action::from_u64(self.action));

        self.action_count += 1;
    }

    pub fn action_expired(&mut self) {
        match Action::from_u64(self.action) {
            Some(Action::Spawn)       => { self.set_action(Action::SpawnIdle);   },
            Some(Action::SpawnIdle)   => { self.set_action(Action::Fall);        },
            Some(Action::Fall)        => { self.set_action(Action::Fall);        },
            Some(Action::AerialFall)  => { self.set_action(Action::AerialFall);  },
            Some(Action::Land)        => { self.set_action(Action::Idle);        },
            Some(Action::Idle)        => { self.set_action(Action::Idle);        },
            Some(Action::JumpSquat)   => { self.set_action(Action::JumpF);       }, //TODO: Or JumpB
            Some(Action::JumpF)       => { self.set_action(Action::Fall);        },
            Some(Action::JumpB)       => { self.set_action(Action::Fall);        },
            Some(Action::JumpAerialF) => { self.set_action(Action::AerialFall);  },
            Some(Action::JumpAerialB) => { self.set_action(Action::AerialFall);  },
            Some(Action::Turn)        => { self.set_action(Action::Turn);        }, //TODO: I guess this is highly dependent on some other state
            Some(Action::Dash)        => { self.set_action(Action::Dash);        },
            Some(Action::Run)         => { self.set_action(Action::RunEnd);      },
            Some(Action::RunEnd)      => { self.set_action(Action::Idle);        },
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
            None                      => { panic!("Custom defined action expirations have not been implemented"); },
        };
    }
    
    //always change self.action through this method
    fn set_action(&mut self, action: Action) {
        self.action = action as u64;
        self.action_count = 0;
    }
}
