#![forbid(unsafe_code)]

use engine_api::{GameJournalEntry, JournalWriter, LogicalTime};

fn main() {
    let mut writer = JournalWriter::new();
    let mut entry = writer.record(GameJournalEntry::create_saucer());
    entry.logical_time = LogicalTime::from_ticks(7);
}
