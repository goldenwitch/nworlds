use std::{fs, path::PathBuf};

use caravan_domain::{ActorKind, Effect, GameJournalEntry, Saucer, Terrain, TileId};
use caravan_reference::actual;
use caravan_seeded::generate_spawn_journal;
use engine_branches::BranchKind;
use engine_index::game_tick_index;
use engine_journal::{Journal, JournalWriter};
use engine_lookahead::{branch_view, future, ViewKind};
use engine_presentation::{present, present_with_animation, LinearPlayback};
use engine_time::{LogicalTime, Tau, GAME_TICK_PERIOD, TICKS_PER_LOGICAL_SECOND};

use crate::fixtures::{
    actor_id, actor_ids, journal, reference_query, seeded_worldline, snapshot_at, spawn, state,
    terrain, tile, time, worldline, ParityAnimation, RenderValue, TraceRenderer,
};

pub fn empty_journal_is_empty_and_owns_exact_time() {
    let actual = actual(Journal::empty());
    let future = state(&actual, 7);
    let past = state(&actual, -3);

    assert_eq!(future.logical_time(), time(7));
    assert_eq!(past.logical_time(), time(-3));
    assert!(!future.payload().has_saucer());
    assert!(future.payload().tiles().is_empty());
    assert!(future.payload().actors().is_empty());
    assert_eq!(future.payload().resources().wheat(), 0);
    assert_eq!(future.payload().resources().wood(), 0);
    assert_ne!(future, past);
    assert_eq!(
        caravan_reference::context().payload().saucer().tile_count(),
        91
    );
}

pub fn create_saucer_has_91_void_tiles_and_empty_layers() {
    let actual = worldline([(0, GameJournalEntry::create_saucer())]);
    let created = state(&actual, 0);
    let snapshot = created.payload();

    assert_eq!(created.logical_time(), time(0));
    assert!(snapshot.has_saucer());
    assert_eq!(snapshot.saucer(), Some(Saucer::new()));
    assert_eq!(snapshot.tiles().len(), 91);
    assert_eq!(snapshot.tiles().len(), Saucer::new().tile_count());
    assert_eq!(
        snapshot
            .tiles()
            .iter()
            .map(|indexed| indexed.tile())
            .collect::<Vec<_>>(),
        Saucer::new().tiles().to_vec()
    );
    assert!(snapshot.tiles().iter().all(|indexed| {
        indexed.layers().terrain() == Terrain::Void
            && indexed.layers().actor().is_none()
            && indexed.layers().effect() == Effect::None
    }));
}

pub fn journal_timestamps_control_visibility_and_append_order() {
    let mut writer = JournalWriter::new();
    let create = writer.record(GameJournalEntry::create_saucer());
    writer
        .advance_to(time(10))
        .expect("postdating is a forward cursor move");
    let spawn = writer.record(spawn(1, ActorKind::Farmer, TileId::origin()));
    let authored_journal = writer.finish();

    assert_eq!(create.logical_time(), time(0));
    assert_eq!(spawn.logical_time(), time(10));
    assert_eq!(authored_journal.visible_at(time(9)).count(), 1);
    assert_eq!(authored_journal.visible_at(time(10)).count(), 2);

    let equal_time_journal = journal([
        (0, GameJournalEntry::create_saucer()),
        (3, terrain(TileId::origin(), Terrain::Wheat)),
        (3, terrain(TileId::origin(), Terrain::Forest)),
    ]);
    let at_equal_time = snapshot_at(&actual(equal_time_journal.clone()), 3);
    assert_eq!(
        at_equal_time.terrain_at(TileId::origin()),
        Some(Terrain::Forest)
    );
    let payloads = equal_time_journal
        .iter()
        .map(|entry| *entry.payload())
        .collect::<Vec<_>>();
    assert_eq!(payloads[1], terrain(TileId::origin(), Terrain::Wheat));
    assert_eq!(payloads[2], terrain(TileId::origin(), Terrain::Forest));
}

