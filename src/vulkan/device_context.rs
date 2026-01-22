use std::{ffi::CString, sync::Arc};

use ash::vk::{self, DebugUtilsObjectNameInfoEXT};

#[derive(Clone)]
pub struct DeviceContext {
    pub device: Arc<ash::Device>,
    pub debug_instance: Option<Arc<ash::ext::debug_utils::Instance>>,
    pub debug_utils: Option<Arc<ash::ext::debug_utils::Device>>,
}

impl DeviceContext {
    pub fn name_pipeline_layout(
        &self,
        layout: &vk::PipelineLayout,
        debug_name: &str,
    ) -> anyhow::Result<()> {
        if let Some(d) = &self.debug_utils {
            let cname = CString::new(debug_name).expect("debug name contains interior null byte");
            let name_info = DebugUtilsObjectNameInfoEXT::default()
                .object_handle(*layout)
                .object_name(&cname);
            unsafe {
                d.set_debug_utils_object_name(&name_info)
                    .map_err(|e| anyhow::anyhow!("failed to set debug name: {:?}", e))
            }
        } else {
            Ok(())
        }
    }

    pub fn name_object<T>(&self, handle: T, debug_name: impl AsRef<str>) -> anyhow::Result<()>
    where
        T: vk::Handle,
    {
        let Some(debug) = &self.debug_utils else {
            return Ok(());
        };

        let cname =
            CString::new(debug_name.as_ref()).expect("debug name contains interior null byte");

        let name_info = vk::DebugUtilsObjectNameInfoEXT::default()
            .object_handle(handle)
            .object_name(&cname);

        unsafe {
            debug
                .set_debug_utils_object_name(&name_info)
                .map_err(|e| anyhow::anyhow!("failed to set debug name: {:?}", e))
        }
    }
}
