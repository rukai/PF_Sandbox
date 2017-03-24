extern crate pf_sandbox;

use pf_sandbox::app::{run};
use pf_sandbox::cli::cli;

fn main() {
    run(cli());
}
