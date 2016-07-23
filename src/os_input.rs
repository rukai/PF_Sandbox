use glium::glutin::Event;
use glium::glutin::ElementState::{Pressed, Released};
use glium::glutin::{VirtualKeyCode, MouseButton};
use std::sync::mpsc::{Sender, Receiver, channel};

struct CurrentInput {
    pub mouse_actions:  Vec<MouseAction>,
    pub key_actions:    Vec<KeyAction>,
    pub key_held:       [bool; 255],
    pub mouse_held:     [bool; 255],
    pub mouse_location: Option<(f32, f32)>,
    pub resolution:     (u32, u32),
}

impl CurrentInput {
    pub fn new() -> CurrentInput {
        CurrentInput {
            mouse_actions:  vec!(),
            key_actions:    vec!(),
            key_held:       [false; 255],
            mouse_held:     [false; 255],
            mouse_location: None,
            resolution:     (0, 0),
        }
    }

    pub fn update(&mut self) {
        self.mouse_actions = vec!();
        self.key_actions   = vec!();
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
                //TODO replace * 200.0 and - 100.0 with camera zoom and account for camera offset
                let x = (200.0 * (x as f32)) / (self.resolution.0 as f32) - 100.0;
                let y = (-200.0 * (y as f32)) / (self.resolution.1 as f32) + 100.0;
                self.mouse_location = Some((x, y));
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
            Event::Resized (x, y) => {
                self.resolution = (x, y);
            }
            _ => {},
        }
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

    pub fn mouse(&self) -> Option<(f32, f32)> {
        match self.current {
            Some(ref current) => { current.mouse_location },
            None => { None },
        }
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
