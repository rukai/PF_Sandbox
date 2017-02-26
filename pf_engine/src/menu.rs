use ::input::{Input, PlayerInput};
use ::package::Package;
use ::graphics::{GraphicsMessage, Render};
use ::app::GameSetup;

pub struct Menu {
    package:              Package,
    state:                MenuState,
    current_frame:        usize,
    fighter_selections: Vec<CharacterSelect>,
    stage_selection:      usize,
}

impl Menu {
    pub fn new() -> Menu {
        let package = Package::open_or_generate("base_package");
        Menu {
            package:              package,
            state:                MenuState::CharacterSelect,
            fighter_selections:   vec!(),
            stage_selection:      0,
            current_frame:        0,
        }
    }

    fn add_remove_fighter_selections(&mut self, player_inputs: &[PlayerInput]) {
        // TODO: add/remove fighter_selections on input add/remove

        // HACK
        if self.fighter_selections.len() == 0 {
            for input in player_inputs {
                self.fighter_selections.push(CharacterSelect {
                    plugged_in: input.plugged_in,
                    selection:  None,
                    cursor:     0,
                });
            }
        }
    }

    fn step_fighter_select(&mut self, player_inputs: &[PlayerInput], input: &mut Input) {
        {
            // update selections
            let mut selections = &mut self.fighter_selections.iter_mut();
            let fighters = &self.package.fighters;
            for (ref mut selection, ref input) in selections.zip(player_inputs) {
                selection.plugged_in = input.plugged_in;
                match selection.selection {
                    Some (_) => {
                        if input.b.press {
                            selection.selection = None;
                        }
                    }
                    None => {
                        if input.a.press && selection.cursor < fighters.len() {
                            selection.selection = Some(selection.cursor);
                        }
                    }
                }

                if input.a.press && selection.cursor >= fighters.len() {
                    // TODO: run extra options
                }
            }
        }

        if input.start_pressed() && self.fighter_selections.iter().all(|x| !x.plugged_in || x.selection.is_some()) {
            self.state = MenuState::StageSelect;
        }
    }

    fn step_stage_select(&mut self, player_inputs: &[PlayerInput], input: &mut Input) {
        for _ in player_inputs {
        }

        if input.start_pressed() {
            self.state = MenuState::StartGame;
        }
    }

    pub fn step(&mut self, input: &mut Input) -> Option<GameSetup> {
        input.game_update(self.current_frame);
        let player_inputs = input.players(self.current_frame);

        self.add_remove_fighter_selections(&player_inputs);

        match self.state {
            MenuState::CharacterSelect => { self.step_fighter_select(&player_inputs, input) }
            MenuState::StageSelect     => { self.step_stage_select  (&player_inputs, input) }
            MenuState::SetRules        => { self.step_stage_select  (&player_inputs, input) }
            MenuState::SwitchPackages  => { self.step_stage_select  (&player_inputs, input) }
            MenuState::BrowsePackages  => { self.step_stage_select  (&player_inputs, input) }
            MenuState::CreatePackage   => { self.step_stage_select  (&player_inputs, input) }
            MenuState::CreateFighter   => { self.step_stage_select  (&player_inputs, input) }
            MenuState::StartGame       => { self.step_stage_select  (&player_inputs, input) }
        };

        self.current_frame += 1;

        if let MenuState::StartGame = self.state {
            let mut selected_fighters: Vec<usize> = vec!();
            for _ in &player_inputs {
                selected_fighters.push(0);
            }

            let mut controllers: Vec<usize> = vec!();
            for (i, _) in (&player_inputs).iter().enumerate() {
                controllers.push(i);
            }

            Some(GameSetup {
                controllers: controllers,
                fighters:    selected_fighters,
                stage:       0,
                netplay:     false,
            })
        }
        else {
            None
        }
    }

    pub fn render(&self) -> RenderMenu {
        RenderMenu {
            state: match self.state {
                MenuState::CharacterSelect => { RenderMenuState::CharacterSelect (self.fighter_selections.clone()) }
                MenuState::StageSelect     => { RenderMenuState::StageSelect     (self.stage_selection) }
                MenuState::SetRules        => { RenderMenuState::SetRules }
                MenuState::SwitchPackages  => { RenderMenuState::SwitchPackages }
                MenuState::BrowsePackages  => { RenderMenuState::BrowsePackages }
                MenuState::CreatePackage   => { RenderMenuState::CreatePackage }
                MenuState::CreateFighter   => { RenderMenuState::CreateFighter }
                MenuState::StartGame       => { RenderMenuState::StartGame }
            }
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

#[derive(Clone)]
pub enum MenuState {
    CharacterSelect,
    StageSelect,
    SetRules,
    SwitchPackages,
    BrowsePackages,
    CreatePackage,
    CreateFighter,
    StartGame,
}

pub enum RenderMenuState {
    CharacterSelect (Vec<CharacterSelect>),
    StageSelect     (usize),
    SetRules,
    SwitchPackages,
    BrowsePackages,
    CreatePackage,
    CreateFighter,
    StartGame,
}

#[derive(Clone)]
pub struct CharacterSelect {
    pub plugged_in: bool,
    pub selection:  Option<usize>,
    pub cursor:     usize,
}

pub struct RenderMenu {
    pub state: RenderMenuState,
}
