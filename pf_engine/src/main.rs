extern crate pf_engine;

use pf_engine::app::{run};
use pf_engine::cli::cli;

fn main() {
    run(cli());
}
