use treeflection::{Node, NodeRunner, NodeToken, ContextVec};

impl Fighter {
    pub fn base() -> Fighter { // TODO: Change to default
        let action_frame1 = ActionFrame {
            colboxes:     ContextVec::new(),
            colbox_links: vec!(),
            effects:      vec!(),
            ecb_w:        3.5,
            ecb_h:        12.0,
            ecb_y:        6.0,
        };

        let action_def = ActionDef {
            frames: ContextVec::from_vec(vec!(action_frame1)),
            iasa:   0,
        };
        let mut actions: ContextVec<ActionDef> = ContextVec::new();
        for _ in 0..((Action::TurnRun as usize)+1) { // TODO: Super gross but what is a man to do?
            actions.push(action_def.clone());
        }
        
        Fighter {
            //css render
            name:       "Base Fighter".to_string(),
            name_short: "BF".to_string(),
            css_action: Action::Idle,
            css_frame:  0,
            css_point1: (0.0, 0.0),
            css_point2: (0.0, 0.0),
            css_hide:   false,

            //in game attributes
            air_jumps:             1,
            weight:                100,
            gravity:               -0.1,
            terminal_vel:          -2.0,
            fastfall_terminal_vel: -3.0,
            jump_y_init_vel:       3.0,
            jump_y_init_vel_short: 2.0,
            jump_x_init_vel:       1.00,
            shield_size:           15.0,
            walk_init_vel:         0.2,
            walk_acc:              0.1,
            walk_max_vel:          1.00,
            slow_walk_max_vel:     1.00,
            dash_init_vel:         2.0,
            dash_run_acc_a:        1.0,
            dash_run_acc_b:        0.0,
            dash_run_term_vel:     2.0,
            friction:              0.10,
            actions:               actions,
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct Fighter {
    //css render
    pub name:       String,
    pub name_short: String,
    pub css_action: Action,
    pub css_frame:  u64,
    pub css_point1: (f32, f32),
    pub css_point2: (f32, f32),
    pub css_hide:   bool,

    //in game attributes
    pub air_jumps:             u64,
    pub weight:                u64,
    pub gravity:               f32,
    pub terminal_vel:          f32,
    pub fastfall_terminal_vel: f32,
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
    pub actions:               ContextVec<ActionDef>,
}

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct ActionDef {
    pub frames: ContextVec<ActionFrame>,
    pub iasa:   u64,
}

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct ActionFrame {
    pub colboxes:     ContextVec<CollisionBox>,
    pub colbox_links: Vec<CollisionBoxLink>,
    pub effects:      Vec<FrameEffect>,
    pub ecb_w:        f32,
    pub ecb_h:        f32,
    pub ecb_y:        f32,
    //pub item_hold_pos: (f32, f32),
    //pub grab_hold_pos: (f32, f32),
}

#[derive(Clone, Default, Serialize, Deserialize, Node)]
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

#[derive(Clone, Serialize, Deserialize, Node)]
pub enum LinkType {
    Meld,
    Simple,
}

impl Default for LinkType {
    fn default() -> LinkType {
        LinkType::Meld
    }
}

enum_from_primitive! {
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, Node)]
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

    // unsorted for compatibility with packages
    PassPlatform,
    TurnRun,
    TurnDash,
}
}

impl Default for Action {
    fn default() -> Action {
        Action::Spawn
    }
}

#[derive(Clone, Serialize, Deserialize, Node)]
pub enum FrameEffect {
    Velocity     {x: f32, y: f32},
    Acceleration {x: f32, y: f32},
}

impl Default for FrameEffect {
    fn default() -> FrameEffect {
        FrameEffect::Velocity { x: 0.0, y: 0.0 }
    }
}

#[derive(Clone, Default, Serialize, Deserialize, Node)]
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

#[derive(Clone, Serialize, Deserialize, Node)]
pub enum CollisionBoxRole {
    Hurt (HurtBox), // a target
    Hit  (HitBox),  // a launching attack
    Grab,           // a grabbing attack
    Intangible,     // cannot be interacted with
    Invincible,     // cannot receive damage or knockback.
    Reflect,        // reflects projectiles
    Absorb,         // absorb projectiles
}

impl Default for CollisionBoxRole {
    fn default() -> CollisionBoxRole {
        CollisionBoxRole::Hurt ( HurtBox::default())
    }
}

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct HurtBox {
    pub knockback_mod: u64,
    pub damage_mod:    u64,
}

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct HitBox {
    pub shield_damage: u64,
    pub damage:        u64,
    pub bkb:           u64, // base knockback
    pub kbg:           u64, // knockback growth
    pub angle:         u64,
    pub priority:      u64,
    pub effect:        HitboxEffect,
}

#[derive(Clone, Serialize, Deserialize, Node)]
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
    fn default() -> HitboxEffect {
        HitboxEffect::None
    }
}
