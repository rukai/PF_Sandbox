use ::input::{Input, PlayerInput};
use ::package::Package;
use ::graphics::{GraphicsMessage, Render};
use ::app::GameSetup;
use ::config::Config;
use ::records::GameResult;

pub struct Menu {
    package:              Package,
    config:               Config,
    state:                MenuState,
    current_frame:        usize,
    fighter_selections:   Vec<CharacterSelect>,
    stage_ticker:         MenuTicker,
}

impl Menu {
    pub fn new(package: Package, config: Config, state: MenuState) -> Menu {
        Menu {
            config:               config,
            state:                state,
            fighter_selections:   vec!(),
            stage_ticker:         MenuTicker::new(package.stages.len() - 1),
            package:              package,
            current_frame:        0,
        }
    }

    fn add_remove_fighter_selections(&mut self, player_inputs: &[PlayerInput]) {
        // HACK to populate fighter_selections, if not done so yet
        let cursor_max = self.fighter_select_cursor_max();
        if self.fighter_selections.len() == 0 {
            for input in player_inputs {
                self.fighter_selections.push(CharacterSelect {
                    plugged_in:      input.plugged_in,
                    selection:       None,
                    ticker:          MenuTicker::new(cursor_max),
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
                    if selection.ticker.cursor < fighters.len() {
                        selection.selection = Some(selection.ticker.cursor);
                    }
                    else {
                        // TODO: run extra options
                    }
                }

                if input[0].stick_y > 0.4 || input[0].up {
                    selection.ticker.up();
                }
                else if input[0].stick_y < -0.4 || input[0].down {
                    selection.ticker.down();
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

    fn fighter_select_cursor_max(&self) -> usize {
        self.package.fighters.len() - 1 // last index of fighters
        + 0                // number of extra options
    }

    fn step_stage_select(&mut self, player_inputs: &[PlayerInput], input: &mut Input) {
        if player_inputs.iter().any(|x| x[0].stick_y > 0.4 || x[0].up) {
            self.stage_ticker.up();
        }
        else if player_inputs.iter().any(|x| x[0].stick_y < -0.4 || x[0].down) {
            self.stage_ticker.down();
        }
        else {
            self.stage_ticker.reset();
        }

        if input.start_pressed() || player_inputs.iter().any(|x| x.a.press) {
            self.state = MenuState::StartGame;
        }
        else if player_inputs.iter().any(|x| x.b.press) {
            self.state = MenuState::CharacterSelect;
        }
    }

    fn step_results(&mut self, player_inputs: &[PlayerInput], input: &mut Input) {
        if input.start_pressed() || player_inputs.iter().any(|x| x.a.press) {
            self.state = MenuState::CharacterSelect;
        }
    }

    pub fn step(&mut self, input: &mut Input) -> Option<GameSetup> {
        input.game_update(self.current_frame);
        let player_inputs = input.players(self.current_frame);

        self.add_remove_fighter_selections(&player_inputs);

        match self.state {
            MenuState::CharacterSelect => { self.step_fighter_select(&player_inputs, input) }
            MenuState::StageSelect     => { self.step_stage_select  (&player_inputs, input) }
            MenuState::GameResults (_) => { self.step_results       (&player_inputs, input) }
            MenuState::SetRules        => { }
            MenuState::SwitchPackages  => { }
            MenuState::BrowsePackages  => { }
            MenuState::CreatePackage   => { }
            MenuState::CreateFighter   => { }
            MenuState::StartGame       => { }
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
                stage:       self.stage_ticker.cursor,
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
                MenuState::GameResults (ref results) => { RenderMenuState::GameResults (results.clone()) }
                MenuState::CharacterSelect => { RenderMenuState::CharacterSelect (self.fighter_selections.clone()) }
                MenuState::StageSelect     => { RenderMenuState::StageSelect     (self.stage_ticker.cursor) }
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
    GameResults (Vec<GameResult>),
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
    GameResults     (Vec<GameResult>),
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
    pub ticker:     MenuTicker,
}

#[derive(Clone)]
pub struct MenuTicker {
    pub cursor:      usize,
    cursor_max:      usize,
    ticks_remaining: usize,
    tick_duration_i: usize,
    reset:           bool,
}

impl MenuTicker {
    fn new(cursor_max: usize) -> MenuTicker {
        MenuTicker {
            cursor:          0,
            cursor_max:      cursor_max,
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

    fn up(&mut self) {
        if self.tick() {
            if self.cursor == 0 {
                self.cursor = self.cursor_max;
            }
            else {
                self.cursor -= 1;
            }
        }
    }

    fn down(&mut self) {
        if self.tick() {
            if self.cursor == self.cursor_max {
                self.cursor = 0;
            }
            else {
                self.cursor += 1;
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
