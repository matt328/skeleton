use anyhow::Context;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};

use crate::AppEvent;
use crate::engine::Engine;

#[derive(Default)]
pub enum AppState {
    #[default]
    Running,
    FatalError(anyhow::Error),
}

pub struct App {
    window: Option<Window>,
    engine: Option<Engine>,
    pub app_state: AppState,
    proxy: EventLoopProxy<AppEvent>,
}

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

impl App {
    pub fn new(proxy: EventLoopProxy<AppEvent>) -> Self {
        Self {
            window: None,
            engine: None,
            app_state: AppState::Running,
            proxy,
        }
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title("Arbor")
                .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT)),
        ) {
            Ok(w) => w,
            Err(e) => {
                self.app_state = AppState::FatalError(e.into());
                return;
            }
        };

        let engine =
            match Engine::new(&window, self.proxy.clone()).context("failed to create engine") {
                Ok(e) => e,
                Err(e) => {
                    self.app_state = AppState::FatalError(e);
                    return;
                }
            };

        self.window = Some(window);
        self.engine = Some(engine);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                log::debug!("The close button was pressed; stopping");
                if let Some(mut e) = self.engine.take() {
                    match e.shutdown() {
                        Ok(e) => e,
                        Err(e) => {
                            self.app_state = AppState::FatalError(e);
                            return;
                        }
                    }
                }
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::EngineFailed => {
                log::error!("Engine thread failed; shutting down");

                if let Some(mut engine) = self.engine.take() {
                    let _ = engine.shutdown();
                }

                event_loop.exit();
            }
        }
    }
}
