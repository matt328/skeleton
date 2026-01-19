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
}

impl<'a> AttachmentResolver<'a> {
    pub fn image_view(&self, alias: ImageAlias) -> anyhow::Result<vk::ImageView> {
        let image_view_key = self
            .registry
            .image_views
            .get(&alias)
            .copied()
            .with_context(|| {
                format!(
                    "no image view registered for alias {:?} (frame_index = {})",
                    alias, self.frame_index
                )
            })?;

        let image_view = self
            .image_manager
            .image_view(image_view_key, Some(self.frame_index))
            .with_context(|| {
                format!(
                    "failed to resolve vk::ImageView for alias {:?} (frame_index = {})",
                    alias, self.frame_index
                )
            })?;

        Ok(image_view.vk_image_view)
    }
}
