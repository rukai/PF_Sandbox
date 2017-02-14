use ::menu::RenderMenu;
use ::package::PackageUpdate;
use ::game::RenderGame;
use ::fighter::CollisionBoxRole;

pub struct GraphicsMessage {
    pub render: Render,
    pub package_updates: Vec<PackageUpdate>,
}

pub enum Render {
    Game (RenderGame),
    Menu (RenderMenu),
}

pub fn get_render_id(role: &CollisionBoxRole) -> f32 {
    match role {
        &CollisionBoxRole::Hurt (_)       => { 1.0 }
        &CollisionBoxRole::Hit (_)        => { 2.0 }
        &CollisionBoxRole::Grab           => { 3.0 }
        &CollisionBoxRole::Intangible     => { 4.0 }
        &CollisionBoxRole::IntangibleItem => { 5.0 }
        &CollisionBoxRole::Invincible     => { 6.0 }
        &CollisionBoxRole::Reflect        => { 7.0 }
        &CollisionBoxRole::Absorb         => { 8.0 }
    }
}
