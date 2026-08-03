use crate::{ActorId, ActorKind, Terrain, TileId, SAUCER_RADIUS};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GameJournalEntry {
    CreateSaucer {
        radius: u8,
    },
    SpawnActor {
        id: ActorId,
        kind: ActorKind,
        tile: TileId,
    },
    SetTerrain {
        tile: TileId,
        terrain: Terrain,
    },
}

impl GameJournalEntry {
    pub const fn create_saucer() -> Self {
        Self::CreateSaucer {
            radius: SAUCER_RADIUS,
        }
    }
}

pub type DomainJournalEntry = GameJournalEntry;

#[cfg(test)]
mod tests {
    use crate::{ActorId, ActorKind, Axial, Terrain, TileId, SAUCER_RADIUS};

    use super::GameJournalEntry;

    #[test]
    fn compact_entries_carry_domain_values_without_timestamps() {
        let tile = TileId::from_axial(Axial::new(0, 0)).expect("origin is inside the saucer");
        let actor_id = ActorId::new(11).expect("positive IDs are valid");

        assert_eq!(
            GameJournalEntry::create_saucer(),
            GameJournalEntry::CreateSaucer {
                radius: SAUCER_RADIUS
            }
        );
        assert_eq!(
            GameJournalEntry::SpawnActor {
                id: actor_id,
                kind: ActorKind::Farmer,
                tile,
            },
            GameJournalEntry::SpawnActor {
                id: actor_id,
                kind: ActorKind::Farmer,
                tile,
            }
        );
        assert_eq!(
            GameJournalEntry::SetTerrain {
                tile,
                terrain: Terrain::Wheat,
            },
            GameJournalEntry::SetTerrain {
                tile,
                terrain: Terrain::Wheat,
            }
        );
    }
}
