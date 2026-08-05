use caravan_domain::{Terrain, TileId};
use caravan_reference::{state, ReferenceWorldline, State, Worldline};
use engine_branches::BranchError;
use engine_journal::{Journal, JournalWriter, JournalWriterError};
use engine_presentation::LinearPlayback;
use engine_sdk::Playback;
use engine_time::{LogicalTime, Tau};

use crate::input::{interaction_query, Button, InputPacket, InputPacketSet, InteractionDefinition};
use crate::publication::{writer_from_journal, PublicationError};
use crate::transformation::Transformation;

/// Errors raised while coordinating mutable Orchestrator control state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrchestratorError {
    Publication(PublicationError),
    NonActualAppend,
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

/// Developer-authored interaction logic for the first Caravan event.
#[derive(Clone, Copy, Debug, Default)]
pub struct CaravanInteraction;

impl InteractionDefinition for CaravanInteraction {
    type Transformation = Transformation;

    fn query(
        &self,
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
    playback: LinearPlayback,
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
        playback: LinearPlayback,
        tau: Tau,
        interaction: I,
    ) -> Result<Self, OrchestratorError> {
        let writer = writer_from_journal(worldline.journal())?;
        Ok(Self {
            worldline,
            writer,
            playback,
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

    /// Returns the logical time selected by the current playback policy.
    pub fn logical_time(&self) -> LogicalTime {
        self.playback.logical_time_at(self.tau)
    }

    /// Returns the statically composed playback mapping.
    pub const fn playback(&self) -> LinearPlayback {
        self.playback
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
    pub fn sample(&self) -> State {
        state(&self.worldline, self.logical_time())
    }

    /// Performs a direct lookahead query without changing the selected sample.
    pub fn lookahead_at(&self, logical_time: LogicalTime) -> State {
        state(&self.worldline, logical_time)
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
    pub fn interaction(&self) -> Transformation {
        interaction_query(
            &self.interaction,
            &self.packets,
            self.tau,
            self.logical_time(),
        )
    }

    /// Applies the current interaction and clears its packet accumulation.
    pub fn interact_and_apply(&mut self) -> Result<bool, OrchestratorError> {
        let transformation = self.interaction();
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
            return Err(OrchestratorError::NonActualAppend);
        }

        self.writer.record(payload);
        self.worldline = Worldline::new(self.worldline.context().clone(), self.writer.snapshot());
        Ok(true)
    }

    /// Selects an immutable counterfactual child and refreshes authoring state.
    pub fn select_counterfactual(
        &mut self,
        fork_boundary: LogicalTime,
        suffix: &Journal,
    ) -> Result<(), OrchestratorError> {
        let child = self.worldline.counterfactual(fork_boundary, suffix)?;
        self.writer = writer_from_journal(child.journal())?;
        self.worldline = child;
        Ok(())
    }

    /// Selects an immutable corrected child and refreshes authoring state.
    pub fn select_corrected(
        &mut self,
        fork_boundary: LogicalTime,
        suffix: &Journal,
    ) -> Result<(), OrchestratorError> {
        let child = self.worldline.corrected_suffix(fork_boundary, suffix)?;
        self.writer = writer_from_journal(child.journal())?;
        self.worldline = child;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{CaravanInteraction, CaravanOrchestrator, OrchestratorError};
    use crate::input::{Button, InputPacket};
    use caravan_domain::GameJournalEntry;
    use caravan_reference::{actual, state};
    use engine_journal::JournalWriter;
    use engine_presentation::LinearPlayback;
    use engine_time::{LogicalTime, Tau};

    fn time(ticks: i64) -> LogicalTime {
        LogicalTime::from_game_ticks(ticks).expect("test time is representable")
    }

    fn orchestrator() -> CaravanOrchestrator {
        let mut writer = JournalWriter::new();
        writer.record(GameJournalEntry::create_saucer());
        CaravanOrchestrator::new(
            actual(writer.finish()),
            LinearPlayback::one_to_one(),
            Tau::zero(),
            CaravanInteraction,
        )
        .expect("test orchestrator should initialize")
    }

    #[test]
    fn orchestrator_chooses_tau_and_queries_through_playback() {
        let mut orchestrator = orchestrator();
        orchestrator.set_tau(Tau::from_ticks(time(4).ticks()));

        assert_eq!(orchestrator.logical_time(), time(4));
        assert_eq!(orchestrator.sample().logical_time(), time(4));
        assert_eq!(orchestrator.lookahead_at(time(8)).logical_time(), time(8));
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
        orchestrator.receive_packet(InputPacket::ButtonPressed(Button::Primary));

        assert_eq!(
            orchestrator.interact_and_apply(),
            Err(OrchestratorError::NonActualAppend)
        );
    }
}
