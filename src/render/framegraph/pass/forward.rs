use ash::vk;

use crate::{
    image::{ImageLifetime, ImageSpec, ResizePolicy},
    render::framegraph::{
        alias::AliasRegistry,
        image::{ImageRequirement, ImageUseSpec},
        pass::{PassDescription, RenderPass},
    },
};

pub struct ForwardPass {
    description: PassDescription,
}

impl RenderPass for ForwardPass {
    fn id(&self) -> u32 {
        todo!()
    }

    fn register_aliases(&self, registry: &mut AliasRegistry) -> anyhow::Result<()> {
        for image_req in &self.description.image_requirements {
            if let Some(spec) = &image_req.spec {
                registry.register(&image_req.alias, spec);
            }
        }
        Ok(())
    }

    fn execute(
        &self,
        frame: &crate::render::Frame,
        cmd: ash::vk::CommandBuffer,
    ) -> anyhow::Result<()> {
        todo!()
    }
}

impl ForwardPass {
    pub fn new() -> anyhow::Result<Self> {
        let description = PassDescription {
            name: "Forward".to_string(),
            image_requirements: vec![
                ImageRequirement {
                    alias: "SwapchainImage".to_string(),
                    use_spec: ImageUseSpec {
                        access_flags: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                        pipeline_stage_flags: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                        image_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        image_aspect_flags: vk::ImageAspectFlags::COLOR,
                    },
                    spec: None,
                },
                ImageRequirement {
                    alias: "DepthBuffer".to_string(),
                    use_spec: ImageUseSpec {
                        access_flags: vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
                        pipeline_stage_flags: vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                            | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS,
                        image_layout: vk::ImageLayout::ATTACHMENT_OPTIMAL,
                        image_aspect_flags: vk::ImageAspectFlags::DEPTH,
                    },
                    spec: Some(ImageSpec::default()),
                },
            ],
            depends_on: vec![],
        };
        Ok(Self { description })
    }
}
