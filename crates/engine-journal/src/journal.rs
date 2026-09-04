use engine_sdk::{Journal as SdkJournal, JournalEntry};
use engine_time::LogicalTime;

/// An immutable, append-ordered journal of game entries.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Journal<P> {
    storage: SdkJournal<P>,
}

impl<P> Journal<P> {
    /// Creates an empty journal.
    pub fn empty() -> Self {
        Self {
            storage: SdkJournal::empty(),
        }
    }

    /// Returns the number of recorded entries.
    pub fn len(&self) -> usize {
        self.storage.len()
    }

    /// Reports whether no entries have been recorded.
    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    /// Borrows entries in their append order.
    pub fn iter(&self) -> impl Iterator<Item = &JournalEntry<P>> + '_ {
        self.storage.iter()
    }

    /// Borrows an entry at an append-order index.
    pub fn get(&self, index: usize) -> Option<&JournalEntry<P>> {
        self.storage.get(index)
    }

    /// Borrows entries whose assigned time is at or before the target time.
    pub fn visible_at(
        &self,
        target_time: LogicalTime,
    ) -> impl Iterator<Item = &JournalEntry<P>> + '_ {
        self.storage.visible_at(target_time)
    }

    pub(crate) fn from_entries(entries: impl IntoIterator<Item = JournalEntry<P>>) -> Self {
        Self {
            storage: SdkJournal::from_assigned_entries(entries),
        }
    }
}

impl<P> Default for Journal<P> {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::Journal;
    use engine_time::LogicalTime;

    #[test]
    fn empty_journal_has_no_visible_entries() {
        let journal = Journal::<()>::empty();

        assert!(journal.is_empty());
        assert_eq!(journal.len(), 0);
        assert_eq!(journal.iter().count(), 0);
        assert_eq!(journal.visible_at(LogicalTime::zero()).count(), 0);
    }
}
