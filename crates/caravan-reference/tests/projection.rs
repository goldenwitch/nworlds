use caravan_domain::{ActorId, ActorKind, GameJournalEntry, Terrain, TileId};
use caravan_reference::{
    actual, discontinuity_index, project, project_with_index, ActorThreshold,
    CaravanBreakpointSource, RuleThreshold,
};
use caravan_seeded::{generate_spawn_journal, hand_authored_behavior_fixture};
use engine_branches::BranchKind;
use engine_index::BreakpointSource as EngineBreakpointSource;
use engine_journal::{Journal, JournalWriter};
use engine_sdk::Context;
use engine_time::LogicalTime;

fn time(ticks: i64) -> LogicalTime {
    LogicalTime::from_ticks(ticks)
}

fn game_time(ticks: i64) -> LogicalTime {
    LogicalTime::from_game_ticks(ticks).expect("test game-tick time is representable")
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

#[test]
fn projection_emits_independent_anchor_observations() {
    let worldline = actual(hand_authored_behavior_fixture());

    let before_creation = project(&worldline, time(-1));
    assert_eq!(before_creation.logical_time(), time(-1));
    assert!(!before_creation.payload().has_saucer());

    let at_zero = project(&worldline, time(0));
    assert_eq!(at_zero.payload().tiles().len(), 91);
    assert_eq!(at_zero.payload().actors().len(), 5);
    assert_eq!(
        at_zero.payload().terrain_at(tile(1, 0)),
        Some(Terrain::Wheat)
    );
    assert_eq!(at_zero.payload().resources().wheat(), 2);
    assert_eq!(at_zero.payload().resources().wood(), 1);

    let at_first_tick = project(&worldline, game_time(1));
    assert_eq!(at_first_tick.payload().actors().len(), 3);
    assert_eq!(
        at_first_tick.payload().effect_at(tile(-1, 0)),
        Some(caravan_domain::Effect::fire(0))
    );
    assert_eq!(
        at_first_tick.payload().terrain_at(tile(1, 0)),
        Some(Terrain::Wheat)
    );

    let at_fourth_tick = project(&worldline, game_time(4));
    assert_eq!(
        at_fourth_tick.payload().terrain_at(tile(-1, 0)),
        Some(Terrain::Void)
    );
    assert_eq!(
        at_fourth_tick.payload().effect_at(tile(-1, 0)),
        Some(caravan_domain::Effect::None)
    );
}

#[test]
fn projection_handles_seeded_piece_boundaries_without_a_baseline_query() {
    let worldline = actual(generate_spawn_journal(0xCAFE, 20));

    assert!(project(&worldline, time(-1)).payload().actors().is_empty());
    assert!(project(&worldline, time(9_999))
        .payload()
        .actors()
        .is_empty());
    assert_eq!(
        project(&worldline, game_time(10))
            .payload()
            .actors()
            .iter()
            .map(|actor| actor.id().get())
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(
        project(&worldline, game_time(20))
            .payload()
            .actors()
            .iter()
            .map(|actor| actor.id().get())
            .collect::<Vec<_>>(),
        vec![4, 5, 6]
    );
    assert!(project(&worldline, time(21_000))
        .payload()
        .actors()
        .is_empty());
}

#[test]
fn projection_preserves_exact_inside_tick_journal_visibility_and_order() {
    let worldline = actual(journal(&[
        (0, GameJournalEntry::create_saucer()),
        (500, spawn(1, ActorKind::Forester, TileId::origin())),
        (500, terrain(TileId::origin(), Terrain::Forest)),
    ]));

    assert!(project(&worldline, time(499)).payload().actors().is_empty());
    assert_eq!(project(&worldline, time(500)).payload().actors().len(), 1);
    assert_eq!(project(&worldline, time(999)).payload().actors().len(), 1);
    assert_eq!(
        project(&worldline, game_time(1))
            .payload()
            .resources()
            .wood(),
        1
    );

    assert_eq!(
        project(&worldline, time(500)).payload().actors()[0].tile(),
        TileId::origin()
    );
    assert_eq!(
        project(&worldline, time(500))
            .payload()
            .terrain_at(TileId::origin()),
        Some(Terrain::Forest)
    );
}

#[test]
fn projection_applies_authored_terrain_before_derived_terrain_at_one_tick() {
    let worldline = actual(journal(&[
        (0, GameJournalEntry::create_saucer()),
        (0, spawn(1, ActorKind::Farmer, TileId::origin())),
        (500, terrain(tile(2, 0), Terrain::Forest)),
    ]));

    assert_eq!(
        project(&worldline, game_time(1))
            .payload()
            .terrain_at(tile(2, 0)),
        Some(Terrain::Wheat)
    );
}

#[test]
fn projection_moves_foresters_from_shared_pre_tick_occupancy() {
    let worldline = actual(journal(&[
        (0, GameJournalEntry::create_saucer()),
        (0, spawn(1, ActorKind::Forester, TileId::origin())),
        (0, spawn(2, ActorKind::Forester, tile(0, 1))),
        (0, spawn(3, ActorKind::Arborist, tile(1, 1))),
    ]));

    let at_one = project(&worldline, game_time(1));
    assert_eq!(
        at_one
            .payload()
            .actors()
            .iter()
            .find(|actor| actor.id() == actor_id(1))
            .expect("first forester remains present")
            .tile(),
        tile(1, 0)
    );
    assert_eq!(
        at_one
            .payload()
            .actors()
            .iter()
            .find(|actor| actor.id() == actor_id(2))
            .expect("second forester remains present")
            .tile(),
        tile(0, 1)
    );

    let at_two = project(&worldline, game_time(2));
    assert_eq!(
        at_two
            .payload()
            .actors()
            .iter()
            .find(|actor| actor.id() == actor_id(1))
            .expect("first forester remains present")
            .tile(),
        tile(2, 0)
    );
    assert_eq!(
        at_two
            .payload()
            .actors()
            .iter()
            .find(|actor| actor.id() == actor_id(2))
            .expect("second forester remains present")
            .tile(),
        TileId::origin()
    );
}

#[test]
fn projection_updates_occupancy_after_a_farmer_leaves() {
    let worldline = actual(journal(&[
        (0, GameJournalEntry::create_saucer()),
        (0, terrain(tile(1, 0), Terrain::Forest)),
        (0, spawn(1, ActorKind::Forester, TileId::origin())),
        (0, spawn(2, ActorKind::Farmer, tile(1, 0))),
        (0, spawn(3, ActorKind::Arborist, tile(1, -1))),
        (0, spawn(4, ActorKind::Arborist, tile(0, -1))),
        (0, spawn(5, ActorKind::Arborist, tile(-1, 0))),
        (0, spawn(6, ActorKind::Arborist, tile(-1, 1))),
        (0, spawn(7, ActorKind::Arborist, tile(0, 1))),
        (0, spawn(8, ActorKind::Arborist, tile(2, 0))),
        (0, spawn(9, ActorKind::Arborist, tile(2, -1))),
        (0, spawn(10, ActorKind::Arborist, tile(1, 1))),
    ]));

    let at_one = project(&worldline, game_time(1));
    assert_eq!(
        at_one
            .payload()
            .actors()
            .iter()
            .find(|actor| actor.id() == actor_id(1))
            .expect("forester remains present")
            .tile(),
        TileId::origin()
    );
    assert!(at_one
        .payload()
        .actors()
        .iter()
        .all(|actor| actor.id() != actor_id(2)));

    let at_two = project(&worldline, game_time(2));
    assert_eq!(
        at_two
            .payload()
            .actors()
            .iter()
            .find(|actor| actor.id() == actor_id(1))
            .expect("forester remains present")
            .tile(),
        tile(1, 0)
    );
    assert_eq!(at_two.payload().resources().wood(), 1);
}

#[test]
fn projection_uses_an_immutable_piece_index_with_caravan_sources() {
    let journal = journal(&[
        (0, GameJournalEntry::create_saucer()),
        (500, spawn(1, ActorKind::Farmer, TileId::origin())),
    ]);
    let index = discontinuity_index(&journal);

    assert!(index.breakpoints().iter().any(|breakpoint| {
        breakpoint.source() == EngineBreakpointSource::Journal { append_ordinal: 0 }
            && breakpoint.payload() == &CaravanBreakpointSource::JournalEntry { append_ordinal: 0 }
    }));
    assert!(index.breakpoints().iter().any(|breakpoint| {
        breakpoint.payload() == &CaravanBreakpointSource::CreateSaucer { append_ordinal: 0 }
    }));
    assert!(index.breakpoints().iter().any(|breakpoint| {
        breakpoint.payload() == &CaravanBreakpointSource::GameTick { tick_index: 0 }
    }));
    assert!(index.breakpoints().iter().any(|breakpoint| {
        breakpoint.payload()
            == &CaravanBreakpointSource::ActorThreshold {
                actor_id: actor_id(1),
                kind: ActorKind::Farmer,
                threshold: ActorThreshold::FarmerTerminal,
            }
    }));
    assert!(index.breakpoints().iter().any(|breakpoint| {
        breakpoint.payload()
            == &CaravanBreakpointSource::RuleThreshold {
                threshold: RuleThreshold::WheatResource,
            }
    }));

    let before = index.select(time(499));
    let at_entry = index.select(time(500));
    assert!(before.contains(time(499)));
    assert!(at_entry.contains(time(500)));
    assert_eq!(before.payload().visible_entry_count(), 1);
    assert_eq!(at_entry.payload().visible_entry_count(), 2);
    assert_eq!(before.payload().visible_entries().len(), 1);
    assert_eq!(at_entry.payload().visible_entries().len(), 2);
    assert_eq!(
        at_entry.payload().visible_entries()[1].payload(),
        &spawn(1, ActorKind::Farmer, TileId::origin())
    );

    let context = Context::new(caravan_reference::ReferenceContext::new());
    let projected = project_with_index(&context, &index, time(500));
    assert_eq!(projected.logical_time(), time(500));
    assert_eq!(projected.payload().actors().len(), 1);
    assert_eq!(projected.payload().actors()[0].id(), actor_id(1));
    assert_eq!(
        projected.payload().terrain_at(TileId::origin()),
        Some(Terrain::Void)
    );
}

#[test]
fn projection_matches_actual_counterfactual_and_corrected_branches_in_any_order() {
    let parent = actual(journal(&[
        (0, GameJournalEntry::create_saucer()),
        (500, terrain(TileId::origin(), Terrain::Wheat)),
        (1_000, spawn(1, ActorKind::Farmer, tile(1, 0))),
    ]));
    let alternate = journal(&[(1_500, spawn(2, ActorKind::Forester, tile(-1, 0)))]);
    let replacement = journal(&[(1_500, spawn(3, ActorKind::Arborist, tile(0, 1)))]);
    let counterfactual = parent
        .counterfactual(game_time(1), &alternate)
        .expect("counterfactual suffix is after the fork");
    let corrected = parent
        .corrected_suffix(game_time(1), &replacement)
        .expect("corrected suffix is after the fork");

    assert!(project(&parent, time(499)).payload().actors().is_empty());
    assert_eq!(
        project(&counterfactual, time(1_500))
            .payload()
            .actors()
            .iter()
            .map(|actor| actor.id().get())
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(
        project(&corrected, time(1_500))
            .payload()
            .actors()
            .iter()
            .map(|actor| actor.id().get())
            .collect::<Vec<_>>(),
        vec![1, 3]
    );

    let later_first = project(&counterfactual, game_time(2));
    let earlier = project(&counterfactual, time(499));
    let later_again = project(&counterfactual, game_time(2));
    assert_eq!(later_first, later_again);
    assert!(earlier.payload().actors().is_empty());
    assert_eq!(parent.kind(), BranchKind::Actual);
    assert_eq!(counterfactual.kind(), BranchKind::Counterfactual);
    assert_eq!(corrected.kind(), BranchKind::Corrected);
}

const REMAINING_PROJECTION_GAPS: &[&str] = &[];

#[test]
fn projection_corpus_covers_direct_forms_without_a_tautological_baseline() {
    let worldline = actual(hand_authored_behavior_fixture());
    let samples = [
        (-1, false, 0),
        (0, true, 5),
        (499, true, 5),
        (1_000, true, 3),
        (4_000, true, 3),
    ];

    for (ticks, has_saucer, actor_count) in samples {
        let logical_time = time(ticks);
        let projected = project(&worldline, logical_time);
        assert_eq!(projected.logical_time(), logical_time);
        assert_eq!(projected.payload().has_saucer(), has_saucer);
        assert_eq!(projected.payload().actors().len(), actor_count);
    }

    let later = project(&worldline, time(4_500));
    let earlier = project(&worldline, time(499));
    assert_eq!(later.logical_time(), time(4_500));
    assert_eq!(earlier.logical_time(), time(499));
    assert_eq!(later.payload().tick_index(), 4);
    assert_eq!(earlier.payload().tick_index(), 0);
}

#[test]
fn projection_gaps_are_recorded_as_scope_data() {
    assert_eq!(REMAINING_PROJECTION_GAPS.len(), 0);
    assert!(REMAINING_PROJECTION_GAPS
        .iter()
        .all(|gap| !gap.trim().is_empty()));
}
