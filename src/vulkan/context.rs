use std::sync::Arc;

use anyhow::Context;
use winit::window::Window;

use ash::vk;

use crate::vulkan::DeviceContext;

use super::{
    device::create_logical_device,
    instance::create_instance,
    physical::{QueueFamiliesIndices, pick_physical_device},
};

pub struct DeviceCaps {
    pub device_context: DeviceContext,
    pub queue: vk::Queue,
    pub present_queue: vk::Queue,
}

pub struct SwapchainCreateCaps {
    pub instance: Arc<ash::Instance>,
    pub device_context: DeviceContext,
    pub surface_instance: Arc<ash::khr::surface::Instance>,
    pub physical_device: vk::PhysicalDevice,
    pub surface: vk::SurfaceKHR,
    pub queue_families: QueueFamiliesIndices,
}

pub struct VulkanContext {
    // Instance
    surface_instance: Arc<ash::khr::surface::Instance>,
    surface_khr: vk::SurfaceKHR,
    debug_report_callback: Option<vk::DebugUtilsMessengerEXT>,
    instance: Arc<ash::Instance>,

    // Device
    physical_device: vk::PhysicalDevice,
    queue_families_indices: QueueFamiliesIndices,
    graphics_queue: ash::vk::Queue,
    present_queue: ash::vk::Queue,
    device_context: DeviceContext,
}

impl VulkanContext {
    pub fn new(_window: &Window) -> anyhow::Result<Self> {
        let (surface_instance, surface_khr, debug_report_callback, debug_instance, instance) =
            create_instance(_window).context("failed to create instance")?;

        let (physical_device, queue_families_indices) =
            pick_physical_device(&instance, &surface_instance, surface_khr)
                .context("failed to select a physical device")?;

        let (device, graphics_queue, present_queue) =
            create_logical_device(&instance, physical_device, queue_families_indices)
                .context("failed to create a logical device and/or queues")?;

        let debug_utils = Arc::new(ash::ext::debug_utils::Device::new(&instance, &device));

        Ok(Self {
            surface_instance: Arc::new(surface_instance),
            surface_khr,
            debug_report_callback,
            instance: Arc::new(instance),
            physical_device,
            queue_families_indices,
            graphics_queue,
            present_queue,
            device_context: DeviceContext {
                device,
                debug_instance: debug_instance.map(Arc::new),
                debug_utils: Some(debug_utils),
            },
        })
    }

    pub fn device_caps(&self) -> DeviceCaps {
        DeviceCaps {
            device_context: self.device_context.clone(),
            queue: self.graphics_queue,
            present_queue: self.present_queue,
        }
    }

    pub fn swapchain_caps(&self) -> SwapchainCreateCaps {
        SwapchainCreateCaps {
            instance: self.instance.clone(),
            device_context: self.device_context.clone(),
            surface_instance: self.surface_instance.clone(),
            physical_device: self.physical_device,
            surface: self.surface_khr,
            queue_families: self.queue_families_indices,
        }
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        log::trace!("Destroying Vulkan Context");

        log::trace!("  Destroying Surface");
        unsafe {
            self.surface_instance
                .destroy_surface(self.surface_khr, None);
        }

        // drop(self.allocator);

        log::trace!("  Destroying Device");
        unsafe {
            self.device_context
                .device
                .device_wait_idle()
                .expect("wait_idle failed during VulkanContext Drop");
            self.device_context.device.destroy_device(None);
        }

        if let Some(messenger) = &self.debug_report_callback {
            log::trace!("  Destroying debug messenger");
            unsafe {
                if let Some(instance) = &self.device_context.debug_instance {
                    instance.destroy_debug_utils_messenger(*messenger, None);
                }
            }
        }

        log::trace!("  Destroying Instance");
        unsafe {
            self.instance.destroy_instance(None);
        }
        log::trace!("Vulkan Context Destroyed");
    }
}
