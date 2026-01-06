use std::ffi::{CStr, CString, c_char, c_void};

use anyhow::{Context, bail};
use ash::{Entry, ext::debug_utils, vk};

#[cfg(debug_assertions)]
pub const ENABLE_VALIDATION_LAYERS: bool = true;
#[cfg(not(debug_assertions))]
pub const ENABLE_VALIDATION_LAYERS: bool = false;

const REQUIRED_LAYERS: [&str; 1] = ["VK_LAYER_KHRONOS_validation"];

unsafe extern "system" fn vulkan_debug_callback(
    flag: vk::DebugUtilsMessageSeverityFlagsEXT,
    typ: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
) -> vk::Bool32 {
    unsafe {
        use vk::DebugUtilsMessageSeverityFlagsEXT as Flag;

        let message = CStr::from_ptr((*p_callback_data).p_message);
        match flag {
            Flag::VERBOSE => log::debug!("{:?} - {:?}", typ, message),
            Flag::INFO => log::info!("{:?} - {:?}", typ, message),
            Flag::WARNING => log::warn!("{:?} - {:?}", typ, message),
            _ => log::error!("{:?} - {:?}", typ, message),
        }
        vk::FALSE
    }
}

pub fn get_layer_names_and_pointers() -> (Vec<CString>, Vec<*const c_char>) {
    let layer_names = REQUIRED_LAYERS
        .iter()
        .filter_map(|&name| match CString::new(name) {
            Ok(cstr) => Some(cstr),
            Err(_) => {
                log::warn!("skipping invalid vulkan layer name: {name}");
                None
            }
        })
        .collect::<Vec<_>>();
    let layer_names_ptrs = layer_names
        .iter()
        .map(|name| name.as_ptr())
        .collect::<Vec<_>>();
    (layer_names, layer_names_ptrs)
}

pub fn check_validation_layer_support(entry: &Entry) -> anyhow::Result<()> {
    let supported_layers = unsafe {
        entry
            .enumerate_instance_layer_properties()
            .context("failed to enumerate Vulkan instance layer properties")?
    };
    for required in REQUIRED_LAYERS.iter() {
        let found = supported_layers.iter().any(|layer| {
            let name = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) };
            let name = name.to_str().expect("Failed to get layer name pointer");
            required == &name
        });

        if !found {
            bail!("Validation layer not supported: {}", required);
        }
    }
    Ok(())
}

pub fn setup_debug_messenger(
    entry: &Entry,
    instance: &ash::Instance,
) -> Option<(debug_utils::Instance, vk::DebugUtilsMessengerEXT)> {
    if !ENABLE_VALIDATION_LAYERS {
        return None;
    }

    let create_info = create_debug_create_info();
    let debug_utils = debug_utils::Instance::new(entry, instance);
    let debug_utils_messenger = unsafe {
        match debug_utils.create_debug_utils_messenger(&create_info, None) {
            Ok(m) => m,
            Err(e) => {
                log::warn!("failed to create debug_utils_messenger: {:?}", e);
                return None;
            }
        }
    };

    Some((debug_utils, debug_utils_messenger))
}

pub fn create_debug_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT<'static> {
    vk::DebugUtilsMessengerCreateInfoEXT::default()
        .flags(vk::DebugUtilsMessengerCreateFlagsEXT::empty())
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        )
        .pfn_user_callback(Some(vulkan_debug_callback))
}
