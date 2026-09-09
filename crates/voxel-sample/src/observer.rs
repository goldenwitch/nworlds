use std::convert::Infallible;

use engine_api::{
    AppendPreview, AppendRequest, AppendResult, BranchDescriptor, BranchId, BranchSession, Frame,
    GameSurface, JournalView, LogicalTime, Revision, SurfaceManifest, SurfaceSnapshotRequest, Tau,
};
use engine_observation::snapshot;
use engine_observation::RenderSnapshot;
use engine_observation::RenderSource;
use serde_json::{json, Value};

use crate::camera::Camera;
use crate::engine_integration::{
    cottage_worldline, frame_with_camera, state, VoxelFrame, VoxelWorldline,
};
use crate::world::{BlockKind, VoxelContext, VoxelFact, VoxelPosition, VoxelScale, VoxelTool};

/// A reusable observation source for the voxel game's explicit render path.
#[derive(Clone, Debug)]
pub struct VoxelObservationSource {
    worldline: VoxelWorldline,
    camera: Camera,
}

impl VoxelObservationSource {
    pub fn new(worldline: VoxelWorldline, camera: Camera) -> Self {
        Self { worldline, camera }
    }

    pub fn worldline(&self) -> &VoxelWorldline {
        &self.worldline
    }
}

impl Default for VoxelObservationSource {
    fn default() -> Self {
        let (worldline, _) = cottage_worldline();
        Self::new(worldline, Camera::default())
    }
}

impl RenderSource for VoxelObservationSource {
    type Error = Infallible;

    fn render(
        &self,
        logical_time: LogicalTime,
        tau: Tau,
    ) -> Result<Frame<engine_api::RenderBatch>, Self::Error> {
        let sampled = state(&self.worldline, logical_time);
        let frame: VoxelFrame = frame_with_camera(&sampled, self.camera, tau);
        Ok(frame)
    }
}

/// The voxel package's GameSurface implementation for generic agent tooling.
pub struct VoxelGameSurface {
    session: BranchSession<VoxelContext, VoxelFact>,
    camera: Camera,
}

impl VoxelGameSurface {
    pub fn new() -> Self {
        let (worldline, _) = cottage_worldline();
        Self {
            session: BranchSession::new(worldline).expect("cottage worldline is actual"),
            camera: Camera::default(),
        }
    }

    fn decode_facts(facts: Vec<Value>) -> Result<Vec<VoxelFact>, String> {
        facts.into_iter().map(decode_fact).collect()
    }

    fn snapshot_for(&self, request: SurfaceSnapshotRequest) -> Result<RenderSnapshot, String> {
        let branch = self
            .session
            .branch(request.branch_id)
            .map_err(|error| error.to_string())?;
        let source = VoxelBranchSource {
            worldline: branch,
            camera: self.camera,
        };
        snapshot(
            &source,
            request.logical_time,
            request.tau,
            request.observation,
        )
        .map_err(|error| error.to_string())
    }
}

impl Default for VoxelGameSurface {
    fn default() -> Self {
        Self::new()
    }
}

impl GameSurface for VoxelGameSurface {
    type Error = String;

    fn manifest(&self) -> SurfaceManifest {
        SurfaceManifest {
            name: "voxel-sample".to_owned(),
            capabilities: vec![
                "explicit-observation".to_owned(),
                "journal-read".to_owned(),
                "counterfactual-branches".to_owned(),
                "preview-append".to_owned(),
                "commit-append".to_owned(),
                "discard-branch".to_owned(),
            ],
            fact_schema: json!({
                "type": "object",
                "description": "One voxel journal fact.",
                "oneOf": [
                    { "properties": { "kind": { "const": "Place" }, "position": { "type": "object" }, "block": { "type": "string" } }, "required": ["kind", "position", "block"] },
                    { "properties": { "kind": { "const": "Remove" }, "position": { "type": "object" } }, "required": ["kind", "position"] },
                    { "properties": { "kind": { "const": "SpawnFire" }, "position": { "type": "object" } }, "required": ["kind", "position"] },
                    { "properties": { "kind": { "const": "SelectTool" }, "tool": { "enum": ["Remove", "Fire"] } }, "required": ["kind", "tool"] },
                    { "properties": { "kind": { "const": "SetScale" }, "milli": { "type": "integer", "minimum": 350, "maximum": 1650 } }, "required": ["kind", "milli"] }
                ]
            }),
        }
    }

