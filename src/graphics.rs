use ::app::Render;
use ::buffers::{Buffers, PackageBuffers};
use ::game::{GameState, RenderEntity, RenderGame};
use ::input::{KeyInput, KeyAction};
use ::menu::RenderMenu;
use ::package::{Package, PackageUpdate};

use glium::{DisplayBuild, Surface, self};
use glium::backend::glutin_backend::GlutinFacade;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::fs::{File, self};
use std::io::Read;
use std::path::PathBuf;
use std::thread;
use std::collections::HashMap;


#[allow(dead_code)]
pub struct Graphics {
    shaders:         HashMap<String, String>,
    package_buffers: PackageBuffers,
    display:         GlutinFacade,
    key_input_tx:    Sender<KeyAction>,
    render_rx:       Receiver<GraphicsMessage>,
}

#[allow(unused_variables)]
impl Graphics {
    pub fn init(package: &Package) -> (Sender<GraphicsMessage>, KeyInput) {
        let fighters = package.fighters.clone();
        let stages   = package.stages.clone();
        let (render_tx, render_rx) = channel();
        let (key_input, key_input_tx) = KeyInput::new();
        let package = package.clone();

        thread::spawn(move || {
            let mut graphics = Graphics::new(package, key_input_tx, render_rx);
            graphics.run();
        });
        (render_tx, key_input)
    }

    fn new(
        package: Package,
        key_input_tx: Sender<KeyAction>,
        render_rx: Receiver<GraphicsMessage>,
    ) -> Graphics {
        let display = glium::glutin::WindowBuilder::new()
            .with_title("PF ENGINE")
            .build_glium()
            .unwrap();
        Graphics {
            shaders:         Graphics::load_shaders(),
            package_buffers: PackageBuffers::new(&display, package),
            display:         display,
            key_input_tx:    key_input_tx,
            render_rx:       render_rx,
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
                // get the most recent render
                let mut render = {
                    let message = self.render_rx.recv().unwrap();
                    self.read_message(message)
                };
                loop {
                    match self.render_rx.try_recv() {
                        Ok(message) => { render = self.read_message(message); },
                        Err(_)  => { break; },
                    }
                }

                match render {
                    Render::Game(game) => { self.game_render(game); },
                    Render::Menu(menu) => { self.menu_render(menu); },
                }
            }
            self.handle_events();
        }
    }

    fn read_message(&mut self, message: GraphicsMessage) -> Render {
        self.package_buffers.update(&self.display, message.package_updates);
        message.render
    }

    fn game_render(&mut self, render: RenderGame) {
        let mut target = self.display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);

        //TODO: Run these once only
        let vertex_shader = self.shaders.get("vertex").unwrap();
        let fragment_shader = self.shaders.get("fragment").unwrap();
        let program = glium::Program::from_source(&self.display, vertex_shader, fragment_shader, None).unwrap();

        let zoom: f32 = 0.01;

        match render.state {
            GameState::Local  => { },
            GameState::Paused => { },
            _                 => { },
        }

        for entity in render.entities {
            match entity {
                RenderEntity::Player(player) => {
                    let position: [f32; 2] = [player.bps.x, player.bps.y];
                    let uniform = &uniform! { position_offset: position, zoom: zoom };

                    // draw fighter
                    let fighter_frames = &self.package_buffers.fighters[player.fighter][player.action];
                    if player.frame < fighter_frames.len() {
                        let vertices = &fighter_frames[player.frame].vertex;
                        let indices  = &fighter_frames[player.frame].index;
                        target.draw(vertices, indices, &program, uniform, &Default::default()).unwrap();
                    }
                    else {
                        // TODO: Give some indication that we are rendering a deleted or otherwise nonexistent frame
                    }

                    // draw player ecb
                    if player.ecb_enable {
                        let ecb = Buffers::new_player(&self.display, &player);
                        target.draw(&ecb.vertex, &ecb.index, &program, uniform, &Default::default()).unwrap();
                    }
                },
            }
        }
        let stage = 0;

        let vertices = &self.package_buffers.stages[stage].vertex;
        let indices = &self.package_buffers.stages[stage].index;
        let uniform = &uniform! { position_offset: [0.0 as f32, 0.0 as f32], zoom: zoom };
        target.draw(vertices, indices, &program, uniform, &Default::default()).unwrap();

        target.finish().unwrap();
    }

    fn menu_render(&mut self, render: RenderMenu) {

    }

    fn handle_events(&mut self) {
        for ev in self.display.poll_events() {
            use glium::glutin::Event::*;
            use glium::glutin::ElementState::{Pressed, Released};
            use glium::glutin::VirtualKeyCode;

            match ev {
                Closed => {
                    self.key_input_tx.send(KeyAction::Pressed (VirtualKeyCode::Escape)).unwrap();
                },
                KeyboardInput(Pressed, _, Some(key_code)) => {
                    self.key_input_tx.send(KeyAction::Pressed  (key_code)).unwrap();
                },
                KeyboardInput(Released, _, Some(key_code)) => {
                    self.key_input_tx.send(KeyAction::Released (key_code)).unwrap();
                },
                _   => {},
            }
        }
    }
}

pub struct GraphicsMessage {
    pub render: Render,
    pub package_updates: Vec<PackageUpdate>,
}
