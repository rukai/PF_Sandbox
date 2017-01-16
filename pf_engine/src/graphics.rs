use ::app::Render;
use ::buffers::{Buffers, PackageBuffers};
use ::game::{GameState, RenderEntity, RenderGame};
use ::os_input::OsInput;
use ::menu::RenderMenu;
use ::package::PackageUpdate;

use glium::{DisplayBuild, Surface, self};
use glium::glutin::Event;
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
    os_input_tx:     Sender<Event>,
    render_rx:       Receiver<GraphicsMessage>,
}

impl Graphics {
    pub fn init() -> (Sender<GraphicsMessage>, OsInput) {
        let (render_tx, render_rx) = channel();
        let (os_input, os_input_tx) = OsInput::new();

        thread::spawn(move || {
            let mut graphics = Graphics::new(os_input_tx, render_rx);
            graphics.run();
        });
        (render_tx, os_input)
    }

    fn new(
        os_input_tx: Sender<Event>,
        render_rx: Receiver<GraphicsMessage>,
    ) -> Graphics {
        let display = glium::glutin::WindowBuilder::new()
            .with_title("PF ENGINE")
            .build_glium()
            .unwrap();

        Graphics {
            shaders:         Graphics::load_shaders(),
            package_buffers: PackageBuffers::new(),
            display:         display,
            os_input_tx:     os_input_tx,
            render_rx:       render_rx,
        }
    }

    fn load_shaders() -> HashMap<String, String> {
        let mut shaders: HashMap<String, String> = HashMap::new();

        let dir_path = PathBuf::from("shaders");
        match fs::read_dir(dir_path) {
            Ok (paths) => {
                for path in paths {
                    let full_path = path.unwrap().path();

                    let mut shader_source = String::new();
                    File::open(&full_path).unwrap().read_to_string(&mut shader_source).unwrap();
                    let key = full_path.file_stem().unwrap().to_str().unwrap().to_string();
                    shaders.insert(key, shader_source);
                }
            }
            Err (_) => {
                panic!("Running from incorrect directory");
            }
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
                while let Ok(message) = self.render_rx.try_recv() {
                    render = self.read_message(message);
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

        // TODO: Run these once only
        let program = {
            let vertex_shader = self.shaders.get("generic-vertex").unwrap();
            let fragment_shader = self.shaders.get("generic-fragment").unwrap();
            glium::Program::from_source(&self.display, vertex_shader, fragment_shader, None).unwrap()
        };

        let player_program = {
            let vertex_shader = self.shaders.get("player-vertex").unwrap();
            let fragment_shader = self.shaders.get("generic-fragment").unwrap();
            glium::Program::from_source(&self.display, vertex_shader, fragment_shader, None).unwrap()
        };

        let zoom = render.camera.zoom.recip();
        let pan  = render.camera.pan;
        let (width, height) = self.display.get_window().unwrap().get_inner_size_points().unwrap();
        let aspect_ratio = width as f32 / height as f32;

        match render.state {
            GameState::Local  => { },
            GameState::Paused => { },
            _                 => { },
        }

        let white = [1.0 as f32, 1.0 as f32, 1.0 as f32];
        let green = [0.0 as f32, 1.0 as f32, 0.0 as f32];
        for entity in render.entities {
            match entity {
                RenderEntity::Player(player) => {
                    let position: [f32; 2] = [player.bps.0 + pan.0 as f32, player.bps.1 + pan.1 as f32];
                    let dir = if player.face_right { 1.0 } else { -1.0 } as f32;

                    // draw fighter
                    if !player.debug.no_fighter {
                        let uniform = &uniform! { position_offset: position, zoom: zoom, uniform_rgb: white, direction: dir, aspect_ratio: aspect_ratio};
                        let fighter_frames = &self.package_buffers.fighters[player.fighter][player.action];
                        if player.frame < fighter_frames.len() {
                            let vertices = &fighter_frames[player.frame].vertex;
                            let indices  = &fighter_frames[player.frame].index;
                            target.draw(vertices, indices, &player_program, uniform, &Default::default()).unwrap();
                        }
                        else {
                            // TODO: Give some indication that we are rendering a deleted or otherwise nonexistent frame
                        }
                    }
                    // TODO: Edit::Player  - render selected player's BPS as green
                    // TODO: Edit::Fighter - render selected hitboxes and ecb points as green on selected player
                    // TODO: Edit::Fighter - render outline of selected player as green
                    // TODO: Edit::Stage   - render selected platforms as green

                    // draw player ecb
                    if player.debug.player {
                        let ecb = Buffers::new_player(&self.display, &player);
                        if player.selected {
                            let uniform = &uniform! { position_offset: position, zoom: zoom, uniform_rgb: green, direction: dir, aspect_ratio: aspect_ratio };
                            target.draw(&ecb.vertex, &ecb.index, &player_program, uniform, &Default::default()).unwrap();
                        }
                        else {
                            let uniform = &uniform! { position_offset: position, zoom: zoom, uniform_rgb: white, direction: dir, aspect_ratio: aspect_ratio };
                            target.draw(&ecb.vertex, &ecb.index, &player_program, uniform, &Default::default()).unwrap();
                        }
                    }
                },
                RenderEntity::Selector(rect) => {
                    let vertices = Buffers::rect_vertices(&self.display, rect);
                    let indices = glium::index::NoIndices(glium::index::PrimitiveType::LineStrip);
                    let uniform = &uniform! { position_offset: [pan.0 as f32, pan.1 as f32], zoom: zoom, uniform_rgb: green, aspect_ratio: aspect_ratio };
                    target.draw(&vertices, &indices, &program, uniform, &Default::default()).unwrap();
                },
                RenderEntity::Area(rect) => {
                    let vertices = Buffers::rect_vertices(&self.display, rect);
                    let indices = glium::index::NoIndices(glium::index::PrimitiveType::LineStrip);
                    let uniform = &uniform! { position_offset: [pan.0 as f32, pan.1 as f32], zoom: zoom, uniform_rgb: green, aspect_ratio: aspect_ratio };
                    target.draw(&vertices, &indices, &program, uniform, &Default::default()).unwrap();
                },
            }
        }
        let stage = 0;

        let vertices = &self.package_buffers.stages[stage].vertex;
        let indices = &self.package_buffers.stages[stage].index;
        let uniform = &uniform! { position_offset: [pan.0 as f32, pan.1 as f32], zoom: zoom, uniform_rgb: white, aspect_ratio: aspect_ratio };
        target.draw(vertices, indices, &program, uniform, &Default::default()).unwrap();

        target.finish().unwrap();
    }

    #[allow(unused_variables)]
    fn menu_render(&mut self, render: RenderMenu) {
    }

    fn handle_events(&mut self) {
        // force send the current resolution
        let res = self.display.get_window().unwrap().get_inner_size_points().unwrap();
        self.os_input_tx.send(Event::Resized(res.0, res.1)).unwrap();

        for ev in self.display.poll_events() {
            self.os_input_tx.send(ev).unwrap();
        }
    }
}

pub struct GraphicsMessage {
    pub render: Render,
    pub package_updates: Vec<PackageUpdate>,
}
