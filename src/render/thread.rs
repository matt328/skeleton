use std::sync::Arc;

use anyhow::Context;
use ash::{khr::swapchain, vk};
#[cfg(feature = "tracing")]
use tracy_client::frame_mark;
use vk_mem::AllocatorCreateInfo;

use crate::{
    caps::RenderCaps,
    image::ImageManager,
    messages::{EngineControl, ShutdownPhase},
    render::{
        Frame, FrameRing,
        framegraph::{ForwardPass, FramegraphBuilder, ImageResolveContext},
        pipeline::PipelineManager,
        present::present_frame,
        submit::submit_frame,
        swapchain::SwapchainContext,
    },
    vulkan::SwapchainCreateCaps,
};

use super::render_packet::RenderData;

pub struct FrameExecutionContext<'a> {
    pub device: &'a ash::Device,
    pub frame: &'a mut Frame,
    pub cmd: vk::CommandBuffer,

    pub image_manager: &'a ImageManager,
    pub pipeline_manager: &'a PipelineManager,
    pub swapchain_extent: vk::Extent2D,
    pub viewport: vk::Viewport,
    pub snizzor: vk::Rect2D,
    pub render_data: &'a RenderData,
}

struct FrameExecutionResources<'a> {
    pub frame_ring: &'a mut FrameRing,
    pub swapchain_context: &'a mut SwapchainContext,
}

pub fn render_thread(
    caps: RenderCaps,
    control: Arc<EngineControl>,
    swapchain_create_caps: SwapchainCreateCaps,
) -> anyhow::Result<()> {
    let queue_index = swapchain_create_caps.queue_families.graphics_index;
    let mut swapchain_context = SwapchainContext::new(swapchain_create_caps)
        .context("failed to create Swapchain Context")?;

    let mut image_manager = ImageManager::default();

    let command_pool = {
        let pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        unsafe {
            caps.device
                .create_command_pool(&pool_info, None)
                .context("failed to create command pool in render_thread")?
        }
    };

    let frames: Vec<Frame> = vec![
        Frame::new(&caps.device, command_pool, 1, 0).context("failed to create frame")?,
        Frame::new(&caps.device, command_pool, 1, 1).context("failed to create frame")?,
        Frame::new(&caps.device, command_pool, 1, 2).context("failed to create frame")?,
    ];

    let mut frame_ring = FrameRing::new(frames);

    let mut pipeline_manager =
        PipelineManager::new(&caps.device).context("thread failed to create pipeline manager")?;

    let resolve_alias = |alias| -> vk::Extent2D { vk::Extent2D::default() };

    let image_ctx = ImageResolveContext {
        device: &caps.device,
        swapchain_extent: swapchain_context.swapchain_extent,
        swapchain_format: swapchain_context.swapchain_format,
        resolve_alias: &resolve_alias,
        default_resize_policy: crate::image::ResizePolicy::Swapchain,
        default_initial_layout: vk::ImageLayout::UNDEFINED,
        frame_count: 3,
    };

    let extent = swapchain_context.swapchain_extent;

    let swapchain_keys = image_manager.register_external_perframe_image(
        &swapchain_context.images,
        &swapchain_context.image_views,
    );

    let aci = AllocatorCreateInfo::new(&caps.instance, &caps.device, *caps.physical_device);

    let allocator = unsafe { vk_mem::Allocator::new(aci).context("failed to create allocator")? };

    let framegraph = FramegraphBuilder::new(
        &mut image_manager,
        &allocator,
        &caps.device,
        &[swapchain_context.swapchain_format],
        vk::Format::D32_SFLOAT, // TODO: policy-ize
        &mut pipeline_manager,
    )
    .add_pass(ForwardPass::default())
    .build(&image_ctx, swapchain_keys)?;

    let exec_resources = FrameExecutionResources {
        frame_ring: &mut frame_ring,
        swapchain_context: &mut swapchain_context,
    };

    while control.phase() != ShutdownPhase::StopRender {
        let frame = exec_resources.frame_ring.acquire(&caps.device)?;

        let (image_index, _) = exec_resources
            .swapchain_context
            .acquire_next_image(frame.image_available)?;

        frame.swapchain_image_index = image_index;

        let render_data = gather_mock_render_data();

        let cmd = frame.primary_cmd;
        let extent = exec_resources.swapchain_context.swapchain_extent;
        let viewport = vk::Viewport {
            height: extent.height as f32,
            width: extent.width as f32,
            ..Default::default()
        };
        let snizzor = vk::Rect2D {
            offset: vk::Offset2D::default(),
            extent,
        };

        let mut fg_ctx = FrameExecutionContext {
            device: &caps.device,
            frame,
            cmd,
            image_manager: &image_manager,
            pipeline_manager: &pipeline_manager,
            swapchain_extent: extent,
            viewport,
            snizzor,
            render_data: &render_data,
        };

        framegraph.execute(&mut fg_ctx)?;

        submit_frame(
            &caps.device,
            caps.queue,
            frame,
            &exec_resources.swapchain_context,
        )
        .context("failed to submit frame")?;

        present_frame(caps.present_queue, frame, &exec_resources.swapchain_context)
            .context("failed to present frame")?;

        #[cfg(feature = "tracing")]
        frame_mark();
    }

    unsafe {
        caps.device
            .device_wait_idle()
            .context("render: failed waiting idle")?;
        caps.device.destroy_command_pool(command_pool, None);
    }
    frame_ring.destroy(&caps.device);

    pipeline_manager
        .destroy(&caps.device)
        .context("failed to destroy pipeline manager")?;

    image_manager.cleanup_per_frames(&caps.device, &allocator)?;
    drop(allocator);
    swapchain_context.destroy();

    log::debug!("Render thread shutting down");

    Ok(())
}

fn gather_mock_render_data() -> RenderData {
    RenderData { id: 5 }
}
