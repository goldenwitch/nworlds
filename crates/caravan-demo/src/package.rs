use caravan_domain::GameJournalEntry;
use caravan_reference::actual;
use engine_journal::JournalWriter;
use engine_time::{LogicalTime, Tau};

use crate::{
    CaravanInteraction, CaravanOrchestrator, CaravanRenderer, CaravanStage, OrchestratorError,
};

/// The target-neutral Caravan package composition supplied to a host.
pub type CaravanPackage = CaravanStage<CaravanInteraction, CaravanRenderer>;

/// Builds the small deterministic Caravan package used by host proofs.
pub fn demo_package() -> Result<CaravanPackage, OrchestratorError> {
    let mut writer = JournalWriter::new();
    writer.record(GameJournalEntry::create_saucer());
    let orchestrator = CaravanOrchestrator::new(
        actual(writer.finish()),
        LogicalTime::zero(),
        Tau::zero(),
        CaravanInteraction,
    )?;
    Ok(CaravanStage::new(orchestrator))
}
