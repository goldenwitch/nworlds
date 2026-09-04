use engine_api::{
    present, state, Branch, Context, GameState, IndexedQuery, JournalWriter, LogicalTime,
    QueryInput, Renderer, Tau, Worldline,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct Definitions;

struct CountVisibleEntries;

impl IndexedQuery<Definitions, u8> for CountVisibleEntries {
    type Result = usize;

    fn query(&self, input: QueryInput<'_, Definitions, u8>) -> Self::Result {
        input.visible_entry_count()
    }
}

struct CountRenderer;

impl Renderer<usize> for CountRenderer {
    type Output = (usize, Tau);

    fn render(state: &GameState<usize>, tau: Tau) -> Self::Output {
        (*state.payload(), tau)
    }
}

fn worldline() -> Worldline<Definitions, u8> {
    let mut writer = JournalWriter::<u8>::new();
    writer.record(1);
    writer
        .advance_to(LogicalTime::from_ticks(10))
        .expect("consumer timestamps are monotonic");
    writer.record(2);
    Branch::new(Context::new(Definitions), writer.finish())
}

#[test]
fn external_consumer_composes_generic_query_branch_and_presentation() {
    let worldline = worldline();
    let before_second_entry = state(
        worldline.context(),
        worldline.journal(),
        LogicalTime::from_ticks(9),
        CountVisibleEntries,
    );
    assert_eq!(*before_second_entry.payload(), 1);

    let frame = present::<usize, CountRenderer>(&before_second_entry, Tau::from_ticks(3));
    assert_eq!(frame.payload(), &(1, Tau::from_ticks(3)));
}

#[test]
fn external_consumer_queries_an_immutable_branch_without_query_history() {
    let parent = worldline();
    let mut suffix_writer = JournalWriter::<u8>::new();
    suffix_writer
        .advance_to(LogicalTime::from_ticks(20))
        .expect("branch suffix timestamp is representable");
    suffix_writer.record(3);
    let child = parent
        .counterfactual(LogicalTime::from_ticks(10), &suffix_writer.finish())
        .expect("suffix is after the inclusive branch boundary");

    let parent_late = state(
        parent.context(),
        parent.journal(),
        LogicalTime::from_ticks(20),
        CountVisibleEntries,
    );
    let child_early = state(
        child.context(),
        child.journal(),
        LogicalTime::from_ticks(9),
        CountVisibleEntries,
    );
    let child_late = state(
        child.context(),
        child.journal(),
        LogicalTime::from_ticks(20),
        CountVisibleEntries,
    );

    assert_eq!(*parent_late.payload(), 2);
    assert_eq!(*child_early.payload(), 1);
    assert_eq!(*child_late.payload(), 3);
    assert_eq!(
        *state(
            child.context(),
            child.journal(),
            LogicalTime::from_ticks(9),
            CountVisibleEntries,
        )
        .payload(),
        1
    );
}
