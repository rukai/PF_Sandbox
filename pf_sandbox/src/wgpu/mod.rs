/// There is a lot of duplicated code between this and the vulkan renderer.
/// But thats ok because the vulkan renderer will be deleted once the wgpu renderer reaches feature parity.
mod buffers;

use buffers::{ColorVertex, ColorBuffers, Vertex, Buffers};
use crate::game::{GameState, RenderEntity, RenderGame};
use crate::graphics::{GraphicsMessage, Render, RenderType};
use crate::menu::{RenderMenu};
use crate::particle::ParticleType;
use crate::player::{RenderPlayerFrame, RenderFighter};
use pf_sandbox_lib::fighter::{CollisionBoxRole, Action};
use pf_sandbox_lib::package::{Package, PackageUpdate};

use std::sync::mpsc::{Sender, Receiver, channel};
use std::{thread, mem, f32};
use std::time::{Duration, Instant};

use cgmath::Rad;
use cgmath::{Matrix4, Vector3};
use num_traits::FromPrimitive;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use wgpu::{Device, SwapChain, BindGroup, BindGroupLayout, RenderPipeline, RenderPass, TextureView};
use wgpu_glyph::{Section, GlyphBrush, GlyphBrushBuilder, Scale as GlyphScale};

use winit::{
    Event,
    EventsLoop,
    Window,
    WindowEvent,
};

pub struct WgpuGraphics {
    package:                   Option<Package>,
    glyph_brush:               GlyphBrush<'static>,
    window:                    Window,
    event_loop:                EventsLoop,
    os_input_tx:               Sender<Event>,
    render_rx:                 Receiver<GraphicsMessage>,
    device:                    Device,
    swap_chain:                Option<SwapChain>,
    multisampled_framebuffer:  TextureView,
    render_pipeline:           RenderPipeline,
    pipeline:                  RenderPipeline,
    pipeline_surface:          RenderPipeline,
    bind_group:                BindGroup,
    bind_group_layout_surface: BindGroupLayout,
    prev_fullscreen:           Option<bool>,
    frame_durations:           Vec<Duration>,
    fps:                       String,
    width:                     u32,
    height:                    u32,
}

const SAMPLE_COUNT: u32 = 4;

impl WgpuGraphics {
    pub fn init(os_input_tx: Sender<Event>, device_name: Option<String>) -> Sender<GraphicsMessage> {
        let (render_tx, render_rx) = channel();

        thread::spawn(move || {
            let mut graphics = WgpuGraphics::new(os_input_tx, render_rx, device_name);
            graphics.run();
        });
        render_tx
    }

    fn new(os_input_tx: Sender<Event>, render_rx: Receiver<GraphicsMessage>, _device_name: Option<String>) -> WgpuGraphics {
        let event_loop = EventsLoop::new();

        let (window, instance, size, surface) = {
            let instance = wgpu::Instance::new();

            let window = Window::new(&event_loop).unwrap();
            window.set_title("PF Sandbox");
            let size = window
                .get_inner_size()
                .unwrap()
                .to_physical(window.get_hidpi_factor());

            let surface = instance.create_surface(&window);

            (window, instance, size, surface)
        };

        let adapter = instance.get_adapter(&wgpu::AdapterDescriptor {
            power_preference: wgpu::PowerPreference::LowPower,
        });

        let mut device = adapter.request_device(&wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: wgpu::Limits::default(),
        });

        let vs_bytes = include_bytes!("shader.vert.spv");
        let vs_module = device.create_shader_module(vs_bytes);
        let fs_bytes = include_bytes!("shader.frag.spv");
        let fs_module = device.create_shader_module(fs_bytes);

        let surface_vs_u32 = vk_shader_macros::include_glsl!("src/shaders/surface-vertex.glsl", kind: vert);
        let mut surface_vs_bytes = vec!();
        for word in surface_vs_u32.iter() {
            surface_vs_bytes.extend(&u32::to_le_bytes(*word));
        }
        let surface_vs_module = device.create_shader_module(&surface_vs_bytes);

