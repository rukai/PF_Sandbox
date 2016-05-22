use ::platform::Platform;

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct Stage {
    pub title: String,
    pub platforms: Vec<Platform>,
    pub bounds_x1: u64,
    pub bounds_y1: u64,
    pub bounds_x2: u64,
    pub bounds_y2: u64,
}

impl Stage {
    pub fn base() -> Stage {
        Stage {
            title: "Base Stage".to_string(),
            platforms: Vec::new(),
            bounds_x1: 0,
            bounds_y1: 0,
            bounds_x2: 0,
            bounds_y2: 0,
        }
    }
}
