use anyhow::Context;
use tracy_client::span;

use super::frame::Frame;

pub struct FrameRing {
    frames: Vec<Frame>,
    index: usize,
    pub number: u64,
}

impl FrameRing {
    pub fn new(frames: Vec<Frame>) -> Self {
        assert!(!frames.is_empty());
        Self {
            frames,
            index: 0,
            number: 0,
        }
    }

    pub fn acquire(&mut self, device: &ash::Device) -> anyhow::Result<&mut Frame> {
        let _frame_span = span!("acquire");
        let len = self.frames.len();
        let frame = &mut self.frames[self.index];
        frame.number = self.number;
        self.number += 1;
        frame.wait(device).context("failed to wait for frame")?;
        self.index = (self.index + 1) % len;
        Ok(frame)
    }

    pub fn _len(&self) -> usize {
        self.frames.len()
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        log::trace!("Destroying Frame Ring");
        for frame in &mut self.frames {
            frame.destroy(device);
        }
        self.frames.clear();
    }
}
