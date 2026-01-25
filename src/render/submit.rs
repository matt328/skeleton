use ash::vk;

use crate::render::{
    framegraph::{COLOR_RANGE, ImageState, transition_image},
    swapchain::SwapchainContext,
};

use super::frame::Frame;

pub fn submit_frame(
    device: &ash::Device,
    graphics_queue: vk::Queue,
    frame: &Frame,
    swapchain: &SwapchainContext,
    barrier_cb: vk::CommandBuffer,
) -> anyhow::Result<()> {
    let _frame_span = tracy_client::span!("submit_frame");

    unsafe {
        device.begin_command_buffer(
            barrier_cb,
            &vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
        )?;

        let swapchain_image = swapchain.images[frame.swapchain_image_index as usize];

        transition_image(
            device,
            barrier_cb,
            swapchain_image,
            COLOR_RANGE,
            ImageState::COLOR_ATTACHMENT_WRITE,
            ImageState::PRESENT,
            format!("swapchain #{:?}", frame.swapchain_image_index).as_ref(),
        );

        device.end_command_buffer(barrier_cb)?;
    }

    let signal = [swapchain.image_semaphores[frame.swapchain_image_index as usize]];
    let wait = [frame.image_available];
    let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

    let command_buffers = [frame.primary_cmd, barrier_cb];

    let submit_info = vk::SubmitInfo::default()
        .wait_semaphores(&wait)
        .signal_semaphores(&signal)
        .command_buffers(&command_buffers)
        .wait_dst_stage_mask(wait_stages);

    unsafe {
        device.reset_fences(&[frame.fence])?;
        device.queue_submit(graphics_queue, &[submit_info], frame.fence)?;
    }

    Ok(())
}
