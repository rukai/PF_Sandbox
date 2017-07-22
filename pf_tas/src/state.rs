use winit::VirtualKeyCode;

use controller::{Controller, ElementButton, ElementStick, ElementTrigger};
use input::Input;
use std::cmp;

#[derive(Serialize)]
pub enum NewGameState {
    None,
    Local,
    Paused,
    ReplayForwards,
    ReplayBackwards,
    StepThenPause,
    StepForwardThenPause,
    StepBackwardThenPause,
}

impl NewGameState {
    pub fn should_send(&self) -> bool {
        if let &NewGameState::None = self {
            false
        } else {
            true
        }
    }
}

pub struct State {
    pub controllers:             Vec<Controller>,
    pub current_controller:      usize,
    pub display_all_controllers: bool,
    pub use_aspect_ratio:        bool,
    pub display_analog_as_float: bool,
    pub touchtype:               bool,
    pub new_game_state:          NewGameState,
    number:                      NumberInput,
}

impl State {
    pub fn new() -> State {
        State {
            controllers:             vec!(Controller::new()),
            current_controller:      0,
            display_all_controllers: false,
            use_aspect_ratio:        false,
            display_analog_as_float: false,
            touchtype:               true,
            new_game_state:          NewGameState::None,
            number:                  NumberInput::new(),
        }
    }

    pub fn update(&mut self, input: &Input) {
        // toggle between display all controllers / display selected controller
        if input.key_pressed(VirtualKeyCode::Q) {
            self.display_all_controllers = !self.display_all_controllers;
        }

        // Toggle render graphics to aspect_ratio / stretch to fill
        if input.key_pressed(VirtualKeyCode::W) {
            self.use_aspect_ratio = !self.use_aspect_ratio;
        }

        // toggle display float values / byte values for sticks and triggers
        if input.key_pressed(VirtualKeyCode::E) {
            self.display_analog_as_float = !self.display_analog_as_float;
        }

        // toggle touch typing mode and 1-1 keybindings
        if input.key_pressed(VirtualKeyCode::R) {
            self.touchtype = !self.touchtype
        }

        // controller select
        if input.key_pressed(VirtualKeyCode::F1) && self.controllers.len() > 0 {
            self.current_controller = 0;
            println!("{}", self.number.pop_stick());
        }
        else if input.key_pressed(VirtualKeyCode::F2) && self.controllers.len() > 1 {
            self.current_controller = 1;
        }
        else if input.key_pressed(VirtualKeyCode::F3) && self.controllers.len() > 2 {
            self.current_controller = 2;
        }
        else if input.key_pressed(VirtualKeyCode::F4) && self.controllers.len() > 3 {
            self.current_controller = 3;
        }
        else if input.key_pressed(VirtualKeyCode::F5) && self.controllers.len() > 4 {
            self.current_controller = 4;
        }
        else if input.key_pressed(VirtualKeyCode::F6) && self.controllers.len() > 5 {
            self.current_controller = 5;
        }
        else if input.key_pressed(VirtualKeyCode::F7) && self.controllers.len() > 6 {
            self.current_controller = 6;
        }
        else if input.key_pressed(VirtualKeyCode::F8) && self.controllers.len() > 7 {
            self.current_controller = 7;
        }
        else if input.key_pressed(VirtualKeyCode::F9) && self.controllers.len() > 8 {
            self.current_controller = 8;
        }
        else if input.key_pressed(VirtualKeyCode::F10) && self.controllers.len() > 9 {
            self.current_controller = 9;
        }
        else if input.key_pressed(VirtualKeyCode::F11) && self.controllers.len() > 10 {
            self.current_controller = 10;
        }
        else if input.key_pressed(VirtualKeyCode::F12) && self.controllers.len() > 11 {
            self.current_controller = 11;
        }

        // add/remove controllers
        if input.key_pressed(VirtualKeyCode::LBracket) {
            if self.controllers.len() > 1 {
                self.controllers.pop();
            }
        }
        else if input.key_pressed(VirtualKeyCode::RBracket) {
            if self.controllers.len() < 9 {
                self.controllers.push(Controller::new());
            }
        }

        // number input
        if input.key_pressed(VirtualKeyCode::Key0) || input.key_pressed(VirtualKeyCode::Numpad0) {
            self.number.input(0);
        }
        else if input.key_pressed(VirtualKeyCode::Key1) || input.key_pressed(VirtualKeyCode::Numpad1) {
            self.number.input(1);
        }
        else if input.key_pressed(VirtualKeyCode::Key2) || input.key_pressed(VirtualKeyCode::Numpad2) {
            self.number.input(2);
        }
        else if input.key_pressed(VirtualKeyCode::Key3) || input.key_pressed(VirtualKeyCode::Numpad3) {
            self.number.input(3);
        }
        else if input.key_pressed(VirtualKeyCode::Key4) || input.key_pressed(VirtualKeyCode::Numpad4) {
            self.number.input(4);
        }
        else if input.key_pressed(VirtualKeyCode::Key5) || input.key_pressed(VirtualKeyCode::Numpad5) {
            self.number.input(5);
        }
        else if input.key_pressed(VirtualKeyCode::Key6) || input.key_pressed(VirtualKeyCode::Numpad6) {
            self.number.input(6);
        }
        else if input.key_pressed(VirtualKeyCode::Key7) || input.key_pressed(VirtualKeyCode::Numpad7) {
            self.number.input(7);
        }
        else if input.key_pressed(VirtualKeyCode::Key8) || input.key_pressed(VirtualKeyCode::Numpad8) {
            self.number.input(8);
        }
        else if input.key_pressed(VirtualKeyCode::Key9) || input.key_pressed(VirtualKeyCode::Numpad9) {
            self.number.input(9);
        }
        else if input.key_pressed(VirtualKeyCode::Subtract) {
            self.number.negative();
        }
        else if input.key_pressed(VirtualKeyCode::Equals) {
            self.number.positive();
        }

        // Key -> GC mapping
        let controller = &mut self.controllers[self.current_controller];

        State::map_button(input, VirtualKeyCode::Up,    &mut controller.up);
        State::map_button(input, VirtualKeyCode::Down,  &mut controller.down);
        State::map_button(input, VirtualKeyCode::Left,  &mut controller.left);
        State::map_button(input, VirtualKeyCode::Right, &mut controller.right);

        State::map_button(input, VirtualKeyCode::A, &mut controller.a);
        State::map_button(input, VirtualKeyCode::S, &mut controller.b);
        State::map_button(input, VirtualKeyCode::D, &mut controller.x);
        State::map_button(input, VirtualKeyCode::F, &mut controller.y);

        State::map_button(input, VirtualKeyCode::G, &mut controller.start);
        State::map_button(input, VirtualKeyCode::Y, &mut controller.z);

        State::map_button (input, VirtualKeyCode::N,     &mut controller.l);
        State::map_trigger(input, VirtualKeyCode::M,     &mut controller.l_trigger, &mut self.number);
        State::map_trigger(input, VirtualKeyCode::Comma, &mut controller.r_trigger, &mut self.number);
        State::map_button (input, VirtualKeyCode::Period, &mut controller.r);

        State::map_stick(input, VirtualKeyCode::U, &mut controller.stick_x,   &mut self.number);
        State::map_stick(input, VirtualKeyCode::I, &mut controller.stick_y,   &mut self.number);
        State::map_stick(input, VirtualKeyCode::O, &mut controller.c_stick_x, &mut self.number);
        State::map_stick(input, VirtualKeyCode::P, &mut controller.c_stick_y, &mut self.number);

        // Game flow
        if input.key_pressed(VirtualKeyCode::Return) {
            self.new_game_state = NewGameState::Local;
        }
        else if input.key_pressed(VirtualKeyCode::Space) {
            self.new_game_state = NewGameState::StepThenPause;
        }
        else if input.key_pressed(VirtualKeyCode::H) {
            self.new_game_state = NewGameState::ReplayBackwards;
        }
        else if input.key_pressed(VirtualKeyCode::J) {
            self.new_game_state = NewGameState::StepBackwardThenPause;
        }
        else if input.key_pressed(VirtualKeyCode::K) {
            self.new_game_state = NewGameState::StepForwardThenPause;
        }
        else if input.key_pressed(VirtualKeyCode::L) {
            self.new_game_state = NewGameState::ReplayForwards;
        }
        else {
            self.new_game_state = NewGameState::None;
        }
    }

