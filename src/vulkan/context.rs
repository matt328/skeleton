use std::sync::Arc;

use anyhow::Context;
use vk_mem::AllocatorCreateInfo;
use winit::window::Window;

use ash::vk;

use super::{
    device::create_logical_device,
    instance::create_instance,
    physical::{QueueFamiliesIndices, pick_physical_device},
    swapchain::{SwapchainProperties, create_swapchain},
};

pub struct DeviceCaps {
    pub device: Arc<ash::Device>,
}

pub struct VulkanContext {
    // Instance
    surface_instance: ash::khr::surface::Instance,
    surface_khr: vk::SurfaceKHR,
    debug_report_callback: Option<(ash::ext::debug_utils::Instance, vk::DebugUtilsMessengerEXT)>,
    instance: ash::Instance,

    // Device
    physical_device: Option<vk::PhysicalDevice>,
    queue_families_indices: QueueFamiliesIndices,
    graphics_queue: ash::vk::Queue,
    present_queue: ash::vk::Queue,
    device: Arc<ash::Device>,

    // Swapchain
    swapchain_device: ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    properties: SwapchainProperties,
    images: Vec<vk::Image>,
    semaphores: Vec<vk::Semaphore>,

    allocator: vk_mem::Allocator,
}

impl VulkanContext {
    pub fn new(_window: &Window) -> anyhow::Result<Self> {
        let (surface_instance, surface_khr, debug_report_callback, instance) =
            create_instance(_window).context("failed to create instance")?;

        let (physical_device, queue_families_indices) =
            pick_physical_device(&instance, &surface_instance, surface_khr)
                .context("failed to select a physical device")?;

        let (device, graphics_queue, present_queue) =
            create_logical_device(&instance, physical_device, queue_families_indices)
                .context("failed to create a logical device and/or queues")?;

        let (swapchain_device, swapchain, properties, images, semaphores) = create_swapchain(
            physical_device,
            &surface_instance,
            surface_khr,
            queue_families_indices,
            &instance,
            &device,
        )
        .context("failed initialzing swapchain")?;

        let aci = AllocatorCreateInfo::new(&instance, &device, physical_device);

        let allocator =
            unsafe { vk_mem::Allocator::new(aci).context("failed to create allocator")? };

        Ok(Self {
            surface_instance,
            surface_khr,
            debug_report_callback,
            instance,
            physical_device: Some(physical_device),
            queue_families_indices,
            graphics_queue,
            present_queue,
            device,
            swapchain_device,
            swapchain,
            properties,
            images,
            semaphores,
            allocator,
        })
    }

    pub fn device_caps(&self) -> DeviceCaps {
        DeviceCaps {
            device: self.device.clone(),
        }
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        log::trace!("Destroying Vulkan Context");

        self.images.clear();

        for s in self.semaphores.drain(..) {
            unsafe {
                self.device.destroy_semaphore(s, None);
            }
        }

        log::trace!("  Destroying Swapchain");
        unsafe {
            self.swapchain_device
                .destroy_swapchain(self.swapchain, None);
        }

        log::trace!("  Destroying Surface");
        unsafe {
            self.surface_instance
                .destroy_surface(self.surface_khr, None);
        }

        log::trace!("  Destroying Device");
        unsafe {
            self.device
                .device_wait_idle()
                .expect("wait_idle failed during VulkanContext Drop");
            self.device.destroy_device(None);
        }

        if let Some((debug_utils, messenger)) = &self.debug_report_callback {
            log::trace!("  Destroying debug messenger");
            unsafe {
                debug_utils.destroy_debug_utils_messenger(*messenger, None);
            }
        }

        log::trace!("  Destroying Instance");
        unsafe {
            self.instance.destroy_instance(None);
        }
        log::trace!("Vulkan Context Destroyed");
    }
}
