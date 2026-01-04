use std::ffi::CStr;

use anyhow::Context;
use ash::{khr::surface, vk};

fn get_required_device_extensions() -> [&'static CStr; 1] {
    [ash::khr::swapchain::NAME]
}

#[derive(Clone, Copy)]
pub struct QueueFamiliesIndices {
    pub graphics_index: u32,
    pub present_index: u32,
}

pub fn pick_physical_device(
    instance: &ash::Instance,
    surface: &ash::khr::surface::Instance,
    surface_khr: vk::SurfaceKHR,
) -> anyhow::Result<(vk::PhysicalDevice, QueueFamiliesIndices)> {
    let devices = unsafe {
        instance
            .enumerate_physical_devices()
            .context("failed to enumerate physical devices")?
    };
    let device = devices
        .into_iter()
        .find(|device| is_device_suitable(instance, surface, surface_khr, *device))
        .context("No suitable physical device.")?;

    let props = unsafe { instance.get_physical_device_properties(device) };
    log::debug!("Selected physical device: {:?}", unsafe {
        CStr::from_ptr(props.device_name.as_ptr())
    });

    let (maybe_graphics, maybe_present) =
        find_queue_families(instance, surface, surface_khr, device);

    let (graphics, present) = (
        maybe_graphics.ok_or_else(|| anyhow::anyhow!("missing graphics queue family"))?,
        maybe_present.ok_or_else(|| anyhow::anyhow!("missing present queue family"))?,
    );

    let queue_families_indices = QueueFamiliesIndices {
        graphics_index: graphics,
        present_index: present,
    };

    Ok((device, queue_families_indices))
}

fn is_device_suitable(
    instance: &ash::Instance,
    surface: &surface::Instance,
    surface_khr: vk::SurfaceKHR,
    device: vk::PhysicalDevice,
) -> bool {
    let (graphics, present) = find_queue_families(instance, surface, surface_khr, device);
    let extention_support = check_device_extension_support(instance, device);
    let is_swapchain_adequate =
        match super::swapchain::SwapchainSupportDetails::new(device, surface, surface_khr) {
            Ok(details) => !details.formats.is_empty() && !details.present_modes.is_empty(),
            Err(_) => {
                log::warn!("failed to query swapchain support details");
                false
            }
        };

    let features = unsafe { instance.get_physical_device_features(device) };
    graphics.is_some()
        && present.is_some()
        && extention_support
        && is_swapchain_adequate
        && features.sampler_anisotropy == vk::TRUE
}

fn find_queue_families(
    instance: &ash::Instance,
    surface: &surface::Instance,
    surface_khr: vk::SurfaceKHR,
    device: vk::PhysicalDevice,
) -> (Option<u32>, Option<u32>) {
    let mut graphics = None;
    let mut present = None;

    let props = unsafe { instance.get_physical_device_queue_family_properties(device) };
    for (index, family) in props.iter().filter(|f| f.queue_count > 0).enumerate() {
        let index = index as u32;

        if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) && graphics.is_none() {
            graphics = Some(index);
        }

        let present_support =
            unsafe { surface.get_physical_device_surface_support(device, index, surface_khr) };

        match present_support {
            Ok(true) if present.is_none() => {
                present = Some(index);
            }
            Ok(_) => {}
            Err(e) => {
                log::warn!("failed to uery present support for queue family {index}: {e}");
            }
        }

        if graphics.is_some() && present.is_some() {
            break;
        }
    }

    (graphics, present)
}

fn check_device_extension_support(instance: &ash::Instance, device: vk::PhysicalDevice) -> bool {
    let required_extensions = get_required_device_extensions();

    let extension_props = match unsafe { instance.enumerate_device_extension_properties(device) } {
        Ok(props) => props,
        Err(e) => {
            log::warn!("Failed to enumerate device extension properties: {e}");
            return false;
        }
    };

    for required in required_extensions.iter() {
        let found = extension_props.iter().any(|ext| {
            let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
            required == &name
        });

        if !found {
            log::warn!(
                "Required device extension not supported: {}",
                required.to_string_lossy()
            );
            return false;
        }
    }

    true
}
