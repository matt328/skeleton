use std::collections::HashMap;

use anyhow::Context;
use ash::vk::{self};

use crate::{
    image::ImageManager,
    render::{
        Frame,
        framegraph::{
            alias::ResolvedRegistry,
            graph::ImageAlias,
            pass::{BufferBarrierPrecursor, ImageBarrierPrecursor, RenderPass},
        },
    },
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum BufferAlias {
    _Placeholder,
}

struct TrackedImageState {
    layout: vk::ImageLayout,
    stage: vk::PipelineStageFlags2,
    access: vk::AccessFlags2,
}

struct ImageBarrierDesc {
    pub alias: ImageAlias,
    pub src_stage: vk::PipelineStageFlags2,
    pub src_access: vk::AccessFlags2,
    pub old_layout: vk::ImageLayout,
    pub dst_stage: vk::PipelineStageFlags2,
    pub dst_access: vk::AccessFlags2,
    pub new_layout: vk::ImageLayout,
    pub aspect_flags: vk::ImageAspectFlags,
}

pub struct BarrierPlan {
    image_barrier_descs: HashMap<u32, Vec<ImageBarrierDesc>>,
    _buffer_precursors: HashMap<u32, Vec<BufferBarrierPrecursor>>,
}

impl BarrierPlan {
    pub fn from_passes<'a>(
        passes: &[Box<dyn RenderPass>],
        aliases: impl IntoIterator<Item = &'a ImageAlias>,
    ) -> Self {
        let mut image_states = initial_states(aliases);

        let mut image_barrier_descs: HashMap<u32, Vec<ImageBarrierDesc>> = HashMap::default();

        for pass in passes {
            for precursor in pass.image_precursors() {
                let prev = image_states.get(&precursor.alias);

                let barrier_desc = build_barrier_desc(prev, &precursor);

                image_barrier_descs
                    .entry(pass.id())
                    .or_insert_with(Vec::new)
                    .push(barrier_desc);

                image_states.insert(
                    precursor.alias,
                    TrackedImageState {
                        layout: precursor.image_layout,
                        stage: precursor.pipeline_stage_flags,
                        access: precursor.access_flags,
                    },
                );
            }
        }

        let buffer_precursors = passes
            .iter()
            .enumerate()
            .map(|(_, pass)| (pass.id(), pass.buffer_precursors()))
            .collect::<HashMap<u32, Vec<BufferBarrierPrecursor>>>();

        Self {
            image_barrier_descs,
            _buffer_precursors: buffer_precursors,
        }
    }

    pub fn emit_pre_pass_barriers(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        frame: &Frame,
        image_manager: &ImageManager,
        registry: &ResolvedRegistry,
        pass_id: u32,
    ) -> anyhow::Result<()> {
        let mut image_barriers = Vec::new();

        if let Some(descs) = self.image_barrier_descs.get(&pass_id) {
            for desc in descs {
                let key = registry.images.get(&desc.alias).context("")?;

                let image_handle = image_manager
                    .image(*key, Some(frame.index() as u32))
                    .context("")?
                    .vk_image;

                let barrier = vk::ImageMemoryBarrier2::default()
                    // can optimize this based on what the image was last used for
                    .src_stage_mask(desc.src_stage)
                    .dst_stage_mask(desc.dst_stage)
                    .src_access_mask(desc.src_access)
                    .dst_access_mask(desc.dst_access)
                    // Framegraph should track this
                    .old_layout(desc.old_layout)
                    .new_layout(desc.new_layout)
                    .image(image_handle)
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(desc.aspect_flags)
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

fn initial_states<'a>(
    aliases: impl IntoIterator<Item = &'a ImageAlias>,
) -> HashMap<ImageAlias, TrackedImageState> {
    let mut states = HashMap::new();
    for alias in aliases {
        let state = match alias {
            ImageAlias::SwapchainImage => TrackedImageState {
                layout: vk::ImageLayout::PRESENT_SRC_KHR,
                stage: vk::PipelineStageFlags2::BOTTOM_OF_PIPE,
                access: vk::AccessFlags2::NONE,
            },

            _ => TrackedImageState {
                layout: vk::ImageLayout::UNDEFINED,
                stage: vk::PipelineStageFlags2::NONE,
                access: vk::AccessFlags2::NONE,
            },
        };

        states.insert(*alias, state);
    }

    states
}

fn build_barrier_desc(
    prev: Option<&TrackedImageState>,
    precursor: &ImageBarrierPrecursor,
) -> ImageBarrierDesc {
    let (src_stage, src_access, old_layout) = match prev {
        Some(p) => (p.stage, p.access, p.layout),
        None => (
            vk::PipelineStageFlags2::NONE,
            vk::AccessFlags2::NONE,
            vk::ImageLayout::UNDEFINED,
        ),
    };

    ImageBarrierDesc {
        alias: precursor.alias,

        src_stage,
        src_access,
        old_layout,

        dst_stage: precursor.pipeline_stage_flags,
        dst_access: precursor.access_flags,
        new_layout: precursor.image_layout,

        aspect_flags: precursor.aspect_flags,
    }
}
