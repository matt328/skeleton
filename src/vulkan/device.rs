use std::{ffi::CStr, sync::Arc};

use anyhow::Context;
use ash::vk;

use super::physical::QueueFamiliesIndices;

fn get_required_device_extensions() -> [&'static CStr; 1] {
    [ash::khr::swapchain::NAME]
}

pub fn create_logical_device(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    queue_families_indices: QueueFamiliesIndices,
) -> anyhow::Result<(Arc<ash::Device>, ash::vk::Queue, ash::vk::Queue)> {
    let graphics_family_index = queue_families_indices.graphics_index;
    let present_family_index = queue_families_indices.present_index;
    let queue_priorities = [1.0f32];

    let queue_create_infos = {
        let mut indices = vec![graphics_family_index, present_family_index];
        indices.dedup();

        indices
            .iter()
            .map(|index| {
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(*index)
                    .queue_priorities(&queue_priorities)
            })
            .collect::<Vec<_>>()
    };

    let device_extensions = get_required_device_extensions();
    let device_extensions_ptrs = device_extensions
        .iter()
        .map(|ext| ext.as_ptr())
        .collect::<Vec<_>>();

    let device_features = vk::PhysicalDeviceFeatures::default().sampler_anisotropy(true);

    let mut features13 = vk::PhysicalDeviceVulkan13Features::default().synchronization2(true);

    let device_create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_create_infos)
        .enabled_extension_names(&device_extensions_ptrs)
        .enabled_features(&device_features)
        .push_next(&mut features13);

    let device = Arc::new(unsafe {
        instance
            .create_device(physical_device, &device_create_info, None)
            .context("failed to create logical device.")?
    });
    let graphics_queue = unsafe { device.get_device_queue(graphics_family_index, 0) };
    let present_queue = unsafe { device.get_device_queue(present_family_index, 0) };

    log::trace!("Created logical device");

    Ok((device, graphics_queue, present_queue))
}