pub fn query_order_does_not_change_a_fixed_worldline() {
    let actual = worldline([
        (0, GameJournalEntry::create_saucer()),
        (3, spawn(1, ActorKind::Forester, TileId::origin())),
    ]);
    let original_journal = actual.journal().clone();

    let later_first = state(&actual, 5);
    let earlier = state(&actual, 2);
    let later_again = state(&actual, 5);

    assert_eq!(later_first, later_again);
    assert_eq!(later_first.logical_time(), time(5));
    assert_eq!(earlier.logical_time(), time(2));
    assert!(earlier.payload().actors().is_empty());
    assert_eq!(later_first.payload().actors().len(), 1);
    assert_eq!(actual.journal(), &original_journal);
}

pub fn fixed_journal_repeated_sample_is_stable() {
    let actual = worldline([(0, GameJournalEntry::create_saucer())]);
    let first = state(&actual, 4);
    let repeated = state(&actual, 4);

    assert_eq!(TICKS_PER_LOGICAL_SECOND, 1);
    assert_eq!(GAME_TICK_PERIOD, time(1));
    assert_eq!(game_tick_index(time(4)), 4);
    assert_eq!(first.logical_time(), time(4));
    assert_eq!(repeated.logical_time(), time(4));
    assert_eq!(first.payload(), repeated.payload());
    assert_eq!(actual.journal().len(), 1);
}

pub fn journal_entry_is_visible_at_its_exact_time() {
    let actual = worldline([
        (0, GameJournalEntry::create_saucer()),
        (5, spawn(1, ActorKind::Farmer, TileId::origin())),
    ]);
    let before = state(&actual, 4);
    let at_entry = state(&actual, 5);

    assert_eq!(before.logical_time(), time(4));
    assert_eq!(at_entry.logical_time(), time(5));
    assert!(before.payload().actors().is_empty());
    assert_eq!(actor_ids(at_entry.payload()), vec![1]);
    assert_ne!(before.payload(), at_entry.payload());
}

pub fn terrain_actor_and_effect_layers_coexist_independently() {
    let actual = worldline([
        (0, GameJournalEntry::create_saucer()),
        (0, terrain(tile(1, 0), Terrain::Wheat)),
        (0, spawn(2, ActorKind::Arsonist, TileId::origin())),
        (0, spawn(3, ActorKind::Arborist, tile(1, 0))),
    ]);
    let burning = snapshot_at(&actual, 1);
    let layers = burning
        .layers_at(tile(1, 0))
        .expect("created saucer contains the layer tile");

    assert_eq!(layers.terrain(), Terrain::Wheat);
    assert_eq!(layers.actor(), Some(actor_id(3)));
    assert_eq!(layers.effect(), Effect::fire(0));
    assert_eq!(burning.terrain_at(tile(1, 0)), Some(Terrain::Wheat));
    assert_eq!(burning.actor_at(tile(1, 0)), Some(actor_id(3)));
    assert_eq!(burning.effect_at(tile(1, 0)), Some(Effect::fire(0)));

    let after_destruction = snapshot_at(&actual, 4);
    assert_eq!(
        after_destruction.terrain_at(tile(1, 0)),
        Some(Terrain::Void)
    );
    assert_eq!(after_destruction.actor_at(tile(1, 0)), Some(actor_id(3)));
    assert_eq!(after_destruction.effect_at(tile(1, 0)), Some(Effect::None));
}

pub fn farmer_difference_is_repeatable_and_places_wheat() {
    let actual = worldline([
        (0, GameJournalEntry::create_saucer()),
        (0, spawn(1, ActorKind::Farmer, TileId::origin())),
    ]);
    let initial = state(&actual, 0);
    let next = state(&actual, 1);
    let next_again = state(&actual, 1);

    assert_eq!(actor_ids(initial.payload()), vec![1]);
    assert!(next.payload().actors().is_empty());
    assert_eq!(
        crate::fixtures::terrain_count(next.payload(), Terrain::Wheat),
        6
    );
    for wheat_tile in [tile(2, 0), tile(0, 0), tile(1, 1)] {
        assert_eq!(next.payload().terrain_at(wheat_tile), Some(Terrain::Wheat));
    }
    assert_eq!(next, next_again);
}

