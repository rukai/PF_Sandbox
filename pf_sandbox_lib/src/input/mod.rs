pub mod maps;

use std::ops::Index;
use std::f32;

use gilrs_core::{EvCode, Gamepad, EventType};
use uuid::Uuid;
use treeflection::{Node, NodeRunner, NodeToken};

use self::maps::{ControllerMap, AnalogFilter, AnalogDest, DigitalFilter, DigitalDest};

/// Stores the first value returned from an input source
pub struct Deadzone {
    pub plugged_in: bool,
    pub stick_x:    u8,
    pub stick_y:    u8,
    pub c_stick_x:  u8,
    pub c_stick_y:  u8,
    pub l_trigger:  u8,
    pub r_trigger:  u8,
}

impl Deadzone {
    pub fn empty() -> Self {
        Deadzone {
            plugged_in: false,
            stick_x:    0,
            stick_y:    0,
            c_stick_x:  0,
            c_stick_y:  0,
            l_trigger:  0,
            r_trigger:  0,
        }
    }

    pub fn empty4() -> [Self; 4] {
        [
            Deadzone::empty(),
            Deadzone::empty(),
            Deadzone::empty(),
            Deadzone::empty()
        ]
    }
}

impl ControllerInput {
    pub fn empty() -> ControllerInput {
        ControllerInput {
            plugged_in: false,

            up:    false,
            down:  false,
            right: false,
            left:  false,
            y:     false,
            x:     false,
            b:     false,
            a:     false,
            l:     false,
            r:     false,
            z:     false,
            start: false,

            stick_x:   0.0,
            stick_y:   0.0,
            c_stick_x: 0.0,
            c_stick_y: 0.0,
            l_trigger: 0.0,
            r_trigger: 0.0,
        }
    }

    pub fn stick_angle(&self) -> Option<f32> {
        if self.stick_x == 0.0 && self.stick_y == 0.0 {
            None
        } else {
            Some(self.stick_y.atan2(self.stick_x))
        }
    }

    #[allow(dead_code)]
    pub fn c_stick_angle(&self) -> Option<f32> {
        if self.stick_x == 0.0 && self.stick_y == 0.0 {
            None
        } else {
            Some(self.c_stick_y.atan2(self.c_stick_x))
        }
    }
}

impl PlayerInput {
    pub fn empty() -> PlayerInput {
        PlayerInput {
            plugged_in: false,

            up:    Button { value: false, press: false },
            down:  Button { value: false, press: false },
            right: Button { value: false, press: false },
            left:  Button { value: false, press: false },
            y:     Button { value: false, press: false },
            x:     Button { value: false, press: false },
            b:     Button { value: false, press: false },
            a:     Button { value: false, press: false },
            l:     Button { value: false, press: false },
            r:     Button { value: false, press: false },
            z:     Button { value: false, press: false },
            start: Button { value: false, press: false },

            stick_x:   Stick { value: 0.0, diff: 0.0 },
            stick_y:   Stick { value: 0.0, diff: 0.0 },
            c_stick_x: Stick { value: 0.0, diff: 0.0 },
            c_stick_y: Stick { value: 0.0, diff: 0.0 },

            l_trigger:  Trigger { value: 0.0, diff: 0.0 },
            r_trigger:  Trigger { value: 0.0, diff: 0.0 },
            history: vec!(ControllerInput::empty(); 8),
        }
    }
}

// gilrs returns the code as a u32 in the following formats
// Linux:
// *   16 bytes - kind
// *   16 bytes - code
// Windows:
// *   24 bytes - padding
// *   8 bytes  - code

// On linux we only need the code so we strip out the kind, so the numbers are nicer to work with (when creating maps)
pub fn code_to_usize(code: &EvCode) -> usize {
    (code.into_u32() & 0xFFFF) as usize
}

