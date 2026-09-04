//! The recommended engine integration shape for this sample.
//!
//! The game owns `VoxelFact`, `VoxelState`, and `VoxelContext` in `world.rs`.
//! This file is the small seam that specializes the generic engine: it owns
//! the query, immutable worldline construction, journal authoring, and
//! publication of new worldline values.

use std::collections::BTreeMap;

use engine_api::{
    present, state as query_state, Branch, Context, Frame, GameState, IndexedQuery, JournalWriter,
    LogicalTime, QueryInput, RenderVertex, Renderer, Tau, Worldline,
};

pub use engine_api::RenderBatch;

use crate::camera::Camera;
use crate::world::{cottage_blocks, VoxelContext, VoxelFact, VoxelState};

/// The engine's generic worldline specialized with this game's context/facts.
pub type VoxelWorldline = Worldline<VoxelContext, VoxelFact>;

/// The engine's generic journal writer specialized with this game's facts.
pub type VoxelJournalWriter = JournalWriter<VoxelFact>;

/// The engine's generic state envelope specialized with this game's state.
pub type VoxelGameState = GameState<VoxelState>;

/// The engine frame envelope specialized with the voxel batch.
pub type VoxelFrame = Frame<RenderBatch>;

#[derive(Clone, Copy, Debug, Default)]
pub struct VoxelRenderer;

impl Renderer<VoxelState> for VoxelRenderer {
    type Output = RenderBatch;

    fn render(state: &VoxelGameState, _tau: Tau) -> Self::Output {
        let camera = Camera::default();
        let scale = state.payload().scale().as_f32();
        let mut vertices = Vec::with_capacity(state.payload().voxels().len() * 36);

        for voxel in state.payload().voxels() {
            let position = voxel.position();
            let min = [
                position.x() as f32 * scale,
                position.y() as f32 * scale,
                position.z() as f32 * scale,
            ];
            let max = [min[0] + scale, min[1] + scale, min[2] + scale];
            let [min_x, min_y, min_z] = min;
            let [max_x, max_y, max_z] = max;
            let color = voxel.block().color();

            cube_face(
                &mut vertices,
                &camera,
                [
                    [min_x, min_y, min_z],
                    [max_x, min_y, min_z],
                    [max_x, min_y, max_z],
                    [min_x, min_y, max_z],
                ],
                color,
                0.62,
            );
            cube_face(
                &mut vertices,
                &camera,
                [
                    [min_x, max_y, min_z],
                    [min_x, max_y, max_z],
                    [max_x, max_y, max_z],
                    [max_x, max_y, min_z],
                ],
                color,
                1.12,
            );
            cube_face(
                &mut vertices,
                &camera,
                [
                    [min_x, min_y, min_z],
                    [min_x, max_y, min_z],
                    [max_x, max_y, min_z],
                    [max_x, min_y, min_z],
                ],
                color,
                0.84,
            );
            cube_face(
                &mut vertices,
                &camera,
                [
                    [max_x, min_y, max_z],
                    [max_x, max_y, max_z],
                    [min_x, max_y, max_z],
                    [min_x, min_y, max_z],
                ],
                color,
                0.93,
            );
            cube_face(
                &mut vertices,
                &camera,
                [
                    [min_x, min_y, max_z],
                    [min_x, max_y, max_z],
                    [min_x, max_y, min_z],
                    [min_x, min_y, min_z],
                ],
                color,
                0.74,
            );
            cube_face(
                &mut vertices,
                &camera,
                [
                    [max_x, min_y, min_z],
                    [max_x, max_y, min_z],
                    [max_x, max_y, max_z],
                    [max_x, min_y, max_z],
                ],
                color,
                1.0,
            );
        }

        RenderBatch::new(vertices)
    }
}

fn cube_face(
    vertices: &mut Vec<RenderVertex>,
    camera: &Camera,
    corners: [[f32; 3]; 4],
    color: [f32; 3],
    shade: f32,
) {
    let color = [
        (color[0] * shade).min(1.0),
        (color[1] * shade).min(1.0),
        (color[2] * shade).min(1.0),
        1.0,
    ];
    let projected = corners.map(|corner| camera.project_point(corner));
    vertices.extend([
        RenderVertex::new(projected[0], color),
        RenderVertex::new(projected[1], color),
        RenderVertex::new(projected[2], color),
        RenderVertex::new(projected[0], color),
        RenderVertex::new(projected[2], color),
        RenderVertex::new(projected[3], color),
    ]);
}

