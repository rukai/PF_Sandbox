mod buffers;

use self::buffers::{Vertex, Buffers, PackageBuffers};
use ::game::{GameState, RenderEntity, RenderGame};
use ::menu::{RenderMenu, RenderMenuState, CharacterSelect};
use ::graphics::{self, GraphicsMessage, Render, RenderRect};
use ::player::{RenderFighter, RenderPlayer, DebugPlayer};
use ::fighter::{Action, ECB};
use ::records::GameResult;
use ::package::Verify;

use vulkano_win;
use vulkano_win::VkSurfaceBuild;
use vulkano;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{DynamicState, AutoCommandBufferBuilder, CommandBufferBuilder};
use vulkano::descriptor::descriptor_set::{SimpleDescriptorSet, SimpleDescriptorSetBuf};
use vulkano::descriptor::pipeline_layout::PipelineLayoutAbstract;
use vulkano::device::{Device, Queue};
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, Subpass, RenderPassAbstract};
use vulkano::image::SwapchainImage;
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::swapchain::{Swapchain, SurfaceTransform, AcquireError, PresentMode};
use vulkano::sync::{GpuFuture, FlushError};
use vulkano_text::{DrawText, DrawTextTrait, UpdateTextCache};
use winit::{Event, WindowEvent, WindowBuilder, EventsLoop};

use std::mem;
use std::sync::Arc;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread;
use std::time::{Duration, Instant};
use std::collections::HashSet;
use std::iter;

mod vs {
    #[derive(VulkanoShader)]
    #[ty = "vertex"]
    #[path = "src/shaders/generic-vertex.glsl"]
    #[allow(dead_code)]
    struct Dummy;
}

mod fs {
    #[derive(VulkanoShader)]
    #[ty = "fragment"]
    #[path = "src/shaders/generic-fragment.glsl"]
    #[allow(dead_code)]
    struct Dummy;
}

pub struct Uniform {
    uniform:  Arc<CpuAccessibleBuffer<vs::ty::Data>>,
    set:      Arc<SimpleDescriptorSet<((), SimpleDescriptorSetBuf<Arc<CpuAccessibleBuffer<vs::ty::Data>>>)>>,
}

pub struct VulkanGraphics<'a> {
    package_buffers: PackageBuffers,
    window:          vulkano_win::Window,
    events_loop:     EventsLoop,
    device:          Arc<Device>,
    future:          Box<GpuFuture>,
    swapchain:       Arc<Swapchain>,
    queue:           Arc<Queue>,
    pipeline:        Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>, Box<PipelineLayoutAbstract + Send + Sync>, Arc<RenderPassAbstract + Send + Sync>>>,
    render_pass:     Arc<RenderPassAbstract + Send + Sync>,
    framebuffers:    Vec<Arc<FramebufferAbstract + Send + Sync>>,
    uniforms:        Vec<Uniform>,
    draw_text:       DrawText<'a>,
    os_input_tx:     Sender<WindowEvent>,
    render_rx:       Receiver<GraphicsMessage>,
    frame_durations: Vec<Duration>,
    fps:             String,
    width:           u32,
    height:          u32,
}

impl<'a> VulkanGraphics<'a> {
    pub fn init(os_input_tx: Sender<WindowEvent>) -> Sender<GraphicsMessage> {
        let (render_tx, render_rx) = channel();

        thread::spawn(move || {
            let mut graphics = VulkanGraphics::new(os_input_tx, render_rx);
            graphics.run();
        });
        render_tx
    }

    fn new(os_input_tx: Sender<WindowEvent>, render_rx: Receiver<GraphicsMessage>) -> VulkanGraphics<'a> {
        let instance = {
            let extensions = vulkano_win::required_extensions();
            Instance::new(None, &extensions, None).expect("failed to create Vulkan instance")
        };

        let physical = PhysicalDevice::enumerate(&instance).next().expect("no device available");
        let events_loop = EventsLoop::new();
        let window = WindowBuilder::new().build_vk_surface(&events_loop, instance.clone()).unwrap();
        window.window().set_title("PF Sandbox");

        let queue = physical.queue_families().find(|&q| {
            q.supports_graphics() && window.surface().is_supported(q).unwrap_or(false)
        }).unwrap();

        let (device, mut queues) = {
            let device_ext = vulkano::device::DeviceExtensions {
                khr_swapchain: true,
                .. vulkano::device::DeviceExtensions::none()
            };
            Device::new(&physical, physical.supported_features(), &device_ext, [(queue, 0.5)].iter().cloned()).unwrap()
        };

        let future = Box::new(vulkano::sync::now(device.clone())) as Box<GpuFuture>;

        let queue = queues.next().unwrap();

