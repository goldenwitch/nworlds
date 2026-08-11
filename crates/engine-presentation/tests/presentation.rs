use caravan_domain::{ActorId, ActorKind, GameJournalEntry, TileId};
use caravan_reference::{actual, try_state, ReferenceWorldline, Snapshot};
use engine_journal::{Journal, JournalWriter};
use engine_presentation::{present, Renderer};
use engine_sdk::GameState;
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

    fn render(state: &GameState<Snapshot>, tau: Tau) -> Self::Output {
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
    try_state(worldline, logical_time).expect("presentation fixture should project")
}

#[test]
fn present_accepts_explicit_logical_and_presentation_times() {
    let worldline = actual(journal([
        (0, create_saucer()),
        (3, spawn(1, ActorKind::Forester, TileId::origin())),
    ]));
    let forward_state = reference_state(&worldline, time(5));
    let reverse_state = reference_state(&worldline, time(3));
    let scrubbed_state = reference_state(&worldline, time(2));

    let forward_frame = present::<Snapshot, SnapshotRenderer>(&forward_state, Tau::from_ticks(5));
    let reverse_frame = present::<Snapshot, SnapshotRenderer>(&reverse_state, Tau::from_ticks(2));
    let scrubbed_frame = present::<Snapshot, SnapshotRenderer>(&scrubbed_state, Tau::from_ticks(2));

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
    let later_state = reference_state(&worldline, time(5));
    let earlier_state = reference_state(&worldline, time(2));

    let later_first = present::<Snapshot, SnapshotRenderer>(&later_state, Tau::from_ticks(5));
    let _earlier = present::<Snapshot, SnapshotRenderer>(&earlier_state, Tau::from_ticks(2));
    let later_again = present::<Snapshot, SnapshotRenderer>(&later_state, Tau::from_ticks(5));

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
    let actual_state = reference_state(&parent, time(10));
    let counterfactual_state = reference_state(&counterfactual, time(10));
    let corrected_state = reference_state(&corrected, time(10));

    let actual_frame = present::<Snapshot, SnapshotRenderer>(&actual_state, Tau::from_ticks(10));
    let counterfactual_frame =
        present::<Snapshot, SnapshotRenderer>(&counterfactual_state, Tau::from_ticks(10));
    let corrected_frame =
        present::<Snapshot, SnapshotRenderer>(&corrected_state, Tau::from_ticks(10));

    assert_eq!(actual_frame.payload().actor_ids, vec![1]);
    assert_eq!(counterfactual_frame.payload().actor_ids, vec![2]);
    assert_eq!(corrected_frame.payload().actor_ids, vec![3]);
    assert_eq!(parent.journal().len(), 2);
}

#[test]
fn state_and_renderer_remain_generic_over_opaque_payloads() {
    #[derive(Clone, Debug, Eq, Hash, PartialEq)]
    struct OpaqueWorldline {
        marker: u32,
    }

    struct OpaqueRenderer;

    impl Renderer<u32> for OpaqueRenderer {
        type Output = (u32, Tau);

        fn render(state: &GameState<u32>, tau: Tau) -> Self::Output {
            (*state.payload(), tau)
        }
    }

    let worldline = OpaqueWorldline { marker: 17 };
    let state = GameState::new(LogicalTime::from_ticks(4), worldline.marker);
    let frame = present::<u32, OpaqueRenderer>(&state, Tau::from_ticks(4));

    assert_eq!(frame.payload(), &(17, Tau::from_ticks(4)));
    assert_eq!(frame.tau(), Tau::from_ticks(4));
}
