use crate::render::framegraph::{alias::AliasRegistry, pass::RenderPass};

pub struct PresentPass {}

impl RenderPass for PresentPass {
    fn id(&self) -> u32 {
        todo!()
    }

    fn register_aliases(&self, registry: &mut AliasRegistry) -> anyhow::Result<()> {
        todo!()
    }

    fn execute(
        &self,
        frame: &crate::render::Frame,
        cmd: ash::vk::CommandBuffer,
    ) -> anyhow::Result<()> {
        todo!()
    }
}

impl PresentPass {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {})
    }
}
