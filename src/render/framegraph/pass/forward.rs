use crate::render::framegraph::{alias::AliasRegistry, pass::RenderPass};

pub struct ForwardPass {}

impl RenderPass for ForwardPass {
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

impl ForwardPass {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {})
    }
}
