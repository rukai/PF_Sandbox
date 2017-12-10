#![windows_subsystem = "windows"]
extern crate pf_sandbox;

use pf_sandbox::app::{run};
use pf_sandbox::cli::cli;
use pf_sandbox::logger;

fn main() {
    logger::init();
    run(cli());
}
