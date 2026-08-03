use caravan_domain::{ActorId, ActorKind, GameJournalEntry, TileId};
use caravan_reference::{actual, state, ReferenceWorldline, Snapshot};
use engine_journal::{Journal, JournalWriter};
use engine_presentation::{present, present_with_animation, Animation, LinearPlayback, Renderer};
use engine_sdk::{GameState, Playback};
use engine_time::{LogicalTime, Tau};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct RenderValue {
    logical_time: LogicalTime,
    tau: Tau,
    actor_ids: Vec<u64>,
}

struct SnapshotRenderer;

impl Renderer<Snapshot> for SnapshotRenderer {
    type Output = RenderValue;

    fn render(&self, state: &GameState<Snapshot>, tau: Tau) -> Self::Output {
        RenderValue {
            logical_time: state.logical_time(),
            tau,
            actor_ids: state
                .payload()
                .actors()
                .iter()
                .map(|actor| actor.id().get())
                .collect(),
        }
    }
}

struct DeterministicAnimation;

impl Animation<Snapshot> for DeterministicAnimation {
    type Output = i64;

    fn sample(&self, state: &GameState<Snapshot>, tau: Tau) -> Option<Self::Output> {
        (tau.ticks().rem_euclid(2) == 0).then(|| state.logical_time().ticks() * 10 + tau.ticks())
    }
}

fn time(ticks: i64) -> LogicalTime {
    LogicalTime::from_ticks(ticks)
}

fn journal(entries: impl IntoIterator<Item = (i64, GameJournalEntry)>) -> Journal {
    let mut writer = JournalWriter::new();
    for (ticks, payload) in entries {
        writer
            .advance_to(time(ticks))
            .expect("presentation fixtures use monotonic timestamps");
        writer.record(payload);
    }
    writer.finish()
}

fn spawn(id: u64, kind: ActorKind, tile: TileId) -> GameJournalEntry {
    GameJournalEntry::SpawnActor {
        id: ActorId::new(id).expect("presentation fixtures use positive actor IDs"),
        kind,
        tile,
    }
}

fn create_saucer() -> GameJournalEntry {
    GameJournalEntry::create_saucer()
}

fn reference_state(
    worldline: &ReferenceWorldline,
    logical_time: LogicalTime,
) -> GameState<Snapshot> {
    state(worldline, logical_time)
}

#[test]
fn linear_playback_maps_forward_and_reverse_with_checked_time_arithmetic() {
    let forward = LinearPlayback::new(time(10), 2);
    let reverse = LinearPlayback::reverse_from(time(10));

    assert_eq!(forward.logical_time_at(Tau::from_ticks(3)), time(16));
    assert_eq!(reverse.logical_time_at(Tau::from_ticks(3)), time(7));
    assert_eq!(
        LinearPlayback::new(time(i64::MAX), 1).try_logical_time_at(Tau::from_ticks(1)),
        None
    );
}

#[test]
fn present_supports_forward_reverse_and_arbitrary_scrub() {
    let worldline = actual(journal([
        (0, create_saucer()),
        (3, spawn(1, ActorKind::Forester, TileId::origin())),
    ]));
    let renderer = SnapshotRenderer;
    let forward = LinearPlayback::one_to_one();
    let reverse = LinearPlayback::reverse_from(time(5));

    let forward_frame = present(
        &worldline,
        &reference_state,
        &forward,
        &renderer,
        Tau::from_ticks(5),
    );
    let reverse_frame = present(
        &worldline,
        &reference_state,
        &reverse,
        &renderer,
        Tau::from_ticks(2),
    );
    let scrubbed_frame = present(
        &worldline,
        &reference_state,
        &forward,
        &renderer,
        Tau::from_ticks(2),
    );

    assert_eq!(forward_frame.tau(), Tau::from_ticks(5));
    assert_eq!(forward_frame.payload().logical_time, time(5));
    assert_eq!(reverse_frame.payload().logical_time, time(3));
    assert_eq!(scrubbed_frame.payload().logical_time, time(2));
    assert!(scrubbed_frame.payload().actor_ids.is_empty());
}

