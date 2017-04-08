use ::vulkan_buffers::{Vertex, Buffers, PackageBuffers};
use ::game::{GameState, RenderEntity, RenderGame};
use ::menu::{RenderMenu, RenderMenuState, CharacterSelect};
use ::graphics::{self, GraphicsMessage, Render};
use ::player::{RenderFighter, RenderPlayer, DebugPlayer};
use ::fighter::{Action, ECB};
use ::records::GameResult;
use ::package::Verify;

use vulkano_text::{DrawText, DrawTextTrait, UpdateTextCache};
use vulkano_win;
use vulkano_win::VkSurfaceBuild;
use vulkano;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{PrimaryCommandBufferBuilder, Submission, DynamicState};
use vulkano::command_buffer;
use vulkano::descriptor::descriptor_set::DescriptorPool;
use vulkano::device::{Device, Queue};
use vulkano::framebuffer::{Framebuffer, Subpass};
use vulkano::image::SwapchainImage;
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::pipeline::blend::Blend;
use vulkano::pipeline::depth_stencil::DepthStencil;
use vulkano::pipeline::input_assembly::InputAssembly;
use vulkano::pipeline::multisample::Multisample;
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::viewport::{ViewportsState, Viewport, Scissor};
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineParams};
use vulkano::swapchain::{Swapchain, SurfaceTransform};
use winit::{Event, WindowBuilder};

use std::sync::Arc;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread;
use std::time::Duration;
use std::collections::HashSet;

mod generic_vs { include!{concat!(env!("OUT_DIR"), "/shaders/src/shaders/generic-vertex.glsl")} }
mod generic_fs { include!{concat!(env!("OUT_DIR"), "/shaders/src/shaders/generic-fragment.glsl")} }

mod render_pass {
    use vulkano::format::Format;
    single_pass_renderpass!{
        attachments: {
            color: {
                load:   Clear,
                store:  Store,
                format: Format,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {}
        }
    }
}

mod generic_pipeline_layout {
    pipeline_layout! {
        set0: {
            uniforms: UniformBuffer<::vulkan::generic_vs::ty::Data>
        }
    }
}

pub struct Uniform {
    uniform:  Arc<CpuAccessibleBuffer<generic_vs::ty::Data>>,
    set:      Arc<generic_pipeline_layout::set0::Set>,
}

#[allow(dead_code)]
pub struct VulkanGraphics<'a> {
    package_buffers:  PackageBuffers,
    window:           vulkano_win::Window,
    device:           Arc<Device>,
    swapchain:        Arc<Swapchain>,
    queue:            Arc<Queue>,
    submissions:      Vec<Arc<Submission>>,
    generic_pipeline: Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>, generic_pipeline_layout::CustomPipeline, render_pass::CustomRenderPass>>,
    render_pass:      Arc<render_pass::CustomRenderPass>,
    framebuffers:     Vec<Arc<Framebuffer<render_pass::CustomRenderPass>>>,
    uniforms:         Vec<Uniform>,
    draw_text:        DrawText<'a>,
    os_input_tx:      Sender<Event>,
    render_rx:        Receiver<GraphicsMessage>,
    width:            u32,
    height:           u32,
}

impl<'a> VulkanGraphics<'a> {
    pub fn init(os_input_tx: Sender<Event>) -> Sender<GraphicsMessage> {
        let (render_tx, render_rx) = channel();

        thread::spawn(move || {
            let mut graphics = VulkanGraphics::new(os_input_tx, render_rx);
            graphics.run();
        });
        render_tx
    }

    fn new(os_input_tx: Sender<Event>, render_rx: Receiver<GraphicsMessage>) -> VulkanGraphics<'a> {
        let instance = {
            let extensions = vulkano_win::required_extensions();
            Instance::new(None, &extensions, None).expect("failed to create Vulkan instance")
        };

        let physical = PhysicalDevice::enumerate(&instance).next().expect("no device available");
        let window  = WindowBuilder::new().build_vk_surface(&instance).unwrap();
        window.window().set_title("PF Sandbox");

        let queue = physical.queue_families().find(|q| {
            q.supports_graphics() && window.surface().is_supported(q).unwrap_or(false)
        }).unwrap();

        let (device, mut queues) = {
            let device_ext = vulkano::device::DeviceExtensions {
                khr_swapchain: true,
                .. vulkano::device::DeviceExtensions::none()
            };
            Device::new(&physical, physical.supported_features(), &device_ext, [(queue, 0.5)].iter().cloned()).unwrap()
        };

        let queue = queues.next().unwrap();