        let (swapchain, images) = {
            let caps = window.surface().capabilities(physical).unwrap();
            let dimensions = caps.current_extent.unwrap_or([640, 480]);
            let alpha = caps.supported_composite_alpha.iter().next().unwrap();
            let format = caps.supported_formats[0].0;
            Swapchain::new(device.clone(), window.surface().clone(), caps.min_image_count, format, dimensions, 1,
                caps.supported_usage_flags, &queue, SurfaceTransform::Identity, alpha, PresentMode::Fifo, true, None
            ).unwrap()
        };


        let (uniforms, render_pass, pipeline, framebuffers) = VulkanGraphics::pipeline(device.clone(), queue.clone(), swapchain.clone(), &images);

        let draw_text = DrawText::new(device.clone(), queue.clone(), swapchain.clone(), &images);

        VulkanGraphics {
            package_buffers:  PackageBuffers::new(),
            window:           window,
            events_loop:      events_loop,
            device:           device,
            future:           future,
            swapchain:        swapchain,
            queue:            queue,
            pipeline:         pipeline,
            render_pass:      render_pass,
            framebuffers:     framebuffers,
            uniforms:         uniforms,
            draw_text:        draw_text,
            os_input_tx:      os_input_tx,
            render_rx:        render_rx,
            frame_durations:  vec!(),
            fps:              String::new(),
            width:            0,
            height:           0,
        }
    }

    fn pipeline(
        device: Arc<Device>,
        queue: Arc<Queue>,
        swapchain: Arc<Swapchain>,
        images: &[Arc<SwapchainImage>]
    ) -> (
        Vec<Uniform>,
        Arc<RenderPassAbstract + Send + Sync>,
        Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>, Box<PipelineLayoutAbstract + Send + Sync>, Arc<RenderPassAbstract + Send + Sync>>>,
        Vec<Arc<FramebufferAbstract + Send + Sync>>
    ) {
        let render_pass = Arc::new(single_pass_renderpass!(device.clone(),
            attachments: {
                color: {
                    load:    Clear,
                    store:   Store,
                    format:  swapchain.format(),
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        ).unwrap()) as Arc<RenderPassAbstract + Send + Sync>;

        let framebuffers = images.iter().map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                .add(image.clone()).unwrap()
                .build().unwrap()
            ) as Arc<FramebufferAbstract + Send + Sync>
        }).collect::<Vec<_>>();

        let vs = vs::Shader::load(&device).unwrap();
        let fs = fs::Shader::load(&device).unwrap();

        let pipeline = Arc::new(GraphicsPipeline::start()
            .vertex_input_single_buffer()
            .vertex_shader(vs.main_entry_point(), ())
            .triangle_list()
            .viewports(iter::once(Viewport {
                origin:      [0.0, 0.0],
                depth_range: 0.0..1.0,
                dimensions:  [
                    images[0].dimensions()[0] as f32,
                    images[0].dimensions()[1] as f32
                ],
            }))
            .fragment_shader(fs.main_entry_point(), ())
            .blend_alpha_blending()
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap()
        );

        let mut uniforms: Vec<Uniform> = vec!();
        for _ in 0..1000 {
            let uniform = CpuAccessibleBuffer::<vs::ty::Data>::from_data(
                device.clone(),
                BufferUsage::all(),
                Some(queue.family()),
                vs::ty::Data {
                    position_offset: [0.0, 0.0],
                    zoom:            1.0,
                    aspect_ratio:    1.0,
                    direction:       1.0,
                    edge_color:      [1.0, 1.0, 1.0, 1.0],
                    color:           [1.0, 1.0, 1.0, 1.0],
                    _dummy0:         [0; 12],
                }
            ).unwrap();

            let set = Arc::new(simple_descriptor_set!(pipeline.clone(), 0, {
                uniforms: uniform.clone()
            }));

            uniforms.push(Uniform {
                uniform: uniform,
                set: set
            });
        }

        (uniforms, render_pass, pipeline, framebuffers)
    }

    fn run(&mut self) {
        loop {
            {
                let frame_start = Instant::now();
                self.render_fps();

                // get the most recent render
                let mut render = if let Ok(message) = self.render_rx.recv() {
                    self.read_message(message)
                } else {
                    return;
                };
                while let Ok(message) = self.render_rx.try_recv() {
                    render = self.read_message(message);
                }

                let (new_width, new_height) = self.window.window().get_inner_size_points().unwrap();
                if self.width != new_width || self.height != new_height {
                    self.window_resize(new_width, new_height);
                }
                self.render(render);
                self.frame_durations.push(frame_start.elapsed());
            }
            self.handle_events();
        }
    }

    fn render_fps(&mut self) {
        if self.frame_durations.len() == 60 {
            let total: Duration = self.frame_durations.iter().sum();
            let total = total.as_secs() as f64 + total.subsec_nanos() as f64 / 1_000_000_000.0;
            let average = total / 60.0;
            self.fps = format!("{:.0}", 1.0 / average);
            self.frame_durations.clear();
        }

        self.draw_text.queue_text(self.width as f32 - 30.0, 20.0, 20.0, [1.0, 1.0, 1.0, 1.0], &self.fps);
    }

    fn read_message(&mut self, message: GraphicsMessage) -> Render {
        self.package_buffers.update(self.device.clone(), self.queue.clone(), message.package_updates);
        message.render
    }

    fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    fn window_resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;

        let (new_swapchain, new_images) = self.swapchain.recreate_with_dimension([width, height]).unwrap();
        self.swapchain = new_swapchain.clone();

        let (uniforms, render_pass, pipeline, framebuffers) = VulkanGraphics::pipeline(self.device.clone(), self.queue.clone(), new_swapchain, &new_images);
        self.uniforms = uniforms;
        self.render_pass = render_pass;
        self.pipeline = pipeline;
        self.framebuffers = framebuffers;

        self.draw_text = DrawText::new(self.device.clone(), self.queue.clone(), self.swapchain.clone(), &new_images);
    }

    fn render(&mut self, render: Render) {
        self.future.cleanup_finished();
        let (image_num, new_future) = match vulkano::swapchain::acquire_next_image(self.swapchain.clone(), Duration::new(1, 0)) {
            Ok(result) => { result }
            Err(AcquireError::OutOfDate) => {
                // Just abort this render, the user wont care about losing some frames while resizing. Internal rendering size will be fixed by next frame.
                return;
            }
            Err(err) => { panic!("{:?}", err) }
        };

        let final_command_buffer = match render {
            Render::Game(game) => { self.game_render(game, image_num) },
            Render::Menu(menu) => { self.menu_render(menu, image_num) },
        }.build().unwrap();

        let mut old_future = Box::new(vulkano::sync::now(self.device.clone())) as Box<GpuFuture>; // TODO: Can I avoid making this dummy future?
        mem::swap(&mut self.future, &mut old_future);

        let future_result = old_future.join(new_future)
            .then_execute(self.queue.clone(), final_command_buffer).unwrap()
            .then_swapchain_present(self.queue.clone(), self.swapchain.clone(), image_num)
            .then_signal_fence_and_flush();

        self.future = match future_result {
            Ok(value) => { Box::new(value) as Box<_> }
            Err(FlushError::OutOfDate) => {
                // Just abort this render, the user wont care about losing some frames while resizing. Internal rendering size will be fixed by next frame.
                return;
            }
            Err(err) => { panic!("{:?}", err) }
        };
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
                self.draw_text.queue_text(location, self.height as f32 - 50.0, 110.0, player.fighter_color, format!("{}%", player.damage).as_ref());
            }
        }
    }

    fn game_render(&mut self, render: RenderGame, image_num: usize) -> AutoCommandBufferBuilder {
        self.game_hud_render(&render.entities);

        let zoom = render.camera.zoom.recip();
        let pan  = render.camera.pan;
        let aspect_ratio = self.aspect_ratio();

        let mut command_buffer = AutoCommandBufferBuilder::new(self.device.clone(), self.queue.family()).unwrap()
        .update_text_cache(&mut self.draw_text)
        .begin_render_pass(self.framebuffers[image_num].clone(), false, vec![[0.0, 0.0, 0.0, 1.0].into()]).unwrap();

        match render.state {
            GameState::Local  => { }
            GameState::Paused => {
                // TODO: blue vaporwavey background lines to indicate pause :D
                // also double as measuring/scale lines
                // configurable size via treeflection
                // but this might be desirable to have during normal gameplay to, hmmmm....
                // Just have a 5 second fade in/out time so it doesnt look clunky and can be used during frame advance
            }
            _ => { }
        }

        let stage: &str = render.stage.as_ref();
        let mut uniforms = self.uniforms.iter();
        let uniform = uniforms.next().unwrap();
        {
            let mut buffer_content = uniform.uniform.write().unwrap();
            buffer_content.zoom            = zoom;
            buffer_content.aspect_ratio    = aspect_ratio;
            buffer_content.position_offset = [pan.0 as f32, pan.1 as f32];
            buffer_content.direction       = 1.0;
            buffer_content.edge_color      = [1.0, 1.0, 1.0, 1.0];
            buffer_content.color           = [1.0, 1.0, 1.0, 1.0];
        }
        if let &Some(ref buffers) = &self.package_buffers.stages[stage] {
            command_buffer = command_buffer.draw_indexed(self.pipeline.clone(), DynamicState::none(), buffers.vertex.clone(), buffers.index.clone(), uniform.set.clone(), ()).unwrap();
        }

        for entity in render.entities {
            match entity {
                RenderEntity::Player(player) => {
                    let dir = if player.face_right { 1.0 } else { -1.0 } as f32;
                    let draw_pos = [player.bps.0 + pan.0 as f32, player.bps.1 + pan.1 as f32];
                    // draw player ecb
                    if player.debug.ecb {
                        let buffers = Buffers::new_player(self.device.clone(), self.queue.clone(), &player);
                        let uniform = uniforms.next().unwrap();
                        {
                            let mut buffer_content = uniform.uniform.write().unwrap();
                            buffer_content.zoom            = zoom;
                            buffer_content.aspect_ratio    = aspect_ratio;
                            buffer_content.position_offset = draw_pos;
                            buffer_content.direction       = dir;
                            buffer_content.edge_color      = [0.0, 1.0, 0.0, 1.0];
                            if player.fighter_selected {
                                buffer_content.color = [0.0, 1.0, 0.0, 1.0];
                            }
                            else {
                                buffer_content.color = [1.0, 1.0, 1.0, 1.0];
                            }
                        }
                        command_buffer = command_buffer.draw_indexed(self.pipeline.clone(), DynamicState::none(), buffers.vertex.clone(), buffers.index.clone(), uniform.set.clone(), ()).unwrap();
                    }

                    // setup fighter uniform
                    match player.debug.fighter {
                        RenderFighter::Normal | RenderFighter::Debug => {
                            let uniform = uniforms.next().unwrap();
                            {
                                let mut buffer_content = uniform.uniform.write().unwrap();
                                buffer_content.zoom            = zoom;
                                buffer_content.aspect_ratio    = aspect_ratio;
                                buffer_content.position_offset = draw_pos;
                                buffer_content.direction       = dir;
                                if let RenderFighter::Debug = player.debug.fighter {
                                    buffer_content.color = [0.0, 0.0, 0.0, 0.0];
                                }
                                else {
                                    buffer_content.color = [1.0, 1.0, 1.0, 1.0];
                                }
                                if player.fighter_selected {
                                    buffer_content.edge_color = [0.0, 1.0, 0.0, 1.0];
                                }
                                else {
                                    buffer_content.edge_color = player.fighter_color;
                                }
                            }

                            // draw fighter
                            let fighter_frames = &self.package_buffers.fighters[&player.fighter][player.action];
                            if player.frame < fighter_frames.len() {
                                if let &Some(ref buffers) = &fighter_frames[player.frame] {
                                    command_buffer = command_buffer.draw_indexed(self.pipeline.clone(), DynamicState::none(), buffers.vertex.clone(), buffers.index.clone(), uniform.set.clone(), ()).unwrap();
                                }
                            }
                            else {
                                 // TODO: Give some indication that we are rendering a deleted or otherwise nonexistent frame
                            }
                        }
                        RenderFighter::None => { }
                    }

                    // draw selected hitboxes
                    if player.selected_colboxes.len() > 0 {
                        // I could store which element each vertex is part of and handle this in the shader but then I wouldn't be able to highlight overlapping elements.
                        // The extra vertex generation + draw should be fast enough (this only occurs on the pause screen)
                        let uniform = uniforms.next().unwrap();
                        {
                            let mut buffer_content = uniform.uniform.write().unwrap();
                            buffer_content.zoom            = zoom;
                            buffer_content.aspect_ratio    = aspect_ratio;
                            buffer_content.position_offset = [player.bps.0 + pan.0 as f32, player.bps.1 + pan.1 as f32];
                            buffer_content.direction       = if player.face_right { 1.0 } else { -1.0 } as f32;
                            buffer_content.edge_color      = [0.0, 1.0, 0.0, 1.0];
                            buffer_content.color           = [0.0, 1.0, 0.0, 1.0];
                        }
                        let buffers = self.package_buffers.fighter_frame_colboxes(self.device.clone(), self.queue.clone(), &player.fighter, player.action, player.frame, &player.selected_colboxes);
                        command_buffer = command_buffer.draw_indexed(self.pipeline.clone(), DynamicState::none(), buffers.vertex.clone(), buffers.index.clone(), uniform.set.clone(), ()).unwrap();
                    }

                    // TODO: Edit::Player  - render selected player's BPS as green
                    // TODO: Edit::Fighter - Click and drag on ECB points
                    // TODO: Edit::Stage   - render selected platforms as green
                },
                RenderEntity::Selector(rect) => {
                    let uniform = uniforms.next().unwrap();
                    {
                        let mut buffer_content = uniform.uniform.write().unwrap();
                        buffer_content.zoom            = zoom;
                        buffer_content.aspect_ratio    = aspect_ratio;
                        buffer_content.position_offset = [pan.0 as f32, pan.1 as f32];
                        buffer_content.direction       = 1.0;
                        buffer_content.edge_color      = [0.0, 1.0, 0.0, 1.0];
                        buffer_content.color           = [0.0, 1.0, 0.0, 1.0];
                    }
                    let buffers = Buffers::rect_outline_buffers(self.device.clone(), self.queue.clone(), rect);
                    command_buffer = command_buffer.draw_indexed(self.pipeline.clone(), DynamicState::none(), buffers.vertex.clone(), buffers.index.clone(), uniform.set.clone(), ()).unwrap();
                },
                RenderEntity::Area(rect) => {
                    let uniform = uniforms.next().unwrap();
                    {
                        let mut buffer_content = uniform.uniform.write().unwrap();
                        buffer_content.zoom            = zoom;
                        buffer_content.aspect_ratio    = aspect_ratio;
                        buffer_content.position_offset = [pan.0 as f32, pan.1 as f32];
                        buffer_content.direction       = 1.0;
                        buffer_content.edge_color      = [0.0, 1.0, 0.0, 1.0];
                        buffer_content.color           = [0.0, 1.0, 0.0, 1.0]; // HMMM maybe i can use only the edge to get the outline from a normal rect?
                    }
                    let buffers = Buffers::rect_outline_buffers(self.device.clone(), self.queue.clone(), rect);
                    command_buffer = command_buffer.draw_indexed(self.pipeline.clone(), DynamicState::none(), buffers.vertex.clone(), buffers.index.clone(), uniform.set.clone(), ()).unwrap();
                },
            }
        }
        command_buffer
        .draw_text(&mut self.draw_text, self.queue.clone(), self.width, self.height)
        .end_render_pass().unwrap()
    }

    fn menu_render(&mut self, render: RenderMenu, image_num: usize) -> AutoCommandBufferBuilder {
        let mut entities: Vec<MenuEntity> = vec!();
        match render.state {
            RenderMenuState::GameSelect (selection) => {
                self.draw_game_selector(selection);
                self.draw_package_banner(&render.package_verify);
            }
            RenderMenuState::ReplaySelect (replay_names, selection) => {
                self.draw_replay_selector(&replay_names, selection);
                self.draw_package_banner(&render.package_verify);
            }
            RenderMenuState::CharacterSelect (selections, back_counter, back_counter_max) => {
                let mut plugged_in_controller_indexes: Vec<usize>            = vec!();
                let mut plugged_in_selections:         Vec<&CharacterSelect> = vec!();

                for (i, selection) in selections.iter().enumerate() {
                    if selection.plugged_in {
                        plugged_in_selections.push(selection);
                        plugged_in_controller_indexes.push(i);
                    }
                }

                match plugged_in_selections.len() {
                    0 => {
                        self.draw_text.queue_text(100.0, 50.0, 30.0, [1.0, 1.0, 1.0, 1.0], "There are no controllers plugged in.");
                    }
                    1 => {
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[0], plugged_in_selections[0], -0.9, -0.8, 0.9, 0.9);
                    }
                    2 => {
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[0], plugged_in_selections[0], -0.9, -0.8, 0.0, 0.9);
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[1], plugged_in_selections[1],  0.0, -0.8, 0.9, 0.9);
                    }
                    3 => {
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[0], plugged_in_selections[0], -0.9, -0.8, 0.0, 0.0);
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[1], plugged_in_selections[1],  0.0, -0.8, 0.9, 0.0);
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[2], plugged_in_selections[2], -0.9,  0.0, 0.0, 0.9);
                    }
                    4 => {
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[0], plugged_in_selections[0], -0.9, -0.8, 0.0, 0.0);
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[1], plugged_in_selections[1],  0.0, -0.8, 0.9, 0.0);
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[2], plugged_in_selections[2], -0.9,  0.0, 0.0, 0.9);
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[3], plugged_in_selections[3],  0.0,  0.0, 0.9, 0.9);
                    }
                    _ => {
                        self.draw_text.queue_text(100.0, 50.0, 30.0, [1.0, 1.0, 1.0, 1.0], "Currently only supports up to 4 controllers. Please unplug some.");
                    }
                }
                self.draw_back_counter(&mut entities, back_counter, back_counter_max);
                self.draw_package_banner(&render.package_verify);
            }
            RenderMenuState::StageSelect (selection) => {
                self.draw_stage_selector(&mut entities, selection);
                self.draw_package_banner(&render.package_verify);
            }
            RenderMenuState::GameResults (results) => {
                let max = results.len() as f32;
                for (i, result) in results.iter().enumerate() {
                    let i = i as f32;
                    let start_x = i / max;
                    self.draw_player_result(result, start_x);
                }
            }
            RenderMenuState::SetRules => {
                self.draw_text.queue_text(100.0, 50.0, 30.0, [1.0, 1.0, 1.0, 1.0], "set rules");
            }
            RenderMenuState::PackageSelect (ref names, selection) => {
                self.draw_package_selector(names, selection);
            }
            RenderMenuState::BrowsePackages => {
                self.draw_text.queue_text(100.0, 50.0, 30.0, [1.0, 1.0, 1.0, 1.0], "browse package");
            }
            RenderMenuState::CreatePackage => {
                self.draw_text.queue_text(100.0, 50.0, 30.0, [1.0, 1.0, 1.0, 1.0], "create package");
            }
            RenderMenuState::CreateFighter => {
                self.draw_text.queue_text(100.0, 50.0, 30.0, [1.0, 1.0, 1.0, 1.0], "create fighter");
            }
        }

        let mut command_buffer = AutoCommandBufferBuilder::new(self.device.clone(), self.queue.family()).unwrap()
        .update_text_cache(&mut self.draw_text)
        .begin_render_pass(self.framebuffers[image_num].clone(), false, vec![[0.0, 0.0, 0.0, 1.0].into()]).unwrap();

        for (i, entity) in entities.iter().enumerate() {
            let uniform = self.uniforms[i].set.clone();
            match entity {
                &MenuEntity::Fighter { ref fighter, action, frame } => {
                    let fighter_frames = &self.package_buffers.fighters[fighter][action];
                    if frame < fighter_frames.len() {
                        if let &Some(ref buffers) = &fighter_frames[frame] {
                            command_buffer = command_buffer.draw_indexed(self.pipeline.clone(), DynamicState::none(), buffers.vertex.clone(), buffers.index.clone(), uniform, ()).unwrap();
                        }
                    }
                }
                &MenuEntity::Stage (ref stage) => {
                    let stage: &str = stage.as_ref();
                    if let &Some(ref buffers) = &self.package_buffers.stages[stage] {
                        command_buffer = command_buffer.draw_indexed(self.pipeline.clone(), DynamicState::none(), buffers.vertex.clone(), buffers.index.clone(), uniform, ()).unwrap();
                    }
                }
                &MenuEntity::Rect (ref rect) => {
                    let buffers = Buffers::rect_buffers(self.device.clone(), self.queue.clone(), rect.clone());
                    command_buffer = command_buffer.draw_indexed(self.pipeline.clone(), DynamicState::none(), buffers.vertex.clone(), buffers.index.clone(), uniform, ()).unwrap();
                }
            }
        }

        command_buffer
        .draw_text(&mut self.draw_text, self.queue.clone(), self.width, self.height)
        .end_render_pass().unwrap()
    }

    // TODO: Rewrite text rendering to be part of scene instead of just plastered on top
    // TODO: Then this bar can be drawn on top of the package banner text
    fn draw_back_counter(&mut self, entities: &mut Vec<MenuEntity>, back_counter: usize, back_counter_max: usize) {
        let uniform = &self.uniforms[entities.len()];
        {
            let mut buffer_content = uniform.uniform.write().unwrap();
            buffer_content.zoom            = 1.0;
            buffer_content.aspect_ratio    = 1.0;
            buffer_content.position_offset = [0.0, 0.0];
            buffer_content.direction       = 1.0;
            buffer_content.edge_color      = [1.0, 1.0, 1.0, 1.0];
            buffer_content.color           = [1.0, 1.0, 1.0, 1.0];
        }

        entities.push(MenuEntity::Rect (RenderRect {
            p1: ( -1.0, -0.85),
            p2: (back_counter as f32 / back_counter_max as f32 * 2.0 - 1.0, -1.0),
        }));
    }

    fn draw_package_banner(&mut self, verify: &Verify) {
        let package = &self.package_buffers.package.as_ref().unwrap();
        let color: [f32; 4] = if let &Verify::Ok = verify {
            [0.0, 1.0, 0.0, 1.0]
        } else {
            [1.0, 0.0, 0.0, 1.0]
        };

        let message = match verify {
            &Verify::Ok => {
                format!("{} - {}", package.meta.title, package.meta.source)
            }
            &Verify::IncorrectHash => {
                format!("{} - {} - The computed hash did not match the hash given by the host", package.meta.title, package.meta.source)
            }
            &Verify::UpdateAvailable => {
                format!("{} - {} - There is an update available from the host", package.meta.title, package.meta.source)
            }
            &Verify::CannotConnect => {
                format!("{} - {} - Cannot connect to package host", package.meta.title, package.meta.source)
            }
            &Verify::None => {
                unreachable!();
            }
        };

        self.draw_text.queue_text(30.0, self.height as f32 - 30.0, 30.0, color, message.as_str());
    }

    fn draw_player_result(&mut self, result: &GameResult, start_x: f32) {
        let fighter_name = self.package_buffers.package.as_ref().unwrap().fighters[result.fighter.as_ref()].name.as_ref();
        let color = graphics::get_controller_color(result.controller);
        let x = (start_x + 0.05) * self.width as f32;
        let mut y = 100.0;
        self.draw_text.queue_text(x, y, 100.0, color, (result.place + 1).to_string().as_ref());
        y += 50.0;
        self.draw_text.queue_text(x, y, 30.0, color, fighter_name);
        y += 30.0;
        self.draw_text.queue_text(x, y, 30.0, color, format!("Kills: {}", result.kills.len()).as_str());
        y += 30.0;
        self.draw_text.queue_text(x, y, 30.0, color, format!("Deaths: {}", result.deaths.len()).as_str());
        y += 30.0;
        self.draw_text.queue_text(x, y, 30.0, color, format!("L-Cancel Success: {}%", result.lcancel_percent).as_str());
    }

    fn draw_fighter_selector(&mut self, menu_entities: &mut Vec<MenuEntity>, controller_i: usize, selection: &CharacterSelect, start_x: f32, start_y: f32, end_x: f32, end_y: f32) {
        self.draw_text.queue_text(100.0, 50.0, 50.0, [1.0, 1.0, 1.0, 1.0], "Select Fighters");
        let fighters = &self.package_buffers.package.as_ref().unwrap().fighters;
        for (fighter_i, fighter) in fighters.key_value_iter().enumerate() {
            let (fighter_key, fighter) = fighter;
            let x_offset = if fighter_i == selection.ticker.cursor { 0.1 } else { 0.0 };
            let x = ((start_x+1.0 + x_offset) / 2.0) * self.width  as f32;
            let y = ((start_y+1.0           ) / 2.0) * self.height as f32 + fighter_i as f32 * 50.0;

            let size = 26.0; // TODO: determine from width/height of screen and start/end pos

            let mut color = [1.0, 1.0, 1.0, 1.0];
            if let Some(selection_i) = selection.selection {
                if fighter_i == selection_i {
                    color = graphics::get_controller_color(controller_i);

                    // fudge player data (One day I would like to have the menu selection fighters (mostly) playable)
                    let player = RenderPlayer {
                        debug:             DebugPlayer::default(),
                        damage:            0.0,
                        stocks:            0,
                        bps:               (0.0, 0.0),
                        ecb:               ECB::default(),
                        frame:             0,
                        action:            Action::Idle as usize,
                        fighter:           fighter_key.clone(),
                        face_right:        start_x < 0.0,
                        fighter_color:     color,
                        fighter_selected:  false,
                        player_selected:   false,
                        selected_colboxes: HashSet::new(),
                    };

                    // draw fighter
                    let fighter_frames = &self.package_buffers.fighters[&player.fighter][player.action];
                    if player.frame < fighter_frames.len() {
                        // TODO: dynamically calculate position and zoom (fit width/height of fighter into selection area)
                        let zoom = 40.0;
                        let fighter_x = start_x + (end_x - start_x) / 2.0;
                        let fighter_y = end_y - 0.2; // HACK: dont know why the fighters are drawing so low, so just put them 0.2 higher
                        let fighter_x_scaled = fighter_x * zoom;
                        let fighter_y_scaled = fighter_y * zoom * -1.0 + player.bps.1;
                        let uniform = &self.uniforms[menu_entities.len()];
                        {
                            let mut buffer_content = uniform.uniform.write().unwrap();
                            buffer_content.zoom            = 1.0 / zoom;
                            buffer_content.aspect_ratio    = self.aspect_ratio();
                            buffer_content.position_offset = [fighter_x_scaled, fighter_y_scaled];
                            buffer_content.direction       = if player.face_right { 1.0 } else { -1.0 } as f32;
                            buffer_content.color           = [1.0, 1.0, 1.0, 1.0];
                            buffer_content.edge_color      = color;
                        }

                        if let &Some(_) = &fighter_frames[player.frame] {
                            menu_entities.push(MenuEntity::Fighter {
                                fighter: player.fighter,
                                action:  player.action,
                                frame:   player.frame
                            });
                        }
                    }
                }
            }
            self.draw_text.queue_text(x, y, size, color, fighter.name.as_ref());
        }
    }

    fn draw_stage_selector(&mut self, menu_entities: &mut Vec<MenuEntity>, selection: usize) {
        self.draw_text.queue_text(100.0, 50.0, 50.0, [1.0, 1.0, 1.0, 1.0], "Select Stage");
        let stages = &self.package_buffers.package.as_ref().unwrap().stages;
        for (stage_i, stage) in stages.key_value_iter().enumerate() {
            let (stage_key, stage) = stage;
            let size = 26.0; // TODO: determine from width/height of screen and start/end pos
            let x_offset = if stage_i == selection { 0.1 } else { 0.0 };
            let x = self.width as f32 * (0.1 + x_offset);
            let y = self.height as f32 * 0.1 + stage_i as f32 * 50.0;
            self.draw_text.queue_text(x, y, size, [1.0, 1.0, 1.0, 1.0], stage.name.as_ref());

            if stage_i == selection {
                let uniform = &self.uniforms[menu_entities.len()];
                {
                    let zoom = 100.0;
                    let y = -0.2 * zoom;
                    let mut buffer_content = uniform.uniform.write().unwrap();
                    buffer_content.zoom            = 1.0 / zoom;
                    buffer_content.aspect_ratio    = self.aspect_ratio();
                    buffer_content.position_offset = [0.0, y];
                    buffer_content.direction       = 1.0;
                    buffer_content.edge_color      = [1.0, 1.0, 1.0, 1.0];
                    buffer_content.color           = [1.0, 1.0, 1.0, 1.0];
                }

                menu_entities.push(MenuEntity::Stage(stage_key.clone()));
            }
        }
    }

    fn draw_package_selector(&mut self, package_names: &[String], selection: usize) {
        self.draw_text.queue_text(100.0, 50.0, 50.0, [1.0, 1.0, 1.0, 1.0], "Select Package");
        //self.draw_text.queue_text(100.0, self.height as f32 - 30.0, 30.0, [1.0, 1.0, 1.0, 1.0], "A: Select package    X/Y: Select Package without updating");

        for (package_i, name) in package_names.iter().enumerate() {
            let size = 26.0; // TODO: determine from width/height of screen and start/end pos
            let x_offset = if package_i == selection { 0.1 } else { 0.0 };
            let x = self.width as f32 * (0.1 + x_offset);
            let y = self.height as f32 * 0.1 + package_i as f32 * 50.0;
            self.draw_text.queue_text(x, y, size, [1.0, 1.0, 1.0, 1.0], name.as_ref());
        }
    }

    fn draw_game_selector(&mut self, selection: usize) {
        self.draw_text.queue_text(100.0, 50.0, 50.0, [1.0, 1.0, 1.0, 1.0], "Select Game Mode");

        let modes = vec!("Local", "Host Game", "Connect To Game |AddressInputBox|", "Replays");
        for (mode_i, name) in modes.iter().enumerate() {
            let size = 26.0; // TODO: determine from width/height of screen and start/end pos
            let x_offset = if mode_i == selection { 0.1 } else { 0.0 };
            let x = self.width as f32 * (0.1 + x_offset);
            let y = self.height as f32 * 0.1 + mode_i as f32 * 50.0;
            self.draw_text.queue_text(x, y, size, [1.0, 1.0, 1.0, 1.0], name.as_ref());
        }
    }

    fn draw_replay_selector(&mut self, replay_names: &[String], selection: usize) {
        self.draw_text.queue_text(100.0, 50.0, 50.0, [1.0, 1.0, 1.0, 1.0], "Select Replay");

        for (replay_i, name) in replay_names.iter().enumerate() {
            let size = 26.0; // TODO: determine from width/height of screen and start/end pos
            let x_offset = if replay_i == selection { 0.1 } else { 0.0 };
            let x = self.width as f32 * (0.1 + x_offset);
            let y = self.height as f32 * 0.1 + replay_i as f32 * 50.0;
            self.draw_text.queue_text(x, y, size, [1.0, 1.0, 1.0, 1.0], name.as_ref());
        }
    }

    fn handle_events(&mut self) {
        // force send the current resolution
        let window = self.window.window();
        let res = window.get_inner_size_points().unwrap();
        self.os_input_tx.send(WindowEvent::Resized(res.0, res.1)).unwrap();

        self.events_loop.poll_events(|event| {
            let Event::WindowEvent {event, ..} = event;
            self.os_input_tx.send(event).unwrap();
        });
    }
}

enum MenuEntity {
    Fighter { fighter: String, action: usize, frame: usize },
    Stage   (String),
    Rect    (RenderRect),
}
