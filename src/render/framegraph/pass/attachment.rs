use anyhow::Context;
use ash::vk;

use crate::{
    image::{FrameIndex, ImageManager},
    render::framegraph::{alias::ResolvedRegistry, graph::ImageAlias},
};

pub struct AttachmentResolver<'a> {
    pub registry: &'a ResolvedRegistry,
    pub image_manager: &'a ImageManager,
    pub frame_index: u32,
    pub swapchain_image_index: u32,
}

impl<'a> AttachmentResolver<'a> {
    pub fn image_view(&self, alias: ImageAlias) -> anyhow::Result<vk::ImageView> {
        let index = if alias == ImageAlias::SwapchainImage {
            FrameIndex::Swapchain(self.swapchain_image_index)
        } else {
            FrameIndex::Frame(self.frame_index)
        };
        let image_view_key = self
            .registry
            .image_views
            .get(&alias)
            .copied()
            .with_context(|| {
                format!(
                    "no image view registered for alias {:?} (index = {:?})",
                    alias, index
                )
            })?;

        let image_view = self.image_manager.resolve_image_view(image_view_key, index);

        Ok(image_view.vk_image_view)
    }
}
