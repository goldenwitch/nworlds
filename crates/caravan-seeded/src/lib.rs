#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use caravan_domain::{ActorId, ActorKind, GameJournalEntry, Saucer, TileId, SAUCER_TILE_COUNT};
use engine_journal::{Journal, JournalWriter};
use engine_time::LogicalTime;

pub const SPAWN_PERIOD_GAME_TICKS: u64 = 10;
pub const ACTORS_PER_SPAWN: usize = 3;
pub const SEEDED_ACTOR_KIND: ActorKind = ActorKind::Farmer;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SeededJournalError {
    HorizonExceedsLogicalTime { horizon_game_ticks: u64 },
    TooManyActors { requested: usize, available: usize },
}

impl core::fmt::Display for SeededJournalError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::HorizonExceedsLogicalTime { horizon_game_ticks } => write!(
                formatter,
                "seeded journal horizon {horizon_game_ticks} exceeds the logical-time range"
            ),
            Self::TooManyActors {
                requested,
                available,
            } => write!(
                formatter,
                "seeded journal needs {requested} occupied tiles but the saucer has {available}"
            ),
        }
    }
}

impl std::error::Error for SeededJournalError {}

pub fn try_generate_spawn_journal(
    seed: u64,
    horizon_game_ticks: u64,
) -> Result<Journal, SeededJournalError> {
    if horizon_game_ticks > i64::MAX as u64
        || LogicalTime::from_game_ticks(horizon_game_ticks as i64).is_none()
    {
        return Err(SeededJournalError::HorizonExceedsLogicalTime { horizon_game_ticks });
    }

    let spawn_batches = horizon_game_ticks / SPAWN_PERIOD_GAME_TICKS;
    let requested_actors = spawn_batches.checked_mul(ACTORS_PER_SPAWN as u64).ok_or(
        SeededJournalError::TooManyActors {
            requested: usize::MAX,
            available: SAUCER_TILE_COUNT,
        },
    )?;

    if requested_actors > SAUCER_TILE_COUNT as u64 {
        return Err(SeededJournalError::TooManyActors {
            requested: usize::try_from(requested_actors).unwrap_or(usize::MAX),
            available: SAUCER_TILE_COUNT,
        });
    }

    let mut writer = JournalWriter::new();
    writer.record(GameJournalEntry::create_saucer());

    let mut rng = DeterministicRng::new(seed);
    let mut occupied = BTreeSet::new();
    let mut next_actor_id = 1_u64;

    for batch in 1..=spawn_batches {
        let spawn_time = batch * SPAWN_PERIOD_GAME_TICKS;
        writer
            .advance_to(
                LogicalTime::from_game_ticks(spawn_time as i64)
                    .expect("generated spawn times are representable"),
            )
            .expect("generated spawn times are monotonic");

        for _ in 0..ACTORS_PER_SPAWN {
            let tile = select_unoccupied_tile(&mut rng, &mut occupied)
                .expect("capacity was checked before generating the schedule");
            let actor_id = ActorId::new(next_actor_id).expect("generated actor IDs are positive");

            writer.record(GameJournalEntry::SpawnActor {
                id: actor_id,
                kind: SEEDED_ACTOR_KIND,
                tile,
            });
            next_actor_id += 1;
        }
    }

    Ok(writer.finish())
}

pub fn generate_spawn_journal(seed: u64, horizon_game_ticks: u64) -> Journal {
    try_generate_spawn_journal(seed, horizon_game_ticks)
        .expect("seeded journal parameters must fit the saucer and logical-time range")
}

pub fn hand_authored_behavior_fixture() -> Journal {
    let mut writer = JournalWriter::new();
    writer.record(GameJournalEntry::create_saucer());

    writer.record(GameJournalEntry::SetTerrain {
        tile: tile(1, 0),
        terrain: caravan_domain::Terrain::Wheat,
    });
    writer.record(GameJournalEntry::SetTerrain {
        tile: tile(2, 0),
        terrain: caravan_domain::Terrain::Forest,
    });
    writer.record(GameJournalEntry::SetTerrain {
        tile: tile(-1, 0),
        terrain: caravan_domain::Terrain::Wheat,
    });
    writer.record(GameJournalEntry::SetTerrain {
        tile: tile(-2, 1),
        terrain: caravan_domain::Terrain::Forest,
    });

    writer.record(GameJournalEntry::SpawnActor {
        id: actor_id(1),
        kind: ActorKind::Farmer,
        tile: tile(0, 0),
    });
    writer.record(GameJournalEntry::SpawnActor {
        id: actor_id(2),
        kind: ActorKind::Forester,
        tile: tile(2, 0),
    });
    writer.record(GameJournalEntry::SpawnActor {
        id: actor_id(3),
        kind: ActorKind::Arsonist,
        tile: tile(-2, 0),
    });
    writer.record(GameJournalEntry::SpawnActor {
        id: actor_id(4),
        kind: ActorKind::Fighter,
        tile: tile(-4, 0),
    });
    writer.record(GameJournalEntry::SpawnActor {
        id: actor_id(5),
        kind: ActorKind::Arborist,
        tile: tile(0, 3),
    });

    writer.finish()
}