pub fn wheat_resource_is_indexed_without_query_carryover() {
    let actual = worldline([
        (0, GameJournalEntry::create_saucer()),
        (0, terrain(TileId::origin(), Terrain::Wheat)),
    ]);
    let at_two = state(&actual, 2);
    let at_zero = state(&actual, 0);
    let at_two_again = state(&actual, 2);

    assert_eq!(at_zero.payload().resources().wheat(), 1);
    assert_eq!(at_two.payload().resources().wheat(), 3);
    assert_eq!(at_two.payload().resources().wood(), 0);
    assert_eq!(at_two, at_two_again);
}

pub fn forester_difference_and_wood_total_are_repeatable() {
    let actual = worldline([
        (0, GameJournalEntry::create_saucer()),
        (0, terrain(tile(1, 0), Terrain::Forest)),
        (0, spawn(1, ActorKind::Forester, TileId::origin())),
    ]);
    let at_zero = state(&actual, 0);
    let at_one = state(&actual, 1);
    let at_two = state(&actual, 2);

    assert_eq!(at_zero.payload().actors()[0].tile(), TileId::origin());
    assert_eq!(at_one.payload().actors()[0].tile(), tile(1, 0));
    assert_eq!(at_one.payload().resources().wood(), 1);
    assert_eq!(at_two.payload().actors()[0].tile(), tile(1, 0));
    assert_eq!(at_two.payload().resources().wood(), 2);
    assert_eq!(at_two, state(&actual, 2));
}

pub fn arsonist_and_fire_are_indexed_layer_differences() {
    let actual = worldline([
        (0, GameJournalEntry::create_saucer()),
        (0, terrain(tile(1, 0), Terrain::Wheat)),
        (0, terrain(tile(1, -1), Terrain::Forest)),
        (0, terrain(tile(2, 0), Terrain::Wheat)),
        (0, spawn(1, ActorKind::Arsonist, TileId::origin())),
        (0, spawn(2, ActorKind::Arborist, tile(4, 0))),
    ]);
    let at_one = snapshot_at(&actual, 1);
    let at_two = snapshot_at(&actual, 2);
    let at_three = snapshot_at(&actual, 3);
    let at_four = snapshot_at(&actual, 4);

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
    assert_eq!(at_four, snapshot_at(&actual, 4));
}

pub fn fighter_collision_is_a_repeatable_indexed_difference() {
    let actual = worldline([
        (0, GameJournalEntry::create_saucer()),
        (0, spawn(1, ActorKind::Fighter, TileId::origin())),
        (0, spawn(2, ActorKind::Arsonist, tile(1, 0))),
    ]);
    let next = snapshot_at(&actual, 1);
    let next_again = snapshot_at(&actual, 1);

    assert_eq!(next.actors().len(), 1);
    assert_eq!(next.actors()[0].id(), actor_id(1));
    assert_eq!(next.actors()[0].tile(), tile(1, 0));
    assert_eq!(next, next_again);
}

pub fn arborist_conversion_keeps_actor_and_terrain_separate() {
    let actual = worldline([
        (0, GameJournalEntry::create_saucer()),
        (0, terrain(TileId::origin(), Terrain::Wheat)),
        (0, spawn(1, ActorKind::Arborist, TileId::origin())),
    ]);
    let at_two = snapshot_at(&actual, 2);
    let at_three = snapshot_at(&actual, 3);

    assert_eq!(at_two.terrain_at(TileId::origin()), Some(Terrain::Wheat));
    assert_eq!(at_three.terrain_at(TileId::origin()), Some(Terrain::Forest));
    assert_eq!(at_three.actor_at(TileId::origin()), Some(actor_id(1)));
    assert_eq!(at_three, snapshot_at(&actual, 3));
}

