#![windows_subsystem = "windows"]

#[macro_use] extern crate pf_sandbox;
#[macro_use] extern crate human_panic;

use pf_sandbox::app::run;
use pf_sandbox::cli::cli;
use pf_sandbox::config::Config;
use pf_sandbox::logger;

fn main() {
    pf_sandbox_setup_panic_handler!();
    logger::init();
    let config = Config::load();
    run(cli(&config), config);
}
