#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct Stage {
    pub title:         String,
    pub platforms:     Vec<Platform>,
    pub lower_bounds:  (f32, f32),
    pub higher_bounds: (f32, f32),
    pub spawn_points:  Vec<(f32, f32)>,
}

impl Stage {
    pub fn base() -> Stage {
        let main_platform = Platform {
            x: 0.0,
            y: 0.0,
            w: 150.0,
            h: 10.0,
            pass_through: false,
        };

        let second_platform = Platform {
            x: 50.0,
            y: 50.0,
            w: 50.0,
            h: 2.5,
            pass_through: true,
        };

        Stage {
            title:         "Base Stage".to_string(),
            platforms:     vec!(main_platform, second_platform),
            lower_bounds:  (-200.0, -200.0),
            higher_bounds: (200.0,  200.0),
            spawn_points:  vec!(
                (-50.0, 50.0), (50.0, 50.0),
                (-50.0, 80.0), (50.0, 80.0),
            ),
        }
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct Platform {
    pub x:            f32,
    pub y:            f32,
    pub w:            f32,
    pub h:            f32,
    pub pass_through: bool,
}
