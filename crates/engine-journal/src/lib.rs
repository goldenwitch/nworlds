#![forbid(unsafe_code)]

mod journal;
mod writer;

pub use caravan_domain::GameJournalEntry;
pub type JournalEntry = engine_sdk::JournalEntry<GameJournalEntry>;
pub use journal::Journal;
pub use writer::{JournalWriter, JournalWriterError};
