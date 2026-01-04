use std::{ffi::CString, sync::Arc};

use anyhow::Context;
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

use ash::{ext::debug_utils, vk};

use crate::vulkan::{
    physical,
    swapchain::{SwapchainProperties, create_swapchain},
};

use super::{
    device::create_logical_device,
    physical::{QueueFamiliesIndices, pick_physical_device},
};

use super::debug::{
    ENABLE_VALIDATION_LAYERS, check_validation_layer_support, create_debug_create_info,
    get_layer_names_and_pointers, setup_debug_messenger,
};

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

fn create_instance(
    window: &Window,
) -> anyhow::Result<(
    ash::khr::surface::Instance,
    vk::SurfaceKHR,
    Option<(ash::ext::debug_utils::Instance, vk::DebugUtilsMessengerEXT)>,
    ash::Instance,
)> {
    let entry = ash::Entry::linked();
    let display_handle = window
        .display_handle()
        .context("failed to acquire display handle")?;
    let window_handle = window
        .window_handle()
        .context("failed to acquire window handle")?;

    let instance = {
        let app_name = CString::new("Vulkan Application")?;
        let engine_name = CString::new("Arbor")?;

        let app_info = ash::vk::ApplicationInfo::default()
            .api_version(vk::API_VERSION_1_3)
            .application_name(app_name.as_c_str())
            .application_version(ash::vk::make_api_version(0, 0, 1, 0))
            .engine_name(engine_name.as_c_str())
            .engine_version(ash::vk::make_api_version(0, 0, 1, 0));
        let surface_extensions = {
            ash_window::enumerate_required_extensions(display_handle.as_raw())
                .context("failed to enumerate required extensions")?
        };

        let mut extension_names = surface_extensions.to_vec();
        if ENABLE_VALIDATION_LAYERS {
            extension_names.push(debug_utils::NAME.as_ptr());
        }

        let (_layer_names, layer_names_ptrs) = get_layer_names_and_pointers();

        let create_flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            vk::InstanceCreateFlags::default()
        };
        let mut debug_create_info = create_debug_create_info();
        let mut instance_create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extension_names)
            .flags(create_flags);
        if ENABLE_VALIDATION_LAYERS {
            check_validation_layer_support(&entry)
                .context("failed to check validation layer support")?;
            instance_create_info = instance_create_info
                .enabled_layer_names(&layer_names_ptrs)
                .push_next(&mut debug_create_info);
        }
        unsafe {
            entry
                .create_instance(&instance_create_info, None)
                .context("failed to create ash::Instance")?
        }
    };

    let surface_instance = ash::khr::surface::Instance::new(&entry, &instance);
    let surface_khr = unsafe {
        ash_window::create_surface(
            &entry,
            &instance,
            display_handle.as_raw(),
            window_handle.as_raw(),
            None,
        )
    }
    .context("failed to create surface")?;

    let debug_messenger = setup_debug_messenger(&entry, &instance);
    Ok((surface_instance, surface_khr, debug_messenger, instance))
}

pub struct DeviceCaps {
    pub device: Arc<ash::Device>,
}
