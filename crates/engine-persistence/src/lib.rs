#![forbid(unsafe_code)]

mod format;

pub use caravan_reference::{ReferenceContext, ReferenceWorldline, Snapshot, State};
pub use engine_sdk::LogicalTime;
pub use format::{
    decode, encode, load, save, BranchLineage, PersistenceError, FORMAT_MAGIC, FORMAT_VERSION,
};

/// Returns the branch lineage represented by a reference worldline.
pub fn branch_lineage(worldline: &ReferenceWorldline) -> BranchLineage {
    BranchLineage::from_worldline(worldline)
}

/// Replays direct reference queries in the supplied order.
pub fn replay(
    worldline: &ReferenceWorldline,
    logical_times: impl IntoIterator<Item = LogicalTime>,
) -> Vec<State> {
    logical_times
        .into_iter()
        .map(|logical_time| caravan_reference::state(worldline, logical_time))
        .collect()
}

/// Loads a saved worldline and replays direct reference queries in the supplied order.
pub fn replay_bytes(
    bytes: &[u8],
    logical_times: impl IntoIterator<Item = LogicalTime>,
) -> Result<Vec<State>, PersistenceError> {
    let worldline = decode(bytes)?;
    Ok(replay(&worldline, logical_times))
}

/// Loads a saved worldline from a path and replays direct reference queries in the supplied order.
pub fn replay_path(
    path: impl AsRef<std::path::Path>,
    logical_times: impl IntoIterator<Item = LogicalTime>,
) -> Result<Vec<State>, PersistenceError> {
    let worldline = load(path)?;
    Ok(replay(&worldline, logical_times))
}
