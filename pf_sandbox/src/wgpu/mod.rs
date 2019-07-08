/// There is a lot of duplicated code between this and the vulkan renderer.
/// But thats ok because the vulkan renderer will be deleted once the wgpu renderer reaches feature parity.
mod buffers;

use buffers::{ColorVertex, ColorBuffers, Buffers};
use crate::graphics::{GraphicsMessage, Render, RenderType};
use crate::game::{GameState, RenderEntity, RenderGame};
use crate::menu::{RenderMenu};

use std::sync::mpsc::{Sender, Receiver, channel};
use std::{thread, mem};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use wgpu::{Device, SwapChain, BindGroup, BindGroupLayout, RenderPipeline, RenderPass};
use cgmath::{Matrix4, Vector3};

use winit::{
    Event,
    EventsLoop,
    Window,
    WindowEvent,
};

pub struct WgpuGraphics {
    window:                    Window,
    event_loop:                EventsLoop,
    os_input_tx:               Sender<Event>,
    render_rx:                 Receiver<GraphicsMessage>,
    device:                    Device,
    swap_chain:                Option<SwapChain>,
    render_pipeline:           RenderPipeline,
    pipeline_surface:          RenderPipeline,
    bind_group:                BindGroup,
    bind_group_layout_surface: BindGroupLayout,
    prev_fullscreen:           Option<bool>,
    width:                     u32,
    height:                    u32,
}

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

        let device = adapter.request_device(&wgpu::DeviceDescriptor {
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
            rasterization_state: wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            },
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor {
                format: wgpu::TextureFormat::Bgra8Unorm,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: None,
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[],
            sample_count: 1,
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
            rasterization_state: wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            },
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor {
                format: wgpu::TextureFormat::Bgra8Unorm,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
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
            sample_count: 1,
        });

        let swap_chain = Some(device.create_swap_chain(
            &surface,
            &wgpu::SwapChainDescriptor {
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                format: wgpu::TextureFormat::Bgra8Unorm,
                width: size.width.round() as u32,
                height: size.height.round() as u32,
            },
        ));

        WgpuGraphics {
            window,
            event_loop,
            os_input_tx,
            render_rx,
            device,
            swap_chain,
            render_pipeline,
            pipeline_surface,
            bind_group,
            bind_group_layout_surface,
            prev_fullscreen: None,
            width:           0,
            height:          0,
        }
    }

    fn run(&mut self) {
        loop {
            println!("HI");
            {

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
            }

            if !self.handle_events() {
                println!("aaa");
                return;
            }
        }
    }

    fn read_message(&mut self, message: GraphicsMessage) -> Render {
        //self.package_buffers.update(self.device.clone(), message.package_updates);
        message.render
    }

    fn window_resize(&mut self, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }

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
                        attachment: &frame.view,
                        resolve_target: None,
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
            self.device.get_queue().submit(&[encoder.finish()]);
        }
        self.swap_chain = Some(swap_chain);
    }

    fn _render_buffers(
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
                    range: 0..1
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
                    range: 0..1
                }
            }]
        });

        rpass.set_pipeline(pipeline);
        rpass.set_bind_group(0, &bind_group, &[]);
        rpass.set_index_buffer(&buffers.index, 0);
        rpass.set_vertex_buffers(&[(&buffers.vertex, 0)]);
        rpass.draw_indexed(0 .. buffers.index_count, 0, 0 .. 1);
    }

    fn game_render(&mut self, render: RenderGame, rpass: &mut RenderPass, _command_output: &[String]) {
        let mut rng = StdRng::from_seed(render.seed);

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

        // Some things need to be rendered after everything else as they are transparent
        for (i, entity) in render.entities.iter().enumerate() {
            let z_shield = 0.4 - i as f32 * 0.00001;
            match entity {
                &RenderEntity::Player(ref player) => {
                    // draw shield
                    if let &Some(ref shield) = &player.shield {
                        let _position = Matrix4::from_translation(Vector3::new(shield.pos.0 + pan.0, shield.pos.1 + pan.1, z_shield));
                        let _buffers = Buffers::new_shield(&self.device, shield);
                        let _color = if shield.distort > 0 {
                            let c = shield.color;
                            [c[0] * rng.gen_range(0.75, 1.25), c[1] * rng.gen_range(0.75, 1.25), c[2] * rng.gen_range(0.75, 1.25), c[3] * rng.gen_range(0.8, 1.2)]
                        } else {
                            shield.color
                        };
                        //self.render_buffers(self.pipelines_standard, command_buffer, &render, buffers, &position, shield.color, color);
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
