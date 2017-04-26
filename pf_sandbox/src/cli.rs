use ::package;
use getopts::Options;
use std::env;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] [package_name]\nIf no arguments are given the GUI menu is used instead. (excluding -g)", program);
    print!("{}", opts.usage(&brief));
}

pub fn cli() -> Vec<CLIChoice> {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("l", "list", "List available packages and close");
    opts.optopt("s", "stage",        "Use the stage specified", "NAME");
    opts.optopt("f", "fighter",      "Use the fighters specified", "NAME1,NAME2,NAME3...");
    opts.optopt("p", "players",      "Number of players in the game", "NUMPLAYERS");
    opts.optopt("g", "graphics",     "Graphics backend to use",
        if cfg!(features =  "vulkan") && cfg!(features = "opengl") {
            "[vulkan|opengl|none]"
        } else if cfg!(features =  "vulkan") {
            "[vulkan|none]"
        } else if cfg!(features = "opengl") {
            "[opengl|none]"
        } else {
            "[none]"
        }
    );

    let matches = match opts.parse(&args[1..]) {
        Ok(m)  => { m },
        Err(_) => {
            print_usage(&program, opts);
            return vec!(CLIChoice::Close);
        },
    };

    if matches.opt_present("l") {
        package::print_list();
        return vec!(CLIChoice::Close);
    }

    let mut cli_choices: Vec<CLIChoice> = vec!();

    if matches.free.len() > 1 {
        print_usage(&program, opts);
        return vec!(CLIChoice::Close);
    }
    else if matches.free.len() == 1 {
        cli_choices.push(CLIChoice::Package(matches.free[0].clone()));
    }

    if let Some(players) = matches.opt_str("p") {
        if let Ok(players) = players.parse::<usize>() {
            cli_choices.push(CLIChoice::TotalPlayers(players));
        }
    }

    if let Some(fighter_names) = matches.opt_str("f") {
        let mut result: Vec<String> = vec!();
        for fighter_name in fighter_names.split(",") {
            result.push(fighter_name.to_string());
        }
        cli_choices.push(CLIChoice::FighterNames(result));
    }

    if let Some(stage_name) = matches.opt_str("s") {
        cli_choices.push(CLIChoice::StageName(stage_name));
    }
    if let Some(backend_string) = matches.opt_str("g") {
        match backend_string.to_lowercase().as_ref() {
            #[cfg(feature = "vulkan")]
            "vulkan" => {
                cli_choices.push(CLIChoice::GraphicsBackend (GraphicsBackendChoice::Vulkan));
            }
            #[cfg(feature = "opengl")]
            "opengl" => {
                cli_choices.push(CLIChoice::GraphicsBackend (GraphicsBackendChoice::OpenGL));
            }
            "none" => {
                cli_choices.push(CLIChoice::GraphicsBackend (GraphicsBackendChoice::None));
            }
            _ => {
                print_usage(&program, opts);
                return vec!(CLIChoice::Close);
            }
        }
    }

    cli_choices
}

pub enum CLIChoice {
    TotalPlayers    (usize),
    FighterNames    (Vec<String>),
    StageName       (String),
    Package         (String),
    GraphicsBackend (GraphicsBackendChoice),
    Close,
}

pub enum GraphicsBackendChoice {
    #[cfg(feature = "vulkan")]
    Vulkan,
    #[cfg(feature = "opengl")]
    OpenGL,
    None,
}
