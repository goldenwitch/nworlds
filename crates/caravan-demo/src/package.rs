use caravan_domain::GameJournalEntry;

use crate::{
    engine_integration::{actual_worldline, CaravanJournalWriter, LogicalTime, Tau},
    CaravanInteraction, CaravanOrchestrator, CaravanRenderer, CaravanStage, OrchestratorError,
};

/// The target-neutral Caravan package composition supplied to a host.
pub type CaravanPackage = CaravanStage<CaravanInteraction, CaravanRenderer>;

/// Builds the small deterministic Caravan package used by host proofs.
pub fn demo_package() -> Result<CaravanPackage, OrchestratorError> {
    let mut writer = CaravanJournalWriter::new();
    writer.record(GameJournalEntry::create_saucer());
    let orchestrator = CaravanOrchestrator::new(
        actual_worldline(writer.finish()),
        LogicalTime::zero(),
        Tau::zero(),
        CaravanInteraction,
    )?;
    Ok(CaravanStage::new(orchestrator))
}
