#![forbid(unsafe_code)]

use engine_api::{actual, state, Journal, LogicalTime};

fn interval_transition(_: &mut (), _: LogicalTime, _: LogicalTime) {}

fn main() {
    let worldline = actual(Journal::empty());
    state(&worldline, LogicalTime::zero(), interval_transition);
}
