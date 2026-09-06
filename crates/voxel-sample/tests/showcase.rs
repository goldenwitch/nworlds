use engine_api::{Branch, Context, LogicalTime, Tau};
use engine_controls::{PlaybackMode, StepDirection, TimelineAxis};
use nworlds_host::{GamePackage, InputObservation, ObservationId, OrderedInputBatch};
use voxel_sample::engine_integration::{
    cottage_worldline, publish, redraw, state, VoxelJournalWriter, VoxelWorldline,
};
use voxel_sample::world::{BlockKind, VoxelContext, VoxelFact, VoxelPosition, FIRE_TICK_TICKS};
use voxel_sample::{VoxelInputPacket, VoxelPackage, VoxelTool};

fn batch(packet: VoxelInputPacket) -> OrderedInputBatch<VoxelInputPacket> {
    OrderedInputBatch::from_observations([InputObservation::new(ObservationId::new(0, 0), packet)])
        .expect("showcase packet should form a batch")
}

fn screen_center(rect: engine_controls::ControlRect) -> (u32, u32) {
    let x = (rect.min_x() + rect.max_x()) * 0.5;
    let y = (rect.min_y() + rect.max_y()) * 0.5;
    (
        ((x + 1.0) * 0.5 * 960.0).round() as u32,
        ((1.0 - y) * 0.5 * 720.0).round() as u32,
    )
}

fn fire_worldline() -> VoxelWorldline {
    let mut writer = VoxelJournalWriter::new();
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
    Branch::new(Context::new(VoxelContext), writer.snapshot())
}

fn has_voxel(
    state: &voxel_sample::engine_integration::VoxelGameState,
    position: VoxelPosition,
) -> bool {
    state
        .payload()
        .voxels()
        .iter()
        .any(|voxel| voxel.position() == position)
}

fn has_fire(
    state: &voxel_sample::engine_integration::VoxelGameState,
    position: VoxelPosition,
) -> bool {
    state
        .payload()
        .fires()
        .iter()
        .any(|fire| fire.position() == position)
}

#[test]
fn public_fire_queries_are_repeatable_and_matrix_timed() {
    let worldline = fire_worldline();
    let diagonal_time = LogicalTime::from_ticks(FIRE_TICK_TICKS * 2);
    let orthogonal_time = LogicalTime::from_ticks(FIRE_TICK_TICKS);

    let later = state(&worldline, diagonal_time);
    let _earlier = state(&worldline, LogicalTime::zero());
    let later_again = state(&worldline, diagonal_time);

    assert_eq!(later, later_again);
    assert!(!has_voxel(&later, VoxelPosition::new(1, 0, 0)));
    assert!(!has_voxel(&later, VoxelPosition::new(1, 0, 1)));
    assert!(has_fire(
        &state(&worldline, orthogonal_time),
        VoxelPosition::new(1, 0, 0)
    ));
}

#[test]
fn public_tool_selection_is_recovered_from_logical_state() {
    let mut writer = VoxelJournalWriter::new();
    writer.record(VoxelFact::SelectTool {
        tool: VoxelTool::Remove,
    });
    writer
        .advance_to(LogicalTime::from_ticks(10))
        .expect("tool selection time should advance");
    writer.record(VoxelFact::SelectTool {
        tool: VoxelTool::Fire,
    });
    let worldline = Branch::new(Context::new(VoxelContext), writer.snapshot());

    assert_eq!(
        state(&worldline, LogicalTime::zero()).payload().tool(),
        VoxelTool::Remove
    );
    assert_eq!(
        state(&worldline, LogicalTime::from_ticks(9))
            .payload()
            .tool(),
        VoxelTool::Remove
    );
    assert_eq!(
        state(&worldline, LogicalTime::from_ticks(10))
            .payload()
            .tool(),
        VoxelTool::Fire
    );
}

#[test]
fn public_redraw_is_deterministic_and_tau_drives_only_fire_presentation() {
    let worldline = fire_worldline();
    let camera = voxel_sample::camera::Camera::default();
    let first = redraw(&worldline, LogicalTime::zero(), camera, Tau::zero());
    let repeated = redraw(&worldline, LogicalTime::zero(), camera, Tau::zero());
    let animated = redraw(
        &worldline,
        LogicalTime::zero(),
        camera,
        Tau::from_ticks(125),
    );

    assert_eq!(first, repeated);
    assert_ne!(first.payload(), animated.payload());
    assert_eq!(animated.tau(), Tau::from_ticks(125));
    assert_eq!(
        state(&worldline, LogicalTime::zero()),
        state(&worldline, LogicalTime::zero())
    );
}

#[test]
fn public_publication_keeps_parent_history_unchanged() {
    let (parent, mut writer) = cottage_worldline();
    let position = VoxelPosition::new(0, 1, -3);
    let child = publish(&parent, &mut writer, VoxelFact::Remove { position });

    assert!(has_voxel(&state(&parent, LogicalTime::zero()), position));
    assert!(!has_voxel(&state(&child, LogicalTime::zero()), position));
    assert_eq!(parent.journal().len() + 1, child.journal().len());
}

