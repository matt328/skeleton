use std::collections::HashMap;

use ash::vk;

use crate::render::{
    Frame,
    framegraph::pass::{BufferBarrierPrecursor, ImageBarrierPrecursor, RenderPass},
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum BufferAlias {
    Placeholder,
}

pub struct BarrierPrecursorPlan {
    image_precursors: HashMap<u32, Vec<ImageBarrierPrecursor>>,
    buffer_precursors: HashMap<u32, Vec<BufferBarrierPrecursor>>,
}

impl BarrierPrecursorPlan {
    pub fn from_passes(passes: &[Box<dyn RenderPass>]) -> Self {
        let image_precursors = passes
            .iter()
            .enumerate()
            .map(|(_, pass)| (pass.id(), pass.image_precursors()))
            .collect::<HashMap<u32, Vec<ImageBarrierPrecursor>>>();

        let buffer_precursors = passes
            .iter()
            .enumerate()
            .map(|(_, pass)| (pass.id(), pass.buffer_precursors()))
            .collect::<HashMap<u32, Vec<BufferBarrierPrecursor>>>();

        Self {
            image_precursors,
            buffer_precursors,
        }
    }

    pub fn emit_pre_pass_barriers(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        frame: &Frame,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn emit_post_pass_barriers(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        frame: &Frame,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
