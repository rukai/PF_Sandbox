use getopts::Options;
use std::fs;
use std::env;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] [package_name]\nIf no arguments are given the GUI menu is used instead.", program);
    print!("{}", opts.usage(&brief));
}

pub fn cli() -> Vec<CLIChoice> {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("l", "list", "List available packages and close");
    opts.optopt("s", "stage",        "Use the stage specified by name", "NAME");
    opts.optopt("S", "stageIndex",   "Use the stage specified by index", "INDEX");
    opts.optopt("f", "fighter",      "Use the fighters specified by names", "NAME1,NAME2,NAME3...");
    opts.optopt("F", "fighterIndex", "Use the fighters specified by indexes", "INDEX1,INDEX2,INDEX3...");
    opts.optopt("p", "players",      "Number of players in the game", "NUMPLAYERS");
    let matches = match opts.parse(&args[1..]) {
        Ok(m)  => { m },
        Err(_) => {
            print_usage(&program, opts);
            return vec!(CLIChoice::Close);
        },
    };

    if matches.opt_present("l") {
        for path in fs::read_dir("packages").unwrap() {
            println!("{}", path.unwrap().file_name().to_str().unwrap());
        }
        return vec!(CLIChoice::Close);
    }

    if matches.free.len() != 1 {
        print_usage(&program, opts);
        return vec!(CLIChoice::Close);
    }

    let mut cli_choices: Vec<CLIChoice> = vec!();

    if let Some(players) = matches.opt_str("p") {
        if let Ok(players) = players.parse::<usize>() {
            cli_choices.push(CLIChoice::TotalPlayers(players));
        }
    }
    if let Some(fighter_indexes) = matches.opt_str("F") {
        let mut result: Vec<usize> = vec!();
        for fighter_index in fighter_indexes.split(",") {
            match fighter_index.parse::<usize>() {
                Ok(fighter_index) => {
                    result.push(fighter_index);
                }
                Err(_) => {
                    print_usage(&program, opts);
                    return vec!(CLIChoice::Close);
                }
            }
        }
        cli_choices.push(CLIChoice::FighterIndexes(result));
    }
    if let Some(fighter_names) = matches.opt_str("f") {
        let mut result: Vec<String> = vec!();
        for fighter_name in fighter_names.split(",") {
            result.push(fighter_name.to_string());
        }
        cli_choices.push(CLIChoice::FighterNames(result));
    }
    if let Some(stage_index) = matches.opt_str("S") {
        if let Ok(stage_index) = stage_index.parse::<usize>() {
            cli_choices.push(CLIChoice::StageIndex(stage_index));
        }
    }
    if let Some(stage_name) = matches.opt_str("s") {
        cli_choices.push(CLIChoice::StageName(stage_name));
    }

    cli_choices.push(CLIChoice::Package(matches.free[0].clone()));
    cli_choices
}

pub enum CLIChoice {
    TotalPlayers   (usize),
    FighterIndexes (Vec<usize>),
    FighterNames   (Vec<String>),
    StageIndex     (usize),
    StageName      (String),
    Package        (String),
    Close,
}
