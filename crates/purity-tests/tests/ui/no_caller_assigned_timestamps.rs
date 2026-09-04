#![forbid(unsafe_code)]

use engine_api::{JournalWriter, LogicalTime};

fn main() {
    let mut writer = JournalWriter::<()>::new();
    let mut entry = writer.record(());
    entry.logical_time = LogicalTime::from_ticks(7);
}