    fn observe(&self, request: SurfaceSnapshotRequest) -> Result<RenderSnapshot, Self::Error> {
        self.snapshot_for(request)
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
            .map(|entry| {
                json!({
                    "logical_time_ticks": entry.logical_time().ticks(),
                    "fact": encode_fact(entry.payload()),
                })
            })
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
        let fact_count = request.facts.len();
        let facts = Self::decode_facts(request.facts)?;
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
        let fact_count = request.facts.len();
        let facts = Self::decode_facts(request.facts)?;
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

struct VoxelBranchSource<'a> {
    worldline: &'a VoxelWorldline,
    camera: Camera,
}

impl RenderSource for VoxelBranchSource<'_> {
    type Error = Infallible;

    fn render(
        &self,
        logical_time: LogicalTime,
        tau: Tau,
    ) -> Result<Frame<engine_api::RenderBatch>, Self::Error> {
        let sampled = state(self.worldline, logical_time);
        Ok(frame_with_camera(&sampled, self.camera, tau))
    }
}

fn decode_fact(value: Value) -> Result<VoxelFact, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "fact must be a JSON object".to_owned())?;
    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| "fact.kind must be a string".to_owned())?;
    match kind {
        "Place" => Ok(VoxelFact::Place {
            position: decode_position(object.get("position"))?,
            block: decode_block(object.get("block"))?,
        }),
        "Remove" => Ok(VoxelFact::Remove {
            position: decode_position(object.get("position"))?,
        }),
        "SpawnFire" => Ok(VoxelFact::SpawnFire {
            position: decode_position(object.get("position"))?,
        }),
        "SelectTool" => Ok(VoxelFact::SelectTool {
            tool: match object.get("tool").and_then(Value::as_str) {
                Some("Remove") => VoxelTool::Remove,
                Some("Fire") => VoxelTool::Fire,
                _ => return Err("fact.tool must be Remove or Fire".to_owned()),
            },
        }),
        "SetScale" => {
            let milli = object
                .get("milli")
                .and_then(Value::as_u64)
                .ok_or_else(|| "fact.milli must be an integer".to_owned())?;
            let milli = u16::try_from(milli).map_err(|_| "fact.milli is too large".to_owned())?;
            let scale = VoxelScale::from_milli(milli)
                .ok_or_else(|| "fact.milli must be between 350 and 1650".to_owned())?;
            Ok(VoxelFact::SetScale { scale })
        }
        _ => Err(format!("unsupported voxel fact kind: {kind}")),
    }
}

fn decode_position(value: Option<&Value>) -> Result<VoxelPosition, String> {
    let object = value
        .and_then(Value::as_object)
        .ok_or_else(|| "fact.position must be an object".to_owned())?;
    let coordinate = |name: &str| {
        object
            .get(name)
            .and_then(Value::as_i64)
            .and_then(|value| i32::try_from(value).ok())
            .ok_or_else(|| format!("fact.position.{name} must be an i32"))
    };
    Ok(VoxelPosition::new(
        coordinate("x")?,
        coordinate("y")?,
        coordinate("z")?,
    ))
}

