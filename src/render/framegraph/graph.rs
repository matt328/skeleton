use std::collections::HashMap;

use anyhow::Context;
use ash::vk;

use crate::render::{
    framegraph::{
        alias::ResolvedRegistry,
        barrier::BarrierPlan,
        pass::{RenderPass, RenderPassContext},
    },
    pipeline::PipelineKey,
    thread::FrameExecutionContext,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ImageAlias {
    SwapchainImage,
    DepthBuffer,
    ForwardColor,
}

#[derive(Clone, Copy)]
pub struct RenderingInfo {
    pub color_formats: &'static [vk::Format],
    pub depth_format: Option<vk::Format>,
    pub stencil_format: Option<vk::Format>,
}

pub struct FrameGraph {
    render_passes: Vec<Box<dyn RenderPass>>,
    pass_pipelines: HashMap<u32, PipelineKey>,
    registry: ResolvedRegistry,
    barrier_plan: BarrierPlan,
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
        }
    }

    pub fn execute(&self, ctx: &FrameExecutionContext) -> anyhow::Result<()> {
        let device = ctx.device;
        let frame = &ctx.frame;

        begin_primary(device, frame.primary_cmd)?;

        for (i, pass) in self.render_passes.iter().enumerate() {
            self.barrier_plan.emit_pre_pass_barriers(
                device,
                frame.primary_cmd,
                frame,
                &ctx.image_manager,
                &self.registry,
                i as u32,
            )?;

            let secondary = frame.secondary_cmds[i];
            let rendering = pass.rendering_info();
            begin_secondary(device, secondary, rendering)?;

            let pipeline_key = self.pass_pipelines.get(&pass.id()).context("")?;

            let pipeline = ctx
                .pipeline_manager
                .get_pipeline(pipeline_key)
                .with_context(|| format!("failed to get pipeline for pass {:?}", pass.id()))?;

            let pass_ctx = RenderPassContext {
                device,
                cmd: ctx.cmd,
                pipeline,
                frame_index: frame.index(),
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
        }

        unsafe {
            device.cmd_execute_commands(frame.primary_cmd, &frame.secondary_cmds);
        }

        self.barrier_plan.emit_post_pass_barriers(
            device,
            frame.primary_cmd,
            frame,
            &ctx.image_manager,
            &self.registry,
        )?;

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
