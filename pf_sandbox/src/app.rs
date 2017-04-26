#[cfg(feature = "vulkan")]
use ::vulkan::VulkanGraphics;
#[cfg(feature = "opengl")]
use ::opengl::OpenGLGraphics;
#[cfg(any(feature = "vulkan", feature = "opengl"))]
use ::cli::GraphicsBackendChoice;
#[cfg(any(feature = "vulkan", feature = "opengl"))]
use ::graphics::GraphicsMessage;
#[cfg(any(feature = "vulkan", feature = "opengl"))]
use std::sync::mpsc::Sender;

use ::cli::CLIChoice;
use ::game::{Game, GameState};
use ::input::Input;
use ::menu::{Menu, MenuState};
use ::network::Network;
use ::os_input::OsInput;
use ::package::Package;
use ::package;
use ::config::Config;

use libusb::Context;
use std::thread;
use std::time::{Duration, Instant};

pub fn run(cli_choices: Vec<CLIChoice>) {
    let mut context = Context::new().unwrap();
    let mut input = Input::new(&mut context);
    #[cfg(any(feature = "vulkan", feature = "opengl"))]
    let mut graphics_tx: Option<Sender<GraphicsMessage>> = None;
    let mut next_state = NextAppState::None;
    let mut network = Network::new();

    // CLI options
    let (mut state, mut os_input) = {
        // default values
        let mut stage: Option<String> = None;
        let netplay = false;
        let mut fighters: Vec<String> = vec!();
        let mut controllers: Vec<usize> = vec!();
        input.game_update(0); // TODO: is this needed? What can I do to remove it?
        for (i, _) in input.players(0).iter().enumerate() {
            controllers.push(i);
        }

        #[allow(unused_variables)] // Needed for headless build
        let (os_input, os_input_tx) = OsInput::new();

        package::generate_example_stub();
        let config = Config::load();
        let mut package_string = config.current_package.clone();

        // replace with any cli_choices
        let mut load_menu = true;
        for choice in &cli_choices {
            match choice {
                &CLIChoice::Close => { return; }
                &CLIChoice::FighterNames (ref fighters_names)   => { load_menu = false; fighters = fighters_names.clone(); }
                &CLIChoice::StageName (ref stage_name)          => { load_menu = false; stage = Some(stage_name.clone()); }
                &CLIChoice::Package (ref name)                  => { load_menu = false; package_string = Some(name.clone()); }
                &CLIChoice::GraphicsBackend (_) => { }
                &CLIChoice::TotalPlayers (total_players) => {
                    load_menu = false;
                    while controllers.len() > total_players {
                        controllers.pop();
                    }
                }
            }
        }

        #[cfg(any(feature = "vulkan", feature = "opengl"))]
        {
            let mut set_default_graphics = true;
            for choice in cli_choices {
                match &choice {
                    &CLIChoice::GraphicsBackend (ref backend_choice) => {
                        set_default_graphics = false;
                        match backend_choice {
                            #[cfg(feature = "vulkan")]
                            &GraphicsBackendChoice::Vulkan => {
                                graphics_tx = Some(VulkanGraphics::init(os_input_tx.clone()));
                            }
                            #[cfg(feature = "opengl")]
                            &GraphicsBackendChoice::OpenGL => {
                                graphics_tx = Some(OpenGLGraphics::init(os_input_tx.clone()));
                            }
                            &GraphicsBackendChoice::None => {}
                        }
                    }
                    _ => { }
                }
            }
            if set_default_graphics {
                #[cfg(feature = "vulkan")]
                {
                    graphics_tx = Some(VulkanGraphics::init(os_input_tx.clone()));
                }
                #[cfg(all(not(feature = "vulkan"), feature = "opengl"))]
                {
                    graphics_tx = Some(OpenGLGraphics::init(os_input_tx.clone()));
                }
            }
        }

        let package = if let Some(package_string) = package_string {
            Package::open_or_generate(&package_string)
        } else {
            None
        };
        let menu_state: MenuState = if let Some(_) = package {
            MenuState::character_select()
        } else {
            MenuState::package_select()
        };

        let state = if load_menu {
            AppState::Menu(Menu::new(package, config, menu_state))
        } else {
            AppState::Game(Game::new(package.unwrap(), config, fighters, stage.unwrap(), netplay, controllers)) // TODO: handle no packages nicely
        };
        (state, os_input)
    };

    loop {
        let frame_start = Instant::now();

        os_input.update();

        match &mut state {
            &mut AppState::Menu (ref mut menu) => {
                input.update(&[]);
                if let Some(menu_game_setup) = menu.step(&mut input) {
                    next_state = NextAppState::Game (menu_game_setup);
                }
                #[cfg(any(feature = "vulkan", feature = "opengl"))]
                {
                    if let Some(ref tx) = graphics_tx {
                        tx.send(menu.graphics_message()).unwrap();
                    }
                }
            }
            &mut AppState::Game (ref mut game) => {
                input.update(&game.tas);
                match game.step(&mut input, &os_input) {
                    GameState::ToResults (results) => {
                        next_state = NextAppState::Menu (MenuState::GameResults (results));
                    }
                    GameState::ToCSS => {
                        next_state = NextAppState::Menu (MenuState::character_select());
                    }
                    _ => { }
                }
                network.update(game);
                #[cfg(any(feature = "vulkan", feature = "opengl"))]
                {
                    if let Some(ref tx) = graphics_tx {
                        tx.send(game.graphics_message()).unwrap();
                    }
                }
            }
        };

        match next_state {
            NextAppState::Game (setup) => {
                let (package, config) = match state {
                    AppState::Menu (menu) => { menu.reclaim() }
                    AppState::Game (_)    => { unreachable!() }
                };
                input.reset_history();
                state = AppState::Game(Game::new(package, config, setup.fighters, setup.stage, setup.netplay, setup.controllers));
            }
            NextAppState::Menu (menu_state) => {
                let (package, config) = match state {
                    AppState::Menu (_)    => { unreachable!() }
                    AppState::Game (game) => { game.reclaim() }
                };
                state = AppState::Menu(Menu::new(Some(package), config, menu_state));
            }
            NextAppState::None => { }
        }
        next_state = NextAppState::None;

        if os_input.quit() {
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
    Menu (MenuState),
    None
}

#[derive(Clone)]
pub struct GameSetup {
    pub controllers: Vec<usize>,
    pub fighters:    Vec<String>,
    pub stage:       String,
    pub netplay:     bool,
}
