//! The recommended engine integration shape for the Caravan sample.
//!
//! Caravan owns its domain facts, interaction meaning, and render vocabulary.
//! This module is the narrow composition seam that specializes the generic
//! temporal engine with those values: worldlines, journal authoring, direct
//! queries, immutable publication, persistence, and state-first presentation.

use caravan_domain::GameJournalEntry;
use caravan_reference::{
    actual, try_state as reference_try_state, Journal, ReferenceWorldline, State,
};

pub use crate::render::project_output;
pub use caravan_reference::ProjectionError;
pub use engine_api::JournalWriterError;
pub use engine_api::{
    present, BranchError, BranchKind, Frame, GameState, IndexedQuery, JournalWriter, LogicalTime,
    PresentationDriver, QueryInput, RenderBatch, RenderVertex, Renderer, Tau, Worldline,
    TICKS_PER_LOGICAL_SECOND,
};

/// The Caravan specialization of the engine's immutable worldline.
pub type CaravanWorldline = ReferenceWorldline;

/// The Caravan specialization of the engine's immutable journal.
pub type CaravanJournal = Journal;

/// The Caravan specialization of the engine's journal authoring cursor.
pub type CaravanJournalWriter = JournalWriter<GameJournalEntry>;

/// The Caravan state produced by the reference query.
pub type CaravanState = State;

/// The engine frame envelope specialized by a Caravan renderer.
pub type CaravanFrame<Output> = Frame<Output>;

/// Builds an actual Caravan worldline from an immutable journal.
pub fn actual_worldline(journal: CaravanJournal) -> CaravanWorldline {
    actual(journal)
}

/// Queries Caravan state while preserving projection errors at the seam.
pub fn try_state(
    worldline: &CaravanWorldline,
    logical_time: LogicalTime,
) -> Result<CaravanState, ProjectionError> {
    reference_try_state(worldline, logical_time)
}

/// Queries Caravan state at an arbitrary logical time.
pub fn state(worldline: &CaravanWorldline, logical_time: LogicalTime) -> CaravanState {
    caravan_reference::state(worldline, logical_time)
}

/// Presents a Caravan state through the engine's state-first frame boundary.
pub fn present_state<R>(state: &CaravanState, tau: Tau) -> Frame<R::Output>
where
    R: Renderer<caravan_reference::Snapshot>,
{
    let mut driver = PresentationDriver::new(state.clone());
    driver.set_visual_time(tau);
    driver.present::<R>()
}

/// Rebuilds the game-facing timestamp writer from one immutable journal.
pub fn writer_from_journal(
    journal: &CaravanJournal,
) -> Result<CaravanJournalWriter, JournalWriterError> {
    let mut writer = CaravanJournalWriter::new();
    for entry in journal.iter() {
        writer.advance_to(entry.logical_time())?;
        writer.record(*entry.payload());
    }
    Ok(writer)
}

/// Appends one authoritative fact and returns a new immutable actual worldline.
pub fn append_actual(
    worldline: &CaravanWorldline,
    writer: &mut CaravanJournalWriter,
    payload: GameJournalEntry,
) -> CaravanWorldline {
    writer.record(payload);
    Worldline::new(worldline.context().clone(), writer.snapshot())
}

/// Encodes the selected Caravan worldline through its game-owned codec.
pub fn encode(
    worldline: &CaravanWorldline,
) -> Result<Vec<u8>, caravan_persistence::PersistenceError> {
    caravan_persistence::encode(worldline)
}

/// Decodes a Caravan worldline through its game-owned codec.
pub fn decode(bytes: &[u8]) -> Result<CaravanWorldline, caravan_persistence::PersistenceError> {
    caravan_persistence::decode(bytes)
}
