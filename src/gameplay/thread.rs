use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender};

use crate::messages::{EngineControl, ShutdownPhase, UploadComplete, UploadRequest};

pub fn gameplay_thread(
    upload_tx: Sender<UploadRequest>,
    complete_rx: Receiver<UploadComplete>,
    control: Arc<EngineControl>,
) -> anyhow::Result<()> {
    let mut next_asset = 1;
    while control.phase() == ShutdownPhase::Running {
        let asset = next_asset;
        next_asset += 1;

        if upload_tx.send(UploadRequest { asset_id: asset }).is_err() {
            break;
        }

        match complete_rx.recv() {
            Ok(msg) => {
                log::debug!("Gameplay: upload complete {}", msg.asset_id);
            }
            Err(_) => break,
        }

        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    log::debug!("Gameplay Thread shutting down");

    Ok(())
}
