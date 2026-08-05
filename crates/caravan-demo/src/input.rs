use std::collections::HashSet;

use engine_time::{LogicalTime, Tau};

/// A closed, platform-neutral input packet used by the Caravan Stage.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InputPacket {
    ButtonPressed(Button),
    ButtonReleased(Button),
}

/// The first abstract button vocabulary for the Caravan input seam.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Button {
    Primary,
    Secondary,
}

/// The semantic set of packets supplied to one interaction query.
///
/// The backing collection is private so packet-set semantics remain stable if
/// the implementation later changes.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InputPacketSet {
    packets: HashSet<InputPacket>,
}

impl InputPacketSet {
    /// Creates an empty packet set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a packet set from one collection of observations.
    pub fn from_packets(packets: impl IntoIterator<Item = InputPacket>) -> Self {
        packets.into_iter().collect()
    }

    /// Inserts one packet, returning whether it was not already present.
    pub fn insert(&mut self, packet: InputPacket) -> bool {
        self.packets.insert(packet)
    }

    /// Returns whether the set contains a packet.
    pub fn contains(&self, packet: &InputPacket) -> bool {
        self.packets.contains(packet)
    }

    /// Returns the number of distinct packets in the set.
    pub fn len(&self) -> usize {
        self.packets.len()
    }

    /// Returns whether the set contains no packets.
    pub fn is_empty(&self) -> bool {
        self.packets.is_empty()
    }

    /// Iterates over packet membership without promising an order.
    pub fn iter(&self) -> impl Iterator<Item = &InputPacket> {
        self.packets.iter()
    }
}

impl Extend<InputPacket> for InputPacketSet {
    fn extend<T>(&mut self, packets: T)
    where
        T: IntoIterator<Item = InputPacket>,
    {
        self.packets.extend(packets);
    }
}

impl FromIterator<InputPacket> for InputPacketSet {
    fn from_iter<T>(packets: T) -> Self
    where
        T: IntoIterator<Item = InputPacket>,
    {
        Self {
            packets: packets.into_iter().collect(),
        }
    }
}

/// Developer-authored pure interaction logic for one concrete Stage.
pub trait InteractionDefinition {
    /// The closed value returned by one interaction query.
    type Transformation;

    /// Interprets one packet set at one explicit presentation and game sample.
    fn query(
        &self,
        packets: &InputPacketSet,
        tau: Tau,
        logical_time: LogicalTime,
    ) -> Self::Transformation;
}

/// Applies a statically composed interaction definition at the canonical seam.
pub fn interaction_query<D>(
    definition: &D,
    packets: &InputPacketSet,
    tau: Tau,
    logical_time: LogicalTime,
) -> D::Transformation
where
    D: InteractionDefinition,
{
    definition.query(packets, tau, logical_time)
}

#[cfg(test)]
mod tests {
    use super::{interaction_query, Button, InputPacket, InputPacketSet, InteractionDefinition};
    use engine_time::{LogicalTime, Tau};

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct ObservedInput {
        packets: InputPacketSet,
        tau: Tau,
        logical_time: LogicalTime,
    }

    struct Observe;

    impl InteractionDefinition for Observe {
        type Transformation = ObservedInput;

        fn query(
            &self,
            packets: &InputPacketSet,
            tau: Tau,
            logical_time: LogicalTime,
        ) -> Self::Transformation {
            ObservedInput {
                packets: packets.clone(),
                tau,
                logical_time,
            }
        }
    }

    #[test]
    fn packet_set_collapses_duplicate_packets_without_order_guarantees() {
        let packets = InputPacketSet::from_packets([
            InputPacket::ButtonPressed(Button::Primary),
            InputPacket::ButtonPressed(Button::Primary),
        ]);

        assert_eq!(packets.len(), 1);
        assert!(packets.contains(&InputPacket::ButtonPressed(Button::Primary)));
    }

    #[test]
    fn direct_and_retained_packet_construction_are_query_equivalent() {
        let direct = InputPacketSet::from_packets([
            InputPacket::ButtonPressed(Button::Primary),
            InputPacket::ButtonReleased(Button::Secondary),
        ]);
        let mut retained = InputPacketSet::new();
        retained.insert(InputPacket::ButtonPressed(Button::Primary));
        retained.extend([InputPacket::ButtonReleased(Button::Secondary)]);
        let tau = Tau::from_ticks(7);
        let logical_time = LogicalTime::from_ticks(11);

        assert_eq!(
            interaction_query(&Observe, &direct, tau, logical_time),
            interaction_query(&Observe, &retained, tau, logical_time),
        );
    }
}
