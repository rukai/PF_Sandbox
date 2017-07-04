use camera::Camera;

use winit::ElementState::{Pressed, Released};
use winit::{WindowEvent, MouseScrollDelta, VirtualKeyCode, MouseButton};
use std::sync::mpsc::{Sender, Receiver, channel};

struct CurrentInput {
    pub mouse_actions:    Vec<MouseAction>,
    pub key_actions:      Vec<KeyAction>,
    pub key_held:         [bool; 255],
    pub mouse_held:       [bool; 255],
    pub mouse_point:      Option<(f32, f32)>,
    pub mouse_point_prev: Option<(f32, f32)>,
    pub scroll_diff:      f32,
    pub resolution:       (u32, u32),
}

impl CurrentInput {
    pub fn new() -> CurrentInput {
        CurrentInput {
            mouse_actions:    vec!(),
            key_actions:      vec!(),
            key_held:         [false; 255],
            mouse_held:       [false; 255],
            mouse_point:      None,
            mouse_point_prev: None,
            scroll_diff:      0.0,
            resolution:       (1, 1),
        }
    }

    pub fn update(&mut self) {
        self.mouse_actions    = vec!();
        self.key_actions      = vec!();
        self.scroll_diff      = 0.0;
        self.mouse_point_prev = self.mouse_point;
    }

    pub fn handle_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(keycode) = input.virtual_keycode {
                    match input.state {
                        Pressed => {
                            self.key_held[keycode as usize] = true;
                            self.key_actions.push(KeyAction::Pressed(keycode));
                        }
                        Released => {
                            self.key_held[keycode as usize] = false;
                            self.key_actions.push(KeyAction::Released(keycode));
                        },
                    }
                }
            }
            WindowEvent::MouseMoved { position, .. } => {
                self.mouse_point = Some((position.0 as f32, position.1 as f32));
            },
            WindowEvent::MouseInput { state: Pressed, button, .. } => {
                let button = mouse_button_to_int(button);
                self.mouse_held[button] = true;
                self.mouse_actions.push(MouseAction::Pressed(button));
            },
            WindowEvent::MouseInput { state: Released, button, .. } => {
                let button = mouse_button_to_int(button);
                self.mouse_held[button] = false;
                self.mouse_actions.push(MouseAction::Released(button));
            },
            WindowEvent::MouseWheel { delta, .. } => {
                match delta {
                    MouseScrollDelta::LineDelta  (_, y) => { self.scroll_diff += y; },
                    MouseScrollDelta::PixelDelta (_, _) => { panic!("Ooer, I dont know how to handle PixelDelta...") }, // TODO
                }
            },
            WindowEvent::Resized (x, y) => {
                self.resolution = (x, y);
            }
            _ => {},
        }
    }

    /// Convert a mouse point to the corresponding in game point
    pub fn mouse_to_game(&self, mouse_point: (f32, f32), camera: &Camera) -> (f32, f32) {
        let (m_x, m_y) = mouse_point;
        let (w_w, w_h) = self.resolution;
        let (w_w, w_h) = (w_w as f32, w_h as f32);
        let aspect_ratio = w_w / w_h;

        let zoom = camera.zoom;
        let (pan_x, pan_y) = camera.pan;

        let g_x = zoom * ( 2.0 * m_x / w_w - 1.0)                - pan_x;
        let g_y = zoom * (-2.0 * m_y / w_h + 1.0) / aspect_ratio - pan_y;
        (g_x, g_y)
    }
}

pub struct OsInput {
    current: Option<CurrentInput>,
    quit:    bool,
    rx:      Receiver<WindowEvent>,
}

impl OsInput {
    pub fn new() -> (OsInput, Sender<WindowEvent>) {
        let (tx, rx) = channel();
        let os_input = OsInput {
            current: Some(CurrentInput::new()),
            quit:    false,
            rx:      rx,
        };
        (os_input, tx)
    }

    /// Called every frame
    pub fn update(&mut self) {
        if let Some(ref mut current) = self.current {
            current.update();
        }

        while let Ok(event) = self.rx.try_recv() {
            match event {
                WindowEvent::Closed         => { self.quit = true; },
                WindowEvent::Focused(false) => { self.current = None; },
                WindowEvent::Focused(true)  => { self.current = Some(CurrentInput::new()); },
                _ => { },
            }
            if let Some(ref mut current) = self.current {
                current.handle_event(event);
            }
        }
    }

