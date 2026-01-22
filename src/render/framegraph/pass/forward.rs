use ash::vk;

use crate::{
    image::ImageLifetime,
    render::{
        framegraph::{
            alias::{ImageDesc, ImageFormat, ImageSize},
            graph::{ImageAlias, RenderingInfo},
            image::{ImageCreation, ImageRequirement, ImageUsage},
            pass::RenderPass,
        },
        pipeline::GraphicsPipelineDesc,
        shader::ShaderId,
    },
};

pub struct ForwardPass {
    image_requirements: Vec<ImageRequirement>,
}

impl Default for ForwardPass {
    fn default() -> Self {
        Self {
            image_requirements: vec![ImageRequirement {
                alias: ImageAlias::ForwardColor,
                creation: ImageCreation::Declare(ImageDesc {
                    format: ImageFormat::SwapchainColor,
                    size: ImageSize::SwapchainRelative { scale: 1.0 },
                    usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
                    lifetime: ImageLifetime::PerFrame,
                    samples: vk::SampleCountFlags::TYPE_1,
                    debug_name: Some("ForwardColor".to_string()),
                }),
                usage: ImageUsage {
                    access: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                    stages: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                    layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    aspects: vk::ImageAspectFlags::COLOR,
                },
            }],
        }
    }
}

impl RenderPass for ForwardPass {
    fn id(&self) -> u32 {
        1
    }

    fn execute(&self, _ctx: &super::RenderPassContext) -> anyhow::Result<()> {
        Ok(())
    }

    fn image_precursors(&self) -> Vec<super::ImageBarrierPrecursor> {
        vec![]
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
