#![forbid(unsafe_code)]

use engine_api::{actual, GameJournalEntry, Journal, JournalWriter, LogicalTime};

fn main() {
    let mut writer = JournalWriter::new();
    writer.record(GameJournalEntry::create_saucer());
    let mut journal: Journal = writer.finish();
    journal.push(GameJournalEntry::create_saucer());

    let worldline = actual(journal);
    let mut worldline = worldline;
    worldline.journal_mut().clear();
    let _ = LogicalTime::zero();
}
