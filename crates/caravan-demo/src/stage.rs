use caravan_reference::{state, ReferenceWorldline, Snapshot};
use engine_presentation::{present, Renderer};
use engine_sdk::{Frame, GameState};
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

    /// Mutably borrows the Stage's Orchestrator control state.
    pub fn orchestrator_mut(&mut self) -> &mut CaravanOrchestrator<I> {
        &mut self.orchestrator
    }

    /// Receives one abstract packet through the Stage boundary.
    pub fn receive_packet(&mut self, packet: InputPacket) {
        self.orchestrator.receive_packet(packet);
    }

    /// Runs interaction and publication through the Stage's Orchestrator.
    pub fn interact_and_apply(&mut self) -> Result<bool, OrchestratorError> {
        self.orchestrator.interact_and_apply()
    }

    /// Presents the Orchestrator's currently selected Tau.
    pub fn present(&self) -> Frame<R::Output> {
        self.present_at(self.orchestrator.tau())
    }

    /// Presents one explicit Tau without changing the Orchestrator cursor.
    pub fn present_at(&self, tau: Tau) -> Frame<R::Output> {
        let query = |worldline: &ReferenceWorldline, logical_time: LogicalTime| {
            state(worldline, logical_time)
        };
        present(
            self.orchestrator.worldline(),
            &query,
            &self.orchestrator.playback(),
            &self.renderer,
            tau,
        )
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
        stage.orchestrator_mut().set_tau(tau);

        let frame = stage.present();

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

        let frame = stage.present_at(explicit_tau);

        assert_eq!(frame.tau(), explicit_tau);
        assert_eq!(stage.orchestrator().tau(), Tau::zero());
    }
}
