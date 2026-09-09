use engine_branches::Branch;
use engine_observation::{snapshot, ObservationRequest, RenderSource};
use engine_presentation::{RenderBatch, RenderVertex};
use engine_sdk::{Context, Frame};
use engine_surface::{
    AppendRequest, GameSession, GameSurface, SurfaceManifest, SurfaceSnapshotRequest,
};
use engine_time::{LogicalTime, Tau};
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, Default)]
struct SyntheticSurface;

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
    type Context = ();
    type Fact = u8;
    type View = ();
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

    fn default_view(&self) -> Self::View {}

    fn view(&self, _view: &Self::View) -> Result<Value, Self::Error> {
        Ok(json!({ "kind": "synthetic" }))
    }

    fn update_view(&self, _view: &mut Self::View, update: Value) -> Result<Value, Self::Error> {
        Ok(update)
    }

    fn encode_fact(&self, fact: &Self::Fact) -> Value {
        json!(fact)
    }

    fn decode_fact(&self, value: Value) -> Result<Self::Fact, Self::Error> {
        value
            .as_u64()
            .and_then(|value| u8::try_from(value).ok())
            .ok_or_else(|| "synthetic facts are u8 JSON numbers".to_owned())
    }

    fn observe(
        &self,
        branch: &Branch<(), u8>,
        _view: &Self::View,
        request: SurfaceSnapshotRequest,
    ) -> Result<engine_observation::RenderSnapshot, Self::Error> {
        snapshot(
            &SyntheticSource { branch },
            request.logical_time,
            request.tau,
            request.observation,
        )
        .map_err(|error| error.to_string())
    }
}

#[test]
fn generic_surface_can_party_on_a_branch_and_return_to_actual_history() {
    let mut session = GameSession::new(
        SyntheticSurface,
        Branch::new(Context::new(()), engine_journal::Journal::empty()),
    )
    .expect("synthetic actual should initialize");
    let actual = session
        .commit_append(AppendRequest {
            branch_id: 0,
            expected_revision: 0,
            logical_time: LogicalTime::zero(),
            facts: vec![json!(1)],
        })
        .expect("actual append should commit");
    let branch = session
        .begin_counterfactual(0, actual.new_revision, LogicalTime::zero())
        .expect("counterfactual should open");
    let preview = session
        .preview_append(AppendRequest {
            branch_id: branch.branch_id,
            expected_revision: branch.revision,
            logical_time: LogicalTime::from_ticks(1),
            facts: vec![json!(2)],
        })
        .expect("preview should validate");
    assert_eq!(preview.current_revision, branch.revision);

    session
        .commit_append(AppendRequest {
            branch_id: branch.branch_id,
            expected_revision: branch.revision,
            logical_time: LogicalTime::from_ticks(1),
            facts: vec![json!(2)],
        })
        .expect("branch append should commit");
    assert_eq!(session.journal(0).expect("actual exists").facts.len(), 1);
    assert_eq!(
        session
            .journal(branch.branch_id)
            .expect("branch exists")
            .facts
            .len(),
        2
    );
    assert_eq!(
        session
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

    session
        .discard_branch(branch.branch_id)
        .expect("branch should discard");
    assert_eq!(
        session
            .journal(0)
            .expect("actual should remain")
            .facts
            .len(),
        1
    );
}
