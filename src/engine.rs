use std::{sync::Arc, thread::JoinHandle};

use anyhow::Context;
use crossbeam_channel::unbounded;
use winit::window::Window;

use crate::{
    caps::{RenderCaps, UploadCaps},
    gameplay::gameplay_thread,
    messages::{EngineControl, ShutdownPhase},
    render::render_thread,
    upload::upload_thread,
    vulkan::VulkanContext,
};

pub struct Engine {
    _vk: VulkanContext,
    control: Arc<EngineControl>,

    render: Option<JoinHandle<anyhow::Result<()>>>,
    upload: Option<JoinHandle<anyhow::Result<()>>>,
    gameplay: Option<JoinHandle<anyhow::Result<()>>>,
}

impl Engine {
    pub fn new(window: &Window) -> anyhow::Result<Self> {
        let vk_context = VulkanContext::new(window).context("failed to create vulkan context")?;

        let (upload_tx, upload_rx) = unbounded();
        let (render_tx, render_rx) = unbounded();
        let (complete_tx, complete_rx) = unbounded();

        let control = Arc::new(EngineControl::new());

        let device_caps = vk_context.device_caps();
        let render_caps = RenderCaps {
            device: device_caps.device.clone(),
        };

        let render = std::thread::Builder::new()
            .name("render".to_string())
            .spawn({
                let control = control.clone();
                move || render_thread(render_caps, render_rx, control)
            })
            .context("failed ot start render thread")?;

        let upload_caps = UploadCaps {
            device: device_caps.device,
        };
        let upload = std::thread::Builder::new()
            .name("upload".to_string())
            .spawn({
                let control = control.clone();
                move || upload_thread(upload_caps, upload_rx, render_tx, complete_tx, control)
            })
            .context("failed to start upload thread")?;

        let gameplay = std::thread::Builder::new()
            .name("gameplay".to_string())
            .spawn({
                let control = control.clone();
                move || gameplay_thread(upload_tx, complete_rx, control)
            })
            .context("failed to start gameplay thread")?;

        Ok(Self {
            _vk: vk_context,
            control,
            upload: Some(upload),
            gameplay: Some(gameplay),
            render: Some(render),
        })
    }

    pub fn shutdown(&mut self) -> anyhow::Result<()> {
        self.control.set_phase(ShutdownPhase::StopGameplay);
        if let Some(handle) = self.gameplay.take() {
            join_thread("gameplay", handle)?;
        }

        self.control.set_phase(ShutdownPhase::StopUpload);
        if let Some(handle) = self.upload.take() {
            join_thread("upload", handle)?;
        }

        self.control.set_phase(ShutdownPhase::StopRender);
        if let Some(handle) = self.render.take() {
            join_thread("render", handle)?;
        }

        Ok(())
    }
}

fn join_thread(
    name: &'static str,
    handle: std::thread::JoinHandle<anyhow::Result<()>>,
) -> anyhow::Result<()> {
    match handle.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e).context(format!("{name} thread failed")),
        Err(panic) => Err(anyhow::anyhow!("{name} thread panicked: {:?}", panic)),
    }
}
