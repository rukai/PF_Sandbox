#[derive(Debug)]
pub enum ElementButton {
    None,
    Press,
    Hold,
}

#[derive(Debug)]
pub enum ElementStick {
    Press (i8),
    Hold (i8),
}

#[derive(Debug)]
pub enum ElementTrigger {
    Press (u8),
    Hold (u8),
}

impl ElementButton {
    pub fn hold(&mut self) {
        if let &mut ElementButton::None = self {
            *self = ElementButton::Hold;
        } else {
            *self = ElementButton::None;
        }
    }

    pub fn press(&mut self) {
        if let &mut ElementButton::None = self {
            *self = ElementButton::Press;
        } else {
            *self = ElementButton::None;
        }
    }

    pub fn get(&mut self) -> bool {
        match self {
            &mut ElementButton::None => { false }
            &mut ElementButton::Hold => { true }
            &mut ElementButton::Press => {
                *self = ElementButton::None;
                true
            }
        }
    }
}

impl ElementStick {
    pub fn hold(&mut self, value: i8) {
        *self = ElementStick::Hold(value);
    }

    pub fn press(&mut self, value: i8) {
        *self = ElementStick::Press(value);
    }

    pub fn get_i8(&mut self) -> i8 {
        match self {
            &mut ElementStick::Hold (value) => { value }
            &mut ElementStick::Press (value) => {
                *self = ElementStick::Hold(0);
                value
            }
        }
    }

    pub fn get_u8(&mut self) -> u8 {
        match self {
            &mut ElementStick::Hold (value) => { i8_to_u8(value) }
            &mut ElementStick::Press (value) => {
                *self = ElementStick::Hold(0);
                i8_to_u8(value)
            }
        }
    }
}

fn i8_to_u8(value: i8) -> u8 {
    (127 + value) as u8
}

impl ElementTrigger {
    pub fn hold(&mut self, value: u8) {
        *self = ElementTrigger::Hold(value);
    }

    pub fn press(&mut self, value: u8) {
        *self = ElementTrigger::Press(value);
    }

    pub fn get(&mut self) -> u8 {
        match self {
            &mut ElementTrigger::Hold (value) => { value }
            &mut ElementTrigger::Press (value) => {
                *self = ElementTrigger::Hold(0);
                value
            }
        }
    }
}

pub struct Controller {
    pub plugged_in: bool,

    pub a:     ElementButton,
    pub b:     ElementButton,
    pub x:     ElementButton,
    pub y:     ElementButton,
    pub left:  ElementButton,
    pub right: ElementButton,
    pub down:  ElementButton,
    pub up:    ElementButton,
    pub start: ElementButton,
    pub z:     ElementButton,
    pub r:     ElementButton,
    pub l:     ElementButton,

    pub stick_x:   ElementStick,
    pub stick_y:   ElementStick,
    pub c_stick_x: ElementStick,
    pub c_stick_y: ElementStick,
    pub r_trigger: ElementTrigger,
    pub l_trigger: ElementTrigger,
}

impl Controller {
    pub fn new() -> Controller {
        Controller {
            plugged_in: false,

            a:     ElementButton::None,
            b:     ElementButton::None,
            x:     ElementButton::None,
            y:     ElementButton::None,
            left:  ElementButton::None,
            right: ElementButton::None,
            down:  ElementButton::None,
            up:    ElementButton::None,
            start: ElementButton::None,
            z:     ElementButton::None,
            r:     ElementButton::None,
            l:     ElementButton::None,

            stick_x:   ElementStick::Hold(0),
            stick_y:   ElementStick::Hold(0),
            c_stick_x: ElementStick::Hold(0),
            c_stick_y: ElementStick::Hold(0),
            r_trigger: ElementTrigger::Hold(0),
            l_trigger: ElementTrigger::Hold(0),
        }
    }

    pub fn to_sandbox(&mut self) -> ControllerInput {
        let (stick_x,     stick_y) = stick_filter(self.stick_x.get_u8()   as u8, self.stick_y.get_u8()   as u8);
        let (c_stick_x, c_stick_y) = stick_filter(self.c_stick_x.get_u8() as u8, self.c_stick_y.get_u8() as u8);
        ControllerInput {
            plugged_in: true,

            a:     self.a.get(),
            b:     self.b.get(),
            x:     self.x.get(),
            y:     self.y.get(),
            left:  self.left.get(),
            right: self.right.get(),
            down:  self.down.get(),
            up:    self.up.get(),
            start: self.start.get(),
            z:     self.z.get(),
            r:     self.r.get(),
            l:     self.l.get(),

            stick_x:   stick_x,
            stick_y:   stick_y,
            c_stick_x: c_stick_x,
            c_stick_y: c_stick_y,
            r_trigger: trigger_filter(self.r_trigger.get()),
            l_trigger: trigger_filter(self.l_trigger.get()),
        }
    }
}

// DO NOT MODIFY: Needs to be same as pf_sandbox ControllerInput
#[derive(Serialize, Deserialize)]
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

fn abs_min(a: f32, b: f32) -> f32 {
    if (a >= 0.0 && a > b) || (a <= 0.0 && a < b) {
        b
    } else {
        a
    }
}

fn stick_filter(in_stick_x: u8, in_stick_y: u8) -> (f32, f32) {
    let raw_stick_x = in_stick_x as f32 - 128.0;
    let raw_stick_y = in_stick_y as f32 - 128.0;
    let angle = (raw_stick_y).atan2(raw_stick_x);

    let max = (angle.cos() * 80.0).trunc();
    let mut stick_x = abs_min(raw_stick_x, max) / 80.0;

    let max = (angle.sin() * 80.0).trunc();
    let mut stick_y = abs_min(raw_stick_y, max) / 80.0;

    let deadzone = 0.28;
    if stick_x.abs() < deadzone {
        stick_x = 0.0;
    }
    if stick_y.abs() < deadzone {
        stick_y = 0.0;
    }

    (stick_x, stick_y)
}

fn trigger_filter(trigger: u8) -> f32 {
    let value = (trigger as f32) / 140.0;
    if value > 1.0
    {
        1.0
    }
    else {
        value
    }
}
