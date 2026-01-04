// caps.rs

use std::sync::Arc;

use crate::device::Device;

pub struct RenderCaps {
    pub device: Arc<Device>,
}

pub struct UploadCaps {
    pub device: Arc<Device>,
}
