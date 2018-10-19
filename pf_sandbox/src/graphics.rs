use pf_sandbox_lib::fighter::CollisionBoxRole;
use pf_sandbox_lib::package::PackageUpdate;
use game::RenderGame;
use menu::RenderMenu;

pub struct GraphicsMessage {
    pub render:          Render,
    pub package_updates: Vec<PackageUpdate>,
}

pub struct Render {
    pub command_output: Vec<String>,
    pub render_type:    RenderType,
    pub fullscreen:     bool,
}

pub enum RenderType {
    Game (RenderGame),
    #[allow(dead_code)] // Needed for headless build
    Menu (RenderMenu),
}

#[allow(unused)] // Needed for headless build
pub fn get_render_id(role: &CollisionBoxRole) -> u32 {
    match role {
        &CollisionBoxRole::Hurt (_)       => { 1 }
        &CollisionBoxRole::Hit (_)        => { 2 }
        &CollisionBoxRole::Grab           => { 3 }
        &CollisionBoxRole::Intangible     => { 4 }
        &CollisionBoxRole::IntangibleItem => { 5 }
        &CollisionBoxRole::Invincible     => { 6 }
        &CollisionBoxRole::Reflect        => { 7 }
        &CollisionBoxRole::Absorb         => { 8 }
    }
}

#[allow(unused)] // Needed for headless build
pub fn get_team_color4(i: usize) -> [f32; 4] {
    let colors = get_colors();
    let color = colors[i % colors.len()].value;
    [color[0]/255.0, color[1]/255.0, color[2]/255.0, 1.0]
}

pub fn get_team_color3(i: usize) -> [f32; 3] {
    let colors = get_colors();
    let color = colors[i % colors.len()].value;
    [color[0]/255.0, color[1]/255.0, color[2]/255.0]
}

pub struct Color {
    pub name: String,
    pub value: [f32; 3]
}

pub fn get_colors() -> Vec<Color> {
    vec!(
        Color { name: String::from("Blue"),   value: [0.0,   90.0,   224.0] },
        Color { name: String::from("Orange"), value: [239.0, 100.0,  0.0] },
        Color { name: String::from("Red"),    value: [255.0, 0.0,    40.0] },
        Color { name: String::from("Green"),  value: [10.0,  150.0,  38.0] },

        Color { name: String::from("Pink"),       value: [255.0, 0.0,   163.0] },
        Color { name: String::from("Green #2"),   value: [124.0, 184.0, 0.0] },
        Color { name: String::from("Purple"),     value: [120.0, 46.0,  252.0] },
        Color { name: String::from("Light Blue"), value: [81.0,  229.0, 237.0] },
    )
}
