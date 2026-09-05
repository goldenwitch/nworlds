use engine_sdk::{Frame, GameState, Tau};

use crate::{present, Renderer};

/// Failure raised while advancing downstream visual time.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PresentationError {
    /// The requested visual-time advance is outside the signed tick range.
    VisualTimeOverflow,
}

/// One selected complete state plus its downstream visual-time coordinate.
pub struct PresentationDriver<S> {
    selected: GameState<S>,
    visual_time: Tau,
}

impl<S> PresentationDriver<S> {
    /// Anchors visual time at zero for one selected complete state.
    pub fn new(selected: GameState<S>) -> Self {
        Self {
            selected,
            visual_time: Tau::zero(),
        }
    }

    /// Borrows the selected complete state.
    pub fn selected(&self) -> &GameState<S> {
        &self.selected
    }

    /// Returns the visual time relative to the selected state.
    pub const fn visual_time(&self) -> Tau {
        self.visual_time
    }

    /// Selects a complete state and resets visual time to zero.
    pub fn select(&mut self, selected: GameState<S>) {
        self.selected = selected;
        self.visual_time = Tau::zero();
    }

    /// Advances only downstream visual time.
    pub fn advance_visual_time(&mut self, delta: Tau) -> Result<Tau, PresentationError> {
        self.visual_time = self
            .visual_time
            .checked_add(delta)
            .ok_or(PresentationError::VisualTimeOverflow)?;
        Ok(self.visual_time)
    }

    /// Sets downstream visual time without selecting or querying game state.
    pub const fn set_visual_time(&mut self, visual_time: Tau) {
        self.visual_time = visual_time;
    }

    /// Presents the selected complete state at the current visual time.
    pub fn present<R>(&self) -> Frame<R::Output>
    where
        R: Renderer<S> + ?Sized,
    {
        present::<S, R>(&self.selected, self.visual_time)
    }
}

/// An owned plan of complete state samples for read-ahead, scrubbing, or preview.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SamplePlan<S> {
    states: Vec<GameState<S>>,
}

impl<S> SamplePlan<S> {
    /// Creates a plan from complete state samples in caller-defined order.
    pub fn new(states: impl Into<Vec<GameState<S>>>) -> Self {
        Self {
            states: states.into(),
        }
    }

    /// Returns complete samples in plan order.
    pub fn states(&self) -> &[GameState<S>] {
        &self.states
    }

    /// Returns the number of complete samples in the plan.
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// Reports whether the plan has no complete samples.
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    /// Consumes the plan and returns its complete samples.
    pub fn into_states(self) -> Vec<GameState<S>> {
        self.states
    }
}

#[cfg(test)]
mod tests {
    use engine_sdk::{Frame, GameState, Tau};

    use super::{PresentationDriver, PresentationError, SamplePlan};
    use crate::Renderer;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct ProbeState(u32);

    struct ProbeRenderer;

    impl Renderer<ProbeState> for ProbeRenderer {
        type Output = (u32, Tau);

        fn render(state: &GameState<ProbeState>, tau: Tau) -> Self::Output {
            (state.payload().0, tau)
        }
    }

    fn state(value: u32, logical_time: i64) -> GameState<ProbeState> {
        GameState::new(
            engine_time::LogicalTime::from_ticks(logical_time),
            ProbeState(value),
        )
    }

    #[test]
    fn selecting_a_complete_state_resets_visual_time() {
        let mut driver = PresentationDriver::new(state(1, 0));
        driver
            .advance_visual_time(Tau::from_ticks(7))
            .expect("visual time should advance");

        driver.select(state(2, 10));

        assert_eq!(driver.selected().payload(), &ProbeState(2));
        assert_eq!(driver.visual_time(), Tau::zero());
        assert_eq!(
            driver.present::<ProbeRenderer>().payload(),
            &(2, Tau::zero())
        );
    }

    #[test]
    fn visual_time_changes_presentation_without_changing_selected_state() {
        let mut driver = PresentationDriver::new(state(1, 0));
        driver
            .advance_visual_time(Tau::from_ticks(7))
            .expect("visual time should advance");

        assert_eq!(driver.selected().payload(), &ProbeState(1));
        assert_eq!(driver.selected().logical_time().ticks(), 0);
        assert_eq!(
            driver.present::<ProbeRenderer>().payload(),
            &(1, Tau::from_ticks(7))
        );
    }

    #[test]
    fn visual_time_overflow_does_not_replace_the_selected_state() {
        let mut driver = PresentationDriver::new(state(1, 0));
        driver.set_visual_time(Tau::from_ticks(i64::MAX));

        assert_eq!(
            driver.advance_visual_time(Tau::from_ticks(1)),
            Err(PresentationError::VisualTimeOverflow)
        );
        assert_eq!(driver.selected().payload(), &ProbeState(1));
        assert_eq!(driver.visual_time(), Tau::from_ticks(i64::MAX));
    }

    #[test]
    fn sample_plan_owns_complete_states_without_transition_semantics() {
        let plan = SamplePlan::new(vec![state(1, 0), state(2, 10), state(3, 20)]);

        assert_eq!(plan.len(), 3);
        assert_eq!(plan.states()[1].payload(), &ProbeState(2));
        assert_eq!(plan.states()[2].logical_time().ticks(), 20);
        assert_eq!(plan.into_states().len(), 3);
    }

    fn assert_owned_frame<T: Send + Sync + 'static>() {}

    #[test]
    fn driver_and_plan_are_owned_static_values() {
        assert_owned_frame::<SamplePlan<ProbeState>>();
        assert_owned_frame::<Frame<(u32, Tau)>>();
    }
}