    fn map_button(input: &Input, key: VirtualKeyCode, button: &mut ElementButton) {
        if input.key_pressed(key) {
            if input.held_shift() {
                button.hold();
            }
            else {
                button.press();
            }
        }
    }

    fn map_stick(input: &Input, key: VirtualKeyCode, button: &mut ElementStick, number: &mut NumberInput) {
        if input.key_pressed(key) {
            if input.held_shift() {
                button.hold(number.pop_stick());
            }
            else {
                button.press(number.pop_stick());
            }
        }
    }

    fn map_trigger(input: &Input, key: VirtualKeyCode, button: &mut ElementTrigger, number: &mut NumberInput) {
        if input.key_pressed(key) {
            if input.held_shift() {
                button.hold(number.pop_trigger());
            }
            else {
                button.press(number.pop_trigger());
            }
        }
    }
}

struct NumberInput {
    value:    u64,
    negative: bool,
}

impl NumberInput {
    pub fn new() -> NumberInput {
        NumberInput {
            value: 0,
            negative: false,
        }
    }

    // setters

    pub fn input(&mut self, input: u64) {
        self.value = self.value.saturating_mul(10).saturating_add(input);
    }

    pub fn negative(&mut self) {
        self.value = 0;
        self.negative = true;
    }

    pub fn positive(&mut self) {
        self.value = 0;
        self.negative = false;
    }

    // users

    pub fn pop_stick(&mut self) -> i8 {
        let value_i8 = cmp::min(self.value, i8::max_value() as u64) as i8;
        let value_i8 = (value_i8).saturating_mul(if self.negative { -1 } else { 1 });
        self.value = 0;
        self.negative = false;
        value_i8
    }

    pub fn pop_trigger(&mut self) -> u8 {
        let value_u8 = cmp::min(self.value, u8::max_value() as u64) as u8;
        self.value = 0;
        self.negative = false;
        value_u8
    }
}
