use node::{Node, NodeRunner, NodeToken};

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct Rules {
    pub title:         String,
    pub goal:          Goal,
    pub stock_count:   u64,
    pub time_limit:    u64,
    pub best_of:       u64,
    pub teams:         bool,
    pub pause:         bool,
    pub friendly_fire: bool,
}

impl Rules {
    pub fn base() -> Rules {
        Rules {
            title:         "Base Game Mode".to_string(),
            goal:          Goal::Training,
            stock_count:   4,
            time_limit:    480,
            best_of:       3,
            pause:         true,
            teams:         false,
            friendly_fire: false,
        }
    }
}

impl Node for Rules {
    fn node_step(&mut self, mut runner: NodeRunner) -> String {
        match runner.step() {
            NodeToken::ChainProperty (property) => {
                match property.as_str() {
                    "title"         => self.title.node_step(runner),
                    //"goal"        =>
                    "stock_count"   => self.stock_count.node_step(runner),
                    "time_limit"    => self.time_limit.node_step(runner),
                    "best_of"       => self.best_of.node_step(runner),
                    "teams"         => self.teams.node_step(runner),
                    "pause"         => self.pause.node_step(runner),
                    "friendly_fire" => self.friendly_fire.node_step(runner),
                    prop            => format!("Rules does not have a property '{}'", prop)
                }
            }
            action => { format!("Rules cannot '{:?}'", action) }
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub enum Goal {
    Training,
    Time,
    Stock,
}
