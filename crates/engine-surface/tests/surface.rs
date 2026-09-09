use engine_branches::Branch;
use engine_observation::{snapshot, ObservationRequest, RenderSource};
use engine_presentation::{RenderBatch, RenderVertex};
use engine_sdk::{Context, Frame};
use engine_surface::{
    AppendPreview, AppendRequest, AppendResult, BranchDescriptor, BranchId, BranchSession,
    GameSurface, JournalView, Revision, SurfaceManifest, SurfaceSnapshotRequest,
};
use engine_time::{LogicalTime, Tau};
use serde_json::{json, Value};

struct SyntheticSurface {
    session: BranchSession<(), u8>,
}

impl SyntheticSurface {
    fn new() -> Self {
        Self {
            session: BranchSession::new(Branch::new(
                Context::new(()),
                engine_journal::Journal::empty(),
            ))
            .expect("synthetic actual should initialize"),
        }
    }

    fn decode_facts(facts: Vec<Value>) -> Result<Vec<u8>, String> {
        facts
            .into_iter()
            .map(|fact| {
                fact.as_u64()
                    .and_then(|value| u8::try_from(value).ok())
                    .ok_or_else(|| "synthetic facts are u8 JSON numbers".to_owned())
            })
            .collect()
    }
}

struct SyntheticSource<'a> {
    branch: &'a engine_branches::Branch<(), u8>,
}

impl RenderSource for SyntheticSource<'_> {
    type Error = std::convert::Infallible;

    fn render(
        &self,
        logical_time: LogicalTime,
        tau: Tau,
    ) -> Result<Frame<RenderBatch>, Self::Error> {
        let count = self.branch.journal().visible_at(logical_time).count() as f32;
        let color = [count / (count + 1.0), 0.2, 0.8, 1.0];
        Ok(Frame::new(
            tau,
            RenderBatch::new([
                RenderVertex::new([-0.8, -0.8, 0.0], color),
                RenderVertex::new([0.8, -0.8, 0.0], color),
                RenderVertex::new([0.0, 0.8, 0.0], color),
            ]),
        ))
    }
}

impl GameSurface for SyntheticSurface {
    type Error = String;

    fn manifest(&self) -> SurfaceManifest {
        SurfaceManifest {
            name: "synthetic-surface".to_owned(),
            capabilities: vec![
                "counterfactual-branches".to_owned(),
                "commit-append".to_owned(),
            ],
            fact_schema: json!({ "type": "integer", "minimum": 0, "maximum": 255 }),
            view_schema: json!({ "type": "object" }),
        }
    }

    fn view(&self) -> Result<Value, Self::Error> {
        Ok(json!({ "kind": "synthetic" }))
    }

    fn update_view(&mut self, update: Value) -> Result<Value, Self::Error> {
        Ok(update)
    }

    fn observe(
        &self,
        request: SurfaceSnapshotRequest,
    ) -> Result<engine_observation::RenderSnapshot, Self::Error> {
        let branch = self
            .session
            .branch(request.branch_id)
            .map_err(|error| error.to_string())?;
        snapshot(
            &SyntheticSource { branch },
            request.logical_time,
            request.tau,
            request.observation,
        )
        .map_err(|error| error.to_string())
    }

    fn journal(&self, branch_id: BranchId) -> Result<JournalView, Self::Error> {
        let descriptor = self
            .session
            .descriptor(branch_id)
            .map_err(|error| error.to_string())?;
        let facts = self
            .session
            .branch(branch_id)
            .map_err(|error| error.to_string())?
            .journal()
            .iter()
            .map(|entry| json!({ "logical_time_ticks": entry.logical_time().ticks(), "fact": entry.payload() }))
            .collect();
        Ok(JournalView { descriptor, facts })
    }

    fn begin_counterfactual(
        &mut self,
        parent_id: BranchId,
        expected_revision: Revision,
        fork_boundary: LogicalTime,
    ) -> Result<BranchDescriptor, Self::Error> {
        self.session
            .begin_counterfactual(parent_id, expected_revision, fork_boundary)
            .map_err(|error| error.to_string())
    }

    fn preview_append(&self, request: AppendRequest) -> Result<AppendPreview, Self::Error> {
        let facts = Self::decode_facts(request.facts)?;
        let fact_count = facts.len();
        let descriptor = self
            .session
            .preview_append(
                request.branch_id,
                request.expected_revision,
                request.logical_time,
                facts,
            )
            .map_err(|error| error.to_string())?;
        Ok(AppendPreview {
            branch_id: descriptor.branch_id,
            current_revision: request.expected_revision,
            logical_time: request.logical_time,
            fact_count,
        })
    }

    fn commit_append(&mut self, request: AppendRequest) -> Result<AppendResult, Self::Error> {
        let facts = Self::decode_facts(request.facts)?;
        let fact_count = facts.len();
        let descriptor = self
            .session
            .append(
                request.branch_id,
                request.expected_revision,
                request.logical_time,
                facts,
            )
            .map_err(|error| error.to_string())?;
        Ok(AppendResult {
            branch_id: descriptor.branch_id,
            new_revision: descriptor.revision,
            logical_time: request.logical_time,
            fact_count,
        })
    }

    fn discard_branch(&mut self, branch_id: BranchId) -> Result<(), Self::Error> {
        self.session
            .discard(branch_id)
            .map_err(|error| error.to_string())
    }
}

#[test]
fn generic_surface_can_party_on_a_branch_and_return_to_actual_history() {
    let mut surface = SyntheticSurface::new();
    let actual = surface
        .commit_append(AppendRequest {
            branch_id: 0,
            expected_revision: 0,
            logical_time: LogicalTime::zero(),
            facts: vec![json!(1)],
        })
        .expect("actual append should commit");
    let branch = surface
        .begin_counterfactual(0, actual.new_revision, LogicalTime::zero())
        .expect("counterfactual should open");
    let preview = surface
        .preview_append(AppendRequest {
            branch_id: branch.branch_id,
            expected_revision: branch.revision,
            logical_time: LogicalTime::from_ticks(1),
            facts: vec![json!(2)],
        })
        .expect("preview should validate");
    assert_eq!(preview.current_revision, branch.revision);

    surface
        .commit_append(AppendRequest {
            branch_id: branch.branch_id,
            expected_revision: branch.revision,
            logical_time: LogicalTime::from_ticks(1),
            facts: vec![json!(2)],
        })
        .expect("branch append should commit");
    assert_eq!(surface.journal(0).expect("actual exists").facts.len(), 1);
    assert_eq!(
        surface
            .journal(branch.branch_id)
            .expect("branch exists")
            .facts
            .len(),
        2
    );
    assert_eq!(
        surface
            .observe(SurfaceSnapshotRequest {
                branch_id: branch.branch_id,
                logical_time: LogicalTime::from_ticks(1),
                tau: Tau::zero(),
                observation: ObservationRequest::new(16, 16),
            })
            .expect("branch should render")
            .triangle_count(),
        1
    );

    surface
        .discard_branch(branch.branch_id)
        .expect("branch should discard");
    assert_eq!(
        surface
            .journal(0)
            .expect("actual should remain")
            .facts
            .len(),
        1
    );
}
