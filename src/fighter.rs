impl Fighter {
    pub fn base() -> Fighter {
        let action_frame1 = ActionFrame {
            colboxes:     vec!(),
            colbox_links: vec!(),
            effects:      vec!(),
            ecb_w:        3.5,
            ecb_h:        12.0,
            ecb_y:        6.0,
        };

        let action_def = ActionDef {
            frames: vec!(action_frame1),
            iasa:   0,
        };
        let mut action_defs: Vec<ActionDef> = Vec::new();
        for _ in 0..((Action::CrouchEnd as usize)+1) { // TODO: Super gross but what is a man to do?
            action_defs.push(action_def.clone());
        }
        
        Fighter {
            //css render
            name:       "Base Fighter".to_string(),
            name_short: "BF".to_string(),
            css_action: Action::Idle,
            css_frame:  0,
            css_point1: (0.0, 0.0),
            css_point2: (0.0, 0.0),

            //in game attributes
            air_jumps:             1,
            weight:                80,
            gravity:               -0.13,
            terminal_vel:          -2.0,
            jump_y_init_vel:       3.1,
            jump_y_init_vel_short: 1.9,
            jump_x_init_vel:       0.95,
            shield_size:           15.0,
            walk_init_vel:         0.2,
            walk_acc:              0.1,
            walk_max_vel:          0.85,
            slow_walk_max_vel:     0.85,
            dash_init_vel:         2.0,
            dash_run_acc_a:        1.5,
            dash_run_acc_b:        0.01,
            dash_run_term_vel:     2.3,
            friction:              0.08,
            action_defs:           action_defs,
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct Fighter {
    //css render
    pub name:       String,
    pub name_short: String,
    pub css_action: Action,
    pub css_frame:  u64,
    pub css_point1: (f32, f32),
    pub css_point2: (f32, f32),

    //in game attributes
    pub air_jumps:             u64,
    pub weight:                u64,
    pub gravity:               f32,
    pub terminal_vel:          f32,
    pub jump_y_init_vel:       f32,
    pub jump_y_init_vel_short: f32,
    pub jump_x_init_vel:       f32,
    pub shield_size:           f32,
    pub walk_init_vel:         f32,
    pub walk_acc:              f32,
    pub walk_max_vel:          f32,
    pub slow_walk_max_vel:     f32,
    pub dash_init_vel:         f32,
    pub dash_run_acc_a:        f32,
    pub dash_run_acc_b:        f32,
    pub dash_run_term_vel:     f32,
    pub friction:              f32,
    pub action_defs:           Vec<ActionDef>,
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct ActionDef {
    pub frames: Vec<ActionFrame>,
    pub iasa:   u64,
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct ActionFrame {
    pub colboxes:     Vec<CollisionBox>,
    pub colbox_links: Vec<CollisionBoxLink>,
    pub effects:      Vec<FrameEffect>,
    pub ecb_w:        f32,
    pub ecb_h:        f32,
    pub ecb_y:        f32,
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct CollisionBoxLink {
    pub one:       usize,
    pub two:       usize,
    pub link_type: LinkType,
}

impl CollisionBoxLink {
    pub fn equals (&self, one: usize, two: usize) -> bool {
        self.one == one && self.two == two ||
        self.one == two && self.two == one
    }

    pub fn contains (&self, check: usize) -> bool {
        self.one == check || self.two == check
    }

    pub fn dec_greater_than(&self, check: usize) -> CollisionBoxLink {
        let mut one = self.one;
        let mut two = self.two;

        if self.one > check {
            one -= 1;
        }
        if self.two > check {
            two -= 1;
        }

        CollisionBoxLink {
            one: one,
            two: two,
            link_type: self.link_type.clone(),
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub enum LinkType {
    Meld,
    Simple,
}

enum_from_primitive! {
#[derive(Clone, RustcEncodable, RustcDecodable, PartialEq, Debug)]
pub enum Action {
    // Idle
    Spawn,
    SpawnIdle,
    Idle,
    Crouch,

    // Movement
    Fall,
    AerialFall,
    Land,
    JumpSquat,
    JumpF,
    JumpB,
    JumpAerialF,
    JumpAerialB,
    Turn,
    Dash,
    Run,
    RunEnd,

    // Defense
    ShieldOn,
    Shield,
    ShieldOff,
    RollF,
    RollB,
    AerialDodge,
    SpecialFall,
    SpecialLand,
    TechF,
    TechS,
    TechB,

    // Attacks
    Jab,
    Jab2,
    Jab3,
    Utilt,
    Dtilt,
    Ftilt,
    DashAttack,
    Usmash,
    Dsmash,
    Fsmash,
    Grab,
    DashGrab,

    // Aerials
    Uair,
    Dair,
    Fair,
    Nair,
    UairLand,
    DairLand,
    FairLand,
    NairLand,

    // Taunts
    TauntUp,
    TauntDown,
    TauntLeft,
    TauntRight,

    // crouch
    CrouchStart,
    CrouchEnd,
}
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub enum FrameEffect {
    Velocity     {x: f32, y: f32},
    Acceleration {x: f32, y: f32},
}


#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct CollisionBox {
    pub point:  (f32, f32),
    pub radius: f32,
    pub role:   CollisionBoxRole,
}

impl CollisionBox {
    pub fn new (point: (f32, f32)) -> CollisionBox {
        CollisionBox {
            point:  point,
            radius: 2.0,
            role:   CollisionBoxRole::Intangible,
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub enum CollisionBoxRole {
    Hurt (HurtBox), // a target
    Hit  (HitBox),  // a launching attack
    Grab,
    Intangible,     // cannot be interacted with
    Invincible,     // cannot receive damage or knockback.
    Reflect,        // reflects projectiles
    Absorb,         // absorb projectiles
}

#[derive(Clone, RustcEncodable, RustcDecodable, Default)]
pub struct HurtBox {
    pub knockback_mod: u64,
    pub damage_mod:    u64,
}

#[derive(Clone, RustcEncodable, RustcDecodable, Default)]
pub struct HitBox {
    pub shield_damage: u64,
    pub damage:        u64,
    pub bkb:           u64, // base knockback
    pub kbg:           u64, // knockback growth
    pub angle:         u64,
    pub priority:      u64,
    pub effect:        HitboxEffect,
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub enum HitboxEffect {
    Fire,
    Electric,
    Sleep,
    Reverse,
    Stun,
    Freeze,
    None,
}

impl Default for HitboxEffect {
    fn default() -> HitboxEffect { HitboxEffect::None }
}
