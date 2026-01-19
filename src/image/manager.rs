use anyhow::Context;
use ash::vk;
use slotmap::SlotMap;

use vk_mem::Alloc;

use crate::image::LogicalImageKey;
use crate::image::LogicalImageViewKey;
use crate::image::resource::OwnedImageInfo;
use crate::image::resource::OwnedImageViewInfo;
use crate::image::spec::ImageLifetime;

use super::{
    ImageKey, ImageViewKey,
    resource::{Image, ImageView},
    spec::{ImageSpec, ImageViewSpec},
};

pub struct ImageManager {
    images: SlotMap<ImageKey, Image>,
    image_views: SlotMap<ImageViewKey, ImageView>,
    logical_images: SlotMap<LogicalImageKey, Vec<ImageKey>>,
    logical_image_views: SlotMap<LogicalImageViewKey, Vec<ImageViewKey>>,
}

#[derive(Copy, Clone)]
pub enum CompositeImageKey {
    Global(ImageKey),
    PerFrame(LogicalImageKey),
    External(ImageKey),
}

#[derive(Copy, Clone)]
pub enum CompositeImageViewKey {
    Global(ImageViewKey),
    PerFrame(LogicalImageViewKey),
    External(ImageViewKey),
}

impl ImageManager {
    pub fn create_image(
        &mut self,
        allocator: &vk_mem::Allocator,
        spec: ImageSpec,
        frame_count: u32,
    ) -> anyhow::Result<CompositeImageKey> {
        log::trace!("Creating Image with spec: {}", spec);
        if spec.lifetime == ImageLifetime::Global {
            let (vk_image, allocation) = with_image_create_info(&spec, |ici, aci| unsafe {
                allocator.create_image(ici, aci)
            })
            .context("failed to create image")?;
            let allocation_info = allocator.get_allocation_info(&allocation);
            let key = self.images.insert(Image {
                vk_image,
                owned: Some(OwnedImageInfo {
                    allocation,
                    spec,
                    allocation_info,
                    views: smallvec::smallvec![],
                }),
            });
            Ok(CompositeImageKey::Global(key))
        } else {
            let mut image_keys: Vec<ImageKey> = Vec::with_capacity(frame_count as usize);
            for _ in 0..frame_count {
                let (vk_image, allocation) = with_image_create_info(&spec, |ici, aci| unsafe {
                    allocator.create_image(ici, aci)
                })
                .context("failed to create image")?;
                let allocation_info = allocator.get_allocation_info(&allocation);
                image_keys.push(self.images.insert(Image {
                    vk_image,
                    owned: Some(OwnedImageInfo {
                        allocation,
                        spec,
                        allocation_info,
                        views: smallvec::smallvec![],
                    }),
                }));
            }
            let logical_key = self.logical_images.insert(image_keys);
            Ok(CompositeImageKey::PerFrame(logical_key))
        }
    }

    pub fn register_external_image(
        &mut self,
        image: vk::Image,
        view: vk::ImageView,
        _format: vk::Format,
        _extent: vk::Extent3D,
    ) -> (ImageKey, ImageViewKey) {
        let image_key = self.images.insert(Image {
            vk_image: image,
            owned: None,
        });
        let image_view_key = self.image_views.insert(ImageView {
            vk_image_view: view,
            owned: None,
        });
        (image_key, image_view_key)
    }

    pub fn create_image_view(
        &mut self,
        device: &ash::Device,
        spec: ImageViewSpec,
        frame_count: u32,
    ) -> anyhow::Result<CompositeImageViewKey> {
        let image_key = spec.image_key;

        let image = self
            .image(image_key, Some(0))
            .context("Failed to resolve image for image_key")?;

        let lifetime = image
            .owned
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("failed to create ImageView using non-owned image"))?
            .spec
            .lifetime;

