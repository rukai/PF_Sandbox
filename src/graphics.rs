use ::fighter::Fighter;
use ::game::{GameState, RenderEntity, RenderGame};
use ::player::{RenderPlayer};
use ::stage::Stage;
use ::package::Package;
use ::input::{KeyInput, KeyAction};
use ::app::Render;
use ::menu::RenderMenu;

use glium::{DisplayBuild, Surface, self};
use std::sync::mpsc::{Sender, Receiver, channel};
use std::fs::{File, self};
use std::io::Read;
use std::path::PathBuf;
use std::thread;
use std::collections::HashMap;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}
implement_vertex!(Vertex, position);

#[allow(dead_code)]
pub struct Graphics {
    shaders:      HashMap<String, String>,
    display:      glium::backend::glutin_backend::GlutinFacade,
    stages:       Vec<Stage>,
    fighters:     Vec<Fighter>,
    key_input_tx: Sender<KeyAction>,
    render_rx:    Receiver<Render>,
}

#[allow(unused_variables)]
impl Graphics {
    pub fn init(package: &Package) -> (Sender<Render>, KeyInput) {
        let fighters = package.fighters.clone();
        let stages   = package.stages.clone();
        let (render_tx, render_rx) = channel();
        let (key_input, key_input_tx) = KeyInput::new();

        thread::spawn(move || {
            let mut graphics = Graphics::new(stages, fighters, key_input_tx, render_rx);
            graphics.run(); // TODO: should render_rx go in the constructor?
        });
        (render_tx, key_input)
    }

    fn new(
        stages: Vec<Stage>,
        fighters: Vec<Fighter>,
        key_input_tx: Sender<KeyAction>,
        render_rx: Receiver<Render>,
    ) -> Graphics {
        let display = glium::glutin::WindowBuilder::new()
            .with_title("PF ENGINE")
            .build_glium()
            .unwrap();
        Graphics {
            shaders:      Graphics::load_shaders(),
            display:      display,
            stages:       stages,
            fighters:     fighters,
            key_input_tx: key_input_tx,
            render_rx:    render_rx,
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

    fn run(&mut self) {
        loop {
            {
                let mut target = self.display.draw();
                target.clear_color(0.0, 0.0, 0.0, 1.0);

                // get the most recent render
                let mut render = self.render_rx.recv().unwrap();
                loop {
                    match self.render_rx.try_recv() {
                        Ok(msg) => { render = msg; },
                        Err(_)  => { break; },
                    }
                }

                match render {
                    Render::Game(game) => { self.game_render(game, &mut target); },
                    Render::Menu(menu) => { self.menu_render(menu); },
                }

                target.finish().unwrap();
            }
            self.handle_events();
        }
    }

    fn game_render(&mut self, render: RenderGame, target: &mut glium::Frame) {
        match render.state {
            GameState::Local  => { },
            GameState::Paused => { },
            _                 => { },
        }

        for entity in render.entities {
            match entity {
                RenderEntity::Player(player) => { self.player_render(player, target) },
            }
        }
        self.stage_render(0, target);
    }

    fn menu_render(&mut self, render: RenderMenu) {
    }

    // TODO: Clean up shared code between stage and player render
    fn stage_render(&mut self, stage: usize, target: &mut glium::Frame) {
        let stage = &self.stages[stage];

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

    fn player_render(&mut self, player: RenderPlayer, target: &mut glium::Frame) {
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

    fn handle_events(&mut self) {
        for ev in self.display.poll_events() {
            use glium::glutin::Event::*;
            use glium::glutin::ElementState::{Pressed, Released};
            use glium::glutin::VirtualKeyCode;

            match ev {
                Closed
                    => { self.key_input_tx.send(KeyAction::Pressed (VirtualKeyCode::Escape)).unwrap(); },
                KeyboardInput(Pressed, _, Some(key_code))
                    => { self.key_input_tx.send(KeyAction::Pressed  (key_code)).unwrap(); },
                KeyboardInput(Released, _, Some(key_code))
                    => { self.key_input_tx.send(KeyAction::Released (key_code)).unwrap(); },
                _   => {},
            }
        }
    }
}
