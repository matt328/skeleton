use anyhow::Context;
use ash::vk;

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
        device: &ash::Device,
        pool: vk::CommandPool,
        pass_count: usize,
        index: usize,
    ) -> anyhow::Result<Self> {
        let fence = create_fence(device, true).context("failed to create fence")?;
        let image_available =
            create_semaphore(device).context("failed to create image available semaphore")?;

        let primary_cmd = allocate_primary(device, pool)?;
        let secondary_cmds = allocate_secondary(device, pool, pass_count)?;

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
    device: &ash::Device,
    pool: vk::CommandPool,
) -> anyhow::Result<vk::CommandBuffer> {
    let mut buffers = unsafe {
        device.allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::default()
                .command_pool(pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1),
        )
    }
    .context("failed to allocate primary command buffer")?;

    buffers
        .pop()
        .ok_or_else(|| anyhow::anyhow!("no command buffer allocated"))
}

fn allocate_secondary(
    device: &ash::Device,
    pool: vk::CommandPool,
    count: usize,
) -> anyhow::Result<Vec<vk::CommandBuffer>> {
    let command_buffers = {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(pool)
            .level(vk::CommandBufferLevel::SECONDARY)
            .command_buffer_count(count as u32);
        unsafe {
            device
                .allocate_command_buffers(&alloc_info)
                .context("failed to allocate command buffers in FrameExecutor")?
        }
    };
    Ok(command_buffers)
}
