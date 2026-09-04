use crate::host::input::MemoryInputIngress;
use crate::host::render::CollectingRenderSink;
use crate::host::storage::MemoryStorage;
use crate::CaravanPackage;

pub use nworlds_host::{ApplicationHost as GenericApplicationHost, StorageLoadError};

/// Target-neutral Caravan package host composition used by in-memory proofs.
pub type ApplicationHost<I = MemoryInputIngress, S = MemoryStorage, R = CollectingRenderSink> =
    GenericApplicationHost<CaravanPackage, I, S, R>;

#[cfg(test)]
mod tests {
    use super::ApplicationHost;
    use crate::engine_integration::{CaravanJournalWriter, LogicalTime, Tau};
    use crate::host::input::{InputIngress, MemoryInputIngress};
    use crate::host::render::CollectingRenderSink;
    use crate::host::storage::{MemoryStorage, StorageTransport};
    use crate::{CaravanInteraction, CaravanStage};
    use caravan_domain::{GameJournalEntry, Terrain, TileId};
    use caravan_reference::actual;

    fn host() -> ApplicationHost {
        let mut writer = CaravanJournalWriter::new();
        writer.record(GameJournalEntry::create_saucer());
        let orchestrator = crate::CaravanOrchestrator::new(
            actual(writer.finish()),
            LogicalTime::zero(),
            Tau::zero(),
            CaravanInteraction,
        )
        .expect("host stage should initialize");
        ApplicationHost::new(
            CaravanStage::new(orchestrator),
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
                .package()
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
            restored.package().orchestrator().worldline(),
            application.package().orchestrator().worldline()
        );
    }
}
