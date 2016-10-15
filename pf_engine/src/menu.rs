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
    state:         MenuState,
    current_frame: usize,
}

impl Menu {
    pub fn new() -> Menu {
        Menu {
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

            vec!(MenuChoice::Start {
                controllers: controllers,
                fighters:    selected_fighters,
                stage:       0,
                netplay:     false,
            })
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
}

pub enum MenuChoice {
    ChangePackage (String),
    Start { controllers: Vec<usize>, fighters: Vec<usize>, stage: usize , netplay: bool},
}

#[allow(dead_code)]
pub struct RenderMenu {
    state: MenuState,
}