        let (swapchain, images) = {
            let caps = window.surface().get_capabilities(&physical).unwrap();
            let dimensions = caps.current_extent.unwrap_or([640, 480]);
            let present = caps.present_modes.iter().next().unwrap();
            let alpha = caps.supported_composite_alpha.iter().next().unwrap();
            let format = caps.supported_formats[0].0;
            Swapchain::new(&device, &window.surface(), caps.min_image_count, format, dimensions, 1,
                &caps.supported_usage_flags, &queue, SurfaceTransform::Identity, alpha, present, true, None
            ).unwrap()
        };

        let render_pass = render_pass::CustomRenderPass::new(
            &device, &render_pass::Formats { color: (images[0].format(), 1) }
        ).unwrap();

        let framebuffers = images.iter().map(|image| {
            let dimensions = [image.dimensions()[0], image.dimensions()[1], 1];
            Framebuffer::new(&render_pass, dimensions, render_pass::AList {
                color: image
            }).unwrap()
        }).collect::<Vec<_>>();

        let draw_text = DrawText::new(&device, &queue, &images);

        let (uniforms, generic_pipeline) = VulkanGraphics::generic_pipeline(&device, &queue, &images, &render_pass);

        VulkanGraphics {
            package_buffers:  PackageBuffers::new(),
            window:           window,
            device:           device,
            swapchain:        swapchain,
            queue:            queue,
            submissions:      vec!(),
            generic_pipeline: generic_pipeline,
            render_pass:      render_pass,
            framebuffers:     framebuffers,
            uniforms:         uniforms,
            draw_text:        draw_text,
            os_input_tx:      os_input_tx,
            render_rx:        render_rx,
            width:            0,
            height:           0,
        }
    }

    fn generic_pipeline(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        images: &Vec<Arc<SwapchainImage>>,
        render_pass: &Arc<render_pass::CustomRenderPass>
    ) -> (
        Vec<Uniform>,
        Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>, generic_pipeline_layout::CustomPipeline, render_pass::CustomRenderPass>>
    ) {
        let pipeline_layout = generic_pipeline_layout::CustomPipeline::new(&device).unwrap();

        let vs = generic_vs::Shader::load(&device).unwrap();
        let fs = generic_fs::Shader::load(&device).unwrap();

        let mut uniforms: Vec<Uniform> = vec!();
        for _ in 0..1000 {
            let uniform = CpuAccessibleBuffer::<generic_vs::ty::Data>::from_data(
                &device,
                &BufferUsage::all(),
                Some(queue.family()),
                generic_vs::ty::Data {
                    position_offset: [0.0, 0.0],
                    zoom:            1.0,
                    aspect_ratio:    1.0,
                    direction:       1.0,
                    edge_color:      [1.0, 1.0, 1.0, 1.0],
                    color:           [1.0, 1.0, 1.0, 1.0],
                    _dummy0:         [0; 12],
                }
            ).unwrap();

            let descriptor_pool = DescriptorPool::new(&device);
            let set = generic_pipeline_layout::set0::Set::new(&descriptor_pool, &pipeline_layout, &generic_pipeline_layout::set0::Descriptors {
                uniforms: &uniform
            });
            uniforms.push(Uniform {
                uniform: uniform,
                set: set
            });
        }

        let pipeline = GraphicsPipeline::new(&device,
            GraphicsPipelineParams {
                vertex_input:    SingleBufferDefinition::new(),
                vertex_shader:   vs.main_entry_point(),
                input_assembly:  InputAssembly::triangle_list(),
                tessellation:    None,
                geometry_shader: None,
                viewport:        ViewportsState::Fixed {
                    data: vec![(
                        Viewport {
                            origin:      [0.0, 0.0],
                            depth_range: 0.0..1.0,
                            dimensions:  [
                                images[0].dimensions()[0] as f32,
                                images[0].dimensions()[1] as f32
                            ],
                        },
                        Scissor::irrelevant()
                    )],
                },
                raster:          Default::default(),
                multisample:     Multisample::disabled(),
                fragment_shader: fs.main_entry_point(),
                depth_stencil:   DepthStencil::disabled(),
                blend:           Blend::alpha_blending(),
                layout:          &pipeline_layout,
                render_pass:     Subpass::from(&render_pass, 0).unwrap(),
            }
        ).unwrap();

        (uniforms, pipeline)
    }

    fn run(&mut self) {
        loop {
            self.submissions.retain(|s| s.destroying_would_block());
            {
                // get the most recent render
                let mut render = {
                    let message = self.render_rx.recv().unwrap();
                    self.read_message(message)
                };
                while let Ok(message) = self.render_rx.try_recv() {
                    render = self.read_message(message);
                }

                let size = self.window.window().get_inner_size_points().unwrap();
                self.width = size.0;
                self.height = size.1;

                match render {
                    Render::Game(game) => { self.game_render(game); },
                    Render::Menu(menu) => { self.menu_render(menu); },
                }
            }
            self.handle_events();
        }
    }

    fn read_message(&mut self, message: GraphicsMessage) -> Render {
        self.package_buffers.update(&self.device, &self.queue, message.package_updates);
        message.render
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

    fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    fn game_render(&mut self, render: RenderGame) {
        let zoom = render.camera.zoom.recip();
        let pan  = render.camera.pan;
        let aspect_ratio = self.aspect_ratio();

        match render.state {
            GameState::Local  => { },
            GameState::Paused => {
                // TODO: blue vaporwavey background lines to indicate pause :D
                // also double as measuring/scale lines
                // configurable size via treeflection
                // but this might be desirable to have during normal gameplay to, hmmmm....
                // Just have a 5 second fade in/out time so it doesnt look clunky and can be used during frame advance
            },
            _ => { },
        }

        self.game_hud_render(&render.entities);

        let image_num = self.swapchain.acquire_next_image(Duration::new(1, 0)).unwrap();
        let mut command_buffer = PrimaryCommandBufferBuilder::new(&self.device, self.queue.family())
        .update_text_cache(&mut self.draw_text)
        .draw_inline(&self.render_pass, &self.framebuffers[image_num], render_pass::ClearValues {
            color: [0.0, 0.0, 0.0, 1.0]
        });

        let stage = render.stage;
        let mut uniforms = self.uniforms.iter();
        let uniform = uniforms.next().unwrap();
        {
            let mut buffer_content = uniform.uniform.write(Duration::new(1, 0)).unwrap();
            buffer_content.zoom            = zoom;
            buffer_content.aspect_ratio    = aspect_ratio;
            buffer_content.position_offset = [pan.0 as f32, pan.1 as f32];
            buffer_content.direction       = 1.0;
            buffer_content.edge_color      = [1.0, 1.0, 1.0, 1.0];
            buffer_content.color           = [1.0, 1.0, 1.0, 1.0];
        }
        let vertex_buffer = &self.package_buffers.stages[stage].vertex;
        let index_buffer  = &self.package_buffers.stages[stage].index;

        command_buffer = command_buffer.draw_indexed(&self.generic_pipeline, vertex_buffer, index_buffer, &DynamicState::none(), &uniform.set, &());

        for entity in render.entities {
            match entity {
                RenderEntity::Player(player) => {
                    let dir = if player.face_right { 1.0 } else { -1.0 } as f32;
                    let draw_pos = [player.bps.0 + pan.0 as f32, player.bps.1 + pan.1 as f32];
                    // draw player ecb
                    if player.debug.ecb {
                        let buffers = Buffers::new_player(&self.device, &self.queue, &player);
                        let uniform = uniforms.next().unwrap();
                        {
                            let mut buffer_content = uniform.uniform.write(Duration::new(1, 0)).unwrap();
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
                        command_buffer = command_buffer.draw_indexed(&self.generic_pipeline, &buffers.vertex, &buffers.index, &DynamicState::none(), &uniform.set, &());
                    }

                    // setup fighter uniform
                    match player.debug.fighter {
                        RenderFighter::Normal | RenderFighter::Debug => {
                            let uniform = uniforms.next().unwrap();
                            {
                                let mut buffer_content = uniform.uniform.write(Duration::new(1, 0)).unwrap();
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
                            let fighter_frames = &self.package_buffers.fighters[player.fighter][player.action];
                            if player.frame < fighter_frames.len() {
                                if let &Some(ref buffers) = &fighter_frames[player.frame] {
                                    command_buffer = command_buffer.draw_indexed(&self.generic_pipeline, &buffers.vertex, &buffers.index, &DynamicState::none(), &uniform.set, &());
                                }
                            }
                            else {
                                 //TODO: Give some indication that we are rendering a deleted or otherwise nonexistent frame
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
                            let mut buffer_content = uniform.uniform.write(Duration::new(1, 0)).unwrap();
                            buffer_content.zoom            = zoom;
                            buffer_content.aspect_ratio    = aspect_ratio;
                            buffer_content.position_offset = [player.bps.0 + pan.0 as f32, player.bps.1 + pan.1 as f32];
                            buffer_content.direction       = if player.face_right { 1.0 } else { -1.0 } as f32;
                            buffer_content.edge_color      = [0.0, 1.0, 0.0, 1.0];
                            buffer_content.color           = [0.0, 1.0, 0.0, 1.0];
                        }
                        let buffers = self.package_buffers.fighter_frame_colboxes(&self.device, &self.queue, player.fighter, player.action, player.frame, &player.selected_colboxes);
                        command_buffer = command_buffer.draw_indexed(&self.generic_pipeline, &buffers.vertex, &buffers.index, &DynamicState::none(), &uniform.set, &());
                    }

                    // TODO: Edit::Player  - render selected player's BPS as green
                    // TODO: Edit::Fighter - Click and drag on ECB points
                    // TODO: Edit::Stage   - render selected platforms as green
                },
                RenderEntity::Selector(rect) => {
                    let uniform = uniforms.next().unwrap();
                    {
                        let mut buffer_content = uniform.uniform.write(Duration::new(1, 0)).unwrap();
                        buffer_content.zoom            = zoom;
                        buffer_content.aspect_ratio    = aspect_ratio;
                        buffer_content.position_offset = [pan.0 as f32, pan.1 as f32];
                        buffer_content.direction       = 1.0;
                        buffer_content.edge_color      = [0.0, 1.0, 0.0, 1.0];
                        buffer_content.color           = [0.0, 1.0, 0.0, 1.0];
                    }
                    let buffers = Buffers::rect_buffers(&self.device, &self.queue, rect);
                    command_buffer = command_buffer.draw_indexed(&self.generic_pipeline, &buffers.vertex, &buffers.index, &DynamicState::none(), &uniform.set, &());
                },
                RenderEntity::Area(rect) => {
                    let uniform = uniforms.next().unwrap();
                    {
                        let mut buffer_content = uniform.uniform.write(Duration::new(1, 0)).unwrap();
                        buffer_content.zoom            = zoom;
                        buffer_content.aspect_ratio    = aspect_ratio;
                        buffer_content.position_offset = [pan.0 as f32, pan.1 as f32];
                        buffer_content.direction       = 1.0;
                        buffer_content.edge_color      = [0.0, 1.0, 0.0, 1.0];
                        buffer_content.color           = [0.0, 1.0, 0.0, 1.0]; // HMMM maybe i can use only the edge to get the outline from a normal rect?
                    }
                    let buffers = Buffers::rect_buffers(&self.device, &self.queue, rect);
                    command_buffer = command_buffer.draw_indexed(&self.generic_pipeline, &buffers.vertex, &buffers.index, &DynamicState::none(), &uniform.set, &());
                },
            }
        }

        let final_command_buffer = command_buffer
            .draw_text(&mut self.draw_text, &self.device, &self.queue, self.width, self.height)
            .draw_end()
            .build();
        self.submissions.push(command_buffer::submit(&final_command_buffer, &self.queue).unwrap());
        self.swapchain.present(&self.queue, image_num).unwrap();
    }

    fn menu_render(&mut self, render: RenderMenu) {
        let mut entities: Vec<MenuEntity> = vec!();
        match render.state {
            RenderMenuState::CharacterSelect (selections) => {
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
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[0], plugged_in_selections[0], -0.9, -0.9, 0.9, 0.9);
                    }
                    2 => {
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[0], plugged_in_selections[0], -0.9, -0.9, 0.0, 0.9);
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[1], plugged_in_selections[1],  0.0, -0.9, 0.9, 0.9);
                    }
                    3 => {
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[0], plugged_in_selections[0], -0.9, -0.9, 0.0, 0.0);
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[1], plugged_in_selections[1],  0.0, -0.9, 0.9, 0.0);
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[2], plugged_in_selections[2], -0.9,  0.0, 0.0, 0.9);
                    }
                    4 => {
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[0], plugged_in_selections[0], -0.9, -0.9, 0.0, 0.0);
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[1], plugged_in_selections[1],  0.0, -0.9, 0.9, 0.0);
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[2], plugged_in_selections[2], -0.9,  0.0, 0.0, 0.9);
                        self.draw_fighter_selector(&mut entities, plugged_in_controller_indexes[3], plugged_in_selections[3],  0.0,  0.0, 0.9, 0.9);
                    }
                    _ => {
                        self.draw_text.queue_text(100.0, 50.0, 30.0, [1.0, 1.0, 1.0, 1.0], "Currently only supports up to 4 controllers. Please unplug some.");
                    }
                }
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
            RenderMenuState::SwitchPackages => {
                self.draw_text.queue_text(100.0, 50.0, 30.0, [1.0, 1.0, 1.0, 1.0], "switch package");
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
            RenderMenuState::StartGame => {
                self.draw_text.queue_text(100.0, 50.0, 30.0, [1.0, 1.0, 1.0, 1.0], "Start game");
            }
        }

        let image_num = self.swapchain.acquire_next_image(Duration::new(1, 0)).unwrap();
        let mut command_buffer = PrimaryCommandBufferBuilder::new(&self.device, self.queue.family())
        .update_text_cache(&mut self.draw_text)
        .draw_inline(&self.render_pass, &self.framebuffers[image_num], render_pass::ClearValues {
            color: [0.0, 0.0, 0.0, 1.0]
        });

        for (i, entity) in entities.iter().enumerate() {
            let uniform = &self.uniforms[i].set;
            match entity {
                &MenuEntity::Fighter { fighter, action, frame } => {
                    let fighter_frames = &self.package_buffers.fighters[fighter][action];
                    if frame < fighter_frames.len() {
                        if let &Some(ref buffers) = &fighter_frames[frame] {
                            command_buffer = command_buffer.draw_indexed(&self.generic_pipeline, &buffers.vertex, &buffers.index, &DynamicState::none(), uniform, &());
                        }
                    }
                }
                &MenuEntity::Stage (stage) => {
                    let vertex_buffer = &self.package_buffers.stages[stage].vertex;
                    let index_buffer  = &self.package_buffers.stages[stage].index;
                    command_buffer = command_buffer.draw_indexed(&self.generic_pipeline, vertex_buffer, index_buffer, &DynamicState::none(), uniform, &());
                }
            }
        }

        let final_command_buffer = command_buffer
            .draw_text(&mut self.draw_text, &self.device, &self.queue, self.width, self.height)
            .draw_end()
            .build();
        self.submissions.push(command_buffer::submit(&final_command_buffer, &self.queue).unwrap());
        self.swapchain.present(&self.queue, image_num).unwrap();
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
        };

        self.draw_text.queue_text(30.0, self.height as f32 - 30.0, 30.0, color, message.as_str());
    }

    fn draw_player_result(&mut self, result: &GameResult, start_x: f32) {
        let fighter_name = self.package_buffers.package.as_ref().unwrap().fighters[result.fighter].name.as_ref();
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
        let fighters = &self.package_buffers.package.as_ref().unwrap().fighters;
        for (fighter_i, fighter) in fighters.iter().enumerate() {
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
                        fighter:           fighter_i,
                        face_right:        start_x < 0.0,
                        fighter_color:     color,
                        fighter_selected:  false,
                        player_selected:   false,
                        selected_colboxes: HashSet::new(),
                    };

                    // draw fighter
                    let fighter_frames = &self.package_buffers.fighters[player.fighter][player.action];
                    if player.frame < fighter_frames.len() {
                        // TODO: dynamically calculate position and zoom (fit width/height of fighter into selection area)
                        let zoom = 40.0;
                        let fighter_x = start_x + (end_x - start_x) / 2.0;
                        let fighter_y = end_y - 0.2; // HACK: dont know why the fighters are drawing so low, so just put them 0.2 higher
                        let fighter_x_scaled = fighter_x * zoom;
                        let fighter_y_scaled = fighter_y * zoom * -1.0 + player.bps.1;
                        let uniform = &self.uniforms[menu_entities.len()];
                        {
                            let mut buffer_content = uniform.uniform.write(Duration::new(1, 0)).unwrap();
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
        let stages = &self.package_buffers.package.as_ref().unwrap().stages;
        for (stage_i, stage) in stages.iter().enumerate() {
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
                    let mut buffer_content = uniform.uniform.write(Duration::new(1, 0)).unwrap();
                    buffer_content.zoom            = 1.0 / zoom;
                    buffer_content.aspect_ratio    = self.aspect_ratio();
                    buffer_content.position_offset = [0.0, y];
                    buffer_content.direction       = 1.0;
                    buffer_content.edge_color      = [1.0, 1.0, 1.0, 1.0];
                    buffer_content.color           = [1.0, 1.0, 1.0, 1.0];
                }

                menu_entities.push(MenuEntity::Stage(selection));
            }
        }
    }

    fn handle_events(&mut self) {
        // force send the current resolution
        let window = self.window.window();
        let res = window.get_inner_size_points().unwrap();
        self.os_input_tx.send(Event::Resized(res.0, res.1)).unwrap();

        for ev in window.poll_events() {
            self.os_input_tx.send(ev).unwrap();
        }
    }

}

enum MenuEntity {
    Fighter { fighter: usize, action: usize, frame: usize },
    Stage   (usize),
}
