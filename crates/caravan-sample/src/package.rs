use caravan_domain::GameJournalEntry;
use nworlds_host::{
    HostVersionRequirement, PackageDeclaration, PersistenceRequirement,
    RenderVocabularyRequirement, SchemaVersion, SemanticVersion,
};

use crate::{
    engine_integration::{actual_worldline, CaravanJournalWriter, LogicalTime, Tau},
    CaravanInteraction, CaravanOrchestrator, CaravanRenderer, CaravanStage, OrchestratorError,
};

/// The target-neutral Caravan package composition supplied to a host.
pub type CaravanPackage = CaravanStage<CaravanInteraction, CaravanRenderer>;

/// The semantic requirements declared by the distributable Caravan package.
pub const CARAVAN_PACKAGE_DECLARATION: PackageDeclaration = PackageDeclaration::new(
    "caravan-sample",
    SemanticVersion::new(0, 1, 0),
    &[],
    PersistenceRequirement::new("caravan-worldline", SchemaVersion::new(1)),
    HostVersionRequirement::new(SemanticVersion::new(0, 1, 0)),
    RenderVocabularyRequirement::new("triangle-list-rgba", SemanticVersion::new(1, 0, 0)),
);

/// Builds the small deterministic Caravan package used by the sample and host proofs.
pub fn sample_package() -> Result<CaravanPackage, OrchestratorError> {
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
