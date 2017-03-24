use ::input::{Input, PlayerInput};
use ::package::Package;
use ::graphics::{GraphicsMessage, Render};
use ::app::GameSetup;
use ::config::Config;
use ::fighter::Fighter;
use treeflection::ContextVec;

pub struct Menu {
    package:            Package,
    config:             Config,
    state:              MenuState,
    current_frame:      usize,
    fighter_selections: Vec<CharacterSelect>,
    stage_selection:    usize,
}

impl Menu {
    pub fn new(package: Package, config: Config) -> Menu {
        Menu {
            package:              package,
            config:               config,
            state:                MenuState::CharacterSelect,
            fighter_selections:   vec!(),
            stage_selection:      0,
            current_frame:        0,
        }
    }

    fn add_remove_fighter_selections(&mut self, player_inputs: &[PlayerInput]) {
        // HACK to populate fighter_selections, if not done so yet
        if self.fighter_selections.len() == 0 {
            for input in player_inputs {
                self.fighter_selections.push(CharacterSelect {
                    plugged_in:      input.plugged_in,
                    selection:       None,
                    cursor:          0,
                    ticker:          MenuTicker::new(),
                });
            }
        }

        // TODO: add/remove fighter_selections on input add/remove
    }

    fn step_fighter_select(&mut self, player_inputs: &[PlayerInput], input: &mut Input) {
        {
            // update selections
            let mut selections = &mut self.fighter_selections.iter_mut();
            let fighters = &self.package.fighters;
            for (ref mut selection, ref input) in selections.zip(player_inputs) {
                selection.plugged_in = input.plugged_in;

                if input.b.press {
                    selection.selection = None;
                }
                else if input.a.press {
                    if selection.cursor < fighters.len() {
                        selection.selection = Some(selection.cursor);
                    }
                    else {
                        // TODO: run extra options
                    }
                }

                if input[0].stick_y > 0.4 || input[0].up {
                    if selection.ticker.tick() {
                        if selection.cursor == 0 {
                            selection.cursor = Menu::fighter_select_cursor_max(fighters);
                        }
                        else {
                            selection.cursor -= 1;
                        }
                    }
                }
                else if input[0].stick_y < -0.4 || input[0].down {
                    if selection.ticker.tick() {
                        if selection.cursor == Menu::fighter_select_cursor_max(fighters) {
                            selection.cursor = 0;
                        }
                        else {
                            selection.cursor += 1;
                        }
                    }
                }
                else {
                    selection.ticker.reset();
                }
            }
        }

        if input.start_pressed() {
            self.state = MenuState::StageSelect;
        }
    }

    fn fighter_select_cursor_max(fighters: &ContextVec<Fighter>) -> usize {
        fighters.len() - 1 // last index of fighters
        + 0                // number of extra options
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
            // TODO: dumb hack to create config file, delete soon
            self.config.save();

            let mut selected_fighters: Vec<usize> = vec!();
            let mut controllers: Vec<usize> = vec!();
            for (i, selection) in (&self.fighter_selections).iter().enumerate() {
                if let Some(selection) = selection.selection {
                    selected_fighters.push(selection);
                    if player_inputs[i].plugged_in {
                        controllers.push(i);
                    }
                }
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

    pub fn reclaim(self) -> (Package, Config) {
        (self.package, self.config)
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
    pub plugged_in:      bool,
    pub selection:       Option<usize>,
    pub cursor:          usize,
    pub ticker:          MenuTicker,
}

#[derive(Clone)]
pub struct MenuTicker {
    ticks_remaining: usize,
    tick_duration_i: usize,
    reset:           bool,
}

impl MenuTicker {
    fn new() -> MenuTicker {
        MenuTicker {
            ticks_remaining: 0,
            tick_duration_i: 0,
            reset:           true,
        }
    }

    fn tick(&mut self) -> bool {
        let tick_durations = [20, 12, 10, 8, 6, 5];
        if self.reset {
            self.ticks_remaining = tick_durations[0];
            self.tick_duration_i = 0;
            self.reset = false;
            true
        }

        else {
            self.ticks_remaining -= 1;
            if self.ticks_remaining <= 0 {
                self.ticks_remaining = tick_durations[self.tick_duration_i];
                if self.tick_duration_i < tick_durations.len() - 1 {
                    self.tick_duration_i += 1;
                }
                true
            } else {
                false
            }
        }
    }

    fn reset(&mut self) {
        self.reset = true;
    }
}

pub struct RenderMenu {
    pub state: RenderMenuState,
}
