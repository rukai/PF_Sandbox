use ::geometry::Rect;
use ::os_input::OsInput;

use treeflection::{Node, NodeRunner, NodeToken, ContextVec};
use winit::VirtualKeyCode;

#[derive(Clone, Serialize, Deserialize, Node)]
pub struct Stage {
    pub name:           String,
    pub platforms:      ContextVec<Platform>,
    pub blast:          Rect,
    pub camera:         Rect,
    pub spawn_points:   ContextVec<SpawnPoint>,
    pub respawn_points: ContextVec<SpawnPoint>,
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

        let blast = Rect {
            x1: -200.0,
            x2: 200.0,
            y1:  -200.0,
            y2:   200.0,
        };

        let camera = Rect {
            x1: -150.0,
            x2: 150.0,
            y1:  -150.0,
            y2:   150.0,
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

impl Stage {
    /// return indexes to the floors connected to the passed floor
    pub fn connected_floors(&self, platform_i: usize) -> FloorInfo {
        let mut left_i = None;
        let mut right_i = None;
        if let Some(plat) = self.platforms.get(platform_i) {
            let (l_x, l_y) = plat.left_ledge();
            let (r_x, r_y) = plat.right_ledge();
            for (check_i, check_plat) in self.platforms.iter().enumerate() {
                if platform_i != check_i/* && check_plat.is_floor() TODO */ {
                    let (check_l_x, check_l_y) = check_plat.left_ledge();
                    let (check_r_x, check_r_y) = check_plat.right_ledge();

                    if l_x == check_r_x && l_y == check_r_y {
                        left_i = Some(check_i);
                    }
                    if r_x == check_l_x && r_y == check_l_y {
                        right_i = Some(check_i);
                    }
                }
            }
        }

        FloorInfo {
            left_i,
            right_i,
        }
    }
}

pub struct FloorInfo {
    pub left_i:  Option<usize>,
    pub right_i: Option<usize>,
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
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Platform {
        Platform {
            x1,
            y1,
            x2,
            y2,
            grab1:        false,
            grab2:        false,
            traction:     1.0,
            pass_through: true,
        }
    }

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
            (self.x2 - self.x1).abs() / 2.0
        } else if world_x < self.x1 && world_x < self.x2 {
            (self.x2 - self.x1).abs() / -2.0
        } else {
            world_x - (self.x1 + self.x2) / 2.0
        }
    }

    pub fn plat_x_clamp(&self, plat_x: f32) -> f32 {
        let h_w = (self.x1 - self.x2).abs() / 2.0;
        if plat_x > 0.0 {
            plat_x.min(h_w)
        } else {
            plat_x.max(-h_w)
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
pub struct SpawnPoint {
    pub x:          f32,
    pub y:          f32,
    pub face_right: bool,
}

impl SpawnPoint {
    pub fn new(x: f32, y: f32) -> SpawnPoint {
        SpawnPoint {
            x,
            y,
            face_right: true
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct DebugStage {
    pub blast:          bool,
    pub camera:         bool,
    pub spawn_points:   bool,
    pub respawn_points: bool,
}

impl DebugStage {
    pub fn step(&mut self, os_input: &OsInput) {
        if os_input.key_pressed(VirtualKeyCode::F1) {
            self.blast = !self.blast;
        }
        if os_input.key_pressed(VirtualKeyCode::F2) {
            self.camera = !self.camera;
        }
        if os_input.key_pressed(VirtualKeyCode::F3) {
            self.spawn_points = !self.spawn_points;
        }
        if os_input.key_pressed(VirtualKeyCode::F4) {
            self.respawn_points = !self.respawn_points;
        }
        if os_input.key_pressed(VirtualKeyCode::F11) {
            *self = DebugStage {
                blast:          true,
                camera:         true,
                spawn_points:   true,
                respawn_points: true,
            }
        }
        if os_input.key_pressed(VirtualKeyCode::F12) {
            *self = DebugStage::default();
        }
    }
}
