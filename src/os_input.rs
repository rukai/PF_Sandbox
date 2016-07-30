use camera::Camera;

use glium::glutin::ElementState::{Pressed, Released};
use glium::glutin::{Event, MouseScrollDelta, VirtualKeyCode, MouseButton};
use std::sync::mpsc::{Sender, Receiver, channel};

struct CurrentInput {
    pub mouse_actions:    Vec<MouseAction>,
    pub key_actions:      Vec<KeyAction>,
    pub key_held:         [bool; 255],
    pub mouse_held:       [bool; 255],
    pub mouse_point:      Option<(i32, i32)>,
    pub mouse_point_prev: Option<(i32, i32)>,
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
            resolution:       (0, 0),
        }
    }

    pub fn update(&mut self) {
        self.mouse_actions    = vec!();
        self.key_actions      = vec!();
        self.scroll_diff      = 0.0;
        self.mouse_point_prev = None;
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::Closed => {
                self.key_actions.push(KeyAction::Pressed(VirtualKeyCode::Escape));
            },
            Event::KeyboardInput (Pressed, _, Some(key_code)) => {
                self.key_held[key_code as usize] = true;
                self.key_actions.push(KeyAction::Pressed(key_code));
            },
            Event::KeyboardInput (Released, _, Some(key_code)) => {
                self.key_held[key_code as usize] = false;
                self.key_actions.push(KeyAction::Released(key_code));
            },
            Event::MouseMoved (x, y) => {
                self.mouse_point_prev = self.mouse_point;
                self.mouse_point = Some((x, y));
            },
            Event::MouseInput (Pressed, button) => {
                let button = mouse_button_to_int(button);
                self.mouse_held[button] = true;
                self.mouse_actions.push(MouseAction::Pressed(button));
            },
            Event::MouseInput (Released, button) => {
                let button = mouse_button_to_int(button);
                self.mouse_held[button] = false;
                self.mouse_actions.push(MouseAction::Released(button));
            },
            Event::MouseWheel (sub_event, _) => {
                match sub_event {
                    MouseScrollDelta::LineDelta  (_, y) => { self.scroll_diff += y; },
                    MouseScrollDelta::PixelDelta (_, _) => { panic!("Ooer, I dont know how to handle PixelDelta...") }, // TODO
                }
            },
            Event::Resized (x, y) => {
                self.resolution = (x, y);
            }
            _ => {},
        }
    }

    /// Convert a mouse point to the corresponding in game point
    pub fn mouse_to_game(&self, mouse_point: (i32, i32), camera: &Camera) -> (f32, f32) {
        let (m_x, m_y) = mouse_point;
        let (m_x, m_y) = (m_x as f32, m_y as f32);
        let (w, h) = self.resolution;
        let (w, h) = (w as f32, h as f32);

        let zoom = camera.zoom as f32;
        let (pan_x, pan_y) = camera.pan;
        let (pan_x, pan_y) = (pan_x as f32, pan_y as f32);

        let (width, height) = self.resolution;
        let aspect_ratio = width as f32 / height as f32;

        let x = zoom * ( 2.0 * m_x / w - 1.0)                - pan_x;
        let y = zoom * (-2.0 * m_y / h + 1.0) / aspect_ratio - pan_y;
        (x, y)
    }
}

pub struct OsInput {
    current: Option<CurrentInput>,
    rx: Receiver<Event>,
}

impl OsInput {
    pub fn new() -> (OsInput, Sender<Event>) {
        let (tx, rx) = channel();
        let os_input = OsInput {
            current: Some(CurrentInput::new()),
            rx: rx,
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
                Event::Focused(false) => { self.current = None; },
                Event::Focused(true)  => { self.current = Some(CurrentInput::new()); },
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

    pub fn scroll_diff(&self) -> f32 {
        match self.current {
            Some( ref current) => { current.scroll_diff },
            None               => { 0.0 }
        }
    }

    pub fn mouse(&self) -> Option<(i32, i32)> {
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

    pub fn mouse_diff(&self) -> (i32, i32) {
        if let Some(ref current_input) = self.current {
            if let Some(cur) = current_input.mouse_point {
                if let Some(prev) = current_input.mouse_point_prev {
                    return (cur.0 - prev.0, cur.1 - prev.1);
                }
            }
        }
        (0, 0)
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
