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
use std;

use ::cli::{CLIResults, ContinueFrom};
use ::config::Config;
use ::game::{Game, GameState, GameSetup};
use ::input::Input;
use ::menu::{Menu, MenuState};
use ::network::Network;
use ::os_input::OsInput;
use ::package::Package;
use ::package;

use libusb::Context;
use std::thread;
use std::time::{Duration, Instant};

pub fn run(mut cli_results: CLIResults) {
    if let ContinueFrom::Close = cli_results.continue_from {
        return;
    }

    let mut context = Context::new().unwrap();
    let mut input = Input::new(&mut context);
    #[cfg(any(feature = "vulkan", feature = "opengl"))]
    let mut graphics_tx: Option<Sender<GraphicsMessage>> = None;
    let mut next_state = NextAppState::None;
    let mut network = Network::new();

    // CLI options
    let (mut state, mut os_input) = {
        // default values
        let mut controllers: Vec<usize> = vec!();
        input.game_update(0); // TODO: is this needed? What can I do to remove it?
        for (i, _) in input.players(0).iter().enumerate() {
            controllers.push(i);
        }

        #[allow(unused_variables)] // Needed for headless build
        let (os_input, os_input_tx) = OsInput::new();

        package::generate_example_stub();
        let config = Config::load();
        let package_string = cli_results.package.or(config.current_package.clone());

        if let Some(total_players) = cli_results.total_players {
            while controllers.len() > total_players {
                controllers.pop();
            }
        }

        #[cfg(any(feature = "vulkan", feature = "opengl"))]
        {
            match cli_results.graphics_backend {
                #[cfg(feature = "vulkan")]
                GraphicsBackendChoice::Vulkan => {
                    graphics_tx = Some(VulkanGraphics::init(os_input_tx.clone()));
                }
                #[cfg(feature = "opengl")]
                GraphicsBackendChoice::OpenGL => {
                    graphics_tx = Some(OpenGLGraphics::init(os_input_tx.clone()));
                }
                GraphicsBackendChoice::Headless => {}
                GraphicsBackendChoice::Default => {
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

        let state = match cli_results.continue_from {
            ContinueFrom::Menu => {
                AppState::Menu(Menu::new(package, config, menu_state))
            }
            ContinueFrom::Game => {
                // handle no package
                let package = if let Some(package) = package {
                    package
                } else {
                    println!("No package was selected.");
                    println!("As a fallback we tried to use the last used package, but that wasnt available either.");
                    println!("Please select a package.");
                    return;
                };

                // handle issues with package that prevent starting from game
                if package.fighters.len() == 0 {
                    println!("Selected package has no fighters");
                    return;
                }
                else if package.stages.len() == 0 {
                    println!("Selected package has no stages");
                    return;
                }

                // handle missing and invalid cli input
                if cli_results.fighter_names.len() == 0 {
                    cli_results.fighter_names.push(package.fighters.index_to_key(0).unwrap());
                }
                for name in &cli_results.fighter_names {
                    if !package.fighters.contains_key(name) {
                        println!("Package does not contain selected fighter '{}'", name);
                        return;
                    }
                }
                if let &Some(ref name) = &cli_results.stage_name {
                    if !package.stages.contains_key(name) {
                        println!("Package does not contain selected stage '{}'", name);
                        return;
                    }
                }
                if cli_results.stage_name.is_none() {
                    cli_results.stage_name = package.stages.index_to_key(0);
                }

                let setup = GameSetup {
                    input_history:  vec!(),
                    player_history: vec!(),
                    stage_history:  vec!(),
                    controllers:    controllers,
                    fighters:       cli_results.fighter_names,
                    stage:          cli_results.stage_name.unwrap(),
                    state:          GameState::Local,
                };
                AppState::Game(Game::new(package, config, setup))
            }
            _ => unreachable!()
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
            NextAppState::Game (mut setup) => {
                let (package, config) = match state {
                    AppState::Menu (menu) => { menu.reclaim() }
                    AppState::Game (_)    => { unreachable!() }
                };
                input.set_history(std::mem::replace(&mut setup.input_history, vec!()));
                state = AppState::Game(Game::new(package, config, setup));
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
