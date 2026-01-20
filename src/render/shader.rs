use std::collections::HashMap;

use anyhow::Context;
use ash::{util::read_spv, vk};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderId {
    ForwardVert,
    ForwardFrag,
}

pub struct ShaderManager {
    modules: HashMap<ShaderId, vk::ShaderModule>,
}

impl Default for ShaderManager {
    fn default() -> Self {
        Self {
            modules: Default::default(),
        }
    }
}

impl ShaderManager {
    pub fn load_builtin(&mut self, device: &ash::Device) -> anyhow::Result<()> {
        self.load(
            device,
            ShaderId::ForwardVert,
            include_bytes!("forward.vert.spv"),
        )
        .context("failed to load forward.vert.spv")?;

        self.load(
            device,
            ShaderId::ForwardFrag,
            include_bytes!("forward.frag.spv"),
        )
        .context("failed to load forward.frag.spv")?;

        Ok(())
    }

    pub fn load(&mut self, device: &ash::Device, id: ShaderId, spirv: &[u8]) -> anyhow::Result<()> {
        let spv = read_spv(&mut std::io::Cursor::new(spirv)).context("failed to read spirv")?;
        let module = unsafe {
            device.create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&spv), None)?
        };
        self.modules.insert(id, module);
        Ok(())
    }

    #[track_caller]
    pub fn module(&self, id: ShaderId) -> anyhow::Result<vk::ShaderModule> {
        let loc = std::panic::Location::caller();
        Ok(*self.modules.get(&id).with_context(|| {
            format!(
                "no shader module with {:?} registered ({}:{})",
                id,
                loc.file(),
                loc.line()
            )
        })?)
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        for (_, module) in self.modules.drain() {
            unsafe {
                device.destroy_shader_module(module, None);
            }
        }
    }
}
