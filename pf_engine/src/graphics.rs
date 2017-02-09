use ::menu::RenderMenu;
use ::package::PackageUpdate;
use ::game::RenderGame;

pub struct GraphicsMessage {
    pub render: Render,
    pub package_updates: Vec<PackageUpdate>,
}

pub enum Render {
    Game (RenderGame),
    Menu (RenderMenu),
}
