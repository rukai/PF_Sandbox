use pf_sandbox_lib::package;

use getopts::Options;
use std::env;
use std::net::IpAddr;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] [package_dir]\nIf no arguments are given the GUI menu is used instead. (excluding -g)", program);
    print!("{}", opts.usage(&brief));
}

pub fn cli() -> CLIResults {
    let args: Vec<String> = env::args().collect();
    let program = &args[0];

    let mut opts = Options::new();
    opts.optflag("l", "list", "List available packages and close");
    opts.optopt("s", "stage",          "Use the stage specified", "NAME");
    opts.optopt("f", "fighters",       "Use the fighters specified", "NAME1,NAME2,NAME3...");
    opts.optopt("h", "humanplayers",   "Number of human players in the game", "NUM_HUMAN_PLAYERS");
    opts.optopt("c", "cpuplayers",     "Number of CPU players in the game", "NUM_CPU_PLAYERS");
    opts.optopt("a", "address",        "IP Address of other client to start netplay with", "IP_ADDRESS");
    opts.optopt("n", "netplayplayers", "Search for a netplay game with the specified number of players", "NUM_PLAYERS");
    opts.optopt("r", "netplayregion",  "Search for a netplay game with the specified region", "REGION");
    opts.optopt("g", "graphics",       "Graphics backend to use",
        if cfg!(feature = "wgpu_renderer") {
            "[wgpu|none]"
        } else {
            "[none]"
        }
    );

    let mut results = CLIResults::new();

    let matches = match opts.parse(&args[1..]) {
        Ok(m)  => m,
        Err(_) => {
            print_usage(program, opts);
            results.continue_from = ContinueFrom::Close;
            return results;
        },
    };

    if matches.opt_present("l") {
        package::print_list();
        results.continue_from = ContinueFrom::Close;
        return results;
    }

    if matches.free.len() > 1 {
        print_usage(program, opts);
        results.continue_from = ContinueFrom::Close;
        return results;
    }
    else if matches.free.len() == 1 {
        results.continue_from = ContinueFrom::Game;
        results.package = Some(matches.free[0].clone());
    }

    if let Some(players) = matches.opt_str("h") {
        if let Ok(players) = players.parse::<usize>() {
            results.continue_from = ContinueFrom::Game;
            results.max_human_players = Some(players);
        }
        else {
            print_usage(program, opts);
            results.continue_from = ContinueFrom::Close;
            return results;
        }
    }

    if let Some(players) = matches.opt_str("c") {
        if let Ok(players) = players.parse::<usize>() {
            results.continue_from = ContinueFrom::Game;
            results.total_cpu_players = Some(players);
        }
        else {
            print_usage(program, opts);
            results.continue_from = ContinueFrom::Close;
            return results;
        }
    }

    if let Some(fighter_names) = matches.opt_str("f") {
        for fighter_name in fighter_names.split(",") {
            results.continue_from = ContinueFrom::Game;
            results.fighter_names.push(fighter_name.to_string());
        }
    }

    if let Some(stage) = matches.opt_str("s") {
        results.stage_name = Some(stage);
        results.continue_from = ContinueFrom::Game;
    }

    if let Some(address) = matches.opt_str("a") {
        if let Ok(address) = address.parse() {
            results.address = Some(address);
            results.continue_from = ContinueFrom::Netplay;
        }
        else {
            print_usage(program, opts);
            results.continue_from = ContinueFrom::Close;
            return results;
        }
    }

    if let Some(backend_string) = matches.opt_str("g") {
        results.graphics_backend = match backend_string.to_lowercase().as_ref() {
            #[cfg(feature = "wgpu_renderer")]
            "wgpu" => { GraphicsBackendChoice::Wgpu }
            "none" => { GraphicsBackendChoice::Headless }
            _ => {
                print_usage(program, opts);
                results.continue_from = ContinueFrom::Close;
                return results;
            }
        };
    }

    if let Some(players) = matches.opt_str("n") {
        if let Ok(players) = players.parse() {
            results.netplay_players = Some(players);
            results.continue_from = ContinueFrom::MatchMaking;
        } else {
            print_usage(program, opts);
            results.continue_from = ContinueFrom::Close;
            return results;
        }
    }

    if let Some(region) = matches.opt_str("r") {
        results.netplay_region = Some(region);
        results.continue_from = ContinueFrom::MatchMaking;
    }

    results
}

pub struct CLIResults {
    pub graphics_backend:  GraphicsBackendChoice,
    pub package:           Option<String>,
    pub max_human_players: Option<usize>,
    pub total_cpu_players: Option<usize>,
    pub fighter_names:     Vec<String>,
    pub stage_name:        Option<String>,
    pub address:           Option<IpAddr>,
    pub continue_from:     ContinueFrom,
    pub netplay_players:   Option<u8>,
    pub netplay_region:    Option<String>,
}

impl CLIResults {
    pub fn new() -> CLIResults {
        CLIResults {
            graphics_backend:  GraphicsBackendChoice::Default,
            package:           None,
            max_human_players: None,
            total_cpu_players: None,
            fighter_names:     vec!(),
            stage_name:        None,
            address:           None,
            continue_from:     ContinueFrom::Menu,
            netplay_players:   None,
            netplay_region:    None,
        }
    }
}

pub enum ContinueFrom {
    Menu,
    Netplay,
    MatchMaking,
    Game,
    Close
}

pub enum GraphicsBackendChoice {
    #[cfg(feature = "wgpu_renderer")]
    Wgpu,
    Headless,
    Default,
}