/// Add a single controller to inputs, reading from the passed gamepad
pub fn read_generic(controller_maps: &[ControllerMap], state: &mut ControllerInput, events: Vec<EventType>, gamepad: &Gamepad, deadzone: &mut Deadzone) -> ControllerInput {
    let mut controller_map_use = None;
    for controller_map in controller_maps {
        if controller_map.name == gamepad.name() && controller_map.uuid == Uuid::from_bytes(gamepad.uuid()) {
            controller_map_use = Some(controller_map);
        }
    }

    if let Some(controller_map) = controller_map_use {
        // update internal state
        for event in events {
            match event {
                // TODO: better handle multiple sources pointing to the same destination
                // maybe keep a unique ControllerInput state for each source input
                EventType::ButtonPressed (code) => {
                    for map in &controller_map.analog_maps {
                        if let AnalogFilter::FromDigital { value } = map.filter {
                            if map.source == code_to_usize(&code) {
                                state.set_analog_dest(map.dest.clone(), value);
                            }
                        }
                    }

                    for map in &controller_map.digital_maps {
                        if let DigitalFilter::FromDigital = map.filter {
                            if map.source == code_to_usize(&code) {
                                state.set_digital_dest(map.dest.clone(), true);
                            }
                        };
                    }
                }
                EventType::ButtonReleased (code) => {
                    for map in &controller_map.analog_maps {
                        if let AnalogFilter::FromDigital { .. } = map.filter {
                            if map.source == code_to_usize(&code) {
                                state.set_analog_dest(map.dest.clone(), 0.0);
                            }
                        }
                    }

                    for map in &controller_map.digital_maps {
                        if let DigitalFilter::FromDigital = map.filter {
                            if map.source == code_to_usize(&code) {
                                state.set_digital_dest(map.dest.clone(), false);
                            }
                        };
                    }
                }
                EventType::AxisValueChanged (value, code) => {
                    for map in &controller_map.analog_maps {
                        if let AnalogFilter::FromAnalog { min, max, flip } = map.filter {
                            // Implemented as per https://stackoverflow.com/questions/345187/math-mapping-numbers
                            let mut new_value = ((value-min) as f32) / ((max-min) as f32) * 2.0 - 1.0;

                            new_value *= if flip { -1.0 } else { 1.0 };

                            match &map.dest {
                                &AnalogDest::LTrigger | &AnalogDest::RTrigger => {
                                    new_value = (new_value + 1.0) / 2.0;
                                }
                                _ => { }
                            }

                            if map.source == code_to_usize(&code) {
                                state.set_analog_dest(map.dest.clone(), new_value);
                            }
                        };
                    }

                    for map in &controller_map.digital_maps {
                        if let DigitalFilter::FromAnalog { min, max } = map.filter {
                            let value = value >= min && value <= max;

                            if map.source == code_to_usize(&code) {
                                state.set_digital_dest(map.dest.clone(), value);
                            }
                        };
                    }
                }
                EventType::Connected => {
                    state.plugged_in = true;
                }
                EventType::Disconnected => {
                    state.plugged_in = false;
                }
            }
        }

        // convert state floats to bytes
        let raw_stick_x   = generic_to_byte(state.stick_x);
        let raw_stick_y   = generic_to_byte(state.stick_y);
        let raw_c_stick_x = generic_to_byte(state.c_stick_x);
        let raw_c_stick_y = generic_to_byte(state.c_stick_y);

        let raw_l_trigger = generic_to_byte(state.l_trigger);
        let raw_r_trigger = generic_to_byte(state.r_trigger);

        // update deadzones
        if state.plugged_in && !deadzone.plugged_in { // Only reset deadzone if controller was just plugged in
            *deadzone = Deadzone {
                plugged_in: true,
                stick_x:    raw_stick_x,
                stick_y:    raw_stick_y,
                c_stick_x:  raw_c_stick_x,
                c_stick_y:  raw_c_stick_y,
                l_trigger:  raw_l_trigger,
                r_trigger:  raw_r_trigger,
            };
        }
        if !state.plugged_in {
            *deadzone = Deadzone::empty();
        }

        // convert bytes to result floats
        let (stick_x, stick_y)     = stick_filter(stick_deadzone(raw_stick_x,   deadzone.stick_x),   stick_deadzone(raw_stick_y,   deadzone.stick_y));
        let (c_stick_x, c_stick_y) = stick_filter(stick_deadzone(raw_c_stick_x, deadzone.c_stick_x), stick_deadzone(raw_c_stick_y, deadzone.c_stick_y));

        let l_trigger = trigger_filter(raw_l_trigger.saturating_sub(deadzone.l_trigger));
        let r_trigger = trigger_filter(raw_r_trigger.saturating_sub(deadzone.r_trigger));

        ControllerInput {
            stick_x,
            stick_y,
            c_stick_x,
            c_stick_y,
            l_trigger,
            r_trigger,
            ..state.clone()
        }
    } else {
        ControllerInput::default()
    }
}

fn generic_to_byte(value: f32) -> u8 {
    (value.min(1.0).max(-1.0) * 127.0 + 127.0) as u8
}

/// use the first received stick value to reposition the current stick value around 128
pub fn stick_deadzone(current: u8, first: u8) -> u8 {
    if current > first {
        128u8.saturating_add(current - first)
    } else {
        128u8.saturating_sub(first - current)
    }
}

