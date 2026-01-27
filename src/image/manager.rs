use anyhow::Context;
use ash::vk;
use slotmap::SlotMap;

use vk_mem::Alloc;

use crate::image::LogicalImageKey;
use crate::image::LogicalImageViewKey;
use crate::image::resource::OwnedImageInfo;
use crate::image::resource::OwnedImageViewInfo;
use crate::image::spec::ImageLifetime;
use crate::image::spec::ImageViewTarget;
use crate::vulkan::DeviceContext;

use super::{
    ImageKey, ImageViewKey,
    resource::{Image, ImageView},
    spec::{ImageSpec, ImageViewSpec},
};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum CompositeImageKey {
    Global(ImageKey),
    PerFrame(LogicalImageKey),
}

#[derive(Copy, Clone)]
pub enum CompositeImageViewKey {
    Global(ImageViewKey),
    PerFrame(LogicalImageViewKey),
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum FrameIndex {
    Frame(u32),
    Swapchain(u32),
}

impl FrameIndex {
    pub fn raw(self) -> usize {
        match self {
            FrameIndex::Frame(i) | FrameIndex::Swapchain(i) => i as usize,
        }
    }
}

#[derive(Default)]
pub struct ImageManager {
    images: SlotMap<ImageKey, Image>,
    image_views: SlotMap<ImageViewKey, ImageView>,

    logical_images: SlotMap<LogicalImageKey, Vec<ImageKey>>,
    logical_image_views: SlotMap<LogicalImageViewKey, Vec<ImageViewKey>>,
}

impl ImageManager {
    #[inline]
    pub fn image_global(&self, key: ImageKey) -> &Image {
        self.images
            .get(key)
            .expect("image_global: invalid ImageKey")
    }

    #[inline]
    pub fn image_view_global(&self, key: ImageViewKey) -> &ImageView {
        self.image_views.get(key).expect("invalid ImageViewKey")
    }

    #[inline]
    pub fn image_per_frame(&self, key: LogicalImageKey, frame: FrameIndex) -> &Image {
        let index = frame.raw();
        let image_key = self
            .logical_images
            .get(key)
            .and_then(|v| v.get(index))
            .expect("invalid per-frame ImageKey");
        self.image_global(*image_key)
    }

    #[inline]
    pub fn image_view_per_frame(&self, key: LogicalImageViewKey, frame: FrameIndex) -> &ImageView {
        let index = frame.raw();
        let view_key = self
            .logical_image_views
            .get(key)
            .and_then(|v| v.get(index))
            .expect("invalid per-frame ImageViewKey");
        self.image_view_global(*view_key)
    }

    #[inline]
    pub fn resolve_image(&self, key: CompositeImageKey, frame: FrameIndex) -> &Image {
        match key {
            CompositeImageKey::Global(k) => self.image_global(k),
            CompositeImageKey::PerFrame(k) => self.image_per_frame(k, frame),
        }
    }

    #[inline]
    pub fn resolve_image_view(&self, key: CompositeImageViewKey, frame: FrameIndex) -> &ImageView {
        match key {
            CompositeImageViewKey::Global(k) => self.image_view_global(k),
            CompositeImageViewKey::PerFrame(k) => self.image_view_per_frame(k, frame),
        }
    }

