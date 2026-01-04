use std::ffi::CString;

use anyhow::Context;
use ash::{ext::debug_utils, vk};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

use super::debug::{
    ENABLE_VALIDATION_LAYERS, check_validation_layer_support, create_debug_create_info,
    get_layer_names_and_pointers, setup_debug_messenger,
};

pub fn create_instance(
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
