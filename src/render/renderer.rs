use anyhow::Context;
use ash::vk::{self, CommandBufferBeginInfo};

use super::{frame::Frame, render_packet::RenderData};

pub fn record_commands(
    device: &ash::Device,
    frame: &Frame,
    _render_data: &RenderData,
    swapchain_image: vk::Image,
    pipeline: vk::Pipeline,
    swapchain_extent: vk::Extent2D,
    swapchain_image_view: vk::ImageView,
) -> anyhow::Result<()> {
    let _frame_span = tracy_client::span!("render_commands");
    if let Some(&cmd) = frame.command_buffers.first() {
        let clear_color = vk::ClearColorValue {
            float32: [0.392, 0.584, 0.929, 1.0],
        };
        let clear_value = vk::ClearValue { color: clear_color };
        let viewport = vk::Viewport {
            height: swapchain_extent.height as f32,
            width: swapchain_extent.width as f32,
            ..Default::default()
        };
        let render_rect = vk::Rect2D {
            offset: vk::Offset2D::default(),
            extent: swapchain_extent,
        };
        unsafe {
            device
                .begin_command_buffer(cmd, &CommandBufferBeginInfo::default())
                .context("failed to begin command buffer")?;

            transition_image_to_render(device, cmd, swapchain_image);

            device.cmd_begin_rendering(
                cmd,
                &vk::RenderingInfo::default()
                    .render_area(render_rect)
                    .layer_count(1)
                    .color_attachments(&[vk::RenderingAttachmentInfo::default()
                        .image_view(swapchain_image_view)
                        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .load_op(vk::AttachmentLoadOp::CLEAR)
                        .store_op(vk::AttachmentStoreOp::STORE)
                        .clear_value(clear_value)]),
            );
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline);
            device.cmd_set_viewport(cmd, 0, &[viewport]);
            device.cmd_set_scissor(cmd, 0, &[render_rect]);
            device.cmd_draw(cmd, 3, 1, 0, 0);
            device.cmd_end_rendering(cmd);

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
