use std::sync::{Arc, mpsc};
use std::thread;

use anyhow::Context;
use crossbeam_channel::unbounded;
use winit::window::Window;

use crate::caps::{RenderCaps, UploadCaps};
use crate::gameplay::gameplay_thread;
use crate::messages::{EngineControl, ShutdownPhase};
use crate::render::render_thread;
use crate::upload::upload_thread;
use crate::vulkan::VulkanContext;

pub struct Engine {
    _vk: VulkanContext,
    control: Arc<EngineControl>,
    upload: Option<thread::JoinHandle<()>>,
    render: Option<thread::JoinHandle<()>>,
    gameplay: Option<thread::JoinHandle<()>>,
}

impl Engine {
    pub fn new(window: &Window) -> anyhow::Result<Self> {
        let vk_context = VulkanContext::new(window).context("failed to create Vulkan context")?;

        let (upload_tx, upload_rx) = unbounded();
        let (render_tx, render_rx) = unbounded();
        let (complete_tx, complete_rx) = unbounded();

        let control = Arc::new(EngineControl::new());

        let device_caps = vk_context.device_caps();
        let render_caps = RenderCaps {
            device: device_caps.device.clone(),
            instance: vk_context.swapchain_caps().instance,
            physical_device: Arc::new(vk_context.swapchain_caps().physical_device),
            queue: device_caps.queue,
            present_queue: device_caps.present_queue,
        };
        let swapchain_create_caps = vk_context.swapchain_caps();
        let upload_caps = UploadCaps {
            device: device_caps.device.clone(),
        };

        let (error_tx, error_rx) = mpsc::channel::<(String, anyhow::Error)>();

        let render_handle = {
            let control = control.clone();
            let error_tx = error_tx.clone();
            thread::Builder::new()
                .name("render".to_string())
                .spawn(move || {
                    if let Err(e) = render_thread(render_caps, control, swapchain_create_caps) {
                        let _ = error_tx.send(("render".to_string(), e));
                    }
                })?
        };

        let upload_handle = {
            let control = control.clone();
            let error_tx = error_tx.clone();
            thread::Builder::new()
                .name("upload".to_string())
                .spawn(move || {
                    if let Err(e) =
                        upload_thread(upload_caps, upload_rx, render_tx, complete_tx, control)
                    {
                        let _ = error_tx.send(("upload".to_string(), e));
                    }
                })?
        };

        let gameplay_handle = {
            let control = control.clone();
            let error_tx = error_tx.clone();
            thread::Builder::new()
                .name("gameplay".to_string())
                .spawn(move || {
                    if let Err(e) = gameplay_thread(upload_tx, complete_rx, control) {
                        let _ = error_tx.send(("gameplay".to_string(), e));
                    }
                })?
        };

        let _watchdog = {
            thread::Builder::new()
                .name("thread_watchdog".to_string())
                .spawn(move || {
                    for (name, e) in error_rx {
                        log::error!("Thread {} failed: {:?}", name, e);
                    }
                })?
        };

        Ok(Self {
            _vk: vk_context,
            control,
            render: Some(render_handle),
            upload: Some(upload_handle),
            gameplay: Some(gameplay_handle),
        })
    }

    pub fn shutdown(&mut self) -> anyhow::Result<()> {
        self.control.set_phase(ShutdownPhase::StopGameplay);
        if let Some(handle) = self.gameplay.take() {
            handle.join().ok();
        }

        self.control.set_phase(ShutdownPhase::StopUpload);
        if let Some(handle) = self.upload.take() {
            handle.join().ok();
        }

        self.control.set_phase(ShutdownPhase::StopRender);
        if let Some(handle) = self.render.take() {
            handle.join().ok();
        }

        Ok(())
    }
}
