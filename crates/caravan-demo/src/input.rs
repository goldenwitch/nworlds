use std::collections::HashSet;

use caravan_reference::State;
use engine_time::Tau;

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

/// Stable identity for one observation within one source stream.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObservationId {
    stream_id: u64,
    sequence: u64,
}

impl ObservationId {
    /// Creates an observation identity from a source stream and sequence.
    pub const fn new(stream_id: u64, sequence: u64) -> Self {
        Self {
            stream_id,
            sequence,
        }
    }

    /// Returns the source stream identity.
    pub const fn stream_id(self) -> u64 {
        self.stream_id
    }

    /// Returns the source-local observation sequence.
    pub const fn sequence(self) -> u64 {
        self.sequence
    }
}

/// One identity-bearing platform-neutral input observation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct InputObservation {
    id: ObservationId,
    packet: InputPacket,
}

impl InputObservation {
    /// Creates one observation without assigning game time.
    pub const fn new(id: ObservationId, packet: InputPacket) -> Self {
        Self { id, packet }
    }

    /// Returns the stable observation identity.
    pub const fn id(self) -> ObservationId {
        self.id
    }

    /// Returns the semantic packet payload.
    pub const fn packet(self) -> InputPacket {
        self.packet
    }
}

/// An error raised while normalizing an ordered input batch.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InputBatchError {
    /// Two observations carried the same source-stream identity.
    DuplicateObservation(ObservationId),
}

/// An identity-bearing, deterministically ordered semantic input batch.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct OrderedInputBatch {
    observations: Vec<InputObservation>,
}

impl OrderedInputBatch {
    /// Normalizes observations by stream identity and source-local sequence.
    pub fn from_observations(
        observations: impl IntoIterator<Item = InputObservation>,
    ) -> Result<Self, InputBatchError> {
        let mut observations = observations.into_iter().collect::<Vec<_>>();
        observations.sort_by_key(|observation| observation.id());

        if let Some(duplicate) = observations
            .windows(2)
            .find(|pair| pair[0].id() == pair[1].id())
        {
            return Err(InputBatchError::DuplicateObservation(duplicate[0].id()));
        }

        Ok(Self { observations })
    }

    /// Returns observations in deterministic semantic order.
    pub fn observations(&self) -> &[InputObservation] {
        &self.observations
    }

    /// Returns the number of observations in the batch.
    pub fn len(&self) -> usize {
        self.observations.len()
    }

    /// Reports whether the batch contains no observations.
    pub fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }

    /// Iterates over observations in deterministic semantic order.
    pub fn iter(&self) -> impl Iterator<Item = &InputObservation> {
        self.observations.iter()
    }

    /// Returns semantic payloads in deterministic batch order.
    pub fn packets(&self) -> impl Iterator<Item = InputPacket> + '_ {
        self.observations
            .iter()
            .map(|observation| observation.packet())
    }

    /// Projects the transport-normalized batch into a payload-only game view.
    pub fn semantic_batch(&self) -> SemanticInputBatch {
        SemanticInputBatch {
            packets: self.packets().collect(),
        }
    }

    /// Derives the current membership-only interaction view.
    pub fn membership_view(&self) -> InputPacketSet {
        self.semantic_batch().membership_view()
    }
}

/// The resolution applied after an interaction has inspected one input window.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InputResolution {
    /// Keep the observations pending for a later interaction attempt.
    Retain,
    /// Mark the observations handled by an accepted interaction.
    Consume,
    /// Remove the observations without treating them as handled game input.
    Discard,
}

/// An immutable snapshot of the observations presented to one interaction.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InputWindow {
    observations: Vec<InputObservation>,
}

impl InputWindow {
    /// Returns the identity-bearing observations in semantic order.
    pub fn observations(&self) -> &[InputObservation] {
        &self.observations
    }

    /// Returns the number of observations in the window.
    pub fn len(&self) -> usize {
        self.observations.len()
    }

    /// Reports whether the window contains no observations.
    pub fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }

    /// Returns the ordered semantic payloads without transport identities.
    pub fn packets(&self) -> impl Iterator<Item = InputPacket> + '_ {
        self.observations
            .iter()
            .map(|observation| observation.packet())
    }

    /// Projects the captured window into a payload-only game view.
    pub fn semantic_batch(&self) -> SemanticInputBatch {
        SemanticInputBatch {
            packets: self.packets().collect(),
        }
    }

    /// Derives the current membership-only interaction view.
    pub fn membership_view(&self) -> InputPacketSet {
        self.semantic_batch().membership_view()
    }
}

