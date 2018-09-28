mod buffers;

use pf_sandbox_lib::fighter::{Action, ECB, CollisionBoxRole, ActionFrame};
use pf_sandbox_lib::geometry::Rect;
use pf_sandbox_lib::json_upgrade;
use pf_sandbox_lib::package::Verify;
use self::buffers::{Vertex, ColorVertex, Buffers, ColorBuffers, PackageBuffers};
use game::{GameState, RenderEntity, RenderGame};
use graphics::{self, GraphicsMessage, Render, RenderType};
use menu::{RenderMenu, RenderMenuState, PlayerSelect, PlayerSelectUi};
use particle::ParticleType;
use player::{RenderFighter, RenderPlayer, DebugPlayer};
use results::PlayerResult;

use enum_traits::{FromIndex, ToIndex};
use cgmath::prelude::*;
use cgmath::{Matrix4, Vector3, Rad};
use rand::{StdRng, Rng, SeedableRng};
use vulkano_win;
use vulkano_win::VkSurfaceBuild;
use vulkano;
use vulkano::buffer::BufferUsage;
use vulkano::buffer::cpu_pool::CpuBufferPool;
use vulkano::command_buffer::{DynamicState, AutoCommandBufferBuilder};
use vulkano::descriptor::descriptor_set::{PersistentDescriptorSet, DescriptorSet};
use vulkano::descriptor::pipeline_layout::PipelineLayoutAbstract;
use vulkano::device::{Device, Queue};
use vulkano::format::{Format, ClearValue};
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, Subpass, RenderPassAbstract};
use vulkano::image::SwapchainImage;
use vulkano::image::attachment::AttachmentImage;
use vulkano::instance::{Instance, PhysicalDevice, PhysicalDeviceType};
use vulkano::pipeline::GraphicsPipeline;
//use vulkano::pipeline::blend::LogicOp;
use vulkano::pipeline::depth_stencil::{DepthStencil, DepthBounds, Compare};
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::viewport::Viewport;
use vulkano::swapchain::{Surface, Swapchain, SurfaceTransform, AcquireError, PresentMode, SwapchainCreationError};
use vulkano::sync::{GpuFuture, FlushError};
use vulkano_text::{DrawText, DrawTextTrait};
use winit::{Window, Event, WindowEvent, WindowBuilder, EventsLoop};

use std::collections::HashSet;
use std::f32;
use std::iter;
use std::mem;
use std::sync::Arc;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread;
use std::time::{Duration, Instant};

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

mod surface_vs {
    #[derive(VulkanoShader)]
    #[ty = "vertex"]
    #[path = "src/shaders/surface-vertex.glsl"]
    #[allow(dead_code)]
    struct Dummy;
}

mod surface_fs {
    #[derive(VulkanoShader)]
    #[ty = "fragment"]
    #[path = "src/shaders/surface-fragment.glsl"]
    #[allow(dead_code)]
    struct Dummy;
}

/// Returns a list of physical devices, ordered from most preferred to least preferred
fn get_physical_devices(instance: &Arc<Instance>, force_name: Option<String>) -> Vec<PhysicalDevice> {
    let mut devices: Vec<PhysicalDevice> = PhysicalDevice::enumerate(&instance).collect();
    assert_ne!(devices.len(), 0, "there are no physical devices");

    devices.sort_by_key(rank_physical_device);

    if let Some(force_name) = force_name {
        for device in devices.drain_filter(|x| x.name().to_lowercase() == force_name.to_lowercase()).collect::<Vec<_>>() {
            devices.insert(0, device);
        }
    }

    devices
}

/// Returns an integer ranking for a physical device, the lower number the higher it's preferred
fn rank_physical_device(physical: &PhysicalDevice) -> u32 {
    match physical.ty() {
        PhysicalDeviceType::DiscreteGpu => 0,
        PhysicalDeviceType::IntegratedGpu => 1,
        PhysicalDeviceType::VirtualGpu => 2,
        PhysicalDeviceType::Cpu => 3,
        PhysicalDeviceType::Other => 4,
    }
}

pub fn print_physical_devices(force_name: Option<String>) {
    let extensions = vulkano_win::required_extensions();
    let instance = Instance::new(None, &extensions, None).expect("failed to create Vulkan instance");

    println!("Physical vulkan devices, ordered by preference:");

    let physical_devices = get_physical_devices(&instance, force_name.clone());
    for (i, device) in physical_devices.iter().enumerate() {
        let chosen = if force_name.as_ref().map(|x| x.to_lowercase() == device.name().to_lowercase()).unwrap_or(false) {
            " (Forced by config)"
        } else if i == 0 {
            " (Automatically chosen)"
        } else {
            ""
        };

        println!("    name: '{}' type: {:?}{}", device.name(), device.ty(), chosen);
    }

    if let Some(force_name) = force_name {
        if physical_devices.iter().all(|x| x.name().to_lowercase() != force_name.to_lowercase()) {
            println!("Warning: The physical_device_name '{}' used in config did not match any devices.", force_name);
        }
    }
}

pub struct VulkanGraphics {
    package_buffers:             PackageBuffers,
    surface:                     Arc<Surface<Window>>,
    events_loop:                 EventsLoop,
    device:                      Arc<Device>,
    future:                      Box<GpuFuture>,
    swapchain:                   Arc<Swapchain<Window>>,
    queue:                       Arc<Queue>,
    vs:                          vs::Shader,
    fs:                          fs::Shader,
    surface_vs:                  surface_vs::Shader,
    surface_fs:                  surface_fs::Shader,
    pipelines:                   Pipelines,
    render_pass:                 Arc<RenderPassAbstract + Send + Sync>,
    framebuffers:                Vec<Arc<FramebufferAbstract + Send + Sync>>,
    uniform_buffer_pool:         CpuBufferPool<vs::ty::Data>,
    surface_uniform_buffer_pool: CpuBufferPool<surface_vs::ty::Data>,
    draw_text:                   DrawText,
    os_input_tx:                 Sender<WindowEvent>,
    render_rx:                   Receiver<GraphicsMessage>,
    frame_durations:             Vec<Duration>,
    fps:                         String,
    width:                       u32,
    height:                      u32,
    prev_fullscreen:             Option<bool>,
}

