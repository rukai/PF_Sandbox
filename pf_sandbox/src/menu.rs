use command_line::CommandLine;
use config::Config;
use game::{GameSetup, GameState, PlayerSetup};
use graphics::{GraphicsMessage, Render, RenderType};
use graphics;
use input::{Input, PlayerInput};
use network::{Netplay, NetplayState};
use os_input::OsInput;
use package::{Package, PackageMeta, Verify};
use package;
use replays;
use results::{GameResults, PlayerResult};

use treeflection::{Node, NodeRunner, NodeToken};
use winit::VirtualKeyCode;

use std::sync::mpsc::{Sender, Receiver, channel, TryRecvError};
use std::thread;
use std::mem;

/// For player convenience some data is kept when moving between menus.
/// This data is stored in the Menu struct.
///
/// Because it should be refreshed (sourced from filesystem)
///     or is no longer valid (e.g. back_counter) some data is thrown away when moving between menus.
///     or takes up too much space when copied for netplay e.g. game_results
/// This data is kept in the MenuState variants.

pub struct Menu {
    pub package:        PackageHolder,
    config:             Config,
    state:              MenuState,
    prev_state:         Option<MenuState>, // Only populated when the current state specifically needs to jump back to the previous state i.e we could arrive at the current state via multiple sources.
    fighter_selections: Vec<PlayerSelect>,
    game_ticker:        MenuTicker,
    stage_ticker:       Option<MenuTicker>, // Uses an option because we dont know how many stages there are at Menu creation, but we want to remember which stage was selected
    current_frame:      usize,
    back_counter_max:   usize,
    game_setup:         Option<GameSetup>,
    package_loader:     Option<PackageLoader>,
    game_results:       Option<GameResults>,
    netplay_history:    Vec<NetplayHistory>,
}

