use caravan_domain::{ActorId, ActorKind, Effect, GameJournalEntry, Terrain, TileId};
use caravan_reference::{actual, state, ReferenceWorldline, Snapshot};
use caravan_seeded::{generate_spawn_journal, hand_authored_behavior_fixture};
use engine_journal::{Journal, JournalWriter};
use engine_time::LogicalTime;

fn time(ticks: i64) -> LogicalTime {
    LogicalTime::from_ticks(ticks)
}

fn tile(q: i32, r: i32) -> TileId {
    TileId::new(q, r).expect("frozen corpus coordinate is inside the saucer")
}

fn actor_id(value: u64) -> ActorId {
    ActorId::new(value).expect("frozen corpus actor IDs are positive")
}

fn journal(entries: &[(i64, GameJournalEntry)]) -> Journal {
    let mut writer = JournalWriter::new();
    for (ticks, entry) in entries {
        writer
            .advance_to(time(*ticks))
            .expect("frozen corpus timestamps are monotonic");
        writer.record(*entry);
    }
    writer.finish()
}

fn spawn(id: u64, kind: ActorKind, tile: TileId) -> GameJournalEntry {
    GameJournalEntry::SpawnActor {
        id: actor_id(id),
        kind,
        tile,
    }
}

fn terrain(tile: TileId, value: Terrain) -> GameJournalEntry {
    GameJournalEntry::SetTerrain {
        tile,
        terrain: value,
    }
}

struct ExpectedObservation<'a> {
    logical_time: i64,
    expected_saucer: bool,
    expected_tile_count: usize,
    expected_actor_ids: &'a [u64],
    expected_wheat: u64,
    expected_wood: u64,
    expected_terrain: Option<(TileId, Terrain)>,
    expected_effect: Option<(TileId, Effect)>,
}

macro_rules! assert_observation {
    (
        $worldline:expr,
        $logical_time:expr,
        $expected_saucer:expr,
        $expected_tile_count:expr,
        $expected_actor_ids:expr,
        $expected_wheat:expr,
        $expected_wood:expr,
        $expected_terrain:expr,
        $expected_effect:expr $(,)?
    ) => {
        assert_expected(
            $worldline,
            ExpectedObservation {
                logical_time: $logical_time,
                expected_saucer: $expected_saucer,
                expected_tile_count: $expected_tile_count,
                expected_actor_ids: $expected_actor_ids,
                expected_wheat: $expected_wheat,
                expected_wood: $expected_wood,
                expected_terrain: $expected_terrain,
                expected_effect: $expected_effect,
            },
        );
    };
}

fn assert_expected(worldline: &ReferenceWorldline, expected: ExpectedObservation<'_>) {
    let ExpectedObservation {
        logical_time,
        expected_saucer,
        expected_tile_count,
        expected_actor_ids,
        expected_wheat,
        expected_wood,
        expected_terrain,
        expected_effect,
    } = expected;
    let result = state(worldline, time(logical_time));
    let snapshot: &Snapshot = result.payload();
    let actor_ids = snapshot
        .actors()
        .iter()
        .map(|actor| actor.id().get())
        .collect::<Vec<_>>();

    assert_eq!(result.logical_time(), time(logical_time));
    assert_eq!(snapshot.has_saucer(), expected_saucer);
    assert_eq!(snapshot.tiles().len(), expected_tile_count);
    assert_eq!(actor_ids, expected_actor_ids);
    assert_eq!(
        snapshot.resources().wheat(),
        expected_wheat,
        "frozen corpus wheat at t_={logical_time}"
    );
    assert_eq!(
        snapshot.resources().wood(),
        expected_wood,
        "frozen corpus wood at t_={logical_time}"
    );

    if let Some((tile, terrain)) = expected_terrain {
        assert_eq!(snapshot.terrain_at(tile), Some(terrain));
    }
    if let Some((tile, effect)) = expected_effect {
        assert_eq!(snapshot.effect_at(tile), Some(effect));
    }
}

fn actor_tile(snapshot: &Snapshot, id: u64) -> TileId {
    snapshot
        .actors()
        .iter()
        .find(|actor| actor.id() == actor_id(id))
        .expect("frozen corpus actor is present")
        .tile()
}

