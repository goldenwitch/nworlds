use nworlds_host::{PacketIngress, PlatformInputAdapter};
use winit::event::KeyEvent;

#[derive(Clone, Copy, Debug, Default)]
pub struct NoopInputAdapter;

impl PlatformInputAdapter<KeyEvent, ()> for NoopInputAdapter {
    fn translate(&mut self, _event: KeyEvent, _ingress: &mut dyn PacketIngress<()>) {}
}
