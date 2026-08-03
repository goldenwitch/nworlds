use engine_sdk::{GameState, Tau};

/// Produces an owned render value from one immutable state and presentation sample.
pub trait Renderer<S> {
    /// The renderer-owned value carried by the SDK frame envelope.
    type Output;

    /// Renders without mutating the selected state or its authoritative inputs.
    fn render(&self, state: &GameState<S>, tau: Tau) -> Self::Output;
}
