use anyhow::Context;
use ash::vk;

use crate::vulkan::physical::QueueFamiliesIndices;

#[derive(Clone, Copy, Debug)]
pub struct SwapchainProperties {
    pub format: vk::SurfaceFormatKHR,
    pub present_mode: vk::PresentModeKHR,
    pub extent: vk::Extent2D,
}

pub struct SwapchainSupportDetails {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapchainSupportDetails {
    pub fn new(
        device: vk::PhysicalDevice,
        surface: &ash::khr::surface::Instance,
        surface_khr: vk::SurfaceKHR,
    ) -> anyhow::Result<Self> {
        let capabilities = unsafe {
            surface
                .get_physical_device_surface_capabilities(device, surface_khr)
                .context("failed to get physical device surface capabilities")?
        };

        let formats = unsafe {
            surface
                .get_physical_device_surface_formats(device, surface_khr)
                .context("failed to get physical device surface formats")?
        };

        let present_modes = unsafe {
            surface
                .get_physical_device_surface_present_modes(device, surface_khr)
                .context("failed to get physical device surface present modes")?
        };

        Ok(Self {
            capabilities,
            formats,
            present_modes,
        })
    }

    pub fn get_ideal_swapchain_properties(
        &self,
        preferred_dimensions: [u32; 2],
    ) -> SwapchainProperties {
        let format = Self::choose_swapchain_surface_format(&self.formats);
        let present_mode = Self::choose_swapchain_surface_present_mode(&self.present_modes);
        let extent = Self::choose_swapchain_extent(self.capabilities, preferred_dimensions);
        SwapchainProperties {
            format,
            present_mode,
            extent,
        }
    }

    fn choose_swapchain_surface_format(
        available_formats: &[vk::SurfaceFormatKHR],
    ) -> vk::SurfaceFormatKHR {
        if available_formats.len() == 1 && available_formats[0].format == vk::Format::UNDEFINED {
            return vk::SurfaceFormatKHR {
                format: vk::Format::B8G8R8A8_UNORM,
                color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
            };
        }
        debug_assert!(
            !available_formats.is_empty(),
            "Surface formats list must not be empty"
        );

        *available_formats
            .iter()
            .find(|format| {
                format.format == vk::Format::B8G8R8A8_UNORM
                    && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .unwrap_or(&available_formats[0])
    }

    fn choose_swapchain_surface_present_mode(
        available_present_modes: &[vk::PresentModeKHR],
    ) -> vk::PresentModeKHR {
        if available_present_modes.contains(&vk::PresentModeKHR::MAILBOX) {
            vk::PresentModeKHR::MAILBOX
        } else if available_present_modes.contains(&vk::PresentModeKHR::FIFO) {
            vk::PresentModeKHR::FIFO
        } else {
            vk::PresentModeKHR::IMMEDIATE
        }
    }

    fn choose_swapchain_extent(
        capabilities: vk::SurfaceCapabilitiesKHR,
        preferred_dimensions: [u32; 2],
    ) -> vk::Extent2D {
        if capabilities.current_extent.width != u32::MAX {
            return capabilities.current_extent;
        }

        let min = capabilities.min_image_extent;
        let max = capabilities.max_image_extent;
        let width = preferred_dimensions[0].min(max.width).max(min.width);
        let height = preferred_dimensions[1].min(max.height).max(min.height);
        vk::Extent2D { width, height }
    }
}

type SwapchainComponents = (
    ash::khr::swapchain::Device,
    vk::SwapchainKHR,
    SwapchainProperties,
    Vec<vk::Image>,
    Vec<vk::Semaphore>,
);

pub fn create_swapchain(
    physical_device: vk::PhysicalDevice,
    surface_instance: &ash::khr::surface::Instance,
    surface_khr: vk::SurfaceKHR,
    queue_families_indices: QueueFamiliesIndices,
    instance: &ash::Instance,
    device: &ash::Device,
) -> anyhow::Result<SwapchainComponents> {
    let details = SwapchainSupportDetails::new(physical_device, surface_instance, surface_khr)
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

    let graphics = queue_families_indices.graphics_index;
    let present = queue_families_indices.present_index;
    let families_indices = [graphics, present];

    let create_info = {
        let mut builder = vk::SwapchainCreateInfoKHR::default()
            .surface(surface_khr)
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

    let swapchain_device_fns = ash::khr::swapchain::Device::new(instance, device);
    let swapchain = unsafe {
        swapchain_device_fns
            .create_swapchain(&create_info, None)
            .context("failed to create swapchain")?
    };
    let images = unsafe {
        swapchain_device_fns
            .get_swapchain_images(swapchain)
            .context("failed to get swapchain images")?
    };

    let maybe_semaphores: anyhow::Result<Vec<vk::Semaphore>> = images
        .iter()
        .map(|_| unsafe {
            device
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                .context("failed to create semaphore")
        })
        .collect();

    let semaphores = maybe_semaphores?;

    Ok((
        swapchain_device_fns,
        swapchain,
        properties,
        images,
        semaphores,
    ))
}
