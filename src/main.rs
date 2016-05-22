extern crate pf_engine;
extern crate getopts;

use pf_engine::package::Package;
use std::env;
use getopts::Options;
use std::path::Path;
use std::fs;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] package_name", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("l", "list", "List available packages and close");
    let matches = match opts.parse(&args[1..]) {
        Ok(m)  => { m },
        Err(_) => { print_usage(&program, opts); return; },
    };

    if matches.opt_present("l") {
        for path in fs::read_dir("packages").unwrap() {
            println!("{}", path.unwrap().file_name().to_str().unwrap());
        }
        return;
    }

    if matches.free.len() != 1 {
        print_usage(&program, opts);
        return;
    }

    let package_name = matches.free[0].clone();
    let package_path = Path::new("packages").join(&package_name);

    //if a package does not already exist create a new one
    let package = match fs::metadata(package_path) {
        Ok(_)  => Package::open(&package_name),
        Err(_) => Package::generate_base(&package_name),
    };

    let mut game = package.new_game();
    game.run();
}
