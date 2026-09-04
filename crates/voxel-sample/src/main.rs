#![forbid(unsafe_code)]

use std::sync::Arc;

use nworlds_host::RenderSink;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

mod camera;
mod engine_integration;
mod render;
mod world;

use engine_integration::{
    cottage_worldline, frame, publish, state_at_zero, VoxelJournalWriter, VoxelWorldline,
};
use render::{RenderError, VoxelRenderSink};
use world::VoxelFact;

struct VoxelApplication {
    window: Option<Arc<Window>>,
    sink: Option<VoxelRenderSink>,
    worldline: VoxelWorldline,
    writer: VoxelJournalWriter,
    cursor: Option<(f64, f64)>,
}

impl Default for VoxelApplication {
    fn default() -> Self {
        let (worldline, writer) = cottage_worldline();
        Self {
            window: None,
            sink: None,
            worldline,
            writer,
            cursor: None,
        }
    }
}

impl VoxelApplication {
    fn publish(&mut self, fact: VoxelFact) {
        self.worldline = publish(&self.worldline, &mut self.writer, fact);
        self.update_title();
    }

    fn update_title(&self) {
        let (Some(window),) = (&self.window,) else {
            return;
        };
        let sampled = state_at_zero(&self.worldline);
        window.set_title(&format!(
            "Voxel Cottage | {} blocks | scale {:.3} | click to remove | wheel to scale",
            sampled.payload().voxels().len(),
            sampled.payload().scale().as_f32(),
        ));
    }

    fn draw(&mut self) {
        let Some(sink) = &mut self.sink else {
            return;
        };
        let sampled = state_at_zero(&self.worldline);
        sink.submit(frame(&sampled));
    }

    fn remove_at_cursor(&mut self) {
        let (Some(sink), Some(cursor)) = (&self.sink, self.cursor) else {
            return;
        };
        let sampled = state_at_zero(&self.worldline);
        if let Some(position) = sink.pick(cursor, sampled.payload()) {
            self.publish(VoxelFact::Remove { position });
        }
    }

    fn adjust_scale(&mut self, delta: i32) {
        let sampled = state_at_zero(&self.worldline);
        let current = sampled.payload().scale();
        let next = current.saturating_add_milli(delta);
        if next != current {
            self.publish(VoxelFact::SetScale { scale: next });
        }
    }
}

impl ApplicationHandler for VoxelApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title("Voxel Cottage")
                .with_inner_size(winit::dpi::PhysicalSize::new(960, 720)),
        ) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("could not create the voxel window: {error}");
                event_loop.exit();
                return;
            }
        };
        let sink = match pollster::block_on(VoxelRenderSink::new(window.clone())) {
            Ok(sink) => sink,
            Err(error) => {
                report_render_error(error);
                event_loop.exit();
                return;
            }
        };
        self.window = Some(window.clone());
        self.sink = Some(sink);
        self.update_title();
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
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = Some((position.x, position.y));
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => self.remove_at_cursor(),
            WindowEvent::MouseWheel { delta, .. } => {
                let milli_delta = match delta {
                    MouseScrollDelta::LineDelta(_, lines) => (lines * 60.0).round() as i32,
                    MouseScrollDelta::PixelDelta(delta) => (delta.y as f32 / 4.0).round() as i32,
                };
                self.adjust_scale(milli_delta);
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && event.physical_key
                        == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape) =>
            {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => self.draw(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn report_render_error(error: RenderError) {
    eprintln!("could not initialize voxel rendering: {error}");
}

fn main() {
    let event_loop = EventLoop::new().expect("the voxel event loop should initialize");
    event_loop
        .run_app(&mut VoxelApplication::default())
        .expect("the voxel event loop should run");
}
