use anyhow::Context;

use crate::render::{
    Frame,
    framegraph::{
        alias::AliasRegistry,
        pass::{CullingPass, ForwardPass, PresentPass, RenderPass},
    },
};

pub struct BarrierPrecursorPlan {}

type RenderPassList = Vec<Box<dyn RenderPass>>;

pub struct FrameGraph {
    render_passes: RenderPassList,
    alias_registry: AliasRegistry,
    barrier_plan: BarrierPrecursorPlan,
}

impl FrameGraph {
    pub fn new() -> anyhow::Result<Self> {
        let render_passes: RenderPassList = vec![
            Box::new(CullingPass::new().context("FrameGraph failed to create culling pass")?),
            Box::new(ForwardPass::new().context("FrameGraph failed to create forward pass")?),
            Box::new(PresentPass::new().context("FrameGraph failed to create present pass")?),
        ];

        let mut alias_registry =
            AliasRegistry::new().context("FrameGraph failed creating alias registry")?;
        compile_resources(&render_passes, &mut alias_registry);

        let barrier_plan = bake(&render_passes)?;

        Ok(Self {
            render_passes,
            alias_registry,
            barrier_plan,
        })
    }

    pub fn execute(&self, frame: &Frame) {}
}

/// Creates the BarrierPrecursorPlan
fn bake(passes: &RenderPassList) -> anyhow::Result<BarrierPrecursorPlan> {
    Ok(BarrierPrecursorPlan {})
}

/// Registers aliases with AliasRegistry
fn compile_resources(passes: &RenderPassList, registry: &mut AliasRegistry) -> anyhow::Result<()> {
    for pass in passes {
        pass.register_aliases(registry)
            .context("failed to register aliases")?;
    }
    Ok(())
}
