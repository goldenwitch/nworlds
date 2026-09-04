#![forbid(unsafe_code)]

use caravan_demo::demo_package;
use caravan_demo::input::{Button, InputPacket};
use nworlds_desktop::DesktopApplication;
use nworlds_host::{PacketIngress, PlatformInputAdapter};
use winit::event::{ElementState, KeyEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};

#[derive(Clone, Copy, Debug, Default)]
struct CaravanInputAdapter;

impl PlatformInputAdapter<KeyEvent, InputPacket> for CaravanInputAdapter {
    fn translate(&mut self, event: KeyEvent, ingress: &mut dyn PacketIngress<InputPacket>) {
        if event.state == ElementState::Pressed
            && !event.repeat
            && event.physical_key == PhysicalKey::Code(KeyCode::Space)
        {
            ingress.push(InputPacket::ButtonPressed(Button::Primary));
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("the Caravan desktop event loop should initialize");
    let package = demo_package().expect("the initial Caravan package should be valid");
    event_loop
        .run_app(&mut DesktopApplication::new(package, CaravanInputAdapter))
        .expect("the Caravan desktop event loop should run");
}
