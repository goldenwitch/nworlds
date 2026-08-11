use std::{error::Error, fmt, sync::Arc};

use caravan_domain::GameJournalEntry;
use engine_journal::{Journal, JournalEntry, JournalWriter, JournalWriterError};
use engine_sdk::Context;
use engine_time::LogicalTime;

/// Identifies the role of one immutable branch value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BranchKind {
    /// The original journal supplied to the worldline.
    Actual,
    /// A branch with an alternate suffix after its retained prefix.
    Counterfactual,
    /// A branch whose suffix replaces the parent's suffix after its retained prefix.
    Corrected,
}

/// An error raised while constructing an immutable branch snapshot.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BranchError {
    /// A replacement or alternate suffix attempted to replace an inclusive prefix entry.
    SuffixNotAfterFork {
        fork_boundary: LogicalTime,
        entry_time: LogicalTime,
    },
    /// The source snapshot could not be replayed through the journal writer.
    JournalWriter(JournalWriterError),
}

impl fmt::Display for BranchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SuffixNotAfterFork {
                fork_boundary,
                entry_time,
            } => write!(
                formatter,
                "branch suffix entry at {entry_time} is not after fork boundary {fork_boundary}"
            ),
            Self::JournalWriter(error) => error.fmt(formatter),
        }
    }
}

impl Error for BranchError {}

impl From<JournalWriterError> for BranchError {
    fn from(error: JournalWriterError) -> Self {
        Self::JournalWriter(error)
    }
}

/// An immutable context and journal branch.
///
/// The context is shared by reference-counted ownership between a parent and
/// its children. Every child receives a newly written journal snapshot.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Branch<C> {
    context: Arc<Context<C>>,
    journal: Journal,
    kind: BranchKind,
    fork_boundary: Option<LogicalTime>,
}

/// The query-facing name for an immutable branch value.
pub type Worldline<C> = Branch<C>;

impl<C> Branch<C> {
    /// Creates the actual worldline from an immutable context and journal.
    pub fn new(context: Context<C>, journal: Journal) -> Self {
        Self {
            context: Arc::new(context),
            journal,
            kind: BranchKind::Actual,
            fork_boundary: None,
        }
    }

    /// Creates the actual worldline from an immutable context and journal.
    pub fn actual(context: Context<C>, journal: Journal) -> Self {
        Self::new(context, journal)
    }

    /// Borrows the immutable SDK context envelope.
    pub fn context(&self) -> &Context<C> {
        self.context.as_ref()
    }

    /// Borrows the opaque context payload.
    pub fn context_payload(&self) -> &C {
        self.context.payload()
    }

    /// Borrows this branch's immutable journal snapshot.
    pub fn journal(&self) -> &Journal {
        &self.journal
    }

    /// Returns whether this value is the original actual worldline.
    pub const fn is_actual(&self) -> bool {
        matches!(self.kind, BranchKind::Actual)
    }

    /// Returns whether this value was constructed as a child branch.
    pub const fn is_branch(&self) -> bool {
        !self.is_actual()
    }

    /// Returns this value's branch role.
    pub const fn kind(&self) -> BranchKind {
        self.kind
    }

    /// Returns the retained inclusive fork boundary, if this is a child branch.
    pub const fn fork_boundary(&self) -> Option<LogicalTime> {
        self.fork_boundary
    }

    /// Builds a counterfactual child from the inclusive parent prefix and a strict suffix.
    pub fn counterfactual(
        &self,
        fork_boundary: LogicalTime,
        suffix: &Journal,
    ) -> Result<Self, BranchError> {
        self.fork_with_suffix(fork_boundary, suffix, BranchKind::Counterfactual)
    }

    /// Builds a corrected child by replacing the parent suffix after the inclusive prefix.
    pub fn corrected_suffix(
        &self,
        fork_boundary: LogicalTime,
        replacement_suffix: &Journal,
    ) -> Result<Self, BranchError> {
        self.fork_with_suffix(fork_boundary, replacement_suffix, BranchKind::Corrected)
    }

    fn fork_with_suffix(
        &self,
        fork_boundary: LogicalTime,
        suffix: &Journal,
        kind: BranchKind,
    ) -> Result<Self, BranchError> {
        let journal = build_child_journal(&self.journal, fork_boundary, suffix)?;

        Ok(Self {
            context: Arc::clone(&self.context),
            journal,
            kind,
            fork_boundary: Some(fork_boundary),
        })
    }
}

fn build_child_journal(
    parent: &Journal,
    fork_boundary: LogicalTime,
    suffix: &Journal,
) -> Result<Journal, BranchError> {
    for entry in suffix.iter() {
        if entry.logical_time() <= fork_boundary {
            return Err(BranchError::SuffixNotAfterFork {
                fork_boundary,
                entry_time: entry.logical_time(),
            });
        }
    }

    let mut writer = JournalWriter::new();

    for entry in parent
        .iter()
        .filter(|entry| entry.logical_time() <= fork_boundary)
    {
        append_entry(&mut writer, entry)?;
    }

    for entry in suffix.iter() {
        append_entry(&mut writer, entry)?;
    }

    Ok(writer.finish())
}

fn append_entry(writer: &mut JournalWriter, entry: &JournalEntry) -> Result<(), BranchError> {
    writer.advance_to(entry.logical_time())?;
    let payload: GameJournalEntry = *entry.payload();
    writer.record(payload);
    Ok(())
}
