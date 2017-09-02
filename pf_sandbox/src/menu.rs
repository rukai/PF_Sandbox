use ::command_line::CommandLine;
use ::config::Config;
use ::game::{GameSetup, GameState};
use ::graphics::{GraphicsMessage, Render, RenderType};
use ::input::{Input, PlayerInput};
use ::package::{Package, PackageMeta, Verify};
use ::package;
use ::results::{GameResults, PlayerResult};
use ::replays;

use treeflection::{Node, NodeRunner, NodeToken};

use std::sync::mpsc::{Sender, Receiver, channel, TryRecvError};
use std::thread;

pub struct Menu {
    pub package:        PackageHolder,
    config:             Config,
    state:              MenuState,
    fighter_selections: Vec<CharacterSelect>,
    game_ticker:        MenuTicker,
    stage_ticker:       Option<MenuTicker>, // Uses an option because we dont know how many stages there are at Menu creation, but we want to remember which stage was selected
    current_frame:      usize,
    back_counter_max:   usize,
    game_setup:         Option<GameSetup>,
}

pub enum PackageHolder {
    Package (Package, Verify),
    None,
}

impl PackageHolder {
    fn new(package: Option<Package>, config: &Config) -> PackageHolder {
        if let Some(package) = package {
            if config.verify_package_hashes {
                let verify = package.verify();
                PackageHolder::Package(package, verify)
            } else {
                PackageHolder::Package(package, Verify::CannotConnect)
            }
        } else {
            PackageHolder::None
        }
    }

    fn get(&self) -> &Package {
        match self {
            &PackageHolder::Package (ref package, _) => { package }
            &PackageHolder::None                     => { panic!("Attempted to access the package while there was none") }
        }
    }

    fn verify(&self) -> Verify {
        match self {
            &PackageHolder::Package (_, ref verify) => { verify.clone() }
            &PackageHolder::None                    => { Verify::None }
        }
    }
}

impl Menu {
    pub fn new(package: Option<Package>, config: Config, state: MenuState) -> Menu {
        Menu {
            package:            PackageHolder::new(package, &config),
            config:             config,
            state:              state,
            fighter_selections: vec!(),
            stage_ticker:       None,
            game_ticker:        MenuTicker::new(4),
            current_frame:      0,
            back_counter_max:   90,
            game_setup:         None,
        }
    }

    pub fn step_game_select(&mut self, player_inputs: &[PlayerInput], input: &mut Input) {
        let ticker = &mut self.game_ticker;

        if player_inputs.iter().any(|x| x[0].stick_y > 0.4 || x[0].up) {
            ticker.up();
        }
        else if player_inputs.iter().any(|x| x[0].stick_y < -0.4 || x[0].down) {
            ticker.down();
        }
        else {
            ticker.reset();
        }

        if (input.start_pressed() || player_inputs.iter().any(|x| x.a.press)) && self.package.get().stages.len() > 0 {
            self.state = match ticker.cursor {
                0 => MenuState::character_select(),
                1 => MenuState::GameSelect,
                2 => MenuState::GameSelect,
                3 => MenuState::replay_select(self.package.get()),
                _ => unreachable!()
            }
        }
        else if player_inputs.iter().any(|x| x.b.press) {
            self.state = MenuState::package_select();
        }
    }