#[test]
fn package_selection_and_control_path_are_publicly_selectable() {
    let mut package = VoxelPackage::new();
    package
        .ingest_batch(batch(VoxelInputPacket::SelectTool {
            tool: VoxelTool::Fire,
        }))
        .expect("tool selection should ingest");
    assert!(package.update().expect("tool selection should update"));
    assert_eq!(package.selected_tool(), VoxelTool::Fire);

    package
        .ingest_batch(batch(VoxelInputPacket::PrimaryClick { x: 480, y: 360 }))
        .expect("fire click should ingest");
    assert!(package.step().expect("fire click should step").0);
    assert!(!package
        .present()
        .expect("selected fire state should present")
        .payload()
        .is_empty());
}

#[test]
fn package_timeline_advances_automatically_by_default() {
    let mut package = VoxelPackage::new();

    assert_eq!(package.logical_time(), LogicalTime::zero());
    assert_eq!(package.visual_time(), Tau::zero());
    assert!(package.update().expect("automatic update should succeed"));
    assert_eq!(package.logical_time(), LogicalTime::from_ticks(16));
    assert_eq!(package.visual_time(), Tau::from_ticks(16));
    assert_eq!(package.controls().mode(), PlaybackMode::Automatic);
}

#[test]
fn ingesting_input_does_not_advance_automatic_time_without_update() {
    let mut package = VoxelPackage::new();

    package
        .ingest_batch(batch(VoxelInputPacket::CameraReset))
        .expect("input should ingest");

    assert_eq!(package.logical_time(), LogicalTime::zero());
    assert_eq!(package.visual_time(), Tau::zero());
}

#[test]
fn slider_and_step_controls_pause_and_move_both_time_axes() {
    let mut package = VoxelPackage::new();
    let layout = package.controls().layout();
    let logical_slider = screen_center(layout.logical_slider());
    let logical_forward =
        screen_center(layout.step_rect(TimelineAxis::LogicalTime, StepDirection::Forward));
    let logical_backward =
        screen_center(layout.step_rect(TimelineAxis::LogicalTime, StepDirection::Backward));
    let tau_forward = screen_center(layout.step_rect(TimelineAxis::Tau, StepDirection::Forward));
    let tau_backward = screen_center(layout.step_rect(TimelineAxis::Tau, StepDirection::Backward));

    package
        .ingest_batch(batch(VoxelInputPacket::PointerDown {
            x: logical_slider.0,
            y: logical_slider.1,
        }))
        .expect("logical slider input should ingest");
    assert!(package.update().expect("slider update should succeed"));
    let manually_selected_logical = package.logical_time();
    let manually_selected_tau = package.visual_time();
    assert_eq!(package.controls().mode(), PlaybackMode::Manual);

    assert!(!package.update().expect("manual update should succeed"));
    assert_eq!(package.logical_time(), manually_selected_logical);
    assert_eq!(package.visual_time(), manually_selected_tau);

    package
        .ingest_batch(batch(VoxelInputPacket::PointerDown {
            x: logical_forward.0,
            y: logical_forward.1,
        }))
        .expect("logical forward step should ingest");
    assert!(package.update().expect("logical step should succeed"));
    assert!(package.logical_time() > manually_selected_logical);

    let after_logical_forward = package.logical_time();
    package
        .ingest_batch(batch(VoxelInputPacket::PointerDown {
            x: logical_backward.0,
            y: logical_backward.1,
        }))
        .expect("logical backward step should ingest");
    assert!(package
        .update()
        .expect("logical backward step should succeed"));
    assert!(package.logical_time() < after_logical_forward);

    package
        .ingest_batch(batch(VoxelInputPacket::PointerDown {
            x: tau_forward.0,
            y: tau_forward.1,
        }))
        .expect("Tau forward step should ingest");
    assert!(package.update().expect("Tau forward step should succeed"));
    let after_tau_forward = package.visual_time();
    assert!(after_tau_forward > manually_selected_tau);

    package
        .ingest_batch(batch(VoxelInputPacket::PointerDown {
            x: tau_backward.0,
            y: tau_backward.1,
        }))
        .expect("Tau backward step should ingest");
    assert!(package.update().expect("Tau backward step should succeed"));
    assert!(package.visual_time() < after_tau_forward);
}

#[test]
fn world_click_resumes_automatic_progression_and_controls_are_rendered() {
    let mut package = VoxelPackage::new();
    package
        .ingest_batch(batch(VoxelInputPacket::PointerDown { x: 480, y: 675 }))
        .expect("slider input should ingest");
    package.update().expect("slider update should succeed");
    assert_eq!(package.controls().mode(), PlaybackMode::Manual);

    let frame = package.present().expect("controls should present");
    let (worldline, _) = cottage_worldline();
    let base = voxel_sample::engine_integration::render_batch_at(
        &state(&worldline, LogicalTime::zero()),
        voxel_sample::camera::Camera::default(),
        Tau::zero(),
    );
    assert!(frame.payload().len() > base.len());

    package
        .ingest_batch(batch(VoxelInputPacket::PrimaryClick { x: 480, y: 360 }))
        .expect("world click should ingest");
    package
        .update()
        .expect("world click should resume automatic mode");
    assert_eq!(package.controls().mode(), PlaybackMode::Automatic);
    let resumed_logical = package.logical_time();
    package.update().expect("automatic update should resume");
    assert!(package.logical_time() > resumed_logical);
}