    pub fn create_image(
        &mut self,
        allocator: &vk_mem::Allocator,
        device_context: &DeviceContext,
        spec: ImageSpec,
        frame_count: u32,
    ) -> anyhow::Result<CompositeImageKey> {
        match spec.lifetime {
            ImageLifetime::Global => {
                let (vk_image, allocation) = with_image_create_info(&spec, |ici, aci| unsafe {
                    allocator.create_image(ici, aci)
                })
                .context("failed to create image")?;

                if let Some(debug_name) = spec.debug_name.as_deref() {
                    device_context.name_object(vk_image, debug_name)?;
                }

                let key = self.images.insert(Image {
                    vk_image,
                    owned: Some(OwnedImageInfo {
                        allocation,
                        _spec: spec,
                    }),
                });

                Ok(CompositeImageKey::Global(key))
            }

            ImageLifetime::PerFrame => {
                let mut image_keys: Vec<ImageKey> = Vec::with_capacity(frame_count as usize);

                for i in 0..frame_count {
                    let spec_clone = spec.clone();

                    let (vk_image, allocation) =
                        with_image_create_info(&spec_clone, |ici, aci| unsafe {
                            allocator.create_image(ici, aci)
                        })
                        .context("failed to create image")?;

                    if let Some(debug_name) = spec_clone.debug_name.as_deref() {
                        device_context
                            .name_object(vk_image, format!("{}(Frame {:?})", debug_name, i))?;
                    }

                    image_keys.push(self.images.insert(Image {
                        vk_image,
                        owned: Some(OwnedImageInfo {
                            allocation,
                            _spec: spec_clone,
                        }),
                    }));
                }
                let logical_key = self.logical_images.insert(image_keys);
                Ok(CompositeImageKey::PerFrame(logical_key))
            }
        }
    }

    pub fn register_external_per_frame(
        &mut self,
        images: &[vk::Image],
        views: &[vk::ImageView],
    ) -> (CompositeImageKey, CompositeImageViewKey) {
        assert_eq!(images.len(), views.len());

        let image_keys = images
            .iter()
            .map(|&img| {
                self.images.insert(Image {
                    vk_image: img,
                    owned: None,
                })
            })
            .collect();

        let view_keys = views
            .iter()
            .map(|&view| {
                self.image_views.insert(ImageView {
                    vk_image_view: view,
                    owned: None,
                })
            })
            .collect();

        let image_logical = self.logical_images.insert(image_keys);
        let view_logical = self.logical_image_views.insert(view_keys);

        (
            CompositeImageKey::PerFrame(image_logical),
            CompositeImageViewKey::PerFrame(view_logical),
        )
    }

    pub fn create_image_view(
        &mut self,
        device: &ash::Device,
        spec: ImageViewSpec,
        frame_count: u32,
    ) -> anyhow::Result<CompositeImageViewKey> {
        match spec.target {
            ImageViewTarget::Global(image_key) => {
                let image = self.image_global(image_key);

                let info = spec.to_vk(image.vk_image);
                let vk_image_view = unsafe {
                    device
                        .create_image_view(&info, None)
                        .context("failed to create ImageView")?
                };

                let key = self.image_views.insert(ImageView {
                    vk_image_view,
                    owned: Some(OwnedImageViewInfo {
                        _spec: spec,
                        _debug_name: None,
                    }),
                });
                Ok(CompositeImageViewKey::Global(key))
            }

            ImageViewTarget::PerFrame(logical_key) => {
                let mut keys = Vec::with_capacity(frame_count as usize);

                for frame in 0..frame_count {
                    let image = self.image_per_frame(logical_key, FrameIndex::Frame(frame));

                    let info = spec.to_vk(image.vk_image);
                    let vk_image_view = unsafe {
                        device
                            .create_image_view(&info, None)
                            .context("failed to create ImageView")?
                    };

                    let key = self.image_views.insert(ImageView {
                        vk_image_view,
                        owned: Some(OwnedImageViewInfo {
                            _spec: spec,
                            _debug_name: None,
                        }),
                    });

                    keys.push(key);
                }

                let logical_view = self.logical_image_views.insert(keys);
                Ok(CompositeImageViewKey::PerFrame(logical_view))
            }
        }
    }

    pub fn cleanup_per_frames(
        &mut self,
        device: &ash::Device,
        allocator: &vk_mem::Allocator,
    ) -> anyhow::Result<()> {
        for (_, views) in self.logical_image_views.drain() {
            for key in views {
                if let Some(view) = self.image_views.remove(key)
                    && view.owned.is_some()
                {
                    unsafe { device.destroy_image_view(view.vk_image_view, None) }
                }
            }
        }

        for (_, images) in self.logical_images.drain() {
            for key in images {
                if let Some(image) = self.images.remove(key)
                    && let Some(mut owned) = image.owned
                {
                    unsafe {
                        allocator.destroy_image(image.vk_image, &mut owned.allocation);
                    }
                }
            }
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