fn abs_min(a: f32, b: f32) -> f32 {
    if (a >= 0.0 && a > b) || (a <= 0.0 && a < b) {
        b
    } else {
        a
    }
}

pub fn stick_filter(in_stick_x: u8, in_stick_y: u8) -> (f32, f32) {
    let raw_stick_x = in_stick_x as f32 - 128.0;
    let raw_stick_y = in_stick_y as f32 - 128.0;
    let angle = (raw_stick_y).atan2(raw_stick_x);

    let max_x = (angle.cos() * 80.0).trunc();
    let max_y = (angle.sin() * 80.0).trunc();
    let stick_x = if in_stick_x == 128 { // avoid raw_stick_x = 0 and thus division by zero in the atan2)
        0.0
    } else {
        abs_min(raw_stick_x, max_x) / 80.0
    };
    let stick_y = abs_min(raw_stick_y, max_y) / 80.0;

    let deadzone = 0.28;
    (
        if stick_x.abs() < deadzone { 0.0 } else { stick_x },
        if stick_y.abs() < deadzone { 0.0 } else { stick_y }
    )
}

pub fn trigger_filter(trigger: u8) -> f32 {
    let value = (trigger as f32) / 140.0;
    if value > 1.0 {
        1.0
    } else {
        value
    }
}

/// Internal input storage
#[derive(Clone, Default, Serialize, Deserialize, Node)]
pub struct ControllerInput {
    pub plugged_in: bool,

    pub a:     bool,
    pub b:     bool,
    pub x:     bool,
    pub y:     bool,
    pub left:  bool,
    pub right: bool,
    pub down:  bool,
    pub up:    bool,
    pub start: bool,
    pub z:     bool,
    pub r:     bool,
    pub l:     bool,

    pub stick_x:   f32,
    pub stick_y:   f32,
    pub c_stick_x: f32,
    pub c_stick_y: f32,
    pub r_trigger: f32,
    pub l_trigger: f32,
}

impl ControllerInput {
    fn set_analog_dest(&mut self, analog_dest: AnalogDest, value: f32) {
        match analog_dest {
            AnalogDest::StickX   => { self.stick_x = value }
            AnalogDest::StickY   => { self.stick_y = value }
            AnalogDest::CStickX  => { self.c_stick_x = value }
            AnalogDest::CStickY  => { self.c_stick_y = value }
            AnalogDest::RTrigger => { self.l_trigger = value }
            AnalogDest::LTrigger => { self.r_trigger = value }
        }
    }

    fn set_digital_dest(&mut self, analog_dest: DigitalDest, value: bool) {
        match analog_dest {
            DigitalDest::A     => { self.a = value }
            DigitalDest::B     => { self.b = value }
            DigitalDest::X     => { self.x = value }
            DigitalDest::Y     => { self.y = value }
            DigitalDest::Left  => { self.left = value }
            DigitalDest::Right => { self.right = value }
            DigitalDest::Down  => { self.down = value }
            DigitalDest::Up    => { self.up = value }
            DigitalDest::Start => { self.start = value }
            DigitalDest::Z     => { self.z = value }
            DigitalDest::R     => { self.r = value }
            DigitalDest::L     => { self.l = value }
        }
    }
}

/// External data access
pub struct PlayerInput {
    pub plugged_in: bool,

    pub a:     Button,
    pub b:     Button,
    pub x:     Button,
    pub y:     Button,
    pub left:  Button,
    pub right: Button,
    pub down:  Button,
    pub up:    Button,
    pub start: Button,
    pub z:     Button,
    pub r:     Button,
    pub l:     Button,

    pub stick_x:   Stick,
    pub stick_y:   Stick,
    pub c_stick_x: Stick,
    pub c_stick_y: Stick,
    pub r_trigger:  Trigger,
    pub l_trigger:  Trigger,
    pub history: Vec<ControllerInput>, // guaranteed to contain 8 elements
}

impl Index<usize> for PlayerInput {
    type Output = ControllerInput;

    fn index(&self, index: usize) -> &ControllerInput {
        &self.history[index]
    }
}

// TODO: now that we have history we could remove the value from these, turning them into primitive values

pub struct Button {
    pub value: bool, // on
    pub press: bool, // off->on this frame
}

pub struct Stick {
    pub value: f32, // current.value
    pub diff:  f32, // current.value - previous.value
}

pub struct Trigger {
    pub value: f32, // current.value
    pub diff:  f32, // current.value - previous.value
}
