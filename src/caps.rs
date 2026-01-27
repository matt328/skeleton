use std::sync::Arc;

use ash::vk;

use crate::vulkan::DeviceContext;

pub struct RenderCaps {
    pub device_context: DeviceContext,
    pub instance: Arc<ash::Instance>,
    pub physical_device: Arc<ash::vk::PhysicalDevice>,
    pub queue: vk::Queue,
    pub present_queue: vk::Queue,
}

pub struct UploadCaps {
    pub _device: Arc<ash::Device>,
}
