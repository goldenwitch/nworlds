use core::fmt;

use caravan_domain::GameJournalEntry;
use engine_time::LogicalTime;

use crate::{Journal, JournalEntry};

/// An error from attempting to move a journal writer backward in time.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum JournalWriterError {
    /// The requested cursor time is earlier than the current cursor time.
    BackwardTime {
        current: LogicalTime,
        requested: LogicalTime,
    },
}

impl JournalWriterError {
    /// Returns the cursor time that could not be moved backward.
    pub const fn current_time(self) -> LogicalTime {
        match self {
            Self::BackwardTime { current, .. } => current,
        }
    }

    /// Returns the earlier time that was rejected.
    pub const fn requested_time(self) -> LogicalTime {
        match self {
            Self::BackwardTime { requested, .. } => requested,
        }
    }
}

impl fmt::Display for JournalWriterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackwardTime { current, requested } => write!(
                formatter,
                "cannot move journal time backward from {current} to {requested}"
            ),
        }
    }
}

impl std::error::Error for JournalWriterError {}

/// A monotonic authoring cursor for immutable game journal values.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct JournalWriter {
    current_time: LogicalTime,
    entries: Vec<JournalEntry>,
}

impl JournalWriter {
    /// Creates a writer at logical time zero with no entries.
    pub fn new() -> Self {
        Self {
            current_time: LogicalTime::zero(),
            entries: Vec::new(),
        }
    }

    /// Returns the writer's current timestamp cursor.
    pub const fn current_time(&self) -> LogicalTime {
        self.current_time
    }

    /// Moves the timestamp cursor forward or leaves it at the same time.
    pub fn advance_to(&mut self, target_time: LogicalTime) -> Result<(), JournalWriterError> {
        if target_time < self.current_time {
            return Err(JournalWriterError::BackwardTime {
                current: self.current_time,
                requested: target_time,
            });
        }

        self.current_time = target_time;
        Ok(())
    }

    /// Records a game payload at the current writer time.
    pub fn record(&mut self, payload: GameJournalEntry) -> JournalEntry {
        let entry = JournalEntry::new(self.current_time, payload);
        self.entries.push(entry.clone());
        entry
    }

    /// Publishes an immutable snapshot while retaining this writer.
    pub fn snapshot(&self) -> Journal {
        Journal::from_entries(self.entries.iter().cloned())
    }

    /// Consumes the writer and publishes its immutable journal.
    pub fn finish(self) -> Journal {
        Journal::from_entries(self.entries)
    }
}

impl Default for JournalWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use caravan_domain::{ActorId, ActorKind, GameJournalEntry, Terrain, TileId};
    use engine_time::LogicalTime;

    use super::{JournalWriter, JournalWriterError};

    fn time(ticks: i64) -> LogicalTime {
        LogicalTime::from_ticks(ticks)
    }

    fn spawn(id: u64) -> GameJournalEntry {
        GameJournalEntry::SpawnActor {
            id: ActorId::new(id).expect("test actor IDs are positive"),
            kind: ActorKind::Farmer,
            tile: TileId::origin(),
        }
    }

    #[test]
    fn record_assigns_the_current_time_to_the_sdk_envelope() {
        let mut writer = JournalWriter::new();
        writer.advance_to(time(4)).expect("forward time is valid");

        let recorded = writer.record(GameJournalEntry::create_saucer());

        assert_eq!(recorded.logical_time(), time(4));
        assert_eq!(recorded.payload(), &GameJournalEntry::create_saucer());
    }

    #[test]
    fn postdated_entries_are_hidden_until_their_target_time() {
        let mut writer = JournalWriter::new();
        writer.advance_to(time(0)).expect("zero is the anchor time");
        writer.record(GameJournalEntry::create_saucer());
        writer
            .advance_to(time(10))
            .expect("postdating is forward time");
        writer.record(spawn(1));

        let journal = writer.finish();
        let before = journal.visible_at(time(9)).collect::<Vec<_>>();
        let at_target = journal.visible_at(time(10)).collect::<Vec<_>>();

        assert_eq!(before.len(), 1);
        assert_eq!(before[0].payload(), &GameJournalEntry::create_saucer());
        assert_eq!(at_target.len(), 2);
        assert_eq!(at_target[1].logical_time(), time(10));
    }

    #[test]
    fn equal_time_entries_keep_append_order() {
        let mut writer = JournalWriter::new();
        writer.advance_to(time(3)).expect("forward time is valid");
        writer.record(GameJournalEntry::SetTerrain {
            tile: TileId::origin(),
            terrain: Terrain::Wheat,
        });
        writer.record(GameJournalEntry::SetTerrain {
            tile: TileId::origin(),
            terrain: Terrain::Forest,
        });

        let journal = writer.finish();
        let entries = journal.iter().collect::<Vec<_>>();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].logical_time(), time(3));
        assert_eq!(entries[1].logical_time(), time(3));
        assert_eq!(
            entries[0].payload(),
            &GameJournalEntry::SetTerrain {
                tile: TileId::origin(),
                terrain: Terrain::Wheat,
            }
        );
        assert_eq!(
            entries[1].payload(),
            &GameJournalEntry::SetTerrain {
                tile: TileId::origin(),
                terrain: Terrain::Forest,
            }
        );
    }

    #[test]
    fn backward_time_is_explicit_and_does_not_move_the_cursor() {
        let mut writer = JournalWriter::new();
        writer.advance_to(time(8)).expect("forward time is valid");

        let error = writer
            .advance_to(time(7))
            .expect_err("backward time must fail");

        assert_eq!(
            error,
            JournalWriterError::BackwardTime {
                current: time(8),
                requested: time(7),
            }
        );
        assert_eq!(writer.current_time(), time(8));
        assert_eq!(writer.record(spawn(1)).logical_time(), time(8));
    }

    #[test]
    fn published_snapshots_are_not_changed_by_later_authoring() {
        let mut writer = JournalWriter::new();
        writer.record(GameJournalEntry::create_saucer());
        let earlier = writer.snapshot();

        writer.advance_to(time(5)).expect("forward time is valid");
        writer.record(spawn(1));

        assert_eq!(earlier.len(), 1);
        assert_eq!(writer.snapshot().len(), 2);
    }

    #[test]
    fn equal_advance_is_allowed_for_append_order() {
        let mut writer = JournalWriter::new();
        writer.advance_to(time(2)).expect("forward time is valid");

        assert_eq!(writer.advance_to(time(2)), Ok(()));
        assert_eq!(writer.current_time(), time(2));
    }
}
