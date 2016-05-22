#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct Platform {
    x:            f64,
    y:            f64,
    w:            f64,
    h:            f64,
    pass_through: bool,
}

impl Platform {
    pub fn new(x: f64, y: f64, w: f64, h: f64, pass_through: bool) -> Platform {
        Platform {
            x: x,
            y: y,
            w: w,
            h: h,
            pass_through: pass_through,
        }
    }
}
