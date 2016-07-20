use ::package::Package;
use ::menu::{Menu, RenderMenu, MenuChoice};
use ::cli::CLIChoice;
use ::game::{Game, RenderGame};
use ::graphics::{Graphics, GraphicsMessage};
use ::input::{Input};

use libusb::Context;
use glium::glutin::VirtualKeyCode;
use std::thread;
use std::time::{Duration, Instant};

pub fn run(mut state: AppState) {
    let mut context = Context::new().unwrap();
    let mut input = Input::new(&mut context);
    let mut package = Package::open_or_generate("base_package");
    let (graphics_tx, mut key_input) = Graphics::init(&package);
    let mut next_state: Option<AppState> = None;

    loop {
        let frame_start = Instant::now();

        input.update();
        key_input.update();

        match &mut state {
            &mut AppState::Menu (ref mut menu) => {
                for menu_choice in menu.step(&mut input) {
                    match menu_choice {
                        MenuChoice::Start { controllers, fighters, stage, netplay } => {
                            input.reset_history();
                            next_state = Some(AppState::Game(Game::new(&package, fighters, stage, netplay, controllers)));
                        }
                        MenuChoice::ChangePackage (name) => {
                            package = Package::open_or_generate(&name);
                        }
                    }
                }

                graphics_tx.send(GraphicsMessage {
                    package_updates: package.updates(),
                    render:  Render::Menu(menu.render()),
                }).unwrap();
            }

            &mut AppState::CLI(ref cli_choices) => {
                // default values
                let stage = 0;
                let netplay = false;
                let fighters: Vec<usize> = vec!(0);
                let mut controllers: Vec<usize> = vec!();
                input.game_update(0);
                for (i, _) in input.players(0).iter().enumerate() {
                    controllers.push(i);
                }

                // replace with any cli_choices
                for choice in cli_choices {
                    match choice {
                        &CLIChoice::Package(ref name) => { package = Package::open_or_generate(&name); },
                        &CLIChoice::Close => { return; },
                    }
                }

                input.reset_history();
                next_state = Some(AppState::Game(Game::new(&package, fighters, stage, netplay, controllers)));
            }

            &mut AppState::Game (ref mut game) => {
                game.step(&mut package, &mut input, &key_input);

                graphics_tx.send(GraphicsMessage {
                    package_updates: package.updates(),
                    render:  Render::Game(game.render()),
                }).unwrap();
            }
        };

        if let Some(next) = next_state {
            state = next;
            next_state = None;
        }

        if key_input.pressed(VirtualKeyCode::Escape) {
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

pub enum Render {
    Game (RenderGame),
    Menu (RenderMenu),
}
