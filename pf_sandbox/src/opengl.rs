use ::graphics::{Render, GraphicsMessage};
use ::opengl_buffers::{Buffers, PackageBuffers};
use ::game::{GameState, RenderEntity, RenderGame};
use ::menu::RenderMenu;
use ::player::RenderFighter;

use glium::{DisplayBuild, Surface, self};
use glium::glutin::Event;
use glium::backend::glutin_backend::GlutinFacade;
use glium::draw_parameters::DrawParameters;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::fs::{File, self};
use std::io::Read;
use std::path::PathBuf;
use std::thread;
use std::collections::HashMap;

#[allow(dead_code)]
pub struct OpenGLGraphics {
    shaders:         HashMap<String, String>,
    package_buffers: PackageBuffers,
    display:         GlutinFacade,
    os_input_tx:     Sender<Event>,
    render_rx:       Receiver<GraphicsMessage>,
}

impl OpenGLGraphics {
    pub fn init(os_input_tx: Sender<Event>) -> Sender<GraphicsMessage> {
        let (render_tx, render_rx) = channel();

        thread::spawn(move || {
            let mut graphics = OpenGLGraphics::new(os_input_tx, render_rx);
            graphics.run();
        });
        render_tx
    }

    fn new(
        os_input_tx: Sender<Event>,
        render_rx: Receiver<GraphicsMessage>,
    ) -> OpenGLGraphics {
        let display = glium::glutin::WindowBuilder::new()
            .with_title("PF Sandbox")
            .build_glium()
            .unwrap();

        OpenGLGraphics {
            shaders:         OpenGLGraphics::load_shaders(),
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
            let vertex_shader = self.shaders.get("opengl-generic-vertex").unwrap();
            let fragment_shader = self.shaders.get("opengl-generic-fragment").unwrap();
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

        let stage = 0;

        let uniform = &uniform! {
            position_offset: [pan.0 as f32, pan.1 as f32],
            zoom:            zoom,
            aspect_ratio:    aspect_ratio,
            direction:       1.0f32,
            edge_color:      [1.0f32, 1.0, 1.0, 1.0],
            color:           [1.0f32, 1.0, 1.0, 1.0]
        };

        let draw_params = DrawParameters {
            blend: glium::Blend::alpha_blending(),
            ..Default::default()
        };

        let buffers = &self.package_buffers.stages[stage];
        target.draw(&buffers.vertex, &buffers.index, &program, uniform, &draw_params).unwrap();

        for entity in render.entities {
            match entity {
                RenderEntity::Player(player) => {
                    let dir = if player.face_right { 1.0 } else { -1.0 } as f32;
                    let draw_pos: [f32; 2] = [player.bps.0 + pan.0 as f32, player.bps.1 + pan.1 as f32];

                    // draw player ecb
                    if player.debug.ecb {
                        let ecb = Buffers::new_player(&self.display, &player);
                        let color = if player.fighter_selected {
                            [0.0f32, 1.0, 0.0, 1.0]
                        } else {
                            [1.0f32, 1.0, 1.0, 1.0]
                        };
                        let uniform = &uniform! {
                            position_offset: draw_pos,
                            zoom:            zoom,
                            aspect_ratio:    aspect_ratio,
                            direction:       dir,
                            edge_color:      color,
                            color:           color
                        };
                        target.draw(&ecb.vertex, &ecb.index, &program, uniform, &draw_params).unwrap();
                    }

                    // draw fighter
                    match player.debug.fighter {
                        RenderFighter::Normal | RenderFighter::Debug => {
                            let color = if let RenderFighter::Debug = player.debug.fighter {
                                [0.0f32, 0.0, 0.0, 0.0]
                            } else {
                                [1.0f32, 1.0, 1.0, 1.0]
                            };
                            let edge_color = if player.fighter_selected {
                                [0.0f32, 1.0, 0.0, 1.0]
                            } else {
                                player.fighter_color
                            };
                            let uniform = &uniform! {
                                position_offset: draw_pos,
                                zoom:            zoom,
                                aspect_ratio:    aspect_ratio,
                                direction:       dir,
                                edge_color:      edge_color,
                                color:           color
                            };
                            let fighter_frames = &self.package_buffers.fighters[player.fighter][player.action];
                            if player.frame < fighter_frames.len() {
                                if let &Some(ref buffers) = &fighter_frames[player.frame] {
                                    target.draw(&buffers.vertex, &buffers.index, &program, uniform, &draw_params).unwrap();
                                }
                            }
                            else {
                                // TODO: Give some indication that we are rendering a deleted or otherwise nonexistent frame
                            }
                        }
                        RenderFighter::None => { }
                    }
                    // TODO: Edit::Player  - render selected player's BPS as green
                    // TODO: Edit::Fighter - render selected hitboxes and ecb points as green on selected player
                    // TODO: Edit::Fighter - render outline of selected player as green
                    // TODO: Edit::Stage   - render selected platforms as green

                },
                RenderEntity::Selector(rect) => {
                    let buffers = Buffers::rect_buffers(&self.display, rect);
                    let uniform = &uniform! {
                        position_offset: [pan.0 as f32, pan.1 as f32],
                        zoom:            zoom,
                        aspect_ratio:    aspect_ratio,
                        direction:       1.0f32,
                        edge_color:      [0.0f32, 1.0, 0.0, 1.0],
                        color:           [0.0f32, 1.0, 0.0, 1.0]
                    };
                    target.draw(&buffers.vertex, &buffers.index, &program, uniform, &draw_params).unwrap();
                },
                RenderEntity::Area(rect) => {
                    let buffers = Buffers::rect_buffers(&self.display, rect);
                    let uniform = &uniform! {
                        position_offset: [pan.0 as f32, pan.1 as f32],
                        zoom:            zoom,
                        aspect_ratio:    aspect_ratio,
                        direction:       1.0f32,
                        edge_color:      [0.0f32, 1.0, 0.0, 1.0],
                        color:           [0.0f32, 1.0, 0.0, 1.0]
                    };
                    target.draw(&buffers.vertex, &buffers.index, &program, uniform, &draw_params).unwrap();
                },
            }
        }

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
