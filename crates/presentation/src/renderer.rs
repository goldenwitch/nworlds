use engine_core::{evaluate, AutonomousRule, Event, GameState, Tau, Worldline};

use crate::Playback;

pub trait Renderer<S> {
    type Frame;

    fn render(&self, state: &GameState<S>, tau: Tau) -> Self::Frame;
}

pub trait Animation<S> {
    type Sample;

    fn sample(&self, state: &GameState<S>, tau: Tau) -> Option<Self::Sample>;
}

pub fn present<S, R, E, P, T>(
    worldline: &Worldline<S, R, E>,
    playback: &P,
    renderer: &T,
    tau: Tau,
) -> T::Frame
where
    S: Clone,
    R: AutonomousRule<S>,
    E: Event<S>,
    P: Playback,
    T: Renderer<S>,
{
    let state = evaluate(worldline, playback.logical_time(tau));
    renderer.render(&state, tau)
}
