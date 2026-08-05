use caravan_reference::{try_state, Snapshot};
use engine_presentation::Renderer;
use engine_sdk::{Frame, GameState, Playback};
use engine_time::{LogicalTime, Tau};

use crate::input::{InputPacket, InteractionDefinition};
use crate::orchestrator::{CaravanInteraction, CaravanOrchestrator, OrchestratorError};
use crate::transformation::Transformation;

/// The first concrete Stage composition for the Caravan anchor.
pub struct CaravanStage<I = CaravanInteraction, R = NoopRenderer> {
    orchestrator: CaravanOrchestrator<I>,
    renderer: R,
}

impl<I, R> CaravanStage<I, R>
where
    I: InteractionDefinition<Transformation = Transformation>,
    R: Renderer<Snapshot>,
{
    /// Composes one Orchestrator and one renderer into a Stage.
    pub fn new(orchestrator: CaravanOrchestrator<I>, renderer: R) -> Self {
        Self {
            orchestrator,
            renderer,
        }
    }

    /// Borrows the Stage's developer-authored Orchestrator.
    pub fn orchestrator(&self) -> &CaravanOrchestrator<I> {
        &self.orchestrator
    }

    /// Receives one abstract packet through the Stage boundary.
    pub fn receive_packet(&mut self, packet: InputPacket) {
        self.orchestrator.receive_packet(packet);
    }

    /// Sets the Stage-owned presentation sample.
    pub fn set_tau(&mut self, tau: Tau) {
        self.orchestrator.set_tau(tau);
    }

    /// Advances the Stage-owned presentation sample by signed ticks.
    pub fn advance_tau(&mut self, ticks: i64) -> Result<Tau, OrchestratorError> {
        self.orchestrator.advance_tau(ticks)
    }

    /// Runs interaction and publication through the Stage's Orchestrator.
    pub fn interact_and_apply(&mut self) -> Result<bool, OrchestratorError> {
        self.orchestrator.interact_and_apply()
    }

    /// Publishes a transformation as a counterfactual child through Stage.
    pub fn apply_counterfactual(
        &mut self,
        fork_boundary: LogicalTime,
        authoring_time: LogicalTime,
        transformation: Transformation,
    ) -> Result<bool, OrchestratorError> {
        self.orchestrator
            .apply_counterfactual(fork_boundary, authoring_time, transformation)
    }

    /// Publishes a transformation as a corrected child through Stage.
    pub fn apply_corrected(
        &mut self,
        fork_boundary: LogicalTime,
        authoring_time: LogicalTime,
        transformation: Transformation,
    ) -> Result<bool, OrchestratorError> {
        self.orchestrator
            .apply_corrected(fork_boundary, authoring_time, transformation)
    }

    /// Presents the Orchestrator's currently selected Tau.
    pub fn present(&self) -> Result<Frame<R::Output>, OrchestratorError> {
        self.present_at(self.orchestrator.tau())
    }

    /// Presents one explicit Tau without changing the Orchestrator cursor.
    pub fn present_at(&self, tau: Tau) -> Result<Frame<R::Output>, OrchestratorError> {
        let logical_time = self.orchestrator.playback().logical_time_at(tau);
        let state = try_state(self.orchestrator.worldline(), logical_time)?;
        Ok(Frame::new(tau, self.renderer.render(&state, tau)))
    }
}

/// Placeholder renderer for composition tests that do not inspect output.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopRenderer;

impl Renderer<Snapshot> for NoopRenderer {
    type Output = (LogicalTime, Tau);

    fn render(&self, state: &GameState<Snapshot>, tau: Tau) -> Self::Output {
        (state.logical_time(), tau)
    }
}

#[cfg(test)]
mod tests {
    use super::{CaravanStage, NoopRenderer};
    use crate::orchestrator::{CaravanInteraction, CaravanOrchestrator};
    use caravan_domain::GameJournalEntry;
    use caravan_reference::actual;
    use engine_journal::JournalWriter;
    use engine_presentation::LinearPlayback;
    use engine_time::{LogicalTime, Tau};

    fn worldline() -> caravan_reference::ReferenceWorldline {
        let mut writer = JournalWriter::new();
        writer.record(GameJournalEntry::create_saucer());
        actual(writer.finish())
    }

    fn stage() -> CaravanStage {
        let orchestrator = CaravanOrchestrator::new(
            worldline(),
            LinearPlayback::one_to_one(),
            Tau::zero(),
            CaravanInteraction,
        )
        .expect("stage orchestrator should initialize");
        CaravanStage::new(orchestrator, NoopRenderer)
    }

    #[test]
    fn stage_presents_the_orchestrator_selected_sample() {
        let mut stage = stage();
        let tau = Tau::from_ticks(
            LogicalTime::from_game_ticks(4)
                .expect("test game time is representable")
                .ticks(),
        );
        stage.set_tau(tau);

        let frame = stage.present().expect("stage sample should be valid");

        assert_eq!(frame.tau(), tau);
        assert_eq!(
            frame.payload(),
            &(LogicalTime::from_ticks(tau.ticks()), tau)
        );
    }

    #[test]
    fn explicit_presentation_sample_does_not_mutate_orchestrator_cursor() {
        let stage = stage();
        let explicit_tau = Tau::from_ticks(9);

        let frame = stage
            .present_at(explicit_tau)
            .expect("explicit stage sample should be valid");

        assert_eq!(frame.tau(), explicit_tau);
        assert_eq!(stage.orchestrator().tau(), Tau::zero());
    }
}