    /// off->on
    pub fn key_pressed(&self, check_key_code: VirtualKeyCode) -> bool {
        if let Some(ref current) = self.current {
            for action in &current.key_actions {
                if let &KeyAction::Pressed(key_code) = action {
                    if key_code == check_key_code {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// off->on
    pub fn mouse_pressed(&self, check_mouse_button: usize) -> bool {
        if let Some(ref current) = self.current {
            for action in &current.mouse_actions {
                if let &MouseAction::Pressed(key_code) = action {
                    if key_code == check_mouse_button {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// on->off
    pub fn key_released(&self, check_key_code: VirtualKeyCode) -> bool {
        if let Some(ref current) = self.current {
            for action in &current.key_actions {
                if let &KeyAction::Released(key_code) = action {
                    if key_code == check_key_code {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// on->off
    pub fn mouse_released(&self, check_mouse_button: usize) -> bool {
        if let Some(ref current) = self.current {
            for action in &current.mouse_actions {
                if let &MouseAction::Released(key_code) = action {
                    if key_code == check_mouse_button {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// on
    pub fn key_held(&self, key_code: VirtualKeyCode) -> bool {
        match self.current {
            Some (ref current) => { current.key_held[key_code as usize] },
            None               => { false }
        }
    }

    /// on
    pub fn mouse_held(&self, mouse_button: usize) -> bool {
        match self.current {
            Some (ref current) => { current.mouse_held[mouse_button as usize] },
            None               => { false }
        }
    }

    pub fn held_shift(&self) -> bool {
        return self.key_held(VirtualKeyCode::LShift) || self.key_held(VirtualKeyCode::RShift);
    }

    pub fn held_control(&self) -> bool {
        return self.key_held(VirtualKeyCode::LControl) || self.key_held(VirtualKeyCode::RControl);
    }

    pub fn held_alt(&self) -> bool {
        return self.key_held(VirtualKeyCode::LAlt) || self.key_held(VirtualKeyCode::RAlt);
    }

    pub fn scroll_diff(&self) -> f32 {
        match self.current {
            Some( ref current) => { current.scroll_diff },
            None               => { 0.0 }
        }
    }

    pub fn mouse(&self) -> Option<(f32, f32)> {
        match self.current {
            Some(ref current) => { current.mouse_point },
            None              => { None },
        }
    }

    pub fn game_mouse(&self, camera: &Camera) -> Option<(f32, f32)> {
        if let Some(ref current) = self.current {
            if let Some(point) = current.mouse_point {
                return Some(current.mouse_to_game(point, camera));
            }
        }
        None
    }

    pub fn mouse_diff(&self) -> (f32, f32) {
        if let Some(ref current_input) = self.current {
            if let Some(cur) = current_input.mouse_point {
                if let Some(prev) = current_input.mouse_point_prev {
                    return (cur.0 - prev.0, cur.1 - prev.1);
                }
            }
        }
        (0.0, 0.0)
    }

    pub fn game_mouse_diff(&self, camera: &Camera) -> (f32, f32) {
        if let Some(ref current_input) = self.current {
            if let Some(cur) = current_input.mouse_point {
                if let Some(prev) = current_input.mouse_point_prev {
                    let cur  = current_input.mouse_to_game(cur, camera);
                    let prev = current_input.mouse_to_game(prev, camera);
                    return (cur.0 - prev.0, cur.1 - prev.1);
                }
            }
        }
        (0.0, 0.0)
    }

    pub fn resolution(&self) -> Option<(u32, u32)> {
        match self.current {
            Some(ref current) => { Some(current.resolution) },
            None              => { None }
        }
    }

    pub fn quit(&self) -> bool {
        self.quit
    }
}

pub enum KeyAction {
    Pressed  (VirtualKeyCode),
    Released (VirtualKeyCode),
}

pub enum MouseAction {
    Pressed (usize),
    Released (usize),
}

fn mouse_button_to_int(button: MouseButton) -> usize {
    match button {
        MouseButton::Left        => { 0 },
        MouseButton::Right       => { 1 },
        MouseButton::Middle      => { 2 },
        MouseButton::Other(byte) => { byte as usize },
    }
}
