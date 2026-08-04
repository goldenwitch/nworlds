use engine_api::JournalEntry;

fn main() {
    let _ = JournalEntry::<()>::from_assigned_time(engine_api::LogicalTime::zero(), ());
}
