use std::fmt;

use ash::vk;

use crate::render::framegraph::{alias::ImageDesc, graph::ImageAlias};

#[derive(Clone, PartialEq)]
pub enum ImageCreation {
    Declare(ImageDesc),
    UseExisting,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ImageUsage {
    pub access: vk::AccessFlags2,
    pub stages: vk::PipelineStageFlags2,
    pub layout: vk::ImageLayout,
    pub aspects: vk::ImageAspectFlags,
}

impl fmt::Display for ImageUsage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ImageUseDescription(accessFlags={:?}, stageFlags={:?}, imageLayout={:?}, aspectFlags={:?})",
            self.access, self.stages, self.layout, self.aspects,
        )
    }
}

pub struct ImageRequirement {
    pub alias: ImageAlias,
    pub creation: ImageCreation,
    pub usage: ImageUsage,
}
