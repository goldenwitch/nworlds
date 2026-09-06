use std::marker::PhantomData;
use std::sync::Arc;

use engine_api::{Frame, RenderBatch};
use nworlds_host::{
    ApplicationHost, GamePackage, InputBatchError, MemoryInputIngress, MemoryStorage,
    OrderedInputBatch, PacketIngress,
};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::{DesktopInputAdapter, WgpuRenderSink};

type NativeHost<P, Packet> =
    ApplicationHost<P, MemoryInputIngress<Packet>, MemoryStorage, WgpuRenderSink>;

fn is_console_toggle(physical_key: PhysicalKey, state: ElementState, repeat: bool) -> bool {
    state == ElementState::Pressed
        && !repeat
        && physical_key == PhysicalKey::Code(KeyCode::Backquote)
}

fn translate_input_event<Input>(
    input: &mut Input,
    event: &WindowEvent,
    ingress: &mut dyn PacketIngress<Input::Packet>,
) where
    Input: DesktopInputAdapter,
{
    input.translate(event, ingress);
}

/// Generic native desktop lifecycle around one target-neutral package.
pub struct DesktopApplication<P, Packet, Input> {
    window: Option<Arc<Window>>,
    host: Option<NativeHost<P, Packet>>,
    package: Option<P>,
    input: Input,
    _packet: PhantomData<fn() -> Packet>,
}

impl<P, Packet, Input> DesktopApplication<P, Packet, Input>
where
    P: GamePackage<InputBatch = OrderedInputBatch<Packet>, Frame = Frame<RenderBatch>> + 'static,
    P::Error: From<InputBatchError> + std::fmt::Debug,
    Packet: 'static,
    Input: DesktopInputAdapter<Packet = Packet> + 'static,
{
    /// Creates a desktop application before native resources are available.
    pub fn new(package: P, input: Input) -> Self {
        Self {
            window: None,
            host: None,
            package: Some(package),
            input,
            _packet: PhantomData,
        }
    }

    fn start(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title("nworlds desktop host")
                .with_inner_size(winit::dpi::PhysicalSize::new(960, 720)),
        ) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("could not create the desktop window: {error}");
                event_loop.exit();
                return;
            }
        };
        let sink = match pollster::block_on(WgpuRenderSink::new(window.clone(), P::declaration())) {
            Ok(sink) => sink,
            Err(error) => {
                eprintln!("could not initialize desktop wgpu rendering: {error}");
                event_loop.exit();
                return;
            }
        };
        let package = self
            .package
            .take()
            .expect("desktop package should be available before startup");
        self.host = Some(ApplicationHost::new(
            package,
            MemoryInputIngress::new(),
            MemoryStorage::new(),
            sink,
        ));
        self.window = Some(window.clone());
        window.request_redraw();
    }

    fn translate_event(&mut self, event: &WindowEvent) {
        if let Some(host) = &mut self.host {
            translate_input_event(&mut self.input, event, host.input_mut());
        }
    }

    fn step(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(host) = &mut self.host {
            if let Err(error) = host.step() {
                eprintln!("desktop host step failed: {error:?}");
                event_loop.exit();
            }
        }
    }

    fn toggle_console(&mut self) {
        if let Some(host) = &mut self.host {
            host.render_mut().toggle_console();
        }
    }
}

