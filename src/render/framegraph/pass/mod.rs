mod attachment;
mod composition;
mod forward;

use ash::vk;

use crate::{
    image::ImageManager,
    render::{
        framegraph::{
            alias::ResolvedRegistry,
            barrier::BufferAlias,
            image::{ImageAccess, ImageRequirement},
        },
        pipeline::GraphicsPipelineDesc,
        render_packet::RenderData,
    },
};

pub struct BufferBarrierPrecursor {
    _alias: BufferAlias,
    _access_flags: vk::AccessFlags2,
    _pipeline_stage_flags: vk::PipelineStageFlags2,
}

pub struct ImageBarrierPrecursor {
    pub access: ImageAccess,
    pub is_write: bool,
}

#[inline]
pub fn is_write_access(flags: vk::AccessFlags2) -> bool {
    let write_flags = vk::AccessFlags2::COLOR_ATTACHMENT_WRITE
        | vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE
        | vk::AccessFlags2::TRANSFER_WRITE
        | vk::AccessFlags2::SHADER_WRITE
        | vk::AccessFlags2::MEMORY_WRITE;

    flags.intersects(write_flags)
}

pub struct RenderPassContext<'a> {
    pub device: &'a ash::Device,
    pub cmd: vk::CommandBuffer,
    pub pipeline: vk::Pipeline,
    pub frame_index: usize,
    pub swapchain_image_index: u32,
    pub registry: &'a ResolvedRegistry,
    pub image_manager: &'a ImageManager,
    pub swapchain_extent: vk::Extent2D,
    pub viewport: vk::Viewport,
    pub snizzor: vk::Rect2D,
    pub _render_data: &'a RenderData,
}

pub trait RenderPass {
    fn id(&self) -> u32;
    fn execute(&self, ctx: &RenderPassContext) -> anyhow::Result<()>;
    fn image_precursors(&self) -> Vec<ImageBarrierPrecursor>;
    fn buffer_precursors(&self) -> Vec<BufferBarrierPrecursor>;
    fn image_requirements(&self) -> &[ImageRequirement];
    fn rendering_info(&self) -> super::graph::RenderingInfo;
    fn pipeline_desc(&self) -> GraphicsPipelineDesc;
}

pub use composition::CompositionPass;

pub use forward::ForwardPass;
