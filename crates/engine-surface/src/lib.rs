#![forbid(unsafe_code)]

use std::{error::Error, fmt};

use engine_branches::{Branch, BranchError, BranchKind};
use engine_observation::{ObservationRequest, RenderSnapshot};
use engine_time::{LogicalTime, Tau};
use serde_json::{json, Value};

pub type BranchId = u64;
pub type Revision = u64;

/// A stable handle for one immutable history retained by an agent session.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BranchDescriptor {
    pub branch_id: BranchId,
    pub revision: Revision,
    pub parent: Option<BranchId>,
    pub kind: BranchKind,
    pub fork_boundary: Option<LogicalTime>,
}

/// Errors raised by the generic immutable branch session.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SurfaceError {
    InvalidActual,
    UnknownBranch(BranchId),
    ActualCannotBeDiscarded,
    RevisionConflict {
        branch_id: BranchId,
        expected: Revision,
        actual: Revision,
    },
    EmptyAppend,
    Branch(BranchError),
}

impl fmt::Display for SurfaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidActual => formatter.write_str("surface session requires an actual branch"),
            Self::UnknownBranch(branch_id) => write!(formatter, "unknown branch {branch_id}"),
            Self::ActualCannotBeDiscarded => {
                formatter.write_str("the actual branch cannot be discarded")
            }
            Self::RevisionConflict {
                branch_id,
                expected,
                actual,
            } => write!(
                formatter,
                "branch {branch_id} revision conflict: expected {expected}, actual {actual}"
            ),
            Self::EmptyAppend => formatter.write_str("an append must contain at least one fact"),
            Self::Branch(error) => error.fmt(formatter),
        }
    }
}

impl Error for SurfaceError {}

impl From<BranchError> for SurfaceError {
    fn from(error: BranchError) -> Self {
        Self::Branch(error)
    }
}

#[derive(Debug)]
pub enum GameSessionError<E> {
    Surface(SurfaceError),
    Definition(E),
}

impl<E: fmt::Display> fmt::Display for GameSessionError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Surface(error) => error.fmt(formatter),
            Self::Definition(error) => error.fmt(formatter),
        }
    }
}

impl<E> From<SurfaceError> for GameSessionError<E> {
    fn from(error: SurfaceError) -> Self {
        Self::Surface(error)
    }
}

struct History<C, P> {
    branch: Branch<C, P>,
    revision: Revision,
    parent: Option<BranchId>,
}

/// Owns actual and speculative immutable histories for one agent-facing session.
pub struct BranchSession<C, P> {
    histories: Vec<Option<History<C, P>>>,
}

impl<C, P: Clone> BranchSession<C, P> {
    pub fn new(actual: Branch<C, P>) -> Result<Self, SurfaceError> {
        if !actual.is_actual() {
            return Err(SurfaceError::InvalidActual);
        }
        Ok(Self {
            histories: vec![Some(History {
                branch: actual,
                revision: 0,
                parent: None,
            })],
        })
    }

    pub const fn actual_id() -> BranchId {
        0
    }

    pub fn descriptor(&self, branch_id: BranchId) -> Result<BranchDescriptor, SurfaceError> {
        let history = self.history(branch_id)?;
        Ok(BranchDescriptor {
            branch_id,
            revision: history.revision,
            parent: history.parent,
            kind: history.branch.kind(),
            fork_boundary: history.branch.fork_boundary(),
        })
    }

    pub fn branch(&self, branch_id: BranchId) -> Result<&Branch<C, P>, SurfaceError> {
        Ok(&self.history(branch_id)?.branch)
    }

    pub fn begin_counterfactual(
        &mut self,
        parent_id: BranchId,
        expected_revision: Revision,
        fork_boundary: LogicalTime,
    ) -> Result<BranchDescriptor, SurfaceError> {
        let parent = self.history(parent_id)?;
        if parent.revision != expected_revision {
            return Err(SurfaceError::RevisionConflict {
                branch_id: parent_id,
                expected: expected_revision,
                actual: parent.revision,
            });
        }
        let child = parent
            .branch
            .counterfactual(fork_boundary, &engine_journal::Journal::empty())?;
        let branch_id = self.histories.len() as BranchId;
        self.histories.push(Some(History {
            branch: child,
            revision: 0,
            parent: Some(parent_id),
        }));
        self.descriptor(branch_id)
    }

    pub fn append(
        &mut self,
        branch_id: BranchId,
        expected_revision: Revision,
        logical_time: LogicalTime,
        facts: impl IntoIterator<Item = P>,
    ) -> Result<BranchDescriptor, SurfaceError> {
        let facts = facts.into_iter().collect::<Vec<_>>();
        if facts.is_empty() {
            return Err(SurfaceError::EmptyAppend);
        }
        let history = self.history_mut(branch_id)?;
        if history.revision != expected_revision {
            return Err(SurfaceError::RevisionConflict {
                branch_id,
                expected: expected_revision,
                actual: history.revision,
            });
        }
        history.branch = history.branch.append_at(logical_time, facts)?;
        history.revision = history.revision.saturating_add(1);
        self.descriptor(branch_id)
    }