    pub fn step_replay_select(&mut self, player_inputs: &[PlayerInput], input: &mut Input) {
        let back = if let &mut MenuState::ReplaySelect (ref replays, ref mut ticker) = &mut self.state {
            if player_inputs.iter().any(|x| x[0].stick_y > 0.4 || x[0].up) {
                ticker.up();
            }
            else if player_inputs.iter().any(|x| x[0].stick_y < -0.4 || x[0].down) {
                ticker.down();
            }
            else {
                ticker.reset();
            }

            if (input.start_pressed() || player_inputs.iter().any(|x| x.a.press)) && replays.len() > 0 {
                let name = &replays[ticker.cursor];
                match replays::load_replay(name, self.package.get()) {
                    Ok(replay) => {
                        self.game_setup = Some(GameSetup {
                            init_seed:      replay.init_seed,
                            input_history:  replay.input_history,
                            player_history: replay.player_history,
                            stage_history:  replay.stage_history,
                            controllers:    replay.selected_controllers,
                            fighters:       replay.selected_fighters,
                            stage:          replay.selected_stage,
                            state:          GameState::ReplayForwards,
                        });
                    }
                    Err(error) => {
                        println!("Failed to load replay: {}\n{}", name, error);
                    }
                }
                false
            }
            else {
                player_inputs.iter().any(|x| x.b.press)
            }
        } else { unreachable!() };

        if back {
            self.state = MenuState::GameSelect;
        }
    }

    fn add_remove_fighter_selections(&mut self, player_inputs: &[PlayerInput]) {
        // HACK to populate fighter_selections, if not done so yet
        let cursor_max = self.fighter_select_cursor_max();
        if self.fighter_selections.len() == 0 {
            for input in player_inputs {
                self.fighter_selections.push(CharacterSelect {
                    plugged_in: input.plugged_in,
                    selection:  None,
                    ticker:     MenuTicker::new(cursor_max),
                });
            }
        }

        // TODO: add/remove fighter_selections on input add/remove
    }

