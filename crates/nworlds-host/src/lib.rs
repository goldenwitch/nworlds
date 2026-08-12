#![forbid(unsafe_code)]

use std::collections::VecDeque;

/// Target-neutral game package runtime surface supplied to the host.
pub trait GamePackage {
    /// The package-defined normalized input batch.
    type InputBatch;
    /// The owned frame submitted to a target render sink.
    type Frame;
    /// Error raised while ingesting input or running one package step.
    type Error;
    /// Error raised while encoding the selected package state.
    type SaveError;
    /// Error raised while decoding and selecting package state.
    type LoadError;

    /// Accepts one normalized batch without assigning host or input time.
    fn ingest_batch(&mut self, batch: Self::InputBatch) -> Result<(), Self::Error>;

    /// Runs package-owned semantic control and returns one owned frame.
    fn step(&mut self) -> Result<(bool, Self::Frame), Self::Error>;

    /// Encodes the selected immutable package value for host byte transport.
    fn save_selected(&self) -> Result<Vec<u8>, Self::SaveError>;

    /// Decodes and selects a new immutable package value from host bytes.
    fn load_selected(&mut self, bytes: &[u8]) -> Result<(), Self::LoadError>;
}

/// Target-neutral transport of already translated input observations.
pub trait InputIngress {
    /// The package-defined packet vocabulary transported by this ingress.
    type Packet;
    /// The package-defined normalized batch produced by this ingress.
    type Batch;
    /// Error raised while normalizing the delivered observations.
    type Error;

    /// Delivers one abstract packet into the ingress.
    fn push(&mut self, packet: Self::Packet);

    /// Drains the delivered packets into one normalized package batch.
    fn drain(&mut self) -> Result<Self::Batch, Self::Error>;
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
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InputObservation<Packet> {
    id: ObservationId,
    packet: Packet,
}

impl<Packet> InputObservation<Packet> {
    /// Creates one observation without assigning game time.
    pub const fn new(id: ObservationId, packet: Packet) -> Self {
        Self { id, packet }
    }

    /// Returns the stable observation identity.
    pub const fn id(&self) -> ObservationId {
        self.id
    }

    /// Returns the semantic packet payload.
    pub const fn packet(&self) -> &Packet {
        &self.packet
    }
}

impl<Packet: Copy> Copy for InputObservation<Packet> {}

/// An error raised while normalizing an ordered input batch.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InputBatchError {
    /// Two observations carried the same source-stream identity.
    DuplicateObservation(ObservationId),
}

/// An identity-bearing, deterministically ordered semantic input batch.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct OrderedInputBatch<Packet> {
    observations: Vec<InputObservation<Packet>>,
}

impl<Packet> OrderedInputBatch<Packet> {
    /// Normalizes observations by stream identity and source-local sequence.
    pub fn from_observations(
        observations: impl IntoIterator<Item = InputObservation<Packet>>,
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
    pub fn observations(&self) -> &[InputObservation<Packet>] {
        &self.observations
    }

    /// Consumes the batch and returns its observations in semantic order.
    pub fn into_observations(self) -> Vec<InputObservation<Packet>> {
        self.observations
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
    pub fn iter(&self) -> impl Iterator<Item = &InputObservation<Packet>> {
        self.observations.iter()
    }

    /// Returns cloned semantic payloads in deterministic batch order.
    pub fn packets(&self) -> impl Iterator<Item = Packet> + '_
    where
        Packet: Clone,
    {
        self.observations
            .iter()
            .map(|observation| observation.packet().clone())
    }
}

/// In-memory input transport for tests and target composition proofs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryInputIngress<Packet> {
    observations: VecDeque<InputObservation<Packet>>,
    stream_id: u64,
    next_sequence: u64,
}

impl<Packet> Default for MemoryInputIngress<Packet> {
    fn default() -> Self {
        Self {
            observations: VecDeque::new(),
            stream_id: 0,
            next_sequence: 0,
        }
    }
}

impl<Packet> MemoryInputIngress<Packet> {
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

impl<Packet> InputIngress for MemoryInputIngress<Packet> {
    type Packet = Packet;
    type Batch = OrderedInputBatch<Packet>;
    type Error = InputBatchError;

