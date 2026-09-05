use caravan_domain::GameJournalEntry;

use crate::engine_integration::{
    BranchError, CaravanJournal, CaravanJournalWriter, CaravanWorldline, JournalWriterError,
    LogicalTime,
};

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

/// Builds a counterfactual child with one accepted transformation after a fork.
pub fn publish_counterfactual(
    parent: &CaravanWorldline,
    fork_boundary: LogicalTime,
    authoring_time: LogicalTime,
    transformation: Transformation,
) -> Result<CaravanWorldline, PublicationError> {
    let suffix = single_entry_journal(authoring_time, transformation)?;
    Ok(parent.counterfactual(fork_boundary, &suffix)?)
}

/// Builds a corrected child with one accepted transformation after a fork.
pub fn publish_corrected(
    parent: &CaravanWorldline,
    fork_boundary: LogicalTime,
    authoring_time: LogicalTime,
    transformation: Transformation,
) -> Result<CaravanWorldline, PublicationError> {
    let suffix = single_entry_journal(authoring_time, transformation)?;
    Ok(parent.corrected_suffix(fork_boundary, &suffix)?)
}

fn single_entry_journal(
    authoring_time: LogicalTime,
    transformation: Transformation,
) -> Result<CaravanJournal, PublicationError> {
    let payload = journal_payload(transformation)?;
    let mut writer = CaravanJournalWriter::new();
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
    use super::{publish_corrected, PublicationError};
    use crate::engine_integration::{CaravanJournalWriter, LogicalTime};
    use crate::transformation::Transformation;
    use caravan_domain::{GameJournalEntry, Terrain, TileId};
    use caravan_reference::{actual, state};

    fn time(ticks: i64) -> LogicalTime {
        LogicalTime::from_game_ticks(ticks).expect("test time is representable")
    }

    fn parent() -> caravan_reference::ReferenceWorldline {
        let mut writer = CaravanJournalWriter::new();
        writer.record(GameJournalEntry::create_saucer());
        actual(writer.finish())
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
                crate::engine_integration::BranchError::SuffixNotAfterFork { .. }
            ))
        ));
    }
}