struct Pipelines {
    standard:  Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>,      Box<PipelineLayoutAbstract + Send + Sync>, Arc<RenderPassAbstract + Send + Sync>>>,
    invert:    Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>,      Box<PipelineLayoutAbstract + Send + Sync>, Arc<RenderPassAbstract + Send + Sync>>>,
    wireframe: Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>,      Box<PipelineLayoutAbstract + Send + Sync>, Arc<RenderPassAbstract + Send + Sync>>>,
    surface:   Arc<GraphicsPipeline<SingleBufferDefinition<ColorVertex>, Box<PipelineLayoutAbstract + Send + Sync>, Arc<RenderPassAbstract + Send + Sync>>>,
}

impl VulkanGraphics {
    pub fn init(os_input_tx: Sender<WindowEvent>, device_name: Option<String>) -> Sender<GraphicsMessage> {
        let (render_tx, render_rx) = channel();

        thread::spawn(move || {
            let mut graphics = VulkanGraphics::new(os_input_tx, render_rx, device_name);
            graphics.run();
        });
        render_tx
    }

    fn new(os_input_tx: Sender<WindowEvent>, render_rx: Receiver<GraphicsMessage>, device_name: Option<String>) -> VulkanGraphics {
        let instance = {
            let extensions = vulkano_win::required_extensions();
            Instance::new(None, &extensions, None).expect("failed to create Vulkan instance")
        };

        let physical_device = get_physical_devices(&instance, device_name).remove(0);

        let events_loop = EventsLoop::new();
        let surface = WindowBuilder::new().build_vk_surface(&events_loop, instance.clone()).unwrap();
        surface.window().set_title("PF Sandbox");

        let queue = physical_device.queue_families().find(|&q| {
            q.supports_graphics() && surface.is_supported(q).unwrap_or(false)
        }).unwrap();

        let (device, mut queues) = {
            let device_ext = vulkano::device::DeviceExtensions {
                khr_swapchain: true,
                .. vulkano::device::DeviceExtensions::none()
            };
            Device::new(physical_device, physical_device.supported_features(), &device_ext, [(queue, 0.5)].iter().cloned()).unwrap()
        };

        let vs = vs::Shader::load(device.clone()).unwrap();
        let fs = fs::Shader::load(device.clone()).unwrap();
        let surface_vs = surface_vs::Shader::load(device.clone()).unwrap();
        let surface_fs = surface_fs::Shader::load(device.clone()).unwrap();

        let future = Box::new(vulkano::sync::now(device.clone())) as Box<GpuFuture>;

        let queue = queues.next().unwrap();

        let (swapchain, images) = {
            let caps = surface.capabilities(physical_device).unwrap();
            let dimensions = caps.current_extent.unwrap_or([640, 480]);
            let alpha = caps.supported_composite_alpha.iter().next().unwrap();
            let format = caps.supported_formats[0].0;
            Swapchain::new(device.clone(), surface.clone(), caps.min_image_count, format, dimensions, 1,
                // Windows-nvidia driver doesnt support PresentMode::Immediate
                // linux-intel driver has rendering bugs on PresentMode:Fifo
                caps.supported_usage_flags, &queue, SurfaceTransform::Identity, alpha, PresentMode::Fifo, true, None
            ).unwrap()
        };

        let (render_pass, pipelines, framebuffers) = VulkanGraphics::pipeline(&vs, &fs, &surface_vs, &surface_fs, device.clone(), swapchain.clone(), &images);

        let draw_text = DrawText::new(device.clone(), queue.clone(), swapchain.clone(), &images);
        let uniform_buffer_pool = CpuBufferPool::<vs::ty::Data>::new(device.clone(), BufferUsage::all());
        let surface_uniform_buffer_pool = CpuBufferPool::<surface_vs::ty::Data>::new(device.clone(), BufferUsage::all());

        VulkanGraphics {
            surface,
            events_loop,
            device,
            future,
            swapchain,
            queue,
            vs,
            fs,
            surface_vs,
            surface_fs,
            pipelines,
            render_pass,
            framebuffers,
            uniform_buffer_pool,
            surface_uniform_buffer_pool,
            draw_text,
            os_input_tx,
            render_rx,
            package_buffers: PackageBuffers::new(),
            frame_durations: vec!(),
            fps:             String::new(),
            width:           0,
            height:          0,
            prev_fullscreen: None,
        }
    }

