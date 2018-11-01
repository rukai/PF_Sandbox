use treeflection::{Node, NodeRunner, NodeToken};

use crate::json_upgrade::engine_version;

#[derive(Clone, Serialize, Deserialize, Node)]
pub struct Rules {
    pub engine_version:     u64,
    pub title:              String,
    pub goal:               Goal,
    pub stock_count:        Option<u64>,
    pub time_limit_seconds: Option<u64>,
    pub best_of:            u64,
    pub pause:              Pause,
    pub teams:              Teams,
    pub grab_clang:         bool,
    //pub force_user_settings: User,
}

impl Default for Rules {
    fn default() -> Self {
        Rules {
            engine_version:     engine_version(),
            title:              "Base Game Mode".to_string(),
            goal:               Goal::default(),
            stock_count:        Some(4),
            time_limit_seconds: Some(480),
            best_of:            1,
            pause:              Pause::default(),
            teams:              Teams::default(),
            grab_clang:         false,
        }
    }
}

impl Rules {
    pub fn time_limit_frames(&self) -> Option<u64> {
        self.time_limit_seconds.map(|x| x * 60)
    }
}

#[derive(Clone, Serialize, Deserialize, Node)]
pub enum Goal {
    KillDeathScore,
    LastManStanding,
}

#[derive(Clone, Serialize, Deserialize, Node)]
pub enum Pause {
    On,
    Off,
    Hold,
}

#[derive(Clone, Serialize, Deserialize, Node)]
pub enum Teams {
    On { friendly_fire: bool },
    Off,
}

impl Default for Goal {
    fn default() -> Self {
        Goal::LastManStanding
    }
}

impl Default for Pause {
    fn default() -> Self {
        Pause::On
    }
}

impl Default for Teams {
    fn default() -> Self {
        Teams::Off
    }
}
