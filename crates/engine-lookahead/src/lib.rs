#![forbid(unsafe_code)]

use caravan_reference::{state, ReferenceWorldline, State};
use engine_branches::BranchKind;
use engine_time::LogicalTime;

pub use caravan_reference::{ReferenceWorldline as Worldline, Snapshot};
pub use engine_branches::BranchKind as ViewKind;
pub use engine_time::LogicalTime as QueryTime;

/// Queries one fixed branch through the reference oracle.
pub fn future(worldline: &ReferenceWorldline, logical_time: LogicalTime) -> State {
    state(worldline, logical_time)
}

/// Creates a read-only query view over one immutable branch.
pub fn branch_view(worldline: &ReferenceWorldline) -> BranchView<'_> {
    BranchView { worldline }
}

/// A read-only view of one immutable reference branch.
#[derive(Clone, Copy, Debug)]
pub struct BranchView<'a> {
    worldline: &'a ReferenceWorldline,
}

impl<'a> BranchView<'a> {
    /// Returns the branch role represented by this view.
    pub const fn kind(self) -> BranchKind {
        self.worldline.kind()
    }

    /// Returns the inclusive fork boundary for a child branch.
    pub const fn fork_boundary(self) -> Option<LogicalTime> {
        self.worldline.fork_boundary()
    }

    /// Queries this fixed branch through the reference oracle.
    pub fn query(self, logical_time: LogicalTime) -> State {
        state(self.worldline, logical_time)
    }
}
