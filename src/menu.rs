use ::input::{Input};

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

    pub fn run(&mut self, input: &mut Input) -> MenuChoice {
        loop {
            input.update();
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

                return MenuChoice {
                    package_name: "base_package".to_string(),
                    selected_fighters: selected_fighters,
                    selected_stage: 0,
                }
            }
            self.frames += 1
        }
    }
}

pub struct MenuChoice {
    pub package_name: String,
    pub selected_fighters: Vec<usize>,
    pub selected_stage: usize,
}