/// Ordered semantic input payloads supplied to game-facing interaction logic.
///
/// Transport identities, source streams, and delivery metadata are not exposed
/// through this type. Repeated equal payloads and their semantic order remain.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SemanticInputBatch {
    packets: Vec<InputPacket>,
}

impl SemanticInputBatch {
    /// Creates a semantic batch from payloads in the supplied order.
    pub fn from_packets(packets: impl IntoIterator<Item = InputPacket>) -> Self {
        Self {
            packets: packets.into_iter().collect(),
        }
    }

    /// Returns payloads in semantic order.
    pub fn packets(&self) -> &[InputPacket] {
        &self.packets
    }

    /// Iterates payloads in semantic order.
    pub fn iter(&self) -> impl Iterator<Item = &InputPacket> {
        self.packets.iter()
    }

    /// Returns the number of payload observations.
    pub fn len(&self) -> usize {
        self.packets.len()
    }

    /// Reports whether the batch contains no payload observations.
    pub fn is_empty(&self) -> bool {
        self.packets.is_empty()
    }

    /// Reports whether one payload occurs in the batch.
    pub fn contains(&self, packet: &InputPacket) -> bool {
        self.packets.contains(packet)
    }

    /// Derives the prototype unordered membership view.
    pub fn membership_view(&self) -> InputPacketSet {
        self.packets.iter().copied().collect()
    }
}

/// Orchestrator-owned pending input retention.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct InputBuffer {
    observations: Vec<InputObservation>,
}

impl InputBuffer {
    /// Creates an empty semantic input buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends one normalized batch, rejecting identities already pending.
    pub fn ingest(&mut self, batch: OrderedInputBatch) -> Result<(), InputBatchError> {
        if let Some(duplicate) = batch.observations.iter().find(|incoming| {
            self.observations
                .iter()
                .any(|pending| pending.id() == incoming.id())
        }) {
            return Err(InputBatchError::DuplicateObservation(duplicate.id()));
        }

        self.observations.extend(batch.observations);
        self.observations
            .sort_by_key(|observation| observation.id());
        Ok(())
    }

    /// Captures the observations currently pending for one interaction.
    pub fn snapshot(&self) -> InputWindow {
        InputWindow {
            observations: self.observations.clone(),
        }
    }

    /// Resolves one previously captured window.
    pub fn resolve(&mut self, window: &InputWindow, resolution: InputResolution) {
        if matches!(resolution, InputResolution::Retain) {
            return;
        }

        let ids = window
            .observations
            .iter()
            .map(|observation| observation.id())
            .collect::<HashSet<_>>();
        self.observations
            .retain(|observation| !ids.contains(&observation.id()));
    }

    /// Removes all pending observations.
    pub fn clear(&mut self) {
        self.observations.clear();
    }

    /// Returns the current pending membership view.
    pub fn semantic_batch(&self) -> SemanticInputBatch {
        self.snapshot().semantic_batch()
    }

    /// Returns the current pending membership view.
    pub fn membership_view(&self) -> InputPacketSet {
        self.semantic_batch().membership_view()
    }

    /// Returns the number of pending observations.
    pub fn len(&self) -> usize {
        self.observations.len()
    }

    /// Reports whether no observations are pending.
    pub fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }
}

/// The prototype unordered membership view derived from semantic input.
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

    /// Derives the prototype membership view from an ordered transport batch.
    pub fn from_batch(batch: &OrderedInputBatch) -> Self {
        batch.semantic_batch().membership_view()
    }

    /// Derives the prototype membership view from a semantic payload batch.
    pub fn from_semantic_batch(batch: &SemanticInputBatch) -> Self {
        batch.membership_view()
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

    /// Interprets one ordered semantic batch at one selected state and
    /// presentation sample.
    fn query(&self, state: &State, input: &SemanticInputBatch, tau: Tau) -> Self::Transformation;
}

/// Applies a statically composed interaction definition at the canonical seam.
pub fn interaction_query<D>(
    definition: &D,
    state: &State,
    input: &SemanticInputBatch,
    tau: Tau,
) -> D::Transformation
where
    D: InteractionDefinition,
{
    definition.query(state, input, tau)
}

#[cfg(test)]
mod tests {
    use super::{
        interaction_query, Button, InputBatchError, InputBuffer, InputObservation, InputPacket,
        InputPacketSet, InputResolution, InteractionDefinition, ObservationId, OrderedInputBatch,
        SemanticInputBatch,
    };
    use caravan_reference::{actual, state as reference_state};
    use engine_journal::Journal;
    use engine_time::{LogicalTime, Tau};

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct ObservedInput {
        state_logical_time: LogicalTime,
        packets: SemanticInputBatch,
        tau: Tau,
    }

    struct Observe;

