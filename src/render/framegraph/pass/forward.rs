use ash::vk::{self};

use crate::{
    image::ImageLifetime,
    render::{
        framegraph::{
            alias::{ImageDesc, ImageFormat, ImageSize},
            graph::ImageAlias,
            image::{ImageCreation, ImageRequirement, ImageUsage},
            pass::{
                BufferBarrierPrecursor, ImageBarrierPrecursor, RenderPass, RenderPassContext,
                attachment::AttachmentResolver, is_write_access,
            },
        },
        pipeline::GraphicsPipelineDesc,
        shader::ShaderId,
    },
};

pub struct ForwardPass {
    image_requirements: Vec<ImageRequirement>,
    color_value: vk::ClearValue,
    _depth_value: vk::ClearValue,
}

impl Default for ForwardPass {
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

        ForwardPass {
            image_requirements: vec![
                ImageRequirement {
                    alias: ImageAlias::DepthBuffer,
                    creation: ImageCreation::Declare(ImageDesc {
                        format: ImageFormat::Depth,
                        size: ImageSize::SwapchainRelative { scale: 1.0 },
                        usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                        lifetime: ImageLifetime::PerFrame,
                        samples: vk::SampleCountFlags::TYPE_1,
                    }),
                    usage: ImageUsage {
                        access: vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
                        stages: vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS,
                        layout: vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
                        aspects: vk::ImageAspectFlags::DEPTH,
                    },
                },
                ImageRequirement {
                    alias: ImageAlias::SwapchainImage,
                    creation: ImageCreation::UseExisting,
                    usage: ImageUsage {
                        access: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                        stages: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        aspects: vk::ImageAspectFlags::COLOR,
                    },
                },
            ],
            color_value,
            _depth_value: depth_value,
        }
    }
}

impl RenderPass for ForwardPass {
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
                alias: image_req.alias,
                write_access: is_write_access(image_req.usage.access),
                access_flags: image_req.usage.access,
                pipeline_stage_flags: image_req.usage.stages,
                image_layout: image_req.usage.layout,
                aspect_flags: image_req.usage.aspects,
            })
            .collect()
    }

    fn buffer_precursors(&self) -> Vec<BufferBarrierPrecursor> {
        vec![]
    }

    fn pipeline_desc(&self) -> GraphicsPipelineDesc {
        GraphicsPipelineDesc {
            vertex_id: ShaderId::ForwardVert,
            fragment_id: ShaderId::ForwardFrag,
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
            // set up push constants
            // bind texture_shader_bindings
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

impl ForwardPass {}
