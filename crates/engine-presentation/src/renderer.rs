use engine_sdk::{GameState, Tau};

/// Produces an owned render value from one immutable state and presentation sample.
///
/// The renderer is a type-level composition, not a source of runtime render
/// input. Only `GameState<S>` and `Tau` enter render production.
pub trait Renderer<S> {
    /// The renderer-owned value carried by the SDK frame envelope.
    type Output;

    /// Renders without mutating the selected state or its authoritative inputs.
    fn render(state: &GameState<S>, tau: Tau) -> Self::Output;
}
