#![forbid(unsafe_code)]

use std::sync::Arc;

use caravan_demo::host::application::ApplicationHost;
use caravan_demo::host::input::{InputIngress, MemoryInputIngress};
use caravan_demo::host::storage::MemoryStorage;
use caravan_demo::host::wgpu::WgpuRenderSink;
use caravan_demo::{CaravanInteraction, CaravanOrchestrator, CaravanStage};
use caravan_domain::GameJournalEntry;
use caravan_reference::actual;
use engine_journal::JournalWriter;
use engine_time::{LogicalTime, Tau};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

type NativeHost = ApplicationHost<MemoryInputIngress, MemoryStorage, WgpuRenderSink>;

#[derive(Default)]
struct NativeApplication {
    window: Option<Arc<Window>>,
    host: Option<NativeHost>,
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
                eprintln!("could not create the Caravan window: {error}");
                event_loop.exit();
                return;
            }
        };
        let sink = match pollster::block_on(WgpuRenderSink::new(window.clone())) {
            Ok(sink) => sink,
            Err(error) => {
                eprintln!("could not initialize Caravan wgpu rendering: {error}");
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
                    host.input_mut()
                        .push(caravan_demo::input::InputPacket::ButtonPressed(
                            caravan_demo::input::Button::Primary,
                        ));
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(host) = &mut self.host {
                    if let Err(error) = host.step() {
                        eprintln!("Caravan host step failed: {error:?}");
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
    let mut writer = JournalWriter::new();
    writer.record(GameJournalEntry::create_saucer());
    let orchestrator = CaravanOrchestrator::new(
        actual(writer.finish()),
        LogicalTime::zero(),
        Tau::zero(),
        CaravanInteraction,
    )
    .expect("the initial Caravan worldline should be valid");
    ApplicationHost::new(
        CaravanStage::new(orchestrator),
        MemoryInputIngress::new(),
        MemoryStorage::new(),
        sink,
    )
}

fn main() {
    let event_loop = EventLoop::new().expect("the Windows event loop should initialize");
    event_loop
        .run_app(&mut NativeApplication::default())
        .expect("the Windows event loop should run");
}
