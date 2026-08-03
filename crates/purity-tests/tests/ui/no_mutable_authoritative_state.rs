#![forbid(unsafe_code)]

use engine_api::{actual, state, Journal, LogicalTime};

fn main() {
    let worldline = actual(Journal::empty());
    let mut worldline = worldline;
    worldline.journal_mut().clear();

    let mut sampled = state(&worldline, LogicalTime::zero());
    sampled.payload_mut();
}
