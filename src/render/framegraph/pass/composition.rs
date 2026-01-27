use ash::vk::{self};

use crate::render::{
    framegraph::{
        ImageState,
        graph::ImageAlias,
        image::{
            FrameIndexKind, ImageAccess, ImageCreation, ImageIndexing, ImageRequirement, ImageUsage,
        },
        pass::{
            BufferBarrierPrecursor, ImageBarrierPrecursor, RenderPass, RenderPassContext,
            attachment::AttachmentResolver, is_write_access,
        },
    },
    pipeline::GraphicsPipelineDesc,
    shader::ShaderId,
};

pub struct CompositionPass {
    image_requirements: Vec<ImageRequirement>,
    color_value: vk::ClearValue,
    _depth_value: vk::ClearValue,
}

impl Default for CompositionPass {
    fn default() -> Self {
        let color_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.392, 0.584, 0.929, 1.0],
            },
        };
        let depth_value = vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            },
        };

        CompositionPass {
            image_requirements: vec![
                ImageRequirement {
                    access: ImageAccess {
                        alias: ImageAlias::SwapchainImage,
                        usage: ImageUsage {
                            state: ImageState {
                                access: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                                stage: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                            },
                            aspects: vk::ImageAspectFlags::COLOR,
                        },
                        indexing: ImageIndexing::PerFrame(FrameIndexKind::Swapchain),
                    },
                    creation: ImageCreation::UseExisting,
                },
                ImageRequirement {
                    access: ImageAccess {
                        alias: ImageAlias::ForwardColor,
                        usage: ImageUsage {
                            // Create ImageState for Sampling an image
                            state: ImageState::COLOR_ATTACHMENT_WRITE,
                            aspects: vk::ImageAspectFlags::COLOR,
                        },
                        indexing: ImageIndexing::PerFrame(FrameIndexKind::Frame),
                    },
                    creation: ImageCreation::UseExisting,
                },
            ],
            color_value,
            _depth_value: depth_value,
        }
    }
}

impl RenderPass for CompositionPass {
    fn id(&self) -> u32 {
        1
    }

    fn image_requirements(&self) -> &[ImageRequirement] {
        &self.image_requirements
    }

    fn image_precursors(&self) -> Vec<ImageBarrierPrecursor> {
        self.image_requirements
            .iter()
            .map(|image_req| ImageBarrierPrecursor {
                access: image_req.access,
                is_write: is_write_access(image_req.access.usage.state.access),
            })
            .collect()
    }

    fn buffer_precursors(&self) -> Vec<BufferBarrierPrecursor> {
        vec![]
    }

    fn pipeline_desc(&self) -> GraphicsPipelineDesc {
        GraphicsPipelineDesc {
            vertex_id: ShaderId::CompositionVert,
            fragment_id: ShaderId::CompositionFrag,
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            color_formats: vec![],
            depth_format: None,
        }
    }

    fn execute(&self, ctx: &RenderPassContext) -> anyhow::Result<()> {
        let resolver = AttachmentResolver {
            registry: ctx.registry,
            image_manager: ctx.image_manager,
            frame_index: ctx.frame_index as u32,
            swapchain_image_index: ctx.swapchain_image_index,
        };

        let swapchain_image_view = resolver.image_view(ImageAlias::SwapchainImage)?;

        let color_attachment_info = [vk::RenderingAttachmentInfo::default()
            .image_view(swapchain_image_view)
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

    fn rendering_info(&self) -> crate::render::framegraph::graph::RenderingInfo {
        crate::render::framegraph::graph::RenderingInfo {
            color_formats: &[vk::Format::B8G8R8A8_SRGB],
            depth_format: None,
            stencil_format: None,
        }
    }
}

impl CompositionPass {}
