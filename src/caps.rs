use std::sync::Arc;

use ash::vk;

pub struct RenderCaps {
    // Device is thread-safe, so this Arc here is fine.
    pub device: Arc<ash::Device>,
    pub instance: Arc<ash::Instance>,
    pub physical_device: Arc<ash::vk::PhysicalDevice>,
    pub queue: vk::Queue,
    pub present_queue: vk::Queue,
}

pub struct UploadCaps {
    pub device: Arc<ash::Device>,
}
