use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender};

use crate::{
    caps::UploadCaps,
    messages::{EngineControl, RenderRequest, ShutdownPhase, UploadComplete, UploadRequest},
};

pub fn upload_thread(
    caps: UploadCaps,
    upload_rx: Receiver<UploadRequest>,
    render_tx: Sender<RenderRequest>,
    complete_tx: Sender<UploadComplete>,
    control: Arc<EngineControl>,
) -> anyhow::Result<()> {
    caps.device.create_command_pool("upload");
    let mut in_flight = 0usize;

    while control.phase() != ShutdownPhase::StopUpload || in_flight > 0 {
        if control.phase() == ShutdownPhase::Running
            && let Ok(req) = upload_rx.try_recv()
        {
            log::debug!("Upload thread: uploading {}", req.asset_id);
            in_flight += 1;
            caps.device.submit("upload");
            let _ = render_tx.send(RenderRequest {
                asset_id: req.asset_id,
            });
            let _ = complete_tx.send(UploadComplete {
                asset_id: req.asset_id,
            });
        }
        in_flight = in_flight.saturating_sub(1);
    }
    log::debug!("Upload Thread shutting down");

    Ok(())
}
