#![forbid(unsafe_code)]

use caravan_domain::GameJournalEntry;
use engine_journal::Journal as GameJournal;
use engine_sdk::{Context, GameState, Journal as SdkJournal, JournalEntry};
use engine_time::LogicalTime;

pub use engine_time::game_tick_index;

/// Supplies immutable journal entries to the direct query boundary.
pub trait JournalSource {
    /// The opaque game payload carried by each journal entry.
    type Payload;

    /// Returns entries visible at `target_time` in append order.
    fn visible_entries_at(&self, target_time: LogicalTime) -> Vec<&JournalEntry<Self::Payload>>;
}

impl<P> JournalSource for SdkJournal<P> {
    type Payload = P;

    fn visible_entries_at(&self, target_time: LogicalTime) -> Vec<&JournalEntry<Self::Payload>> {
        self.visible_at(target_time).collect()
    }
}

impl JournalSource for GameJournal {
    type Payload = GameJournalEntry;

    fn visible_entries_at(&self, target_time: LogicalTime) -> Vec<&JournalEntry<Self::Payload>> {
        self.visible_at(target_time).collect()
    }
}

/// Immutable inputs prepared for one direct indexed query.
pub struct QueryInput<'a, C, P> {
    context: &'a Context<C>,
    logical_time: LogicalTime,
    game_tick_index: i64,
    visible_entries: Vec<&'a JournalEntry<P>>,
}

impl<'a, C, P> QueryInput<'a, C, P> {
    fn new(
        context: &'a Context<C>,
        journal: &'a impl JournalSource<Payload = P>,
        logical_time: LogicalTime,
    ) -> Self {
        Self {
            context,
            logical_time,
            game_tick_index: game_tick_index(logical_time),
            visible_entries: journal.visible_entries_at(logical_time),
        }
    }

    /// Borrows the immutable SDK context envelope.
    pub fn context(&self) -> &Context<C> {
        self.context
    }

    /// Borrows the opaque context payload.
    pub fn context_payload(&self) -> &C {
        self.context.payload()
    }

    /// Returns the exact logical time selected for this query.
    pub const fn logical_time(&self) -> LogicalTime {
        self.logical_time
    }

    /// Returns the discrete game-tick index selected from the logical time.
    pub const fn game_tick_index(&self) -> i64 {
        self.game_tick_index
    }

    /// Borrows visible journal entries in their immutable append order.
    pub fn visible_entries(&self) -> impl Iterator<Item = &JournalEntry<P>> + '_ {
        self.visible_entries.iter().copied()
    }

    /// Returns the number of journal entries visible at the selected time.
    pub fn visible_entry_count(&self) -> usize {
        self.visible_entries.len()
    }
}

/// Defines one pure result-producing interpretation of a prepared query.
pub trait IndexedQuery<C, P> {
    /// The opaque result carried by the returned SDK game state.
    type Result;

    /// Produces the indexed result from immutable context and journal inputs.
    fn query(&self, input: QueryInput<'_, C, P>) -> Self::Result;
}

/// Evaluates a direct indexed query at the exact requested logical time.
pub fn state<C, J, Q>(
    context: &Context<C>,
    journal: &J,
    logical_time: LogicalTime,
    query: Q,
) -> GameState<Q::Result>
where
    J: JournalSource,
    Q: IndexedQuery<C, J::Payload>,
{
    let input = QueryInput::new(context, journal, logical_time);
    let result = query.query(input);

    GameState::new(logical_time, result)
}