    pub fn preview_append(
        &self,
        branch_id: BranchId,
        expected_revision: Revision,
        logical_time: LogicalTime,
        facts: impl IntoIterator<Item = P>,
    ) -> Result<BranchDescriptor, SurfaceError> {
        let facts = facts.into_iter().collect::<Vec<_>>();
        if facts.is_empty() {
            return Err(SurfaceError::EmptyAppend);
        }
        let history = self.history(branch_id)?;
        if history.revision != expected_revision {
            return Err(SurfaceError::RevisionConflict {
                branch_id,
                expected: expected_revision,
                actual: history.revision,
            });
        }
        let _ = history.branch.append_at(logical_time, facts)?;
        Ok(BranchDescriptor {
            branch_id,
            revision: history.revision.saturating_add(1),
            parent: history.parent,
            kind: history.branch.kind(),
            fork_boundary: history.branch.fork_boundary(),
        })
    }

    pub fn discard(&mut self, branch_id: BranchId) -> Result<(), SurfaceError> {
        if branch_id == Self::actual_id() {
            return Err(SurfaceError::ActualCannotBeDiscarded);
        }
        let slot = self
            .histories
            .get_mut(branch_id as usize)
            .ok_or(SurfaceError::UnknownBranch(branch_id))?;
        if slot.take().is_none() {
            return Err(SurfaceError::UnknownBranch(branch_id));
        }
        Ok(())
    }

    fn history(&self, branch_id: BranchId) -> Result<&History<C, P>, SurfaceError> {
        self.histories
            .get(branch_id as usize)
            .and_then(Option::as_ref)
            .ok_or(SurfaceError::UnknownBranch(branch_id))
    }