pub struct NetplayHistory {
    state:              MenuState,
    prev_state:         Option<MenuState>,
    fighter_selections: Vec<PlayerSelect>,
    stage_ticker:       Option<MenuTicker>,
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
            prev_state:         None,
            fighter_selections: vec!(),
            stage_ticker:       None,
            game_ticker:        MenuTicker::new(3),
            current_frame:      0,
            back_counter_max:   90,
            game_setup:         None,
            package_loader:     None,
            game_results:       None,
            netplay_history:    vec!(),
        }
    }

    pub fn resume(&mut self, package: Package, config: Config, resume_menu: ResumeMenu) {
        self.current_frame = 0;
        self.package = PackageHolder::new(Some(package), &config);
        self.config = config;
        match resume_menu {
            ResumeMenu::NetplayDisconnect { reason: message } => {
                self.state = MenuState::NetplayWait { message };
            }
            ResumeMenu::Results (results) => {
                self.game_results = Some(results);
                self.prev_state = Some(mem::replace(&mut self.state, MenuState::game_results()));
            }
            ResumeMenu::Unchanged => { }
        }
    }

    pub fn step_game_select(&mut self, player_inputs: &[PlayerInput], netplay: &mut Netplay) {
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

        if (player_inputs.iter().any(|x| x.a.press || x.start.press)) && self.package.get().stages.len() > 0 {
            match ticker.cursor {
                0 => {
                    self.state = MenuState::character_select()
                }
                1 => {
                    netplay.connect_match_making(
                        self.config.netplay_region.clone().unwrap_or(String::from("AU")), // TODO: set region screen if region.is_none()
                        2,
                        self.package.get().compute_hash()
                    );
                    self.state = MenuState::NetplayWait { message: String::from("") };
                }
                2 => {
                    self.state = MenuState::replay_select(self.package.get());
                }
                _ => unreachable!()
            }
        }
        else if player_inputs.iter().any(|x| x.b.press) {
            self.package_loader = None;
            self.state = MenuState::package_select();
        }
    }

    pub fn step_replay_select(&mut self, player_inputs: &[PlayerInput]) {
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

            if (player_inputs.iter().any(|x| x.start.press || x.a.press)) && replays.len() > 0 {
                let name = &replays[ticker.cursor];
                match replays::load_replay(name, self.package.get()) {
                    Ok(replay) => {
                        self.game_setup = Some(GameSetup {
                            init_seed:      replay.init_seed,
                            input_history:  replay.input_history,
                            player_history: replay.player_history,
                            stage_history:  replay.stage_history,
                            controllers:    replay.selected_controllers,
                            players:        replay.selected_players,
                            ais:            replay.selected_ais,
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

    /// If controllers are added or removed then the indexes
    /// are going be out of whack so just reset the fighter selection state
    /// If a controller is added on the same frame another is removed, then no reset occurs.
    /// However this is rare and the problem is minor, so ¯\_(ツ)_/¯
    fn add_remove_fighter_selections(&mut self, player_inputs: &[PlayerInput]) {
        if self.fighter_selections.iter().filter(|x| !x.ui.is_cpu()).count() != player_inputs.len() {
            self.fighter_selections.clear();
            for (i, input) in player_inputs.iter().enumerate() {
                let ui = if input.plugged_in {
                    PlayerSelectUi::human_fighter(self.package.get())
                } else {
                    PlayerSelectUi::HumanUnplugged
                };
                let team = Menu::get_free_team(&self.fighter_selections);
                self.fighter_selections.push(PlayerSelect {
                    controller:      Some((i, MenuTicker::new(1))),
                    fighter:         None,
                    cpu_ai:          None,
                    ui:              ui,
                    animation_frame: 0,
                    team
                });
            }
        }
    }

    fn step_fighter_select(&mut self, player_inputs: &[PlayerInput], netplay: &mut Netplay) {
        self.add_remove_fighter_selections(&player_inputs);
        let mut new_state: Option<MenuState> = None;
        if let &mut MenuState::CharacterSelect { ref mut back_counter } = &mut self.state {
            let fighters = &self.package.get().fighters;

            // animate fighters
            for selection in self.fighter_selections.iter_mut() {
                if let Some(fighter_key) = selection.fighter {
                    let fighter = &self.package.get().fighters[fighter_key];
                    if let Some(action) = fighter.actions.get(fighter.css_action as usize) {
                        selection.animation_frame = (selection.animation_frame + 1) % action.frames.len();
                    }
                }
            }

            // plug/unplug humans
            for (input_i, input) in player_inputs.iter().enumerate() {
                let free_team = Menu::get_free_team(&self.fighter_selections);
                if input.plugged_in {
                    let selection = &mut self.fighter_selections[input_i];
                    if let PlayerSelectUi::HumanUnplugged = selection.ui {
                        selection.ui = PlayerSelectUi::human_fighter(self.package.get());
                        selection.team = free_team;
                        selection.controller = Some((input_i, MenuTicker::new(1)));
                    }
                }
                else {
                    if let PlayerSelectUi::HumanFighter (_) = self.fighter_selections[input_i].ui {
                        self.fighter_selections[input_i].ui = PlayerSelectUi::HumanUnplugged;

                        // Handle CPU's who are currently manipulated by the input
                        for selection in &mut self.fighter_selections {
                            if let Some((controller, _)) = selection.controller.clone() {
                                if controller == input_i {
                                    selection.controller = None
                                }
                            }
                        }
                    }
                }
            }

            for (controller_i, ref input) in player_inputs.iter().enumerate() {
                if !input.plugged_in {
                    continue;
                }

                // get current selection
                let mut selection_i = 0;
                for (check_selection_i, selection) in self.fighter_selections.iter().enumerate() {
                    if let Some((check_controller_i, _)) = selection.controller {
                        if check_controller_i == controller_i {
                            selection_i = check_selection_i;
                        }
                    }
                }

                // move left/right
                if input[0].stick_x < -0.7 || input[0].left {
                    if self.fighter_selections[selection_i].controller.as_mut().unwrap().1.tick() {
                        // find prev selection to move to
                        let mut new_selection_i: Option<usize> = None;
                        for (check_selection_i, selection) in self.fighter_selections.iter().enumerate() {
                            if check_selection_i > selection_i && (selection.is_free() || check_selection_i == controller_i) {
                                new_selection_i = Some(check_selection_i);
                            }
                        }
                        for (check_selection_i, selection) in self.fighter_selections.iter().enumerate() {
                            if check_selection_i < selection_i && (selection.is_free() || check_selection_i == controller_i) {
                                new_selection_i = Some(check_selection_i);
                            }
                        }

                        // move selection
                        if let Some(new_selection_i) = new_selection_i {
                            self.fighter_selections[new_selection_i].controller = self.fighter_selections[selection_i].controller.clone();
                            self.fighter_selections[selection_i].controller = None;
                            self.fighter_selections[selection_i].ui.ticker_full_reset();
                        }
                    }
                }
                else if input[0].stick_x > 0.7 || input[0].right {
                    if self.fighter_selections[selection_i].controller.as_mut().unwrap().1.tick() {
                        // find next selection to move to
                        let mut new_selection_i: Option<usize> = None;
                        for (check_selection_i, selection) in self.fighter_selections.iter().enumerate().rev() {
                            if check_selection_i < selection_i && (selection.is_free() || check_selection_i == controller_i) {
                                new_selection_i = Some(check_selection_i);
                            }
                        }
                        for (check_selection_i, selection) in self.fighter_selections.iter().enumerate().rev() {
                            if check_selection_i > selection_i && (selection.is_free() || check_selection_i == controller_i) {
                                new_selection_i = Some(check_selection_i);
                            }
                        }

                        // move selection
                        if let Some(new_selection_i) = new_selection_i {
                            self.fighter_selections[new_selection_i].controller = self.fighter_selections[selection_i].controller.clone();
                            self.fighter_selections[selection_i].controller = None;
                            self.fighter_selections[selection_i].ui.ticker_full_reset();
                        }
                    }
                }
                else {
                    self.fighter_selections[selection_i].controller.as_mut().unwrap().1.reset();
                }
            }

            // update selections
            let mut add_cpu = false;
            let mut remove_cpu: Option<usize> = None;

            for (selection_i, selection) in self.fighter_selections.iter_mut().enumerate() {
                if let Some((controller, _)) = selection.controller {
                    let input = &player_inputs[controller];
                    if input.b.press {
                        match selection.ui.clone() {
                            PlayerSelectUi::HumanFighter (_) |
                            PlayerSelectUi::CpuFighter (_) => {
                                selection.fighter = None;
                            }
                            PlayerSelectUi::HumanTeam (_) => {
                                selection.ui = PlayerSelectUi::human_fighter(self.package.get());
                            }
                            PlayerSelectUi::CpuTeam (_) |
                            PlayerSelectUi::CpuAi (_) => {
                                selection.ui = PlayerSelectUi::cpu_fighter(self.package.get());
                                selection.ui = PlayerSelectUi::cpu_fighter(self.package.get());
                            }
                            PlayerSelectUi::HumanUnplugged => unreachable!(),
                        }
                    }
                    else if input.a.press {
                        match selection.ui.clone() {
                            PlayerSelectUi::HumanFighter (ticker) => {
                                if ticker.cursor < fighters.len() {
                                    selection.fighter = Some(ticker.cursor);
                                }
                                else {
                                    match ticker.cursor - fighters.len() {
                                        0 => { selection.ui = PlayerSelectUi::human_team() }
                                        1 => { add_cpu = true; }
                                        _ => { unreachable!() }
                                    }
                                }
                            }
                            PlayerSelectUi::CpuFighter (ticker) => {
                                if ticker.cursor < fighters.len() {
                                    selection.fighter = Some(ticker.cursor);
                                }
                                else {
                                    match ticker.cursor - fighters.len() {
                                        0 => { selection.ui = PlayerSelectUi::cpu_team() }
                                        1 => { /* TODO: selection.ui = PlayerSelectUi::cpu_ai()*/ }
                                        2 => { remove_cpu = Some(selection_i); }
                                        _ => { unreachable!() }
                                    }
                                }
                            }
                            PlayerSelectUi::HumanTeam (ticker) => {
                                let colors = graphics::get_colors();
                                if ticker.cursor < colors.len() {
                                    selection.team = ticker.cursor;
                                } else {
                                    match ticker.cursor - colors.len() {
                                        0 => { selection.ui = PlayerSelectUi::human_fighter(self.package.get()) }
                                        _ => { unreachable!() }
                                    }
                                }
                            }
                            PlayerSelectUi::CpuTeam (ticker) => {
                                let colors = graphics::get_colors();
                                if ticker.cursor < colors.len() {
                                    selection.team = ticker.cursor;
                                } else {
                                    match ticker.cursor - colors.len() {
                                        0 => { selection.ui = PlayerSelectUi::cpu_fighter(self.package.get()) }
                                        _ => { unreachable!() }
                                    }
                                }
                            }
                            PlayerSelectUi::CpuAi (_) => { }
                            PlayerSelectUi::HumanUnplugged => unreachable!(),
                        }
                    }

                    match selection.ui {
                        PlayerSelectUi::HumanFighter (ref mut ticker) |
                        PlayerSelectUi::CpuFighter   (ref mut ticker) |
                        PlayerSelectUi::HumanTeam    (ref mut ticker) |
                        PlayerSelectUi::CpuTeam      (ref mut ticker) |
                        PlayerSelectUi::CpuAi        (ref mut ticker) => {
                            if input[0].stick_y > 0.4 || input[0].up {
                                ticker.up();
                            }
                            else if input[0].stick_y < -0.4 || input[0].down {
                                ticker.down();
                            }
                            else {
                                ticker.reset();
                            }
                        }
                        PlayerSelectUi::HumanUnplugged => { }
                    }
                }
            }

            // run selection modifications that were previously immutably borrowed
            if let Some(selection_i) = remove_cpu {
                let home_selection_i = self.fighter_selections[selection_i].controller.clone().unwrap().0;
                self.fighter_selections[home_selection_i].controller = self.fighter_selections[selection_i].controller.clone();
                self.fighter_selections.remove(selection_i);
            }

            if add_cpu {
                if self.fighter_selections.iter().filter(|x| x.ui.is_visible()).count() < 4 {
                    let team = Menu::get_free_team(&self.fighter_selections);
                    self.fighter_selections.push(PlayerSelect {
                        controller:      None,
                        fighter:         None,
                        cpu_ai:          None,
                        ui:              PlayerSelectUi::cpu_fighter(self.package.get()),
                        animation_frame: 0,
                        team
                    });
                }
            }

            if player_inputs.iter().any(|x| x.start.press) && fighters.len() > 0 {
                new_state = Some(MenuState::StageSelect);
                if let None = self.stage_ticker {
                    self.stage_ticker = Some(MenuTicker::new(self.package.get().stages.len()));
                }
            }
            else if player_inputs.iter().any(|x| x[0].b) {
                if *back_counter > self.back_counter_max {
                    netplay.set_offline();
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

    fn get_free_team(selections: &[PlayerSelect]) -> usize {
        let mut team = 0;
        while selections.iter().any(|x| x.ui.is_visible() && x.team == team) {
            team += 1;
        }
        team
    }

    fn step_stage_select(&mut self, player_inputs: &[PlayerInput], netplay: &Netplay) {
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

        if (player_inputs.iter().any(|x| x.start.press || x.a.press)) && self.package.get().stages.len() > 0 {
            self.game_setup(netplay);
        }
        else if player_inputs.iter().any(|x| x.b.press) {
            self.state = MenuState::character_select();
        }
    }

    pub fn game_setup(&mut self, netplay: &Netplay) {
        let mut players: Vec<PlayerSetup> = vec!();
        let mut controllers: Vec<usize> = vec!();
        let mut ais: Vec<usize> = vec!();
        let mut ais_skipped = 0;
        for (i, selection) in (&self.fighter_selections).iter().enumerate() {
            // add human players
            if selection.ui.is_human_plugged_in() {
                if let Some(fighter) = selection.fighter {
                    players.push(PlayerSetup {
                        fighter: self.package.get().fighters.index_to_key(fighter).unwrap(),
                        team:    selection.team,
                    });
                    controllers.push(i);
                }
            }

            // add CPU players
            if selection.ui.is_cpu() {
                if selection.fighter.is_some() /* && selection.cpu.is_some() TODO */ {
                    let fighter = selection.fighter.unwrap();
                    players.push(PlayerSetup {
                        fighter: self.package.get().fighters.index_to_key(fighter).unwrap(),
                        team:    selection.team,
                    });
                    controllers.push(i - ais_skipped);
                    ais.push(0); // TODO: delete this
                    // ais.push(selection.cpu_ai.unwrap()); TODO: add this
                }
                else {
                    ais_skipped += 1;
                }
            }
        }

        let stage = self.package.get().stages.index_to_key(self.stage_ticker.as_ref().unwrap().cursor).unwrap();
        let state = if netplay.number_of_peers() == 1 { GameState::Local } else { GameState::Netplay };
        let init_seed = netplay.get_seed().unwrap_or(GameSetup::gen_seed());

        self.game_setup = Some(GameSetup {
            input_history:  vec!(),
            player_history: vec!(),
            stage_history:  vec!(),
            init_seed,
            controllers,
            ais,
            players,
            stage,
            state,
        });
    }

    pub fn step_package_select(&mut self, player_inputs: &[PlayerInput]) {
        let mut package = None;
        if let &mut MenuState::PackageSelect (ref package_metas, ref mut ticker) = &mut self.state {
            let update_selection = if let &mut Some(ref mut loader) = &mut self.package_loader {
                loop {
                    match loader.rx.try_recv() {
                        Ok (new_state) => {
                            if let PackageLoadState::Finished(new_package) = new_state {
                                package = Some(new_package);
                                break;
                            }
                            else {
                                loader.state = new_state;
                            }
                        }
                        Err (TryRecvError::Empty) => {
                            break;
                        }
                        Err (TryRecvError::Disconnected) => {
                            if let PackageLoadState::Failed(_) = loader.state.clone() { } else {
                                loader.state = PackageLoadState::Failed(String::from("tx was destroyed"));
                            }
                            break;
                        }
                    }
                }
                if let PackageLoadState::Failed(_) = loader.state.clone() {
                    true
                } else {
                    false
                }
            }
            else {
                true
            };

            if update_selection {
                Menu::step_package_select_inner(&mut self.package_loader, player_inputs, package_metas, ticker);
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
        package_loader: &mut Option<PackageLoader>,
        player_inputs: &[PlayerInput],
        package_metas: &[(String, PackageMeta)],
        ticker: &mut MenuTicker,
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
            if player_inputs.iter().any(|x| x.start.press || x.a.press) {
                let meta = package_metas[ticker.cursor].1.clone();

                let (tx, rx) = channel();
                thread::spawn(move || Menu::load_package(meta, tx));
                *package_loader = Some(PackageLoader {
                    state: PackageLoadState::Starting,
                    rx
                });
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

    fn step_results(&mut self, player_inputs: &[PlayerInput]) {
        if player_inputs.iter().any(|x| x.start.press || x.a.press) {
            self.state = self.prev_state.take().unwrap();
        }

        // TODO:
        // Make the following changes so this state is managed locally (one peer saving a replay does not cause the other to save a replay)
        // *    Run it in a seperate thread so main thread does not halt
        // *    Dont use remote peers inputs
        // *    move replay_saved into its own non-rollbacked state
        if let &mut MenuState::GameResults { ref mut replay_saved, .. } = &mut self.state {
            if !*replay_saved {
                if self.config.auto_save_replay || player_inputs.iter().any(|x| x.l.press && x.r.press) {
                    replays::save_replay(&self.game_results.as_ref().unwrap().replay, self.package.get());
                    *replay_saved = true;
                }
            }
        }
    }

    fn step_netplay_wait(&mut self, player_inputs: &[PlayerInput], netplay: &mut Netplay) {
        if player_inputs.iter().any(|x| x.b.press) {
            self.state = MenuState::GameSelect;
        }

        let loading_characters = ["|", "/", "-", "\\"];
        let load_character = loading_characters[(self.current_frame / 5) % loading_characters.len()];

        match netplay.state() {
            NetplayState::Offline => { }
            NetplayState::MatchMaking { request, .. } => {
                self.state = MenuState::NetplayWait { message: format!("Searching for online match in {} {}", request.region, load_character) };
                if player_inputs.iter().any(|x| x.b.press) {
                    netplay.set_offline();
                    self.state = MenuState::GameSelect;
                }
            }
            NetplayState::InitConnection {..} => {
                self.state = MenuState::NetplayWait { message: format!("Connecting to peer {}", load_character) };
                if player_inputs.iter().any(|x| x.b.press) {
                    netplay.set_offline();
                    self.state = MenuState::GameSelect;
                }
            }
            NetplayState::PingTest { .. } => {
                self.state = MenuState::NetplayWait { message: format!("Testing ping {}", load_character) };
                if player_inputs.iter().any(|x| x.b.press) {
                    netplay.set_offline();
                    self.state = MenuState::GameSelect;
                }
            }
            NetplayState::Disconnected { .. } => {
                if player_inputs.iter().any(|x| x.a.press || x.b.press) {
                    netplay.set_offline();
                    self.state = MenuState::GameSelect;
                }
            }
            NetplayState::Running { .. } => {
                self.state = MenuState::character_select();
            }
        }
    }

    pub fn step(&mut self, input: &mut Input, os_input: &OsInput, netplay: &mut Netplay) -> Option<GameSetup> {
        if os_input.held_alt() && os_input.key_pressed(VirtualKeyCode::Return) {
            self.config.fullscreen = !self.config.fullscreen;
            self.config.save();
        }

        if let &PackageHolder::Package (ref package, _) = &self.package {
            if package.has_updates() {
                self.fighter_selections = vec!();
                self.stage_ticker = None;
            }
        }

        if let Some(path) = os_input.dropped_file() {
            package::extract_from_path(path);
            self.package_loader = None;
            self.state = MenuState::package_select();
            netplay.set_offline();
        }

        // skip a frame so the other clients can catch up.
        if !netplay.skip_frame() {
            self.current_frame += 1;

            let start = self.current_frame - netplay.frames_to_step();
            let end = self.current_frame;

            self.netplay_history.truncate(start);
            if start > 0 {
                let history = self.netplay_history.get(start-1).unwrap();
                self.state              = history.state.clone();
                self.prev_state         = history.prev_state.clone();
                self.fighter_selections = history.fighter_selections.clone();
                self.stage_ticker       = history.stage_ticker.clone();
            }

            input.netplay_update();

            for frame in start..end {
                if let NetplayState::Disconnected { reason } = netplay.state() {
                    self.state = MenuState::NetplayWait { message: reason };
                }

                let player_inputs = input.players(frame, netplay);

                // In order to avoid hitting buttons still held down from the game, dont do anything on the first frame.
                if frame > 1 {
                    match self.state {
                        MenuState::GameSelect           => self.step_game_select   (&player_inputs, netplay),
                        MenuState::ReplaySelect (_, _)  => self.step_replay_select (&player_inputs),
                        MenuState::PackageSelect (_, _) => self.step_package_select(&player_inputs),
                        MenuState::CharacterSelect {..} => self.step_fighter_select(&player_inputs, netplay),
                        MenuState::StageSelect          => self.step_stage_select  (&player_inputs, netplay),
                        MenuState::GameResults {..}     => self.step_results       (&player_inputs),
                        MenuState::NetplayWait {..}     => self.step_netplay_wait  (&player_inputs, netplay),
                    };
                }

                self.netplay_history.push(NetplayHistory {
                    state:              self.state.clone(),
                    prev_state:         self.prev_state.clone(),
                    fighter_selections: self.fighter_selections.clone(),
                    stage_ticker:       self.stage_ticker.clone(),
                });
            }
        }

        debug!("current_frame: {}", self.current_frame);
        self.game_setup.take()
    }

    pub fn render(&self) -> RenderMenu {
        RenderMenu {
            state: match self.state {
                MenuState::PackageSelect (ref names, ref ticker) => {
                    RenderMenuState::PackageSelect (
                        names.iter().map(|x| x.1.title.clone()).collect(),
                        ticker.cursor,
                        self.package_loader.as_ref().map(|x| x.state.message()).unwrap_or_default()
                    )
                }
                MenuState::GameResults { replay_saved } => RenderMenuState::GameResults { results: self.game_results.as_ref().unwrap().player_results.clone(), replay_saved },
                MenuState::CharacterSelect { back_counter, .. } => RenderMenuState::CharacterSelect (self.fighter_selections.clone(), back_counter, self.back_counter_max),
                MenuState::ReplaySelect (ref replays, ref ticker) => RenderMenuState::ReplaySelect (replays.clone(), ticker.cursor),
                MenuState::NetplayWait { ref message } => RenderMenuState::GenericText (message.clone()),
                MenuState::GameSelect  => RenderMenuState::GameSelect  (self.game_ticker.cursor),
                MenuState::StageSelect => RenderMenuState::StageSelect (self.stage_ticker.as_ref().unwrap().cursor),
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
            fullscreen:      self.config.fullscreen
        };

        GraphicsMessage {
            package_updates: updates,
            render:          render,
        }
    }

    pub fn reclaim(&mut self) -> (Package, Config) {
        match mem::replace(&mut self.package, PackageHolder::None) {
            PackageHolder::Package (package, _) => (package, self.config.clone()),
            PackageHolder::None                 => panic!("Attempted to access the package while there was none")
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

#[derive(Clone)]
pub enum MenuState {
    GameSelect,
    ReplaySelect (Vec<String>, MenuTicker), // MenuTicker must be tied with the Vec<String>, otherwise they may become out of sync
    CharacterSelect { back_counter: usize },
    StageSelect,
    GameResults { replay_saved: bool },
    PackageSelect (Vec<(String, PackageMeta)>, MenuTicker),
    NetplayWait { message: String },
}

impl MenuState {
    pub fn package_select() -> MenuState {
        let packages = package::get_package_metas();
        let ticker = MenuTicker::new(packages.len());
        MenuState::PackageSelect(packages, ticker)
    }

    pub fn replay_select(package: &Package) -> MenuState {
        let replays = replays::get_replay_names(package);
        let ticker = MenuTicker::new(replays.len());
        MenuState::ReplaySelect (replays, ticker)
    }

    pub fn character_select() -> MenuState {
        MenuState::CharacterSelect { back_counter: 0 }
    }

    pub fn game_results() -> MenuState {
        MenuState::GameResults { replay_saved: false }
    }
}

struct PackageLoader {
    state: PackageLoadState,
    rx:    Receiver<PackageLoadState>
}

#[derive(Clone)]
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

pub enum RenderMenuState {
    GameSelect      (usize),
    ReplaySelect    (Vec<String>, usize),
    CharacterSelect (Vec<PlayerSelect>, usize, usize),
    StageSelect     (usize),
    GameResults     { results: Vec<PlayerResult>, replay_saved: bool },
    PackageSelect   (Vec<String>, usize, String),
    GenericText     (String),
}

#[derive(Clone)]
pub struct PlayerSelect {
    pub controller:      Option<(usize, MenuTicker)>, // the cursor of the ticker is ignored
    pub fighter:         Option<usize>,
    pub cpu_ai:          Option<usize>,
    pub team:            usize,
    pub ui:              PlayerSelectUi,
    pub animation_frame: usize,
}

impl PlayerSelect {
    /// Returns true iff a controller can move to this selection
    pub fn is_free(&self) -> bool {
        self.ui.is_cpu() && self.controller.is_none()
    }
}

#[derive(Clone)]
pub enum PlayerSelectUi {
    CpuAi        (MenuTicker),
    CpuFighter   (MenuTicker),
    CpuTeam      (MenuTicker),
    HumanFighter (MenuTicker),
    HumanTeam    (MenuTicker),
    HumanUnplugged,
}

impl PlayerSelectUi {
    pub fn cpu_ai() -> Self {
        PlayerSelectUi::CpuAi (MenuTicker::new(/* TODO: number_of_ai + */ 1))
    }

    pub fn cpu_fighter(package: &Package) -> Self {
        PlayerSelectUi::CpuFighter (MenuTicker::new(package.fighters.len() + 3))
    }

    pub fn human_fighter(package: &Package) -> Self {
        PlayerSelectUi::HumanFighter (MenuTicker::new(package.fighters.len() + 2))
    }

    pub fn cpu_team() -> Self {
        PlayerSelectUi::CpuTeam (MenuTicker::new(graphics::get_colors().len() + 1))
    }

    pub fn human_team() -> Self {
        PlayerSelectUi::HumanTeam (MenuTicker::new(graphics::get_colors().len() + 1))
    }

    pub fn is_visible(&self) -> bool {
        match self {
            &PlayerSelectUi::HumanUnplugged => false,
            _                               => true
        }
    }

    pub fn is_cpu(&self) -> bool {
        match self {
            &PlayerSelectUi::CpuAi (_) |
            &PlayerSelectUi::CpuFighter (_) |
            &PlayerSelectUi::CpuTeam (_) => true,
            _                            => false
        }
    }

    pub fn is_human_plugged_in(&self) -> bool {
        match self {
            &PlayerSelectUi::HumanFighter (_) |
            &PlayerSelectUi::HumanTeam (_) => true,
            _                              => false
        }
    }

    pub fn ticker_unwrap(&self) -> &MenuTicker {
        match self {
            &PlayerSelectUi::HumanFighter (ref ticker) |
            &PlayerSelectUi::CpuFighter   (ref ticker) |
            &PlayerSelectUi::HumanTeam    (ref ticker) |
            &PlayerSelectUi::CpuTeam      (ref ticker) |
            &PlayerSelectUi::CpuAi        (ref ticker) => { ticker }
            &PlayerSelectUi::HumanUnplugged => {
                panic!("Tried to unwrap the PlayerSelectUi ticker but was HumanUnplugged")
            }
        }
    }

    pub fn ticker_full_reset(&mut self) {
        match self {
            &mut PlayerSelectUi::HumanFighter (ref mut ticker) |
            &mut PlayerSelectUi::CpuFighter   (ref mut ticker) |
            &mut PlayerSelectUi::HumanTeam    (ref mut ticker) |
            &mut PlayerSelectUi::CpuTeam      (ref mut ticker) |
            &mut PlayerSelectUi::CpuAi        (ref mut ticker) => {
                ticker.reset();
                ticker.cursor = 0;
            }
            &mut PlayerSelectUi::HumanUnplugged => { }
        }
    }
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

    /// increments internal state and returns true if a tick occurs
    pub fn tick(&mut self) -> bool {
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

/// # Game -> Menu Transitions
/// Results:   Game complete   -> display results -> CSS
/// Unchanged: Game quit       -> CSS
/// Results:   Replay complete -> display results -> replay ui
/// Unchanged: Replay quit     -> replay ui

#[derive(Clone, Serialize, Deserialize, Node)]
pub enum ResumeMenu {
    Results(GameResults),
    Unchanged,
    NetplayDisconnect { reason: String },
}

impl Default for ResumeMenu {
    fn default() -> Self {
        ResumeMenu::Unchanged
    }
}
