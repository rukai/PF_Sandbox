#![windows_subsystem = "windows"]
extern crate pf_sandbox;

use pf_sandbox::app::{run};
use pf_sandbox::cli::cli;
use pf_sandbox::logger;
use pf_sandbox::config::Config;

fn main() {
    logger::init();
    let config = Config::load();
    run(cli(&config), config);
}