fn decode_block(value: Option<&Value>) -> Result<BlockKind, String> {
    match value.and_then(Value::as_str) {
        Some("FoundationStone") => Ok(BlockKind::FoundationStone),
        Some("Floorboard") => Ok(BlockKind::Floorboard),
        Some("TimberFrame") => Ok(BlockKind::TimberFrame),
        Some("Plaster") => Ok(BlockKind::Plaster),
        Some("Brick") => Ok(BlockKind::Brick),
        Some("RoofTile") => Ok(BlockKind::RoofTile),
        Some("Thatch") => Ok(BlockKind::Thatch),
        Some("Glass") => Ok(BlockKind::Glass),
        Some("Door") => Ok(BlockKind::Door),
        Some("WindowFrame") => Ok(BlockKind::WindowFrame),
        Some("ChimneyCap") => Ok(BlockKind::ChimneyCap),
        Some("Moss") => Ok(BlockKind::Moss),
        Some("Flower") => Ok(BlockKind::Flower),
        Some("Lantern") => Ok(BlockKind::Lantern),
        Some("PathStone") => Ok(BlockKind::PathStone),
        _ => Err("fact.block must name a supported BlockKind".to_owned()),
    }
}

fn encode_fact(fact: &VoxelFact) -> Value {
    match fact {
        VoxelFact::Place { position, block } => json!({
            "kind": "Place",
            "position": encode_position(*position),
            "block": format!("{block:?}"),
        }),
        VoxelFact::Remove { position } => json!({
            "kind": "Remove",
            "position": encode_position(*position),
        }),
        VoxelFact::SpawnFire { position } => json!({
            "kind": "SpawnFire",
            "position": encode_position(*position),
        }),
        VoxelFact::SelectTool { tool } => json!({
            "kind": "SelectTool",
            "tool": format!("{tool:?}"),
        }),
        VoxelFact::SetScale { scale } => json!({
            "kind": "SetScale",
            "milli": scale.milli(),
        }),
    }
}

fn encode_position(position: VoxelPosition) -> Value {
    json!({
        "x": position.x(),
        "y": position.y(),
        "z": position.z(),
    })
}

#[cfg(test)]
mod render_tests {
    use super::VoxelGameSurface;
    use engine_api::{
        AppendRequest, BranchSession, GameSurface, LogicalTime, SurfaceSnapshotRequest,
    };
    use engine_observation::ObservationRequest;
    use serde_json::json;

    #[test]
    fn voxel_surface_preview_does_not_mutate_actual_history() {
        let surface = VoxelGameSurface::default();
        let manifest = surface.manifest();
        assert!(manifest
            .capabilities
            .iter()
            .any(|capability| capability == "preview-append"));
        let preview = surface
            .preview_append(AppendRequest {
                branch_id:
                    BranchSession::<crate::world::VoxelContext, crate::world::VoxelFact>::actual_id(
                    ),
                expected_revision: 0,
                logical_time: LogicalTime::zero(),
                facts: vec![json!({
                    "kind": "SelectTool",
                    "tool": "Fire"
                })],
            })
            .expect("preview should validate");
        assert_eq!(preview.current_revision, 0);
        let snapshot = surface
            .observe(SurfaceSnapshotRequest {
                branch_id: 0,
                logical_time: LogicalTime::zero(),
                tau: engine_api::Tau::zero(),
                observation: ObservationRequest::new(16, 16),
            })
            .expect("actual should remain observable");
        assert!(snapshot.vertex_count() > 0);
    }
}

#[cfg(test)]
mod surface_tests {
    use super::VoxelObservationSource;
    use engine_api::{LogicalTime, Tau};
    use engine_observation::{snapshot, ObservationRequest};

    #[test]
    fn voxel_source_uses_the_generic_observation_boundary() {
        let result = snapshot(
            &VoxelObservationSource::default(),
            LogicalTime::zero(),
            Tau::zero(),
            ObservationRequest::new(32, 24),
        )
        .expect("the voxel source should render a snapshot");

        assert!(result.vertex_count() > 0);
        assert!(result.triangle_count() > 0);
        assert_eq!(&result.png()[..8], b"\x89PNG\r\n\x1a\n");
    }
}
