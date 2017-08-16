use treeflection::{Node, NodeRunner, NodeToken, ContextVec};

#[derive(Clone, Serialize, Deserialize, Node)]
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

impl Default for Stage {
    fn default() -> Stage {
        let main_platform = Platform {
            x1:           -75.0,
            y1:           0.0,
            grab1:        true,
            x2:           75.0,
            y2:           0.0,
            grab2:        false,
            traction:     1.0,
            pass_through: false,
        };

        let second_platform = Platform {
            x1:           25.0,
            y1:           50.0,
            grab1:        false,
            x2:           75.0,
            y2:           50.0,
            grab2:        false,
            traction:     1.0,
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
            SpawnPoint {
                x:          -50.0,
                y:          10.0,
                face_right: true,
            },
            SpawnPoint {
                x:          -25.0,
                y:          10.0,
                face_right: false,
            },
            SpawnPoint {
                x:          25.0,
                y:          10.0,
                face_right: true,
            },
            SpawnPoint {
                x:          50.0,
                y:          50.0,
                face_right: false,
            },
        ));

        Stage {
            name:           "Base Stage".to_string(),
            platforms:      ContextVec::from_vec(vec!(main_platform, second_platform)),
            blast:          blast,
            camera:         camera,
            spawn_points:   spawn_points.clone(),
            respawn_points: spawn_points,
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct Platform {
    pub x1:           f32,
    pub y1:           f32,
    pub grab1:        bool,
    pub x2:           f32,
    pub y2:           f32,
    pub grab2:        bool,
    pub traction:     f32,
    pub pass_through: bool,
}


/// plat_x/plat_y/plat_p is offset from the centre of the platform
/// world_x/world_y/world_p is world coordinates
impl Platform {
    pub fn angle(&self) -> f32 {
        (self.y1-self.y2).atan2(self.x1-self.x2)
    }

    pub fn plat_x_in_bounds(&self, plat_x: f32) -> bool {
        let world_x = self.plat_x_to_world_x(plat_x);
        if self.x1 < self.x2 {
            self.x1 <= world_x && world_x <= self.x2
        } else {
            self.x2 <= world_x && world_x <= self.x1
        }
    }

    pub fn plat_x_to_world_y(&self, plat_x: f32) -> f32 {
        // y - y1 = m(x - x1)
        (self.y2 - self.y1) / (self.x2 - self.x1) * (plat_x - self.x1) + self.y1
    }

    pub fn world_x_to_plat_x(&self, world_x: f32) -> f32 {
        world_x - (self.x1 + self.x2) / 2.0
    }

    /// Converts the world x value to be relative to the center of the platform
    /// If it goes beyond the range of the platform, it is clamped to the edges
    pub fn world_x_to_plat_x_clamp(&self, world_x: f32) -> f32 {
        if world_x > self.x1 && world_x > self.x2 {
            self.x1.max(self.x2)
        } else if world_x < self.x1 && world_x < self.x2 {
            self.x1.min(self.x2)
        } else {
            world_x - (self.x1 + self.x2) / 2.0
        }
    }

    pub fn plat_x_to_world_x(&self, plat_x: f32) -> f32 {
        (self.x1 + self.x2) / 2.0 + plat_x
    }

    pub fn plat_x_to_world_p(&self, plat_x: f32) -> (f32, f32) {
        let world_x = self.plat_x_to_world_x(plat_x);
        let world_y = self.plat_x_to_world_y(world_x);
        (world_x, world_y)
    }

    pub fn left_ledge(&self) -> (f32, f32) {
        if self.x1 < self.x2 {
            (self.x1, self.y1)
        } else {
            (self.x2, self.y2)
        }
    }

    pub fn left_grab(&self) -> bool {
        if self.x1 < self.x2 {
            self.grab1
        } else {
            self.grab2
        }
    }

    pub fn right_ledge(&self) -> (f32, f32) {
        if self.x1 > self.x2 {
            (self.x1, self.y1)
        } else {
            (self.x2, self.y2)
        }
    }

    pub fn right_grab(&self) -> bool {
        if self.x1 > self.x2 {
            self.grab1
        } else {
            self.grab2
        }
    }

    pub fn p1(&self) -> (f32, f32) {
        (self.x1, self.y1)
    }

    pub fn p2(&self) -> (f32, f32) {
        (self.x2, self.y2)
    }
}

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct Area {
    pub left:  f32,
    pub right: f32,
    pub bot:   f32,
    pub top:   f32,
}
