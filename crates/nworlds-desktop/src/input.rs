use nworlds_host::PacketIngress;
use winit::event::WindowEvent;

#[derive(Clone, Copy, Debug, Default)]
pub struct NoopInputAdapter;

/// Translates target-native window events into one package-defined packet type.
pub trait DesktopInputAdapter {
    type Packet: 'static;

    fn translate(&mut self, event: &WindowEvent, ingress: &mut dyn PacketIngress<Self::Packet>);
}

impl DesktopInputAdapter for NoopInputAdapter {
    type Packet = ();

    fn translate(&mut self, _event: &WindowEvent, _ingress: &mut dyn PacketIngress<()>) {}
}
