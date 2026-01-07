use std::sync::Arc;

use anyhow::Context;
use ash::vk::{self};
#[cfg(feature = "tracing")]
use tracy_client::frame_mark;
use tracy_client::span;

use crate::{
    caps::RenderCaps,
    render::{
        Frame, pipeline::create_default_pipeline, present::present_frame,
        render_packet::RenderData, renderer::record_commands, submit::submit_frame,
        swapchain::SwapchainContext,
    },
    vulkan::SwapchainCreateCaps,
};

use super::FrameRing;

pub struct RenderContext {
    device: Arc<ash::Device>,
    graphics_queue: vk::Queue,
    swapchain_context: SwapchainContext,
    frame_ring: FrameRing,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    frag_module: vk::ShaderModule,
    vert_module: vk::ShaderModule,
}

impl RenderContext {
    pub fn new(
        caps: &RenderCaps,
        swapchain_create_caps: SwapchainCreateCaps,
    ) -> anyhow::Result<Self> {
        let queue_index = swapchain_create_caps.queue_families.graphics_index;
        let swapchain_context = SwapchainContext::new(swapchain_create_caps)
            .context("failed to create Swapchain Context")?;
        let mut frames: Vec<Frame> = Vec::new();
        frames.push(Frame::new(&caps.device, queue_index).context("failed to create frame")?);
        frames.push(Frame::new(&caps.device, queue_index).context("failed to create frame")?);
        frames.push(Frame::new(&caps.device, queue_index).context("failed to create frame")?);
        let frame_ring = FrameRing::new(frames);

        let (pipeline_layout, pipeline, frag_module, vert_module) =
            create_default_pipeline(&caps.device, swapchain_context.swapchain_format)?;

        Ok(Self {
            device: caps.device.clone(),
            graphics_queue: caps.queue,
            swapchain_context,
            frame_ring,
            pipeline_layout,
            pipeline,
            frag_module,
            vert_module,
        })
    }

    pub fn render_frame(&mut self, caps: &RenderCaps) -> anyhow::Result<()> {
        let _frame_span = span!("RenderFrame");
        /*
         - swapchain has array of image semaphores
         - frame has single in_flight_fence
         - frame has single image available semaphore

           acquire frame
            wait for the frame's in_flight_fence
            reset the frame's in_flight_fence
            acquire the next image
            set the swapchain image index in the frame

           execute the framegraph

           submit frame
               signal_semaphore is swapchain.getImageSemaphore(frame.image_index)
               wait_semaphore = frame.imageAvailableSemaphore
               queue submit with frame.in_flight_fence

            present frame
                wait_semaphore = swapchain.getImageSemaphore(frame.image_index)
        */
        let frame = self
            .frame_ring
            .acquire(&caps.device)
            .context("failed to acquire frame")?;

        let (image_index, _needs_recreate) = self
            .swapchain_context
            .acquire_next_image(frame.image_available)
            .context("failed to acquire next image")?;

        frame.swapchain_image_index = image_index;

        let render_data = gather_mock_render_data();
        record_commands(
            &caps.device,
            frame,
            &render_data,
            self.swapchain_context.images[image_index as usize],
            self.pipeline,
            self.swapchain_context.swapchain_extent,
            self.swapchain_context.image_views[image_index as usize],
        )?;

        submit_frame(&caps.device, caps.queue, frame, &self.swapchain_context)
            .context("failed to submit frame")?;

        present_frame(caps.present_queue, frame, &self.swapchain_context)
            .context("failed to present frame")?;

        #[cfg(feature = "tracing")]
        frame_mark();
        Ok(())
    }
}

impl Drop for RenderContext {
    fn drop(&mut self) {
        log::trace!("Destroying RenderContext");
        unsafe {
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_shader_module(self.frag_module, None);
            self.device.destroy_shader_module(self.vert_module, None);
        }
        self.frame_ring.destroy(&self.device);
        self.swapchain_context.destroy();
    }
}

fn gather_mock_render_data() -> RenderData {
    RenderData { id: 32 }
}
