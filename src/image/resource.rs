use ash::vk;

use crate::image::spec::{ImageSpec, ImageViewSpec};

pub struct OwnedImageInfo {
    pub allocation: vk_mem::Allocation,
    pub spec: ImageSpec,
}

pub struct Image {
    pub vk_image: vk::Image,
    pub owned: Option<OwnedImageInfo>,
}

impl Image {
    pub fn vk_image(&self) -> vk::Image {
        self.vk_image
    }
}

pub struct OwnedImageViewInfo {
    pub _spec: ImageViewSpec,
    pub _debug_name: Option<&'static str>,
}

pub struct ImageView {
    pub vk_image_view: vk::ImageView,
    pub owned: Option<OwnedImageViewInfo>,
}
