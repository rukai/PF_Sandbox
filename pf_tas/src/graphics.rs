use winit::{Window, WindowBuilder, EventsLoop};

use vulkano_text::{DrawText, DrawTextTrait};

use vulkano_win;
use vulkano_win::VkSurfaceBuild;

use vulkano;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::device::{Device, Queue};
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract};
use vulkano::image::SwapchainImage;
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::swapchain::{Surface, Swapchain, SurfaceTransform, AcquireError, PresentMode};
use vulkano::sync::{GpuFuture, FlushError};

use std::sync::Arc;
use std::mem;

use state::State;

pub struct Graphics {
    surface:         Arc<Surface<Window>>,
    device:          Arc<Device>,
    future:          Box<GpuFuture>,
    swapchain:       Arc<Swapchain<Window>>,
    queue:           Arc<Queue>,
    framebuffers:    Vec<Arc<FramebufferAbstract + Send + Sync>>,
    draw_text:       DrawText,
    width:           u32,
    height:          u32,
}

impl Graphics {
    pub fn new(events_loop: &EventsLoop) -> Graphics {
        let instance = {
            let extensions = vulkano_win::required_extensions();
            Instance::new(None, &extensions, None).expect("failed to create Vulkan instance")
        };

        let physical = PhysicalDevice::enumerate(&instance).next().expect("no device available");
        let surface = WindowBuilder::new().build_vk_surface(events_loop, instance.clone()).unwrap();
        surface.window().set_title("PF TAS");

        let queue = physical.queue_families().find(|&q| {
            q.supports_graphics() && surface.is_supported(q).unwrap_or(false)
        }).unwrap();

        let (device, mut queues) = {
            let device_ext = vulkano::device::DeviceExtensions {
                khr_swapchain: true,
                .. vulkano::device::DeviceExtensions::none()
            };
            Device::new(physical, physical.supported_features(), &device_ext, [(queue, 0.5)].iter().cloned()).unwrap()
        };

        let future = Box::new(vulkano::sync::now(device.clone())) as Box<GpuFuture>;

        let queue = queues.next().unwrap();

        let (swapchain, images) = {
            let caps = surface.capabilities(physical).unwrap();
            let dimensions = caps.current_extent.unwrap_or([640, 480]);
            let alpha = caps.supported_composite_alpha.iter().next().unwrap();
            let format = caps.supported_formats[0].0;
            Swapchain::new(device.clone(), surface.clone(), caps.min_image_count, format, dimensions, 1,
                caps.supported_usage_flags, &queue, SurfaceTransform::Identity, alpha, PresentMode::Fifo, true, None
            ).unwrap()
        };

        let framebuffers = Graphics::framebuffers(device.clone(), swapchain.clone(), &images);

        let draw_text = DrawText::new(device.clone(), queue.clone(), swapchain.clone(), &images);

        Graphics {
            width:  0,
            height: 0,
            surface,
            device,
            future,
            swapchain,
            queue,
            framebuffers,
            draw_text,
        }
    }

