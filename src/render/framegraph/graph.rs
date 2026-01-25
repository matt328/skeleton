use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use anyhow::Context;
use ash::vk;

use crate::{
    image::CompositeImageKey,
    render::{
        framegraph::{
            COLOR_RANGE, ImageState,
            alias::ResolvedRegistry,
            barrier::BarrierPlan,
            pass::{RenderPass, RenderPassContext},
            transition_image,
        },
        pipeline::PipelineKey,
        thread::FrameExecutionContext,
    },
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ImageAlias {
    SwapchainImage,
    DepthBuffer,
    ForwardColor,
}

impl fmt::Display for ImageAlias {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            ImageAlias::SwapchainImage => "SwapchainImage",
            ImageAlias::DepthBuffer => "DepthBuffer",
            ImageAlias::ForwardColor => "ForwardColor",
        };

        f.write_str(name)
    }
}

#[derive(Clone, Copy)]
pub struct RenderingInfo {
    pub color_formats: &'static [vk::Format],
    pub depth_format: Option<vk::Format>,
    pub stencil_format: Option<vk::Format>,
}

struct SwapchainState {
    first_frame: Vec<bool>,
}

impl Default for SwapchainState {
    fn default() -> Self {
        Self {
            first_frame: vec![true; 3],
        }
    }
}

type PhysicalImageKey = (CompositeImageKey, u32);

struct GraphFirstUse {
    seen: HashSet<PhysicalImageKey>,
}

impl GraphFirstUse {
    fn is_first_use(&mut self, key: PhysicalImageKey) -> bool {
        self.seen.insert(key)
    }
}

pub struct FrameGraph {
    render_passes: Vec<Box<dyn RenderPass>>,
    pass_pipelines: HashMap<u32, PipelineKey>,
    registry: ResolvedRegistry,
    barrier_plan: BarrierPlan,
    first_frame: SwapchainState,
    graph_first_use: GraphFirstUse,
}

impl FrameGraph {
    pub fn new(
        render_passes: Vec<Box<dyn RenderPass>>,
        pass_pipelines: HashMap<u32, PipelineKey>,
        registry: ResolvedRegistry,
        barrier_plan: BarrierPlan,
    ) -> Self {
        Self {
            render_passes,
            pass_pipelines,
            registry,
            barrier_plan,
            first_frame: SwapchainState::default(),
            graph_first_use: GraphFirstUse {
                seen: HashSet::default(),
            },
        }
    }

    pub fn execute(&mut self, ctx: &FrameExecutionContext) -> anyhow::Result<()> {
        let device = ctx.device;
        let frame = &ctx.frame;

        begin_primary(device, frame.primary_cmd)?;

        // Composition Pass
        let composition_pass = self.render_passes.get(1).context("comp pass is None")?;

        let swapchain_image_ckey = self
            .registry
            .images
            .get(&ImageAlias::SwapchainImage)
            .context("no alias for SwapchainImage registered.")?;

        let swapchain_image = ctx
            .image_manager
            .image(*swapchain_image_ckey, Some(frame.swapchain_image_index))
            .context("no image found for SwapchainImage")?;

        let swapchain_first_use = self
            .graph_first_use
            .is_first_use((*swapchain_image_ckey, frame.index() as u32));

        transition_image(
            device,
            frame.primary_cmd,
            swapchain_image.vk_image,
            COLOR_RANGE,
            if swapchain_first_use {
                ImageState::UNDEFINED
            } else {
                ImageState::PRESENT
            },
            ImageState::COLOR_ATTACHMENT_WRITE,
            format!("swapchain #{:?}", frame.swapchain_image_index).as_ref(),
        );

        let secondary = frame.secondary_cmds[1];

        begin_secondary(device, secondary, composition_pass.rendering_info())?;
        let pipeline_key = self
            .pass_pipelines
            .get(&composition_pass.id())
            .context("failed to get pipeline")?;

        let pipeline = ctx
            .pipeline_manager
            .get_pipeline(pipeline_key)
            .with_context(|| {
                format!(
                    "failed to get pipeline for pass {:?}",
                    composition_pass.id()
                )
            })?;

        let pass_ctx = RenderPassContext {
            device,
            cmd: secondary,
            pipeline,
            frame_index: frame.index(),
            swapchain_image_index: frame.swapchain_image_index,
            registry: &self.registry,
            image_manager: ctx.image_manager,
            swapchain_extent: ctx.swapchain_extent,
            viewport: ctx.viewport,
            snizzor: ctx.snizzor,
            _render_data: ctx.render_data,
        };

        composition_pass
            .execute(&pass_ctx)
            .context("framegraph failed to execute pass")?;

        end_secondary(device, secondary)?;

        unsafe {
            device.cmd_execute_commands(frame.primary_cmd, &[secondary]);
        }

        end_primary(device, frame.primary_cmd)?;
        self.first_frame.first_frame[frame.index()] = false;
        Ok(())
    }
}

fn begin_primary(device: &ash::Device, cmd: vk::CommandBuffer) -> anyhow::Result<()> {
    unsafe {
        device
            .begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::default())
            .map_err(|e| anyhow::anyhow!(e))
    }
}

fn begin_secondary(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    rendering: RenderingInfo,
) -> anyhow::Result<()> {
    let mut info = vk::CommandBufferInheritanceRenderingInfo::default()
        .color_attachment_formats(rendering.color_formats)
        .depth_attachment_format(rendering.depth_format.unwrap_or(vk::Format::UNDEFINED))
        .stencil_attachment_format(rendering.stencil_format.unwrap_or(vk::Format::UNDEFINED))
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    let inheritance = vk::CommandBufferInheritanceInfo::default().push_next(&mut info);

    let begin_info = vk::CommandBufferBeginInfo::default().inheritance_info(&inheritance);

    unsafe {
        device
            .begin_command_buffer(cmd, &begin_info)
            .map_err(|e| anyhow::anyhow!(e))
    }
}

fn end_secondary(device: &ash::Device, cmd: vk::CommandBuffer) -> anyhow::Result<()> {
    unsafe {
        device
            .end_command_buffer(cmd)
            .context("device failed to end_command_buffer")
    }
}

fn end_primary(device: &ash::Device, cmd: vk::CommandBuffer) -> anyhow::Result<()> {
    unsafe {
        device
            .end_command_buffer(cmd)
            .map_err(|e| anyhow::anyhow!(e))
    }
}
