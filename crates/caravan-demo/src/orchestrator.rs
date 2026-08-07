use caravan_domain::{Terrain, TileId};
use caravan_reference::{try_state, ProjectionError, ReferenceWorldline, State, Worldline};
use engine_branches::BranchError;
use engine_journal::{Journal, JournalWriter, JournalWriterError};
use engine_time::{LogicalTime, Tau};

use crate::input::{interaction_query, Button, InputPacket, InputPacketSet, InteractionDefinition};
use crate::publication::{
    publish_corrected, publish_counterfactual, writer_from_journal, PublicationError,
};
use crate::transformation::Transformation;

/// Errors raised while coordinating mutable Orchestrator control state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrchestratorError {
    Publication(PublicationError),
    AppendRequiresActualWorldline,
    Projection(ProjectionError),
    TauOverflow,
}

impl From<PublicationError> for OrchestratorError {
    fn from(error: PublicationError) -> Self {
        Self::Publication(error)
    }
}

impl From<BranchError> for OrchestratorError {
    fn from(error: BranchError) -> Self {
        Self::Publication(PublicationError::Branch(error))
    }
}

impl From<JournalWriterError> for OrchestratorError {
    fn from(error: JournalWriterError) -> Self {
        Self::Publication(PublicationError::JournalWriter(error))
    }
}

impl From<ProjectionError> for OrchestratorError {
    fn from(error: ProjectionError) -> Self {
        Self::Projection(error)
    }
}

/// Developer-authored interaction logic for the first Caravan event.
#[derive(Clone, Copy, Debug, Default)]
pub struct CaravanInteraction;

impl InteractionDefinition for CaravanInteraction {
    type Transformation = Transformation;

    fn query(
        &self,
        _state: &State,
        packets: &InputPacketSet,
        _tau: Tau,
        _logical_time: LogicalTime,
    ) -> Self::Transformation {
        if packets.contains(&InputPacket::ButtonPressed(Button::Primary)) {
            Transformation::SetTerrain {
                tile: TileId::origin(),
                terrain: Terrain::Wheat,
            }
        } else {
            Transformation::Noop
        }
    }
}

/// The first concrete mutable game orchestrator, composed inside Stage.
pub struct CaravanOrchestrator<I = CaravanInteraction> {
    worldline: ReferenceWorldline,
    writer: JournalWriter,
    logical_time: LogicalTime,
    tau: Tau,
    packets: InputPacketSet,
    interaction: I,
}

