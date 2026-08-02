use crate::{evaluate, AutonomousRule, Event, GameState, LogicalTime, Worldline};

pub fn evaluate_future<S, R, E>(
    worldline: &Worldline<S, R, E>,
    future_time: LogicalTime,
) -> GameState<S>
where
    S: Clone,
    R: AutonomousRule<S>,
    E: Event<S>,
{
    evaluate(worldline, future_time)
}
