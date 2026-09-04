use engine_branches::BranchKind;
use engine_time::LogicalTime;

use crate::{state, ReferenceWorldline, State};

pub use engine_branches::BranchKind as ViewKind;

pub fn future(worldline: &ReferenceWorldline, logical_time: LogicalTime) -> State {
    state(worldline, logical_time)
}

pub fn branch_view(worldline: &ReferenceWorldline) -> BranchView<'_> {
    BranchView { worldline }
}

#[derive(Clone, Copy, Debug)]
pub struct BranchView<'a> {
    worldline: &'a ReferenceWorldline,
}

impl<'a> BranchView<'a> {
    pub const fn kind(self) -> BranchKind {
        self.worldline.kind()
    }

    pub const fn fork_boundary(self) -> Option<LogicalTime> {
        self.worldline.fork_boundary()
    }

    pub fn query(self, logical_time: LogicalTime) -> State {
        state(self.worldline, logical_time)
    }
}
