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
use engine_controls::TimelineControls;

pub use engine_api::RenderBatch;

use crate::camera::Camera;
use crate::world::{
    cottage_blocks, Fire, VoxelContext, VoxelFact, VoxelPosition, VoxelState, VoxelTool,
    FIRE_LIFETIME_TICKS, FIRE_SPREAD_MATRIX, FIRE_TICK_TICKS,
};

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

    fn render(state: &VoxelGameState, tau: Tau) -> Self::Output {
        render_batch_at(state, Camera::default(), tau)
    }
}

/// Projects one complete voxel state through an explicit presentation camera.
pub fn render_batch(state: &VoxelGameState, camera: Camera) -> RenderBatch {
    render_batch_at(state, camera, Tau::zero())
}

pub fn render_batch_at(state: &VoxelGameState, camera: Camera, tau: Tau) -> RenderBatch {
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

    for fire in state.payload().fires() {
        fire_geometry(&mut vertices, &camera, *fire, scale, tau);
    }

    crate::tool::append_palette(&mut vertices, state.payload().tool());

    RenderBatch::new(vertices)
}

fn fire_geometry(
    vertices: &mut Vec<RenderVertex>,
    camera: &Camera,
    fire: Fire,
    scale: f32,
    tau: Tau,
) {
    let phase = (i64::from(fire.position().x()) * 3
        + i64::from(fire.position().y()) * 5
        + i64::from(fire.position().z()) * 7)
        .rem_euclid(4);
    let frame = (tau.ticks().div_euclid(125) + phase).rem_euclid(4) as usize;
    let heights = [0.42, 0.58, 0.48, 0.66];
    let colors = [
        [0.95, 0.22, 0.04],
        [1.0, 0.48, 0.05],
        [0.98, 0.72, 0.10],
        [0.90, 0.16, 0.03],
    ];
    let position = fire.position();
    let min = [
        position.x() as f32 * scale + scale * 0.25,
        (position.y() as f32 + 1.0) * scale,
        position.z() as f32 * scale + scale * 0.25,
    ];
    let max = [
        min[0] + scale * 0.5,
        min[1] + scale * heights[frame],
        min[2] + scale * 0.5,
    ];
    let [min_x, min_y, min_z] = min;
    let [max_x, max_y, max_z] = max;
    let color = colors[frame];

    cube_face(
        vertices,
        camera,
        [
            [min_x, min_y, min_z],
            [max_x, min_y, min_z],
            [max_x, min_y, max_z],
            [min_x, min_y, max_z],
        ],
        color,
        0.72,
    );
    cube_face(
        vertices,
        camera,
        [
            [min_x, max_y, min_z],
            [min_x, max_y, max_z],
            [max_x, max_y, max_z],
            [max_x, max_y, min_z],
        ],
        color,
        1.0,
    );
    cube_face(
        vertices,
        camera,
        [
            [min_x, min_y, min_z],
            [min_x, max_y, min_z],
            [max_x, max_y, min_z],
            [max_x, min_y, min_z],
        ],
        color,
        0.86,
    );
    cube_face(
        vertices,
        camera,
        [
            [max_x, min_y, max_z],
            [max_x, max_y, max_z],
            [min_x, max_y, max_z],
            [min_x, min_y, max_z],
        ],
        color,
        0.94,
    );
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
        let mut fire_starts = BTreeMap::new();
        let mut scale = crate::world::VoxelScale::default();
        let mut tool = VoxelTool::default();

        for entry in input.visible_entries() {
            match *entry.payload() {
                VoxelFact::Place { position, block } => {
                    voxels.insert(position, block);
                }
                VoxelFact::Remove { position } => {
                    voxels.remove(&position);
                }
                VoxelFact::SpawnFire { position } => {
                    fire_starts
                        .entry(position)
                        .or_insert(entry.logical_time().ticks());
                }
                VoxelFact::SelectTool { tool: next_tool } => {
                    tool = next_tool;
                }
                VoxelFact::SetScale { scale: next_scale } => {
                    scale = next_scale;
                }
            }
        }

        let target_ticks = input.logical_time().ticks();
        let mut fire_events = fire_starts
            .iter()
            .map(|(&position, &start_ticks)| ((start_ticks, position), ()))
            .collect::<BTreeMap<_, _>>();
        let mut started = BTreeMap::<VoxelPosition, i64>::new();
        let mut burns = BTreeMap::<VoxelPosition, i64>::new();

        while let Some((&(start_ticks, position), ())) = fire_events.iter().next() {
            fire_events.remove(&(start_ticks, position));
            if start_ticks > target_ticks {
                break;
            }
            if started.contains_key(&position) {
                continue;
            }
            started.insert(position, start_ticks);
            let expiry = start_ticks.saturating_add(FIRE_LIFETIME_TICKS * FIRE_TICK_TICKS);
            burns
                .entry(position)
                .and_modify(|existing| *existing = (*existing).min(expiry))
                .or_insert(expiry);

            for (row, matrix_row) in FIRE_SPREAD_MATRIX.iter().enumerate() {
                for (column, delay) in matrix_row.iter().enumerate() {
                    if *delay == 0 {
                        continue;
                    }
                    let neighbor = VoxelPosition::new(
                        position.x() + column as i32 - 1,
                        position.y(),
                        position.z() + row as i32 - 1,
                    );
                    if !voxels.contains_key(&neighbor) {
                        continue;
                    }
                    let event_ticks =
                        start_ticks.saturating_add(i64::from(*delay) * FIRE_TICK_TICKS);
                    burns
                        .entry(neighbor)
                        .and_modify(|existing| *existing = (*existing).min(event_ticks))
                        .or_insert(event_ticks);
                    fire_events.entry((event_ticks, neighbor)).or_insert(());
                }
            }
        }

        voxels.retain(|position, _| {
            burns
                .get(position)
                .is_none_or(|burn_time| *burn_time > target_ticks)
        });

        let fires = started
            .into_iter()
            .filter_map(|(position, start_ticks)| {
                let age = target_ticks
                    .saturating_sub(start_ticks)
                    .div_euclid(FIRE_TICK_TICKS);
                (age < FIRE_LIFETIME_TICKS)
                    .then(|| Fire::new(position, age.clamp(0, u8::MAX as i64) as u8))
            })
            .collect();

        VoxelState::from_parts(
            voxels
                .into_iter()
                .map(|(position, block)| crate::world::Voxel::new(position, block))
                .collect(),
            fires,
            scale,
            tool,
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
    writer.record(VoxelFact::SelectTool {
        tool: VoxelTool::default(),
    });
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

/// Presents one complete voxel state through an explicit camera and Tau.
pub fn frame_with_camera(state: &VoxelGameState, camera: Camera, tau: Tau) -> VoxelFrame {
    Frame::new(tau, render_batch_at(state, camera, tau))
}

/// Presents one complete voxel state with the explicit camera and timeline controls.
pub fn frame_with_camera_and_controls(
    state: &VoxelGameState,
    camera: Camera,
    tau: Tau,
    controls: &TimelineControls,
) -> VoxelFrame {
    let mut vertices = render_batch_at(state, camera, tau).vertices().to_vec();
    vertices.extend_from_slice(controls.render().vertices());
    Frame::new(tau, RenderBatch::new(vertices))
}

/// Samples one immutable worldline at one logical and presentation time pair.
pub fn redraw(
    worldline: &VoxelWorldline,
    logical_time: LogicalTime,
    camera: Camera,
    tau: Tau,
) -> VoxelFrame {
    let sampled = state(worldline, logical_time);
    Frame::new(tau, render_batch_at(&sampled, camera, tau))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use engine_api::{Context, LogicalTime, Tau};

    use super::{cottage_worldline, publish, redraw, state};
    use crate::camera::Camera;
    use crate::world::{BlockKind, VoxelContext, VoxelFact, VoxelPosition, FIRE_TICK_TICKS};

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

    #[test]
    fn redraw_queries_logical_time_and_keeps_tau_independent() {
        let mut writer = super::VoxelJournalWriter::new();
        writer
            .advance_to(LogicalTime::from_ticks(1))
            .expect("future authoring time should be valid");
        writer.record(VoxelFact::Place {
            position: VoxelPosition::new(0, 0, 0),
            block: BlockKind::FoundationStone,
        });
        let worldline = super::Branch::new(Context::new(VoxelContext), writer.snapshot());
        let camera = Camera::default();

        let before = redraw(&worldline, LogicalTime::zero(), camera, Tau::zero());
        let first = redraw(&worldline, LogicalTime::from_ticks(1), camera, Tau::zero());
        let second = redraw(
            &worldline,
            LogicalTime::from_ticks(1),
            camera,
            Tau::from_ticks(7),
        );

        assert!(!before.payload().is_empty());
        assert!(first.payload().len() > before.payload().len());
        assert_eq!(first.payload(), second.payload());
        assert_eq!(second.tau(), Tau::from_ticks(7));
    }

    #[test]
    fn fire_matrix_is_query_derived_at_explicit_logical_times() {
        let mut writer = super::VoxelJournalWriter::new();
        for position in [
            VoxelPosition::new(0, 0, 0),
            VoxelPosition::new(1, 0, 0),
            VoxelPosition::new(1, 0, 1),
        ] {
            writer.record(VoxelFact::Place {
                position,
                block: BlockKind::TimberFrame,
            });
        }
        writer.record(VoxelFact::SpawnFire {
            position: VoxelPosition::new(0, 0, 0),
        });
        let worldline = super::Branch::new(Context::new(VoxelContext), writer.snapshot());

        let at_start = state(&worldline, LogicalTime::zero());
        let after_orthogonal = state(&worldline, LogicalTime::from_ticks(FIRE_TICK_TICKS));
        let after_diagonal = state(&worldline, LogicalTime::from_ticks(FIRE_TICK_TICKS * 2));
        let expired = state(&worldline, LogicalTime::from_ticks(FIRE_TICK_TICKS * 5));

        assert_eq!(at_start.payload().fires().len(), 1);
        assert!(at_start
            .payload()
            .voxel_at(VoxelPosition::new(1, 0, 0))
            .is_some());
        assert!(after_orthogonal
            .payload()
            .voxel_at(VoxelPosition::new(1, 0, 0))
            .is_none());
        assert!(after_diagonal
            .payload()
            .voxel_at(VoxelPosition::new(1, 0, 1))
            .is_none());
        assert!(expired.payload().fires().is_empty());
        assert!(expired
            .payload()
            .voxel_at(VoxelPosition::new(0, 0, 0))
            .is_none());
    }

    #[test]
    fn fire_animation_uses_tau_without_changing_queried_state() {
        let mut writer = super::VoxelJournalWriter::new();
        writer.record(VoxelFact::Place {
            position: VoxelPosition::new(0, 0, 0),
            block: BlockKind::TimberFrame,
        });
        writer.record(VoxelFact::SpawnFire {
            position: VoxelPosition::new(0, 0, 0),
        });
        let worldline = super::Branch::new(Context::new(VoxelContext), writer.snapshot());
        let first = state(&worldline, LogicalTime::zero());
        let repeated = state(&worldline, LogicalTime::zero());
        let frame_zero = redraw(
            &worldline,
            LogicalTime::zero(),
            Camera::default(),
            Tau::zero(),
        );
        let frame_later = redraw(
            &worldline,
            LogicalTime::zero(),
            Camera::default(),
            Tau::from_ticks(125),
        );

        assert_eq!(first, repeated);
        assert_eq!(
            frame_zero,
            redraw(
                &worldline,
                LogicalTime::zero(),
                Camera::default(),
                Tau::zero(),
            )
        );
        assert_ne!(frame_zero.payload(), frame_later.payload());
        assert_eq!(frame_later.tau(), Tau::from_ticks(125));
    }
}
