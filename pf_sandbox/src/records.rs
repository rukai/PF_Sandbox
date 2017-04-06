use treeflection::{Node, NodeRunner, NodeToken};

#[derive(Debug, Clone, Default, Serialize, Deserialize, Node)]
pub struct GameResult {
    fighter:          usize,
    controller:       usize,
    place:            usize,
    kills:            Vec<DeathRecord>,
    deaths:           Vec<DeathRecord>,
    lcancel_percent:  f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Node)]
pub struct PlayerResult {
    fighter:          usize,
    controller:       usize,
    deaths:           Vec<DeathRecord>,
    lcancel_attempts: u64,
    lcancel_success:  u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Node)]
struct DeathRecord {
    player: Option<usize>, // None indicates self-destruct
    frame:  usize,
}

pub fn generate_game_results(player_results: Vec<PlayerResult>) -> Vec<GameResult> {
    let mut game_results: Vec<GameResult> = vec!();
    for player_result in player_results {
        let lcancel_percent = if player_result.lcancel_attempts == 0 {
            0.0
        }
        else {
            player_result.lcancel_success as f32 / player_result.lcancel_attempts as f32
        };
        game_results.push(GameResult {
            fighter:         player_result.fighter,
            controller:      player_result.controller,
            place:           0,
            kills:           vec!(),
            deaths:          player_result.deaths,
            lcancel_percent: lcancel_percent,
        });
    }
    game_results
}
