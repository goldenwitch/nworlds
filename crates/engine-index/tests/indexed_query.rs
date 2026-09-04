use caravan_domain::{ActorId, ActorKind, GameJournalEntry, TileId};
use engine_index::{state, IndexedQuery, QueryInput};
use engine_journal::{Journal, JournalWriter};
use engine_sdk::{Context, GameState};
use engine_time::LogicalTime;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Definitions {
    marker: u8,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct OpaqueQueryResult {
    game_tick_index: i64,
    visible_entries: Vec<GameJournalEntry>,
}

#[derive(Clone, Copy)]
struct TinyQuery;

impl<C> IndexedQuery<C, GameJournalEntry> for TinyQuery {
    type Result = OpaqueQueryResult;

    fn query(&self, input: QueryInput<'_, C, GameJournalEntry>) -> Self::Result {
        OpaqueQueryResult {
            game_tick_index: input.game_tick_index(),
            visible_entries: input
                .visible_entries()
                .map(|entry| *entry.payload())
                .collect(),
        }
    }
}

fn time(ticks: i64) -> LogicalTime {
    LogicalTime::from_ticks(ticks)
}

fn context() -> Context<Definitions> {
    Context::new(Definitions { marker: 7 })
}

fn journal(entries: &[(i64, GameJournalEntry)]) -> Journal<GameJournalEntry> {
    let mut writer = JournalWriter::new();

    for (ticks, payload) in entries {
        writer
            .advance_to(time(*ticks))
            .expect("test journal times are nondecreasing");
        writer.record(*payload);
    }

    writer.finish()
}

fn spawn(id: u64) -> GameJournalEntry {
    GameJournalEntry::SpawnActor {
        id: ActorId::new(id).expect("test actor IDs are positive"),
        kind: ActorKind::Farmer,
        tile: TileId::origin(),
    }
}

fn result(state: &GameState<OpaqueQueryResult>) -> &OpaqueQueryResult {
    state.payload()
}

#[test]
fn empty_journal_produces_an_empty_opaque_result() {
    let state = state(&context(), &Journal::empty(), time(0), TinyQuery);

    assert_eq!(state.logical_time(), time(0));
    assert_eq!(result(&state).game_tick_index, 0);
    assert!(result(&state).visible_entries.is_empty());
}

#[test]
fn create_saucer_is_visible_at_its_exact_timestamp() {
    let create_saucer = GameJournalEntry::create_saucer();
    let state = state(
        &context(),
        &journal(&[(0, create_saucer)]),
        time(0),
        TinyQuery,
    );

    assert_eq!(result(&state).visible_entries, vec![create_saucer]);
}

#[test]
fn postdated_entries_are_hidden_before_their_timestamp() {
    let create_saucer = GameJournalEntry::create_saucer();
    let spawn = spawn(1);
    let journal = journal(&[(0, create_saucer), (10, spawn)]);

    let before = state(&context(), &journal, time(9), TinyQuery);
    let at_target = state(&context(), &journal, time(10), TinyQuery);

    assert_eq!(result(&before).visible_entries, vec![create_saucer]);
    assert_eq!(
        result(&at_target).visible_entries,
        vec![create_saucer, spawn]
    );
}

#[test]
fn query_order_does_not_change_a_fixed_journal_result() {
    let journal = journal(&[(0, GameJournalEntry::create_saucer()), (10, spawn(1))]);

    let later_first = state(&context(), &journal, time(10), TinyQuery);
    let earlier = state(&context(), &journal, time(2), TinyQuery);
    let later_again = state(&context(), &journal, time(10), TinyQuery);

    assert_eq!(later_first, later_again);
    assert_eq!(result(&earlier).visible_entries.len(), 1);
    assert_eq!(result(&later_first).visible_entries.len(), 2);
    assert_eq!(earlier.logical_time(), time(2));
    assert_eq!(later_first.logical_time(), time(10));
}

#[test]
fn fixed_journal_repeated_samples_at_one_tick_are_stable() {
    let journal = journal(&[(0, GameJournalEntry::create_saucer())]);

    let first = state(&context(), &journal, time(4), TinyQuery);
    let repeated = state(&context(), &journal, time(4), TinyQuery);

    assert_eq!(first.payload(), repeated.payload());
    assert_eq!(first.logical_time(), repeated.logical_time());
}

#[test]
fn an_entry_creates_an_exact_timestamp_discontinuity() {
    let journal = journal(&[(0, GameJournalEntry::create_saucer()), (5, spawn(1))]);

    let immediately_before = state(&context(), &journal, time(4), TinyQuery);
    let exactly_at_entry = state(&context(), &journal, time(5), TinyQuery);

    assert_eq!(result(&immediately_before).visible_entries.len(), 1);
    assert_eq!(result(&exactly_at_entry).visible_entries.len(), 2);
    assert_eq!(exactly_at_entry.logical_time(), time(5));
}
