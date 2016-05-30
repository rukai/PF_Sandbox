#[allow(dead_code)]
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

#[allow(dead_code)]
pub struct Menu {
    state:         MenuState,
}

impl Menu {
    pub fn new() -> Menu {
        Menu {
            state: MenuState::CharacterSelect,
        }
    }

    fn step_select(&mut self) {
    }

    pub fn run(&mut self) -> MenuChoice {
        loop {
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

            return MenuChoice {
                package_name: "base_package".to_string(),
                selected_fighters: vec!(0, 0),
                selected_stage: 0,
            }
        }
    }
}

pub struct MenuChoice {
    pub package_name: String,
    pub selected_fighters: Vec<usize>,
    pub selected_stage: usize,
}
