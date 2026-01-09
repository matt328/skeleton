mod culling;
mod forward;
mod present;

use crate::render::{Frame, framegraph::alias::AliasRegistry};

pub struct PassDescription {}

pub trait RenderPass {
    fn id(&self) -> u32;
    fn execute(&self, frame: &Frame, cmd: vk::CommandBuffer) -> anyhow::Result<()>;
    fn register_aliases(&self, registry: &mut AliasRegistry) -> anyhow::Result<()>;
}

use ash::vk;
pub use culling::CullingPass;
pub use forward::ForwardPass;
pub use present::PresentPass;
