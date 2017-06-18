use vulkano::framebuffer::{RenderPassDesc, RenderPassDescClearValues};
use vulkano::format::{Format, ClearValue};
use vulkano::framebuffer::{LayoutAttachmentDescription, LayoutPassDescription, LayoutPassDependencyDescription, LoadOp, StoreOp};
use vulkano::image::ImageLayout;

pub struct Desc {
    pub color: (Format, u32),
}

unsafe impl RenderPassDesc for Desc {
    fn num_attachments(&self) -> usize {
        1
    }

    fn attachment_desc(&self, id: usize) -> Option<LayoutAttachmentDescription> {
        if id == 0 {
            Some(LayoutAttachmentDescription {
                format:         self.color.0,
                samples:        self.color.1,
                load:           LoadOp::Clear,
                store:          StoreOp::Store,
                stencil_load:   LoadOp::Clear,
                stencil_store:  StoreOp::Store,
                initial_layout: ImageLayout::Undefined,
                final_layout:   ImageLayout::ColorAttachmentOptimal,
            })
        } else {
            None
        }
    }

    fn num_subpasses(&self) -> usize {
        1
    }

    fn subpass_desc(&self, id: usize) -> Option<LayoutPassDescription> {
        if id == 0 {
            Some(LayoutPassDescription {
                color_attachments:    vec!((0, ImageLayout::ColorAttachmentOptimal)),
                depth_stencil:        None,
                input_attachments:    vec!(),
                resolve_attachments:  vec!(),
                preserve_attachments: vec!(),
            })
        } else {
            None
        }
    }

    fn num_dependencies(&self) -> usize {
        0
    }

    #[allow(unused_variables)]
    fn dependency_desc(&self, id: usize) -> Option<LayoutPassDependencyDescription> {
        None
    }
}

unsafe impl RenderPassDescClearValues<Vec<ClearValue>> for Desc {
    fn convert_clear_values(&self, values: Vec<ClearValue>) -> Box<Iterator<Item = ClearValue>> {
        Box::new(values.into_iter())
    }
}

// equivalent macro call:
//let render_pass = Arc::new(single_pass_renderpass!(
//    device.clone(),
//    attachments: {
//        color: {
//            load: Clear,
//            store: Store,
//            format: swapchain.format(),
//            samples: 1,
//        }
//    },
//    pass: {
//        color: [color],
//        depth_stencil: {}
//    }
//).unwrap());
