use std::collections::HashMap;

use anyhow::Context;
use ash::vk;

use crate::{
    image::{CompositeImageKey, CompositeImageViewKey, ImageManager},
    render::{
        framegraph::{
            FrameGraph,
            alias::{AliasRegistry, ImageResolveContext},
            barrier::BarrierPlan,
            graph::ImageAlias,
            image::ImageCreation,
            pass::RenderPass,
        },
        pipeline::PipelineManager,
    },
    vulkan::DeviceContext,
};

type RenderPassList = Vec<Box<dyn RenderPass>>;

pub struct FramegraphBuilder<'a> {
    image_manager: &'a mut ImageManager,
    allocator: &'a vk_mem::Allocator,
    device_context: DeviceContext,
    render_passes: Vec<Box<dyn RenderPass>>,
    swapchain_formats: &'a [vk::Format],
    _depth_format: vk::Format,
    pipeline_manager: &'a mut PipelineManager,
}

impl<'a> FramegraphBuilder<'a> {
    pub fn new(
        image_manager: &'a mut ImageManager,
        allocator: &'a vk_mem::Allocator,
        device_context: DeviceContext,
        swapchain_formats: &'a [vk::Format],
        depth_format: vk::Format,
        pipeline_manager: &'a mut PipelineManager,
    ) -> Self {
        Self {
            image_manager,
            allocator,
            device_context,
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

        let im = self.image_manager;

        let registry = alias_registry
            .resolve(im, self.allocator, ctx)
            .context("FrameGraphBuilder failed to build resources")?;

        let barrier_plans = build_barrier_plans(&self.render_passes, registry.images.keys())?;

        log::debug!("Barrier Plan: {}", barrier_plans);

        let pipeline_manager = self.pipeline_manager;

        let mut pipelines = HashMap::default();

        for pass in &self.render_passes {
            let mut desc = pass.pipeline_desc();
            desc.color_formats = self.swapchain_formats.to_vec();
            let pipeline_key = pipeline_manager.get_or_create(&self.device_context, desc)?;
            pipelines.insert(pass.id(), pipeline_key);
        }

        Ok(FrameGraph::new(
            self.render_passes,
            pipelines,
            registry,
            barrier_plans,
        ))
    }
}

/// Creates the BarrierPlans
fn build_barrier_plans<'a>(
    passes: &[Box<dyn RenderPass>],
    aliases: impl IntoIterator<Item = &'a ImageAlias>,
) -> anyhow::Result<BarrierPlan> {
    Ok(BarrierPlan::from_passes(passes, aliases))
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
