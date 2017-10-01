use ::collision::CollisionResult;
use ::fighter::*;
use ::input::{PlayerInput};
use ::math;
use ::package::Package;
use ::results::{RawPlayerResult, DeathRecord};
use ::rules::Goal;
use ::stage::{Stage, Platform, Area, SpawnPoint};

use std::f32;
use std::f32::consts::PI;
use std::collections::HashSet;
use treeflection::{Node, NodeRunner, NodeToken, KeyedContextVec};
use rand::StdRng;
use rand::Rng;
use enum_traits::{FromIndex, ToIndex};

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct Player {
    pub fighter:            String,
    pub action:             u64, // always change through self.set_action
    pub frame:              u64,
    pub stocks:             Option<u64>,
    pub damage:             f32,
    pub location:           Location,
    pub respawn:            SpawnPoint,
    pub x_vel:              f32,
    pub y_vel:              f32,
    pub kb_x_vel:           f32,
    pub kb_y_vel:           f32,
    pub kb_x_dec:           f32,
    pub kb_y_dec:           f32,
    pub face_right:         bool,
    pub frames_since_ledge: u64,
    pub ledge_idle_timer:   u64,
    pub fastfalled:         bool,
    pub air_jumps_left:     u64,
    pub jumpsquat_button:   bool,
    pub turn_dash_buffer:   bool,
    pub shield_hp:          f32,
    pub shield_analog:      f32,
    pub shield_offset_x:    f32,
    pub shield_offset_y:    f32,
    pub stun_timer:         u64,
    pub shield_stun_timer:  u64,
    pub parry_timer:        u64,
    pub ecb:                ECB,
    pub hitlist:            Vec<usize>,
    pub hitlag:             Hitlag,
    pub hitstun:            f32,
    pub hit_by:             Option<usize>,
    pub result:             RawPlayerResult,

    // Only use for debug display
    pub frames_since_hit:   u64,
    pub hit_angle_pre_di:   Option<f32>,
    pub hit_angle_post_di:  Option<f32>,
    pub stick:              Option<(f32, f32)>,
    pub c_stick:            Option<(f32, f32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
pub enum LedgeLogic {
    Hog,
    Share,
    Trump
}

// Describes the player location by offsets from other locations
#[derive(Debug, Clone, Serialize, Deserialize, Node)]
pub enum Location {
    Platform { platform_i: usize, x: f32 },
    GrabbedLedge { platform_i: usize, d_x: f32, d_y: f32, logic: LedgeLogic }, // player.face_right determines which edge on the platform
    GrabbedByPlayer (usize),
    Airbourne { x: f32, y: f32 },
}

impl Default for Location {
    fn default() -> Location {
        Location::Airbourne { x: 0.0, y: 0.0 }
    }
}

impl Default for LedgeLogic {
    fn default() -> Self {
        LedgeLogic::Hog
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
pub enum Hitlag {
    Some (u64), // TODO: rename Some
    Launch { counter: u64, kb_vel: f32, angle: f32, wobble_x: f32 },
    None
}

impl Default for Hitlag {
    fn default() -> Hitlag {
        Hitlag::None
    }
}

impl Hitlag {
    pub fn decrement(&mut self) -> bool {
        let end = match self {
            &mut Hitlag::Some (ref mut counter) |
            &mut Hitlag::Launch { ref mut counter, .. } => {
                *counter -= 1;
                *counter <= 1
            }
            &mut Hitlag::None => {
                false
            }
        };
        if end {
            *self = Hitlag::None
        }
        end
    }

    fn wobble(&mut self, rng: &mut StdRng) {
        if let &mut Hitlag::Launch { ref mut wobble_x, .. } = self {
            *wobble_x = (rng.next_f32() - 0.5) * 3.0;
        }
    }
}

impl Player {
    pub fn new(fighter: String, spawn: SpawnPoint, respawn: SpawnPoint, package: &Package) -> Player {
        Player {
            action:             Action::DummyFramePreStart.index(),
            frame:              0,
            stocks:             package.rules.stock_count,
            damage:             0.0,
            location:           Location::Airbourne { x: spawn.x, y: spawn.y },
            respawn:            respawn,
            x_vel:              0.0,
            y_vel:              0.0,
            kb_x_vel:           0.0,
            kb_y_vel:           0.0,
            kb_x_dec:           0.0,
            kb_y_dec:           0.0,
            face_right:         spawn.face_right,
            frames_since_ledge: 0,
            ledge_idle_timer:   0,
            fastfalled:         false,
            air_jumps_left:     0,
            jumpsquat_button:   false,
            turn_dash_buffer:   false,
            shield_hp:          package.fighters[fighter.as_ref()].shield.as_ref().map_or(60.0, |x| x.hp_max),
            shield_analog:      0.0,
            shield_offset_x:    0.0,
            shield_offset_y:    0.0,
            stun_timer:         0,
            shield_stun_timer:  0,
            parry_timer:        0,
            ecb:                ECB::default(),
            hitlist:            vec!(),
            hitlag:             Hitlag::None,
            hitstun:            0.0,
            hit_by:             None,
            result:             RawPlayerResult::default(),
            fighter:            fighter,

            // Only use for debug display
            frames_since_hit:   0,
            hit_angle_pre_di:   None,
            hit_angle_post_di:  None,
            stick:              None,
            c_stick:            None,
        }
    }

    pub fn bps_xy(&self, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform]) -> (f32, f32) {
        let bps_xy = match self.location {
            Location::Platform { platform_i, x } => {
                if let Some(platform) = platforms.get(platform_i) {
                    platform.plat_x_to_world_p(x)
                } else {
                    (0.0, 0.0)
                }
            }
            Location::GrabbedLedge { platform_i, d_x, d_y, .. } => {
                if let Some(platform) = platforms.get(platform_i) {
                    let (ledge_x, ledge_y) = if self.face_right {
                        platform.left_ledge()
                    } else {
                        platform.right_ledge()
                    };
                    (ledge_x + self.relative_f(d_x), ledge_y + d_y)
                } else {
                    (0.0, 0.0)
                }
            }
            Location::GrabbedByPlayer (player_i) => {
                if let Some(player) = players.get(player_i) {
                     player.grab_xy(players, fighters, platforms)
                } else {
                    (0.0, 0.0)
                }
            }
            Location::Airbourne { x, y } => {
                (x, y)
            }
        };

        match &self.hitlag {
            &Hitlag::Launch { wobble_x, .. } => {
                (bps_xy.0 + wobble_x, bps_xy.1)
            }
            _ => {
                bps_xy
            }
        }
    }

    pub fn grab_xy(&self, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform]) -> (f32, f32) {
        let (x, y) = self.bps_xy(players, fighters, platforms);
        let fighter_frame = &fighters[self.fighter.as_ref()].actions[self.action as usize].frames[self.frame as usize];
        (x + fighter_frame.grab_hold_x, y + fighter_frame.grab_hold_y)
    }

    pub fn is_platform(&self) -> bool {
        if let &Location::Platform { .. } = &self.location {
            true
        } else {
            false
        }
    }

    pub fn is_ledge(&self) -> bool {
        if let &Location::GrabbedLedge { .. } = &self.location {
            true
        } else {
            false
        }
    }

    pub fn is_grabbed(&self) -> bool {
        if let &Location::GrabbedByPlayer (_) = &self.location {
            true
        } else {
            false
        }
    }

    pub fn is_airbourne(&self) -> bool {
        if let &Location::Airbourne { .. } = &self.location {
            true
        } else {
            false
        }
    }

    pub fn is_hogging_ledge(&self, check_platform_i: usize, face_right: bool) -> bool {
        if let &Location::GrabbedLedge { platform_i, ref logic, .. } = &self.location {
            if let &LedgeLogic::Hog = logic {
                return self.face_right == face_right && check_platform_i == platform_i;
            }
        }
        false
    }

    pub fn is_shielding(&self) -> bool {
        match Action::from_index(self.action) {
            Some(Action::Shield) |
            Some(Action::ShieldOn) |
            Some(Action::ShieldOff) |
            Some(Action::PowerShield)
              => true,
            _ => false
        }
    }

    pub fn set_airbourne(&mut self, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform]) {
        let (x, y) = self.bps_xy(players, fighters, platforms);
        self.fastfalled = false;
        self.location = Location::Airbourne { x, y };
    }

    // always change self.action through this method
    fn set_action(&mut self, action: Action) {
        self.action = action as u64;
        self.hitlist.clear();
        self.frame = 0;
    }

    fn interruptible(&self, fighter: &Fighter) -> bool {
        self.frame >= fighter.actions[self.action as usize].iasa
    }

    pub fn step_collision(&mut self, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform], col_results: &[CollisionResult]) {
        for col_result in col_results {
            match col_result {
                &CollisionResult::HitAtk { player_def_i, ref hitbox } => {
                    self.hitlist.push(player_def_i);
                    self.hitlag = Hitlag::Some ((hitbox.damage / 3.0 + 3.0) as u64);
                }
                &CollisionResult::HitDef { ref hitbox, ref hurtbox, player_atk_i } => {
                    let fighter = &fighters[self.fighter.as_ref()];
                    let damage_done = hitbox.damage * hurtbox.damage_mult; // TODO: apply staling
                    self.damage += damage_done;

                    let damage_launch = 0.05 * (hitbox.damage * (damage_done + self.damage.floor())) + (damage_done + self.damage) * 0.1;
                    let weight = 2.0 - (2.0 * fighter.weight) / (1.0 + fighter.weight);
                    let kbg = hitbox.kbg + hurtbox.kbg_add;
                    let bkb = hitbox.bkb + hurtbox.bkb_add;

                    let mut kb_vel = (bkb + kbg * (damage_launch * weight * 1.4 + 18.0)).min(2500.0);

                    if let Some(action) = Action::from_index(self.action) {
                        match action {
                            Action::Crouch => {
                                kb_vel *= 0.67;
                            }
                            _ => { }
                        }
                    }

                    if !self.is_grabbed() || kb_vel > 50.0 {
                        self.hitstun = match hitbox.hitstun {
                            HitStun::FramesTimesKnockback (frames) => { frames * kb_vel }
                            HitStun::Frames               (frames) => { frames as f32 }
                        };

                        self.set_airbourne(players, fighters, platforms);

                        if kb_vel > 80.0 {
                            self.set_action(Action::DamageFly);
                        }
                        else {
                            self.set_action(Action::Damage);
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
                    let angle = angle_deg * PI / 180.0 + if angle_deg < 0.0 { PI * 2.0 } else { 0.0 };

                    // debug data
                    self.hit_angle_pre_di = Some(angle);
                    self.hit_angle_post_di = None;
                    self.frames_since_hit = 0;

                    self.hitlag = Hitlag::Launch { counter: (hitbox.damage / 3.0 + 3.0) as u64, kb_vel, angle, wobble_x: 0.0 };
                    self.hit_by = Some(player_atk_i);
                    self.face_right = self.bps_xy(players, fighters, platforms).0 < players[player_atk_i].bps_xy(players, fighters, platforms).0;
                }
                &CollisionResult::HitShieldAtk { ref hitbox, ref power_shield, player_def_i} => {
                    self.hitlist.push(player_def_i);
                    if let &Some(ref power_shield) = power_shield {
                        if let (Some(Action::PowerShield), &Some(ref stun)) = (Action::from_index(self.action), &power_shield.enemy_stun) {
                            if stun.window > players[player_def_i].frame {
                                self.stun_timer = stun.duration;
                            }
                        }
                    }

                    let x_diff = self.bps_xy(players, fighters, platforms).0 - players[player_def_i].bps_xy(players, fighters, platforms).0;
                    let vel = hitbox.damage.floor() * (players[player_def_i].shield_analog - 0.3) * 0.1 + 0.02;
                    if self.is_platform() {
                        self.x_vel += vel * x_diff.signum();
                    }
                    self.hitlag = Hitlag::Some ((hitbox.damage / 3.0 + 3.0) as u64);
                }
                &CollisionResult::HitShieldDef { ref hitbox, ref power_shield, player_atk_i } => {
                    if let &Some(ref power_shield) = power_shield {
                        if let (Some(Action::PowerShield), &Some(ref parry)) = (Action::from_index(self.action), &power_shield.parry) {
                            if parry.window > self.frame {
                                self.parry_timer = parry.duration;
                            }
                        }
                    }

                    if self.parry_timer == 0 {
                        self.shield_hp -= hitbox.shield_damage;
                        if self.shield_hp <= 0.0 {
                            continue;
                        }
                    }

                    let analog_mult = 1.0 - (self.shield_analog - 0.3) / 0.7;
                    let vel_mult = if self.parry_timer > 0 { 1.0 } else { 0.6 };
                    let x_diff = self.bps_xy(players, fighters, platforms).0 - players[player_atk_i].bps_xy(players, fighters, platforms).0;
                    let vel = (hitbox.damage.floor() * (0.195 * analog_mult + 0.09) + 0.4) * vel_mult;
                    self.x_vel = vel.min(2.0) * x_diff.signum();
                    self.shield_stun_timer = (hitbox.damage.floor() * (analog_mult + 0.3) * 0.975 + 2.0) as u64;
                    self.hitlag = Hitlag::Some ((hitbox.damage / 3.0 + 3.0) as u64);
                }
                _ => { }
            }
        }
    }

    /*
     *  Begin action section
     */

    pub fn action_hitlag_step(&mut self, input: &PlayerInput, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform], rng: &mut StdRng) {
        match self.hitlag.clone() {
            Hitlag::Some (_) => {
                self.hitlag.decrement();
            }
            Hitlag::Launch { kb_vel, angle, .. } => {
                self.hitlag.wobble(rng);

                if self.hitlag.decrement() {
                    self.hitlag_def_end(input, players, fighters, platforms, kb_vel, angle);
                }
            }
            Hitlag::None => {
                self.action_step(input, players, fighters, platforms);
            }
        }
    }

    /// 0 < angle < 2pi
    fn di(input: &PlayerInput, angle: f32) -> f32 {
        let range = 18.0 * PI / 180.0;
        let x = input[0].stick_x;
        let y = input[0].stick_y;

        let di_angle = y.atan2(x);                                                      // -pi  <= di_angle     <= pi
        let pos_di_angle = di_angle + if di_angle < 0.0 { PI * 2.0 } else { 0.0 };      // 0    <= pos_di_angle <= 2pi
        let angle_diff = angle - pos_di_angle;                                          // -2pi <= angle_diff   <= 2pi

        let offset_distance = (angle_diff).sin() * (x * x + y * y).sqrt();                 // -1     <= offset_distance <= 1
        let offset = offset_distance.signum() * offset_distance * offset_distance * range; // -range <= offset          <= range
        angle - offset
    }

    fn hitlag_def_end(&mut self, input: &PlayerInput, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform], kb_vel: f32, angle: f32) {
        let angle = if (kb_vel >= 80.0 || self.is_airbourne() || (angle != 0.0 && angle != PI)) // can di
            && !(input[0].stick_x == 0.0 && input[0].stick_y == 0.0) // not deadzone
        {
            Player::di(input, angle)
        } else {
            angle
        };
        self.hit_angle_post_di = Some(angle);

        // launch velocity
        let (sin, cos) = angle.sin_cos();
        self.x_vel = 0.0;
        self.y_vel = 0.0;
        self.kb_x_vel = cos * kb_vel * 0.03;
        self.kb_y_vel = sin * kb_vel * 0.03;
        self.kb_x_dec = cos * 0.051;
        self.kb_y_dec = sin * 0.051;

        if self.kb_y_vel == 0.0 {
            if kb_vel >= 80.0 {
                self.set_airbourne(players, fighters, platforms);
                if let &mut Location::Airbourne { ref mut y, .. } = &mut self.location {
                    *y += 0.0001;
                }
            }
        }
        else if self.kb_y_vel > 0.0 {
            self.set_airbourne(players, fighters, platforms);
        }
    }

    fn action_step(&mut self, input: &PlayerInput, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform]) {
        let fighter = &fighters[self.fighter.as_ref()];
        let action_frames = fighter.actions[self.action as usize].frames.len() as u64;

        self.frame += 1; // Action::DummyFramePreStart is used so that this doesnt skip a real frame when the game starts
        if self.frame >= action_frames {
            self.action_expired(input, players, fighters, platforms);
        }

        let fighter_frame = &fighter.actions[self.action as usize].frames[self.frame as usize];
        let action = Action::from_index(self.action);

        // update ecb
        let prev_bot_y = self.ecb.bot_y;
        self.ecb = fighter_frame.ecb.clone();
        match action {
            Some(Action::JumpF) | Some(Action::JumpB) | Some(Action::JumpAerialF) | Some(Action::JumpAerialB) if self.frame < 10
                => { self.ecb.bot_y = prev_bot_y }
            _   => { }
        }

        if fighter_frame.force_hitlist_reset {
            self.hitlist.clear();
        }

        if let Some(action) = action {
            match action {
                Action::SpawnIdle  | Action::Fall |
                Action::AerialFall | Action::JumpAerialF |
                Action::JumpF      | Action::JumpB |
                Action::Fair       | Action::Bair |
                Action::Dair       | Action::Uair |
                Action::Nair       | Action::JumpAerialB |
                Action::DamageFall
                => { self.aerial_action(input, fighter) }

                Action::Jab       | Action::Jab2 |
                Action::Jab3      | Action::Utilt |
                Action::Ftilt     | Action::DashAttack |
                Action::Dsmash    | Action::Fsmash |
                Action::Usmash    | Action::Idle |
                Action::Grab      | Action::DashGrab |
                Action::FairLand  | Action::BairLand |
                Action::UairLand  | Action::DairLand |
                Action::Land      | Action::SpecialLand |
                Action::CrouchEnd
                => { self.ground_idle_action(input, fighter) }

                Action::Teeter |
                Action::TeeterIdle       => self.teeter_action(input, fighter),
                Action::DamageFly        => self.damagefly_action(fighter),
                Action::Damage           => self.damage_action(fighter),
                Action::AerialDodge      => self.aerialdodge_action(input, fighter),
                Action::SpecialFall      => self.specialfall_action(input, fighter),
                Action::Dtilt            => self.dtilt_action(input, fighter),
                Action::CrouchStart      => self.crouch_start_action(input, players, fighters, platforms),
                Action::Crouch           => self.crouch_action(input, fighter),
                Action::Walk             => self.walk_action(input, fighter),
                Action::Dash             => self.dash_action(input, fighter),
                Action::Run              => self.run_action(input, fighter),
                Action::Turn             => self.turn_action(input, fighter),
                Action::LedgeIdle        => self.ledge_idle_action(input, players, fighters, platforms),
                Action::ShieldOn         => self.shield_on_action(input, players, fighters, platforms),
                Action::PowerShield      => self.power_shield_action(input, players, fighters, platforms),
                Action::Shield           => self.shield_action(input, players, fighters, platforms),
                Action::ShieldOff        => self.shield_off_action(input, players, fighters, platforms),
                Action::ShieldBreakFall  => self.shield_break_fall_action(fighter),
                Action::ShieldBreakGetup => self.shield_break_getup_action(),
                Action::Stun             => self.stun_action(fighter),
                _ => { },
            }
        }

        // Timers

        if !self.is_shielding() {
            if let Some(ref shield) = fighter.shield {
                self.shield_hp = shield.hp_max.min(self.shield_hp + shield.hp_regen);
            }
        }

        if self.parry_timer > 0 {
            self.parry_timer -= 1;
        }

        if self.shield_stun_timer > 0 {
            self.shield_stun_timer -= 1;
        }

        self.frames_since_hit += 1;
        if self.frames_since_hit > 60 {
            self.hit_angle_pre_di = None;
            self.hit_angle_post_di = None;
        }

        if input[0].stick_x == 0.0 && input[0].stick_y == 0.0 {
            self.stick = None;
        }
        else {
            self.stick = Some((input[0].stick_x, input[0].stick_y));
        }

        if input[0].c_stick_x == 0.0 && input[0].c_stick_y == 0.0 {
            self.c_stick = None;
        }
        else {
            self.c_stick = Some((input[0].c_stick_x, input[0].c_stick_y));
        }
    }

    fn ledge_idle_action(&mut self, input: &PlayerInput, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform]) {
        if
            (input[0].  stick_y < -0.2 && input[1].  stick_y >= -0.2) ||
            (input[0].c_stick_y < -0.2 && input[1].c_stick_y >= -0.2) ||
            (self.relative_f(input[0].  stick_x) < -0.2 && self.relative_f(input[1].  stick_x) >= -0.2) ||
            (self.relative_f(input[0].c_stick_x) < -0.2 && self.relative_f(input[1].c_stick_x) >= -0.2)
        {
            self.set_airbourne(players, fighters, platforms);
            self.set_action(Action::Fall);
            self.frames_since_ledge = 0;
        }
        else if input.x.press || input.y.press || (input[0].stick_y > 0.65 && input[1].stick_y <= 0.65) {
            if self.damage < 100.0 {
                self.set_action(Action::LedgeJump);
            }
            else {
                self.set_action(Action::LedgeJumpSlow);
            }
        }
        else if
            (self.relative_f(input[0].stick_x) > 0.2 && self.relative_f(input[1].stick_x) <= 0.2) ||
            (input[0].stick_y > 0.2 && input[1].stick_y <= 0.2)
        {
            if self.damage < 100.0 {
                self.set_action(Action::LedgeGetup);
            }
            else {
                self.set_action(Action::LedgeGetupSlow);
            }
        }
        else if input.a.press || input.b.press || (input[0].c_stick_y > 0.65 && input[1].c_stick_x <= 0.65) {
            if self.damage < 100.0 {
                self.set_action(Action::LedgeAttack);
            }
            else {
                self.set_action(Action::LedgeAttackSlow);
            }
        }
        else if
            input.l.press || input.r.press ||
            (input[0].l_trigger > 0.3 && input[1].l_trigger <= 0.3) || (input[0].r_trigger > 0.3 && input[1].r_trigger <= 0.3) ||
            (self.relative_f(input[0].c_stick_x) > 0.8 && self.relative_f(input[1].c_stick_x) <= 0.8)
        {
            if self.damage < 100.0 {
                self.set_action(Action::LedgeRoll);
            }
            else {
                self.set_action(Action::LedgeRollSlow);
            }
        }
        else if self.ledge_idle_timer > 600 {
            self.set_airbourne(players, fighters, platforms);
            self.set_action(Action::DamageFall);
            self.frames_since_ledge = 0;
        }
        self.ledge_idle_timer += 1;
    }

    fn damage_action(&mut self, fighter: &Fighter) {
        self.hitstun -= 1.0;
        if self.hitstun <= 0.0 {
            if self.is_airbourne() {
                self.set_action(Action::Fall);
            }
            else {
                self.set_action(Action::Idle);
            }
        }
        else {
            if self.is_airbourne() {
                self.fall_action(fighter);
            }
            else {
                self.apply_friction(fighter);
            }
        }
    }

    fn damagefly_action(&mut self, fighter: &Fighter) {
        self.hitstun -= 1.0;
        self.fall_action(fighter);
        if self.hitstun <= 0.0 {
            self.set_action(Action::DamageFall);
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
                self.y_vel = fighter.air_jump_y_vel;
                self.x_vel = fighter.air_jump_x_vel * input[0].stick_x;
                self.fastfalled = false;

                if self.relative_f(input.stick_x.value) < -0.3 {
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
        self.fastfall_action(input, fighter);
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
        else if self.check_shield(input, fighter) { }
        else if self.check_special(input) { } // TODO: No neutral special
        else if self.check_smash(input) { }
        else if self.check_attacks(input) { }
        else if self.check_taunt(input) { }
        else {
            self.apply_friction(fighter);
        }
    }

    fn crouch_start_action(&mut self, input: &PlayerInput, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform]) {
        let fighter = &fighters[self.fighter.as_ref()];
        if self.interruptible(fighter) {
            if self.check_pass_platform(input, players, fighters, platforms) { }
            else if self.check_shield(input, fighter) { }
            else if self.check_special(input) { } // TODO: no neutral/side special
            else if self.check_smash(input) { }
            else if self.check_attacks(input) { }
            else if self.check_taunt(input) { }
            else if self.check_jump(input) { }
        }
        self.apply_friction(fighter);
    }

    fn crouch_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        if self.interruptible(fighter) {
            if input.stick_y.value > -0.61 { self.set_action(Action::CrouchEnd); }
            if self.check_jump(input) { }
            else if self.check_shield(input, fighter) { }
            else if self.check_special(input) { } // TODO: no neutral/side special
            else if self.check_smash(input) { }
            else if self.check_attacks(input) { }
            else if self.check_taunt(input) { }
            else if self.check_dash(input, fighter) { }
            else if self.check_turn(input) { }
        }
        self.apply_friction(fighter);
    }

    fn dtilt_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        self.apply_friction(fighter);
        if self.interruptible(fighter) {
            if self.check_jump(input) { }
            else if self.check_shield(input, fighter) { }
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
            else if self.check_shield(input, fighter) { }
            else if self.check_special(input) { }
            else if self.check_smash(input) { }
            else if self.check_attacks(input) { }
            else if self.check_taunt(input) { }
            else if self.check_crouch(input) { }
            else if self.check_dash(input, fighter) { }
            else if self.check_turn(input) { }
            else if self.check_walk(input, fighter) { }
        }
    }

    fn teeter_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        self.apply_friction(fighter);
        if self.interruptible(fighter) {
            if self.check_jump(input) { }
            else if self.check_shield(input, fighter) { }
            else if self.check_special(input) { }
            else if self.check_smash(input) { }
            else if self.check_attacks(input) { }
            else if self.check_taunt(input) { }
            else if self.check_crouch(input) { }
            else if self.check_dash(input, fighter) { }
            else if self.check_turn(input) { }
            else if self.check_walk_teeter(input, fighter) { }
        }
    }

    fn walk_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
        if input[0].stick_x == 0.0 {
            self.set_action(Action::Idle);
        }
        else if self.check_jump(input) { }
        else if self.check_shield(input, fighter) { }
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
                    self.x_vel = vel_max;
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
        if self.check_shield(input, fighter) {
            self.x_vel *= 0.25;
        }
        else if self.relative_f(input.stick_x.value) < -0.35 { // TODO: refine
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
        if self.check_jump(input) { }
        else if self.check_shield(input, fighter) { }
        else if self.relative_f(input.stick_x.value) <= -0.3 {
            self.set_action(Action::TurnRun);
        }
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
            self.fastfall_action(input, fighter);
        }
    }

    fn shield_on_action(&mut self, input: &PlayerInput, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform]) {
        let fighter = &fighters[self.fighter.as_ref()];
        let stick_lock = fighter.shield.as_ref().map_or(false, |x| x.stick_lock) && input[0].b;
        let stun_lock = self.shield_stun_timer > 0;
        let lock = stun_lock && stick_lock;
        let power_shield_len = fighter.actions[Action::PowerShield as usize].frames.len();

        if !lock && self.check_jump(input) { }
        else if !lock && self.check_pass_platform(input, players, fighters, platforms) { }
        else if fighter.power_shield.is_some() && self.frame == 1 && (input.l.press || input.r.press) {
            // allow the first frame to transition to power shield so that powershield input is more consistent
            self.action = Action::PowerShield as u64;
            self.frame = if power_shield_len >= 2 { 1 } else { 0 }; // change self.frame so that a powershield isnt laggier than a normal shield
        }
        self.apply_friction(fighter);
        self.shield_shared_action(input, players, fighters, platforms);
    }

    fn shield_action(&mut self, input: &PlayerInput, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform]) {
        let fighter = &fighters[self.fighter.as_ref()];
        let stick_lock = fighter.shield.as_ref().map_or(false, |x| x.stick_lock) && input[0].b;
        let stun_lock = self.shield_stun_timer > 0;
        let lock = stun_lock && stick_lock;

        if !lock && self.check_jump(input) { }
        else if !lock && self.check_pass_platform(input, players, fighters, platforms) { }
        else if !stun_lock && input[0].l_trigger < 0.165 && input[0].r_trigger < 0.165 && !input[0].l && !input[0].r {
            if self.parry_timer > 0 {
                self.set_action(Action::Idle);
            } else {
                self.set_action(Action::ShieldOff);
            }
        }
        self.apply_friction(fighter);
        self.shield_shared_action(input, players, fighters, platforms);
    }

    fn shield_off_action(&mut self, input: &PlayerInput, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform]) {
        let fighter = &fighters[self.fighter.as_ref()];
        let stick_lock = fighter.shield.as_ref().map_or(false, |x| x.stick_lock) && input[0].b;
        let stun_lock = self.shield_stun_timer > 0;
        let lock = stun_lock && stick_lock;

        if !lock && self.check_jump(input) { }
        else if !lock && self.check_pass_platform(input, players, fighters, platforms) { }
        self.apply_friction(fighter);
        self.shield_shared_action(input, players, fighters, platforms);
    }

    fn power_shield_action(&mut self, input: &PlayerInput, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform]) {
        let fighter = &fighters[self.fighter.as_ref()];
        let stick_lock = fighter.shield.as_ref().map_or(false, |x| x.stick_lock) && input[0].b;
        let stun_lock = self.shield_stun_timer > 0;
        let lock = stun_lock && stick_lock;

        match (&fighter.shield, &fighter.power_shield) {
            (&Some(_), &Some(_)) => {
                if !lock && self.check_jump(input) { }
                else if !lock && self.check_pass_platform(input, players, fighters, platforms) { }
                self.shield_shared_action(input, players, fighters, platforms);
            }
            _ => {
                self.apply_friction(fighter);
            }
        }
    }

    fn shield_shared_action(&mut self, input: &PlayerInput, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform]) {
        let fighter = &fighters[self.fighter.as_ref()];
        self.apply_friction(fighter);
        if let Some(ref shield) = fighter.shield {
            // shield analog
            self.shield_analog = if input[0].l || input[0].r {
                1.0
            } else {
                input[0].l_trigger.max(input[0].r_trigger)
            };

            // shield offset
            let stick_x = input[0].stick_x;
            let stick_y = input[0].stick_y;
            let target_offset = (stick_x * stick_x + stick_y * stick_y).sqrt() * fighter.shield.as_ref().map_or(1.0, |x| x.stick_mult);
            let target_angle = stick_y.atan2(stick_x);
            let target_x = target_angle.cos() * target_offset;
            let target_y = target_angle.sin() * target_offset;
            self.shield_offset_x += (target_x - self.shield_offset_x) / 5.0 + 0.01;
            self.shield_offset_y += (target_y - self.shield_offset_y) / 5.0 + 0.01;

            // shield hp
            self.shield_hp -= shield.hp_cost * self.shield_analog - (1.0 - self.shield_analog) / 10.0;
            if self.shield_hp <= 0.0 {
                self.set_action(Action::ShieldBreakFall);
                self.shield_hp = 0.0;
                self.kb_y_vel = shield.break_vel;
                self.kb_y_dec = 0.051;
                self.kb_x_dec = 0.0;
                self.set_airbourne(players, fighters, platforms);
            }
        }
    }

    fn shield_break_fall_action(&mut self, fighter: &Fighter) {
        if !self.is_airbourne() {
            self.set_action(Action::ShieldBreakGetup);
        } else {
            self.fall_action(fighter);
        }
    }

    fn shield_break_getup_action(&mut self) {
        self.x_vel = 0.0;
    }

    fn stun_action(&mut self, fighter: &Fighter) {
        self.apply_friction(fighter);
        if self.shield_hp > 30.0 {
            self.shield_hp = 30.0;
        }

        self.stun_timer -= 1;

        // TODO: Mashout

        if self.stun_timer <= 0 {
            self.set_action(Action::Idle);
        }
    }

    pub fn shield_size(&self, shield: &Shield) -> f32 {
        let analog_size = (1.0 - self.shield_analog) * 0.6;
        let hp_size = (self.shield_hp / shield.hp_max) * shield.hp_scaling;
        let hp_size_unscaled = ((shield.hp_max - self.shield_hp) / shield.hp_max) * 2.0;

        shield.scaling * (analog_size + hp_size) + hp_size_unscaled
    }

    fn shield_pos(&self, shield: &Shield, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform]) -> (f32, f32) {
        let xy = self.bps_xy(players, fighters, platforms);
        (
            xy.0 + self.shield_offset_x + self.relative_f(shield.offset_x),
            xy.1 + self.shield_offset_y + shield.offset_y
        )
    }

    fn check_crouch(&mut self, input: &PlayerInput) -> bool {
        if input[0].stick_y < -0.69 {
            if let Some(action) = Action::from_index(self.action) {
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

    fn check_pass_platform(&mut self, input: &PlayerInput, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform]) -> bool {
        let fighter = &fighters[self.fighter.as_ref()];
        if let Location::Platform { platform_i, .. } = self.location {
            if let Some(platform) = platforms.get(platform_i) {
                let action_frames = fighter.actions[self.action as usize].frames.len() as u64 - 1;
                if platform.pass_through && self.frame == action_frames && (input[0].stick_y < -0.65 || input[1].stick_y < -0.65 || input[2].stick_y < -0.65) && input[6].stick_y > -0.3 {
                    self.set_action(Action::PassPlatform);
                    self.set_airbourne(players, fighters, platforms);
                    return true;
                }
            }
        }
        false
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

    fn check_walk_teeter(&mut self, input: &PlayerInput, fighter: &Fighter) -> bool {
        if input[0].stick_x.abs() > 0.6 {
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

    fn check_jump(&mut self, input: &PlayerInput) -> bool {
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

    fn check_shield(&mut self, input: &PlayerInput, fighter: &Fighter) -> bool {
        match (&fighter.shield, &fighter.power_shield) {
            (&Some(_), &Some(_)) => {
                if input.l.press || input.r.press {
                    self.set_action(Action::PowerShield);
                    true
                } else if input[0].l || input[0].r || input[0].l_trigger > 0.165 || input[0].r_trigger > 0.165 {
                    self.set_action(Action::ShieldOn);
                    true
                } else {
                    false
                }
            }
            (&None, &Some(_)) => {
                if input[0].l || input[0].r || input[0].l_trigger > 0.165 || input[0].r_trigger > 0.165 {
                    self.set_action(Action::PowerShield);
                    true
                } else {
                    false
                }
            }
            (&Some(_), &None) => {
                if input[0].l || input[0].r || input[0].l_trigger > 0.165 || input[0].r_trigger > 0.165 {
                    self.set_action(Action::ShieldOn);
                    true
                } else {
                    false
                }
            }
            (&None, &None) => false
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

    fn action_expired(&mut self, input: &PlayerInput, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform]) {
        let fighter = &fighters[self.fighter.as_ref()];
        match Action::from_index(self.action) {
            None => { panic!("Custom defined action expirations have not been implemented"); },

            // Idle
            Some(Action::Spawn)      => self.set_action(Action::SpawnIdle),
            Some(Action::SpawnIdle)  => self.set_action(Action::SpawnIdle),
            Some(Action::Idle)       => self.set_action(Action::Idle),
            Some(Action::LedgeIdle)  => self.set_action(Action::LedgeIdle),
            Some(Action::Teeter)     => self.set_action(Action::TeeterIdle),
            Some(Action::TeeterIdle) => self.set_action(Action::TeeterIdle),

            // crouch
            Some(Action::CrouchStart) => self.set_action(Action::Crouch),
            Some(Action::Crouch)      => self.set_action(Action::Crouch),
            Some(Action::CrouchEnd)   => self.set_action(Action::Idle),

            // Movement
            Some(Action::Fall)           => self.set_action(Action::Fall),
            Some(Action::AerialFall)     => self.set_action(Action::AerialFall),
            Some(Action::Land)           => self.set_action(Action::Idle),
            Some(Action::JumpF)          => self.set_action(Action::Fall),
            Some(Action::JumpB)          => self.set_action(Action::Fall),
            Some(Action::JumpAerialF)    => self.set_action(Action::AerialFall),
            Some(Action::JumpAerialB)    => self.set_action(Action::AerialFall),
            Some(Action::TurnDash)       => self.set_action(Action::Dash),
            Some(Action::TurnRun)        => self.set_action(Action::Idle),
            Some(Action::Dash)           => self.set_action(Action::Run),
            Some(Action::Run)            => self.set_action(Action::Run),
            Some(Action::RunEnd)         => self.set_action(Action::Idle),
            Some(Action::Walk)           => self.set_action(Action::Walk),
            Some(Action::PassPlatform)   => self.set_action(Action::AerialFall),
            Some(Action::Damage)         => self.set_action(Action::Damage),
            Some(Action::DamageFly)      => self.set_action(Action::DamageFly),
            Some(Action::DamageFall)     => self.set_action(Action::DamageFall),
            Some(Action::LedgeGetup)     => self.set_action_idle_from_ledge(players, fighters, platforms),
            Some(Action::LedgeGetupSlow) => self.set_action_idle_from_ledge(players, fighters, platforms),
            Some(Action::LedgeJump)      => self.set_action_fall_from_ledge_jump(players, fighters, platforms),
            Some(Action::LedgeJumpSlow)  => self.set_action_fall_from_ledge_jump(players, fighters, platforms),
            Some(Action::LedgeGrab) => {
                self.set_action(Action::LedgeIdle);
                self.ledge_idle_timer = 0;
            },
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
                self.set_airbourne(players, fighters, platforms);

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

                self.x_vel = self.x_vel * fighter.jump_x_vel_ground_mult + input[0].stick_x * fighter.jump_x_init_vel;
                if self.x_vel.abs() > fighter.jump_x_term_vel {
                    self.x_vel = fighter.jump_x_term_vel * self.x_vel.signum();
                }

                if self.relative_f(input[2].stick_x) >= -0.3 {
                    self.set_action(Action::JumpF);
                }
                else {
                    self.set_action(Action::JumpB);
                }
            },

            // Defense
            Some(Action::PowerShield)   => if fighter.shield.is_some() { self.set_action(Action::Shield) } else { self.set_action(Action::Idle) },
            Some(Action::ShieldOn)      => self.set_action(Action::Shield),
            Some(Action::Shield)        => self.set_action(Action::Shield),
            Some(Action::ShieldOff)     => self.set_action(Action::Idle),
            Some(Action::RollF)         => self.set_action(Action::Idle),
            Some(Action::RollB)         => self.set_action(Action::Idle),
            Some(Action::SpotDodge)     => self.set_action(Action::Idle),
            Some(Action::AerialDodge)   => self.set_action(Action::SpecialFall),
            Some(Action::SpecialFall)   => self.set_action(Action::SpecialFall),
            Some(Action::SpecialLand)   => self.set_action(Action::Idle),
            Some(Action::TechF)         => self.set_action(Action::Idle),
            Some(Action::TechS)         => self.set_action(Action::Idle),
            Some(Action::TechB)         => self.set_action(Action::Idle),
            Some(Action::Rebound)       => self.set_action(Action::Idle),
            Some(Action::LedgeRoll)     => self.set_action_idle_from_ledge(players, fighters, platforms),
            Some(Action::LedgeRollSlow) => self.set_action_idle_from_ledge(players, fighters, platforms),

            // Vulnerable
            Some(Action::ShieldBreakFall)  => self.set_action(Action::ShieldBreakFall),
            Some(Action::Stun)             => self.set_action(Action::Stun),
            Some(Action::ShieldBreakGetup) => {
                self.stun_timer = 490;
                self.set_action(Action::Stun);
            }

            // Attack
            Some(Action::Jab)             => self.set_action(Action::Idle),
            Some(Action::Jab2)            => self.set_action(Action::Idle),
            Some(Action::Jab3)            => self.set_action(Action::Idle),
            Some(Action::Utilt)           => self.set_action(Action::Idle),
            Some(Action::Dtilt)           => self.set_action(Action::Crouch),
            Some(Action::Ftilt)           => self.set_action(Action::Idle),
            Some(Action::DashAttack)      => self.set_action(Action::Idle),
            Some(Action::Usmash)          => self.set_action(Action::Idle),
            Some(Action::Dsmash)          => self.set_action(Action::Idle),
            Some(Action::Fsmash)          => self.set_action(Action::Idle),
            Some(Action::Grab)            => self.set_action(Action::Idle),
            Some(Action::DashGrab)        => self.set_action(Action::Idle),
            Some(Action::LedgeAttack)     => self.set_action_idle_from_ledge(players, fighters, platforms),
            Some(Action::LedgeAttackSlow) => self.set_action_idle_from_ledge(players, fighters, platforms),

            // Aerials
            Some(Action::Uair)     => self.set_action(Action::Fall),
            Some(Action::Dair)     => self.set_action(Action::Fall),
            Some(Action::Fair)     => self.set_action(Action::Fall),
            Some(Action::Bair)     => self.set_action(Action::Fall),
            Some(Action::Nair)     => self.set_action(Action::Fall),
            Some(Action::UairLand) => self.set_action(Action::Idle),
            Some(Action::DairLand) => self.set_action(Action::Idle),
            Some(Action::FairLand) => self.set_action(Action::Idle),
            Some(Action::BairLand) => self.set_action(Action::Idle),
            Some(Action::NairLand) => self.set_action(Action::Idle),

            // Taunts
            Some(Action::TauntUp)    => self.set_action(Action::Idle),
            Some(Action::TauntDown)  => self.set_action(Action::Idle),
            Some(Action::TauntLeft)  => self.set_action(Action::Idle),
            Some(Action::TauntRight) => self.set_action(Action::Idle),

            Some(Action::Eliminated)         => self.set_action(Action::Eliminated),
            Some(Action::DummyFramePreStart) => self.set_action(Action::Spawn),
        };
    }

    pub fn set_action_idle_from_ledge(&mut self, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform]) {
        if let Location::GrabbedLedge { platform_i, .. } = self.location {
            let platform = &platforms[platform_i];
            let (world_x, _) = self.bps_xy(players, fighters, platforms);
            let x = platform.world_x_to_plat_x_clamp(world_x);

            self.location = Location::Platform { platform_i, x };
            self.set_action(Action::Idle);
            self.frames_since_ledge = 0;
        }
        else {
            panic!("Location must be on ledge to call this function.")
        }
    }

    pub fn set_action_fall_from_ledge_jump(&mut self, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform]) {
        self.set_airbourne(players, fighters, platforms);
        self.set_action(Action::Fall);
        self.frames_since_ledge = 0;
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
        for colbox in &mut frame.colboxes[..] {
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
        self.fall_action(fighter);
        self.air_drift(input, fighter);
    }

    fn fall_action(&mut self, fighter: &Fighter) {
        self.y_vel += fighter.gravity;
        if self.y_vel < fighter.terminal_vel {
            self.y_vel = fighter.terminal_vel;
        }
    }

    fn fastfall_action(&mut self, input: &PlayerInput, fighter: &Fighter) {
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

    pub fn physics_step(&mut self, input: &PlayerInput, players: &[Player], fighters: &KeyedContextVec<Fighter>, stage: &Stage, game_frame: usize, goal: Goal) {
        if let Hitlag::None = self.hitlag {
            let fighter = &fighters[self.fighter.as_ref()];

            if self.kb_x_vel.abs() > 0.0 {
                let vel_dir = self.kb_x_vel.signum();
                if self.is_airbourne() {
                    self.kb_x_vel -= self.kb_x_dec;
                } else {
                    self.kb_x_vel -= vel_dir * fighter.friction;
                }
                if vel_dir != self.kb_x_vel.signum() {
                    self.kb_x_vel = 0.0;
                }
            }

            if self.kb_y_vel.abs() > 0.0 {
                if self.is_airbourne() {
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

            match self.location.clone() {
                Location::Airbourne { x, y } => {
                    let new_x = x + self.x_vel + self.kb_x_vel;
                    let new_y = y + self.y_vel + self.kb_y_vel;
                    if let Some(platform_i) = self.land_stage_collision(fighter, stage, (x, y), (new_x, new_y), input) {
                        let x = stage.platforms[platform_i].world_x_to_plat_x(new_x);
                        self.land(fighter, platform_i, x);
                    } else {
                        self.location = Location::Airbourne { x: new_x, y: new_y };
                    }
                }
                Location::Platform { platform_i, mut x } => {
                    let platform = &stage.platforms[platform_i];
                    x += self.x_vel + self.kb_x_vel;
                    if stage.platforms.get(platform_i).map_or(false, |plat| plat.plat_x_in_bounds(x)) {
                        self.location = Location::Platform { platform_i, x };
                    }
                    else if !fighter.actions[self.action as usize].frames[self.frame as usize].ledge_cancel {
                        self.location = Location::Platform { platform_i, x: platform.plat_x_clamp(x) };
                    }
                    else if self.face_right && x < 0.0 || !self.face_right && x >= 0.0 || // facing away from the ledge
                      self.relative_f(input.stick_x.value) > 0.6
                    {
                        // fall
                        self.set_action(Action::Fall);
                        self.set_airbourne(players, fighters, &stage.platforms);

                        // force set past platform
                        let x_offset = if x > 0.0 { 0.000001 } else { -0.000001 }; // just being cautious, probably dont need this
                        let (air_x, air_y) = platform.plat_x_to_world_p(x + x_offset);
                        self.location = Location::Airbourne { x: air_x, y: air_y };
                    }
                    else {
                        self.x_vel = 0.0;
                        self.location = Location::Platform { platform_i, x: platform.plat_x_clamp(x) };
                        self.set_action(Action::Teeter);
                    }
                }
                _ => { }
            }

            // death
            let blast = &stage.blast;
            let (x, y) = self.bps_xy(players, fighters, &stage.platforms);
            if x < blast.left || x > blast.right || y < blast.bot || y > blast.top {
                self.die(fighter, game_frame, goal);
            }

            // ledge grabs
            let fighter_frame = &fighter.actions[self.action as usize].frames[self.frame as usize];
            self.frames_since_ledge += 1;
            if self.frames_since_ledge >= 30 && self.y_vel < 0.0 && input.stick_y.value > -0.5 {
                if let Some(ref ledge_grab_box) = fighter_frame.ledge_grab_box {
                    self.check_ledge_grab(players, &fighter, &ledge_grab_box, &stage.platforms);
                }
            }
        }
    }

    fn apply_friction(&mut self, fighter: &Fighter) {
        match Action::from_index(self.action) {
            Some(Action::Idle) |
            Some(Action::Dash) |
            Some(Action::Shield) |
            Some(Action::ShieldOn) |
            Some(Action::ShieldOff) |
            Some(Action::Damage)
              => { self.apply_friction_weak(fighter) }
            _ => { self.apply_friction_strong(fighter) }
        }
    }

    // TODO: These functions are split up as weak/strong so that one day they may be called individually by player scripts
    fn apply_friction_weak(&mut self, fighter: &Fighter) {
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

    fn apply_friction_strong(&mut self, fighter: &Fighter) {
        if self.x_vel > 0.0 {
            self.x_vel -= fighter.friction * if self.x_vel > fighter.walk_max_vel { 2.0 } else { 1.0 };
            if self.x_vel < 0.0 {
                self.x_vel = 0.0;
            }
        }
        else {
            self.x_vel += fighter.friction * if self.x_vel < -fighter.walk_max_vel { 2.0 } else { 1.0 };
            if self.x_vel > 0.0 {
                self.x_vel = 0.0;
            }
        }
    }

    /// returns the index platform that the player will land on
    fn land_stage_collision(&mut self, fighter: &Fighter, stage: &Stage, old_p: (f32, f32), new_p: (f32, f32), input: &PlayerInput) -> Option<usize> {
        if new_p.1 > old_p.1 {
            return None;
        }

        for (platform_i, platform) in stage.platforms.iter().enumerate() {
            if !self.pass_through_platform(fighter, platform, input) &&
              math::segments_intersect(old_p, new_p, platform.p1(), platform.p2()) {
                return Some(platform_i)
            }
        }
        None
    }

    pub fn pass_through_platform(&self, fighter: &Fighter, platform: &Platform, input: &PlayerInput) -> bool {
        let fighter_frame = &fighter.actions[self.action as usize].frames[self.frame as usize];
        platform.pass_through && fighter_frame.pass_through && input[0].stick_y <= -0.56
    }

    /// Returns the area sorounding the player that the camera must include
    pub fn cam_area(&self, cam_max: &Area, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform]) -> Area {
        let (x, y) = self.bps_xy(players, fighters, platforms);
        let mut left  = x;
        let mut right = x;
        let mut bot   = y - 5.0;
        let mut top   = y + 25.0;

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

    fn land(&mut self, fighter: &Fighter, platform_i: usize, x: f32) {
        match Action::from_index(self.action) {
            Some(Action::Uair)            => self.set_action(Action::UairLand),
            Some(Action::Dair)            => self.set_action(Action::DairLand),
            Some(Action::Fair)            => self.set_action(Action::FairLand),
            Some(Action::Bair)            => self.set_action(Action::BairLand),
            Some(Action::Nair)            => self.set_action(Action::NairLand),
            Some(Action::ShieldBreakFall) => self.set_action(Action::ShieldBreakGetup),
            _ if self.y_vel >= -1.0 => { self.set_action(Action::Idle) }, // no impact land

            Some(Action::SpecialFall) |
            Some(Action::AerialDodge) |
            None    => self.set_action(Action::SpecialLand),
            Some(_) => self.set_action(Action::Land)
        }

        self.y_vel = 0.0;
        self.fastfalled = false;
        self.air_jumps_left = fighter.air_jumps;
        self.hit_by = None;
        self.location = Location::Platform { platform_i, x };
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

    fn die(&mut self, fighter: &Fighter, game_frame: usize, goal: Goal) {
        self.damage = 0.0;
        self.location = Location::Airbourne { x: self.respawn.x, y: self.respawn.y };
        self.face_right = self.respawn.face_right;
        self.x_vel = 0.0;
        self.y_vel = 0.0;
        self.kb_x_vel = 0.0;
        self.kb_y_vel = 0.0;
        self.air_jumps_left = fighter.air_jumps;
        self.fastfalled = false;
        self.hitstun = 0.0;
        self.hitlag = Hitlag::None;

        self.result.deaths.push(DeathRecord {
            player: self.hit_by,
            frame: game_frame,
        });

        match goal {
            Goal::LastManStanding => {
                if let Some(mut stocks) = self.stocks {
                    stocks -= 1;
                    self.stocks = Some(stocks);

                    if stocks == 0 {
                        self.set_action(Action::Eliminated);
                    }
                    else {
                        self.set_action(Action::Spawn);
                    }
                }
            }
            Goal::KillDeathScore => {
                self.set_action(Action::Spawn);
            }
        }
    }

    fn check_ledge_grab(&mut self, players: &[Player], fighter: &Fighter, ledge_grab_box: &LedgeGrabBox, platforms: &[Platform]) {
        for (platform_i, platform) in platforms.iter().enumerate() {
            let left_grab  = platform.left_grab()  && self.check_ledge_collision(ledge_grab_box, platform.left_ledge()) && players.iter().all(|x| !x.is_hogging_ledge(platform_i, true));
            let right_grab = platform.right_grab() && self.check_ledge_collision(ledge_grab_box, platform.right_ledge()) && players.iter().all(|x| !x.is_hogging_ledge(platform_i, false));

            // If both left and right ledges are in range then keep the same direction.
            // This prevents always facing left or right on small platforms.
            if left_grab && !right_grab {
                self.face_right = true;
            }
            else if !left_grab && right_grab {
                self.face_right = false;
            }

            if left_grab || right_grab {
                self.x_vel = 0.0;
                self.y_vel = 0.0;
                self.fastfalled = false;
                self.air_jumps_left = fighter.air_jumps;
                self.hit_by = None;
                self.location = Location::GrabbedLedge { platform_i, d_x: -3.0, d_y: -24.0, logic: LedgeLogic::Hog };
                self.set_action(Action::LedgeGrab);
            }
        }
    }

    fn check_ledge_collision(&self, ledge_grab_box: &LedgeGrabBox, ledge: (f32, f32)) -> bool {
        if let Location::Airbourne { x: p_x, y: p_y } = self.location {
            let b_x1 = self.relative_f(ledge_grab_box.x1).min(self.relative_f(ledge_grab_box.x2));
            let b_y1 =                 ledge_grab_box.y1.min(ledge_grab_box.y2);

            let b_x2 = self.relative_f(ledge_grab_box.x1).max(self.relative_f(ledge_grab_box.x2));
            let b_y2 =                 ledge_grab_box.y1.max(ledge_grab_box.y2);

            let (l_x, l_y) = ledge;

            l_x > p_x + b_x1 && l_x < p_x + b_x2 &&
            l_y > p_y + b_y1 && l_y < p_y + b_y2
        } else {
            false
        }
    }

    pub fn debug_print(&self, fighter: &Fighter, player_input: &PlayerInput, debug: &DebugPlayer, index: usize) -> Vec<String> {
        let mut lines: Vec<String> = vec!();
        if debug.physics {
            lines.push(format!("Player: {}    location: {:?}    x_vel: {:.5}    y_vel: {:.5}    kb_x_vel: {:.5}    kb_y_vel: {:.5} ",
                index, self.location, self.x_vel, self.y_vel, self.kb_x_vel, self.kb_y_vel));
        }

        if debug.input {
            let stick_x   = player_input.stick_x.value;
            let stick_y   = player_input.stick_y.value;
            let c_stick_x = player_input.c_stick_x.value;
            let c_stick_y = player_input.c_stick_y.value;
            let l_trigger = player_input.l_trigger.value;
            let r_trigger = player_input.r_trigger.value;

            lines.push(format!("Player: {}    VALUE    stick_x: {:.5}    stick_y: {:.5}    c_stick_x: {:.5}    c_stick_y: {:.5}    l_trigger: {:.5}    r_trigger: {:.5}",
                index, stick_x, stick_y, c_stick_x, c_stick_y, l_trigger, r_trigger));
        }

        if debug.input_diff {
            let stick_x   = player_input.stick_x.diff;
            let stick_y   = player_input.stick_y.diff;
            let c_stick_x = player_input.c_stick_x.diff;
            let c_stick_y = player_input.c_stick_y.diff;
            let l_trigger = player_input.l_trigger.diff;
            let r_trigger = player_input.r_trigger.diff;

            lines.push(format!("Player: {}    DIFF    stick_x: {:.5}    stick_y: {:.5}    c_stick_x: {:.5}    c_stick_y: {:.5}    l_trigger: {:.5}    r_trigger: {:.5}",
                index, stick_x, stick_y, c_stick_x, c_stick_y, l_trigger, r_trigger));
        }

        if debug.action {
            let action = Action::from_index(self.action).unwrap();
            let action_frames = fighter.actions[self.action as usize].frames.len() as u64 - 1;
            let iasa = fighter.actions[self.action as usize].iasa;

            lines.push(format!("Player: {}    action: {:?}    frame: {}/{}    IASA: {}",
                index, action, self.frame, action_frames, iasa));
        }

        if debug.frame {
            let frames = &fighter.actions[self.action as usize].frames;
            if frames.len() > self.frame as usize {
                let frame = &frames[self.frame as usize];
                let hitbox_count = frame.colboxes.len();
                let effects_count = frame.effects.len();
                lines.push(format!("Player: {}    colboxes: {}    effects: {}",
                    index, hitbox_count, effects_count));
            }
            else {
                lines.push(format!("Player: {}    frame {} does not exist.", index, self.frame));
            }
        }
        lines
    }

    pub fn render(&self, fighter_color: [f32; 4], selected_colboxes: HashSet<usize>, fighter_selected: bool, player_selected: bool, debug: DebugPlayer, players: &[Player], fighters: &KeyedContextVec<Fighter>, platforms: &[Platform]) -> RenderPlayer {
        let fighter = &fighters[self.fighter.as_ref()];
        let mut vector_arrows = vec!();
        if debug.stick_vector {
            if let Some((x, y)) = self.stick {
                vector_arrows.push(VectorArrow {
                    x,
                    y,
                    color: [0.7, 0.7, 0.7, 1.0]
                });
            }
        }
        if debug.c_stick_vector {
            if let Some((x, y)) = self.c_stick {
                vector_arrows.push(VectorArrow {
                    x,
                    y,
                    color: [1.0, 1.0, 0.0, 1.0]
                });
            }
        }
        if debug.di_vector {
            if let Some(angle) = self.hit_angle_pre_di {
                vector_arrows.push(VectorArrow {
                    x: angle.cos(),
                    y: angle.sin(),
                    color: [1.0, 0.0, 0.0, 1.0]
                });
            }
            if let Some(angle) = self.hit_angle_post_di {
                vector_arrows.push(VectorArrow {
                    x: angle.cos(),
                    y: angle.sin(),
                    color: [0.0, 1.0, 0.0, 1.0]
                });
            }
        }

        let shield = if self.is_shielding() {
            if let &Some(ref shield) = &fighter.shield {
                let c = &fighter_color;
                let m =  1.0 - self.shield_analog;
                Some(RenderShield {
                    distort: self.shield_stun_timer,
                    color:   [c[0] + (1.0 - c[0]) * m, c[1] + (1.0 - c[1]) * m, c[2] + (1.0 - c[2]) * m, 0.2 + self.shield_analog / 2.0],
                    radius:  self.shield_size(shield),
                    pos:     self.shield_pos(shield, players, fighters, platforms),
                })
            } else { None }
        } else { None };

        RenderPlayer {
            debug:             debug,
            damage:            self.damage,
            stocks:            self.stocks,
            bps:               self.bps_xy(players, fighters, platforms),
            ecb:               self.ecb.clone(),
            frame:             self.frame as usize,
            action:            self.action as usize,
            fighter:           self.fighter.clone(),
            face_right:        self.face_right,
            fighter_color:     fighter_color,
            fighter_selected:  fighter_selected,
            player_selected:   player_selected,
            selected_colboxes: selected_colboxes,
            shield:            shield,
            vector_arrows:     vector_arrows,
        }
    }

    pub fn result(&self) -> RawPlayerResult {
        let mut result = self.result.clone();
        result.final_damage = Some(self.damage);
        result.ended_as_fighter = Some(self.fighter.clone());
        result
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
    pub damage:            f32,
    pub stocks:            Option<u64>,
    pub bps:               (f32, f32),
    pub ecb:               ECB,
    pub frame:             usize,
    pub action:            usize,
    pub fighter:           String,
    pub face_right:        bool,
    pub fighter_color:     [f32; 4],
    pub fighter_selected:  bool,
    pub player_selected:   bool,
    pub selected_colboxes: HashSet<usize>,
    pub shield:            Option<RenderShield>,
    pub vector_arrows:     Vec<VectorArrow>,
}

pub struct RenderShield {
    pub distort: u64,
    pub color:   [f32; 4],
    pub radius:  f32,
    pub pos:     (f32, f32),
}

pub struct VectorArrow {
    pub x:     f32,
    pub y:     f32,
    pub color: [f32; 4]
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
