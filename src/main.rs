extern crate pf_engine;
extern crate getopts;

use pf_engine::package::Package;
use pf_engine::menu::{Menu, MenuChoice};
use pf_engine::game::Game;
use pf_engine::graphics::Graphics;

use getopts::Options;
use std::env;
use std::fs;
use std::path::Path;
use std::thread;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] [package_name]\nIf no arguments are given the GUI menu is used instead.", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    if env::args().len() == 1 {
        gui();
    } else {
        cli();
    }
}

fn cli() {
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

    let mut game = Game::new(&package, vec!("base_fighter".to_string()), "base_stage".to_string());
    init_graphics(&game, &package);
    game.run();
}

fn init_graphics(game: &Game, package: &Package) {
    let players = game.players.clone();
    let fighters = package.fighters.clone();
    let stages = package.stages.clone();
    thread::spawn(move || {
        let mut graphics = Graphics::new(players, fighters, stages);
        graphics.run();
    });
}

fn gui() {
    loop {
        let menu_choice = Menu::new().run();
        let package = Package::open(&menu_choice.package_name); //package should already exist as the menu has generated it.
        let mut game = Game::new(&package, menu_choice.fighter_names, menu_choice.stage_name);
        init_graphics(&game, &package);
        game.run();
    }
}
