use caravan_domain::{ActorId, ActorKind, Effect, GameJournalEntry, Terrain, TileId};
use caravan_reference::{actual, state, Snapshot};
use caravan_seeded::generate_spawn_journal;
use engine_branches::BranchKind;
use engine_journal::{Journal, JournalWriter};
use engine_time::LogicalTime;

fn time(ticks: i64) -> LogicalTime {
    LogicalTime::from_game_ticks(ticks).expect("test game-tick times are representable")
}

fn tile(q: i32, r: i32) -> TileId {
    TileId::new(q, r).expect("test coordinate is inside the saucer")
}

fn actor_id(value: u64) -> ActorId {
    ActorId::new(value).expect("test actor IDs are positive")
}

fn journal(entries: &[(i64, GameJournalEntry)]) -> Journal {
    let mut writer = JournalWriter::new();
    for (ticks, entry) in entries {
        writer
            .advance_to(time(*ticks))
            .expect("test journal timestamps are monotonic");
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

fn snapshot_at(entries: &[(i64, GameJournalEntry)], ticks: i64) -> Snapshot {
    let worldline = actual(journal(entries));
    state(&worldline, time(ticks)).into_payload()
}

#[test]
fn empty_journal_is_an_empty_set_with_exact_time() {
    let state = state(&actual(Journal::empty()), time(7));
    let snapshot = state.payload();

    assert_eq!(state.logical_time(), time(7));
    assert!(!snapshot.has_saucer());
    assert!(snapshot.tiles().is_empty());
    assert!(snapshot.actors().is_empty());
    assert_eq!(snapshot.resources().wheat(), 0);
    assert_eq!(snapshot.resources().wood(), 0);
}

#[test]
fn create_saucer_has_ninety_one_void_tiles_and_empty_layers() {
    let snapshot = snapshot_at(&[(0, GameJournalEntry::create_saucer())], 0);

    assert!(snapshot.has_saucer());
    assert_eq!(snapshot.tiles().len(), 91);
    assert!(snapshot.tiles().iter().all(|tile| {
        tile.layers().terrain() == Terrain::Void
            && tile.layers().actor().is_none()
            && tile.layers().effect() == caravan_domain::Effect::None
    }));
}

#[test]
fn postdated_entries_are_hidden_before_timestamp_and_visible_at_target() {
    let entries = [
        (0, GameJournalEntry::create_saucer()),
        (10, spawn(1, ActorKind::Farmer, tile(0, 0))),
    ];
    let before = snapshot_at(&entries, 9);
    let at_target = snapshot_at(&entries, 10);

    assert!(before.actors().is_empty());
    assert_eq!(at_target.actors().len(), 1);
    assert_eq!(at_target.actors()[0].id(), actor_id(1));
}

#[test]
fn farmer_disappears_and_places_wheat_around_its_selected_destination() {
    let entries = [
        (0, GameJournalEntry::create_saucer()),
        (0, spawn(1, ActorKind::Farmer, tile(0, 0))),
    ];
    let initial = snapshot_at(&entries, 0);
    let next = snapshot_at(&entries, 1);

    assert_eq!(initial.actors().len(), 1);
    assert!(next.actors().is_empty());
    assert_eq!(next.terrain_at(tile(2, 0)), Some(Terrain::Wheat));
    assert_eq!(next.terrain_at(tile(0, 0)), Some(Terrain::Wheat));
    assert_eq!(next.terrain_at(tile(1, 1)), Some(Terrain::Wheat));
}

#[test]
fn wheat_is_counted_as_an_indexed_resource_without_query_carryover() {
    let entries = [
        (0, GameJournalEntry::create_saucer()),
        (0, terrain(TileId::origin(), Terrain::Wheat)),
    ];
    let at_zero = snapshot_at(&entries, 0);
    let at_two = snapshot_at(&entries, 2);

    assert_eq!(at_zero.resources().wheat(), 1);
    assert_eq!(at_two.resources().wheat(), 3);
    assert_eq!(at_two.resources().wood(), 0);
}

#[test]
fn forester_moves_to_forest_then_produces_wood() {
    let entries = [
        (0, GameJournalEntry::create_saucer()),
        (0, terrain(tile(1, 0), Terrain::Forest)),
        (0, spawn(1, ActorKind::Forester, TileId::origin())),
    ];
    let at_zero = snapshot_at(&entries, 0);
    let at_one = snapshot_at(&entries, 1);
    let at_two = snapshot_at(&entries, 2);

    assert_eq!(at_zero.actors()[0].tile(), TileId::origin());
    assert_eq!(at_one.actors()[0].tile(), tile(1, 0));
    assert_eq!(at_one.resources().wood(), 1);
    assert_eq!(at_two.actors()[0].tile(), tile(1, 0));
    assert_eq!(at_two.resources().wood(), 2);
}

#[test]
fn arsonist_fire_ages_spreads_and_removes_burnable_terrain() {
    let entries = [
        (0, GameJournalEntry::create_saucer()),
        (0, terrain(tile(1, 0), Terrain::Wheat)),
        (0, terrain(tile(1, -1), Terrain::Forest)),
        (0, terrain(tile(2, 0), Terrain::Wheat)),
        (0, spawn(1, ActorKind::Arsonist, TileId::origin())),
        (0, spawn(2, ActorKind::Arborist, tile(4, 0))),
    ];
    let at_one = snapshot_at(&entries, 1);
    let at_two = snapshot_at(&entries, 2);
    let at_three = snapshot_at(&entries, 3);
    let at_four = snapshot_at(&entries, 4);

    assert!(at_one
        .actors()
        .iter()
        .all(|actor| actor.id() != actor_id(1)));
    assert_eq!(at_one.effect_at(tile(1, 0)), Some(Effect::fire(0)));
    assert_eq!(at_two.effect_at(tile(1, 0)), Some(Effect::fire(1)));
    assert_eq!(at_three.effect_at(tile(1, 0)), Some(Effect::fire(2)));
    assert_eq!(at_four.terrain_at(tile(1, 0)), Some(Terrain::Void));
    assert_eq!(at_four.effect_at(tile(1, 0)), Some(Effect::None));
    assert_eq!(at_four.effect_at(tile(2, 0)), Some(Effect::fire(0)));
}

#[test]
fn fighter_moves_into_and_removes_the_selected_arsonist() {
    let entries = [
        (0, GameJournalEntry::create_saucer()),
        (0, spawn(1, ActorKind::Fighter, TileId::origin())),
        (0, spawn(2, ActorKind::Arsonist, tile(1, 0))),
    ];
    let next = snapshot_at(&entries, 1);

    assert_eq!(next.actors().len(), 1);
    assert_eq!(next.actors()[0].id(), actor_id(1));
    assert_eq!(next.actors()[0].tile(), tile(1, 0));
}

#[test]
fn arborist_converts_its_tile_on_the_fourth_indexed_snapshot() {
    let entries = [
        (0, GameJournalEntry::create_saucer()),
        (0, terrain(TileId::origin(), Terrain::Wheat)),
        (0, spawn(1, ActorKind::Arborist, TileId::origin())),
    ];
    let at_two = snapshot_at(&entries, 2);
    let at_three = snapshot_at(&entries, 3);

    assert_eq!(at_two.terrain_at(TileId::origin()), Some(Terrain::Wheat));
    assert_eq!(at_three.terrain_at(TileId::origin()), Some(Terrain::Forest));
    assert_eq!(at_three.actors()[0].id(), actor_id(1));
}

#[test]
fn seeded_journals_and_oracle_states_repeat_for_the_same_seed() {
    let first_journal = generate_spawn_journal(0xCAFE, 20);
    let second_journal = generate_spawn_journal(0xCAFE, 20);
    let first_worldline = actual(first_journal.clone());
    let second_worldline = actual(second_journal.clone());

    assert_eq!(first_journal, second_journal);
    assert_eq!(
        state(&first_worldline, time(10)),
        state(&second_worldline, time(10))
    );
    assert_eq!(
        state(&first_worldline, time(20)),
        state(&second_worldline, time(20))
    );
}

#[test]
fn actual_counterfactual_and_corrected_branches_share_the_same_query() {
    let parent = actual(journal(&[
        (0, GameJournalEntry::create_saucer()),
        (
            5,
            GameJournalEntry::SetTerrain {
                tile: tile(0, 0),
                terrain: Terrain::Wheat,
            },
        ),
        (10, spawn(1, ActorKind::Farmer, tile(1, 0))),
    ]));
    let alternate = journal(&[(7, spawn(2, ActorKind::Forester, tile(-1, 0)))]);
    let replacement = journal(&[(6, spawn(3, ActorKind::Arborist, tile(0, 1)))]);
    let counterfactual = parent
        .counterfactual(time(5), &alternate)
        .expect("counterfactual suffix is after its boundary");
    let corrected = parent
        .corrected_suffix(time(5), &replacement)
        .expect("corrected suffix is after its boundary");

    let actual_state = state(&parent, time(10));
    let counterfactual_state = state(&counterfactual, time(10));
    let corrected_state = state(&corrected, time(10));

    assert_eq!(parent.kind(), BranchKind::Actual);
    assert_eq!(counterfactual.kind(), BranchKind::Counterfactual);
    assert_eq!(corrected.kind(), BranchKind::Corrected);
    assert_eq!(actual_state.logical_time(), time(10));
    assert_eq!(counterfactual_state.logical_time(), time(10));
    assert_eq!(corrected_state.logical_time(), time(10));
    assert_eq!(actual_state.payload().actors().len(), 1);
    assert_eq!(counterfactual_state.payload().actors().len(), 1);
    assert_eq!(corrected_state.payload().actors().len(), 1);
    assert_eq!(actual_state.payload().actors()[0].id(), actor_id(1));
    assert_eq!(counterfactual_state.payload().actors()[0].id(), actor_id(2));
    assert_eq!(corrected_state.payload().actors()[0].id(), actor_id(3));
    assert_eq!(parent.journal().len(), 3);
    assert_eq!(counterfactual.journal().len(), 3);
    assert_eq!(corrected.journal().len(), 3);
}

#[test]
fn non_monotonic_queries_are_repeatable_and_do_not_share_current_state() {
    let worldline = actual(journal(&[
        (0, GameJournalEntry::create_saucer()),
        (3, spawn(1, ActorKind::Forester, tile(0, 0))),
    ]));

    let later_first = state(&worldline, time(5));
    let earlier = state(&worldline, time(2));
    let later_again = state(&worldline, time(5));

    assert_eq!(later_first, later_again);
    assert!(earlier.payload().actors().is_empty());
    assert_eq!(later_first.payload().actors().len(), 1);
    assert_eq!(later_first.logical_time(), time(5));
    assert_eq!(earlier.logical_time(), time(2));
}
