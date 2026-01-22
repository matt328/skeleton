use anyhow::Context;
use ash::vk;

use crate::vulkan::DeviceContext;

pub struct Frame {
    pub index: usize,
    pub fence: vk::Fence,
    pub image_available: vk::Semaphore,
    pub primary_cmd: vk::CommandBuffer,
    pub secondary_cmds: Vec<vk::CommandBuffer>,
    pub swapchain_image_index: u32,
}

impl Frame {
    pub fn new(
        device_context: &DeviceContext,
        pool: vk::CommandPool,
        pass_count: usize,
        index: usize,
    ) -> anyhow::Result<Self> {
        let device = &device_context.device;
        let fence = create_fence(device, true).context("failed to create fence")?;
        let image_available =
            create_semaphore(device).context("failed to create image available semaphore")?;

        let primary_cmd = allocate_primary(device_context, pool, index as u32)?;
        let secondary_cmds = allocate_secondary(device_context, pool, pass_count, index as u32)?;

        Ok(Self {
            index,
            fence,
            image_available,
            primary_cmd,
            secondary_cmds,
            swapchain_image_index: 0,
        })
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        log::trace!("Destroying Frame");
        unsafe {
            device.destroy_semaphore(self.image_available, None);
            device.destroy_fence(self.fence, None);
        }
    }

    pub fn wait(&self, device: &ash::Device) -> anyhow::Result<()> {
        unsafe {
            device
                .wait_for_fences(&[self.fence], true, u64::MAX)
                .context("failed waiting for fences")?;
        }
        Ok(())
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

fn create_semaphore(device: &ash::Device) -> anyhow::Result<vk::Semaphore> {
    unsafe {
        device
            .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            .context("failed to create semaphore")
    }
}

fn create_fence(device: &ash::Device, signaled: bool) -> anyhow::Result<vk::Fence> {
    let flags = if signaled {
        vk::FenceCreateFlags::SIGNALED
    } else {
        vk::FenceCreateFlags::empty()
    };

    let create_info = vk::FenceCreateInfo::default().flags(flags);
    unsafe {
        device
            .create_fence(&create_info, None)
            .context("failed to create fence")
    }
}

fn allocate_primary(
    device_context: &DeviceContext,
    pool: vk::CommandPool,
    index: u32,
) -> anyhow::Result<vk::CommandBuffer> {
    let mut buffers = unsafe {
        device_context.device.allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::default()
                .command_pool(pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1),
        )
    }
    .context("failed to allocate primary command buffer")?;

    let cmd = buffers
        .pop()
        .ok_or_else(|| anyhow::anyhow!("no command buffer allocated"))?;

    device_context
        .name_object(cmd, format!("PrimaryCommandBuffer(Frame {:?})", index))
        .context("failed to name primary command buffer")?;

    Ok(cmd)
}

fn allocate_secondary(
    device_context: &DeviceContext,
    pool: vk::CommandPool,
    count: usize,
    index: u32,
) -> anyhow::Result<Vec<vk::CommandBuffer>> {
    let command_buffers = {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(pool)
            .level(vk::CommandBufferLevel::SECONDARY)
            .command_buffer_count(count as u32);
        unsafe {
            device_context
                .device
                .allocate_command_buffers(&alloc_info)
                .context("failed to allocate command buffers in FrameExecutor")?
        }
    };
    for (i, cmd) in command_buffers.iter().enumerate() {
        device_context.name_object(*cmd, format!("Secondary(#{:?}, Frame {:?})", i, index))?;
    }
    Ok(command_buffers)
}
