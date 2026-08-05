use caravan_domain::GameJournalEntry;
use caravan_reference::{ReferenceWorldline, Worldline};
use engine_branches::BranchError;
use engine_journal::{Journal, JournalWriter, JournalWriterError};
use engine_time::LogicalTime;

use crate::transformation::Transformation;

/// Errors raised while publishing a transformation through immutable values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationError {
    NoJournalEntry,
    JournalWriter(JournalWriterError),
    Branch(BranchError),
}

impl From<JournalWriterError> for PublicationError {
    fn from(error: JournalWriterError) -> Self {
        Self::JournalWriter(error)
    }
}

impl From<BranchError> for PublicationError {
    fn from(error: BranchError) -> Self {
        Self::Branch(error)
    }
}

/// Appends an accepted transformation to an actual worldline as a new value.
pub fn publish_actual_append(
    parent: &ReferenceWorldline,
    authoring_time: LogicalTime,
    transformation: Transformation,
) -> Result<ReferenceWorldline, PublicationError> {
    let payload = journal_payload(transformation)?;
    let mut writer = writer_from_journal(parent.journal())?;
    writer.advance_to(authoring_time)?;
    writer.record(payload);

    Ok(Worldline::new(parent.context().clone(), writer.finish()))
}

/// Builds a counterfactual child with one accepted transformation after a fork.
pub fn publish_counterfactual(
    parent: &ReferenceWorldline,
    fork_boundary: LogicalTime,
    authoring_time: LogicalTime,
    transformation: Transformation,
) -> Result<ReferenceWorldline, PublicationError> {
    let suffix = single_entry_journal(authoring_time, transformation)?;
    Ok(parent.counterfactual(fork_boundary, &suffix)?)
}

/// Builds a corrected child with one accepted transformation after a fork.
pub fn publish_corrected(
    parent: &ReferenceWorldline,
    fork_boundary: LogicalTime,
    authoring_time: LogicalTime,
    transformation: Transformation,
) -> Result<ReferenceWorldline, PublicationError> {
    let suffix = single_entry_journal(authoring_time, transformation)?;
    Ok(parent.corrected_suffix(fork_boundary, &suffix)?)
}

/// Reconstructs a mutable authoring cursor from one immutable journal value.
pub fn writer_from_journal(journal: &Journal) -> Result<JournalWriter, JournalWriterError> {
    let mut writer = JournalWriter::new();
    for entry in journal.iter() {
        writer.advance_to(entry.logical_time())?;
        writer.record(*entry.payload());
    }
    Ok(writer)
}

fn single_entry_journal(
    authoring_time: LogicalTime,
    transformation: Transformation,
) -> Result<Journal, PublicationError> {
    let payload = journal_payload(transformation)?;
    let mut writer = JournalWriter::new();
    writer.advance_to(authoring_time)?;
    writer.record(payload);
    Ok(writer.finish())
}

fn journal_payload(transformation: Transformation) -> Result<GameJournalEntry, PublicationError> {
    transformation
        .into_journal_entry()
        .ok_or(PublicationError::NoJournalEntry)
}

#[cfg(test)]
mod tests {
    use super::{publish_actual_append, publish_corrected, PublicationError};
    use crate::transformation::Transformation;
    use caravan_domain::{GameJournalEntry, Terrain, TileId};
    use caravan_reference::{actual, state};
    use engine_journal::JournalWriter;
    use engine_time::LogicalTime;

    fn time(ticks: i64) -> LogicalTime {
        LogicalTime::from_game_ticks(ticks).expect("test time is representable")
    }

    fn parent() -> caravan_reference::ReferenceWorldline {
        let mut writer = JournalWriter::new();
        writer.record(GameJournalEntry::create_saucer());
        actual(writer.finish())
    }

    #[test]
    fn actual_publication_creates_visible_value_without_mutating_parent() {
        let parent = parent();
        let published = publish_actual_append(
            &parent,
            time(4),
            Transformation::SetTerrain {
                tile: TileId::origin(),
                terrain: Terrain::Wheat,
            },
        )
        .expect("forward authoring should publish");

        assert_eq!(parent.journal().len(), 1);
        assert_eq!(published.journal().len(), 2);
        assert_eq!(
            state(&parent, time(4))
                .payload()
                .terrain_at(TileId::origin()),
            Some(Terrain::Void)
        );
        assert_eq!(
            state(&published, time(4))
                .payload()
                .terrain_at(TileId::origin()),
            Some(Terrain::Wheat)
        );
        assert_eq!(published.journal().get(1).unwrap().logical_time(), time(4));
    }

    #[test]
    fn corrected_publication_keeps_prefix_and_rejects_boundary_timestamp() {
        let parent = parent();
        let corrected = publish_corrected(
            &parent,
            time(3),
            time(4),
            Transformation::SetTerrain {
                tile: TileId::origin(),
                terrain: Terrain::Forest,
            },
        )
        .expect("strict suffix should publish");
        assert_eq!(corrected.fork_boundary(), Some(time(3)));
        assert_eq!(state(&corrected, time(3)), state(&parent, time(3)));

        let rejected = publish_corrected(
            &parent,
            time(3),
            time(3),
            Transformation::SetTerrain {
                tile: TileId::origin(),
                terrain: Terrain::Forest,
            },
        );
        assert!(matches!(
            rejected,
            Err(PublicationError::Branch(
                engine_branches::BranchError::SuffixNotAfterFork { .. }
            ))
        ));
    }
}