/// Interprets immutable voxel facts as the current voxel state.
pub struct VoxelQuery;

impl IndexedQuery<VoxelContext, VoxelFact> for VoxelQuery {
    type Result = VoxelState;

    fn query(&self, input: QueryInput<'_, VoxelContext, VoxelFact>) -> Self::Result {
        let mut voxels = BTreeMap::new();
        let mut scale = crate::world::VoxelScale::default();

        for entry in input.visible_entries() {
            match *entry.payload() {
                VoxelFact::Place { position, block } => {
                    voxels.insert(position, block);
                }
                VoxelFact::Remove { position } => {
                    voxels.remove(&position);
                }
                VoxelFact::SetScale { scale: next_scale } => {
                    scale = next_scale;
                }
            }
        }

        VoxelState::from_parts(
            voxels
                .into_iter()
                .map(|(position, block)| crate::world::Voxel::new(position, block))
                .collect(),
            scale,
        )
    }
}

/// Performs the engine's direct immutable query at an arbitrary logical time.
pub fn state(worldline: &VoxelWorldline, logical_time: LogicalTime) -> VoxelGameState {
    query_state(
        worldline.context(),
        worldline.journal(),
        logical_time,
        VoxelQuery,
    )
}

/// Authors the cottage through the engine's journal-owned timestamp path.
pub fn cottage_worldline() -> (VoxelWorldline, VoxelJournalWriter) {
    let mut writer = VoxelJournalWriter::new();
    writer.record(VoxelFact::SetScale {
        scale: crate::world::VoxelScale::default(),
    });

    for (position, block) in cottage_blocks() {
        writer.record(VoxelFact::Place { position, block });
    }

    let worldline = Branch::new(Context::new(VoxelContext), writer.snapshot());
    (worldline, writer)
}

/// Publishes one fact as a new immutable worldline value.
pub fn publish(
    worldline: &VoxelWorldline,
    writer: &mut VoxelJournalWriter,
    fact: VoxelFact,
) -> VoxelWorldline {
    writer.record(fact);
    Branch::new(worldline.context().clone(), writer.snapshot())
}

/// Queries the sample's initial world at the engine's zero logical time.
pub fn state_at_zero(worldline: &VoxelWorldline) -> VoxelGameState {
    state(worldline, LogicalTime::zero())
}

/// Uses the engine's `GameState + Tau -> Frame` presentation boundary.
pub fn frame(state: &VoxelGameState) -> VoxelFrame {
    present::<VoxelState, VoxelRenderer>(state, Tau::zero())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use engine_api::LogicalTime;

    use super::{cottage_worldline, publish, state};
    use crate::world::{BlockKind, VoxelFact, VoxelPosition};

    #[test]
    fn cottage_uses_many_distinct_block_kinds() {
        let (worldline, _) = cottage_worldline();
        let sampled = state(&worldline, LogicalTime::zero());
        let kinds = sampled
            .payload()
            .voxels()
            .iter()
            .map(|voxel| voxel.block())
            .collect::<BTreeSet<BlockKind>>();

        assert!(sampled.payload().voxels().len() > 200);
        assert!(kinds.len() >= 12);
    }

    #[test]
    fn removing_a_voxel_publishes_a_new_worldline_without_mutating_the_parent() {
        let (parent, mut writer) = cottage_worldline();
        let position = VoxelPosition::new(0, 1, -3);
        let child = publish(&parent, &mut writer, VoxelFact::Remove { position });

        assert!(state(&parent, LogicalTime::zero())
            .payload()
            .voxel_at(position)
            .is_some());
        assert!(state(&child, LogicalTime::zero())
            .payload()
            .voxel_at(position)
            .is_none());
    }

    #[test]
    fn scale_is_a_continuous_fixed_point_state_parameter() {
        let (parent, mut writer) = cottage_worldline();
        let scale = state(&parent, LogicalTime::zero()).payload().scale();
        let child = publish(
            &parent,
            &mut writer,
            VoxelFact::SetScale {
                scale: scale.saturating_add_milli(1),
            },
        );

        assert_eq!(
            state(&child, LogicalTime::zero()).payload().scale().milli(),
            scale.milli() + 1
        );
    }

    #[test]
    fn presentation_preserves_the_selected_logical_time() {
        let (worldline, _) = cottage_worldline();
        let sampled = state(&worldline, LogicalTime::from_ticks(17));
        let presented = super::frame(&sampled);

        assert_eq!(sampled.logical_time(), LogicalTime::from_ticks(17));
        assert!(!presented.payload().is_empty());
    }
}