pub fn same_seed_reproduces_journal_and_states() {
    let first_journal = generate_spawn_journal(0xCAFE, 20);
    let second_journal = generate_spawn_journal(0xCAFE, 20);
    let first = actual(first_journal.clone());
    let second = actual(second_journal.clone());

    assert_eq!(first_journal, second_journal);
    assert_eq!(first_journal.len(), 7);
    assert_eq!(first_journal.get(0).unwrap().logical_time(), time(0));
    assert_eq!(
        first_journal
            .iter()
            .filter(|entry| matches!(entry.payload(), GameJournalEntry::SpawnActor { .. }))
            .count(),
        6
    );
    assert!(first_journal.iter().skip(1).all(|entry| {
        matches!(entry.payload(), GameJournalEntry::SpawnActor { tile, .. } if Saucer::new().tiles().contains(tile))
    }));
    assert_eq!(state(&first, 10), state(&second, 10));
    assert_eq!(state(&first, 20), state(&second, 20));
}

pub fn branches_agree_at_prefix_and_diverge_without_parent_mutation() {
    let parent = worldline([
        (0, GameJournalEntry::create_saucer()),
        (10, spawn(1, ActorKind::Farmer, TileId::origin())),
    ]);
    let original_journal = parent.journal().clone();
    let alternate = journal([(7, spawn(2, ActorKind::Forester, TileId::origin()))]);
    let replacement = journal([(6, spawn(3, ActorKind::Arborist, tile(0, 1)))]);
    let counterfactual = parent
        .counterfactual(time(5), &alternate)
        .expect("counterfactual suffix is after the fork boundary");
    let corrected = parent
        .corrected_suffix(time(5), &replacement)
        .expect("corrected suffix is after the fork boundary");

    assert_eq!(parent.kind(), BranchKind::Actual);
    assert_eq!(counterfactual.kind(), BranchKind::Counterfactual);
    assert_eq!(corrected.kind(), BranchKind::Corrected);
    assert_eq!(counterfactual.fork_boundary(), Some(time(5)));
    assert_eq!(corrected.fork_boundary(), Some(time(5)));
    assert_eq!(state(&parent, 5), state(&counterfactual, 5));
    assert_eq!(state(&parent, 5), state(&corrected, 5));
    assert_eq!(actor_ids(state(&parent, 10).payload()), vec![1]);
    assert_eq!(actor_ids(state(&counterfactual, 10).payload()), vec![2]);
    assert_eq!(actor_ids(state(&corrected, 10).payload()), vec![3]);
    assert_eq!(parent.journal(), &original_journal);
    assert_eq!(parent.journal().len(), 2);
}

pub fn lookahead_keeps_the_journal_fixed() {
    let actual = seeded_worldline();
    let original_journal = actual.journal().clone();
    let earlier = future(&actual, time(4));
    let future_state = future(&actual, time(30));
    let future_again = future(&actual, time(30));
    let suffix = journal([(25, spawn(99, ActorKind::Arborist, tile(0, 1)))]);
    let alternate = actual
        .counterfactual(time(20), &suffix)
        .expect("lookahead branch suffix is after the fork boundary");

    assert_eq!(future_state.logical_time(), time(30));
    assert_eq!(future_state.payload().tick_index(), 30);
    assert_eq!(future_state.payload().tiles().len(), 91);
    assert_eq!(future_state, future_again);
    assert_eq!(actual.journal(), &original_journal);
    assert_eq!(actual.journal().len(), 7);
    assert_eq!(earlier, future(&actual, time(4)));
    assert_eq!(future(&actual, time(4)), future(&alternate, time(4)));
    assert_eq!(branch_view(&actual).kind(), ViewKind::Actual);
    assert_eq!(branch_view(&alternate).kind(), ViewKind::Counterfactual);
}

