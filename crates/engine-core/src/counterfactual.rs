use crate::{JournalError, LogicalTime, Worldline};

pub fn fork_counterfactual<S, R, E, I>(
    worldline: &Worldline<S, R, E>,
    fork_time: LogicalTime,
    alternate_events: I,
) -> Result<Worldline<S, R, E>, JournalError>
where
    E: Clone,
    I: IntoIterator<Item = (LogicalTime, E)>,
{
    worldline.fork(fork_time, alternate_events)
}
