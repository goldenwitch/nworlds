use engine_sdk::{Frame, GameState, Playback};
use engine_time::{LogicalTime, Tau};

use crate::{animate, Animation, Renderer};

/// Supplies a direct indexed state query for an immutable worldline value.
pub trait QueryAdapter<W: ?Sized, S> {
    /// Queries the selected logical time without consuming or mutating the worldline.
    fn query(&self, worldline: &W, logical_time: LogicalTime) -> GameState<S>;
}

impl<W: ?Sized, S, F> QueryAdapter<W, S> for F
where
    F: Fn(&W, LogicalTime) -> GameState<S>,
{
    fn query(&self, worldline: &W, logical_time: LogicalTime) -> GameState<S> {
        self(worldline, logical_time)
    }
}

/// An SDK frame together with an optional animation value for the same sample.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AnimatedFrame<R, A> {
    frame: Frame<R>,
    animation: Option<A>,
}

impl<R, A> AnimatedFrame<R, A> {
    /// Creates an animated frame value.
    pub fn new(frame: Frame<R>, animation: Option<A>) -> Self {
        Self { frame, animation }
    }

    /// Borrows the rendered SDK frame.
    pub fn frame(&self) -> &Frame<R> {
        &self.frame
    }

    /// Borrows the optional animation value.
    pub fn animation(&self) -> Option<&A> {
        self.animation.as_ref()
    }

    /// Consumes the value and returns its frame and animation parts.
    pub fn into_parts(self) -> (Frame<R>, Option<A>) {
        (self.frame, self.animation)
    }
}

/// Queries and renders one exact presentation-time sample.
pub fn present<W: ?Sized, S, Q, P, R>(
    worldline: &W,
    query: &Q,
    playback: &P,
    renderer: &R,
    tau: Tau,
) -> Frame<R::Output>
where
    Q: QueryAdapter<W, S> + ?Sized,
    P: Playback + ?Sized,
    R: Renderer<S> + ?Sized,
{
    let state = query.query(worldline, playback.logical_time_at(tau));
    Frame::new(tau, renderer.render(&state, tau))
}

/// Queries, renders, and optionally samples one exact presentation-time value.
pub fn present_with_animation<W: ?Sized, S, Q, P, R, A>(
    worldline: &W,
    query: &Q,
    playback: &P,
    renderer: &R,
    animation: Option<&A>,
    tau: Tau,
) -> AnimatedFrame<R::Output, A::Output>
where
    Q: QueryAdapter<W, S> + ?Sized,
    P: Playback + ?Sized,
    R: Renderer<S> + ?Sized,
    A: Animation<S>,
{
    let state = query.query(worldline, playback.logical_time_at(tau));
    let rendered = renderer.render(&state, tau);
    let animation = animate(animation, &state, tau);

    AnimatedFrame::new(Frame::new(tau, rendered), animation)
}
