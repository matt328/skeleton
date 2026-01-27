use std::sync::Arc;

use anyhow::Context;
use ash::vk;
#[cfg(feature = "tracing")]
use tracy_client::frame_mark;
use tracy_client::{Client, plot};
use vk_mem::AllocatorCreateInfo;

use crate::{
    caps::RenderCaps,
    image::ImageManager,
    messages::EngineControl,
    render::{
        Frame, FrameRing,
        framegraph::{CompositionPass, ForwardPass, FramegraphBuilder, ImageResolveContext},
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
    _control: Arc<EngineControl>,
    swapchain_create_caps: SwapchainCreateCaps,
) -> anyhow::Result<()> {
    let queue_index = swapchain_create_caps.queue_families.graphics_index;
    let mut swapchain_context = SwapchainContext::new(swapchain_create_caps)
        .context("failed to create Swapchain Context")?;

    let mut image_manager = ImageManager::default();

    let device = &caps.device_context.device;

    let command_pool = {
        let pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        unsafe {
            device
                .create_command_pool(&pool_info, None)
                .context("failed to create command pool in render_thread")?
        }
    };

    let frames: Vec<Frame> = vec![
        Frame::new(&caps.device_context, command_pool, 2, 0).context("failed to create frame")?,
        Frame::new(&caps.device_context, command_pool, 2, 1).context("failed to create frame")?,
    ];

    let mut frame_ring = FrameRing::new(frames);

    let mut pipeline_manager =
        PipelineManager::new(device).context("thread failed to create pipeline manager")?;

    let resolve_alias = |_alias| -> vk::Extent2D { vk::Extent2D::default() };

    let image_ctx = ImageResolveContext {
        device_context: &caps.device_context,
        swapchain_extent: swapchain_context.swapchain_extent,
        swapchain_format: swapchain_context.swapchain_format,
        resolve_alias: &resolve_alias,
        default_resize_policy: crate::image::ResizePolicy::Swapchain,
        default_initial_layout: vk::ImageLayout::UNDEFINED,
        frame_count: 2,
    };

    let _extent = swapchain_context.swapchain_extent;

    let swapchain_keys = image_manager
        .register_external_per_frame(&swapchain_context.images, &swapchain_context.image_views);

    let aci = AllocatorCreateInfo::new(&caps.instance, device, *caps.physical_device);

    let allocator = unsafe { vk_mem::Allocator::new(aci).context("failed to create allocator")? };

    let mut framegraph = FramegraphBuilder::new(
        &mut image_manager,
        &allocator,
        caps.device_context.clone(),
        &[swapchain_context.swapchain_format],
        vk::Format::D32_SFLOAT, // TODO: policy-ize
        &mut pipeline_manager,
    )
    .add_pass(ForwardPass::default())
    .add_pass(CompositionPass::default())
    .build(&image_ctx, swapchain_keys)?;

    let exec_resources = FrameExecutionResources {
        frame_ring: &mut frame_ring,
        swapchain_context: &mut swapchain_context,
    };

    // while control.phase() != ShutdownPhase::StopRender {
    for _ in 0..10 {
        let frame = exec_resources.frame_ring.acquire(device)?;

        let (image_index, _) = exec_resources
            .swapchain_context
            .acquire_next_image(frame.image_available)?;

        plot!("swapchain image index", image_index as f64);
        plot!("frame index", frame.index as f64);

        Client::running().context("no client")?.message(
            format!("swapchain image index: {:?}", image_index).as_ref(),
            0,
        );

        Client::running()
            .context("no client")?
            .message(format!("frame index: {:?}", frame.index).as_ref(), 0);

        frame.swapchain_image_index = image_index;

        let render_data = gather_mock_render_data();

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

        let fg_ctx = FrameExecutionContext {
            device,
            frame,
            image_manager: &image_manager,
            pipeline_manager: &pipeline_manager,
            swapchain_extent: extent,
            viewport,
            snizzor,
            render_data: &render_data,
        };

        framegraph.execute(&fg_ctx)?;

        let cmd = create_single_use_command_buffer(device, command_pool)?;

        submit_frame(
            device,
            caps.queue,
            frame,
            exec_resources.swapchain_context,
            cmd,
        )
        .context("failed to submit frame")?;

        present_frame(caps.present_queue, frame, exec_resources.swapchain_context)
            .context("failed to present frame")?;

        #[cfg(feature = "tracing")]
        frame_mark();
    }

    unsafe {
        device
            .device_wait_idle()
            .context("render: failed waiting idle")?;
        device.destroy_command_pool(command_pool, None);
    }
    frame_ring.destroy(device);

    pipeline_manager
        .destroy(device)
        .context("failed to destroy pipeline manager")?;

    image_manager.cleanup_per_frames(device, &allocator)?;
    drop(allocator);
    swapchain_context.destroy();

    log::debug!("Render thread shutting down");
    anyhow::bail!("forced render-thread failure (ARBOR_FAIL_RENDER)");
}

fn gather_mock_render_data() -> RenderData {
    RenderData { _id: 5 }
}

pub fn create_single_use_command_buffer(
    device: &ash::Device,
    command_pool: vk::CommandPool,
) -> anyhow::Result<vk::CommandBuffer> {
    let alloc_info = vk::CommandBufferAllocateInfo::default()
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);

    let cbs = unsafe { device.allocate_command_buffers(&alloc_info)? };
    Ok(cbs[0])
}
