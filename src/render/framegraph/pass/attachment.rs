use anyhow::Context;
use ash::vk;

use crate::{
    image::ImageManager,
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
            self.swapchain_image_index
        } else {
            self.frame_index
        };
        let image_view_key = self
            .registry
            .image_views
            .get(&alias)
            .copied()
            .with_context(|| {
                format!(
                    "no image view registered for alias {:?} (index = {})",
                    alias, index
                )
            })?;

        let image_view = self
            .image_manager
            .image_view(image_view_key, Some(index))
            .with_context(|| {
                format!(
                    "failed to resolve vk::ImageView for alias {:?} (index = {})",
                    alias, index
                )
            })?;

        Ok(image_view.vk_image_view)
    }
}
