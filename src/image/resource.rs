use ash::vk;

use crate::image::spec::{ImageSpec, ImageViewSpec};

pub struct OwnedImageInfo {
    pub allocation: vk_mem::Allocation,
    pub _spec: ImageSpec,
}

pub struct Image {
    pub vk_image: vk::Image,
    pub owned: Option<OwnedImageInfo>,
}

pub struct OwnedImageViewInfo {
    pub _spec: ImageViewSpec,
    pub _debug_name: Option<&'static str>,
}

pub struct ImageView {
    pub vk_image_view: vk::ImageView,
    pub owned: Option<OwnedImageViewInfo>,
}
