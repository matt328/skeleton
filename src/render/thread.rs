use std::sync::Arc;

use crossbeam_channel::Receiver;

use crate::{
    caps::RenderCaps,
    messages::{EngineControl, RenderRequest, ShutdownPhase},
};

pub fn render_thread(
    caps: RenderCaps,
    render_rx: Receiver<RenderRequest>,
    control: Arc<EngineControl>,
) -> anyhow::Result<()> {
    let mut in_flight = 0usize;

    while control.phase() != ShutdownPhase::StopRender || in_flight > 0 {
        if let Ok(req) = render_rx.try_recv() {
            log::debug!("Render thread: rendering {}", req.asset_id);
            caps.device.submit("render");
            in_flight += 1;
        }

        in_flight = in_flight.saturating_sub(1);

        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    log::debug!("Render thread shutting down");
    Ok(())
}
