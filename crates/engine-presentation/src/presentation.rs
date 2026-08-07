use engine_sdk::{Frame, GameState};
use engine_time::Tau;

use crate::Renderer;

/// Renders one already-selected state at one exact presentation-time sample.
pub fn present<S, R>(state: &GameState<S>, renderer: &R, tau: Tau) -> Frame<R::Output>
where
    R: Renderer<S> + ?Sized,
{
    Frame::new(tau, renderer.render(state, tau))
}
