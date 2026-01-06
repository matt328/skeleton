use std::sync::Arc;

use anyhow::Context;

use crate::{
    caps::RenderCaps,
    messages::{EngineControl, ShutdownPhase},
    render::context::RenderContext,
    vulkan::SwapchainCreateCaps,
};

use super::render_packet::RenderData;

pub fn render_thread(
    caps: &RenderCaps,
    control: Arc<EngineControl>,
    swapchain_create_caps: SwapchainCreateCaps,
) -> anyhow::Result<()> {
    let mut render_ctx = RenderContext::new(caps, swapchain_create_caps)
        .context("failed to create render context")?;

    while control.phase() != ShutdownPhase::StopRender {
        render_ctx
            .render_frame(caps)
            .context("failed to render frame")?;
    }

    unsafe {
        caps.device
            .device_wait_idle()
            .context("render: failed waiting idle")?;
    }

    log::debug!("Render thread shutting down");

    Ok(())
}

fn gather_mock_render_data() -> RenderData {
    RenderData { id: 5 }
}
