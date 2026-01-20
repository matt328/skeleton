use std::collections::HashMap;

use anyhow::Context;
use ash::vk;

use crate::{
    image::{CompositeImageKey, CompositeImageViewKey, ImageManager},
    render::{
        framegraph::{
            FrameGraph,
            alias::{AliasRegistry, ImageResolveContext},
            barrier::BarrierPrecursorPlan,
            graph::ImageAlias,
            image::ImageCreation,
            pass::RenderPass,
        },
        pipeline::PipelineManager,
    },
};

type RenderPassList = Vec<Box<dyn RenderPass>>;

pub struct FramegraphBuilder<'a> {
    image_manager: &'a mut ImageManager,
    allocator: &'a vk_mem::Allocator,
    device: &'a ash::Device,
    render_passes: Vec<Box<dyn RenderPass>>,
    swapchain_formats: &'a [vk::Format],
    _depth_format: vk::Format,
    pipeline_manager: &'a mut PipelineManager,
}

impl<'a> FramegraphBuilder<'a> {
    pub fn new(
        image_manager: &'a mut ImageManager,
        allocator: &'a vk_mem::Allocator,
        device: &'a ash::Device,
        swapchain_formats: &'a [vk::Format],
        depth_format: vk::Format,
        pipeline_manager: &'a mut PipelineManager,
    ) -> Self {
        Self {
            image_manager,
            allocator,
            device,
            render_passes: Vec::new(),
            swapchain_formats,
            _depth_format: depth_format,
            pipeline_manager,
        }
    }

    pub fn add_pass(mut self, pass: impl RenderPass + 'static) -> Self {
        self.render_passes.push(Box::new(pass));
        self
    }

    pub fn build(
        self,
        ctx: &ImageResolveContext,
        keys: (CompositeImageKey, CompositeImageViewKey),
    ) -> anyhow::Result<FrameGraph> {
        let mut alias_registry = AliasRegistry::default();

        alias_registry.declare_external_image(ImageAlias::SwapchainImage, keys)?;

        compile_resources(&self.render_passes, &mut alias_registry)?;

        let barrier_plan = bake(&self.render_passes)?;
        let im = self.image_manager;
        let registry = alias_registry
            .resolve(im, self.allocator, ctx)
            .context("FrameGraphBuilder failed to build resources")?;

        let pipeline_manager = self.pipeline_manager;

        let mut pipelines = HashMap::default();
        for pass in &self.render_passes {
            let mut desc = pass.pipeline_desc();
            desc.color_formats = self.swapchain_formats.to_vec();
            let pipeline_key = pipeline_manager.get_or_create(self.device, desc)?;
            pipelines.insert(pass.id(), pipeline_key);
        }

        Ok(FrameGraph::new(
            self.render_passes,
            pipelines,
            registry,
            barrier_plan,
        ))
    }
}

/// Creates the BarrierPrecursorPlan
fn bake(passes: &[Box<dyn RenderPass>]) -> anyhow::Result<BarrierPrecursorPlan> {
    Ok(BarrierPrecursorPlan::from_passes(passes))
}

/// Registers aliases with AliasRegistry
fn compile_resources(passes: &RenderPassList, registry: &mut AliasRegistry) -> anyhow::Result<()> {
    for pass in passes {
        for req in pass.image_requirements() {
            match &req.creation {
                ImageCreation::Declare(desc) => {
                    registry.declare_image(req.alias, desc.clone())?;
                }
                ImageCreation::UseExisting => {
                    // registry.require_image(req.alias)?;
                }
            }
        }
    }
    Ok(())
}
