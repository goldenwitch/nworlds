use engine_sdk::{GameState, Tau};

/// Produces an optional deterministic visual value from one state and `tau`.
pub trait Animation<S> {
    /// The animation value carried by an animated presentation result.
    type Output;

    /// Samples the animation without requiring a previous frame.
    fn sample(&self, state: &GameState<S>, tau: Tau) -> Option<Self::Output>;
}

/// Samples an optional animation boundary for an already selected state.
pub fn animate<S, A>(animation: Option<&A>, state: &GameState<S>, tau: Tau) -> Option<A::Output>
where
    A: Animation<S>,
{
    animation.and_then(|animation| animation.sample(state, tau))
}