    fn pipeline(
        vs: &vs::Shader,
        fs: &fs::Shader,
        surface_vs: &surface_vs::Shader,
        surface_fs: &surface_fs::Shader,
        device: Arc<Device>,
        swapchain: Arc<Swapchain<Window>>,
        images: &[Arc<SwapchainImage<Window>>]
    ) -> (Arc<RenderPassAbstract + Send + Sync>, Pipelines, Vec<Arc<FramebufferAbstract + Send + Sync>>) {
        let render_pass = Arc::new(single_pass_renderpass!(device.clone(),
            attachments: {
                multisampled_color: {
                    load:    Clear,
                    store:   DontCare,
                    format:  swapchain.format(),
                    samples: 4,
                },
                resolve_color: {
                    load:    DontCare,
                    store:   Store,
                    format:  swapchain.format(),
                    samples: 1,
                },
                multisampled_depth: {
                    load:    Clear,
                    store:   DontCare,
                    format:  Format::D16Unorm,
                    samples: 4,
                },
                resolve_depth: {
                    load:    DontCare,
                    store:   Store,
                    format:  Format::D16Unorm,
                    samples: 1,
                    initial_layout: ImageLayout::Undefined,
                    final_layout: ImageLayout::DepthStencilAttachmentOptimal,
                }
            },
            pass: {
                color: [multisampled_color],
                depth_stencil: {multisampled_depth},
                resolve: [resolve_color]
            }
        ).unwrap()) as Arc<RenderPassAbstract + Send + Sync>;

        let dimensions = images[0].dimensions();
        let depth = AttachmentImage::transient(device.clone(), dimensions, Format::D16Unorm).unwrap();
        let multisampled_depth = AttachmentImage::transient_multisampled(device.clone(), dimensions, 4, Format::D16Unorm).unwrap();
        let multisampled_image = AttachmentImage::transient_multisampled(device.clone(), dimensions, 4, swapchain.format()).unwrap();
        let framebuffers = images.iter().map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                .add(multisampled_image.clone()).unwrap()
                .add(image.clone()).unwrap()
                .add(multisampled_depth.clone()).unwrap()
                .add(depth.clone()).unwrap()
                .build().unwrap()
            ) as Arc<FramebufferAbstract + Send + Sync>
        }).collect::<Vec<_>>();

        let dimensions = [dimensions[0] as f32, dimensions[1] as f32];
        let standard        = VulkanGraphics::pipeline_base(vs, fs, device.clone(), render_pass.clone(), dimensions, false, false);
        let invert          = VulkanGraphics::pipeline_base(vs, fs, device.clone(), render_pass.clone(), dimensions, true, false);
        let wireframe       = VulkanGraphics::pipeline_base(vs, fs, device.clone(), render_pass.clone(), dimensions, false, true);
        let builder = GraphicsPipeline::start()
            .vertex_input_single_buffer()
            .vertex_shader(surface_vs.main_entry_point(), ())
            .triangle_list()
            .viewports(iter::once(Viewport {
                origin:      [0.0, 0.0],
                depth_range: 0.0..1.0,
                dimensions
            }))
            .fragment_shader(surface_fs.main_entry_point(), ())
            .blend_alpha_blending()
            .depth_stencil(DepthStencil {
                depth_write:       true,
                depth_compare:     Compare::LessOrEqual,
                depth_bounds_test: DepthBounds::Disabled,
                stencil_front:     Default::default(),
                stencil_back:      Default::default(),
            })
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap());
        let surface = Arc::new(builder.build(device.clone()).unwrap());
        let pipelines = Pipelines { standard, invert, wireframe, surface };

        (render_pass, pipelines, framebuffers)
    }

    fn pipeline_base(
        vs: &vs::Shader,
        fs: &fs::Shader,
        device: Arc<Device>,
        render_pass: Arc<RenderPassAbstract + Send + Sync>,
        dimensions: [f32; 2],
        invert: bool,
        wireframe: bool)
        -> Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>, Box<PipelineLayoutAbstract + Send + Sync>, Arc<RenderPassAbstract + Send + Sync>>>
    {
        let builder = GraphicsPipeline::start()
            .vertex_input_single_buffer()
            .vertex_shader(vs.main_entry_point(), ())
            .triangle_list()
            .viewports(iter::once(Viewport {
                origin:      [0.0, 0.0],
                depth_range: 0.0..1.0,
                dimensions
            }))
            .fragment_shader(fs.main_entry_point(), ())
            .blend_alpha_blending()
            .depth_stencil(DepthStencil {
                depth_write:       true,
                depth_compare:     Compare::LessOrEqual,
                depth_bounds_test: DepthBounds::Disabled,
                stencil_front:     Default::default(),
                stencil_back:      Default::default(),
            })
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap());

        let builder = if invert {
            //builder.blend_logic_op(LogicOp::Invert) // Fixes crash on my laptop with intel device
            builder
        } else {
            builder
        };

        let builder = if wireframe {
            builder.polygon_mode_line()
        } else {
            builder
        };

        Arc::new(builder.build(device.clone()).unwrap())
    }

    fn new_uniform_set(&self, uniform: vs::ty::Data) -> Arc<DescriptorSet + Send + Sync> {
        let uniform_buffer = self.uniform_buffer_pool.next(uniform).unwrap();
        Arc::new(
            PersistentDescriptorSet::start(self.pipelines.standard.clone(), 0)
            .add_buffer(uniform_buffer).unwrap()
            .build().unwrap()
        )
    }

    fn new_surface_uniform_set(&self, uniform: surface_vs::ty::Data) -> Arc<DescriptorSet + Send + Sync> {
        let uniform_buffer = self.surface_uniform_buffer_pool.next(uniform).unwrap();
        Arc::new(
            PersistentDescriptorSet::start(self.pipelines.standard.clone(), 0)
            .add_buffer(uniform_buffer).unwrap()
            .build().unwrap()
        )
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
                if let Some(resolution) = self.surface.window().get_inner_size() {
                    let resolution: (u32, u32) = resolution.to_physical(self.surface.window().get_hidpi_factor()).into();
                    self.window_resize(resolution.0, resolution.1);
                }
                else {
                    return;
                }

                // TODO: Waiting on: https://github.com/tomaka/winit/issues/574
                #[cfg(target_os = "windows")]
                {
                    render.fullscreen = false;
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
        self.package_buffers.update(self.device.clone(), message.package_updates);
        message.render
    }

    fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    fn window_resize(&mut self, width: u32, height: u32) {
        if self.width == width && self.height == height
        {
            return;
        }

        // Prevents a host OoM when vk.CreateSwapchainKHR is called in recreate_with_dimension. Only occurs on my laptop in windows when minimizing. Seems like a driver issue so should be safe to remove this if it stops happening.
        if width == 0 || height == 0 {
            self.width = width; // force recreate swapchain when we return to sensible values
            self.height = height;
            return
        }

        match self.swapchain.recreate_with_dimension([width, height]) {
            Ok((new_swapchain, new_images)) => {
                self.width = width;
                self.height = height;
                self.swapchain = new_swapchain.clone();

                let (render_pass, pipelines, framebuffers) = VulkanGraphics::pipeline(&self.vs, &self.fs, &self.surface_vs, &self.surface_fs, self.device.clone(), new_swapchain, &new_images);
                self.render_pass = render_pass;
                self.pipelines = pipelines;
                self.framebuffers = framebuffers;

                self.draw_text = DrawText::new(self.device.clone(), self.queue.clone(), self.swapchain.clone(), &new_images);
            }
            Err(SwapchainCreationError::UnsupportedDimensions) => { } // Occurs when minimized on MS Windows as dimensions are (0, 0)
            Err(err) => { panic!("resize error: width={}, height={}, err={:?}", width, height, err) }
        }
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
                let monitor = self.surface.window().get_current_monitor();
                self.surface.window().set_fullscreen(Some(monitor));
            }
            else {
                self.surface.window().set_fullscreen(None);
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
        self.surface.window().hide_cursor(render.fullscreen && !in_game_paused);

        self.future.cleanup_finished();
        let (image_num, new_future) = match vulkano::swapchain::acquire_next_image(self.swapchain.clone(), None) {
            Ok(result) => { result }
            Err(AcquireError::OutOfDate) => {
                // Just abort this render, the user wont care about losing some frames while resizing. Internal rendering size will be fixed by next frame.
                return;
            }
            Err(err) => { panic!("{:?}", err) }
        };

        let command_buffer = AutoCommandBufferBuilder::primary_one_time_submit(self.device.clone(), self.queue.family()).unwrap()
        .begin_render_pass(self.framebuffers[image_num].clone(), false, vec![[0.0, 0.0, 0.0, 1.0].into(), ClearValue::None, 1f32.into(), ClearValue::None]).unwrap();

        let final_command_buffer = match render.render_type {
            RenderType::Game(game) => { self.game_render(game, command_buffer, &render.command_output) },
            RenderType::Menu(menu) => { self.menu_render(menu, command_buffer, &render.command_output) },
        }
        .end_render_pass().unwrap()
        .draw_text(&mut self.draw_text, image_num)
        .build().unwrap();

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

    fn command_render(&mut self, lines: &[String]) {
        // TODO: Render white text, with black background
        for (i, line) in lines.iter().enumerate() {
            self.draw_text.queue_text(0.05, self.height as f32 - 15.0 - 20.0 * i as f32, 20.0, [1.0, 1.0, 0.0, 1.0], line);
        }
    }

    fn game_timer_render(&mut self, timer: &Option<Duration>) {
        if let &Some(ref timer) = timer {
            let minutes = timer.as_secs() / 60;
            let seconds = timer.as_secs() % 60;
            self.draw_text.queue_text((self.width / 2) as f32 - 50.0, 35.0, 40.0, [1.0, 1.0, 1.0, 1.0], format!("{:02}:{:02}", minutes, seconds).as_ref());
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
                match Action::from_index(player.action as u64) {
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
                            self.draw_text.queue_text(location + 10.0, self.height as f32 - 130.0, 22.0, color, stocks_string.as_ref());
                        }
                        self.draw_text.queue_text(location, self.height as f32 - 47.0, 110.0, color, format!("{}%", player.damage).as_ref());
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

        self.draw_text.queue_text(self.width as f32 - 30.0, 20.0, 20.0, [1.0, 1.0, 1.0, 1.0], &self.fps);
    }

    fn debug_lines_render(&mut self, lines: &[String]) {
        if lines.len() > 1 {
            for (i, line) in lines.iter().enumerate() {
                self.draw_text.queue_text(0.05, 20.0 * (i+1) as f32, 20.0, [1.0, 1.0, 0.0, 1.0], line);
            }
        }
    }

    fn render_buffers(
        &self,
        pipeline:       Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>, Box<PipelineLayoutAbstract + Send + Sync>, Arc<RenderPassAbstract + Send + Sync>>>,
        command_buffer: AutoCommandBufferBuilder,
        render:         &RenderGame,
        buffers:        Buffers,
        entity:         &Matrix4<f32>,
        edge_color:     [f32; 4],
        color:          [f32; 4]
    ) -> AutoCommandBufferBuilder {
        let zoom = render.camera.zoom.recip();
        let aspect_ratio = self.aspect_ratio();
        let camera = Matrix4::from_nonuniform_scale(zoom, zoom * aspect_ratio, 1.0);
        let transformation = camera * entity;
        let uniform = vs::ty::Data {
            edge_color,
            color,
            transformation:  transformation.into(),
        };
        let uniform_buffer = self.uniform_buffer_pool.next(uniform).unwrap();
        let set = Arc::new(
            PersistentDescriptorSet::start(pipeline.clone(), 0)
            .add_buffer(uniform_buffer).unwrap()
            .build().unwrap()
        );
        command_buffer.draw_indexed(pipeline, &DynamicState::none(), buffers.vertex, buffers.index, set, ()).unwrap()
    }

    fn render_surface_buffers(
        &self,
        pipeline:       Arc<GraphicsPipeline<SingleBufferDefinition<ColorVertex>, Box<PipelineLayoutAbstract + Send + Sync>, Arc<RenderPassAbstract + Send + Sync>>>,
        command_buffer: AutoCommandBufferBuilder,
        render:         &RenderGame,
        buffers:        ColorBuffers,
        entity:         &Matrix4<f32>,
    ) -> AutoCommandBufferBuilder {
        let zoom = render.camera.zoom.recip();
        let aspect_ratio = self.aspect_ratio();
        let camera = Matrix4::from_nonuniform_scale(zoom, zoom * aspect_ratio, 1.0);
        let transformation = camera * entity;
        let uniform = surface_vs::ty::Data { transformation: transformation.into() };
        let uniform_buffer = self.surface_uniform_buffer_pool.next(uniform).unwrap();
        let set = Arc::new(
            PersistentDescriptorSet::start(pipeline.clone(), 0)
            .add_buffer(uniform_buffer).unwrap()
            .build().unwrap()
        );
        command_buffer.draw_indexed(pipeline, &DynamicState::none(), buffers.vertex, buffers.index, set, ()).unwrap()
    }

    fn game_render(&mut self, render: RenderGame, mut command_buffer: AutoCommandBufferBuilder, command_output: &[String]) -> AutoCommandBufferBuilder {
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
                // Just have a 5 second fade in/out time so it doesnt look clunky and can be used during frame advance
            }
            _ => { }
        }

        if let Some(buffers) = Buffers::new_surfaces(self.device.clone(), &render.surfaces) {
            let transformation = Matrix4::from_translation(Vector3::new(pan.0, pan.1, 0.6001));
            command_buffer = self.render_surface_buffers(self.pipelines.surface.clone(), command_buffer, &render, buffers, &transformation);
        }

        if let Some(buffers) = Buffers::new_surfaces_fill(self.device.clone(), &render.surfaces) {
            let transformation = Matrix4::from_translation(Vector3::new(pan.0, pan.1, 0.6002));
            command_buffer = self.render_surface_buffers(self.pipelines.surface.clone(), command_buffer, &render, buffers, &transformation);
        }

        if let Some(buffers) = Buffers::new_selected_surfaces(self.device.clone(), &render.surfaces, &render.selected_surfaces) {
            let transformation = Matrix4::from_translation(Vector3::new(pan.0, pan.1, 0.6));
            command_buffer = self.render_surface_buffers(self.pipelines.surface.clone(), command_buffer, &render, buffers, &transformation);
        }

        for (i, entity) in render.entities.iter().enumerate() {
            let z_debug       = 0.1 - i as f32 * 0.00001;
            let z_particle_fg = 0.2 - i as f32 * 0.00001;
            //let z_shield    = 0.4 - i as f32 * 0.00001; // used in transparent pass below
            let z_player      = 0.5 - i as f32 * 0.00001;
            let z_particle_bg = 0.8 - i as f32 * 0.00001;
            match entity {
                &RenderEntity::Player(ref player) => {
                    let dir      = Matrix4::from_nonuniform_scale(if player.face_right { 1.0 } else { -1.0 }, 1.0, 1.0);
                    let rotate   = Matrix4::from_angle_z(Rad(player.angle));
                    let position = Matrix4::from_translation(Vector3::new(player.bps.0 + pan.0, player.bps.1 + pan.1, z_player));

                    // draw player ecb
                    if player.debug.ecb {
                        let buffers = Buffers::new_player(self.device.clone(), &player);
                        let edge_color = [0.0, 1.0, 0.0, 1.0];
                        let color = if player.fighter_selected {
                            [0.0, 1.0, 0.0, 1.0]
                        } else {
                            [1.0, 1.0, 1.0, 1.0]
                        };
                        let transformation = position * dir;

                        command_buffer = self.render_buffers(self.pipelines.standard.clone(), command_buffer, &render, buffers, &transformation, edge_color, color);
                    }

                    // draw fighter
                    match player.debug.fighter {
                        RenderFighter::Normal | RenderFighter::Debug => {
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
                            let transformation = position * rotate * dir;

                            // draw fighter
                            if let Some(buffers) = self.package_buffers.new_fighter_frame(self.device.clone(), &player.fighter, player.action, player.frame) {
                                command_buffer = self.render_buffers(self.pipelines.standard.clone(), command_buffer, &render, buffers.clone(), &transformation, edge_color, color);
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
                        let transformation = position * rotate * dir;
                        let buffers = self.package_buffers.new_fighter_frame_colboxes(self.device.clone(), &player.fighter, player.action, player.frame, &player.selected_colboxes);
                        command_buffer = self.render_buffers(self.pipelines.standard.clone(), command_buffer, &render, buffers, &transformation, color, color);
                    }

                    let arrow_buffers = Buffers::new_arrow(self.device.clone());

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
                                let x = player.bps.0 + pan.0 + colbox.point.0;
                                let y = player.bps.1 + pan.1 + colbox.point.1;
                                let position = Matrix4::from_translation(Vector3::new(x, y, z_debug));
                                let transformation_bkb = position * rotate * squish_bkb;
                                let transformation_kbg = position * rotate * squish_kbg;
                                command_buffer = self.render_buffers(self.pipelines.standard.clone(), command_buffer, &render, arrow_buffers.clone(), &transformation_kbg, kbg_color.clone(), kbg_color.clone());
                                command_buffer = self.render_buffers(self.pipelines.standard.clone(), command_buffer, &render, arrow_buffers.clone(), &transformation_bkb, bkb_color.clone(), bkb_color.clone());
                            }
                        }
                    }

                    // draw debug vector arrows
                    let num_arrows = player.vector_arrows.len() as f32;
                    for (i, arrow) in player.vector_arrows.iter().enumerate() {
                        let squish = Matrix4::from_nonuniform_scale((num_arrows - i as f32) / num_arrows, 1.0, 1.0); // consecutive arrows are drawn slightly thinner so we can see arrows behind
                        let rotate = Matrix4::from_angle_z(Rad(arrow.y.atan2(arrow.x) - f32::consts::PI / 2.0));
                        let position = Matrix4::from_translation(Vector3::new(player.bps.0 + pan.0, player.bps.1 + pan.1, z_debug));
                        let transformation = position * rotate * squish;
                        command_buffer = self.render_buffers(self.pipelines.standard.clone(), command_buffer, &render, arrow_buffers.clone(), &transformation, arrow.color.clone(), arrow.color.clone());
                    }

                    // draw particles
                    let triangle_buffers = Buffers::new_triangle(self.device.clone());
                    let jump_buffers = Buffers::new_circle(self.device.clone());
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
                                    self.pipelines.wireframe.clone()
                                } else {
                                    self.pipelines.standard.clone()
                                };
                                command_buffer = self.render_buffers(pipeline, command_buffer, &render, triangle_buffers.clone(), &transformation, color, color)
                            }
                            &ParticleType::AirJump => {
                                let size = Matrix4::from_nonuniform_scale(3.0 + particle.counter_mult(), 1.15 + particle.counter_mult(), 1.0);
                                let position = Matrix4::from_translation(Vector3::new(particle.x + pan.0, particle.y + pan.1, z_particle_bg));
                                let transformation = position * size;
                                let color = [c[0], c[1], c[2], (1.0 - particle.counter_mult()) * 0.7];
                                command_buffer = self.render_buffers(self.pipelines.standard.clone(), command_buffer, &render, jump_buffers.clone(), &transformation, color, color)
                            }
                            &ParticleType::Hit { knockback, damage } => {
                                // needs to rendered last to ensure we dont have anything drawn on top of the inversion
                                let size = Matrix4::from_nonuniform_scale(0.2 * knockback, 0.08 * damage, 1.0);
                                let rotate = Matrix4::from_angle_z(Rad(particle.angle - f32::consts::PI / 2.0));
                                let position = Matrix4::from_translation(Vector3::new(particle.x + pan.0, particle.y + pan.1, z_particle_fg));
                                let transformation = position * rotate * size;
                                let color = [0.5, 0.5, 0.5, 1.5];
                                command_buffer = self.render_buffers(self.pipelines.invert.clone(), command_buffer, &render, jump_buffers.clone(), &transformation, color, color)
                            }
                        }
                    }

                    // TODO: Edit::Player  - render selected player's BPS as green
                    // TODO: Edit::Fighter - Click and drag on ECB points
                    // TODO: Edit::Stage   - render selected surfaces as green
                },
                &RenderEntity::RectOutline (ref render_rect) => {
                    let transformation = Matrix4::from_translation(Vector3::new(pan.0, pan.1, 0.0));
                    let color = render_rect.color;
                    let buffers = Buffers::rect_outline_buffers(self.device.clone(), &render_rect.rect);
                    command_buffer = self.render_buffers(self.pipelines.standard.clone(), command_buffer, &render, buffers, &transformation, color, color);
                },

                &RenderEntity::SpawnPoint (ref render_point) => {
                    let point = &render_point.point;
                    let buffers = Buffers::new_spawn_point(self.device.clone());
                    let flip = Matrix4::from_nonuniform_scale(if point.face_right { 1.0 } else { -1.0 }, 1.0, 1.0);
                    let position = Matrix4::from_translation(Vector3::new(point.x + pan.0, point.y + pan.1, z_debug));
                    let transformation = position * flip;
                    command_buffer = self.render_buffers(self.pipelines.standard.clone(), command_buffer, &render, buffers, &transformation, render_point.color.clone(), render_point.color.clone())
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
                        let buffers = Buffers::new_shield(self.device.clone(), shield);
                        let color = if shield.distort > 0 {
                            let c = shield.color;
                            [c[0] * rng.gen_range(0.75, 1.25), c[1] * rng.gen_range(0.75, 1.25), c[2] * rng.gen_range(0.75, 1.25), c[3] * rng.gen_range(0.8, 1.2)]
                        } else {
                            shield.color
                        };
                        command_buffer = self.render_buffers(self.pipelines.standard.clone(), command_buffer, &render, buffers, &position, shield.color, color);
                    }
                }
                _ => { }
            }
        }

        command_buffer
    }

    fn menu_render(&mut self, render: RenderMenu, mut command_buffer: AutoCommandBufferBuilder, command_output: &[String]) -> AutoCommandBufferBuilder {
        self.fps_render();
        let mut entities: Vec<MenuEntityAndSet> = vec!();
        match render.state {
            RenderMenuState::GameSelect (selection) => {
                self.draw_game_selector(selection);
                self.draw_package_banner(&render.package_verify, command_output);
            }
            RenderMenuState::ReplaySelect (replay_names, selection) => {
                self.draw_replay_selector(&replay_names, selection);
                self.draw_package_banner(&render.package_verify, command_output);
            }
            RenderMenuState::CharacterSelect (selections, back_counter, back_counter_max) => {
                let mut plugged_in_selections: Vec<(&PlayerSelect, usize)> = vec!();
                for (i, selection) in selections.iter().enumerate() {
                    if selection.ui.is_visible() {
                        plugged_in_selections.push((selection, i));
                    }
                }

                self.draw_back_counter(&mut entities, back_counter, back_counter_max);
                self.draw_text.queue_text(100.0, 50.0, 50.0, [1.0, 1.0, 1.0, 1.0], "Select Fighters");
                match plugged_in_selections.len() {
                    0 => {
                        self.draw_text.queue_text(100.0, 100.0, 30.0, [1.0, 1.0, 1.0, 1.0], "There are no controllers plugged in.");
                    }
                    1 => {
                        self.draw_fighter_selector(&mut entities, &plugged_in_selections, 0, -0.9, -0.8, 0.9, 0.9);

                    }
                    2 => {
                        self.draw_fighter_selector(&mut entities, &plugged_in_selections, 0, -0.9, -0.8, 0.0, 0.9);
                        self.draw_fighter_selector(&mut entities, &plugged_in_selections, 1,  0.0, -0.8, 0.9, 0.9);
                    }
                    3 => {
                        self.draw_fighter_selector(&mut entities, &plugged_in_selections, 0, -0.9, -0.8, 0.0, 0.0);
                        self.draw_fighter_selector(&mut entities, &plugged_in_selections, 1,  0.0, -0.8, 0.9, 0.0);
                        self.draw_fighter_selector(&mut entities, &plugged_in_selections, 2, -0.9,  0.0, 0.0, 0.9);
                    }
                    4 => {
                        self.draw_fighter_selector(&mut entities, &plugged_in_selections, 0, -0.9, -0.8, 0.0, 0.0);
                        self.draw_fighter_selector(&mut entities, &plugged_in_selections, 1,  0.0, -0.8, 0.9, 0.0);
                        self.draw_fighter_selector(&mut entities, &plugged_in_selections, 2, -0.9,  0.0, 0.0, 0.9);
                        self.draw_fighter_selector(&mut entities, &plugged_in_selections, 3,  0.0,  0.0, 0.9, 0.9);
                    }
                    _ => {
                        self.draw_text.queue_text(100.0, 100.0, 30.0, [1.0, 1.0, 1.0, 1.0], "Currently only supports up to 4 controllers. Please unplug some.");
                    }
                }
                self.draw_package_banner(&render.package_verify, command_output);
            }
            RenderMenuState::StageSelect (selection) => {
                self.draw_stage_selector(&mut entities, selection);
                self.draw_package_banner(&render.package_verify, command_output);
            }
            RenderMenuState::GameResults { results, replay_saved } => {
                let max = results.len() as f32;
                for (i, result) in results.iter().enumerate() {
                    let i = i as f32;
                    let start_x = i / max;
                    self.draw_player_result(result, start_x);
                }

                if replay_saved {
                    self.draw_text.queue_text(30.0, self.height as f32 - 30.0, 30.0, [1.0, 1.0, 1.0, 1.0], "Replay saved!");
                }
            }
            RenderMenuState::PackageSelect (ref names, selection, ref message) => {
                self.draw_package_selector(names, selection, message, command_output);
            }
            RenderMenuState::GenericText (ref text) => {
                self.draw_text.queue_text(100.0, 50.0, 30.0, [1.0, 1.0, 1.0, 1.0], text);
            }
        }

        for entity_and_set in entities {
            let set = entity_and_set.set;
            match entity_and_set.entity {
                MenuEntity::Fighter { ref fighter, action, frame } => {
                    if let Some(buffers) = self.package_buffers.new_fighter_frame(self.device.clone(), fighter, action, frame) {
                        command_buffer = command_buffer.draw_indexed(self.pipelines.standard.clone(), &DynamicState::none(), buffers.vertex.clone(), buffers.index.clone(), set, ()).unwrap();
                    }
                }
                MenuEntity::Stage (ref stage) => {
                    let stage: &str = stage.as_ref();
                    if let &Some(ref buffers) = &self.package_buffers.stages[stage] {
                        command_buffer = command_buffer.draw_indexed(self.pipelines.surface.clone(), &DynamicState::none(), buffers.vertex.clone(), buffers.index.clone(), set, ()).unwrap();
                    }
                }
                MenuEntity::StageFill (ref stage) => {
                    let stage: &str = stage.as_ref();
                    if let &Some(ref buffers) = &self.package_buffers.stages_fill[stage] {
                        command_buffer = command_buffer.draw_indexed(self.pipelines.surface.clone(), &DynamicState::none(), buffers.vertex.clone(), buffers.index.clone(), set, ()).unwrap();
                    }
                }
                MenuEntity::Rect (ref rect) => {
                    let buffers = Buffers::rect_buffers(self.device.clone(), rect.clone());
                    command_buffer = command_buffer.draw_indexed(self.pipelines.standard.clone(), &DynamicState::none(), buffers.vertex.clone(), buffers.index.clone(), set, ()).unwrap();
                }
            }
        }

        command_buffer
    }

    // TODO: Rewrite text rendering to be part of scene instead of just plastered on top
    // TODO: Then this bar can be drawn on top of the package banner text
    fn draw_back_counter(&mut self, entities: &mut Vec<MenuEntityAndSet>, back_counter: usize, back_counter_max: usize) {
        let uniform = vs::ty::Data {
            edge_color:      [1.0, 1.0, 1.0, 1.0],
            color:           [1.0, 1.0, 1.0, 1.0],
            transformation:  Matrix4::identity().into(),
        };
        let set = self.new_uniform_set(uniform);

        let entity = MenuEntity::Rect (Rect {
            x1: -1.0,
            y1: -0.85,
            x2: back_counter as f32 / back_counter_max as f32 * 2.0 - 1.0,
            y2: -1.0,
        });

        entities.push(MenuEntityAndSet { set, entity });
    }

    fn draw_package_banner(&mut self, verify: &Verify, command_output: &[String]) {
        if command_output.len() == 0 {
            let package = &self.package_buffers.package.as_ref().unwrap();
            let color: [f32; 4] = if let &Verify::Ok = verify {
                [0.0, 1.0, 0.0, 1.0]
            } else {
                [1.0, 0.0, 0.0, 1.0]
            };

            let message = if let Some(ref source) = package.meta.source {
                match verify {
                    &Verify::Ok => {
                        format!("{} - {}", package.meta.title, source)
                    }
                    &Verify::IncorrectHash => {
                        format!("{} - {} - The computed hash did not match the hash given by the host", package.meta.title, source)
                    }
                    &Verify::UpdateAvailable => {
                        format!("{} - {} - There is an update available from the host", package.meta.title, source)
                    }
                    &Verify::CannotConnect => {
                        format!("{} - {} - Cannot connect to package host", package.meta.title, source)
                    }
                    &Verify::None => {
                        unreachable!();
                    }
                }
            } else {
                package.meta.title.clone()
            };

            self.draw_text.queue_text(30.0, self.height as f32 - 30.0, 30.0, color, message.as_str());
        }
        else {
            self.command_render(command_output);
        }
    }

    fn draw_player_result(&mut self, result: &PlayerResult, start_x: f32) {
        let fighter_name = self.package_buffers.package.as_ref().unwrap().fighters[result.fighter.as_ref()].name.as_ref();
        let color = graphics::get_team_color4(result.team);
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

    fn draw_fighter_selector(&mut self, entities: &mut Vec<MenuEntityAndSet>, selections: &[(&PlayerSelect, usize)], i: usize, start_x: f32, start_y: f32, end_x: f32, end_y: f32) {
        let fighters = &self.package_buffers.package.as_ref().unwrap().fighters;
        let (selection, controller_i) = selections[i];

        // render player name
        {
            let x = ((start_x+1.0) / 2.0) * self.width  as f32;
            let y = ((start_y+1.0) / 2.0) * self.height as f32;
            let size = 26.0; // TODO: determine from width/height of screen and start/end pos
            let color = if let Some((check_i, _)) = selection.controller {
                // Use the team color of the controller currently manipulating this selection
                let mut team = 0;
                for val in selections {
                    let &(controller_selection, i) = val;
                    if check_i == i {
                        team = controller_selection.team;
                    }
                }
                graphics::get_team_color4(team)
            } else {
                [0.5, 0.5, 0.5, 1.0]
            };
            let name = match selection.ui {
                PlayerSelectUi::CpuAi        (_) => format!("CPU AI"),
                PlayerSelectUi::CpuFighter   (_) => format!("CPU Fighter"),
                PlayerSelectUi::HumanFighter (_) => format!("Port #{}", controller_i+1),
                PlayerSelectUi::HumanTeam    (_) => format!("Port #{} Team", controller_i+1),
                PlayerSelectUi::CpuTeam      (_) => format!("CPU Team"),
                PlayerSelectUi::HumanUnplugged   => unreachable!()
            };
            self.draw_text.queue_text(x, y, size, color, name.as_ref());
        }

        // render UI
        let mut options = vec!();
        match selection.ui {
            PlayerSelectUi::HumanFighter (_) => {
                options.extend(fighters.iter().map(|x| x.name.clone()));
                options.push(String::from("Change Team"));
                options.push(String::from("Add CPU"));
            }
            PlayerSelectUi::CpuFighter (_) => {
                options.extend(fighters.iter().map(|x| x.name.clone()));
                options.push(String::from("Change Team"));
                options.push(String::from("Change AI"));
                options.push(String::from("Remove CPU"));
            }
            PlayerSelectUi::HumanTeam (_) => {
                options.extend(graphics::get_colors().iter().map(|x| x.name.clone()));
                options.push(String::from("Return"));
            }
            PlayerSelectUi::CpuTeam (_) => {
                options.extend(graphics::get_colors().iter().map(|x| x.name.clone()));
                options.push(String::from("Return"));
            }
            PlayerSelectUi::CpuAi (_) => {
                options.push(String::from("Return"));
            }
            PlayerSelectUi::HumanUnplugged => unreachable!()
        }

        for (option_i, option) in options.iter().enumerate() {
            let x_offset = if option_i == selection.ui.ticker_unwrap().cursor { 0.1 } else { 0.0 };
            let x = ((start_x+1.0 + x_offset) / 2.0) * self.width  as f32;
            let y = ((start_y+1.0           ) / 2.0) * self.height as f32 + (option_i+1) as f32 * 40.0;

            let size = 26.0; // TODO: determine from width/height of screen and start/end pos
            let mut color = [1.0, 1.0, 1.0, 1.0];
            match selection.ui {
                PlayerSelectUi::HumanFighter (_) |
                PlayerSelectUi::CpuFighter (_) => {
                    if let Some(selected_option_i) = selection.fighter {
                        if selected_option_i == option_i {
                            color = graphics::get_team_color4(selection.team);
                        }
                    }
                }
                PlayerSelectUi::HumanTeam (_) |
                PlayerSelectUi::CpuTeam (_) => {
                    if option_i < graphics::get_colors().len() {
                        color = graphics::get_team_color4(option_i);
                    }
                }
                _ => { }
            }
            self.draw_text.queue_text(x, y, size, color, option.as_ref());
        }

        // render fighter
        for (fighter_i, (fighter_key, _)) in fighters.key_value_iter().enumerate() {
            if let Some(selection_i) = selection.fighter {
                if fighter_i == selection_i {
                    let fighter = fighters.key_to_value(fighter_key).unwrap();

                    // Determine action, handling the user setting it to an invalid value
                    let css_action = fighter.css_action as usize;
                    let action = if css_action < fighter.actions.len() {
                        css_action
                    } else {
                        Action::Idle.index() as usize
                    };

                    // draw fighter
                    let player = RenderPlayer {
                        team:              selection.team,
                        debug:             DebugPlayer::default(),
                        damage:            0.0,
                        stocks:            None,
                        bps:               (0.0, 0.0),
                        ecb:               ECB::default(),
                        frame:             selection.animation_frame,
                        frame_data:        ActionFrame::default(),
                        fighter:           fighter_key.clone(),
                        face_right:        start_x < 0.0,
                        angle:             0.0,
                        fighter_color:     graphics::get_team_color3(selection.team),
                        fighter_selected:  false,
                        player_selected:   false,
                        selected_colboxes: HashSet::new(),
                        shield:            None,
                        vector_arrows:     vec!(),
                        particles:         vec!(),
                        action,
                    };

                    // fudge player data TODO: One day I would like to have the menu selection fighters (mostly) playable
                    let zoom = fighter.css_scale / 40.0;
                    let fighter_x = start_x + (end_x - start_x) / 2.0;
                    let fighter_y = end_y * -1.0 + 0.05;

                    let camera   = Matrix4::from_nonuniform_scale(zoom, zoom * self.aspect_ratio(), 1.0);
                    let position = Matrix4::from_translation(Vector3::new(fighter_x, fighter_y, 0.0));
                    let dir      = Matrix4::from_nonuniform_scale(if player.face_right { 1.0 } else { -1.0 }, 1.0, 1.0);
                    let transformation = position * (camera * dir);
                    let uniform = vs::ty::Data {
                        edge_color:     graphics::get_team_color4(selection.team),
                        color:          [0.9, 0.9, 0.9, 1.0],
                        transformation: transformation.into(),
                    };
                    let set = self.new_uniform_set(uniform);

                    let entity = MenuEntity::Fighter {
                        fighter: player.fighter,
                        action:  player.action,
                        frame:   player.frame
                    };

                    entities.push(MenuEntityAndSet { set, entity });
                }
            }
        }
    }

    fn draw_stage_selector(&mut self, entities: &mut Vec<MenuEntityAndSet>, selection: usize) {
        self.draw_text.queue_text(100.0, 50.0, 50.0, [1.0, 1.0, 1.0, 1.0], "Select Stage");
        let stages = &self.package_buffers.package.as_ref().unwrap().stages;
        for (stage_i, stage) in stages.key_value_iter().enumerate() {
            let (stage_key, stage) = stage;
            let size = 26.0; // TODO: determine from width/height of screen and start/end pos
            let x_offset = if stage_i == selection { 0.05 } else { 0.0 };
            let x = self.width as f32 * (0.1 + x_offset);
            let y = self.height as f32 * 0.1 + stage_i as f32 * 50.0;
            self.draw_text.queue_text(x, y, size, [1.0, 1.0, 1.0, 1.0], stage.name.as_ref());

            if stage_i == selection {
                let zoom_divider = 100.0;
                let zoom = 1.0 / zoom_divider;
                let y = -0.2 * zoom_divider;

                let camera   = Matrix4::from_nonuniform_scale(zoom, zoom * self.aspect_ratio(), 1.0);
                let position = Matrix4::from_translation(Vector3::new(1.0, y, 0.0));
                let transformation = camera * position;
                let uniform = surface_vs::ty::Data { transformation: transformation.into() };

                let set = self.new_surface_uniform_set(uniform);
                let entity = MenuEntity::Stage(stage_key.clone());
                entities.push(MenuEntityAndSet { set, entity });

                let set = self.new_surface_uniform_set(uniform);
                let entity = MenuEntity::StageFill(stage_key.clone());
                entities.push(MenuEntityAndSet { set, entity });
            }
        }
    }

    fn draw_package_selector(&mut self, package_names: &[String], selection: usize, message: &str, command_output: &[String]) {
        let color: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
        self.draw_text.queue_text(100.0, 50.0, 50.0, color, "Select Package");
        self.draw_text.queue_text(100.0, self.height as f32 - 30.0, 30.0, color, message);
        self.draw_text.queue_text(self.width as f32 - 380.0, self.height as f32 - 30.0, 30.0, color, json_upgrade::build_version().as_str());

        for (package_i, name) in package_names.iter().enumerate() {
            let size = 26.0; // TODO: determine from width/height of screen and start/end pos
            let x_offset = if package_i == selection { 0.1 } else { 0.0 };
            let x = self.width as f32 * (0.1 + x_offset);
            let y = self.height as f32 * 0.1 + package_i as f32 * 50.0;
            self.draw_text.queue_text(x, y, size, [1.0, 1.0, 1.0, 1.0], name.as_ref());
        }
        self.command_render(command_output);
    }

    fn draw_game_selector(&mut self, selection: usize) {
        self.draw_text.queue_text(100.0, 50.0, 50.0, [1.0, 1.0, 1.0, 1.0], "Select Game Mode");

        let modes = vec!("Local", "Netplay", "Replays");
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

    /// returns true iff succeeds
    fn handle_events(&mut self) -> bool {
        // We need to force send the resolution and dpi every frame because OsInput may receive the normal events while it isn't listening for them.
        let window = self.surface.window();

        if let Some(resolution) = window.get_inner_size() {
            // force send the current resolution
            if let Err(_) = self.os_input_tx.send(WindowEvent::Resized(resolution)) {
                return false;
            }
        } else {
            // MS Windows removes the window immediately on close before the process ends
            return false;
        }

        // force send the current dpi
        if let Err(_) = self.os_input_tx.send(WindowEvent::HiDpiFactorChanged(window.get_hidpi_factor())) {
            return false;
        }

        let os_input_tx = self.os_input_tx.clone();
        self.events_loop.poll_events(|event| {
            if let Event::WindowEvent { event, .. } = event {
                os_input_tx.send(event).ok();
            };
        });
        true
    }
}

enum MenuEntity {
    Fighter   { fighter: String, action: usize, frame: usize },
    Stage     (String),
    StageFill (String),
    Rect      (Rect),
}

struct MenuEntityAndSet {
    entity: MenuEntity,
    set:    Arc<DescriptorSet + Send + Sync>,
}