impl<I> CaravanOrchestrator<I>
where
    I: InteractionDefinition<Transformation = Transformation>,
{
    /// Creates an Orchestrator from one immutable published worldline.
    pub fn new(
        worldline: ReferenceWorldline,
        logical_time: LogicalTime,
        tau: Tau,
        interaction: I,
    ) -> Result<Self, OrchestratorError> {
        let writer = writer_from_journal(worldline.journal())?;
        Ok(Self {
            worldline,
            writer,
            logical_time,
            tau,
            packets: InputPacketSet::new(),
            interaction,
        })
    }

    /// Borrows the currently selected immutable worldline.
    pub fn worldline(&self) -> &ReferenceWorldline {
        &self.worldline
    }

    /// Returns the currently selected presentation time.
    pub const fn tau(&self) -> Tau {
        self.tau
    }

    /// Returns the currently selected logical game time.
    pub fn logical_time(&self) -> LogicalTime {
        self.logical_time
    }

    /// Replaces the selected logical game time explicitly.
    pub fn set_logical_time(&mut self, logical_time: LogicalTime) {
        self.logical_time = logical_time;
    }

    /// Replaces the selected presentation sample explicitly.
    pub fn set_tau(&mut self, tau: Tau) {
        self.tau = tau;
    }

    /// Advances the selected presentation sample by signed ticks.
    pub fn advance_tau(&mut self, ticks: i64) -> Result<Tau, OrchestratorError> {
        self.tau = self
            .tau
            .checked_add_ticks(ticks)
            .ok_or(OrchestratorError::TauOverflow)?;
        Ok(self.tau)
    }

    /// Queries the selected worldline at the current Stage sample.
    pub fn sample(&self) -> Result<State, OrchestratorError> {
        Ok(try_state(&self.worldline, self.logical_time())?)
    }

    /// Performs a direct lookahead query without changing the selected sample.
    pub fn lookahead_at(&self, logical_time: LogicalTime) -> Result<State, OrchestratorError> {
        Ok(try_state(&self.worldline, logical_time)?)
    }

    /// Adds one abstract packet to the current packet accumulation.
    pub fn receive_packet(&mut self, packet: InputPacket) {
        self.packets.insert(packet);
    }

    /// Removes all currently accumulated packets after an interaction call.
    pub fn clear_packets(&mut self) {
        self.packets = InputPacketSet::new();
    }

    /// Borrows the current packet set for inspection by the orchestration code.
    pub fn packets(&self) -> &InputPacketSet {
        &self.packets
    }

    /// Runs the pure interaction seam at the current selected sample.
    pub fn interaction(&self) -> Result<Transformation, OrchestratorError> {
        let state = self.sample()?;
        Ok(interaction_query(
            &self.interaction,
            &state,
            &self.packets,
            self.tau,
            self.logical_time(),
        ))
    }

    /// Applies the current interaction and clears its packet accumulation.
    pub fn interact_and_apply(&mut self) -> Result<bool, OrchestratorError> {
        let transformation = self.interaction()?;
        self.clear_packets();
        self.apply_transformation(transformation)
    }

    /// Publishes an accepted transformation onto the selected actual branch.
    pub fn apply_transformation(
        &mut self,
        transformation: Transformation,
    ) -> Result<bool, OrchestratorError> {
        let Some(payload) = transformation.into_journal_entry() else {
            return Ok(false);
        };
        if !self.worldline.is_actual() {
            return Err(OrchestratorError::AppendRequiresActualWorldline);
        }

        self.writer.record(payload);
        self.worldline = Worldline::new(self.worldline.context().clone(), self.writer.snapshot());
        Ok(true)
    }

    /// Publishes an accepted transformation as a new counterfactual child.
    pub fn apply_counterfactual(
        &mut self,
        fork_boundary: LogicalTime,
        authoring_time: LogicalTime,
        transformation: Transformation,
    ) -> Result<bool, OrchestratorError> {
        if transformation.into_journal_entry().is_none() {
            return Ok(false);
        }
        let child = publish_counterfactual(
            &self.worldline,
            fork_boundary,
            authoring_time,
            transformation,
        )?;
        self.replace_worldline(child)?;
        Ok(true)
    }

    /// Publishes an accepted transformation as a new corrected child.
    pub fn apply_corrected(
        &mut self,
        fork_boundary: LogicalTime,
        authoring_time: LogicalTime,
        transformation: Transformation,
    ) -> Result<bool, OrchestratorError> {
        if transformation.into_journal_entry().is_none() {
            return Ok(false);
        }
        let child = publish_corrected(
            &self.worldline,
            fork_boundary,
            authoring_time,
            transformation,
        )?;
        self.replace_worldline(child)?;
        Ok(true)
    }

    /// Selects an immutable counterfactual child and refreshes authoring state.
    pub fn select_counterfactual(
        &mut self,
        fork_boundary: LogicalTime,
        suffix: &Journal,
    ) -> Result<(), OrchestratorError> {
        let child = self.worldline.counterfactual(fork_boundary, suffix)?;
        self.replace_worldline(child)
    }

    /// Selects an immutable corrected child and refreshes authoring state.
    pub fn select_corrected(
        &mut self,
        fork_boundary: LogicalTime,
        suffix: &Journal,
    ) -> Result<(), OrchestratorError> {
        let child = self.worldline.corrected_suffix(fork_boundary, suffix)?;
        self.replace_worldline(child)
    }

    /// Serializes the selected immutable worldline for an Orchestrator save choice.
    pub fn save_selected(&self) -> Result<Vec<u8>, engine_persistence::PersistenceError> {
        engine_persistence::encode(&self.worldline)
    }

    fn replace_worldline(
        &mut self,
        worldline: ReferenceWorldline,
    ) -> Result<(), OrchestratorError> {
        self.writer = writer_from_journal(worldline.journal())?;
        self.worldline = worldline;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{CaravanInteraction, CaravanOrchestrator, OrchestratorError};
    use crate::input::{Button, InputPacket};
    use crate::transformation::Transformation;
    use caravan_domain::GameJournalEntry;
    use caravan_reference::{actual, state, ProjectionError};
    use engine_journal::JournalWriter;
    use engine_time::{LogicalTime, Tau};

    fn time(ticks: i64) -> LogicalTime {
        LogicalTime::from_game_ticks(ticks).expect("test time is representable")
    }

    fn orchestrator() -> CaravanOrchestrator {
        let mut writer = JournalWriter::new();
        writer.record(GameJournalEntry::create_saucer());
        CaravanOrchestrator::new(
            actual(writer.finish()),
            LogicalTime::zero(),
            Tau::zero(),
            CaravanInteraction,
        )
        .expect("test orchestrator should initialize")
    }

    #[test]
    fn orchestrator_selects_logical_and_presentation_times_independently() {
        let mut orchestrator = orchestrator();
        orchestrator.set_logical_time(time(4));
        orchestrator.set_tau(Tau::from_ticks(9));
        let original_worldline = orchestrator.worldline().clone();
        let original_tau = orchestrator.tau();

        assert_eq!(orchestrator.logical_time(), time(4));
        assert_eq!(
            orchestrator
                .sample()
                .expect("sample should be valid")
                .logical_time(),
            time(4)
        );
        assert_eq!(
            orchestrator
                .lookahead_at(time(8))
                .expect("lookahead should be valid")
                .logical_time(),
            time(8)
        );
        assert_eq!(orchestrator.worldline(), &original_worldline);
        assert_eq!(orchestrator.tau(), original_tau);
    }

    #[test]
    fn interaction_publication_replaces_worldline_without_mutating_previous_value() {
        let mut orchestrator = orchestrator();
        let parent = orchestrator.worldline().clone();
        orchestrator.receive_packet(InputPacket::ButtonPressed(Button::Primary));

        assert!(orchestrator
            .interact_and_apply()
            .expect("publication succeeds"));
        assert_eq!(parent.journal().len(), 1);
        assert_eq!(orchestrator.worldline().journal().len(), 2);
        assert_eq!(
            state(&parent, time(0))
                .payload()
                .terrain_at(caravan_domain::TileId::origin()),
            Some(caravan_domain::Terrain::Void)
        );
        assert_eq!(
            orchestrator
                .sample()
                .expect("published sample should be valid")
                .payload()
                .terrain_at(caravan_domain::TileId::origin()),
            Some(caravan_domain::Terrain::Wheat)
        );
    }

    #[test]
    fn empty_input_is_a_noop_and_branch_append_requires_explicit_policy() {
        let mut orchestrator = orchestrator();
        assert!(!orchestrator.interact_and_apply().expect("noop is valid"));

        let suffix = {
            let mut writer = JournalWriter::new();
            writer.advance_to(time(2)).expect("suffix time is forward");
            writer.record(GameJournalEntry::create_saucer());
            writer.finish()
        };
        orchestrator
            .select_counterfactual(time(0), &suffix)
            .expect("counterfactual selection succeeds");
        let child_before_rejected_append = orchestrator.worldline().clone();
        orchestrator.receive_packet(InputPacket::ButtonPressed(Button::Primary));

        assert_eq!(
            orchestrator.interact_and_apply(),
            Err(OrchestratorError::AppendRequiresActualWorldline)
        );
        assert_eq!(
            orchestrator.worldline(),
            &child_before_rejected_append,
            "rejected child append must not publish a replacement"
        );
    }

    #[test]
    fn orchestrator_appends_at_writer_cursor_not_selected_tau() {
        let mut writer = JournalWriter::new();
        writer.record(GameJournalEntry::create_saucer());
        writer
            .advance_to(time(7))
            .expect("postdated time is forward");
        writer.record(GameJournalEntry::SetTerrain {
            tile: caravan_domain::TileId::origin(),
            terrain: caravan_domain::Terrain::Forest,
        });
        let mut orchestrator = CaravanOrchestrator::new(
            caravan_reference::actual(writer.finish()),
            LogicalTime::zero(),
            Tau::from_ticks(time(2).ticks()),
            CaravanInteraction,
        )
        .expect("postdated orchestrator should initialize");
        orchestrator.receive_packet(InputPacket::ButtonPressed(Button::Primary));

        orchestrator
            .interact_and_apply()
            .expect("publication should use the writer cursor");

        assert_eq!(orchestrator.worldline().journal().len(), 3);
        assert_eq!(
            orchestrator
                .worldline()
                .journal()
                .get(2)
                .unwrap()
                .logical_time(),
            time(7)
        );
        assert_ne!(
            orchestrator
                .worldline()
                .journal()
                .get(2)
                .unwrap()
                .logical_time(),
            LogicalTime::from_ticks(orchestrator.tau().ticks())
        );
    }

    #[test]
    fn accepted_transformations_can_publish_explicit_child_branches() {
        let mut orchestrator = orchestrator();
        let parent = orchestrator.worldline().clone();

        assert!(orchestrator
            .apply_counterfactual(
                time(0),
                time(1),
                Transformation::SetTerrain {
                    tile: caravan_domain::TileId::origin(),
                    terrain: caravan_domain::Terrain::Forest,
                },
            )
            .expect("counterfactual publication succeeds"));
        assert_eq!(
            orchestrator.worldline().kind(),
            engine_branches::BranchKind::Counterfactual
        );
        assert_eq!(parent.journal().len(), 1);
        assert_eq!(orchestrator.worldline().journal().len(), 2);

        assert!(orchestrator
            .apply_corrected(
                time(0),
                time(2),
                Transformation::SetTerrain {
                    tile: caravan_domain::TileId::origin(),
                    terrain: caravan_domain::Terrain::Wheat,
                },
            )
            .expect("corrected publication succeeds"));
        assert_eq!(
            orchestrator.worldline().kind(),
            engine_branches::BranchKind::Corrected
        );
    }

    #[test]
    fn malformed_radius_is_an_explicit_projection_error() {
        let mut writer = JournalWriter::new();
        writer.record(GameJournalEntry::CreateSaucer { radius: 4 });
        let mut orchestrator = CaravanOrchestrator::new(
            caravan_reference::actual(writer.finish()),
            LogicalTime::zero(),
            Tau::zero(),
            CaravanInteraction,
        )
        .expect("authoring cursor can represent malformed payloads");

        assert!(matches!(
            orchestrator.sample(),
            Err(OrchestratorError::Projection(
                ProjectionError::UnsupportedSaucerRadius { found: 4, .. }
            ))
        ));

        orchestrator.receive_packet(InputPacket::ButtonPressed(Button::Primary));
        assert!(matches!(
            orchestrator.interact_and_apply(),
            Err(OrchestratorError::Projection(
                ProjectionError::UnsupportedSaucerRadius { found: 4, .. }
            ))
        ));
        assert_eq!(orchestrator.packets().len(), 1);
    }
}