fn select_unoccupied_tile(
    rng: &mut DeterministicRng,
    occupied: &mut BTreeSet<TileId>,
) -> Option<TileId> {
    let start = (rng.next_u64() % SAUCER_TILE_COUNT as u64) as usize;

    for retry in 0..SAUCER_TILE_COUNT {
        let index = (start + retry) % SAUCER_TILE_COUNT;
        let tile = Saucer::new().tiles()[index];

        if occupied.insert(tile) {
            return Some(tile);
        }
    }

    None
}

fn actor_id(value: u64) -> ActorId {
    ActorId::new(value).expect("behavior fixture actor IDs are positive")
}

fn tile(q: i32, r: i32) -> TileId {
    TileId::new(q, r).expect("behavior fixture coordinates are inside the saucer")
}

#[derive(Clone, Copy)]
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use caravan_domain::{ActorKind, GameJournalEntry, Saucer, SAUCER_TILE_COUNT};
    use engine_time::LogicalTime;

    use super::{
        generate_spawn_journal, hand_authored_behavior_fixture, select_unoccupied_tile,
        try_generate_spawn_journal, DeterministicRng, ACTORS_PER_SPAWN, SEEDED_ACTOR_KIND,
        SPAWN_PERIOD_GAME_TICKS,
    };

    const TEST_SEED: u64 = 0xC0FF_EE12_3456_7890;

    #[test]
    fn same_seed_reproduces_identical_ordered_entries() {
        let first = generate_spawn_journal(TEST_SEED, 50);
        let second = generate_spawn_journal(TEST_SEED, 50);

        let first_entries = first
            .iter()
            .map(|entry| (entry.logical_time(), *entry.payload()))
            .collect::<Vec<_>>();
        let second_entries = second
            .iter()
            .map(|entry| (entry.logical_time(), *entry.payload()))
            .collect::<Vec<_>>();

        assert_eq!(first_entries, second_entries);
    }

    #[test]
    fn generated_schedule_has_one_saucer_and_valid_unoccupied_spawns() {
        let horizon = 50;
        let journal = generate_spawn_journal(TEST_SEED, horizon);
        let entries = journal.iter().collect::<Vec<_>>();

        assert_eq!(
            entries.len(),
            1 + (horizon as usize / 10) * ACTORS_PER_SPAWN
        );
        assert_eq!(entries[0].logical_time(), LogicalTime::zero());
        assert_eq!(entries[0].payload(), &GameJournalEntry::create_saucer());
        assert_eq!(
            entries
                .iter()
                .filter(|entry| matches!(entry.payload(), GameJournalEntry::CreateSaucer { .. }))
                .count(),
            1
        );

        let saucer = Saucer::new();
        let mut occupied = BTreeSet::new();
        let mut actor_ids = BTreeSet::new();
        let mut entries_per_time = BTreeMap::new();

        for entry in entries.iter().skip(1) {
            let GameJournalEntry::SpawnActor { id, kind, tile, .. } = entry.payload() else {
                panic!("generated entries after CreateSaucer must be spawns");
            };

            let game_ticks = entry.logical_time().game_tick_index();
            assert!(game_ticks >= SPAWN_PERIOD_GAME_TICKS as i64);
            assert!(game_ticks <= horizon as i64);
            assert_eq!(game_ticks % SPAWN_PERIOD_GAME_TICKS as i64, 0);
            assert_eq!(*kind, SEEDED_ACTOR_KIND);
            assert!(saucer.tiles().contains(tile));
            assert!(occupied.insert(*tile));
            assert!(actor_ids.insert(*id));
            *entries_per_time.entry(game_ticks).or_insert(0) += 1;
        }

        assert_eq!(entries_per_time.len(), horizon as usize / 10);
        assert!(entries_per_time
            .values()
            .all(|&count| count == ACTORS_PER_SPAWN));
        assert_eq!(occupied.len(), entries.len() - 1);
        assert!(occupied.len() <= SAUCER_TILE_COUNT);
    }

    #[test]
    fn retries_skip_an_occupied_first_candidate_deterministically() {
        let seed = 7;
        let tiles = Saucer::new().tiles();
        let mut preview = DeterministicRng::new(seed);
        let first_index = (preview.next_u64() % SAUCER_TILE_COUNT as u64) as usize;
        let mut occupied = BTreeSet::from([tiles[first_index]]);
        let mut rng = DeterministicRng::new(seed);

        let selected = select_unoccupied_tile(&mut rng, &mut occupied)
            .expect("the saucer has another available tile");

        assert_eq!(selected, tiles[(first_index + 1) % SAUCER_TILE_COUNT]);
    }

    #[test]
    fn capacity_and_time_limits_are_explicit() {
        assert!(try_generate_spawn_journal(TEST_SEED, 300).is_ok());
        assert!(try_generate_spawn_journal(TEST_SEED, 310).is_err());
        assert!(try_generate_spawn_journal(TEST_SEED, i64::MAX as u64 + 1).is_err());
    }

    #[test]
    fn hand_authored_fixture_is_repeatable_and_covers_actor_kinds() {
        let first = hand_authored_behavior_fixture();
        let second = hand_authored_behavior_fixture();

        assert_eq!(first, second);
        assert_eq!(
            first.iter().next().unwrap().payload(),
            &GameJournalEntry::create_saucer()
        );

        let kinds = first
            .iter()
            .filter_map(|entry| match entry.payload() {
                GameJournalEntry::SpawnActor { kind, .. } => Some(*kind),
                _ => None,
            })
            .collect::<BTreeSet<ActorKind>>();

        assert_eq!(kinds.len(), 5);
    }
}
