use anyhow::Context;
use ash::{amd::display_native_hdr::Device, vk};

use crate::render::{Frame, swapchain::SwapchainContext};

pub fn present_frame(
    queue: vk::Queue,
    frame: &Frame,
    swapchain_context: &SwapchainContext,
) -> anyhow::Result<()> {
    let image_index = frame.swapchain_image_index;
    let wait_semaphores = &[swapchain_context.image_semaphores[image_index as usize]];
    let index = [image_index as u32];
    let sc = [swapchain_context.swapchain];

    let present_info = vk::PresentInfoKHR::default()
        .image_indices(&index)
        .wait_semaphores(wait_semaphores)
        .swapchains(&sc);

    unsafe {
        swapchain_context
            .swapchain_device
            .queue_present(queue, &present_info)
            .context("failed presenting queue")?;
    }
    Ok(())
}
