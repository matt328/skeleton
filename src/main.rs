use anyhow::Context;

use winit::event_loop::{ControlFlow, EventLoop};

use crate::app::{App, AppState};

mod app;
mod caps;
mod device;
mod engine;
mod gameplay;
mod messages;
mod render;
mod upload;
mod vulkan;

fn main() -> anyhow::Result<()> {
    log4rs::init_file("log4rs.yml", Default::default())
        .context("failed to load logging config file")?;

    let event_loop = EventLoop::new().context("failed to create event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut application = App::default();
    event_loop
        .run_app(&mut application)
        .context("failed to run application")?;

    if let AppState::FatalError(e) = &application.app_state {
        log::error!("{:?}", e);
    }

    Ok(())
}
