use std::sync::Arc;

use ash::vk;

pub struct RenderCaps {
    pub device: Arc<ash::Device>,
    pub queue: vk::Queue,
}

pub struct UploadCaps {
    pub device: Arc<ash::Device>,
}
