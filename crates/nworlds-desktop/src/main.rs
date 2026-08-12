#![forbid(unsafe_code)]

use std::sync::Arc;

use caravan_demo::input::InputPacket;
use caravan_demo::CaravanPackage;
use input::DesktopInputAdapter;
use nworlds_host::{ApplicationHost, MemoryInputIngress, MemoryStorage, PlatformInputAdapter};
use wgpu::WgpuRenderSink;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

mod input;
mod wgpu;

type NativeHost =
    ApplicationHost<CaravanPackage, MemoryInputIngress<InputPacket>, MemoryStorage, WgpuRenderSink>;

#[derive(Default)]
struct NativeApplication {
    window: Option<Arc<Window>>,
    host: Option<NativeHost>,
    input: DesktopInputAdapter,
}

impl ApplicationHandler for NativeApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title("Caravan of Seasons")
                .with_inner_size(winit::dpi::PhysicalSize::new(960, 720)),
        ) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("could not create the desktop window: {error}");
                event_loop.exit();
                return;
            }
        };
        let sink = match pollster::block_on(WgpuRenderSink::new(window.clone())) {
            Ok(sink) => sink,
            Err(error) => {
                eprintln!("could not initialize desktop wgpu rendering: {error}");
                event_loop.exit();
                return;
            }
        };
        self.host = Some(initial_host(sink));
        self.window = Some(window.clone());
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && !event.repeat
                    && event.physical_key == PhysicalKey::Code(KeyCode::Space) =>
            {
                if let Some(host) = &mut self.host {
                    self.input.translate(event, host.input_mut());
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(host) = &mut self.host {
                    if let Err(error) = host.step() {
                        eprintln!("desktop host step failed: {error:?}");
                        event_loop.exit();
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn initial_host(sink: WgpuRenderSink) -> NativeHost {
    ApplicationHost::new(
        caravan_demo::demo_package().expect("the initial Caravan package should be valid"),
        MemoryInputIngress::<InputPacket>::new(),
        MemoryStorage::new(),
        sink,
    )
}

fn main() {
    let event_loop = EventLoop::new().expect("the desktop event loop should initialize");
    event_loop
        .run_app(&mut NativeApplication::default())
        .expect("the desktop event loop should run");
}
