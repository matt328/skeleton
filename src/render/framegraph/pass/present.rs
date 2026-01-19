use crate::render::framegraph::pass::{RenderPass, RenderPassContext};

pub struct PresentPass {}

impl RenderPass for PresentPass {
    fn id(&self) -> u32 {
        todo!()
    }

    fn execute(&self, ctx: &RenderPassContext) -> anyhow::Result<()> {
        todo!()
    }

    fn image_precursors(&self) -> Vec<super::ImageBarrierPrecursor> {
        todo!()
    }

    fn buffer_precursors(&self) -> Vec<super::BufferBarrierPrecursor> {
        todo!()
    }

    fn pipeline_desc(&self) -> crate::render::pipeline::GraphicsPipelineDesc {
        todo!()
    }

    fn image_requirements(&self) -> &[crate::render::framegraph::image::ImageRequirement] {
        todo!()
    }

    fn rendering_info(&self) -> crate::render::framegraph::graph::RenderingInfo {
        todo!()
    }
}

impl PresentPass {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {})
    }
}
