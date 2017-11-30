extern crate serde_json;
extern crate winit;
extern crate vulkano_win;
extern crate vulkano_text;
#[macro_use] extern crate vulkano;
#[macro_use] extern crate serde_derive;

pub mod graphics;
pub mod input;
pub mod state;
pub mod controller;
pub mod connection;

use std::time::{Instant, Duration};
use std::thread;

use graphics::Graphics;
use input::Input;
use state::State;
use winit::EventsLoop;

fn main() {
    let mut events_loop = EventsLoop::new();
    let mut graphics = Graphics::new(&mut events_loop);
    let mut input = Input::new(events_loop);
    let mut state = State::new();

    loop {
        let frame_start = Instant::now();

        input.update();
        state.update(&input);
        connection::send(&mut state);
        graphics.draw(&state);

        if input.quit() {
            connection::quit();
            return; // TODO: Despite hitting this return, my laptop does not close the program o.0 Attach a debugger I guess.
        }

        let frame_duration = Duration::from_secs(1) / 60;
        let frame_duration_actual = frame_start.elapsed();
        if frame_duration_actual < frame_duration {
            thread::sleep(frame_duration - frame_start.elapsed());
        }
    }
}
