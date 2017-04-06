use treeflection::{Node, NodeRunner, NodeToken, ContextVec};

impl Fighter {
    pub fn base() -> Fighter { // TODO: Change to default
        let action_def = ActionDef {
            frames: ContextVec::from_vec(vec!(ActionFrame::default())),
            iasa:   0,
        };
        let mut actions: ContextVec<ActionDef> = ContextVec::new();
        for _ in 0..((Action::Eliminated as usize)+1) { // TODO: Super gross but what is a man to do?
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
            air_jumps:               1,
            weight:                  1.0, // weight = old value / 100
            gravity:                 -0.1,
            terminal_vel:            -2.0,
            fastfall_terminal_vel:   -3.0,
            jump_y_init_vel:         3.0,
            jump_y_init_vel_short:   2.0,
            jump_x_init_vel:         1.0,
            air_mobility_a:          0.04,
            air_mobility_b:          0.02,
            air_x_term_vel:          1.0,
            air_friction:            0.05,
            shield_size:             15.0,
            walk_init_vel:           0.2,
            walk_acc:                0.1,
            walk_max_vel:            1.0,
            slow_walk_max_vel:       1.0,
            dash_init_vel:           2.0,
            dash_run_acc_a:          0.01,
            dash_run_acc_b:          0.2,
            dash_run_term_vel:       2.0,
            friction:                0.1,
            aerialdodge_mult:        3.0,
            aerialdodge_drift_frame: 20,
            actions:                 actions,
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
    pub air_jumps:               u64,
    pub weight:                  f32,
    pub gravity:                 f32,
    pub terminal_vel:            f32,
    pub fastfall_terminal_vel:   f32,
    pub jump_y_init_vel:         f32,
    pub jump_y_init_vel_short:   f32,
    pub jump_x_init_vel:         f32,
    pub air_mobility_a:          f32,
    pub air_mobility_b:          f32,
    pub air_x_term_vel:          f32,
    pub air_friction:            f32,
    pub shield_size:             f32,
    pub walk_init_vel:           f32,
    pub walk_acc:                f32,
    pub walk_max_vel:            f32,
    pub slow_walk_max_vel:       f32,
    pub dash_init_vel:           f32,
    pub dash_run_acc_a:          f32,
    pub dash_run_acc_b:          f32,
    pub dash_run_term_vel:       f32,
    pub friction:                f32,
    pub aerialdodge_mult:        f32,
    pub aerialdodge_drift_frame: u64,
    pub actions:               ContextVec<ActionDef>,
}

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct ActionDef {
    pub frames: ContextVec<ActionFrame>,
    pub iasa:   u64,
}

#[derive(Clone, Serialize, Deserialize, Node)]
pub struct ActionFrame {
    pub ecb:          ECB,
    pub colboxes:     ContextVec<CollisionBox>,
    pub colbox_links: Vec<CollisionBoxLink>,
    pub render_order: Vec<RenderOrder>,
    pub effects:      Vec<FrameEffect>,
    pub item_hold_x:  f32,
    pub item_hold_y:  f32,
    pub grab_hold_x:  f32,
    pub grab_hold_y:  f32,
    pub force_hitlist_reset: bool,
}

impl Default for ActionFrame {
    fn default() -> ActionFrame {
        ActionFrame {
            colboxes:     ContextVec::new(),
            colbox_links: vec!(),
            render_order: vec!(),
            effects:      vec!(),
            ecb:          ECB::default(),
            item_hold_x:  4.0,
            item_hold_y:  11.0,
            grab_hold_x:  4.0,
            grab_hold_y:  11.0,
            force_hitlist_reset: false,
        }
    }
}

impl ActionFrame {
    pub fn get_hitboxes(&self) -> Vec<&CollisionBox> {
        let mut result = self.get_colboxes();
        result.retain(|x| matches!(x.role, CollisionBoxRole::Hit(_)));
        result
    }

    pub fn get_hurtboxes(&self) -> Vec<&CollisionBox> {
        let mut result = self.get_colboxes();
        result.retain(|x| matches!(x.role, CollisionBoxRole::Hurt(_)));
        result
    }

    pub fn get_colboxes(&self) -> Vec<&CollisionBox> {
        let mut result: Vec<&CollisionBox> = vec!();
        for (i, colbox) in self.colboxes.iter().enumerate() {
            if self.is_unordered(&RenderOrder::Colbox(i)) {
                result.push(colbox);
            }
        }

        for order in &self.render_order {
            if let &RenderOrder::Colbox (index) = order {
                result.push(&self.colboxes[index]);
            }
        }

        result
    }

    pub fn get_links(&self) -> Vec<&CollisionBoxLink> {
        let mut result: Vec<&CollisionBoxLink> = vec!();
        for (i, link) in self.colbox_links.iter().enumerate() {
            if self.is_unordered(&RenderOrder::Link(i)) {
                result.push(link);
            }
        }

        for order in &self.render_order {
            if let &RenderOrder::Link (index) = order {
                result.push(&self.colbox_links[index]);
            }
        }

        result
    }

    /// Returns all collisionboxes and linked collisionboxes
    /// collisionboxes referenced by a link are not invluded individually
    pub fn get_colboxes_and_links(&self) -> Vec<ColboxOrLink> {
        let mut result: Vec<ColboxOrLink> = vec!();
        for (i, colbox) in self.colboxes.iter().enumerate() {
            if self.is_unordered(&RenderOrder::Colbox(i)) && self.is_unlinked(i) {
                result.push(ColboxOrLink::Colbox(colbox));
            }
        }
        for (i, link) in self.colbox_links.iter().enumerate() {
            if self.is_unordered(&RenderOrder::Link(i)) {
                result.push(ColboxOrLink::Link(link));
            }
        }

        for order in &self.render_order {
            match order {
                &RenderOrder::Colbox (index) => {
                    result.push(ColboxOrLink::Colbox(&self.colboxes[index]));
                }
                &RenderOrder::Link (index) => {
                    result.push(ColboxOrLink::Link(&self.colbox_links[index]));
                }
            }
        }

        result
    }

    fn is_unordered(&self, check_order: &RenderOrder) -> bool {
        for order in &self.render_order {
            if check_order == order {
                return false;
            }
        }
        true
    }

    fn is_unlinked(&self, i: usize) -> bool {
        for link in &self.colbox_links {
            if link.one == i || link.two == i {
                return false;
            }
        }
        true
    }

    pub fn get_links_containing_colbox(&self, colbox_i: usize) -> Vec<usize> {
        let mut result: Vec<usize> = vec!();
        for (link_i, link) in self.colbox_links.iter().enumerate() {
            if link.one == colbox_i || link.two == colbox_i {
                result.push(link_i);
            }
        }
        result
    }
}

pub enum ColboxOrLink <'a> {
    Colbox (&'a CollisionBox),
    Link   (&'a CollisionBoxLink)
}

#[derive(PartialEq, Clone, Serialize, Deserialize, Node)]
pub enum RenderOrder {
    Colbox (usize),
    Link   (usize)
}

impl Default for RenderOrder {
    fn default() -> RenderOrder {
        RenderOrder::Colbox (0)
    }
}

impl RenderOrder {
    pub fn dec_greater_than(&self, check: usize) -> RenderOrder {
        match self {
            &RenderOrder::Colbox (i) => {
                if i > check {
                    RenderOrder::Colbox (i-1)
                }
                else {
                    RenderOrder::Colbox (i)
                }
            }
            &RenderOrder::Link (i) => {
                if i > check {
                    RenderOrder::Link (i-1)
                }
                else {
                    RenderOrder::Link (i)
                }
            }
        }
    }
}

// GUI Editor will need to ensure that values are kept sane, e.g. left is leftmost, top is topmost etc.
// CLI is fine as it is easier to keep track of which point is which
#[derive(Clone, Serialize, Deserialize, Node)]
pub struct ECB {
    pub top_x:   f32,
    pub top_y:   f32,
    pub left_x:  f32,
    pub left_y:  f32,
    pub right_x: f32,
    pub right_y: f32,
    pub bot_x:   f32,
    pub bot_y:   f32,
}

impl Default for ECB {
    fn default() -> ECB {
        ECB {
            top_x:   0.0,
            top_y:   16.0,
            left_x:  -4.0,
            left_y:  11.0,
            right_x: 4.0,
            right_y: 11.0,
            bot_x:   0.0,
            bot_y:   0.0,
        }
    }
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
    MeldFirst,
    MeldSecond,
    Simple,
}

impl Default for LinkType {
    fn default() -> LinkType {
        LinkType::MeldFirst
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
    TurnRun,
    TurnDash,
    Dash,
    Run,
    RunEnd,
    Walk,
    PassPlatform,
    Damage,
    DamageFly,
    DamageFall,

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
    Rebound, // State after clang

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
    Bair,
    Nair,
    UairLand,
    DairLand,
    FairLand,
    BairLand,
    NairLand,

    // Taunts
    TauntUp,
    TauntDown,
    TauntLeft,
    TauntRight,

    // Crouch
    CrouchStart,
    CrouchEnd,

    Eliminated,
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

#[derive(Clone, Serialize, Deserialize, Node)]
pub struct CollisionBox {
    pub point:  (f32, f32),
    pub radius: f32,
    pub role:   CollisionBoxRole,
}

impl CollisionBox {
    pub fn new(point: (f32, f32)) -> CollisionBox {
        CollisionBox {
            point:  point,
            radius: 3.0,
            role:   CollisionBoxRole::default()
        }
    }

    /// Warning: panics when not a hitbox
    pub fn hitbox_ref(&self) -> &HitBox {
        match &self.role {
            &CollisionBoxRole::Hit (ref hitbox) => hitbox,
            _ => panic!("Called hitbox_ref on a CollisionBox that is not a HitBox")
        }
    }
}

impl Default for CollisionBox {
    fn default() -> CollisionBox {
        CollisionBox {
            point:  (0.0, 0.0),
            radius: 3.0,
            role:   CollisionBoxRole::default()
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Node)]
pub enum CollisionBoxRole {
    Hurt (HurtBox),   // a target
    Hit  (HitBox),    // a launching attack
    Grab,             // a grabbing attack
    Intangible,       // cannot be interacted with rendered transparent with normal outline
    IntangibleItem,   // cannot be interacted with rendered as a grey surface with no outline
    Invincible,       // cannot receive damage or knockback.
    Reflect,          // reflects projectiles
    Absorb,           // absorb projectiles
}

impl Default for CollisionBoxRole {
    fn default() -> CollisionBoxRole {
        CollisionBoxRole::Hurt ( HurtBox::default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
pub struct HurtBox {
    pub bkb_add:     f32,
    pub kbg_add:     f32,
    pub damage_mult: f32,
}

impl Default for HurtBox {
    fn default() -> HurtBox {
        HurtBox {
            bkb_add:     0.0,
            kbg_add:     0.0,
            damage_mult: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
pub struct HitBox {
    pub shield_damage:  f32,
    pub damage:         f32,
    pub bkb:            f32, // base knockback
    pub kbg:            f32, // knockback growth = old value / 100
    pub angle:          f32,
    pub hitstun:        HitStun,
    pub enable_clang:   bool,
    pub enable_rebound: bool,
    pub effect:         HitboxEffect,
}

impl Default for HitBox {
    fn default() -> HitBox {
        HitBox {
            shield_damage:  10.0,
            damage:         10.0,
            bkb:            60.0,
            kbg:            1.0,
            angle:          0.0,
            enable_clang:   true,
            enable_rebound: true,
            hitstun:        HitStun::default(),
            effect:         HitboxEffect::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
pub enum HitStun {
    FramesTimesKnockback (f32),
    Frames (u64)
}

impl Default for HitStun {
    fn default() -> HitStun {
        HitStun::FramesTimesKnockback(0.5)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
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
