extern crate pf_engine;

use pf_engine::app::{run, AppState};
use pf_engine::cli::cli;
use pf_engine::menu::Menu;
use std::env;

fn main() {
    if env::args().len() == 1 {
        run(AppState::Menu(Menu::new()));
    } else {
        run(AppState::CLI(cli()));
    }
}
