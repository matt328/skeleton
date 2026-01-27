use std::fmt;

use ash::vk;

use crate::render::framegraph::{ImageState, alias::ImageDesc, graph::ImageAlias};

#[derive(Copy, Clone, Debug)]
pub enum ImageIndexing {
    _Global,
    PerFrame(FrameIndexKind),
}

#[derive(Copy, Clone, Debug)]
pub enum FrameIndexKind {
    Frame,
    Swapchain,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ImageCreation {
    Declare(ImageDesc),
    UseExisting,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ImageUsage {
    pub state: ImageState,
    pub aspects: vk::ImageAspectFlags,
}

impl ImageUsage {
    pub fn subresource_range(self) -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange {
            aspect_mask: self.aspects,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        }
    }
}

impl fmt::Display for ImageUsage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ImageUseDescription(accessFlags={:?}, stageFlags={:?}, imageLayout={:?}, aspectFlags={:?})",
            self.state.access, self.state.stage, self.state.layout, self.aspects,
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ImageAccess {
    pub alias: ImageAlias,
    pub usage: ImageUsage,
    pub indexing: ImageIndexing,
}

#[derive(Clone, Copy, Debug)]
pub struct ImageRequirement {
    pub access: ImageAccess,
    pub creation: ImageCreation,
}
