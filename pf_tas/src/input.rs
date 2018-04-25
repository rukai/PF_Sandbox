use winit::ElementState::{Pressed, Released};
use winit::{Event, VirtualKeyCode, EventsLoop, WindowEvent};

struct CurrentInput {
    pub key_actions:      Vec<KeyAction>,
    pub key_held:         [bool; 255],
    pub scroll_diff:      f32,
    pub resolution:       (u32, u32),
}

impl CurrentInput {
    pub fn new() -> CurrentInput {
        CurrentInput {
            key_actions:      vec!(),
            key_held:         [false; 255],
            scroll_diff:      0.0,
            resolution:       (1, 1),
        }
    }

    pub fn update(&mut self) {
        self.key_actions      = vec!();
        self.scroll_diff      = 0.0;
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
            WindowEvent::Resized (x, y) => {
                self.resolution = (x, y);
            }
            _ => {},
        }
    }
}

pub struct Input {
    events_loop: EventsLoop,
    current:     Option<CurrentInput>,
    quit:        bool,
}

impl Input {
    pub fn new(events_loop: EventsLoop) -> Input {
        Input {
            events_loop: events_loop,
            current: Some(CurrentInput::new()),
            quit:    false,
        }
    }

    /// Called every frame
    pub fn update(&mut self) {
        if let Some(ref mut current) = self.current {
            current.update();
        }

        let mut events: Vec<WindowEvent> = vec!();
        self.events_loop.poll_events(|event| {
            if let Event::WindowEvent { event, .. } = event {
                events.push(event);
            };
        });

        for event in events {
            match event {
                WindowEvent::CloseRequested |
                WindowEvent::Destroyed      => { self.quit = true }
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

    /// on
    pub fn key_held(&self, key_code: VirtualKeyCode) -> bool {
        match self.current {
            Some (ref current) => { current.key_held[key_code as usize] },
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
