use os_input::OsInput;
use player::Player;
use stage::Stage;
use fighter::Fighter;

use winit::VirtualKeyCode;
use treeflection::{Node, NodeRunner, NodeToken, KeyedContextVec};

#[derive(Debug, Clone, Default, Serialize, Deserialize, Node)]
pub struct Camera {
    aspect_ratio: f32,
    pub zoom:     f32,
    pub pan:      (f32, f32),
    pub state:    CameraState,
}

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
pub enum CameraState {
    Manual,
    Auto,
}

impl Default for CameraState {
    fn default() -> CameraState {
        CameraState::Auto
    }
}

impl Camera {
    pub fn new() -> Camera {
        Camera {
            aspect_ratio: 1.0,
            zoom:         100.0,
            pan:          (0.0, 0.0),
            state:        CameraState::Auto,
        }
    }

    pub fn update_os_input(&mut self, os_input: &OsInput) {
        // set manual/automatic camera control
        if os_input.mouse_pressed(2) || os_input.scroll_diff() != 0.0 {
            self.state = CameraState::Manual;
        }
        else if os_input.key_pressed(VirtualKeyCode::Back) {
            self.state = CameraState::Auto;
        }

        if let CameraState::Manual = self.state {
            // pan camera
            if os_input.mouse_held(2) {
                let mouse_diff = os_input.mouse_diff();
                self.pan = (self.pan.0 + mouse_diff.0 as f32,
                            self.pan.1 - mouse_diff.1 as f32);
            }

            // zoom camera
            self.zoom = (self.zoom - os_input.scroll_diff() * 4.0).max(1.0);
        }
    }

    pub fn update(&mut self, os_input: &OsInput, players: &[Player], fighters: &KeyedContextVec<Fighter>, stage: &Stage) {
        if let CameraState::Auto = self.state {
            if let Some((width, height)) = os_input.resolution() {
                self.aspect_ratio = width as f32 / height as f32;
            }

            // initialise cam_area using only the first player
            let mut player_iter = players.iter();
            let mut cam_area = match player_iter.next() {
                Some(player) => {
                    player.cam_area(&stage.camera, players, fighters, &stage.surfaces)
                },
                None => {
                    self.pan = (0.0, 0.0);
                    self.zoom = 100.0;
                    return;
                }
            };

            // grow cam_area to cover all other players
            for player in player_iter {
                let next_area = player.cam_area(&stage.camera, players, fighters, &stage.surfaces);
                cam_area.x1 = cam_area.x1.min(next_area.left());
                cam_area.x2 = cam_area.x2.max(next_area.right());
                cam_area.y1 = cam_area.y1.min(next_area.bot());
                cam_area.y2 = cam_area.y2.max(next_area.top());
            }

            // grow cam_area to fill aspect ratio
            let mut width  = (cam_area.x1 - cam_area.x2).abs();
            let mut height = (cam_area.y1 - cam_area.y2).abs();
            if width / height > self.aspect_ratio {
                height = width / self.aspect_ratio;

                // TODO: push changes back so it doesnt go past the stage camera area
                let avg_vertical = (cam_area.y2 + cam_area.y1) / 2.0;
                cam_area.y2 = avg_vertical + height / 2.0;
                cam_area.y1 = avg_vertical - height / 2.0;
            }
            else {
                width = height * self.aspect_ratio;

                // TODO: push changes back so it doesnt go past the stage camera area
                let avg_horizontal = (cam_area.x2 + cam_area.x1) / 2.0;
                cam_area.x2 = avg_horizontal + width / 2.0;
                cam_area.x1 = avg_horizontal - width / 2.0;
            }

            // push aspect_ratio changes back so it doesnt go past the stage camera area
            let cam_max = &stage.camera;
            if cam_area.x1 < cam_max.left() {
                let diff = cam_area.x1 - cam_max.left();
                cam_area.x1 -= diff;
                cam_area.x2 -= diff;
            }
            else if cam_area.x2 > cam_max.right() {
                let diff = cam_area.x2 - cam_max.right();
                cam_area.x1 -= diff;
                cam_area.x2 -= diff;
            }
            if cam_area.y1 < cam_max.bot() {
                let diff = cam_area.y1 - cam_max.bot();
                cam_area.y1 -= diff;
                cam_area.y2 -= diff;
            }
            else if cam_area.y2 > cam_max.top() {
                let diff = cam_area.y2 - cam_max.top();
                cam_area.y1 -= diff;
                cam_area.y2 -= diff;
            }

            // set new camera values
            let dest_pan_x = -((cam_area.x1 + cam_area.x2) / 2.0);
            let dest_pan_y = -((cam_area.y1 + cam_area.y2) / 2.0);
            let dest_zoom = width / 2.0;

            let diff_pan_x = dest_pan_x - self.pan.0;
            let diff_pan_y = dest_pan_y - self.pan.1;
            let diff_zoom = dest_zoom - self.zoom;

            self.pan.0 += diff_pan_x / 10.0;
            self.pan.1 += diff_pan_y / 10.0;
            self.zoom += diff_zoom / 10.0;
        }
    }
}
