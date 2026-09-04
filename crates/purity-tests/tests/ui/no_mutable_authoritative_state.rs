#![forbid(unsafe_code)]

use engine_api::{Context, GameState, Journal, LogicalTime, Worldline};

fn main() {
    let worldline: Worldline<(), ()> = Worldline::new(Context::new(()), Journal::empty());
    let mut worldline = worldline;
    worldline.journal_mut().clear();

    let mut sampled = GameState::new(LogicalTime::zero(), ());
    sampled.payload_mut();
}
