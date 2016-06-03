use ::fighter::Fighter;
use ::player::Player;
use ::stage::Stage;

use glium::{DisplayBuild, Surface, self};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

pub struct Graphics {
    display: glium::backend::glutin_backend::GlutinFacade,
}

#[allow(unused_variables)]
impl Graphics {
    pub fn new() -> Graphics {
        let display = glium::glutin::WindowBuilder::new()
            .with_title("PF ENGINE")
            .build_glium()
            .unwrap();
        Graphics { display: display }
    }

    pub fn run(&mut self, players: Arc<Mutex<Vec<Player>>>, fighters: Arc<Mutex<Vec<Fighter>>>, stages: Arc<Mutex<Vec<Stage>>>) {
        loop {
            {
                let mut target = self.display.draw();
                target.clear_color(0.0, 0.0, 0.0, 1.0);
                let players = players.lock().unwrap();
                for player in players.iter() {
                    self.player_render(player, &mut target);
                }
                target.finish().unwrap();
            }
            self.handle_events();

            thread::sleep(Duration::from_millis(16));
        }
    }

    fn player_render(&mut self, player: &Player, target: &mut glium::Frame) {
        let ecb_w = (player.ecb_w) as f32;
        let ecb_y = (player.ecb_y) as f32;
        let ecb_top = (player.ecb_top) as f32;
        let ecb_bottom = (player.ecb_bottom) as f32;

        // ecb
        let vertex0 = Vertex { position: [ 0.0, ecb_y + ecb_bottom] };
        let vertex1 = Vertex { position: [-ecb_w/2.0, ecb_y] };
        let vertex2 = Vertex { position: [ ecb_w/2.0, ecb_y] };
        let vertex3 = Vertex { position: [ 0.0, ecb_y + ecb_top] };

        // horizontal bps
        let vertex4 = Vertex { position: [-4.0,-0.15] };
        let vertex5 = Vertex { position: [-4.0, 0.15] };
        let vertex6 = Vertex { position: [ 4.0,-0.15] };
        let vertex7 = Vertex { position: [ 4.0, 0.15] };

        // vertical bps
        let vertex8  = Vertex { position: [-0.15,-4.0] };
        let vertex9  = Vertex { position: [ 0.15,-4.0] };
        let vertex10 = Vertex { position: [-0.15, 4.0] };
        let vertex11 = Vertex { position: [ 0.15, 4.0] };

        let shape = vec![vertex0, vertex1, vertex2, vertex3, vertex4, vertex5, vertex6, vertex7, vertex8, vertex9, vertex10, vertex11];
        let indices: [u16; 18] = [
            1,  2,  0,
            1,  2,  3,
            4,  5,  6,
            7,  6,  5,
            8,  9,  10,
            11, 10, 13,
        ];

        let vertex_buffer = glium::VertexBuffer::new(&self.display, &shape).unwrap();
        let indices = glium::IndexBuffer::new(&self.display, glium::index::PrimitiveType::TrianglesList, &indices).unwrap();

        let vertex_shader_src = r#"
            # version 140

            in vec2 position;
            uniform mat4 matrix;

            void main() {
                vec2 pos = position;
                gl_Position = vec4(pos, 0.0, 1.0) * matrix;
            }
        "#;
        let fragment_shader_src = r#"
            # version 140

            out vec4 color;

            void main() {
                color = vec4(1.0, 1.0, 1.0, 0.5);
            }
        "#;
        let program = glium::Program::from_source(&self.display, vertex_shader_src, fragment_shader_src, None).unwrap();

        let scale: f32 = 0.01;
        let p_x: f32 = (player.bps.x as f32) * scale;
        let p_y: f32 = (player.bps.y as f32) * scale;
        let matrix = [
            [scale, 0.0,   0.0, p_x],
            [0.0,   scale, 0.0, p_y],
            [0.0,   0.0,   1.0, 0.0],
            [0.0,   0.0,   0.0, 1.0f32],
        ];

        target.draw(&vertex_buffer, &indices, &program, &uniform! {matrix: matrix}, &Default::default()).unwrap();
    }

    fn close(&mut self){
        //TODO: errr I guess I need to get the main thread to terminate
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
