pub enum ElementButton {
    None,
    Press,
    Hold,
}

pub enum ElementStick {
    Press (i8),
    Hold (i8),
}

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

    pub fn get(&mut self) -> i8 {
        match self {
            &mut ElementStick::Hold (value) => { value }
            &mut ElementStick::Press (value) => {
                *self = ElementStick::Hold(0);
                value
            }
        }
    }
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
}
