use vulkano_text::{DrawText, DrawTextTrait, UpdateTextCache};
use vulkano_win;
use vulkano_win::VkSurfaceBuild;
use vulkano;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{PrimaryCommandBuffer, PrimaryCommandBufferBuilder, Submission, DynamicState};
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
use vulkano::swapchain::{Swapchain, SurfaceTransform, AcquireError, PresentError};
use winit::{Event, WindowBuilder, PollEventsIterator};

use std::sync::Arc;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread;
use std::time::Duration;
use std::collections::HashSet;

use buffers::{Vertex, Buffers};
use input::Input;

pub mod vs { include!{concat!(env!("OUT_DIR"), "/shaders/src/shaders/vertex.glsl")} }
pub mod fs { include!{concat!(env!("OUT_DIR"), "/shaders/src/shaders/fragment.glsl")} }

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

mod pipeline_layout {
    pipeline_layout! {
        set0: {
            uniforms: UniformBuffer<::graphics::vs::ty::Data>
        }
    }
}

pub struct Uniform {
    uniform:  Arc<CpuAccessibleBuffer<vs::ty::Data>>,
    set:      Arc<pipeline_layout::set0::Set>,
}

#[allow(dead_code)]
pub struct Graphics<'a> {
    package_buffers:  Buffers,
    window:           vulkano_win::Window,
    device:           Arc<Device>,
    swapchain:        Arc<Swapchain>,
    queue:            Arc<Queue>,
    submissions:      Vec<Arc<Submission>>,
    pipeline:         Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>, pipeline_layout::CustomPipeline, render_pass::CustomRenderPass>>,
    render_pass:      Arc<render_pass::CustomRenderPass>,
    framebuffers:     Vec<Arc<Framebuffer<render_pass::CustomRenderPass>>>,
    uniforms:         Vec<Uniform>,
    draw_text:        DrawText<'a>,
    width:            u32,
    height:           u32,
}

impl<'a> Graphics<'a> {
    pub fn new() -> Graphics<'a> {
        let instance = {
            let extensions = vulkano_win::required_extensions();
            Instance::new(None, &extensions, None).expect("failed to create Vulkan instance")
        };

        let physical = PhysicalDevice::enumerate(&instance).next().expect("no device available");
        let window  = WindowBuilder::new().build_vk_surface(&instance).unwrap();
        window.window().set_title("PF TAS");

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

        let framebuffers = Graphics::gen_framebuffers(&images, &render_pass);
        let draw_text = DrawText::new(&device, &queue, &images);
        let (uniforms, pipeline) = Graphics::pipeline(&device, &queue, &images, &render_pass);

        Graphics {
            package_buffers:  Buffers::new(),
            window:           window,
            device:           device,
            swapchain:        swapchain,
            queue:            queue,
            submissions:      vec!(),
            pipeline: pipeline,
            render_pass:      render_pass,
            framebuffers:     framebuffers,
            uniforms:         uniforms,
            draw_text:        draw_text,
            width:            0,
            height:           0,
        }
    }

    fn pipeline(
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        images: &Vec<Arc<SwapchainImage>>,
        render_pass: &Arc<render_pass::CustomRenderPass>
    ) -> (
        Vec<Uniform>,
        Arc<GraphicsPipeline<SingleBufferDefinition<Vertex>, pipeline_layout::CustomPipeline, render_pass::CustomRenderPass>>
    ) {
        let pipeline_layout = pipeline_layout::CustomPipeline::new(&device).unwrap();

        let vs = vs::Shader::load(&device).unwrap();
        let fs = fs::Shader::load(&device).unwrap();

        let mut uniforms: Vec<Uniform> = vec!();
        for _ in 0..1000 {
            let uniform = CpuAccessibleBuffer::<vs::ty::Data>::from_data(
                &device,
                &BufferUsage::all(),
                Some(queue.family()),
                vs::ty::Data {
                    position_offset: [0.0, 0.0],
                    zoom:            1.0,
                    aspect_ratio:    1.0,
                    color:           [1.0, 1.0, 1.0, 1.0],
                }
            ).unwrap();

            let descriptor_pool = DescriptorPool::new(&device);
            let set = pipeline_layout::set0::Set::new(&descriptor_pool, &pipeline_layout, &pipeline_layout::set0::Descriptors {
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

    fn gen_framebuffers(images: &Vec<Arc<SwapchainImage>>, render_pass: &Arc<render_pass::CustomRenderPass>) -> Vec<Arc<Framebuffer<render_pass::CustomRenderPass>>> {
        images.iter().map(|image| {
            let dimensions = [image.dimensions()[0], image.dimensions()[1], 1];
            Framebuffer::new(&render_pass, dimensions, render_pass::AList {
                color: image
            }).unwrap()
        }).collect::<Vec<_>>()
    }

    pub fn poll_events(&mut self) -> PollEventsIterator {
        self.window.window().poll_events()
    }

    pub fn draw(&mut self) {
    }
}
