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

    cli_choices.push(CLIChoice::Package(matches.free[0].clone()));
    cli_choices
}

pub enum CLIChoice {
    Package (String),
    Close,
}