        let surface_fs_u32 = vk_shader_macros::include_glsl!("src/shaders/surface-fragment.glsl", kind: frag);
        let mut surface_fs_bytes = vec!();
        for word in surface_fs_u32.iter() {
            surface_fs_bytes.extend(&u32::to_le_bytes(*word));
        }
        let surface_fs_module = device.create_shader_module(&surface_fs_bytes);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { bindings: &[] });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
        });

        let bind_group_layout_surface = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { bindings: &[
            wgpu::BindGroupLayoutBinding {
                binding:    0,
                visibility: wgpu::ShaderStage::all(),
                ty:         wgpu::BindingType::UniformBuffer,
            }
        ] });
        let pipeline_layout_surface = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout_surface],
        });

        let rasterization_state = wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
        };

        let color_states = [wgpu::ColorStateDescriptor {
            format: wgpu::TextureFormat::Bgra8Unorm,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }];

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::PipelineStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::PipelineStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: rasterization_state.clone(),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &color_states,
            depth_stencil_state: None,
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[],
            sample_count: SAMPLE_COUNT,
        });

        let pipeline_surface = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout_surface,
            vertex_stage: wgpu::PipelineStageDescriptor {
                module: &surface_vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::PipelineStageDescriptor {
                module: &surface_fs_module,
                entry_point: "main",
            }),
            rasterization_state: rasterization_state.clone(),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &color_states,
            depth_stencil_state: None,
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: mem::size_of::<ColorVertex>() as wgpu::BufferAddress,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float4,
                        offset: 4 * 2,
                        shader_location: 1,
                    },
                ],
            }],
            sample_count: SAMPLE_COUNT,
        });

        let vs_u32 = vk_shader_macros::include_glsl!("src/shaders/generic-vertex.glsl", kind: vert);
        let mut vs_bytes = vec!();
        for word in vs_u32.iter() {
            vs_bytes.extend(&u32::to_le_bytes(*word));
        }
        let vs_module = device.create_shader_module(&vs_bytes);

        let fs_u32 = vk_shader_macros::include_glsl!("src/shaders/generic-fragment.glsl", kind: frag);
        let mut fs_bytes = vec!();
        for word in fs_u32.iter() {
            fs_bytes.extend(&u32::to_le_bytes(*word));
        }
        let fs_module = device.create_shader_module(&fs_bytes);

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout_surface,
            vertex_stage: wgpu::PipelineStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::PipelineStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: rasterization_state.clone(),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &color_states,
            depth_stencil_state: None,
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float,
                        offset: 8,
                        shader_location: 1,
                    },
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Uint,
                        offset: 12,
                        shader_location: 2,
                    },
                ],
            }],
            sample_count: SAMPLE_COUNT,
        });

        let font: &[u8] = include_bytes!("DejaVuSans.ttf");
        let glyph_brush = GlyphBrushBuilder::using_font_bytes(font).build(&mut device, wgpu::TextureFormat::Bgra8Unorm);

        let width = size.width.round() as u32;
        let height = size.height.round() as u32;

        let swap_chain = Some(device.create_swap_chain(
            &surface,
            &wgpu::SwapChainDescriptor {
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                format: wgpu::TextureFormat::Bgra8Unorm,
                width,
                height,
            },
        ));

        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            size: wgpu::Extent3d { width, height, depth: 1 },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        };

        let multisampled_framebuffer = device.create_texture(multisampled_frame_descriptor).create_default_view();

        WgpuGraphics {
            package: None,
            glyph_brush,
            window,
            event_loop,
            os_input_tx,
            render_rx,
            device,
            swap_chain,
            multisampled_framebuffer,
            render_pipeline,
            pipeline,
            pipeline_surface,
            bind_group,
            bind_group_layout_surface,
            prev_fullscreen: None,
            frame_durations: vec!(),
            fps: "".into(),
            width,
            height,
        }
    }

    fn run(&mut self) {
        loop {
            {
                let frame_start = Instant::now();

                // get the most recent render
                let mut render = if let Ok(message) = self.render_rx.recv() {
                    self.read_message(message)
                } else {
                    return;
                };
                while let Ok(message) = self.render_rx.try_recv() {
                    render = self.read_message(message);
                }

                // MS Windows removes the window immediately on close before the process ends
                if let Some(resolution) = self.window.get_inner_size() {
                    let resolution: (u32, u32) = resolution.to_physical(self.window.get_hidpi_factor()).into();
                    self.window_resize(resolution.0, resolution.1);
                }
                else {
                    return;
                }

                self.render(render);
                self.frame_durations.push(frame_start.elapsed());
            }

            if !self.handle_events() {
                return;
            }
        }
    }

    fn read_message(&mut self, message: GraphicsMessage) -> Render {
        // TODO: Refactor out the vec + enum once vulkano backend is removed
        for package_update in message.package_updates {
            match package_update {
                PackageUpdate::Package (package) => {
                    self.package = Some(package);
                }
                _ => { }
            }
        }
        message.render
    }

    fn window_resize(&mut self, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }

        self.width = width;
        self.height = height;
        // TODO
    }

    fn render(&mut self, render: Render) {
        // TODO: Fullscreen logic should handle the window manager setting fullscreen state.
        // *    Use this instead of self.prev_fullscreen
        // *    Send new fullscreen state back to the main thread
        // Waiting on Window::get_fullscreen() to be added to winit: https://github.com/tomaka/winit/issues/579

        if self.prev_fullscreen.is_none() {
            self.prev_fullscreen = Some(!render.fullscreen); // force set fullscreen state on first update
        }
        if render.fullscreen != self.prev_fullscreen.unwrap() { // Need to avoid needlessly recalling set_fullscreen(Some(..)) or it causes FPS drops on at least X11
            if render.fullscreen {
                let monitor = self.window.get_current_monitor();
                self.window.set_fullscreen(Some(monitor));
            }
            else {
                self.window.set_fullscreen(None);
            }
            self.prev_fullscreen = Some(render.fullscreen);
        }

        // hide cursor during regular play in fullscreen
        let in_game_paused = if let RenderType::Game(game) = &render.render_type {
            if let GameState::Paused = &game.state {
                true
            } else {
                false
            }
        } else {
            false
        };
        self.window.hide_cursor(render.fullscreen && !in_game_paused);

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
        let mut swap_chain = self.swap_chain.take().unwrap();
        {
            let frame = swap_chain.get_next_texture();

            {
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &self.multisampled_framebuffer,
                        resolve_target: Some(&frame.view),
                        load_op: wgpu::LoadOp::Clear,
                        store_op: wgpu::StoreOp::Store,
                        clear_color: wgpu::Color::BLACK,
                    }],
                    depth_stencil_attachment: None,
                });

                match render.render_type {
                    RenderType::Game(game) => self.game_render(game, &mut rpass, &render.command_output),
                    RenderType::Menu(menu) => self.menu_render(menu, &mut rpass, &render.command_output)
                }
            }

            self.glyph_brush.draw_queued(&mut self.device, &mut encoder, &frame.view, self.width, self.height).unwrap();

            self.device.get_queue().submit(&[encoder.finish()]);
        }
        self.swap_chain = Some(swap_chain);
    }

    fn command_render(&mut self, lines: &[String]) {
        // TODO: Render white text, with black background
        for (i, line) in lines.iter().enumerate() {
            self.glyph_brush.queue(Section {
                text: line,
                color: [1.0, 1.0, 0.0, 1.0],
                screen_position: (0.0, self.height as f32 - 15.0 - 20.0 * i as f32),
                scale: GlyphScale::uniform(20.0),
                .. Section::default()
            });
        }
    }

    fn game_timer_render(&mut self, timer: &Option<Duration>) {
        if let &Some(ref timer) = timer {
            let minutes = timer.as_secs() / 60;
            let seconds = timer.as_secs() % 60;
            self.glyph_brush.queue(Section {
                text: format!("{:02}:{:02}", minutes, seconds).as_ref(),
                color: [1.0, 1.0, 1.0, 1.0],
                screen_position: ((self.width / 2) as f32 - 50.0, 4.0),
                scale: GlyphScale::uniform(40.0),
                .. Section::default()
            });
        }
    }

    fn game_hud_render(&mut self, entities: &[RenderEntity]) {
        let mut players = 0;
        for entity in entities {
            if let &RenderEntity::Player(_) = entity {
                players += 1;
            }
        }
        let distance = (self.width / (players + 1)) as f32;

        let mut location = -100.0;
        for entity in entities {
            if let &RenderEntity::Player(ref player) = entity {
                location += distance;
                match Action::from_u64(player.frames[0].action as u64) {
                    Some(Action::Eliminated) => { }
                    _ => {
                        let c = player.fighter_color.clone();
                        let color = [c[0], c[1], c[2], 1.0];

                        if let Some(stocks) = player.stocks {
                            let stocks_string = if stocks > 5 {
                                format!("⬤ x {}", stocks)
                            } else {
                                let mut stocks_string = String::new();
                                for _ in 0..stocks {
                                    stocks_string.push('⬤');
                                }
                                stocks_string
                            };

                            self.glyph_brush.queue(Section {
                                text: stocks_string.as_ref(),
                                color,
                                screen_position: (location + 10.0, self.height as f32 - 130.0),
                                scale: GlyphScale::uniform(22.0),
                                .. Section::default()
                            });
                        }

                        self.glyph_brush.queue(Section {
                            text: format!("{}%", player.damage).as_ref(),
                            color,
                            screen_position: (location, self.height as f32 - 117.0),
                            scale: GlyphScale::uniform(110.0),
                            .. Section::default()
                        });
                    }
                }
            }
        }
    }

    fn fps_render(&mut self) {
        if self.frame_durations.len() == 60 {
            let total: Duration = self.frame_durations.iter().sum();
            let total = total.as_secs() as f64 + total.subsec_nanos() as f64 / 1_000_000_000.0;
            let average = total / 60.0;
            self.fps = format!("{:.0}", 1.0 / average);
            self.frame_durations.clear();
        }

        self.glyph_brush.queue(Section {
            text: &self.fps,
            color: [1.0, 1.0, 1.0, 1.0],
            screen_position: (self.width as f32 - 30.0, 4.0),
            scale: GlyphScale::uniform(20.0),
            .. Section::default()
        });
    }

    fn debug_lines_render(&mut self, lines: &[String]) {
        if lines.len() > 1 {
            for (i, line) in lines.iter().enumerate() {
                self.glyph_brush.queue(Section {
                    text: line,
                    color: [1.0, 1.0, 0.0, 1.0],
                    screen_position: (0.0, 12.0 + 20.0 * i as f32),
                    scale: GlyphScale::uniform(20.0),
                    .. Section::default()
                });
            }
        }
    }

    fn render_buffers(
        &self,
        pipeline:   &RenderPipeline,
        rpass:      &mut RenderPass,
        render:     &RenderGame,
        buffers:    Buffers,
        entity:     &Matrix4<f32>,
        edge_color: [f32; 4],
        color:      [f32; 4]
    ) {
        let zoom = render.camera.zoom.recip();
        let aspect_ratio = self.aspect_ratio();
        let camera = Matrix4::from_nonuniform_scale(zoom, zoom * aspect_ratio, 1.0);
        let transformation = camera * entity;
        let uniform = Uniform {
            edge_color,
            color,
            transformation: transformation.into(),
        };

        #[derive(Clone, Copy)]
        #[allow(dead_code)]
        #[repr(C)]
        struct Uniform {
            edge_color:     [f32; 4],
            color:          [f32; 4],
            transformation: [[f32; 4]; 4],
        }
        let uniform_buffer = self.device.create_buffer_mapped(1, wgpu::BufferUsage::UNIFORM)
            .fill_from_slice(&[uniform]);

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout_surface,
            bindings: &[wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer {
                    buffer: &uniform_buffer,
                    range: 0..mem::size_of::<Uniform>() as wgpu::BufferAddress,
                }
            }]
        });

        rpass.set_pipeline(pipeline);
        rpass.set_bind_group(0, &bind_group, &[]);
        rpass.set_index_buffer(&buffers.index, 0);
        rpass.set_vertex_buffers(&[(&buffers.vertex, 0)]);
        rpass.draw_indexed(0 .. buffers.index_count, 0, 0 .. 1);
    }

    fn render_surface_buffers(
        &self,
        pipeline: &RenderPipeline,
        rpass:    &mut RenderPass,
        render:   &RenderGame,
        buffers:  ColorBuffers,
        entity:   &Matrix4<f32>,
    ) {
        let zoom = render.camera.zoom.recip();
        let aspect_ratio = self.aspect_ratio();
        let camera = Matrix4::from_nonuniform_scale(zoom, zoom * aspect_ratio, 1.0);
        let transformation = camera * entity;
        let uniform = ColorUniform { transformation: transformation.into() };

        #[derive(Clone, Copy)]
        #[allow(dead_code)]
        #[repr(C)]
        struct ColorUniform {
            transformation: [[f32; 4]; 4],
        }
        let uniform_buffer = self.device.create_buffer_mapped(1, wgpu::BufferUsage::UNIFORM)
            .fill_from_slice(&[uniform]);

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout_surface,
            bindings: &[wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer {
                    buffer: &uniform_buffer,
                    range: 0..mem::size_of::<ColorUniform>() as u64,
                }
            }]
        });

        rpass.set_pipeline(pipeline);
        rpass.set_bind_group(0, &bind_group, &[]);
        rpass.set_index_buffer(&buffers.index, 0);
        rpass.set_vertex_buffers(&[(&buffers.vertex, 0)]);
        rpass.draw_indexed(0 .. buffers.index_count, 0, 0 .. 1);
    }

    fn game_render(&mut self, render: RenderGame, rpass: &mut RenderPass, command_output: &[String]) {
        let mut rng = StdRng::from_seed(render.seed);
        if command_output.len() == 0 {
            self.game_hud_render(&render.entities);
            self.game_timer_render(&render.timer);
            self.debug_lines_render(&render.debug_lines);
            self.fps_render();
        }
        else {
            self.command_render(command_output);
        }

        let pan = render.camera.pan;

        match render.state {
            GameState::Local  => { }
            GameState::Paused => {
                // TODO: blue vaporwavey background lines to indicate pause :D
                // also double as measuring/scale lines
                // configurable size via treeflection
                // but this might be desirable to have during normal gameplay to, hmmmm....
                // Just have a 5 second fade out time so it doesnt look clunky and can be used during frame advance
            }
            _ => { }
        }

        if let Some(buffers) = ColorBuffers::new_surfaces(&self.device, &render.surfaces) {
            let transformation = Matrix4::from_translation(Vector3::new(pan.0, pan.1, 0.6001));
            self.render_surface_buffers(&self.pipeline_surface, rpass, &render, buffers, &transformation);
        }

        if let Some(buffers) = ColorBuffers::new_surfaces_fill(&self.device, &render.surfaces) {
            let transformation = Matrix4::from_translation(Vector3::new(pan.0, pan.1, 0.6002));
            self.render_surface_buffers(&self.pipeline_surface, rpass, &render, buffers, &transformation);
        }

        if let Some(buffers) = ColorBuffers::new_selected_surfaces(&self.device, &render.surfaces, &render.selected_surfaces) {
            let transformation = Matrix4::from_translation(Vector3::new(pan.0, pan.1, 0.6));
            self.render_surface_buffers(&self.pipeline_surface, rpass, &render, buffers, &transformation);
        }

        for (i, entity) in render.entities.iter().enumerate() {
            let z_debug        = 0.1  - i as f32 * 0.00001;
            let z_particle_fg  = 0.2  - i as f32 * 0.00001;
            //let z_shield     = 0.4  - i as f32 * 0.00001; // used in transparent pass below
            let z_respawn_plat = 0.45 - i as f32 * 0.00001;
            let z_player       = 0.5  - i as f32 * 0.00001;
            let z_particle_bg  = 0.8  - i as f32 * 0.00001;
            match entity {
                &RenderEntity::Player(ref player) => {
                    // draw player ecb
                    if player.debug.ecb {
                        let buffers = Buffers::new_player(&self.device, &player);
                        let edge_color = [0.0, 1.0, 0.0, 1.0];
                        let color = if player.fighter_selected {
                            [0.0, 1.0, 0.0, 1.0]
                        } else {
                            [1.0, 1.0, 1.0, 1.0]
                        };
                        let dir      = Matrix4::from_nonuniform_scale(if player.frames[0].face_right { 1.0 } else { -1.0 }, 1.0, 1.0);
                        let position = Matrix4::from_translation(Vector3::new(player.frames[0].bps.0 + pan.0, player.frames[0].bps.1 + pan.1, z_player));
                        let transformation = position * dir;

                        self.render_buffers(&self.pipeline, rpass, &render, buffers, &transformation, edge_color, color);
                    }

                    fn player_matrix(frame: &RenderPlayerFrame, pan: (f32, f32), z_player: f32) -> Matrix4<f32> {
                        let dir      = Matrix4::from_nonuniform_scale(if frame.face_right { 1.0 } else { -1.0 }, 1.0, 1.0);
                        let rotate   = Matrix4::from_angle_z(Rad(frame.angle));
                        let position = Matrix4::from_translation(Vector3::new(frame.bps.0 + pan.0, frame.bps.1 + pan.1, z_player));
                        position * rotate * dir
                    }

                    let transformation = player_matrix(&player.frames[0], pan, z_player);

                    // draw fighter
                    match player.debug.fighter {
                        RenderFighter::Normal | RenderFighter::Debug | RenderFighter::OnionSkin => {
                            if let RenderFighter::OnionSkin = player.debug.fighter {
                                if let Some(frame) = player.frames.get(2) {
                                    if let Some(buffers) = Buffers::new_fighter_frame(&self.device, &self.package.as_ref().unwrap(), &frame.fighter, frame.action, frame.frame) {
                                        let transformation = player_matrix(frame, pan, z_player);
                                        let onion_color = [0.4, 0.4, 0.4, 0.4];
                                        self.render_buffers(&self.pipeline, rpass, &render, buffers.clone(), &transformation, onion_color, onion_color);
                                    }
                                }

                                if let Some(frame) = player.frames.get(1) {
                                    if let Some(buffers) = Buffers::new_fighter_frame(&self.device, &self.package.as_ref().unwrap(), &frame.fighter, frame.action, frame.frame) {
                                        let transformation = player_matrix(frame, pan, z_player);
                                        let onion_color = [0.80, 0.80, 0.80, 0.9];
                                        self.render_buffers(&self.pipeline, rpass, &render, buffers.clone(), &transformation, onion_color, onion_color);
                                    }
                                }
                            }

                            // draw fighter
                            if let Some(buffers) = Buffers::new_fighter_frame(&self.device, &self.package.as_ref().unwrap(), &player.frames[0].fighter, player.frames[0].action, player.frames[0].frame) {
                                let color = if let RenderFighter::Debug = player.debug.fighter {
                                    [0.0, 0.0, 0.0, 0.0]
                                } else {
                                    [0.9, 0.9, 0.9, 1.0]
                                };
                                let edge_color = if player.fighter_selected {
                                    [0.0, 1.0, 0.0, 1.0]
                                } else {
                                    let c = player.fighter_color.clone();
                                    [c[0], c[1], c[2], 1.0]
                                };
                                self.render_buffers(&self.pipeline, rpass, &render, buffers.clone(), &transformation, edge_color, color);
                            }
                            else {
                                 // TODO: Give some indication that we are rendering a deleted or otherwise nonexistent frame
                            }
                        }
                        RenderFighter::None => { }
                    }

                    // draw selected colboxes
                    if player.selected_colboxes.len() > 0 {
                        let color = [0.0, 1.0, 0.0, 1.0];
                        // TODO
                        let buffers = Buffers::new_fighter_frame_colboxes(&self.device, &self.package.as_ref().unwrap(), &player.frames[0].fighter, player.frames[0].action, player.frames[0].frame, &player.selected_colboxes);
                        self.render_buffers(&self.pipeline, rpass, &render, buffers, &transformation, color, color);
                    }

                    let arrow_buffers = Buffers::new_arrow(&self.device);

                    // draw hitbox debug arrows
                    if player.debug.hitbox_vectors {
                        let kbg_color = [1.0,  1.0,  1.0, 1.0];
                        let bkb_color = [0.17, 0.17, 1.0, 1.0];
                        for colbox in player.frame_data.colboxes.iter() {
                            if let CollisionBoxRole::Hit(ref hitbox) = colbox.role {
                                let kb_squish = 0.5;
                                let squish_kbg = Matrix4::from_nonuniform_scale(0.6, hitbox.kbg * kb_squish, 1.0);
                                let squish_bkb = Matrix4::from_nonuniform_scale(0.3, (hitbox.bkb / 100.0) * kb_squish, 1.0); // divide by 100 so the arrows are comparable if the hit fighter is on 100%
                                let rotate = Matrix4::from_angle_z(Rad(hitbox.angle.to_radians() - f32::consts::PI / 2.0));
                                let x = player.frames[0].bps.0 + pan.0 + colbox.point.0;
                                let y = player.frames[0].bps.1 + pan.1 + colbox.point.1;
                                let position = Matrix4::from_translation(Vector3::new(x, y, z_debug));
                                let transformation_bkb = position * rotate * squish_bkb;
                                let transformation_kbg = position * rotate * squish_kbg;
                                self.render_buffers(&self.pipeline, rpass, &render, arrow_buffers.clone(), &transformation_kbg, kbg_color.clone(), kbg_color.clone());
                                self.render_buffers(&self.pipeline, rpass, &render, arrow_buffers.clone(), &transformation_bkb, bkb_color.clone(), bkb_color.clone());
                            }
                        }
                    }

                    // draw debug vector arrows
                    let num_arrows = player.vector_arrows.len() as f32;
                    for (i, arrow) in player.vector_arrows.iter().enumerate() {
                        let squish = Matrix4::from_nonuniform_scale((num_arrows - i as f32) / num_arrows, 1.0, 1.0); // consecutive arrows are drawn slightly thinner so we can see arrows behind
                        let rotate = Matrix4::from_angle_z(Rad(arrow.y.atan2(arrow.x) - f32::consts::PI / 2.0));
                        let position = Matrix4::from_translation(Vector3::new(player.frames[0].bps.0 + pan.0, player.frames[0].bps.1 + pan.1, z_debug));
                        let transformation = position * rotate * squish;
                        self.render_buffers(&self.pipeline, rpass, &render, arrow_buffers.clone(), &transformation, arrow.color.clone(), arrow.color.clone());
                    }

                    // draw particles
                    let triangle_buffers = Buffers::new_triangle(&self.device);
                    let jump_buffers = Buffers::new_circle(&self.device);
                    for particle in &player.particles {
                        let c = particle.color.clone();
                        match &particle.p_type {
                            &ParticleType::Spark { size, background, .. } => {
                                let rotate = Matrix4::from_angle_z(Rad(particle.angle));
                                let size = size * (1.0 - particle.counter_mult());
                                let size = Matrix4::from_nonuniform_scale(size, size, 1.0);
                                let position = Matrix4::from_translation(Vector3::new(
                                    particle.x + pan.0,
                                    particle.y + pan.1,
                                    if background { z_particle_bg } else { z_particle_fg }
                                ));
                                let transformation = position * rotate * size;
                                let color = [c[0], c[1], c[2], 1.0];
                                let pipeline = if c[0] == 1.0 && c[1] == 1.0 && c[2] == 1.0 {
                                    //self.pipelines.wireframe.clone() // TODO
                                    &self.pipeline
                                } else {
                                    &self.pipeline
                                };
                                self.render_buffers(pipeline, rpass, &render, triangle_buffers.clone(), &transformation, color, color)
                            }
                            &ParticleType::AirJump => {
                                let size = Matrix4::from_nonuniform_scale(3.0 + particle.counter_mult(), 1.15 + particle.counter_mult(), 1.0);
                                let position = Matrix4::from_translation(Vector3::new(particle.x + pan.0, particle.y + pan.1, z_particle_bg));
                                let transformation = position * size;
                                let color = [c[0], c[1], c[2], (1.0 - particle.counter_mult()) * 0.7];
                                self.render_buffers(&self.pipeline, rpass, &render, jump_buffers.clone(), &transformation, color, color)
                            }
                            &ParticleType::Hit { knockback, damage } => {
                                // needs to rendered last to ensure we dont have anything drawn on top of the inversion
                                let size = Matrix4::from_nonuniform_scale(0.2 * knockback, 0.08 * damage, 1.0);
                                let rotate = Matrix4::from_angle_z(Rad(particle.angle - f32::consts::PI / 2.0));
                                let position = Matrix4::from_translation(Vector3::new(particle.x + pan.0, particle.y + pan.1, z_particle_fg));
                                let transformation = position * rotate * size;
                                let color = [0.5, 0.5, 0.5, 1.5];
                                self.render_buffers(&self.pipeline, rpass, &render, jump_buffers.clone(), &transformation, color, color) // TODO: Invert
                            }
                        }
                    }

                    // Draw spawn plat
                    match Action::from_u64(player.frames[0].action as u64) {
                        Some(Action::ReSpawn) | Some(Action::ReSpawnIdle) => {
                            // TODO: get width from player dimensions
                            let width = 15.0;
                            let height = width / 4.0;
                            let scale = Matrix4::from_nonuniform_scale(width, -height, 1.0); // negative y to point triangle downwards.
                            let rotate = Matrix4::from_angle_z(Rad(player.frames[0].angle));
                            let bps = &player.frames[0].bps;
                            let position = Matrix4::from_translation(Vector3::new(bps.0 + pan.0, bps.1 + pan.1, z_respawn_plat));
                            let transformation = position * rotate * scale;

                            let c = player.fighter_color.clone();
                            let color = [c[0], c[1], c[2], 1.0];

                            self.render_buffers(&self.pipeline, rpass, &render, triangle_buffers.clone(), &transformation, color, color)
                        }
                        _ => { }
                    }
                }
                &RenderEntity::RectOutline (ref render_rect) => {
                    let transformation = Matrix4::from_translation(Vector3::new(pan.0, pan.1, 0.0));
                    let color = render_rect.color;
                    let buffers = Buffers::rect_outline_buffers(&self.device, &render_rect.rect);
                    self.render_buffers(&self.pipeline, rpass, &render, buffers, &transformation, color, color);
                }

                &RenderEntity::SpawnPoint (ref render_point) => {
                    let buffers = Buffers::new_spawn_point(&self.device);
                    let flip = Matrix4::from_nonuniform_scale(if render_point.face_right { 1.0 } else { -1.0 }, 1.0, 1.0);
                    let position = Matrix4::from_translation(Vector3::new(render_point.x + pan.0, render_point.y + pan.1, z_debug));
                    let transformation = position * flip;
                    self.render_buffers(&self.pipeline, rpass, &render, buffers, &transformation, render_point.color.clone(), render_point.color.clone())
                }
            }
        }

        // Some things need to be rendered after everything else as they are transparent
        for (i, entity) in render.entities.iter().enumerate() {
            let z_shield = 0.4 - i as f32 * 0.00001;
            match entity {
                &RenderEntity::Player(ref player) => {
                    // draw shield
                    if let &Some(ref shield) = &player.shield {
                        let position = Matrix4::from_translation(Vector3::new(shield.pos.0 + pan.0, shield.pos.1 + pan.1, z_shield));
                        let buffers = Buffers::new_shield(&self.device, shield);
                        let color = if shield.distort > 0 {
                            let c = shield.color;
                            [c[0] * rng.gen_range(0.75, 1.25), c[1] * rng.gen_range(0.75, 1.25), c[2] * rng.gen_range(0.75, 1.25), c[3] * rng.gen_range(0.8, 1.2)]
                        } else {
                            shield.color
                        };
                        self.render_buffers(&self.pipeline, rpass, &render, buffers, &position, shield.color, color);
                    }
                }
                _ => { }
            }
        }
    }

    fn menu_render(&mut self, _render: RenderMenu, rpass: &mut RenderPass, _command_output: &[String]) {
        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.draw(0 .. 3, 0 .. 1);
    }

    fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    /// returns true iff succeeds
    fn handle_events(&mut self) -> bool {
        // We need to force send the resolution and dpi every frame because OsInput may receive the normal events while it isn't listening for them.
        if let Some(resolution) = self.window.get_inner_size() {
            // force send the current resolution
            let event = Event::WindowEvent {
                window_id: self.window.id(),
                event: WindowEvent::Resized(resolution)
            };

            if let Err(_) = self.os_input_tx.send(event) {
                return false;
            }
        } else {
            // MS Windows removes the window immediately on close before the process ends
            return false;
        }

        // force send the current dpi
        let event = Event::WindowEvent {
            window_id: self.window.id(),
            event: WindowEvent::HiDpiFactorChanged(self.window.get_hidpi_factor())
        };
        if let Err(_) = self.os_input_tx.send(event) {
            return false;
        }

        let os_input_tx = self.os_input_tx.clone();
        self.event_loop.poll_events(|event| {
            os_input_tx.send(event).ok();
        });
        true
    }
}
