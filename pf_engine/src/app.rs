use ::package::Package;
use ::menu::{Menu, RenderMenu, MenuChoice};
use ::cli::CLIChoice;
use ::game::{Game, RenderGame, GameState};
use ::graphics::Graphics;
use ::input::Input;
use ::network::Network;

use libusb::Context;
use winit::VirtualKeyCode;
use std::thread;
use std::time::{Duration, Instant};

pub fn run(mut state: AppState) {
    let mut context = Context::new().unwrap();
    let mut input = Input::new(&mut context);
    let (graphics_tx, mut os_input) = Graphics::init();
    let mut next_state = NextAppState::None;
    let mut network = Network::new();

    loop {
        let frame_start = Instant::now();

        os_input.update();

        match &mut state {
            &mut AppState::Menu (ref mut menu) => {
                input.update(&[]);
                for menu_choice in menu.step(&mut input) {
                    match menu_choice {
                        MenuChoice::Start (menu_game_setup) => {
                            next_state = NextAppState::Game (menu_game_setup, PackageSource::FromState);
                        }
                    }
                }

                graphics_tx.send(menu.graphics_message()).unwrap();
            }

            &mut AppState::CLI(ref cli_choices) => {
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

                // replace with any cli_choices
                for choice in cli_choices {
                    match choice {
                        &CLIChoice::Close => { return; }
                        &CLIChoice::FighterIndexes (ref fighters_index) => { fighters = fighters_index.clone() }
                        &CLIChoice::FighterNames (_)                  => { panic!("Unimplemented") }
                        &CLIChoice::StageIndex (ref stage_index)       => { stage = *stage_index }
                        &CLIChoice::StageName (_)                      => { panic!("Unimplemented") }
                        &CLIChoice::Package (ref name)                 => { load_package = Some(Package::open_or_generate(&name)); }
                        &CLIChoice::TotalPlayers (total_players) => {
                            while controllers.len() > total_players {
                                controllers.pop();
                            }
                        }
                    }
                }

                let package = match load_package {
                    Some(p) => p,
                    None    => Package::open_or_generate("base_package")
                };

                next_state = NextAppState::Game(
                    GameSetup {
                        controllers: controllers,
                        fighters: fighters,
                        stage: stage,
                        netplay: netplay,
                    },
                    PackageSource::Move(package)
                );
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
                graphics_tx.send(game.graphics_message()).unwrap();
            }
        };

        match next_state {
            NextAppState::Game (setup, package_source) => {
                let package = match package_source {
                    PackageSource::Move (package) => {
                        package
                    }
                    PackageSource::FromState => {
                        match state {
                            AppState::Menu (menu) => {
                                menu.reclaim()
                            }
                            _ => { panic!("Unaccounted for!") }
                        }
                    }
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
    CLI  (Vec<CLIChoice>),
}

enum NextAppState {
    Game (GameSetup, PackageSource), // retrieve package from the menu
    Menu,
    None
}

enum PackageSource { // TODO: maybe I could get rid of this enum by adding package to cli struct
    Move (Package),
    FromState,
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
