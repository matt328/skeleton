use anyhow::Context;
use ash::vk;
use slotmap::SlotMap;
use vk_mem::Alloc;

use crate::{
    buffer::{
        keys::{BufferKey, LogicalBufferKey},
        resource::Buffer,
        spec::{BufferLifetime, BufferSpec},
    },
    vulkan::DeviceContext,
};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum CompositeBufferKey {
    Global(BufferKey),
    PerFrame(LogicalBufferKey),
}

#[derive(Default)]
pub struct BufferManager {
    buffers: SlotMap<BufferKey, Buffer>,
    logical_buffers: SlotMap<LogicalBufferKey, Vec<BufferKey>>,
}

impl BufferManager {
    #[inline]
    pub fn buffer_global(&self, key: BufferKey) -> &Buffer {
        self.buffers
            .get(key)
            .expect("buffer_global: invalid BufferKey")
    }

    #[inline]
    pub fn buffer_per_frame(&self, key: LogicalBufferKey, index: usize) -> &Buffer {
        let buffer_key = self
            .logical_buffers
            .get(key)
            .and_then(|v| v.get(index))
            .expect("invalid per-frame BufferKey");
        self.buffer_global(*buffer_key)
    }

    #[inline]
    pub fn resolve_buffer(&self, key: CompositeBufferKey, index: usize) -> &Buffer {
        match key {
            CompositeBufferKey::Global(k) => self.buffer_global(k),
            CompositeBufferKey::PerFrame(k) => self.buffer_per_frame(k, index),
        }
    }

    pub fn create_buffer(
        &mut self,
        allocator: &vk_mem::Allocator,
        device_context: &DeviceContext,
        spec: BufferSpec,
        frame_count: u32,
    ) -> anyhow::Result<CompositeBufferKey> {
        match spec.lifetime {
            BufferLifetime::Global => {
                let (vk_buffer, allocation) = with_buffer_create_info(&spec, |bci, aci| unsafe {
                    allocator.create_buffer(bci, aci)
                })
                .context("failed to create buffer")?;

                if let Some(name) = spec.debug_name.as_deref() {
                    device_context.name_object(vk_buffer, name)?;
                }

                let key = self.buffers.insert(Buffer {
                    vk_buffer,
                    allocation,
                    spec,
                });
                Ok(CompositeBufferKey::Global(key))
            }

            BufferLifetime::PerFrame => {
                let mut buffer_keys: Vec<BufferKey> = Vec::with_capacity(frame_count as usize);

                for i in 0..frame_count {
                    let spec_clone = spec.clone();

                    let (vk_buffer, allocation) =
                        with_buffer_create_info(&spec_clone, |bci, aci| unsafe {
                            allocator.create_buffer(bci, aci)
                        })
                        .context("failed to create buffer")?;

                    if let Some(name) = spec.debug_name.as_deref() {
                        device_context
                            .name_object(vk_buffer, format!("{}(Frame {:?})", name, i))?;
                    }

                    buffer_keys.push(self.buffers.insert(Buffer {
                        vk_buffer,
                        allocation,
                        spec: spec_clone,
                    }));
                }
                let logical_key = self.logical_buffers.insert(buffer_keys);
                Ok(CompositeBufferKey::PerFrame(logical_key))
            }
        }
    }

    pub fn cleanup_per_frames(&mut self, allocator: &vk_mem::Allocator) -> anyhow::Result<()> {
        for (_, buffers) in self.logical_buffers.drain() {
            for key in buffers {
                if let Some(mut buffer) = self.buffers.remove(key) {
                    unsafe {
                        allocator.destroy_buffer(buffer.vk_buffer, &mut buffer.allocation);
                    }
                }
            }
        }
        Ok(())
    }
}

fn with_buffer_create_info<R>(
    spec: &BufferSpec,
    f: impl FnOnce(&vk::BufferCreateInfo, &vk_mem::AllocationCreateInfo) -> R,
) -> R {
    let bci = vk::BufferCreateInfo::default();
    let aci = vk_mem::AllocationCreateInfo {
        usage: vk_mem::MemoryUsage::Auto,
        ..Default::default()
    };
    f(&bci, &aci)
}
