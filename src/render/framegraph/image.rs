use std::fmt;

use ash::vk;

use crate::image::ImageSpec;

#[derive(PartialEq, Eq)]
pub struct ImageUseSpec {
    pub access_flags: vk::AccessFlags2,
    pub pipeline_stage_flags: vk::PipelineStageFlags2,
    pub image_layout: vk::ImageLayout,
    pub image_aspect_flags: vk::ImageAspectFlags,
}

impl fmt::Display for ImageUseSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ImageUseDescription(accessFlags={:?}, stageFlags={:?}, imageLayout={:?}, aspectFlags={:?})",
            self.access_flags,
            self.pipeline_stage_flags,
            self.image_layout,
            self.image_aspect_flags,
        )
    }
}

pub struct ImageRequirement {
    pub alias: String,
    pub spec: Option<ImageSpec>,
    pub use_spec: ImageUseSpec,
}

impl fmt::Display for ImageRequirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ImageRequirement(alias=\"{}\", spec={}, useSpec={})",
            self.alias,
            match &self.spec {
                Some(spec) => spec.to_string(),
                None => "<none>".to_string(),
            },
            self.use_spec
        )
    }
}
