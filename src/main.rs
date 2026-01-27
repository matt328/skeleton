use anyhow::Context;

#[cfg(feature = "tracing")]
use tracing_tracy::DefaultConfig;
use winit::event_loop::{ControlFlow, EventLoop};

use crate::app::{App, AppState};

mod app;
mod caps;
mod engine;
mod gameplay;
mod image;
mod messages;
mod render;
mod upload;
mod vulkan;

#[cfg(feature = "tracing")]
use tracing::{Level, span};
#[cfg(feature = "tracing")]
use tracing_subscriber::layer::SubscriberExt;

enum AppEvent {
    EngineFailed,
}

fn main() -> anyhow::Result<()> {
    log4rs::init_file("log4rs.yml", Default::default())
        .context("failed to load logging config file")?;

    #[cfg(feature = "tracing")]
    log::info!("Tracing enabled");

    #[cfg(feature = "tracing")]
    #[global_allocator]
    static GLOBAL: tracy_client::ProfiledAllocator<std::alloc::System> =
        tracy_client::ProfiledAllocator::new(std::alloc::System, 100);

    #[cfg(feature = "tracing")]
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(tracing_tracy::TracyLayer::new(DefaultConfig::default())),
    )
    .expect("setting up tracing");

    #[cfg(feature = "tracing")]
    let _root = span!(Level::INFO, "root").entered();

    let event_loop = EventLoop::<AppEvent>::with_user_event()
        .build()
        .context("failed to create event loop")?;

    event_loop.set_control_flow(ControlFlow::Poll);

    let proxy = event_loop.create_proxy();

    let mut application = App::new(proxy);

    event_loop
        .run_app(&mut application)
        .context("failed to run application")?;

    if let AppState::FatalError(e) = &application.app_state {
        log::error!("{:?}", e);
    }

    Ok(())
}
