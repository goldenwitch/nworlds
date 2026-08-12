use crate::input::InputPacket;

pub use nworlds_host::{InputIngress, PacketIngress, PlatformInputAdapter};
pub type MemoryInputIngress = nworlds_host::MemoryInputIngress<InputPacket>;

#[cfg(test)]
mod tests {
    use super::{InputIngress, MemoryInputIngress};
    use crate::input::{Button, InputPacket};

    #[test]
    fn memory_ingress_preserves_transport_order_and_drains_only_once() {
        let mut ingress = MemoryInputIngress::new();
        ingress.push(InputPacket::ButtonPressed(Button::Primary));
        ingress.push(InputPacket::ButtonReleased(Button::Primary));

        assert_eq!(ingress.len(), 2);
        let batch = ingress
            .drain()
            .expect("distinct stream sequence identities");
        assert_eq!(
            batch.packets().collect::<Vec<_>>(),
            vec![
                InputPacket::ButtonPressed(Button::Primary),
                InputPacket::ButtonReleased(Button::Primary),
            ]
        );
        assert!(ingress.is_empty());
        assert!(ingress.drain().unwrap().is_empty());
    }
}
