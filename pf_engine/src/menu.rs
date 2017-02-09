use ::input::Input;
use ::package::Package;
use ::graphics::{GraphicsMessage, Render};
use ::app::GameSetup;

#[allow(dead_code)]
#[derive(Clone)]
enum MenuState {
    CharacterSelect,
    StageSelect,

    SetRules,

    SwitchPackages,
    BrowsePackages,
    CreatePackage,

    BrowseFighter,
    CreateFighter,
}

pub struct Menu {
    package:       Package,
    state:         MenuState,
    current_frame: usize,
}

impl Menu {
    pub fn new() -> Menu {
        let package = Package::open_or_generate("base_package");
        Menu {
            package:       package,
            state:         MenuState::CharacterSelect,
            current_frame: 0,
        }
    }

    fn step_select(&mut self, input: &mut Input) -> Vec<MenuChoice> {
        let player_inputs = input.players(self.current_frame);
        if input.start_pressed() {
            let mut selected_fighters: Vec<usize> = vec!();
            for _ in &player_inputs {
                selected_fighters.push(0);
            }

            let mut controllers: Vec<usize> = vec!();
            for (i, _) in (&player_inputs).iter().enumerate() {
                controllers.push(i);
            }

            vec!(MenuChoice::Start (GameSetup {
                controllers: controllers,
                fighters:    selected_fighters,
                stage:       0,
                netplay:     false,
            }))
        }
        else {
            vec!()
        }
    }

    pub fn step(&mut self, input: &mut Input) -> Vec<MenuChoice> {
        input.game_update(self.current_frame);

        let result = match self.state {
            MenuState::CharacterSelect => { self.step_select(input) },
            MenuState::StageSelect     => { self.step_select(input) },

            MenuState::SetRules        => { self.step_select(input) },

            MenuState::SwitchPackages  => { self.step_select(input) },
            MenuState::BrowsePackages  => { self.step_select(input) },
            MenuState::CreatePackage   => { self.step_select(input) },

            MenuState::BrowseFighter   => { self.step_select(input) },
            MenuState::CreateFighter   => { self.step_select(input) },
        };

        self.current_frame += 1;
        result
    }

    pub fn render(&self) -> RenderMenu {
        RenderMenu {
            state: self.state.clone(),
        }
    }

    pub fn graphics_message(&mut self) -> GraphicsMessage {
        GraphicsMessage {
            package_updates: self.package.updates(),
            render: Render::Menu (self.render())
        }
    }

    pub fn reclaim(self) -> Package {
        self.package
    }
}

pub enum MenuChoice {
    Start (GameSetup)
}

#[allow(dead_code)]
pub struct RenderMenu {
    state: MenuState,
}
