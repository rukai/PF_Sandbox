use treeflection::{Node, NodeRunner, NodeToken, ContextVec};

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct Stage {
    pub name:           String,
    pub platforms:      ContextVec<Platform>,
    pub blast:          Area,
    pub camera:         Area,
    pub spawn_points:   ContextVec<SpawnPoint>,
    pub respawn_points: ContextVec<SpawnPoint>,
}

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct SpawnPoint {
    pub x:          f32,
    pub y:          f32,
    pub face_right: bool,
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

        let blast = Area {
            left: -200.0,
            right: 200.0,
            bot:  -200.0,
            top:   200.0,
        };

        let camera = Area {
            left: -150.0,
            right: 150.0,
            bot:  -150.0,
            top:   150.0,
        };

        let spawn_points = ContextVec::from_vec(vec!(
            SpawnPoint{
                x:          -50.0,
                y:          10.0,
                face_right: true,
            },
            SpawnPoint{
                x:          -25.0,
                y:          10.0,
                face_right: false,
            },
            SpawnPoint{
                x:          25.0,
                y:          10.0,
                face_right: true,
            },
            SpawnPoint{
                x:          50.0,
                y:          50.0,
                face_right: false,
            },
        ));

        Stage {
            name:            "Base Stage".to_string(),
            platforms:       ContextVec::from_vec(vec!(main_platform, second_platform)),
            blast:           blast,
            camera:          camera,
            spawn_points:    spawn_points.clone(),
            respawn_points:  spawn_points,
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct Platform {
    pub x:            f32,
    pub y:            f32,
    pub w:            f32,
    pub h:            f32,
    pub pass_through: bool,
}

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct Area {
    pub left:  f32,
    pub right: f32,
    pub bot:   f32,
    pub top:   f32,
}