    fn step_fighter_select(&mut self, player_inputs: &[PlayerInput], input: &mut Input) {
        self.add_remove_fighter_selections(&player_inputs);
        let mut new_state: Option<MenuState> = None;
        if let &mut MenuState::CharacterSelect (ref mut back_counter) = &mut self.state {
            let fighters = &self.package.get().fighters;
            {
                // update selections
                let selections = &mut self.fighter_selections.iter_mut();
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

            if input.start_pressed() && fighters.len() > 0 {
                new_state = Some(MenuState::StageSelect);
                if let None = self.stage_ticker {
                    self.stage_ticker = Some(MenuTicker::new(self.package.get().stages.len()));
                }
            }
            else if player_inputs.iter().any(|x| x[0].b) {
                if *back_counter > self.back_counter_max {
                    new_state = Some(MenuState::GameSelect);
                }
                else {
                    *back_counter += 1;
                }
            }
            else {
                *back_counter = 0;
            }
        }

        if let Some(state) = new_state {
            self.state = state;
        }
    }

    fn fighter_select_cursor_max(&self) -> usize {
        self.package.get().fighters.len() // last index of fighters
        + 0                               // number of extra options
    }

    fn step_stage_select(&mut self, player_inputs: &[PlayerInput], input: &mut Input) {
        if let None = self.stage_ticker {
            self.stage_ticker = Some(MenuTicker::new(self.package.get().stages.len()));
        }

        {
            let ticker = self.stage_ticker.as_mut().unwrap();

            if player_inputs.iter().any(|x| x[0].stick_y > 0.4 || x[0].up) {
                ticker.up();
            }
            else if player_inputs.iter().any(|x| x[0].stick_y < -0.4 || x[0].down) {
                ticker.down();
            }
            else {
                ticker.reset();
            }
        }

        if (input.start_pressed() || player_inputs.iter().any(|x| x.a.press)) && self.package.get().stages.len() > 0 {
            self.game_setup(player_inputs);
        }
        else if player_inputs.iter().any(|x| x.b.press) {
            self.state = MenuState::character_select();
        }
    }

    pub fn game_setup(&mut self, player_inputs: &[PlayerInput]) {
        let mut selected_fighters: Vec<String> = vec!();
        let mut controllers: Vec<usize> = vec!();
        for (i, selection) in (&self.fighter_selections).iter().enumerate() {
            if let Some(selection) = selection.selection {
                selected_fighters.push(self.package.get().fighters.index_to_key(selection).unwrap());
                if player_inputs[i].plugged_in {
                    controllers.push(i);
                }
            }
        }

        let stage = self.package.get().stages.index_to_key(self.stage_ticker.as_ref().unwrap().cursor).unwrap();

        self.game_setup = Some(GameSetup {
            init_seed:      GameSetup::gen_seed(),
            input_history:  vec!(),
            player_history: vec!(),
            stage_history:  vec!(),
            controllers:    controllers,
            fighters:       selected_fighters,
            stage:          stage,
            state:          GameState::Local,
        });
    }

    pub fn step_package_select(&mut self, player_inputs: &[PlayerInput], input: &mut Input) {
        let mut package = None;
        if let &mut MenuState::PackageSelect (ref package_metas, ref mut ticker, ref mut load) = &mut self.state {
            let update_selection = if let &mut Some((ref mut load_state, ref mut load_rx)) = load {
                loop {
                    match load_rx.try_recv() {
                        Ok (new_state) => {
                            if let PackageLoadState::Finished(new_package) = new_state {
                                package = Some(new_package);
                                break;
                            }
                            else {
                                *load_state = new_state;
                            }
                        }
                        Err (TryRecvError::Empty) => {
                            break;
                        }
                        Err (TryRecvError::Disconnected) => {
                            if let &mut PackageLoadState::Failed(_) = load_state { } else {
                                *load_state = PackageLoadState::Failed(String::from("tx was destroyed"));
                            }
                            break;
                        }
                    }
                }
                if let &mut PackageLoadState::Failed(_) = load_state {
                    true
                } else {
                    false
                }
            }
            else {
                true
            };

            if update_selection {
                Menu::step_package_select_inner(player_inputs, input, package_metas, ticker, load);
            }
        } else { unreachable!(); }

        if let Some(package) = package {
            self.set_package(package);
        }
    }

    fn set_package(&mut self, package: Package) {
        // setup for GameSelect
        self.package = PackageHolder::new(Some(package), &self.config);
        self.state = MenuState::GameSelect;
        self.fighter_selections = vec!();
        self.stage_ticker = None;

        // remember selection
        self.config.current_package = Some(self.package.get().meta.folder_name());
        self.config.save();
    }

    fn step_package_select_inner(
        player_inputs: &[PlayerInput],
        input: &mut Input,
        package_metas: &[(String, PackageMeta)],
        ticker: &mut MenuTicker,
        load: &mut Option<(PackageLoadState, Receiver<PackageLoadState>)>
    ) {
        if player_inputs.iter().any(|x| x[0].stick_y > 0.4 || x[0].up) {
            ticker.up();
        }
        else if player_inputs.iter().any(|x| x[0].stick_y < -0.4 || x[0].down) {
            ticker.down();
        }
        else {
            ticker.reset();
        }

        if package_metas.len() > 0 {
            if input.start_pressed() || player_inputs.iter().any(|x| x.a.press) {
                let meta = package_metas[ticker.cursor].1.clone();

                let (tx, rx) = channel();
                thread::spawn(move || Menu::load_package(meta, tx));
                *load = Some((PackageLoadState::Starting, rx));
            }
        }
    }

    // TODO: provide messages for:
    // *   downloading package x%
    // *   unzipping package
    // *   writing package
    fn load_package(meta: PackageMeta, tx: Sender<PackageLoadState>) {
        tx.send(PackageLoadState::Downloading).unwrap();
        meta.update();

        tx.send(PackageLoadState::Loading).unwrap();
        match meta.load() {
            Ok (package) => tx.send(PackageLoadState::Finished(package)).unwrap(),
            Err (err)    => tx.send(PackageLoadState::Failed(err)).unwrap()
        };
    }

    fn step_results(&mut self, player_inputs: &[PlayerInput], input: &mut Input) {
        if input.start_pressed() || player_inputs.iter().any(|x| x.a.press) {
            self.state = MenuState::character_select();
        }

        if let &mut MenuState::GameResults { ref results, ref mut replay_saved } = &mut self.state {
            if !*replay_saved {
                if self.config.auto_save_replay || player_inputs.iter().any(|x| x.l.press && x.r.press) {
                    replays::save_replay(&results.replay, self.package.get());
                    *replay_saved = true;
                }
            }
        }
    }

    pub fn step(&mut self, input: &mut Input) -> Option<GameSetup> {
        if let &PackageHolder::Package (ref package, _) = &self.package {
            if package.has_updates() {
                self.fighter_selections = vec!();
                self.stage_ticker = None;
            }
        }
        input.game_update(self.current_frame);
        let player_inputs = input.players(self.current_frame);

        match self.state {
            MenuState::GameSelect             => { self.step_game_select   (&player_inputs, input) }
            MenuState::ReplaySelect (_, _)    => { self.step_replay_select (&player_inputs, input) }
            MenuState::PackageSelect (_, _,_) => { self.step_package_select(&player_inputs, input) }
            MenuState::CharacterSelect (_)    => { self.step_fighter_select(&player_inputs, input) }
            MenuState::StageSelect            => { self.step_stage_select  (&player_inputs, input) }
            MenuState::GameResults {..}       => { self.step_results       (&player_inputs, input) }
            MenuState::SetRules               => { }
            MenuState::BrowsePackages         => { }
            MenuState::CreatePackage          => { }
            MenuState::CreateFighter          => { }
        };

        self.current_frame += 1;
        self.game_setup.clone()
    }

    pub fn render(&self) -> RenderMenu {
        RenderMenu {
            state: match self.state {
                MenuState::PackageSelect (ref names, ref ticker, ref load) => { RenderMenuState::PackageSelect (names.iter().map(|x| x.1.title.clone()).collect(), ticker.cursor, load.as_ref().map(|x| x.0.message()).unwrap_or_default() ) }
                MenuState::GameResults {ref results, replay_saved} => { RenderMenuState::GameResults { results: results.player_results.clone(), replay_saved } }
                MenuState::CharacterSelect (back_counter)          => { RenderMenuState::CharacterSelect (self.fighter_selections.clone(), back_counter, self.back_counter_max) }
                MenuState::ReplaySelect (ref replays, ref ticker)  => { RenderMenuState::ReplaySelect (replays.clone(), ticker.cursor) }
                MenuState::GameSelect     => { RenderMenuState::GameSelect (self.game_ticker.cursor) }
                MenuState::StageSelect    => { RenderMenuState::StageSelect (self.stage_ticker.as_ref().unwrap().cursor) }
                MenuState::SetRules       => { RenderMenuState::SetRules }
                MenuState::BrowsePackages => { RenderMenuState::BrowsePackages }
                MenuState::CreatePackage  => { RenderMenuState::CreatePackage }
                MenuState::CreateFighter  => { RenderMenuState::CreateFighter }
            },
            package_verify: self.package.verify(),
        }
    }

    pub fn graphics_message(&mut self, command_line: &CommandLine) -> GraphicsMessage {
        let updates = match &mut self.package {
            &mut PackageHolder::Package (ref mut package, _) => {
                package.updates()
            }
            &mut PackageHolder::None => {
                vec!()
            }
        };

        let render = Render {
            command_output:  command_line.output(),
            render_type:     RenderType::Menu (self.render()),
        };

        GraphicsMessage {
            package_updates: updates,
            render:          render,
        }
    }

    pub fn reclaim(self) -> (Package, Config) {
        match self.package {
            PackageHolder::Package (package, _) => { (package, self.config) }
            PackageHolder::None                 => { panic!("Attempted to access the package while there was none") }
        }
    }
}

impl Node for Menu {
    fn node_step(&mut self, mut runner: NodeRunner) -> String {
        let result = match runner.step() {
            NodeToken::ChainProperty (property) => {
                match property.as_str() {
                    "package" => {
                        if let &mut PackageHolder::Package (ref mut package, _) = &mut self.package {
                            package.node_step(runner)
                        } else {
                            String::from("No package is loaded.")
                        }
                    }
                    prop      => format!("Menu does not have a property '{}'", prop)
                }
            }
            NodeToken::Help => {
                String::from(r#"
Menu Help

Commands:
*   help               - display this help
*   open_package $name - loads the package with the given folder name, if it doesnt exist it is created.

Accessors:
*   .package - Package"#)
            }
            NodeToken::Custom (action, args) => {
                match action.as_ref() {
                    "open_package" => {
                        if args.len() > 0 {
                            let package_name = &args[0];
                            match Package::open_or_generate(package_name) {
                                Some (package) => {
                                    self.set_package(package);
                                    format!("Successfully opened or created package {}", package_name)
                                }
                                None => {
                                    format!("Failed to open package: {}", package_name)
                                }
                            }
                        } else {
                            format!("Didn't specify a package")
                        }
                    }
                    _ => {
                        format!("Menu cannot '{}'", action)
                    }
                }
            }
            action => { format!("Menu cannot '{:?}'", action) }
        };
        result
    }
}

pub enum MenuState {
    GameSelect,
    ReplaySelect (Vec<String>, MenuTicker),
    CharacterSelect (usize), // TODO: name usize value as backcounter
    StageSelect,
    GameResults { results: GameResults, replay_saved: bool },
    SetRules,
    PackageSelect (Vec<(String, PackageMeta)>, MenuTicker, Option<(PackageLoadState, Receiver<PackageLoadState>)>),
    BrowsePackages,
    CreatePackage,
    CreateFighter,
}

pub enum PackageLoadState {
    Starting,
    Downloading,
    Unzipping,
    Updating,
    Writing,
    Loading,
    Finished (Package),
    Failed (String),
}

impl PackageLoadState {
    pub fn message(&self) -> String {
        match self {
            &PackageLoadState::Starting     => format!(""),
            &PackageLoadState::Downloading  => format!("Downloading package"),
            &PackageLoadState::Unzipping    => format!("Unzipping package"),
            &PackageLoadState::Updating     => format!("Updating package"),
            &PackageLoadState::Writing      => format!("Writing package"),
            &PackageLoadState::Loading      => format!("Loading package"),
            &PackageLoadState::Finished (_) => format!("Package ready"),
            &PackageLoadState::Failed (ref message) => message.clone()
        }
    }
}

impl MenuState {
    pub fn package_select() -> MenuState {
        let packages = package::get_package_metas();
        let ticker = MenuTicker::new(packages.len());
        MenuState::PackageSelect(packages, ticker, None)
    }

    pub fn replay_select(package: &Package) -> MenuState {
        let replays = replays::get_replay_names(package);
        let ticker = MenuTicker::new(replays.len());
        MenuState::ReplaySelect (replays, ticker)
    }

    pub fn character_select() -> MenuState {
        MenuState::CharacterSelect(0)
    }

    pub fn game_results(results: GameResults) -> MenuState {
        MenuState::GameResults { results, replay_saved: false }
    }
}

pub enum RenderMenuState {
    GameSelect      (usize),
    ReplaySelect    (Vec<String>, usize),
    CharacterSelect (Vec<CharacterSelect>, usize, usize),
    StageSelect     (usize),
    GameResults     { results: Vec<PlayerResult>, replay_saved: bool },
    SetRules,
    PackageSelect   (Vec<String>, usize, String),
    BrowsePackages,
    CreatePackage,
    CreateFighter,
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
    fn new(item_count: usize) -> MenuTicker {
        MenuTicker {
            cursor:          0,
            cursor_max:      if item_count > 0 { item_count - 1 } else { 0 },
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
    pub state:          RenderMenuState,
    pub package_verify: Verify,
}