#[test]
fn repeated_samples_are_equal_and_do_not_depend_on_query_order() {
    let worldline = actual(journal([
        (0, create_saucer()),
        (3, spawn(1, ActorKind::Forester, TileId::origin())),
    ]));
    let renderer = SnapshotRenderer;
    let playback = LinearPlayback::one_to_one();

    let later_first = present(
        &worldline,
        &reference_state,
        &playback,
        &renderer,
        Tau::from_ticks(5),
    );
    let _earlier = present(
        &worldline,
        &reference_state,
        &playback,
        &renderer,
        Tau::from_ticks(2),
    );
    let later_again = present(
        &worldline,
        &reference_state,
        &playback,
        &renderer,
        Tau::from_ticks(5),
    );

    assert_eq!(later_first, later_again);
}

#[test]
fn actual_counterfactual_and_corrected_branches_use_one_presentation_path() {
    let parent = actual(journal([
        (0, create_saucer()),
        (10, spawn(1, ActorKind::Farmer, TileId::origin())),
    ]));
    let counterfactual = parent
        .counterfactual(
            time(5),
            &journal([(7, spawn(2, ActorKind::Forester, TileId::origin()))]),
        )
        .expect("counterfactual suffix is after its boundary");
    let corrected = parent
        .corrected_suffix(
            time(5),
            &journal([(6, spawn(3, ActorKind::Arborist, TileId::origin()))]),
        )
        .expect("corrected suffix is after its boundary");
    let renderer = SnapshotRenderer;
    let playback = LinearPlayback::one_to_one();

    let actual_frame = present(
        &parent,
        &reference_state,
        &playback,
        &renderer,
        Tau::from_ticks(10),
    );
    let counterfactual_frame = present(
        &counterfactual,
        &reference_state,
        &playback,
        &renderer,
        Tau::from_ticks(10),
    );
    let corrected_frame = present(
        &corrected,
        &reference_state,
        &playback,
        &renderer,
        Tau::from_ticks(10),
    );

    assert_eq!(actual_frame.payload().actor_ids, vec![1]);
    assert_eq!(counterfactual_frame.payload().actor_ids, vec![2]);
    assert_eq!(corrected_frame.payload().actor_ids, vec![3]);
    assert_eq!(parent.journal().len(), 2);
}

#[test]
fn optional_animation_is_a_deterministic_value_boundary() {
    let worldline = actual(journal([(0, create_saucer())]));
    let renderer = SnapshotRenderer;
    let animation = DeterministicAnimation;
    let playback = LinearPlayback::one_to_one();

    let even = present_with_animation(
        &worldline,
        &reference_state,
        &playback,
        &renderer,
        Some(&animation),
        Tau::from_ticks(2),
    );
    let even_again = present_with_animation(
        &worldline,
        &reference_state,
        &playback,
        &renderer,
        Some(&animation),
        Tau::from_ticks(2),
    );
    let odd = present_with_animation(
        &worldline,
        &reference_state,
        &playback,
        &renderer,
        Some(&animation),
        Tau::from_ticks(3),
    );

    assert_eq!(even, even_again);
    assert_eq!(even.animation(), Some(&22));
    assert_eq!(odd.animation(), None);
}

#[test]
fn query_and_renderer_remain_generic_over_opaque_payloads() {
    #[derive(Clone, Debug, Eq, Hash, PartialEq)]
    struct OpaqueWorldline {
        marker: u32,
    }

    struct OpaqueRenderer;

    impl Renderer<u32> for OpaqueRenderer {
        type Output = (u32, Tau);

        fn render(&self, state: &GameState<u32>, tau: Tau) -> Self::Output {
            (*state.payload(), tau)
        }
    }

    let worldline = OpaqueWorldline { marker: 17 };
    let query = |worldline: &OpaqueWorldline, logical_time: LogicalTime| {
        GameState::new(logical_time, worldline.marker)
    };
    let frame = present(
        &worldline,
        &query,
        &LinearPlayback::one_to_one(),
        &OpaqueRenderer,
        Tau::from_ticks(4),
    );

    assert_eq!(frame.payload(), &(17, Tau::from_ticks(4)));
    assert_eq!(frame.tau(), Tau::from_ticks(4));
}
