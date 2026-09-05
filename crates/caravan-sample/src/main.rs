#![forbid(unsafe_code)]

use caravan_sample::input::{Button, InputPacket};
use caravan_sample::sample_package;
use nworlds_desktop::{DesktopApplication, DesktopInputAdapter};
use nworlds_host::PacketIngress;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};

#[derive(Clone, Copy, Debug, Default)]
struct CaravanInputAdapter;

impl DesktopInputAdapter for CaravanInputAdapter {
    type Packet = InputPacket;

    fn translate(&mut self, event: &WindowEvent, ingress: &mut dyn PacketIngress<InputPacket>) {
        if let WindowEvent::KeyboardInput { event, .. } = event {
            if event.state == ElementState::Pressed
                && !event.repeat
                && event.physical_key == PhysicalKey::Code(KeyCode::Space)
            {
                ingress.push(InputPacket::ButtonPressed(Button::Primary));
            }
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("the Caravan desktop event loop should initialize");
    let package = sample_package().expect("the initial Caravan package should be valid");
    event_loop
        .run_app(&mut DesktopApplication::new(package, CaravanInputAdapter))
        .expect("the Caravan desktop event loop should run");
}
