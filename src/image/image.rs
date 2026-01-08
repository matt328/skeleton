use anyhow::Context;
use ash::vk;
use slotmap::SecondaryMap;
use slotmap::SlotMap;

use slotmap::new_key_type;
use vk_mem::Alloc;

new_key_type! {pub struct ImageKey; }

#[derive(PartialEq, Eq)]
pub enum ResizePolicy {
    Swapchain,
    Fixed,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ImageLifetime {
    Persistent,
    Transient,
    Swapchain,
}

#[derive(PartialEq, Eq)]
pub struct ImageSpec {
    pub format: vk::Format,
    pub extent: vk::Extent3D,
    pub usage: vk::ImageUsageFlags,
    pub mips: u32,
    pub layers: u32,
    pub samples: vk::SampleCountFlags,
    resize_policy: ResizePolicy,
    pub lifetime: ImageLifetime,
    pub initial_layout: vk::ImageLayout,
    debug_name: Option<String>,
}

impl Default for ImageSpec {
    fn default() -> Self {
        Self {
            format: Default::default(),
            extent: Default::default(),
            usage: Default::default(),
            mips: Default::default(),
            layers: Default::default(),
            samples: Default::default(),
            resize_policy: ResizePolicy::Fixed,
            lifetime: ImageLifetime::Persistent,
            initial_layout: Default::default(),
            debug_name: Default::default(),
        }
    }
}

struct Image {
    vk_image: vk::Image,
    allocation: vk_mem::Allocation,
    spec: ImageSpec,
    allocation_info: vk_mem::AllocationInfo,
}

pub struct ImageManager {
    images: SlotMap<ImageKey, Image>,
    image_uses: SecondaryMap<ImageKey, u32>,
}

impl ImageManager {
    pub fn create_image(
        &mut self,
        allocator: &vk_mem::Allocator,
        spec: ImageSpec,
    ) -> anyhow::Result<ImageKey> {
        let (vk_image, allocation) = with_image_create_info(&spec, |ici, aci| unsafe {
            allocator.create_image(ici, aci)
        })
        .context("failed to create image")?;
        let allocation_info = allocator.get_allocation_info(&allocation);
        let key = self.images.insert(Image {
            vk_image,
            allocation,
            spec,
            allocation_info,
        });
        Ok(key)
    }

    pub fn destroy_image(&mut self, key: ImageKey, allocator: &vk_mem::Allocator) {
        if let Some(mut image) = self.images.remove(key) {
            unsafe {
                allocator.destroy_image(image.vk_image, &mut image.allocation);
            }
        }
    }

    pub fn image(&self, key: ImageKey) -> Option<&Image> {
        self.images.get(key)
    }

    pub fn increment_image_use(&mut self, key: ImageKey) {
        match self.image_uses.get_mut(key) {
            Some(count) => *count += 1,
            None => {
                self.image_uses.insert(key, 1);
            }
        }
    }

    pub fn decrement_image_use(&mut self, key: ImageKey) {
        match self.image_uses.get_mut(key) {
            Some(count) => *count -= 1,
            None => {}
        }
    }
}

fn with_image_create_info<R>(
    spec: &ImageSpec,
    f: impl FnOnce(&vk::ImageCreateInfo, &vk_mem::AllocationCreateInfo) -> R,
) -> R {
    let ici = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(spec.format)
        .extent(spec.extent)
        .usage(spec.usage);

    let aci = vk_mem::AllocationCreateInfo {
        usage: vk_mem::MemoryUsage::Auto,
        ..Default::default()
    };

    f(&ici, &aci)
}

impl Default for ImageManager {
    fn default() -> Self {
        Self {
            images: Default::default(),
            image_uses: Default::default(),
        }
    }
}
