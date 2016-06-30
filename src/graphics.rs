use ::fighter::Fighter;
use ::player::Player;
use ::stage::Stage;
use ::input::{KeyInput, KeyAction};

use glium::{DisplayBuild, Surface, self};
use std::fs::{File, self};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::collections::HashMap;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

pub struct Graphics {
    shaders: HashMap<String, String>,
    display: glium::backend::glutin_backend::GlutinFacade,
}

#[allow(unused_variables)]
impl Graphics {
    pub fn new() -> Graphics {
        let display = glium::glutin::WindowBuilder::new()
            .with_title("PF ENGINE")
            .build_glium()
            .unwrap();
        Graphics {
            shaders: Graphics::load_shaders(),
            display: display
        }
    }

    fn load_shaders() -> HashMap<String, String> {
        let mut shaders: HashMap<String, String> = HashMap::new();

        let dir_path = PathBuf::from("shaders");
        for path in fs::read_dir(dir_path).unwrap() {
            let full_path = path.unwrap().path();

            let mut shader_source = String::new();
            File::open(&full_path).unwrap().read_to_string(&mut shader_source).unwrap();
            let key = full_path.file_stem().unwrap().to_str().unwrap().to_string();
            shaders.insert(key, shader_source);
        }

        shaders
    }

    pub fn run(&mut self,
               players:   Arc<Mutex<Vec<Player>>>,
               fighters:  Arc<Mutex<Vec<Fighter>>>,
               stages:    Arc<Mutex<Vec<Stage>>>,
               mut key_input: Arc<Mutex<KeyInput>>) {
        loop {
            {
                let mut target = self.display.draw();
                target.clear_color(0.0, 0.0, 0.0, 1.0);

                let players = players.lock().unwrap();
                for player in players.iter() {
                    self.player_render(player, &mut target);
                }

                let stages = stages.lock().unwrap();
                self.stage_render(&stages[0], &mut target);

                target.finish().unwrap();
            }
            self.handle_events(&mut key_input);

            thread::sleep(Duration::from_millis(16));
        }
    }

    // TODO: Clean up shared code between stage and player render
    fn stage_render(&mut self, stage: &Stage, target: &mut glium::Frame) {
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u16> = Vec::new();
        let mut indice_count = 0;
        for platform in &stage.platforms {
            let x1 = (platform.x - platform.w / 2.0) as f32;
            let y1 = (platform.y - platform.h / 2.0) as f32;
            let x2 = (platform.x + platform.w / 2.0) as f32;
            let y2 = (platform.y + platform.h / 2.0) as f32;

            vertices.push(Vertex { position: [x1, y1] });
            vertices.push(Vertex { position: [x1, y2] });
            vertices.push(Vertex { position: [x2, y1] });
            vertices.push(Vertex { position: [x2, y2] });

            indices.push(indice_count + 0);
            indices.push(indice_count + 1);
            indices.push(indice_count + 2);
            indices.push(indice_count + 1);
            indices.push(indice_count + 2);
            indices.push(indice_count + 3);
            indice_count += 4;
        }
        let vertex_buffer = glium::VertexBuffer::new(&self.display, &vertices).unwrap();
        let indices = glium::IndexBuffer::new(&self.display, glium::index::PrimitiveType::TrianglesList, &indices).unwrap();

        let vertex_shader = self.shaders.get("vertex").unwrap();
        let fragment_shader = self.shaders.get("fragment").unwrap();
        let program = glium::Program::from_source(&self.display, vertex_shader, fragment_shader, None).unwrap();
        
        let scale: f32 = 0.01;
        let matrix = [
            [scale, 0.0,   0.0, 0.0],
            [0.0,   scale, 0.0, 0.0],
            [0.0,   0.0,   1.0, 0.0],
            [0.0,   0.0,   0.0, 1.0f32],
        ];
        target.draw(&vertex_buffer, &indices, &program, &uniform! {matrix: matrix}, &Default::default()).unwrap();
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

        let program = glium::Program::from_source(&self.display, self.shaders.get("vertex").unwrap(), self.shaders.get("fragment").unwrap(), None).unwrap();

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
        // TODO: errr I guess I need to get the main thread to terminate
    }

    fn handle_events(&mut self, key_input: &mut Arc<Mutex<KeyInput>>) {
        let mut close = false;
        let mut key_actions: Vec<KeyAction> = vec!();

        for ev in self.display.poll_events() {
            use glium::glutin::Event::*;
            use glium::glutin::ElementState::{Pressed, Released};
            use glium::glutin::VirtualKeyCode::Escape;

            match ev {
                KeyboardInput(Pressed, _, Some(Escape)) | Closed
                    => { close = true; },
                KeyboardInput(Pressed, _, Some(key_code))
                    => { key_actions.push(KeyAction::Pressed  (key_code)) },
                KeyboardInput(Released, _, Some(key_code))
                    => { key_actions.push(KeyAction::Released (key_code)) },
                _   => {},
            }
        }

        if close {
            self.close()
        }

        let mut key_input = key_input.lock().unwrap();
        key_input.set_actions(key_actions);
    }
}
