use treeflection::{Node, NodeRunner, NodeToken};

#[derive(Debug, Clone, Default, Serialize, Deserialize, Node)]
pub struct GameResult {
    pub fighter:          usize,
    pub controller:       usize,
    pub place:            usize,
    pub kills:            Vec<DeathRecord>,
    pub deaths:           Vec<DeathRecord>,
    pub lcancel_percent:  f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Node)]
pub struct PlayerResult {
    pub deaths:           Vec<DeathRecord>,
    pub lcancel_attempts: u64,
    pub lcancel_success:  u64,
    pub final_damage:     Option<f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Node)]
pub struct DeathRecord {
    pub player: Option<usize>, // None indicates self-destruct
    pub frame:  usize,
}
