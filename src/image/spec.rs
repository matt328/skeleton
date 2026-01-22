use std::fmt;

use ash::vk::{self, ImageUsageFlags};

use crate::image::manager::CompositeImageKey;

use super::ImageManager;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ImageLifetime {
    Global,
    PerFrame,
    External,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ResizePolicy {
    Swapchain,
    Fixed,
}

impl fmt::Display for ResizePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ResizePolicy::Swapchain => "Swapchain",
            ResizePolicy::Fixed => "Fixed",
        };
        f.write_str(s)
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct ImageSpec {
    pub format: vk::Format,
    pub extent: vk::Extent3D,
    pub usage: vk::ImageUsageFlags,
    pub mips: u32,
    pub layers: u32,
    pub samples: vk::SampleCountFlags,
    pub resize_policy: ResizePolicy,
    pub lifetime: ImageLifetime,
    pub initial_layout: vk::ImageLayout,
    pub debug_name: Option<String>,
}

impl Default for ImageSpec {
    fn default() -> Self {
        Self {
            format: Default::default(),
            extent: Default::default(),
            usage: Default::default(),
            mips: 1,
            layers: 1,
            samples: Default::default(),
            resize_policy: ResizePolicy::Fixed,
            lifetime: ImageLifetime::Global,
            initial_layout: Default::default(),
            debug_name: Default::default(),
        }
    }
}

impl ImageSpec {
    pub fn format(mut self, format: vk::Format) -> Self {
        self.format = format;
        self
    }

    pub fn extent(mut self, extent: vk::Extent3D) -> Self {
        self.extent = extent;
        self
    }

    pub fn usage(mut self, usage: ImageUsageFlags) -> Self {
        self.usage = usage;
        self
    }

    pub fn samples(mut self, samples: vk::SampleCountFlags) -> Self {
        self.samples = samples;
        self
    }

    pub fn resize_policy(mut self, resize_policy: ResizePolicy) -> Self {
        self.resize_policy = resize_policy;
        self
    }

    pub fn lifetime(mut self, lifetime: ImageLifetime) -> Self {
        self.lifetime = lifetime;
        self
    }

    pub fn initial_layout(mut self, layout: vk::ImageLayout) -> Self {
        self.initial_layout = layout;
        self
    }

    pub fn debug_name(mut self, debug_name: impl AsRef<str>) -> Self {
        self.debug_name = Some(debug_name.as_ref().to_owned());
        self
    }
}

impl fmt::Display for ImageSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ImageSpec(format={:?}, extent={}x{}x{}, usage={:?}, mips={}, layers={}, samples={:?}, resizePolicy={}, lifetime={:?}, initialLayout={:?}, debugName={})",
            self.format,
            self.extent.width,
            self.extent.height,
            self.extent.depth,
            self.usage,
            self.mips,
            self.layers,
            self.samples,
            self.resize_policy,
            self.lifetime,
            self.initial_layout,
            match &self.debug_name {
                Some(name) => name,
                None => "<none>",
            }
        )
    }
}

#[derive(Clone, Copy)]
pub struct ImageViewSpec {
    pub image_key: CompositeImageKey,
    pub view_type: vk::ImageViewType,
    pub format: vk::Format,
    pub aspect_mask: vk::ImageAspectFlags,
    pub base_mip_level: u32,
    pub level_count: u32,
    pub base_array_layer: u32,
    pub layer_count: u32,
    pub debug_name: Option<&'static str>,
}
impl ImageViewSpec {
    pub fn new(image_key: CompositeImageKey) -> Self {
        Self {
            image_key,
            view_type: vk::ImageViewType::TYPE_2D,
            format: vk::Format::UNDEFINED,
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
            debug_name: None,
        }
    }
    pub fn view_type(mut self, view_type: vk::ImageViewType) -> Self {
        self.view_type = view_type;
        self
    }

    pub fn format(mut self, format: vk::Format) -> Self {
        self.format = format;
        self
    }

    pub fn aspect(mut self, aspect: vk::ImageAspectFlags) -> Self {
        self.aspect_mask = aspect;
        self
    }

    pub fn mip_range(mut self, base: u32, count: u32) -> Self {
        self.base_mip_level = base;
        self.level_count = count;
        self
    }

    pub fn layers(mut self, base: u32, count: u32) -> Self {
        self.base_array_layer = base;
        self.layer_count = count;
        self
    }
}

impl ImageViewSpec {
    pub fn to_vk(
        &self,
        image_manager: &ImageManager,
        frame_index: Option<u32>,
    ) -> anyhow::Result<vk::ImageViewCreateInfo<'_>> {
        if let Some(image) = image_manager.image(self.image_key, frame_index) {
            Ok(vk::ImageViewCreateInfo::default()
                .image(image.vk_image())
                .view_type(self.view_type)
                .format(self.format)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: self.aspect_mask,
                    base_mip_level: self.base_mip_level,
                    level_count: self.level_count,
                    base_array_layer: self.base_array_layer,
                    layer_count: self.layer_count,
                }))
        } else {
            Err(anyhow::anyhow!("Failed to create ImageViewCreateInfo"))
        }
    }
}
