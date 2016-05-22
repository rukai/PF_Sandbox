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
            stock_count:   8,
            time_limit:    480,
            best_of:       3,
            pause:         true,
            teams:         false,
            friendly_fire: false,
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub enum Goal {
    Training,
    Time,
    Stock,
}