pub fn presentation_supports_scrubbing_branches_and_repeatable_animation() {
    let parent = worldline([
        (0, GameJournalEntry::create_saucer()),
        (3, spawn(1, ActorKind::Forester, TileId::origin())),
    ]);
    let original_parent = parent.clone();
    let renderer = TraceRenderer;
    let forward = LinearPlayback::one_to_one();
    let reverse = LinearPlayback::reverse_from(time(5));

    let forward_frame = present(
        &parent,
        &reference_query,
        &forward,
        &renderer,
        Tau::from_ticks(5),
    );
    let reverse_frame = present(
        &parent,
        &reference_query,
        &reverse,
        &renderer,
        Tau::from_ticks(2),
    );
    let scrubbed_frame = present(
        &parent,
        &reference_query,
        &forward,
        &renderer,
        Tau::from_ticks(2),
    );
    let repeated_frame = present(
        &parent,
        &reference_query,
        &forward,
        &renderer,
        Tau::from_ticks(5),
    );

    assert_eq!(forward_frame.tau(), Tau::from_ticks(5));
    assert_eq!(forward_frame.payload().sampled_time, 5);
    assert_eq!(reverse_frame.payload().sampled_time, 3);
    assert_eq!(scrubbed_frame.payload().sampled_time, 2);
    assert!(scrubbed_frame.payload().actor_ids.is_empty());
    assert_eq!(forward_frame, repeated_frame);
    assert_eq!(parent, original_parent);

    let animation = ParityAnimation;
    let even = present_with_animation(
        &parent,
        &reference_query,
        &forward,
        &renderer,
        Some(&animation),
        Tau::from_ticks(2),
    );
    let even_again = present_with_animation(
        &parent,
        &reference_query,
        &forward,
        &renderer,
        Some(&animation),
        Tau::from_ticks(2),
    );
    let odd = present_with_animation(
        &parent,
        &reference_query,
        &forward,
        &renderer,
        Some(&animation),
        Tau::from_ticks(3),
    );
    assert_eq!(even, even_again);
    assert_eq!(even.animation(), Some(&22));
    assert_eq!(odd.animation(), None);

    let counterfactual = parent
        .counterfactual(
            time(1),
            &journal([(4, spawn(2, ActorKind::Arborist, TileId::origin()))]),
        )
        .expect("presentation branch suffix is after the fork boundary");
    let corrected = parent
        .corrected_suffix(
            time(1),
            &journal([(4, spawn(3, ActorKind::Farmer, TileId::origin()))]),
        )
        .expect("presentation correction suffix is after the fork boundary");
    let actual_frame = present(
        &parent,
        &reference_query,
        &forward,
        &renderer,
        Tau::from_ticks(4),
    );
    let counterfactual_frame = present(
        &counterfactual,
        &reference_query,
        &forward,
        &renderer,
        Tau::from_ticks(4),
    );
    let corrected_frame = present(
        &corrected,
        &reference_query,
        &forward,
        &renderer,
        Tau::from_ticks(4),
    );
    assert_eq!(actual_frame.payload().actor_ids, vec![1]);
    assert_eq!(counterfactual_frame.payload().actor_ids, vec![2]);
    assert_eq!(corrected_frame.payload().actor_ids, vec![3]);
}

pub fn demo_trace_contains_the_anchor_observables() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/caravan-demo/snapshots/anchor-trace.txt");
    let trace = fs::read_to_string(path).expect("the checked-in demo trace is readable");
    for line in [
        "empty journal: saucer=false tiles=0",
        "create saucer: journal_t_=0 radius=5 tiles=91 void=91",
        "postdated spawn: t_=9 actors=[]; t_=10 actors=[1]",
        "arbitrary sampling: t_=[10,2,10]",
        "three layers: t_=1 tile=(1, 0)",
        "seeded journal: seed=0xCAFE horizon=20",
        "lookahead: fixed_entries=7 t_=30",
        "branch views: actual=Actual fork=- counterfactual=Counterfactual fork=5 corrected=Corrected fork=5",
        "presentation actual: sdk_tau=10 game_t_=10",
    ] {
        assert!(trace.contains(line), "demo trace is missing: {line}");
    }
}

#[allow(dead_code)]
fn _keep_public_types_in_scope(_: Option<RenderValue>) {
    let _ = LogicalTime::zero();
}
