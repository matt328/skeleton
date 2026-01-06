use anyhow::Context;
use ash::vk;

pub struct Frame {
    pub fence: vk::Fence,
    pub image_available: vk::Semaphore,
    command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub swapchain_image_index: u32,
}

impl Frame {
    pub fn new(device: &ash::Device, queue_family: u32) -> anyhow::Result<Self> {
        let fence = create_fence(device, true).context("failed to create fence")?;
        let image_available =
            create_semaphore(device).context("failed to create image available semaphore")?;

        let command_pool = {
            let pool_info = vk::CommandPoolCreateInfo::default()
                .queue_family_index(queue_family)
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
            unsafe {
                device
                    .create_command_pool(&pool_info, None)
                    .context("failed to create command pool in FrameExecutor")?
            }
        };

        let command_buffers = {
            let alloc_info = vk::CommandBufferAllocateInfo::default()
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);
            unsafe {
                device
                    .allocate_command_buffers(&alloc_info)
                    .context("failed to allocate command buffer in FrameExecutor")?
            }
        };

        Ok(Self {
            fence,
            image_available,
            command_pool,
            command_buffers,
            swapchain_image_index: 0,
        })
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        log::trace!("Destroying Frame");
        unsafe {
            device.destroy_command_pool(self.command_pool, None);
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
