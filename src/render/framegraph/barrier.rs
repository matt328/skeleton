use std::{collections::HashMap, fmt};

use ash::vk::{self};

use crate::{
    image::{ImageKey, LogicalImageKey},
    render::framegraph::{
        ImageState,
        graph::ImageAlias,
        image::{FrameIndexKind, ImageIndexing},
        pass::{BufferBarrierPrecursor, ImageBarrierPrecursor, RenderPass},
    },
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum BufferAlias {
    _Placeholder,
}

#[derive(Copy, Clone, Debug)]
pub enum ImageUse {
    Global(ImageKey),
    PerFrame {
        key: LogicalImageKey,
        index: FrameIndexKind,
    },
}

pub struct ImageBarrierDesc {
    pub alias: ImageAlias,
    pub indexing: ImageIndexing,
    pub old_state: ImageState,
    pub new_state: ImageState,
    pub subresource_range: vk::ImageSubresourceRange,
}

impl fmt::Display for ImageBarrierDesc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.alias)?;
        writeln!(
            f,
            "layout: {:?} -> {:?}",
            self.old_state.layout, self.new_state.layout
        )?;
        writeln!(
            f,
            "stage:  {:?} -> {:?}",
            self.old_state.stage, self.new_state.stage
        )?;
        write!(
            f,
            "access: {:?} -> {:?}",
            self.old_state.access, self.new_state.access
        )
    }
}

pub struct BarrierPlan {
    pub image_barrier_descs: HashMap<u32, Vec<ImageBarrierDesc>>,
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
                let prev = image_states.get(&precursor.access.alias);

                let barrier_desc = build_barrier_desc(prev, &precursor);

                image_barrier_descs
                    .entry(pass.id())
                    .or_insert_with(Vec::new)
                    .push(barrier_desc);

                image_states.insert(precursor.access.alias, precursor.access.usage.state);
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
}

impl fmt::Display for BarrierPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "BarrierPlan {{")?;

        if self.image_barrier_descs.is_empty() {
            writeln!(f, "  image barriers: <none>")?;
        } else {
            writeln!(f, "  image barriers:")?;
            for (key, barriers) in &self.image_barrier_descs {
                writeln!(f, "    pass {}:", key)?;

                if barriers.is_empty() {
                    writeln!(f, "      <none>")?;
                    continue;
                }

                for (i, barrier) in barriers.iter().enumerate() {
                    writeln!(f, "      [{}] {}", i, barrier)?;
                }
            }
        }

        // Intentionally not dumping buffer precursors yet
        if !self._buffer_precursors.is_empty() {
            writeln!(
                f,
                "  buffer precursors: {} (not displayed)",
                self._buffer_precursors.len()
            )?;
        }

        write!(f, "}}")
    }
}

fn initial_states<'a>(
    aliases: impl IntoIterator<Item = &'a ImageAlias>,
) -> HashMap<ImageAlias, ImageState> {
    let mut states = HashMap::new();
    for alias in aliases {
        let state = match alias {
            ImageAlias::SwapchainImage => ImageState {
                layout: vk::ImageLayout::PRESENT_SRC_KHR,
                stage: vk::PipelineStageFlags2::BOTTOM_OF_PIPE,
                access: vk::AccessFlags2::NONE,
            },

            _ => ImageState {
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
    prev: Option<&ImageState>,
    precursor: &ImageBarrierPrecursor,
) -> ImageBarrierDesc {
    let old_state = match prev {
        Some(p) => p,
        None => &ImageState::UNDEFINED,
    };

    ImageBarrierDesc {
        alias: precursor.access.alias,
        old_state: *old_state,
        new_state: precursor.access.usage.state,
        subresource_range: precursor.access.usage.subresource_range(),
        indexing: precursor.access.indexing,
    }
}