        match lifetime {
            ImageLifetime::Global => {
                let info = spec
                    .to_vk(self, None)
                    .context("create_image_view failed to create ImageViewCreateInfo")?;
                let vk_image_view = unsafe {
                    device
                        .create_image_view(&info, None)
                        .context("failed to create ImageView")?
                };
                let key = self.image_views.insert(ImageView {
                    vk_image_view,
                    owned: Some(OwnedImageViewInfo {
                        spec,
                        debug_name: None,
                    }),
                });
                Ok(CompositeImageViewKey::Global(key))
            }
            ImageLifetime::PerFrame => {
                let mut image_view_keys: Vec<ImageViewKey> =
                    Vec::with_capacity(frame_count as usize);
                for index in 0..frame_count {
                    let info = spec
                        .to_vk(self, Some(index))
                        .context("create_image_view failed to create ImageViewCreateInfo")?;
                    let vk_image_view = unsafe {
                        device
                            .create_image_view(&info, None)
                            .context("failed to create ImageView")?
                    };
                    let key = self.image_views.insert(ImageView {
                        vk_image_view,
                        owned: Some(OwnedImageViewInfo {
                            spec,
                            debug_name: None,
                        }),
                    });
                    image_view_keys.push(key);
                }
                let logical_view_key = self.logical_image_views.insert(image_view_keys);
                Ok(CompositeImageViewKey::PerFrame(logical_view_key))
            }
            ImageLifetime::External => {
                anyhow::bail!("Cannot create image view for external images")
            }
        }
    }

    pub fn destroy_image_view(
        &mut self,
        device: &ash::Device,
        key: ImageViewKey,
    ) -> anyhow::Result<()> {
        log::trace!("Destroying ImageView: {:?}", key);
        if let Some(image_view) = self.image_views.remove(key) {
            if let Some(_) = image_view.owned {
                unsafe { device.destroy_image_view(image_view.vk_image_view, None) }
            }
        }
        Ok(())
    }

    pub fn destroy_image(&mut self, key: ImageKey, allocator: &vk_mem::Allocator) {
        log::trace!("Destroying Image: {:?}", key);
        if let Some(image) = self.images.remove(key) {
            if let Some(mut owned) = image.owned {
                unsafe {
                    allocator.destroy_image(image.vk_image, &mut owned.allocation);
                }
            }
        }
    }

    pub fn image(&self, key: CompositeImageKey, frame_index: Option<u32>) -> Option<&Image> {
        match key {
            CompositeImageKey::Global(image_key) | CompositeImageKey::External(image_key) => {
                self.images.get(image_key)
            }
            CompositeImageKey::PerFrame(logical_image_key) => {
                let index = frame_index?;
                let image_key = self.logical_images.get(logical_image_key)?[index as usize];
                self.images.get(image_key)
            }
        }
    }

    pub fn image_view(
        &self,
        key: CompositeImageViewKey,
        frame_index: Option<u32>,
    ) -> Option<&ImageView> {
        match key {
            CompositeImageViewKey::Global(image_key)
            | CompositeImageViewKey::External(image_key) => self.image_views.get(image_key),
            CompositeImageViewKey::PerFrame(logical_image_key) => {
                let index = frame_index?;
                let image_key = self.logical_image_views.get(logical_image_key)?[index as usize];
                self.image_views.get(image_key)
            }
        }
    }

    pub fn cleanup_per_frames(
        &mut self,
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
    ) -> anyhow::Result<()> {
        let view_keys: Vec<ImageViewKey> = self
            .logical_image_views
            .drain()
            .flat_map(|(_, keys)| keys)
            .collect();

        for view_key in view_keys {
            self.destroy_image_view(device, view_key)?;
        }

        let keys: Vec<ImageKey> = self
            .logical_images
            .drain()
            .flat_map(|(_, keys)| keys)
            .collect();
        for key in keys {
            self.destroy_image(key, allocator);
        }
        Ok(())
    }
}

fn with_image_create_info<R>(
    spec: &ImageSpec,
    f: impl FnOnce(&vk::ImageCreateInfo, &vk_mem::AllocationCreateInfo) -> R,
) -> R {
    let ici = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(spec.format)
        .mip_levels(spec.mips)
        .array_layers(spec.layers)
        .extent(spec.extent)
        .samples(spec.samples)
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
            image_views: Default::default(),
            logical_images: Default::default(),
            logical_image_views: Default::default(),
        }
    }
}
