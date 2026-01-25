use std::sync::Arc;

use anyhow::Context;
use ash::vk::{self};

use crate::vulkan::{SurfaceSupportDetails, SwapchainCreateCaps, SwapchainProperties};

pub struct SwapchainContext {
    device: Arc<ash::Device>,
    pub swapchain_device: ash::khr::swapchain::Device,
    pub swapchain: vk::SwapchainKHR,
    _properties: SwapchainProperties,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
    pub image_semaphores: Vec<vk::Semaphore>,
    pub swapchain_format: vk::Format,
    pub swapchain_extent: vk::Extent2D,
}

impl SwapchainContext {
    pub fn new(caps: SwapchainCreateCaps) -> anyhow::Result<Self> {
        let details =
            SurfaceSupportDetails::new(caps.physical_device, &caps.surface_instance, caps.surface)
                .context("failed to create swapchain support details")?;
        let properties = details.get_ideal_swapchain_properties([800, 600]);

        let format = properties.format;
        let present_mode = properties.present_mode;
        let extent = properties.extent;
        let image_count = {
            let max = details.capabilities.max_image_count;
            let mut preferred = details.capabilities.min_image_count + 1;
            if max > 0 && preferred > max {
                preferred = max;
            }
            preferred
        };

        log::debug!(
            "Creating swapchain.\n\tFormat: {:?}\n\tColorSpace: {:?}\n\tPresentMode: {:?}\n\tExtent: {:?}\n\tImageCount: {:?}",
            format.format,
            format.color_space,
            present_mode,
            extent,
            image_count,
        );

        let graphics = caps.queue_families.graphics_index;
        let present = caps.queue_families.present_index;
        let families_indices = [graphics, present];

        let create_info = {
            let mut builder = vk::SwapchainCreateInfoKHR::default()
                .surface(caps.surface)
                .min_image_count(image_count)
                .image_format(format.format)
                .image_color_space(format.color_space)
                .image_extent(extent)
                .image_array_layers(1)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT);

            builder = if graphics != present {
                builder
                    .image_sharing_mode(vk::SharingMode::CONCURRENT)
                    .queue_family_indices(&families_indices)
            } else {
                builder.image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            };

            builder
                .pre_transform(details.capabilities.current_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(present_mode)
                .clipped(true)
        };

        let swapchain_device =
            ash::khr::swapchain::Device::new(&caps.instance, &caps.device_context.device);
        let swapchain = unsafe {
            swapchain_device
                .create_swapchain(&create_info, None)
                .context("failed to create swapchain")?
        };

        let images = unsafe {
            swapchain_device
                .get_swapchain_images(swapchain)
                .context("failed to get swapchain images")?
        };

        for (i, image) in images.iter().enumerate() {
            caps.device_context
                .name_object(*image, format!("SwapchainImage(#{:?})", i))?;
        }

        let image_views: anyhow::Result<Vec<vk::ImageView>> = images
            .iter()
            .map(|&image| {
                let view_info = vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(properties.format.format)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });

                unsafe {
                    caps.device_context
                        .device
                        .create_image_view(&view_info, None)
                        .context("failed to create swapchain image view")
                }
            })
            .collect();
        let image_views = image_views.context("failed to create swapchain image views")?;

        for (i, image_view) in image_views.iter().enumerate() {
            caps.device_context
                .name_object(*image_view, format!("SwapchainImageView(#{:?})", i))?;
        }

        let mut image_semaphores = Vec::new();
        for _ in 0..image_count {
            unsafe {
                image_semaphores.push(
                    caps.device_context
                        .device
                        .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                        .context("failed to create semaphore")?,
                );
            }
        }

        Ok(Self {
            device: caps.device_context.device.clone(),
            swapchain_device,
            swapchain,
            _properties: properties,
            images,
            image_views,
            image_semaphores,
            swapchain_format: format.format,
            swapchain_extent: extent,
        })
    }

    pub fn destroy(&mut self) {
        log::trace!("Destroying Swapchain Context");
        unsafe {
            for &sem in &self.image_semaphores {
                self.device.destroy_semaphore(sem, None);
            }
            for &image_view in &self.image_views {
                self.device.destroy_image_view(image_view, None);
            }
            self.swapchain_device
                .destroy_swapchain(self.swapchain, None);
        }

        self.image_semaphores.clear();
        self.images.clear();
        self.swapchain = vk::SwapchainKHR::null();
    }

    pub fn acquire_next_image(&mut self, semaphore: vk::Semaphore) -> anyhow::Result<(u32, bool)> {
        let _frame_span = tracy_client::span!("acquire_next_image");
        unsafe {
            self.swapchain_device
                .acquire_next_image(self.swapchain, u64::MAX, semaphore, vk::Fence::null())
                .map_err(|e| anyhow::anyhow!("acquire_next_image2 failed: {:?}", e))
        }
    }
}
