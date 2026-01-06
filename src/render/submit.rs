use ash::vk;

use crate::render::swapchain::SwapchainContext;

use super::frame::Frame;

pub fn submit_frame(
    device: &ash::Device,
    graphics_queue: vk::Queue,
    frame: &Frame,
    swapchain: &SwapchainContext,
) -> anyhow::Result<()> {
    let signal = [swapchain.image_semaphores[frame.swapchain_image_index as usize]];
    let wait = [frame.image_available];
    let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

    let submit_info = vk::SubmitInfo::default()
        .wait_semaphores(&wait)
        .signal_semaphores(&signal)
        .command_buffers(&frame.command_buffers)
        .wait_dst_stage_mask(wait_stages);

    unsafe {
        device.reset_fences(&[frame.fence])?;
        device.queue_submit(graphics_queue, &[submit_info], frame.fence)?;
    }

    Ok(())
}
