use ::package::Package;
use ::menu::{Menu, RenderMenu, MenuChoice};
use ::cli::{CLIChoice, GraphicsBackendChoice};
use ::game::{Game, RenderGame, GameState};
use ::graphics::{Graphics, GraphicsMessage};
use ::input::Input;
use ::network::Network;
use ::os_input::OsInput;

use libusb::Context;
use winit::VirtualKeyCode;
use std::thread;
use std::time::{Duration, Instant};
use std::sync::mpsc::Sender;

pub fn run(cli_choices: Vec<CLIChoice>) {
    let mut context = Context::new().unwrap();
    let mut input = Input::new(&mut context);
    let mut graphics_tx: Option<Sender<GraphicsMessage>> = None;
    let mut next_state = NextAppState::None;
    let mut network = Network::new();

    // CLI options
    let (mut state, mut os_input) = {
        // default values
        let mut stage = 0usize;
        let netplay = false;
        let mut fighters: Vec<usize> = vec!(0);
        let mut controllers: Vec<usize> = vec!();
        input.game_update(0); // TODO: is this needed? What can I do to remove it?
        for (i, _) in input.players(0).iter().enumerate() {
            controllers.push(i);
        }

        let mut load_package: Option<Package> = None;

        let (os_input, os_input_tx) = OsInput::new();

        // replace with any cli_choices
        let mut load_menu = true;
        let mut set_default_graphics = true;
        for choice in cli_choices {
            match &choice {
                &CLIChoice::Close => { return; }
                &CLIChoice::FighterIndexes (ref fighters_index) => { load_menu = false; fighters = fighters_index.clone() }
                &CLIChoice::FighterNames (_)                    => { panic!("Unimplemented") }
                &CLIChoice::StageIndex (ref stage_index)        => { load_menu = false; stage = *stage_index }
                &CLIChoice::StageName (_)                       => { panic!("Unimplemented") }
                &CLIChoice::Package (ref name)                  => { load_menu = false; load_package = Some(Package::open_or_generate(&name)); }
                &CLIChoice::GraphicsBackend (ref backend_choice) => {
                    match backend_choice {
                        // use conditional compilation to choose
                        &GraphicsBackendChoice::Vulkan => {
                            graphics_tx = Some(Graphics::init(os_input_tx.clone()));
                        }
                        &GraphicsBackendChoice::None => {
                            set_default_graphics = false;
                        }
                    }
                }
                &CLIChoice::TotalPlayers (total_players) => {
                    load_menu = false;
                    while controllers.len() > total_players {
                        controllers.pop();
                    }
                }
            }
        }

        if set_default_graphics {
            graphics_tx = Some(Graphics::init(os_input_tx.clone()));
        }

        let package = match load_package {
            Some(p) => p,
            None    => Package::open_or_generate("base_package")
        };

        let state = if load_menu {
            AppState::Menu(Menu::new())
        } else {
            AppState::Game(Game::new(package, fighters, stage, netplay, controllers))
        };
        (state, os_input)
    };

    loop {
        let frame_start = Instant::now();

        os_input.update();

        match &mut state {
            &mut AppState::Menu (ref mut menu) => {
                input.update(&[]);
                for menu_choice in menu.step(&mut input) {
                    match menu_choice {
                        MenuChoice::Start (menu_game_setup) => {
                            next_state = NextAppState::Game (menu_game_setup);
                        }
                    }
                }
                if let Some(ref tx) = graphics_tx {
                    tx.send(menu.graphics_message()).unwrap();
                }
            }
            &mut AppState::Game (ref mut game) => {
                input.update(&game.tas);
                match game.step(&mut input, &os_input) {
                    GameState::Results => {
                        next_state = NextAppState::Menu;
                    }
                    _ => { }
                }
                network.update(game);
                if let Some(ref tx) = graphics_tx {
                    tx.send(game.graphics_message()).unwrap();
                }
            }
        };

        match next_state {
            NextAppState::Game (setup) => {
                let package = match state {
                    AppState::Menu (menu) => {
                        menu.reclaim()
                    }
                    AppState::Game (_) => { unreachable!() }
                };
                input.reset_history();
                state = AppState::Game(Game::new(package, setup.fighters, setup.stage, setup.netplay, setup.controllers));
            }
            NextAppState::Menu => { }
            NextAppState::None => { }
        }
        next_state = NextAppState::None;

        if os_input.key_pressed(VirtualKeyCode::Escape) {
            return;
        }

        let frame_duration = Duration::from_secs(1) / 60;
        let frame_duration_actual = frame_start.elapsed();
        if frame_duration_actual < frame_duration {
            thread::sleep(frame_duration - frame_start.elapsed());
        }
    }
}

pub enum AppState {
    Game (Game),
    Menu (Menu),
}

enum NextAppState {
    Game (GameSetup), // retrieve package from the menu
    Menu,
    None
}

#[derive(Clone)]
pub struct GameSetup {
    pub controllers: Vec<usize>,
    pub fighters:    Vec<usize>,
    pub stage:       usize,
    pub netplay:     bool,
}

pub enum Render {
    Game (RenderGame),
    Menu (RenderMenu),
}