    fn history_mut(&mut self, branch_id: BranchId) -> Result<&mut History<C, P>, SurfaceError> {
        self.histories
            .get_mut(branch_id as usize)
            .and_then(Option::as_mut)
            .ok_or(SurfaceError::UnknownBranch(branch_id))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceManifest {
    pub name: String,
    pub capabilities: Vec<String>,
    pub fact_schema: Value,
    pub view_schema: Value,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceSnapshotRequest {
    pub branch_id: BranchId,
    pub logical_time: LogicalTime,
    pub tau: Tau,
    pub observation: ObservationRequest,
}

#[derive(Clone, Debug, PartialEq)]
pub struct JournalView {
    pub descriptor: BranchDescriptor,
    pub facts: Vec<Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppendRequest {
    pub branch_id: BranchId,
    pub expected_revision: Revision,
    pub logical_time: LogicalTime,
    pub facts: Vec<Value>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppendPreview {
    pub branch_id: BranchId,
    pub current_revision: Revision,
    pub logical_time: LogicalTime,
    pub fact_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppendResult {
    pub branch_id: BranchId,
    pub new_revision: Revision,
    pub logical_time: LogicalTime,
    pub fact_count: usize,
}

/// The package-owned definition of a game surface.
pub trait GameSurface {
    type Context;
    type Fact: Clone;
    type View: Clone;
    type Error: fmt::Display;

    fn manifest(&self) -> SurfaceManifest;
    fn default_view(&self) -> Self::View;
    fn view(&self, view: &Self::View) -> Result<Value, Self::Error>;
    fn update_view(&self, view: &mut Self::View, update: Value) -> Result<Value, Self::Error>;
    fn encode_fact(&self, fact: &Self::Fact) -> Value;
    fn decode_fact(&self, value: Value) -> Result<Self::Fact, Self::Error>;
    fn observe(
        &self,
        branch: &Branch<Self::Context, Self::Fact>,
        view: &Self::View,
        request: SurfaceSnapshotRequest,
    ) -> Result<RenderSnapshot, Self::Error>;
}

/// A local session combining one game-surface definition with context, history, and view state.
pub struct GameSession<S>
where
    S: GameSurface,
{
    surface: S,
    histories: BranchSession<S::Context, S::Fact>,
    view: S::View,
}

impl<S> GameSession<S>
where
    S: GameSurface,
{
    pub fn new(surface: S, actual: Branch<S::Context, S::Fact>) -> Result<Self, SurfaceError> {
        let view = surface.default_view();
        Ok(Self {
            surface,
            histories: BranchSession::new(actual)?,
            view,
        })
    }

    pub fn definition(&self) -> &S {
        &self.surface
    }

    pub fn manifest(&self) -> SurfaceManifest {
        self.surface.manifest()
    }

    pub fn view(&self) -> Result<Value, GameSessionError<S::Error>> {
        self.surface
            .view(&self.view)
            .map_err(GameSessionError::Definition)
    }

    pub fn update_view(&mut self, update: Value) -> Result<Value, GameSessionError<S::Error>> {
        self.surface
            .update_view(&mut self.view, update)
            .map_err(GameSessionError::Definition)
    }

    pub fn observe(
        &self,
        request: SurfaceSnapshotRequest,
    ) -> Result<RenderSnapshot, GameSessionError<S::Error>> {
        let branch = self.histories.branch(request.branch_id)?;
        self.surface
            .observe(branch, &self.view, request)
            .map_err(GameSessionError::Definition)
    }

    pub fn journal(&self, branch_id: BranchId) -> Result<JournalView, GameSessionError<S::Error>> {
        let descriptor = self.histories.descriptor(branch_id)?;
        let facts = self
            .histories
            .branch(branch_id)?
            .journal()
            .iter()
            .map(|entry| {
                json!({
                    "logical_time_ticks": entry.logical_time().ticks(),
                    "fact": self.surface.encode_fact(entry.payload()),
                })
            })
            .collect();
        Ok(JournalView { descriptor, facts })
    }

    pub fn begin_counterfactual(
        &mut self,
        parent_id: BranchId,
        expected_revision: Revision,
        fork_boundary: LogicalTime,
    ) -> Result<BranchDescriptor, GameSessionError<S::Error>> {
        Ok(self
            .histories
            .begin_counterfactual(parent_id, expected_revision, fork_boundary)?)
    }

    pub fn preview_append(
        &self,
        request: AppendRequest,
    ) -> Result<AppendPreview, GameSessionError<S::Error>> {
        let fact_count = request.facts.len();
        let facts = request
            .facts
            .into_iter()
            .map(|fact| self.surface.decode_fact(fact))
            .collect::<Result<Vec<_>, _>>()
            .map_err(GameSessionError::Definition)?;
        let descriptor = self.histories.preview_append(
            request.branch_id,
            request.expected_revision,
            request.logical_time,
            facts,
        )?;
        Ok(AppendPreview {
            branch_id: descriptor.branch_id,
            current_revision: request.expected_revision,
            logical_time: request.logical_time,
            fact_count,
        })
    }

    pub fn commit_append(
        &mut self,
        request: AppendRequest,
    ) -> Result<AppendResult, GameSessionError<S::Error>> {
        let fact_count = request.facts.len();
        let facts = request
            .facts
            .into_iter()
            .map(|fact| self.surface.decode_fact(fact))
            .collect::<Result<Vec<_>, _>>()
            .map_err(GameSessionError::Definition)?;
        let descriptor = self.histories.append(
            request.branch_id,
            request.expected_revision,
            request.logical_time,
            facts,
        )?;
        Ok(AppendResult {
            branch_id: descriptor.branch_id,
            new_revision: descriptor.revision,
            logical_time: request.logical_time,
            fact_count,
        })
    }

    pub fn discard_branch(
        &mut self,
        branch_id: BranchId,
    ) -> Result<(), GameSessionError<S::Error>> {
        self.histories.discard(branch_id)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{BranchSession, SurfaceError};
    use engine_branches::Branch;
    use engine_journal::Journal;
    use engine_sdk::Context;
    use engine_time::LogicalTime;

    fn actual() -> Branch<(), u8> {
        Branch::new(Context::new(()), Journal::empty())
    }

    #[test]
    fn speculative_branch_commit_and_discard_preserve_actual_history() {
        let mut session = BranchSession::new(actual()).expect("actual branch should initialize");
        let branch = session
            .begin_counterfactual(0, 0, LogicalTime::zero())
            .expect("counterfactual should open");
        let committed = session
            .append(branch.branch_id, 0, LogicalTime::from_ticks(1), [7])
            .expect("branch append should commit");

        assert_eq!(session.descriptor(0).expect("actual exists").revision, 0);
        assert_eq!(committed.revision, 1);
        assert_eq!(session.branch(0).expect("actual exists").journal().len(), 0);
        assert_eq!(
            session
                .branch(branch.branch_id)
                .expect("branch exists")
                .journal()
                .len(),
            1
        );

        session
            .discard(branch.branch_id)
            .expect("branch should discard");
        assert!(matches!(
            session.descriptor(branch.branch_id),
            Err(SurfaceError::UnknownBranch(_))
        ));
        assert_eq!(session.branch(0).expect("actual exists").journal().len(), 0);
    }

    #[test]
    fn stale_revision_and_actual_discard_are_rejected() {
        let mut session = BranchSession::new(actual()).expect("actual branch should initialize");
        let error = session
            .append(0, 1, LogicalTime::zero(), [1])
            .expect_err("stale actual revision should fail");
        assert!(matches!(error, SurfaceError::RevisionConflict { .. }));
        assert!(matches!(
            session.discard(0),
            Err(SurfaceError::ActualCannotBeDiscarded)
        ));
    }
}
