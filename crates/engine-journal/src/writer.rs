use core::fmt;

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
pub struct JournalWriter<P> {
    current_time: Option<LogicalTime>,
    entries: Vec<JournalEntry<P>>,
}

impl<P> JournalWriter<P> {
    /// Creates an empty writer whose implicit record time is logical zero.
    pub fn new() -> Self {
        Self {
            current_time: None,
            entries: Vec::new(),
        }
    }

    /// Returns the writer's current timestamp cursor.
    pub const fn current_time(&self) -> LogicalTime {
        match self.current_time {
            Some(current_time) => current_time,
            None => LogicalTime::zero(),
        }
    }

    /// Moves the timestamp cursor forward or leaves it at the same time.
    pub fn advance_to(&mut self, target_time: LogicalTime) -> Result<(), JournalWriterError> {
        if self
            .current_time
            .is_some_and(|current_time| target_time < current_time)
        {
            return Err(JournalWriterError::BackwardTime {
                current: self.current_time(),
                requested: target_time,
            });
        }

        self.current_time = Some(target_time);
        Ok(())
    }

    /// Finalizes the writer into an immutable journal.
    pub fn finish(self) -> Journal<P> {
        Journal::from_entries(self.entries)
    }
}

impl<P: Clone> JournalWriter<P> {
    /// Records a payload at the current writer time.
    pub fn record(&mut self, payload: P) -> JournalEntry<P> {
        let current_time = self.current_time();
        self.current_time = Some(current_time);
        let entry = JournalEntry::from_assigned_time(current_time, payload);
        self.entries.push(entry.clone());
        entry
    }
}

impl<P: Clone> JournalWriter<P> {
    /// Publishes an immutable snapshot while retaining this writer.
    pub fn snapshot(&self) -> Journal<P> {
        Journal::from_entries(self.entries.iter().cloned())
    }
}

impl<P> Default for JournalWriter<P> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use engine_time::LogicalTime;

    use super::{JournalWriter, JournalWriterError};

    fn time(ticks: i64) -> LogicalTime {
        LogicalTime::from_ticks(ticks)
    }

    #[test]
    fn record_assigns_the_current_time_to_the_sdk_envelope() {
        let mut writer = JournalWriter::new();
        writer.advance_to(time(4)).expect("forward time is valid");

        let recorded = writer.record(1_u8);

        assert_eq!(recorded.logical_time(), time(4));
        assert_eq!(recorded.payload(), &1);
    }

    #[test]
    fn postdated_entries_are_hidden_until_their_target_time() {
        let mut writer = JournalWriter::new();
        writer.advance_to(time(0)).expect("zero is the anchor time");
        writer.record(1_u8);
        writer
            .advance_to(time(10))
            .expect("postdating is forward time");
        writer.record(2_u8);

        let journal = writer.finish();
        let before = journal.visible_at(time(9)).collect::<Vec<_>>();
        let at_target = journal.visible_at(time(10)).collect::<Vec<_>>();

        assert_eq!(before.len(), 1);
        assert_eq!(before[0].payload(), &1);
        assert_eq!(at_target.len(), 2);
        assert_eq!(at_target[1].logical_time(), time(10));
    }

    #[test]
    fn equal_time_entries_keep_append_order() {
        let mut writer = JournalWriter::new();
        writer.advance_to(time(3)).expect("forward time is valid");
        writer.record(1_u8);
        writer.record(2_u8);

        let journal = writer.finish();
        let entries = journal.iter().collect::<Vec<_>>();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].logical_time(), time(3));
        assert_eq!(entries[1].logical_time(), time(3));
        assert_eq!(entries[0].payload(), &1);
        assert_eq!(entries[1].payload(), &2);
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
        assert_eq!(writer.record(1_u8).logical_time(), time(8));
    }

    #[test]
    fn published_snapshots_are_not_changed_by_later_authoring() {
        let mut writer = JournalWriter::new();
        writer.record(1_u8);
        let earlier = writer.snapshot();

        writer.advance_to(time(5)).expect("forward time is valid");
        writer.record(2_u8);

        assert_eq!(earlier.len(), 1);
        assert_eq!(writer.snapshot().len(), 2);
    }

    #[test]
    fn equal_advance_is_allowed_for_append_order() {
        let mut writer = JournalWriter::<u8>::new();
        writer.advance_to(time(2)).expect("forward time is valid");

        assert_eq!(writer.advance_to(time(2)), Ok(()));
        assert_eq!(writer.current_time(), time(2));
    }

    #[test]
    fn first_explicit_timestamp_may_be_negative() {
        let mut writer = JournalWriter::new();
        writer
            .advance_to(time(-1))
            .expect("the first timestamp may precede logical zero");

        let entry = writer.record(1_u8);

        assert_eq!(entry.logical_time(), time(-1));
        assert_eq!(writer.current_time(), time(-1));
        assert!(writer.advance_to(time(-2)).is_err());
    }
}
