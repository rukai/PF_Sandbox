use replays::Replay;
use treeflection::{Node, NodeRunner, NodeToken};

#[derive(Clone, Serialize, Deserialize)]
pub struct GameResults {
    pub player_results: Vec<PlayerResult>,
    pub replay:         Replay,
}

impl Node for GameResults {
    fn node_step(&mut self, _: NodeRunner) -> String {
        String::from("GameResults is not accessible via treeflection.")
    }
}

impl Default for GameResults {
    fn default() -> Self {
        panic!("Wow you must have tried really hard to hit this code...\nYour reward is a panic.\nAre you happy now?"); // TODO
    }
}

/// An individual players results: processed according to other players and current game mode
#[derive(Debug, Clone, Default, Serialize, Deserialize, Node)]
pub struct PlayerResult {
    pub fighter:          String,
    pub team:             usize,
    pub controller:       usize,
    pub place:            usize,
    pub kills:            Vec<DeathRecord>,
    pub deaths:           Vec<DeathRecord>,
    pub lcancel_percent:  f32,
}

/// An individual players results: unprocessed
#[derive(Debug, Clone, Default, Serialize, Deserialize, Node)]
pub struct RawPlayerResult {
    pub team:             usize,
    pub deaths:           Vec<DeathRecord>,
    pub lcancel_attempts: u64,
    pub lcancel_success:  u64,
    pub final_damage:     Option<f32>,
    pub ended_as_fighter: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Node)]
pub struct DeathRecord {
    pub player: Option<usize>, // None indicates self-destruct
    pub frame:  usize,
}
