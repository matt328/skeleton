use ash::vk;

use crate::buffer::spec::BufferSpec;

pub struct Buffer {
    pub vk_buffer: vk::Buffer,
    pub allocation: vk_mem::Allocation,
    pub spec: BufferSpec,
}
