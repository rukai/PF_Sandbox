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

pub fn get_player_color(i: usize) -> [f32; 4] {
    let colors: Vec<[f32; 4]> = vec!(
        [0.0, 90.0/255.0, 224.0/255.0, 1.0],
        [239.0/255.0, 100.0/255.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0, 1.0],
    );
    colors[i]
}
