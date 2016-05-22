#[derive(Default)]
pub struct Control {
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

    pub stick_x:   i8,
    pub stick_y:   i8,
    pub c_stick_x: i8,
    pub c_stick_y: i8,
    pub r_analog:  u8,
    pub l_analog:  u8,
}
