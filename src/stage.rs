use ::platform::Platform;
use ::game::Point;

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct Stage {
    pub title:        String,
    pub platforms:    Vec<Platform>,
    pub bounds1:      Point,
    pub bounds2:      Point,
    pub spawn_points: Vec<Point>,
}

impl Stage {
    pub fn base() -> Stage {
        Stage {
            title:        "Base Stage".to_string(),
            platforms:    Vec::new(),
            bounds1:      Point {x:-200.0, y:-200.0},
            bounds2:      Point {x: 200.0, y: 200.0},
            spawn_points: vec!(Point{x: -50.0, y: 50.0}, Point{x: 50.0, y: 50.0}),
        }
    }
}
