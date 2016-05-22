use glium::{DisplayBuild, Surface, self};

use ::player::Player;
use ::stage::Stage;

pub struct Graphics {
    display: glium::backend::glutin_backend::GlutinFacade,
}

impl Graphics {
    pub fn new() -> Graphics {
        let display = glium::glutin::WindowBuilder::new().build_glium().unwrap();
        Graphics { display: display}
    }

    pub fn render(&mut self, stage: &Stage, players: &Vec<Player>) {
        let mut target = self.display.draw();
        target.clear_color(0.4, 1.0, 1.0, 1.0);
        target.finish().unwrap();
    }

    pub fn check_close(&self) -> bool {
        for ev in self.display.poll_events() {
            match ev {
                glium::glutin::Event::Closed => return true,
                _ => ()
            }
        }
        false
    }
}
