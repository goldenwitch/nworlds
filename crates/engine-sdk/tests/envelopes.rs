use engine_sdk::{
    Context, Frame, GameState, Journal, JournalEntry, LogicalTime, Playback, QueryResult, Tau,
    Worldline,
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Definitions {
    marker: u8,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Fact {
    marker: u16,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Snapshot {
    marker: u32,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Image {
    marker: u64,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum Boundary {
    BeforeCreation,
}

struct OffsetPlayback;

impl Playback for OffsetPlayback {
    fn logical_time_at(&self, tau: Tau) -> LogicalTime {
        LogicalTime::from_ticks(tau.ticks() + 7)
    }
}

#[test]
fn distinct_time_types_cross_each_boundary() {
    let tau = Tau::from_ticks(11);
    let logical_time = OffsetPlayback.logical_time_at(tau);
    let state = GameState::new(logical_time, Snapshot { marker: 1 });
    let frame = Frame::new(tau, Image { marker: 2 });

    assert_eq!(logical_time, LogicalTime::from_ticks(18));
    assert_eq!(state.logical_time(), LogicalTime::from_ticks(18));
    assert_eq!(frame.tau(), Tau::from_ticks(11));
}

#[test]
fn state_owns_the_exact_sampled_logical_time() {
    let earlier = GameState::new(LogicalTime::from_ticks(2), Snapshot { marker: 9 });
    let later = GameState::new(LogicalTime::from_ticks(3), Snapshot { marker: 9 });

    assert_eq!(earlier.payload(), later.payload());
    assert_ne!(earlier.logical_time(), later.logical_time());
}

#[test]
fn opaque_payloads_remain_generic_across_context_journal_and_worldline() {
    let context = Context::new(Definitions { marker: 5 });
    let entry = JournalEntry::from_assigned_time(LogicalTime::zero(), Fact { marker: 13 });
    let journal = Journal::from_assigned_entries([entry]);
    let worldline = Worldline::new(context, journal);

    assert_eq!(worldline.context().payload().marker, 5);
    let visible = worldline
        .journal()
        .visible_at(LogicalTime::zero())
        .next()
        .expect("the exact-time entry is visible");
    assert_eq!(visible.payload().marker, 13);
}

#[test]
fn typed_query_results_keep_values_and_domain_reasons_distinct() {
    let value: QueryResult<GameState<Snapshot>, Boundary> =
        QueryResult::Value(GameState::new(LogicalTime::zero(), Snapshot { marker: 21 }));
    let reason =
        QueryResult::<GameState<Snapshot>, Boundary>::OutOfDomain(Boundary::BeforeCreation);

    assert!(value.is_value());
    assert_eq!(value.value().expect("value result").payload().marker, 21);
    assert!(reason.is_out_of_domain());
    assert_eq!(reason.out_of_domain(), Some(Boundary::BeforeCreation));
}

#[test]
fn public_payload_access_is_shared_only() {
    let context = Context::new(Definitions { marker: 34 });
    let entry = JournalEntry::from_assigned_time(LogicalTime::zero(), Fact { marker: 55 });
    let state = GameState::new(LogicalTime::zero(), Snapshot { marker: 89 });
    let frame = Frame::new(Tau::zero(), Image { marker: 144 });

    let _: &Definitions = context.payload();
    let _: &Fact = entry.payload();
    let _: &Snapshot = state.payload();
    let _: &Image = frame.payload();
}
