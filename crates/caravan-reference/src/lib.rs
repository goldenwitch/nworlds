#![forbid(unsafe_code)]

mod discontinuities;
mod projection;
mod snapshot;

use caravan_domain::Saucer;
use engine_branches::Worldline as BranchWorldline;
use engine_journal::Journal;
use engine_sdk::{Context, GameState};
use engine_time::LogicalTime;

pub use caravan_vegetation::IndexedTile;
pub use discontinuities::{
    discontinuity_index, ActorThreshold, CaravanBreakpointSource, DiscontinuityIndex, PieceInput,
    RuleThreshold,
};
pub use projection::{project, project_query, project_with_index};
pub use snapshot::Snapshot;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct ReferenceContext {
    saucer: Saucer,
}

impl ReferenceContext {
    pub const fn new() -> Self {
        Self {
            saucer: Saucer::new(),
        }
    }

    pub const fn saucer(self) -> Saucer {
        self.saucer
    }
}

pub type ReferenceWorldline = BranchWorldline<ReferenceContext>;
pub type Worldline = ReferenceWorldline;
pub type State = GameState<Snapshot>;

pub fn context() -> Context<ReferenceContext> {
    Context::new(ReferenceContext::new())
}

pub fn actual(journal: Journal) -> ReferenceWorldline {
    ReferenceWorldline::actual(context(), journal)
}

pub fn state(worldline: &ReferenceWorldline, logical_time: LogicalTime) -> State {
    project(worldline, logical_time)
}

pub fn query(
    context: &Context<ReferenceContext>,
    journal: &Journal,
    logical_time: LogicalTime,
) -> State {
    project_query(context, journal, logical_time)
}
