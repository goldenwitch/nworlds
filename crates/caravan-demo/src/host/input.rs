use std::collections::VecDeque;

use crate::input::{
    InputBatchError, InputObservation, InputPacket, ObservationId, OrderedInputBatch,
};

/// A Stage-facing transport port for already translated input packets.
pub trait InputIngress {
    /// Delivers one packet into the transport buffer.
    fn push(&mut self, packet: InputPacket);

    /// Drains packets in transport delivery order.
    fn drain(&mut self) -> Result<OrderedInputBatch, InputBatchError>;
}

/// In-memory input transport for tests and the first target composition.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MemoryInputIngress {
    observations: VecDeque<InputObservation>,
    stream_id: u64,
    next_sequence: u64,
}

impl MemoryInputIngress {
    /// Creates an empty in-memory ingress.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an ingress with an explicit source stream identity.
    pub fn with_stream_id(stream_id: u64) -> Self {
        Self {
            observations: VecDeque::new(),
            stream_id,
            next_sequence: 0,
        }
    }

    /// Returns the number of packets waiting in transport.
    pub fn len(&self) -> usize {
        self.observations.len()
    }

    /// Reports whether transport has no waiting packets.
    pub fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }
}

impl InputIngress for MemoryInputIngress {
    fn push(&mut self, packet: InputPacket) {
        let observation = InputObservation::new(
            ObservationId::new(self.stream_id, self.next_sequence),
            packet,
        );
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .expect("input stream sequence exhausted");
        self.observations.push_back(observation);
    }

    fn drain(&mut self) -> Result<OrderedInputBatch, InputBatchError> {
        OrderedInputBatch::from_observations(self.observations.drain(..))
    }
}

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
