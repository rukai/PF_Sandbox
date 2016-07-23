use game::Point;

impl Fighter {

    //TODO: Eventually this should produce a blank Fighter
    //      An immutable demonstration package will be provided instead
    pub fn base() -> Fighter {
        let point1 = (3.0, 5.0);
        let point2 = (5.0, 6.0);

        let hitbox1 = Hitbox {
            point:  point1,
            form:   HitboxType::Hurt,
            radius: 1.4,
            damage: 0,
            bkb:    0,
            kbg:    0,
            angle:  0,
            clang:  0,
        };

        let hitbox2 = Hitbox {
            point: point2,
            form:   HitboxType::Hit,
            radius: 1.0,
            damage: 13,
            bkb:    50,
            kbg:    70,
            angle:  40,
            clang:  3,
        };

        let action_frame1 = ActionFrame {
            hitboxes: vec!(hitbox1, hitbox2),
            effects:  Vec::new(),
            ecb_w:    3.5,
            ecb_h:    12.0,
            ecb_y:    6.0,
        };

        let action_frame2 = action_frame1.clone();

        let action_def = ActionDef {
            frames: vec!(action_frame1, action_frame2),
            iasa:   0,
        };
        let mut action_defs: Vec<ActionDef> = Vec::new();
        for _ in 0..((Action::TauntRight as usize)+1) { // TODO: Super gross but what is a man to do?
            action_defs.push(action_def.clone());
        }
        
        Fighter {
            //css render
            name:       "Base Fighter".to_string(),
            name_short: "BF".to_string(),
            css_action: Action::Idle,
            css_frame:  0,
            css_point1: Point {x: 0.0, y: 0.0},
            css_point2: Point {x: 0.0, y: 0.0},

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
    pub css_point1: Point,
    pub css_point2: Point,

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
    pub hitboxes: Vec<Hitbox>,
    pub effects:  Vec<FrameEffect>,
    pub ecb_w:    f32,
    pub ecb_h:    f32,
    pub ecb_y:    f32,
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct Hitbox {
    pub point:  (f32, f32),
    pub form:   HitboxType,
    pub radius: f32,
    pub damage: u64,
    pub bkb:    u64,
    pub kbg:    u64,
    pub angle:  u64,
    pub clang:  u64,
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
}
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub enum FrameEffect {
    Velocity     {x: i64, y: i64},
    Acceleration {x: i64, y: i64},
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub enum HitboxType {
    Hurt,
    Intantigible,
    Invincible,
    Hit,
    Grab,
    Sleep,
    Freeze,
    Fire,
    Electric,
}
