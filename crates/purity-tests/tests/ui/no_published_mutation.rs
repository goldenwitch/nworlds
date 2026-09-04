#![forbid(unsafe_code)]

use engine_api::{Context, Journal, JournalWriter, LogicalTime, Worldline};

fn main() {
    let mut writer = JournalWriter::<()>::new();
    writer.record(());
    let mut journal: Journal<()> = writer.finish();
    journal.push(());

    let mut worldline = Worldline::new(Context::new(()), journal);
    worldline.journal_mut().clear();
    let _ = LogicalTime::zero();
}
