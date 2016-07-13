use ::input::{Input};

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
    state: MenuState,
    frames: u64,
}

impl Menu {
    pub fn new() -> Menu {
        Menu {
            state: MenuState::CharacterSelect,
            frames: 0,
        }
    }

    fn step_select(&mut self) {
    }

    pub fn step(&mut self, input: &mut Input) -> Vec<MenuChoice> {
        match self.state {
            MenuState::CharacterSelect => { self.step_select(); },
            MenuState::StageSelect     => { self.step_select(); },

            MenuState::SetRules        => { self.step_select(); },

            MenuState::SwitchPackages  => { self.step_select(); },
            MenuState::BrowsePackages  => { self.step_select(); },
            MenuState::CreatePackage   => { self.step_select(); },

            MenuState::BrowseFighter   => { self.step_select(); },
            MenuState::CreateFighter   => { self.step_select(); },
        }
        if input.start_pressed() {
            let mut selected_fighters: Vec<usize> = vec!();
            for _ in input.player_inputs() {
                selected_fighters.push(0);
            }

            let mut controllers: Vec<usize> = vec!();
            for (i, _) in input.player_inputs().iter().enumerate() {
                controllers.push(i);
            }

            return vec!(MenuChoice::Start {
                controllers: controllers,
                fighters:    selected_fighters,
                stage:       0,
                netplay:     false,
            });
        }
        self.frames += 1;
        return vec!();
    }

    pub fn render(&self) -> RenderMenu {
        RenderMenu {
            state: self.state.clone(),
        }
    }
}

pub enum MenuChoice {
    ChangePackage (String),
    Start { controllers: Vec<usize>, fighters: Vec<usize>, stage: usize , netplay: bool},
}

#[allow(dead_code)]
pub struct RenderMenu {
    state: MenuState,
}
