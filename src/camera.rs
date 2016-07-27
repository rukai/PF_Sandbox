use os_input::OsInput;
use glium::glutin::VirtualKeyCode;

#[derive(Debug, Clone)]
pub struct Camera {
    pub pan: (i32, i32),
    pub zoom:  f32,
    pub state: CameraState,
}

#[derive(Debug, Clone)]
pub enum CameraState {
    Manual,
    Auto,
}

impl Camera {
    pub fn new() -> Camera {
        Camera {
            pan:   (0, 0),
            zoom:  100.0,
            state: CameraState::Auto,
        }
    }

    pub fn update(&mut self, os_input: &OsInput) {
        match self.state {
            CameraState::Manual => {
                // pan camera
                if os_input.mouse_held(2) {
                    let mouse_diff = os_input.mouse_diff();
                    self.pan = (self.pan.0 + mouse_diff.0,
                                self.pan.1 - mouse_diff.1);
                }

                // zoom camera
                self.zoom = (self.zoom - os_input.scroll_diff() * 4.0).max(1.0);

                // enable automatic camera control
                if os_input.key_pressed(VirtualKeyCode::Back) {
                    self.state = CameraState::Auto;
                }
            },
            CameraState::Auto => {
                // TODO: replace with automatic camera control
                self.pan = (0, 0);
                self.zoom = 100.0;

                // enable manual camera control
                if os_input.mouse_pressed(2) || os_input.scroll_diff() != 0.0 {
                    self.state = CameraState::Manual;
                }
            },
        }
    }
}
