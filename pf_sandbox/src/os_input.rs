use camera::Camera;

use winit::ElementState::{Pressed, Released};
use winit::{WindowEvent, MouseScrollDelta, MouseButton, KeyboardInput, VirtualKeyCode};
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
    pub text:             Vec<TextChar>,
}

#[derive(Clone)]
pub enum TextChar {
    Char (char),
    Back,
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
            text:             vec!(),
        }
    }

    pub fn update(&mut self) {
        self.mouse_actions    = vec!();
        self.key_actions      = vec!();
        self.scroll_diff      = 0.0;
        self.mouse_point_prev = self.mouse_point;
        self.text.clear();
    }

    pub fn handle_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(keycode) = input.virtual_keycode {
                    match input.state {
                        Pressed => {
                            self.key_held[keycode as usize] = true;
                            self.key_actions.push(KeyAction::Pressed(keycode));
                            self.push_text(input);
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

    fn push_text(&mut self, input: KeyboardInput) {
        if input.modifiers.shift {
            match input.virtual_keycode {
                Some(VirtualKeyCode::A) => self.text.push(TextChar::Char('A')),
                Some(VirtualKeyCode::B) => self.text.push(TextChar::Char('B')),
                Some(VirtualKeyCode::C) => self.text.push(TextChar::Char('C')),
                Some(VirtualKeyCode::D) => self.text.push(TextChar::Char('D')),
                Some(VirtualKeyCode::E) => self.text.push(TextChar::Char('E')),
                Some(VirtualKeyCode::F) => self.text.push(TextChar::Char('F')),
                Some(VirtualKeyCode::G) => self.text.push(TextChar::Char('G')),
                Some(VirtualKeyCode::H) => self.text.push(TextChar::Char('H')),
                Some(VirtualKeyCode::I) => self.text.push(TextChar::Char('I')),
                Some(VirtualKeyCode::J) => self.text.push(TextChar::Char('J')),
                Some(VirtualKeyCode::K) => self.text.push(TextChar::Char('K')),
                Some(VirtualKeyCode::L) => self.text.push(TextChar::Char('L')),
                Some(VirtualKeyCode::M) => self.text.push(TextChar::Char('M')),
                Some(VirtualKeyCode::N) => self.text.push(TextChar::Char('N')),
                Some(VirtualKeyCode::O) => self.text.push(TextChar::Char('O')),
                Some(VirtualKeyCode::P) => self.text.push(TextChar::Char('P')),
                Some(VirtualKeyCode::Q) => self.text.push(TextChar::Char('Q')),
                Some(VirtualKeyCode::R) => self.text.push(TextChar::Char('R')),
                Some(VirtualKeyCode::S) => self.text.push(TextChar::Char('S')),
                Some(VirtualKeyCode::T) => self.text.push(TextChar::Char('T')),
                Some(VirtualKeyCode::U) => self.text.push(TextChar::Char('U')),
                Some(VirtualKeyCode::V) => self.text.push(TextChar::Char('V')),
                Some(VirtualKeyCode::W) => self.text.push(TextChar::Char('W')),
                Some(VirtualKeyCode::X) => self.text.push(TextChar::Char('X')),
                Some(VirtualKeyCode::Y) => self.text.push(TextChar::Char('Y')),
                Some(VirtualKeyCode::Z) => self.text.push(TextChar::Char('Z')),
                Some(VirtualKeyCode::Key1)       => self.text.push(TextChar::Char('!')),
                Some(VirtualKeyCode::Key2)       => self.text.push(TextChar::Char('@')),
                Some(VirtualKeyCode::Key3)       => self.text.push(TextChar::Char('#')),
                Some(VirtualKeyCode::Key4)       => self.text.push(TextChar::Char('$')),
                Some(VirtualKeyCode::Key5)       => self.text.push(TextChar::Char('%')),
                Some(VirtualKeyCode::Key6)       => self.text.push(TextChar::Char('^')),
                Some(VirtualKeyCode::Key7)       => self.text.push(TextChar::Char('&')),
                Some(VirtualKeyCode::Key8)       => self.text.push(TextChar::Char('*')),
                Some(VirtualKeyCode::Key9)       => self.text.push(TextChar::Char('(')),
                Some(VirtualKeyCode::Key0)       => self.text.push(TextChar::Char(')')),
                Some(VirtualKeyCode::Period)     => self.text.push(TextChar::Char('<')),
                Some(VirtualKeyCode::Comma)      => self.text.push(TextChar::Char('>')),
                Some(VirtualKeyCode::Semicolon)  => self.text.push(TextChar::Char(':')),
                Some(VirtualKeyCode::LBracket)   => self.text.push(TextChar::Char('{')),
                Some(VirtualKeyCode::RBracket)   => self.text.push(TextChar::Char('}')),
                Some(VirtualKeyCode::Apostrophe) => self.text.push(TextChar::Char('"')),
                Some(VirtualKeyCode::Subtract)   => self.text.push(TextChar::Char('_')),
                Some(VirtualKeyCode::Equals)     => self.text.push(TextChar::Char('+')),
                Some(VirtualKeyCode::Slash)      => self.text.push(TextChar::Char('?')),
                Some(VirtualKeyCode::Backslash)  => self.text.push(TextChar::Char('|')),
                Some(VirtualKeyCode::Grave)      => self.text.push(TextChar::Char('~')),
                Some(VirtualKeyCode::Space)      => self.text.push(TextChar::Char(' ')),
                Some(VirtualKeyCode::Back)       => self.text.push(TextChar::Back),
                _ => { }
            }
        } else {
            match input.virtual_keycode {
                Some(VirtualKeyCode::A) => self.text.push(TextChar::Char('a')),
                Some(VirtualKeyCode::B) => self.text.push(TextChar::Char('b')),
                Some(VirtualKeyCode::C) => self.text.push(TextChar::Char('c')),
                Some(VirtualKeyCode::D) => self.text.push(TextChar::Char('d')),
                Some(VirtualKeyCode::E) => self.text.push(TextChar::Char('e')),
                Some(VirtualKeyCode::F) => self.text.push(TextChar::Char('f')),
                Some(VirtualKeyCode::G) => self.text.push(TextChar::Char('g')),
                Some(VirtualKeyCode::H) => self.text.push(TextChar::Char('h')),
                Some(VirtualKeyCode::I) => self.text.push(TextChar::Char('i')),
                Some(VirtualKeyCode::J) => self.text.push(TextChar::Char('j')),
                Some(VirtualKeyCode::K) => self.text.push(TextChar::Char('k')),
                Some(VirtualKeyCode::L) => self.text.push(TextChar::Char('l')),
                Some(VirtualKeyCode::M) => self.text.push(TextChar::Char('m')),
                Some(VirtualKeyCode::N) => self.text.push(TextChar::Char('n')),
                Some(VirtualKeyCode::O) => self.text.push(TextChar::Char('o')),
                Some(VirtualKeyCode::P) => self.text.push(TextChar::Char('p')),
                Some(VirtualKeyCode::Q) => self.text.push(TextChar::Char('q')),
                Some(VirtualKeyCode::R) => self.text.push(TextChar::Char('r')),
                Some(VirtualKeyCode::S) => self.text.push(TextChar::Char('s')),
                Some(VirtualKeyCode::T) => self.text.push(TextChar::Char('t')),
                Some(VirtualKeyCode::U) => self.text.push(TextChar::Char('u')),
                Some(VirtualKeyCode::V) => self.text.push(TextChar::Char('v')),
                Some(VirtualKeyCode::W) => self.text.push(TextChar::Char('w')),
                Some(VirtualKeyCode::X) => self.text.push(TextChar::Char('x')),
                Some(VirtualKeyCode::Y) => self.text.push(TextChar::Char('y')),
                Some(VirtualKeyCode::Z) => self.text.push(TextChar::Char('z')),
                Some(VirtualKeyCode::Key1)       => self.text.push(TextChar::Char('1')),
                Some(VirtualKeyCode::Key2)       => self.text.push(TextChar::Char('2')),
                Some(VirtualKeyCode::Key3)       => self.text.push(TextChar::Char('3')),
                Some(VirtualKeyCode::Key4)       => self.text.push(TextChar::Char('4')),
                Some(VirtualKeyCode::Key5)       => self.text.push(TextChar::Char('5')),
                Some(VirtualKeyCode::Key6)       => self.text.push(TextChar::Char('6')),
                Some(VirtualKeyCode::Key7)       => self.text.push(TextChar::Char('7')),
                Some(VirtualKeyCode::Key8)       => self.text.push(TextChar::Char('8')),
                Some(VirtualKeyCode::Key9)       => self.text.push(TextChar::Char('9')),
                Some(VirtualKeyCode::Key0)       => self.text.push(TextChar::Char('0')),
                Some(VirtualKeyCode::Period)     => self.text.push(TextChar::Char('.')),
                Some(VirtualKeyCode::Comma)      => self.text.push(TextChar::Char(',')),
                Some(VirtualKeyCode::Semicolon)  => self.text.push(TextChar::Char(';')),
                Some(VirtualKeyCode::LBracket)   => self.text.push(TextChar::Char('[')),
                Some(VirtualKeyCode::RBracket)   => self.text.push(TextChar::Char(']')),
                Some(VirtualKeyCode::Apostrophe) => self.text.push(TextChar::Char('\'')),
                Some(VirtualKeyCode::Subtract)   => self.text.push(TextChar::Char('-')),
                Some(VirtualKeyCode::Equals)     => self.text.push(TextChar::Char('=')),
                Some(VirtualKeyCode::Slash)      => self.text.push(TextChar::Char('/')),
                Some(VirtualKeyCode::Backslash)  => self.text.push(TextChar::Char('\\')),
                Some(VirtualKeyCode::Grave)      => self.text.push(TextChar::Char('`')),
                Some(VirtualKeyCode::Space)      => self.text.push(TextChar::Char(' ')),
                Some(VirtualKeyCode::Back)       => self.text.push(TextChar::Back),
                _ => { }
            }
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
            Some(ref current) => Some(current.resolution),
            None              => None
        }
    }

    pub fn text(&self) -> Vec<TextChar> {
        match self.current {
            Some(ref current) => current.text.clone(),
            None              => vec!()
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
