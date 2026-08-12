use caravan_demo::input::{Button, InputPacket};
use nworlds_host::{PacketIngress, PlatformInputAdapter};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[derive(Clone, Copy, Debug, Default)]
pub struct DesktopInputAdapter;

impl PlatformInputAdapter<KeyEvent, InputPacket> for DesktopInputAdapter {
    fn translate(&mut self, event: KeyEvent, ingress: &mut dyn PacketIngress<InputPacket>) {
        if event.state == ElementState::Pressed
            && !event.repeat
            && event.physical_key == PhysicalKey::Code(KeyCode::Space)
        {
            ingress.push(InputPacket::ButtonPressed(Button::Primary));
        }
    }
}
