// caps.rs

use std::sync::Arc;

pub struct RenderCaps {
    pub device: Arc<ash::Device>,
}

pub struct UploadCaps {
    pub device: Arc<ash::Device>,
}