    fn push(&mut self, packet: Self::Packet) {
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

    fn drain(&mut self) -> Result<Self::Batch, Self::Error> {
        OrderedInputBatch::from_observations(self.observations.drain(..))
    }
}

/// Narrow push-only view used by target-native input adapters.
pub trait PacketIngress<Packet> {
    /// Delivers one package-defined packet.
    fn push(&mut self, packet: Packet);
}

impl<I> PacketIngress<I::Packet> for I
where
    I: InputIngress,
{
    fn push(&mut self, packet: I::Packet) {
        InputIngress::push(self, packet);
    }
}

/// Translates one target-native event into a package-defined packet.
pub trait PlatformInputAdapter<Event, Packet> {
    /// Emits zero or more abstract packets for one native event.
    fn translate(&mut self, event: Event, ingress: &mut dyn PacketIngress<Packet>);
}

/// Target-neutral execution of owned package frames.
pub trait RenderSink<Frame> {
    /// Submits one owned frame for target execution or collection.
    fn submit(&mut self, frame: Frame);
}

/// Target-neutral transport of game-facing encoded bytes.
pub trait StorageTransport {
    /// Replaces the stored encoded record.
    fn store(&mut self, bytes: Vec<u8>);

    /// Returns an owned copy of the stored encoded record, if present.
    fn load(&self) -> Option<Vec<u8>>;
}

/// In-memory byte storage for tests and target composition proofs.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MemoryStorage {
    bytes: Option<Vec<u8>>,
}

impl MemoryStorage {
    /// Creates empty in-memory storage.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reports whether one encoded record is stored.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_none()
    }
}

impl StorageTransport for MemoryStorage {
    fn store(&mut self, bytes: Vec<u8>) {
        self.bytes = Some(bytes);
    }

    fn load(&self) -> Option<Vec<u8>> {
        self.bytes.clone()
    }
}

/// In-memory render sink for tests and target composition proofs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectingRenderSink<Frame> {
    frames: Vec<Frame>,
}

impl<Frame> Default for CollectingRenderSink<Frame> {
    fn default() -> Self {
        Self { frames: Vec::new() }
    }
}

impl<Frame> CollectingRenderSink<Frame> {
    /// Creates an empty collecting sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns submitted frames in submission order.
    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    /// Returns the most recently submitted frame, if any.
    pub fn last(&self) -> Option<&Frame> {
        self.frames.last()
    }
}

impl<Frame> RenderSink<Frame> for CollectingRenderSink<Frame> {
    fn submit(&mut self, frame: Frame) {
        self.frames.push(frame);
    }
}

/// Explicit host-side load failure when no byte record exists or decoding fails.
#[derive(Debug)]
pub enum StorageLoadError<E> {
    /// The selected storage transport contains no record.
    Empty,
    /// The package rejected the stored bytes.
    Package(E),
}

/// Generic host composition around one target-neutral package and independent
/// target ports.
pub struct ApplicationHost<P, I, S, R> {
    package: P,
    input: I,
    storage: S,
    render: R,
}

impl<P, I, S, R> ApplicationHost<P, I, S, R>
where
    P: GamePackage,
    I: InputIngress<Batch = P::InputBatch>,
    P::Error: From<I::Error>,
    S: StorageTransport,
    R: RenderSink<P::Frame>,
{
    /// Composes a package with independent target ports.
    pub fn new(package: P, input: I, storage: S, render: R) -> Self {
        Self {
            package,
            input,
            storage,
            render,
        }
    }

    /// Borrows the target-neutral package.
    pub fn package(&self) -> &P {
        &self.package
    }

    /// Mutably borrows the target-neutral package.
    pub fn package_mut(&mut self) -> &mut P {
        &mut self.package
    }

    /// Mutably borrows the input ingress port.
    pub fn input_mut(&mut self) -> &mut I {
        &mut self.input
    }

    /// Borrows the storage transport port.
    pub fn storage(&self) -> &S {
        &self.storage
    }

    /// Mutably borrows the storage transport port.
    pub fn storage_mut(&mut self) -> &mut S {
        &mut self.storage
    }

    /// Borrows the render sink port.
    pub fn render(&self) -> &R {
        &self.render
    }

    /// Pulls neutral input, delegates semantic control to the package, and
    /// submits the resulting owned frame to the target sink.
    pub fn step(&mut self) -> Result<bool, P::Error> {
        let batch = self.input.drain().map_err(P::Error::from)?;
        self.package.ingest_batch(batch)?;
        let (applied, frame) = self.package.step()?;
        self.render.submit(frame);
        Ok(applied)
    }

    /// Encodes and stores the package's selected immutable value.
    pub fn save_selected(&mut self) -> Result<(), P::SaveError> {
        self.storage.store(self.package.save_selected()?);
        Ok(())
    }

    /// Loads bytes through the package's game-facing decoder.
    pub fn load_selected(&mut self) -> Result<(), StorageLoadError<P::LoadError>> {
        let bytes = self.storage.load().ok_or(StorageLoadError::Empty)?;
        self.package
            .load_selected(&bytes)
            .map_err(StorageLoadError::Package)
    }
}

