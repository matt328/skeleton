use std::sync::Arc;

use anyhow::Context;
use ash::vk::{self, CommandBufferBeginInfo, RenderingInfo};

use crate::{
    caps::RenderCaps,
    render::{
        Frame, present::present_frame, render_packet::RenderData, submit::submit_frame,
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
        Ok(Self {
            device: caps.device.clone(),
            graphics_queue: caps.queue,
            swapchain_context,
            frame_ring,
        })
    }
    // The frames in flight here isn't quite right.
    // look at the cpp code and do what it does
    pub fn render_frame(&mut self, caps: &RenderCaps) -> anyhow::Result<()> {
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
        )?;

        submit_frame(&caps.device, caps.queue, frame, &self.swapchain_context)
            .context("failed to submit frame")?;

        present_frame(caps.queue, frame, &self.swapchain_context)
            .context("failed to present frame")?;
        Ok(())
    }
}

impl Drop for RenderContext {
    fn drop(&mut self) {
        log::trace!("Destroying RenderContext");
        self.frame_ring.destroy(&self.device);
        self.swapchain_context.destroy();
    }
}

fn gather_mock_render_data() -> RenderData {
    RenderData { id: 32 }
}

fn record_commands(
    device: &ash::Device,
    frame: &Frame,
    _render_data: &RenderData,
    swapchain_image: vk::Image,
) -> anyhow::Result<()> {
    if let Some(&cmd) = frame.command_buffers.first() {
        let _rendering_info = RenderingInfo::default();
        let begin_info = CommandBufferBeginInfo::default();
        unsafe {
            device
                .begin_command_buffer(cmd, &begin_info)
                .context("failed to begin command buffer")?;
            transition_image_to_render(device, cmd, swapchain_image);
            transition_image_to_present(device, cmd, swapchain_image);
            device
                .end_command_buffer(cmd)
                .context("failed to end command buffer")?;
        }
    }

    Ok(())
}

fn transition_image_to_render(device: &ash::Device, cmd: vk::CommandBuffer, image: vk::Image) {
    let barrier = vk::ImageMemoryBarrier::default()
        .old_layout(vk::ImageLayout::UNDEFINED) // or PRESENT_SRC_KHR if previously presented
        .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .image(image)
        .subresource_range(
            vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1),
        )
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

    unsafe {
        device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );
    }
}

fn transition_image_to_present(device: &ash::Device, cmd: vk::CommandBuffer, image: vk::Image) {
    let barrier = vk::ImageMemoryBarrier::default()
        .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
        .image(image)
        .subresource_range(
            vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1),
        )
        .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .dst_access_mask(vk::AccessFlags::empty());

    unsafe {
        device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );
    }
}
