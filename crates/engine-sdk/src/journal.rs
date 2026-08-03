use engine_time::LogicalTime;

/// An immutable journal fact with its assigned logical time.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct JournalEntry<P> {
    logical_time: LogicalTime,
    payload: P,
}

impl<P> JournalEntry<P> {
    /// Wraps an opaque payload at an already selected logical time.
    pub fn new(logical_time: LogicalTime, payload: P) -> Self {
        Self {
            logical_time,
            payload,
        }
    }

    /// Returns the entry's exact logical time.
    pub fn logical_time(&self) -> LogicalTime {
        self.logical_time
    }

    /// Borrows the opaque journal payload.
    pub fn payload(&self) -> &P {
        &self.payload
    }

    /// Consumes the envelope and returns its payload.
    pub fn into_payload(self) -> P {
        self.payload
    }
}

/// An immutable append-ordered sequence of journal entries.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Journal<P> {
    entries: Vec<JournalEntry<P>>,
}

impl<P> Journal<P> {
    /// Creates an empty journal.
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Creates a journal while preserving the supplied append order.
    pub fn from_entries(entries: impl IntoIterator<Item = JournalEntry<P>>) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }

    /// Returns the number of entries in append order.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Reports whether the journal contains no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Borrows entries in append order.
    pub fn iter(&self) -> impl Iterator<Item = &JournalEntry<P>> + '_ {
        self.entries.iter()
    }

    /// Borrows the entry at an append-order index.
    pub fn get(&self, index: usize) -> Option<&JournalEntry<P>> {
        self.entries.get(index)
    }

    /// Borrows entries visible at or before the requested logical time.
    pub fn visible_at(
        &self,
        logical_time: LogicalTime,
    ) -> impl Iterator<Item = &JournalEntry<P>> + '_ {
        self.entries
            .iter()
            .filter(move |entry| entry.logical_time() <= logical_time)
    }

    /// Consumes the journal and returns its entries in append order.
    pub fn into_entries(self) -> Vec<JournalEntry<P>> {
        self.entries
    }
}