#[cfg(test)]
mod tests {
    use super::{ApplicationHost, GamePackage, InputIngress, RenderSink, StorageTransport};

    #[derive(Default)]
    struct TestInput {
        packets: Vec<u8>,
    }

    impl InputIngress for TestInput {
        type Packet = u8;
        type Batch = Vec<u8>;
        type Error = core::convert::Infallible;

        fn push(&mut self, packet: Self::Packet) {
            self.packets.push(packet);
        }

        fn drain(&mut self) -> Result<Self::Batch, Self::Error> {
            Ok(core::mem::take(&mut self.packets))
        }
    }

    #[derive(Default)]
    struct TestPackage {
        batches: Vec<Vec<u8>>,
        loaded: Vec<u8>,
    }

    impl GamePackage for TestPackage {
        type InputBatch = Vec<u8>;
        type Frame = Vec<u8>;
        type Error = core::convert::Infallible;
        type SaveError = core::convert::Infallible;
        type LoadError = core::convert::Infallible;

        fn ingest_batch(&mut self, batch: Self::InputBatch) -> Result<(), Self::Error> {
            self.batches.push(batch);
            Ok(())
        }

        fn step(&mut self) -> Result<(bool, Self::Frame), Self::Error> {
            Ok((true, self.batches.last().cloned().unwrap_or_default()))
        }

        fn save_selected(&self) -> Result<Vec<u8>, Self::SaveError> {
            Ok(self.batches.last().cloned().unwrap_or_default())
        }

        fn load_selected(&mut self, bytes: &[u8]) -> Result<(), Self::LoadError> {
            self.loaded = bytes.to_vec();
            Ok(())
        }
    }

    #[derive(Default)]
    struct TestStorage {
        bytes: Option<Vec<u8>>,
    }

    impl StorageTransport for TestStorage {
        fn store(&mut self, bytes: Vec<u8>) {
            self.bytes = Some(bytes);
        }

        fn load(&self) -> Option<Vec<u8>> {
            self.bytes.clone()
        }
    }

    #[derive(Default)]
    struct TestSink {
        frames: Vec<Vec<u8>>,
    }

    impl RenderSink<Vec<u8>> for TestSink {
        fn submit(&mut self, frame: Vec<u8>) {
            self.frames.push(frame);
        }
    }

    #[test]
    fn composition_delegates_to_a_target_neutral_package() {
        let mut host = ApplicationHost::new(
            TestPackage::default(),
            TestInput::default(),
            TestStorage::default(),
            TestSink::default(),
        );
        host.input_mut().push(7);
        host.input_mut().push(9);

        assert!(host.step().expect("test package step should succeed"));
        assert_eq!(host.package().batches, vec![vec![7, 9]]);
        assert_eq!(host.render().frames, vec![vec![7, 9]]);

        host.save_selected()
            .expect("test package save should succeed");
        let bytes = host.storage().load().expect("host should retain bytes");
        let mut restored = ApplicationHost::new(
            TestPackage::default(),
            TestInput::default(),
            TestStorage { bytes: Some(bytes) },
            TestSink::default(),
        );
        restored
            .load_selected()
            .expect("test package load should succeed");
        assert_eq!(restored.package().loaded, vec![7, 9]);
    }
}
