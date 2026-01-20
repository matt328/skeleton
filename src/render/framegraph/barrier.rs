use std::collections::HashMap;

use anyhow::Context;
use ash::vk::{self, PipelineStageFlags2};

use crate::{
    image::ImageManager,
    render::{
        Frame,
        framegraph::{
            alias::ResolvedRegistry,
            pass::{BufferBarrierPrecursor, ImageBarrierPrecursor, RenderPass},
        },
    },
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum BufferAlias {
    Placeholder,
}

pub struct BarrierPrecursorPlan {
    image_precursors: HashMap<u32, Vec<ImageBarrierPrecursor>>,
    buffer_precursors: HashMap<u32, Vec<BufferBarrierPrecursor>>,
}

impl BarrierPrecursorPlan {
    pub fn from_passes(passes: &[Box<dyn RenderPass>]) -> Self {
        let image_precursors = passes
            .iter()
            .enumerate()
            .map(|(_, pass)| (pass.id(), pass.image_precursors()))
            .collect::<HashMap<u32, Vec<ImageBarrierPrecursor>>>();

        let buffer_precursors = passes
            .iter()
            .enumerate()
            .map(|(_, pass)| (pass.id(), pass.buffer_precursors()))
            .collect::<HashMap<u32, Vec<BufferBarrierPrecursor>>>();

        Self {
            image_precursors,
            buffer_precursors,
        }
    }

    pub fn emit_pre_pass_barriers(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        frame: &Frame,
        image_manager: &ImageManager,
        registry: &ResolvedRegistry,
    ) -> anyhow::Result<()> {
        let mut image_barriers = Vec::new();

        for (_pass_id, precursors) in &self.image_precursors {
            for precursor in precursors {
                let key = registry.images.get(&precursor.alias).context("")?;

                let image_handle = image_manager
                    .image(*key, Some(frame.index() as u32))
                    .context("")?
                    .vk_image;

                let barrier = vk::ImageMemoryBarrier2::default()
                    // can optimize this based on what the image was last used for
                    .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                    .dst_stage_mask(precursor.pipeline_stage_flags)
                    .src_access_mask(if precursor.write_access {
                        vk::AccessFlags2::NONE
                    } else {
                        precursor.access_flags
                    })
                    .dst_access_mask(precursor.access_flags)
                    // Framegraph should track this
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(precursor.image_layout)
                    .image(image_handle)
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(precursor.aspect_flags)
                            .base_mip_level(0)
                            .level_count(vk::REMAINING_MIP_LEVELS)
                            .base_array_layer(0)
                            .layer_count(vk::REMAINING_ARRAY_LAYERS),
                    );
                image_barriers.push(barrier);
            }
        }

        if !image_barriers.is_empty() {
            let dependency_info =
                vk::DependencyInfo::default().image_memory_barriers(&image_barriers);
            unsafe { device.cmd_pipeline_barrier2(cmd, &dependency_info) }
        }
        Ok(())
    }

    pub fn emit_post_pass_barriers(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        frame: &Frame,
        image_manager: &ImageManager,
        registry: &ResolvedRegistry,
    ) -> anyhow::Result<()> {
        let mut image_barriers = Vec::new();

        let key = registry
            .images
            .get(&super::graph::ImageAlias::SwapchainImage)
            .context("")?;

        let image_handle = image_manager
            .image(*key, Some(frame.index() as u32))
            .context("")?
            .vk_image;

        let barrier = vk::ImageMemoryBarrier2::default()
            // can optimize this based on what the image was last used for
            .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE)
            .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
            // Framegraph should track this
            .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .image(image_handle)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(vk::REMAINING_MIP_LEVELS)
                    .base_array_layer(0)
                    .layer_count(vk::REMAINING_ARRAY_LAYERS),
            );

        image_barriers.push(barrier);

        let dependency_info = vk::DependencyInfo::default().image_memory_barriers(&image_barriers);
        unsafe { device.cmd_pipeline_barrier2(cmd, &dependency_info) }
        Ok(())
    }
}
