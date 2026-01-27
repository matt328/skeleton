use anyhow::Context;
use ash::vk;

use crate::{
    image::ImageLifetime,
    render::{
        framegraph::{
            ImageState,
            alias::{ImageDesc, ImageFormat, ImageSize},
            graph::{ImageAlias, RenderingInfo},
            image::{
                FrameIndexKind, ImageAccess, ImageCreation, ImageIndexing, ImageRequirement,
                ImageUsage,
            },
            pass::{
                ImageBarrierPrecursor, RenderPass, attachment::AttachmentResolver, is_write_access,
            },
        },
        pipeline::GraphicsPipelineDesc,
        shader::ShaderId,
    },
};

pub struct ForwardPass {
    image_requirements: Vec<ImageRequirement>,
    color_value: vk::ClearValue,
}

impl Default for ForwardPass {
    fn default() -> Self {
        let color_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.584, 0.929, 1.0],
            },
        };
        Self {
            image_requirements: vec![ImageRequirement {
                access: ImageAccess {
                    alias: ImageAlias::ForwardColor,
                    usage: ImageUsage {
                        state: ImageState::COLOR_ATTACHMENT_WRITE,
                        aspects: vk::ImageAspectFlags::COLOR,
                    },
                    indexing: ImageIndexing::PerFrame(FrameIndexKind::Frame),
                },
                creation: ImageCreation::Declare(ImageDesc {
                    format: ImageFormat::SwapchainColor,
                    size: ImageSize::SwapchainRelative { scale: 1.0 },
                    usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
                    lifetime: ImageLifetime::PerFrame,
                    samples: vk::SampleCountFlags::TYPE_1,
                }),
            }],
            color_value,
        }
    }
}

impl RenderPass for ForwardPass {
    fn id(&self) -> u32 {
        0
    }

    fn execute(&self, ctx: &super::RenderPassContext) -> anyhow::Result<()> {
        let resolver = AttachmentResolver {
            registry: ctx.registry,
            image_manager: ctx.image_manager,
            frame_index: ctx.frame_index as u32,
            swapchain_image_index: ctx.swapchain_image_index,
        };

        let color_image_view = resolver
            .image_view(ImageAlias::ForwardColor)
            .context("forward pass failed to resolve ForwardColor alias")?;

        let color_attachment_info = [vk::RenderingAttachmentInfo::default()
            .image_view(color_image_view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(self.color_value)];

        let rendering_info = vk::RenderingInfo::default()
            .render_area(vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: ctx.swapchain_extent,
            })
            .layer_count(1)
            .color_attachments(&color_attachment_info);

        unsafe {
            ctx.device.cmd_begin_rendering(ctx.cmd, &rendering_info);
            ctx.device
                .cmd_bind_pipeline(ctx.cmd, vk::PipelineBindPoint::GRAPHICS, ctx.pipeline);
            ctx.device.cmd_set_viewport(ctx.cmd, 0, &[ctx.viewport]);
            ctx.device.cmd_set_scissor(ctx.cmd, 0, &[ctx.snizzor]);
            ctx.device.cmd_draw(ctx.cmd, 3, 1, 0, 0);
            ctx.device.cmd_end_rendering(ctx.cmd);
        }

        Ok(())
    }

    fn image_precursors(&self) -> Vec<super::ImageBarrierPrecursor> {
        self.image_requirements
            .iter()
            .map(|image_req| ImageBarrierPrecursor {
                access: image_req.access,
                is_write: is_write_access(image_req.access.usage.state.access),
            })
            .collect()
    }

    fn buffer_precursors(&self) -> Vec<super::BufferBarrierPrecursor> {
        vec![]
    }

    fn image_requirements(&self) -> &[crate::render::framegraph::image::ImageRequirement] {
        &self.image_requirements
    }

    fn rendering_info(&self) -> crate::render::framegraph::graph::RenderingInfo {
        RenderingInfo {
            color_formats: &[vk::Format::B8G8R8A8_SRGB],
            depth_format: None,
            stencil_format: None,
        }
    }

    fn pipeline_desc(&self) -> crate::render::pipeline::GraphicsPipelineDesc {
        GraphicsPipelineDesc {
            vertex_id: ShaderId::ForwardVert,
            fragment_id: ShaderId::ForwardFrag,
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            color_formats: vec![],
            depth_format: None,
        }
    }
}
