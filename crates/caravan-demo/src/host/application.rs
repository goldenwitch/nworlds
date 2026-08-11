use crate::host::input::{InputIngress, MemoryInputIngress};
use crate::host::render::{CollectingRenderSink, RenderSinkAdapter};
use crate::host::storage::{MemoryStorage, StorageTransport};
use crate::{CaravanInteraction, CaravanRenderer, CaravanStage, OrchestratorError};

/// Error raised when a target-local host bundle has no stored record to load.
#[derive(Debug)]
pub enum StorageError {
    /// The selected storage transport contains no record.
    Empty,
    /// The game-facing persistence codec rejected the stored bytes.
    Persistence(engine_persistence::PersistenceError),
}

impl core::fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Empty => formatter.write_str("storage transport contains no record"),
            Self::Persistence(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Empty => None,
            Self::Persistence(error) => Some(error),
        }
    }
}

impl From<engine_persistence::PersistenceError> for StorageError {
    fn from(error: engine_persistence::PersistenceError) -> Self {
        Self::Persistence(error)
    }
}

/// A target-local convenience bundle around independent host ports.
pub struct ApplicationHost<I = MemoryInputIngress, S = MemoryStorage, R = CollectingRenderSink> {
    stage: CaravanStage<CaravanInteraction, CaravanRenderer>,
    input: I,
    storage: S,
    render: R,
}

impl<I, S, R> ApplicationHost<I, S, R>
where
    I: InputIngress,
    S: StorageTransport,
    R: RenderSinkAdapter,
{
    /// Composes a Stage with independent input, storage, and render ports.
    pub fn new(
        stage: CaravanStage<CaravanInteraction, CaravanRenderer>,
        input: I,
        storage: S,
        render: R,
    ) -> Self {
        Self {
            stage,
            input,
            storage,
            render,
        }
    }

    /// Borrows the composed Stage.
    pub fn stage(&self) -> &CaravanStage<CaravanInteraction, CaravanRenderer> {
        &self.stage
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

    /// Pulls transport packets, runs one interaction/publication step, and
    /// submits the selected state-first frame to the render sink.
    pub fn step(&mut self) -> Result<bool, OrchestratorError> {
        let batch = self.input.drain().map_err(OrchestratorError::from)?;
        self.stage.ingest_batch(batch)?;
        let applied = self.stage.interact_and_apply()?;
        let frame = self.stage.present()?;
        self.render.submit(frame);
        Ok(applied)
    }

    /// Encodes and stores the Stage's selected immutable worldline.
    pub fn save_selected(&mut self) -> Result<(), engine_persistence::PersistenceError> {
        self.storage
            .store(self.stage.orchestrator().save_selected()?);
        Ok(())
    }

    /// Loads the stored bytes through the game-facing persistence composition.
    pub fn load_selected(&mut self) -> Result<(), StorageError> {
        let bytes = self.storage.load().ok_or(StorageError::Empty)?;
        self.stage.load_selected(&bytes)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ApplicationHost;
    use crate::host::input::{InputIngress, MemoryInputIngress};
    use crate::host::render::CollectingRenderSink;
    use crate::host::storage::{MemoryStorage, StorageTransport};
    use crate::{CaravanInteraction, CaravanRenderer, CaravanStage};
    use caravan_domain::{GameJournalEntry, Terrain, TileId};
    use caravan_reference::actual;
    use engine_journal::JournalWriter;
    use engine_time::{LogicalTime, Tau};

    fn host() -> ApplicationHost {
        let mut writer = JournalWriter::new();
        writer.record(GameJournalEntry::create_saucer());
        let orchestrator = crate::CaravanOrchestrator::new(
            actual(writer.finish()),
            LogicalTime::zero(),
            Tau::zero(),
            CaravanInteraction,
        )
        .expect("host stage should initialize");
        ApplicationHost::new(
            CaravanStage::new(orchestrator, CaravanRenderer),
            MemoryInputIngress::new(),
            MemoryStorage::new(),
            CollectingRenderSink::new(),
        )
    }

    #[test]
    fn step_pulls_input_publishes_state_and_submits_a_frame() {
        let mut application = host();
        application
            .input_mut()
            .push(crate::input::InputPacket::ButtonPressed(
                crate::input::Button::Primary,
            ));

        assert!(application.step().expect("host step should succeed"));
        assert_eq!(application.render().frames().len(), 1);
        assert_eq!(
            application
                .stage()
                .orchestrator()
                .sample()
                .expect("published state should project")
                .payload()
                .terrain_at(TileId::origin()),
            Some(Terrain::Wheat)
        );
    }

    #[test]
    fn save_and_load_cross_the_host_transport_without_moving_semantics() {
        let mut application = host();
        application
            .input_mut()
            .push(crate::input::InputPacket::ButtonPressed(
                crate::input::Button::Primary,
            ));
        application.step().expect("host step should publish");
        application
            .save_selected()
            .expect("host save should succeed");
        let bytes = application
            .storage()
            .load()
            .expect("saved bytes should exist");

        let mut restored = host();
        restored
            .load_selected()
            .expect_err("empty storage should be explicit");
        restored.storage_mut().store(bytes);
        restored
            .load_selected()
            .expect("stored bytes should load through the host");
        assert_eq!(
            restored.stage().orchestrator().worldline(),
            application.stage().orchestrator().worldline()
        );
    }
}