    impl InteractionDefinition for Observe {
        type Transformation = ObservedInput;

        fn query(
            &self,
            state: &caravan_reference::State,
            packets: &SemanticInputBatch,
            tau: Tau,
        ) -> Self::Transformation {
            ObservedInput {
                state_logical_time: state.logical_time(),
                packets: packets.clone(),
                tau,
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
    fn ordered_batch_normalizes_delivery_order_by_stream_and_sequence() {
        let batch = OrderedInputBatch::from_observations([
            InputObservation::new(
                ObservationId::new(2, 1),
                InputPacket::ButtonPressed(Button::Secondary),
            ),
            InputObservation::new(
                ObservationId::new(1, 2),
                InputPacket::ButtonReleased(Button::Primary),
            ),
            InputObservation::new(
                ObservationId::new(1, 1),
                InputPacket::ButtonPressed(Button::Primary),
            ),
        ])
        .expect("distinct observations should normalize");

        assert_eq!(
            batch
                .iter()
                .map(|observation| observation.id())
                .collect::<Vec<_>>(),
            vec![
                ObservationId::new(1, 1),
                ObservationId::new(1, 2),
                ObservationId::new(2, 1),
            ]
        );
    }

    #[test]
    fn ordered_batch_rejects_duplicate_identity_but_keeps_equal_payloads_distinct() {
        let duplicate_id = ObservationId::new(4, 9);
        let duplicate = OrderedInputBatch::from_observations([
            InputObservation::new(duplicate_id, InputPacket::ButtonPressed(Button::Primary)),
            InputObservation::new(duplicate_id, InputPacket::ButtonReleased(Button::Primary)),
        ]);
        assert_eq!(
            duplicate,
            Err(InputBatchError::DuplicateObservation(duplicate_id))
        );

        let batch = OrderedInputBatch::from_observations([
            InputObservation::new(
                ObservationId::new(4, 1),
                InputPacket::ButtonPressed(Button::Primary),
            ),
            InputObservation::new(
                ObservationId::new(4, 2),
                InputPacket::ButtonPressed(Button::Primary),
            ),
        ])
        .expect("distinct identities should remain distinct");
        assert_eq!(batch.len(), 2);
        assert_eq!(batch.membership_view().len(), 1);
        assert_eq!(
            batch.semantic_batch().packets(),
            &[
                InputPacket::ButtonPressed(Button::Primary),
                InputPacket::ButtonPressed(Button::Primary),
            ]
        );
    }

    #[test]
    fn input_buffer_snapshots_and_resolves_only_the_captured_window() {
        let first = InputObservation::new(
            ObservationId::new(1, 1),
            InputPacket::ButtonPressed(Button::Primary),
        );
        let second = InputObservation::new(
            ObservationId::new(1, 2),
            InputPacket::ButtonReleased(Button::Primary),
        );
        let mut buffer = InputBuffer::new();
        buffer
            .ingest(OrderedInputBatch::from_observations([first]).unwrap())
            .expect("first batch should ingest");
        let window = buffer.snapshot();
        buffer
            .ingest(OrderedInputBatch::from_observations([second]).unwrap())
            .expect("second batch should ingest");

        buffer.resolve(&window, InputResolution::Consume);

        assert_eq!(buffer.snapshot().observations(), &[second]);
    }

    #[test]
    fn retained_window_survives_an_interaction_resolution() {
        let observation = InputObservation::new(
            ObservationId::new(1, 1),
            InputPacket::ButtonPressed(Button::Primary),
        );
        let mut buffer = InputBuffer::new();
        buffer
            .ingest(OrderedInputBatch::from_observations([observation]).unwrap())
            .expect("batch should ingest");
        let window = buffer.snapshot();

        buffer.resolve(&window, InputResolution::Retain);

        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn direct_and_retained_packet_construction_are_query_equivalent() {
        let direct = SemanticInputBatch::from_packets([
            InputPacket::ButtonPressed(Button::Primary),
            InputPacket::ButtonReleased(Button::Secondary),
        ]);
        let retained = SemanticInputBatch::from_packets([
            InputPacket::ButtonPressed(Button::Primary),
            InputPacket::ButtonReleased(Button::Secondary),
        ]);
        let tau = Tau::from_ticks(7);
        let logical_time = LogicalTime::from_ticks(11);
        let worldline = actual(Journal::empty());
        let state = reference_state(&worldline, logical_time);

        assert_eq!(
            interaction_query(&Observe, &state, &direct, tau),
            interaction_query(&Observe, &state, &retained, tau),
        );
        assert_eq!(
            interaction_query(&Observe, &state, &direct, tau).state_logical_time,
            logical_time,
        );
    }
}