#[test]
fn frozen_expected_corpus_matches_the_projection() {
    let empty = actual(Journal::empty());
    assert_observation!(&empty, 7, false, 0, &[], 0, 0, None, None);

    let created = actual(journal(&[(0, GameJournalEntry::create_saucer())]));
    assert_observation!(
        &created,
        0,
        true,
        91,
        &[],
        0,
        0,
        Some((TileId::origin(), Terrain::Void)),
        None,
    );

    let hand_authored = actual(hand_authored_behavior_fixture());
    assert_observation!(&hand_authored, -1, false, 0, &[], 0, 0, None, None);
    assert_observation!(
        &hand_authored,
        0,
        true,
        91,
        &[1, 2, 3, 4, 5],
        2,
        1,
        Some((tile(1, 0), Terrain::Wheat)),
        None,
    );
    assert_observation!(
        &hand_authored,
        499,
        true,
        91,
        &[1, 2, 3, 4, 5],
        2,
        1,
        Some((tile(1, 0), Terrain::Wheat)),
        None,
    );
    assert_observation!(
        &hand_authored,
        1_000,
        true,
        91,
        &[2, 4, 5],
        9,
        2,
        Some((tile(1, 0), Terrain::Wheat)),
        Some((tile(-1, 0), Effect::fire(0))),
    );
    assert_observation!(
        &hand_authored,
        4_000,
        true,
        91,
        &[2, 4, 5],
        29,
        5,
        Some((tile(-1, 0), Terrain::Void)),
        Some((tile(-1, 0), Effect::None)),
    );

    let inside_tick = actual(journal(&[
        (0, GameJournalEntry::create_saucer()),
        (500, spawn(1, ActorKind::Forester, TileId::origin())),
        (500, terrain(TileId::origin(), Terrain::Forest)),
    ]));
    assert_observation!(
        &inside_tick,
        500,
        true,
        91,
        &[1],
        0,
        1,
        Some((TileId::origin(), Terrain::Forest)),
        None,
    );
    assert_observation!(
        &inside_tick,
        999,
        true,
        91,
        &[1],
        0,
        1,
        Some((TileId::origin(), Terrain::Forest)),
        None,
    );
    assert_observation!(
        &inside_tick,
        1_000,
        true,
        91,
        &[1],
        0,
        1,
        Some((TileId::origin(), Terrain::Forest)),
        None,
    );

    let authored_before_derived = actual(journal(&[
        (0, GameJournalEntry::create_saucer()),
        (0, spawn(1, ActorKind::Farmer, TileId::origin())),
        (500, terrain(tile(2, 0), Terrain::Forest)),
    ]));
    assert_observation!(
        &authored_before_derived,
        1_000,
        true,
        91,
        &[],
        6,
        0,
        Some((tile(2, 0), Terrain::Wheat)),
        None,
    );

    let collective = actual(journal(&[
        (0, GameJournalEntry::create_saucer()),
        (0, spawn(1, ActorKind::Forester, TileId::origin())),
        (0, spawn(2, ActorKind::Forester, tile(0, 1))),
        (0, spawn(3, ActorKind::Arborist, tile(1, 1))),
    ]));
    let collective_at_one = state(&collective, time(1_000));
    assert_eq!(actor_tile(collective_at_one.payload(), 1), tile(1, 0));
    assert_eq!(actor_tile(collective_at_one.payload(), 2), tile(0, 1));
    let collective_at_two = state(&collective, time(2_000));
    assert_eq!(actor_tile(collective_at_two.payload(), 1), tile(2, 0));
    assert_eq!(actor_tile(collective_at_two.payload(), 2), TileId::origin());

    let seeded = actual(generate_spawn_journal(0xCAFE, 20));
    assert_observation!(&seeded, 9_999, true, 91, &[], 0, 0, None, None);
    assert_observation!(&seeded, 10_000, true, 91, &[1, 2, 3], 0, 0, None, None);
    assert_observation!(&seeded, 20_000, true, 91, &[4, 5, 6], 160, 0, None, None);

    let parent = actual(journal(&[
        (0, GameJournalEntry::create_saucer()),
        (500, terrain(TileId::origin(), Terrain::Wheat)),
        (1_000, spawn(1, ActorKind::Farmer, tile(1, 0))),
    ]));
    let alternate = journal(&[(1_500, spawn(2, ActorKind::Forester, tile(-1, 0)))]);
    let replacement = journal(&[(1_500, spawn(3, ActorKind::Arborist, tile(0, 1)))]);
    let counterfactual = parent
        .counterfactual(time(1_000), &alternate)
        .expect("frozen counterfactual suffix is after the fork");
    let corrected = parent
        .corrected_suffix(time(1_000), &replacement)
        .expect("frozen corrected suffix is after the fork");

    assert_observation!(
        &parent,
        1_500,
        true,
        91,
        &[1],
        1,
        0,
        Some((TileId::origin(), Terrain::Wheat)),
        None,
    );
    assert_observation!(
        &counterfactual,
        1_500,
        true,
        91,
        &[1, 2],
        1,
        0,
        Some((TileId::origin(), Terrain::Wheat)),
        None,
    );
    assert_observation!(
        &corrected,
        1_500,
        true,
        91,
        &[1, 3],
        1,
        0,
        Some((TileId::origin(), Terrain::Wheat)),
        None,
    );
}