impl<P, Packet, Input> ApplicationHandler for DesktopApplication<P, Packet, Input>
where
    P: GamePackage<InputBatch = OrderedInputBatch<Packet>, Frame = Frame<RenderBatch>> + 'static,
    P::Error: From<InputBatchError> + std::fmt::Debug,
    Packet: 'static,
    Input: DesktopInputAdapter<Packet = Packet> + 'static,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.start(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        self.translate_event(&event);
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if is_console_toggle(event.physical_key, event.state, event.repeat) =>
            {
                self.toggle_console();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && !event.repeat
                    && event.physical_key == PhysicalKey::Code(KeyCode::Escape) =>
            {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { .. } => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => self.step(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::translate_input_event;
    use engine_api::{Frame, RenderBatch, Tau};
    use nworlds_host::{
        ApplicationHost, CollectingRenderSink, GamePackage, InputBatchError, InputIngress,
        MemoryInputIngress, MemoryStorage, OrderedInputBatch, PackageDeclaration, PacketIngress,
    };
    use winit::event::{ElementState, WindowEvent};
    use winit::keyboard::{KeyCode, PhysicalKey};

    #[derive(Default)]
    struct RecordingInputAdapter;

    impl super::DesktopInputAdapter for RecordingInputAdapter {
        type Packet = ();

        fn translate(
            &mut self,
            _event: &WindowEvent,
            ingress: &mut dyn PacketIngress<Self::Packet>,
        ) {
            ingress.push(());
        }
    }

    struct SyntheticPackage;

    impl GamePackage for SyntheticPackage {
        type InputBatch = OrderedInputBatch<()>;
        type Frame = Frame<RenderBatch>;
        type Error = InputBatchError;
        type SaveError = core::convert::Infallible;
        type LoadError = core::convert::Infallible;

        fn declaration() -> PackageDeclaration {
            PackageDeclaration::new(
                "synthetic-desktop-host-test",
                nworlds_host::SemanticVersion::new(0, 1, 0),
                &[],
                nworlds_host::PersistenceRequirement::new(
                    "synthetic",
                    nworlds_host::SchemaVersion::new(1),
                ),
                nworlds_host::HostVersionRequirement::new(nworlds_host::SemanticVersion::new(
                    0, 1, 0,
                )),
                nworlds_host::RenderVocabularyRequirement::new(
                    "triangle-list-rgba",
                    nworlds_host::SemanticVersion::new(1, 0, 0),
                ),
            )
        }

        fn ingest_batch(&mut self, _batch: Self::InputBatch) -> Result<(), Self::Error> {
            Ok(())
        }

        fn update(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }

        fn present(&self) -> Result<Self::Frame, Self::Error> {
            Ok(Frame::new(Tau::zero(), RenderBatch::empty()))
        }

        fn save_selected(&self) -> Result<Vec<u8>, Self::SaveError> {
            Ok(Vec::new())
        }

        fn load_selected(&mut self, _bytes: &[u8]) -> Result<(), Self::LoadError> {
            Ok(())
        }
    }

    #[test]
    fn synthetic_package_submits_owned_render_batch() {
        let mut host = ApplicationHost::new(
            SyntheticPackage,
            MemoryInputIngress::<()>::new(),
            MemoryStorage::new(),
            CollectingRenderSink::<Frame<RenderBatch>>::new(),
        );

        assert!(!host.step().expect("synthetic package should step"));
        assert_eq!(host.render().frames().len(), 1);
        assert!(host
            .render()
            .last()
            .expect("synthetic frame should be collected")
            .payload()
            .is_empty());
    }

    #[test]
    fn window_event_reaches_the_package_input_adapter() {
        let mut input = RecordingInputAdapter;
        let mut ingress = MemoryInputIngress::<()>::new();

        translate_input_event(&mut input, &WindowEvent::RedrawRequested, &mut ingress);

        let batch = ingress
            .drain()
            .expect("the window event should form an input batch");
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn console_toggle_requires_a_non_repeated_backquote_press() {
        assert!(super::is_console_toggle(
            PhysicalKey::Code(KeyCode::Backquote),
            ElementState::Pressed,
            false,
        ));
        assert!(!super::is_console_toggle(
            PhysicalKey::Code(KeyCode::Backquote),
            ElementState::Pressed,
            true,
        ));
        assert!(!super::is_console_toggle(
            PhysicalKey::Code(KeyCode::Backquote),
            ElementState::Released,
            false,
        ));
        assert!(!super::is_console_toggle(
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed,
            false,
        ));
    }
}
