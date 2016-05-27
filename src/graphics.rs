use ::fighter::Fighter;
use ::player::Player;
use ::stage::Stage;

use glium::{DisplayBuild, Surface, self};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct Graphics {
    display: glium::backend::glutin_backend::GlutinFacade,
    players: Arc<Mutex<Vec<Player>>>,
    fighters: Arc<Mutex<Vec<Fighter>>>,
    stages: Arc<Mutex<Vec<Stage>>>,
}

impl Graphics {
    pub fn new(players: Arc<Mutex<Vec<Player>>>, fighters: Arc<Mutex<Vec<Fighter>>>, stages: Arc<Mutex<Vec<Stage>>>) -> Graphics {
        let display = glium::glutin::WindowBuilder::new()
            .with_title("PF ENGINE")
            .build_glium()
            .unwrap();
        Graphics { display: display, players: players, fighters: fighters, stages: stages }
    }

    pub fn run(&mut self) {
        loop {
            self.render();
            self.handle_events();

            thread::sleep(Duration::from_millis(16));
        }
    }

    fn render(&mut self) { //Fighters, Stages
        let mut target = self.display.draw();
        target.clear_color(0.4, 1.0, 1.0, 1.0);
        target.finish().unwrap();
    }

    fn close(&mut self){
        //TODO: errr I guess I need to get the main thead to terminate
    }

    fn handle_events(&mut self) {
        let mut close = false;
        {
            for ev in self.display.poll_events() {
                match ev {
                    glium::glutin::Event::Closed => { close = true; }, 
                    _ => ()
                }
            }
        }

        if close {
            self.close();
        }
    }
}
