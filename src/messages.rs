use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug)]
pub struct UploadRequest {
    pub asset_id: u32,
}

#[derive(Debug)]
pub struct RenderRequest {
    pub asset_id: u32,
}

#[derive(Debug)]
pub struct UploadComplete {
    pub asset_id: u32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ShutdownPhase {
    Running,
    StopGameplay,
    StopUpload,
    StopRender,
}

pub struct EngineControl {
    phase: AtomicU8,
}

impl EngineControl {
    pub fn new() -> Self {
        Self {
            phase: AtomicU8::new(ShutdownPhase::Running as u8),
        }
    }

    pub fn set_phase(&self, phase: ShutdownPhase) {
        self.phase.store(phase as u8, Ordering::Release);
    }

    pub fn phase(&self) -> ShutdownPhase {
        match self.phase.load(Ordering::Acquire) {
            0 => ShutdownPhase::Running,
            1 => ShutdownPhase::StopGameplay,
            2 => ShutdownPhase::StopUpload,
            _ => ShutdownPhase::StopRender,
        }
    }
}
