#[allow(dead_code)]
enum MenuState {
    CharacterSelect,
    StageSelect,

    SetRules,

    SwitchPackages,
    BrowsePackages,
    CreatePackage,

    BrowsePlayers,
    CreatePlayer,
}

#[allow(dead_code)]
pub struct Menu {
    state:         MenuState,
}

impl Menu {
    pub fn new() -> Menu {
        Menu {
            state:    MenuState::CharacterSelect,
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

                MenuState::BrowsePlayers   => { self.step_select(); },
                MenuState::CreatePlayer    => { self.step_select(); },
            }

            return MenuChoice {
                package_name: "base_package".to_string(),
                fighter_names: vec!("base_fighter".to_string()),
                stage_name: "base_stage".to_string(),
            }
        }
    }
}

pub struct MenuChoice {
    pub package_name: String,
    pub fighter_names: Vec<String>,
    pub stage_name: String,
}