    fn framebuffers(
        device: Arc<Device>,
        swapchain: Arc<Swapchain<Window>>,
        images: &[Arc<SwapchainImage<Window>>]
    ) -> Vec<Arc<FramebufferAbstract + Send + Sync>> {
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
        ).unwrap());

        images.iter().map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                .add(image.clone()).unwrap()
                .build().unwrap()
            ) as Arc<FramebufferAbstract + Send + Sync>
        }).collect::<Vec<_>>()
    }

    fn window_resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;

        let (new_swapchain, new_images) = self.swapchain.recreate_with_dimension([width, height]).unwrap();
        self.swapchain = new_swapchain;
        self.framebuffers = Graphics::framebuffers(self.device.clone(), self.swapchain.clone(), &new_images);

        self.draw_text = DrawText::new(self.device.clone(), self.queue.clone(), self.swapchain.clone(), &new_images);
    }

    pub fn draw(&mut self, state: &State) {
        self.future.cleanup_finished();
        let (new_width, new_height) = self.surface.window().get_inner_size().unwrap().to_physical(self.surface.window().get_hidpi_factor()).into();
        if self.width != new_width || self.height != new_height {
            self.window_resize(new_width, new_height);
        }

        let (image_num, new_future) = match vulkano::swapchain::acquire_next_image(self.swapchain.clone(), None) {
            Ok(img) => { img }
            Err(AcquireError::OutOfDate) => {
                // Just abort this render, the user wont care about losing some frames while resizing. Internal rendering size will be fixed by next frame.
                return;
            }
            Err(err) => { panic!("{:?}", err) }
        };

        let controller = &state.controllers[state.current_controller];
        self.draw_text.queue_text(100.0, 20.0,  20.0, [1.0, 1.0, 1.0, 1.0], &format!("Controller port: {}/{}", state.current_controller + 1, state.controllers.len()));
        self.draw_text.queue_text(100.0, 40.0,  20.0, [1.0, 1.0, 1.0, 1.0], &format!("A: {:?}", controller.a));
        self.draw_text.queue_text(100.0, 60.0,  20.0, [1.0, 1.0, 1.0, 1.0], &format!("B: {:?}", controller.b));
        self.draw_text.queue_text(100.0, 80.0,  20.0, [1.0, 1.0, 1.0, 1.0], &format!("X: {:?}", controller.x));
        self.draw_text.queue_text(100.0, 100.0, 20.0, [1.0, 1.0, 1.0, 1.0], &format!("Y: {:?}", controller.y));
        self.draw_text.queue_text(100.0, 120.0, 20.0, [1.0, 1.0, 1.0, 1.0], &format!("Z: {:?}", controller.z));
        self.draw_text.queue_text(100.0, 140.0, 20.0, [1.0, 1.0, 1.0, 1.0], &format!("L: {:?}", controller.l));
        self.draw_text.queue_text(100.0, 160.0, 20.0, [1.0, 1.0, 1.0, 1.0], &format!("R: {:?}", controller.r));
        self.draw_text.queue_text(100.0, 180.0, 20.0, [1.0, 1.0, 1.0, 1.0], &format!("Stick X: {:?}", controller.stick_x));
        self.draw_text.queue_text(100.0, 200.0, 20.0, [1.0, 1.0, 1.0, 1.0], &format!("Stick Y: {:?}", controller.stick_y));
        self.draw_text.queue_text(100.0, 220.0, 20.0, [1.0, 1.0, 1.0, 1.0], &format!("C Stick X: {:?}", controller.c_stick_y));
        self.draw_text.queue_text(100.0, 240.0, 20.0, [1.0, 1.0, 1.0, 1.0], &format!("C Stick Y: {:?}", controller.c_stick_y));
        self.draw_text.queue_text(100.0, 260.0, 20.0, [1.0, 1.0, 1.0, 1.0], &format!("L Trigger: {:?}", controller.l_trigger));
        self.draw_text.queue_text(100.0, 280.0, 20.0, [1.0, 1.0, 1.0, 1.0], &format!("R Trigger: {:?}", controller.r_trigger));
        self.draw_text.queue_text(100.0, 300.0, 20.0, [1.0, 1.0, 1.0, 1.0], &format!("Dpad Left: {:?}", controller.left));
        self.draw_text.queue_text(100.0, 320.0, 20.0, [1.0, 1.0, 1.0, 1.0], &format!("Dpad Right: {:?}", controller.right));
        self.draw_text.queue_text(100.0, 340.0, 20.0, [1.0, 1.0, 1.0, 1.0], &format!("Dpad Down: {:?}", controller.down));
        self.draw_text.queue_text(100.0, 360.0, 20.0, [1.0, 1.0, 1.0, 1.0], &format!("Dpad Up: {:?}", controller.up));

        let command_buffer = AutoCommandBufferBuilder::new(self.device.clone(), self.queue.family()).unwrap()
            .begin_render_pass(self.framebuffers[image_num].clone(), false, vec![[0.0, 0.0, 0.0, 1.0].into()]).unwrap()
            .end_render_pass().unwrap()
            .draw_text(&mut self.draw_text, image_num)
            .build().unwrap();

        let mut old_future = Box::new(vulkano::sync::now(self.device.clone())) as Box<GpuFuture>; // TODO: Can I avoid making this dummy future?
        mem::swap(&mut self.future, &mut old_future);

        let future_result = old_future.join(new_future)
            .then_execute(self.queue.clone(), command_buffer).unwrap()
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
}
