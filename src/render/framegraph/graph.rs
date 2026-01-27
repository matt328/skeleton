use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use anyhow::Context;
use ash::vk;

use crate::{
    image::{CompositeImageKey, FrameIndex},
    render::{
        framegraph::{
            ImageState,
            alias::ResolvedRegistry,
            barrier::BarrierPlan,
            image::{FrameIndexKind, ImageIndexing},
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
    ForwardColor,
}

impl fmt::Display for ImageAlias {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            ImageAlias::SwapchainImage => "SwapchainImage",
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

type PhysicalImageKey = (CompositeImageKey, PhysicalImageInstance);

#[derive(PartialEq, Eq, Hash)]
enum PhysicalImageInstance {
    Frame(FrameIndex),
    Swapchain(u32),
    Global,
}

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
            graph_first_use: GraphFirstUse {
                seen: HashSet::default(),
            },
        }
    }

    pub fn execute(&mut self, ctx: &FrameExecutionContext) -> anyhow::Result<()> {
        let device = ctx.device;
        let frame = &ctx.frame;

        begin_primary(device, frame.primary_cmd)?;

        for (i, pass) in self.render_passes.iter().enumerate() {
            if let Some(barrier_descs) = self.barrier_plan.image_barrier_descs.get(&pass.id()) {
                for desc in barrier_descs {
                    let ckey = self
                        .registry
                        .images
                        .get(&desc.alias)
                        .context(format!("failed to find image: {:?}", desc.alias))?;

                    let mut debug_frame_index: Option<FrameIndex> = None;

                    let image = match desc.indexing {
                        ImageIndexing::Global => match ckey {
                            CompositeImageKey::Global(image_key) => {
                                ctx.image_manager.image_global(*image_key)
                            }
                            CompositeImageKey::PerFrame(_) => unreachable!(
                                "Global Indexing should not reference per-frame composite keys"
                            ),
                        },

                        ImageIndexing::PerFrame(frame_index_kind) => {
                            let frame_index = match frame_index_kind {
                                FrameIndexKind::Frame => FrameIndex::Frame(frame.index as u32),
                                FrameIndexKind::Swapchain => {
                                    FrameIndex::Swapchain(frame.swapchain_image_index)
                                }
                            };
                            debug_frame_index = Some(frame_index);
                            ctx.image_manager.resolve_image(*ckey, frame_index)
                        }
                    };

                    let instance = match debug_frame_index {
                        Some(FrameIndex::Frame(i)) => {
                            PhysicalImageInstance::Frame(FrameIndex::Frame(i))
                        }
                        Some(FrameIndex::Swapchain(i)) => PhysicalImageInstance::Swapchain(i),
                        None => PhysicalImageInstance::Global,
                    };

                    let first_use = self.graph_first_use.is_first_use((*ckey, instance));

                    let old_state = if first_use {
                        ImageState::UNDEFINED
                    } else {
                        desc.old_state
                    };

                    log::trace!(
                        "[Frame {:?} index: {:?}] Image={:?}[{:?}] old={:?} new={:?}",
                        frame.number,
                        frame.index,
                        desc.alias,
                        debug_frame_index,
                        old_state,
                        desc.new_state
                    );
                    transition_image(
                        device,
                        frame.primary_cmd,
                        image.vk_image,
                        desc.subresource_range,
                        old_state,
                        desc.new_state,
                        format!("Image").as_ref(),
                    )
                }
            }

            let secondary = frame.secondary_cmds[i];

            begin_secondary(device, secondary, pass.rendering_info())?;
            let pipeline_key = self
                .pass_pipelines
                .get(&pass.id())
                .context("failed to get pipeline")?;

            let pipeline = ctx
                .pipeline_manager
                .get_pipeline(pipeline_key)
                .with_context(|| format!("failed to get pipeline for pass {:?}", pass.id()))?;

            let pass_ctx = RenderPassContext {
                device,
                cmd: secondary,
                pipeline,
                frame_index: frame.index,
                swapchain_image_index: frame.swapchain_image_index,
                registry: &self.registry,
                image_manager: ctx.image_manager,
                swapchain_extent: ctx.swapchain_extent,
                viewport: ctx.viewport,
                snizzor: ctx.snizzor,
                _render_data: ctx.render_data,
            };

            pass.execute(&pass_ctx)
                .context("framegraph failed to execute pass")?;

            end_secondary(device, secondary)?;

            unsafe {
                device.cmd_execute_commands(frame.primary_cmd, &[secondary]);
            }
        }
        end_primary(device, frame.primary_cmd)?;
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
