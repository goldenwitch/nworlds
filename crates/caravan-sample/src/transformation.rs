use caravan_domain::{GameJournalEntry, Terrain, TileId};

/// The first closed set of game changes produced by Caravan interaction logic.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Transformation {
    Noop,
    SetTerrain { tile: TileId, terrain: Terrain },
}

impl Transformation {
    /// Converts an accepted transformation to an untimestamped game payload.
    pub const fn into_journal_entry(self) -> Option<GameJournalEntry> {
        match self {
            Self::Noop => None,
            Self::SetTerrain { tile, terrain } => {
                Some(GameJournalEntry::SetTerrain { tile, terrain })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Transformation;
    use caravan_domain::{GameJournalEntry, Terrain, TileId};

    #[test]
    fn accepted_transformation_becomes_a_domain_payload_without_time() {
        let transformation = Transformation::SetTerrain {
            tile: TileId::origin(),
            terrain: Terrain::Wheat,
        };

        assert_eq!(
            transformation.into_journal_entry(),
            Some(GameJournalEntry::SetTerrain {
                tile: TileId::origin(),
                terrain: Terrain::Wheat,
            })
        );
    }
}
