use engine_api::{JournalWriterError, LogicalTime, PresentationError, Tau};
use engine_controls::{
    LogicalTimeDelta, NormalizedPoint, ParabolicProjection, PointerTarget, ScreenPoint, TauDelta,
    TimelineConfig, TimelineControls, TimelineError, Viewport,
};
use nworlds_host::{
    GamePackage, HostVersionRequirement, InputBatchError, OrderedInputBatch, PackageDeclaration,
    PersistenceRequirement, RenderVocabularyRequirement, SchemaVersion, SemanticVersion,
};

use crate::camera::Camera;
use crate::engine_integration::{
    cottage_worldline, frame_with_camera_and_controls, publish, state, state_at_zero, VoxelFrame,
    VoxelGameState, VoxelJournalWriter, VoxelWorldline,
};
use crate::world::{VoxelFact, VoxelTool};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VoxelInputPacket {
    PointerMoved { x: u32, y: u32 },
    PointerDown { x: u32, y: u32 },
    PointerUp { x: u32, y: u32 },
    PrimaryClick { x: u32, y: u32 },
    SelectTool { tool: VoxelTool },
    Wheel { milli_delta: i32 },
    ViewportResized { width: u32, height: u32 },
    CameraOrbit { horizontal: i32, vertical: i32 },
    CameraZoom { distance_milli: i32 },
    CameraReset,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VoxelPackageError {
    Input(InputBatchError),
    Journal(JournalWriterError),
    Timeline(TimelineError),
    PersistenceUnavailable,
}

impl From<InputBatchError> for VoxelPackageError {
    fn from(error: InputBatchError) -> Self {
        Self::Input(error)
    }
}

impl From<JournalWriterError> for VoxelPackageError {
    fn from(error: JournalWriterError) -> Self {
        Self::Journal(error)
    }
}

impl From<TimelineError> for VoxelPackageError {
    fn from(error: TimelineError) -> Self {
        Self::Timeline(error)
    }
}

pub struct VoxelPackage {
    worldline: VoxelWorldline,
    writer: VoxelJournalWriter,
    pending: Vec<VoxelInputPacket>,
    viewport: Viewport,
    camera: Camera,
    selected: VoxelGameState,
    controls: TimelineControls,
}

impl VoxelPackage {
    pub fn new() -> Self {
        let (worldline, writer) = cottage_worldline();
        let selected = state_at_zero(&worldline);
        let viewport = Viewport::new(960, 720);
        let controls = TimelineControls::new(
            LogicalTime::zero(),
            Tau::zero(),
            TimelineConfig::new(
                ParabolicProjection::new(
                    LogicalTime::zero(),
                    LogicalTimeDelta::from_ticks(5_000),
                    Tau::zero(),
                    TauDelta::from_ticks(5_000),
                ),
                LogicalTimeDelta::from_ticks(16),
                TauDelta::from_ticks(16),
                LogicalTimeDelta::from_ticks(250),
                TauDelta::from_ticks(250),
            ),
        )
        .with_viewport(viewport);
        let mut camera = Camera::default();
        camera.set_aspect(960.0 / 720.0);
        Self {
            worldline,
            writer,
            pending: Vec::new(),
            viewport,
            camera,
            selected,
            controls,
        }
    }

    fn apply_packet(&mut self, packet: VoxelInputPacket) -> Result<bool, VoxelPackageError> {
        match packet {
            VoxelInputPacket::PointerMoved { x, y } => {
                let changed = self
                    .controls
                    .pointer_move(self.point(x, y))
                    .map_err(VoxelPackageError::from)?;
                if changed {
                    self.refresh_selected();
                }
                Ok(changed)
            }
            VoxelInputPacket::PointerDown { x, y } => self.pointer_down(x, y),
            VoxelInputPacket::PointerUp { x, y } => {
                let changed = self
                    .controls
                    .pointer_up(self.point(x, y))
                    .map_err(VoxelPackageError::from)?;
                if changed {
                    self.refresh_selected();
                }
                Ok(changed)
            }
            VoxelInputPacket::PrimaryClick { x, y } => {
                let changed = self.pointer_down(x, y)?;
                self.controls
                    .pointer_up(self.point(x, y))
                    .map_err(VoxelPackageError::from)?;
                Ok(changed)
            }
            VoxelInputPacket::SelectTool { tool } => Ok(self.select_tool(tool)?),
            VoxelInputPacket::Wheel { milli_delta } => {
                let current = self.selected.payload().scale();
                let next = current.saturating_add_milli(milli_delta);
                if next == current {
                    Ok(false)
                } else {
                    self.publish(VoxelFact::SetScale { scale: next })?;
                    Ok(true)
                }
            }
            VoxelInputPacket::ViewportResized { width, height } => {
                self.viewport = Viewport::new(width, height);
                self.controls.set_viewport(self.viewport);
                self.camera.set_aspect(self.viewport.aspect());
                Ok(true)
            }
            VoxelInputPacket::CameraOrbit {
                horizontal,
                vertical,
            } => {
                self.camera
                    .orbit(horizontal as f32 * 0.01, -(vertical as f32) * 0.01);
                Ok(true)
            }
            VoxelInputPacket::CameraZoom { distance_milli } => {
                self.camera.zoom(distance_milli as f32 / 1_000.0);
                Ok(true)
            }
            VoxelInputPacket::CameraReset => {
                self.camera.reset();
                Ok(true)
            }
        }
    }

    fn pointer_down(&mut self, x: u32, y: u32) -> Result<bool, VoxelPackageError> {
        if let Some(tool) = crate::tool::pick(
            x,
            y,
            self.viewport.width().max_one().get(),
            self.viewport.height().max_one().get(),
        ) {
            return self.select_tool(tool);
        }

        match self
            .controls
            .pointer_down(self.point(x, y))
            .map_err(VoxelPackageError::from)?
        {
            PointerTarget::Timeline => {
                self.refresh_selected();
                Ok(true)
            }
            PointerTarget::World => self.world_click(x, y),
        }
    }

    fn world_click(&mut self, x: u32, y: u32) -> Result<bool, VoxelPackageError> {
        self.synchronize_world_interaction_time()?;
        let position = self.camera.pick(
            x as f32,
            y as f32,
            self.viewport.width().max_one().get() as f32,
            self.viewport.height().max_one().get() as f32,
            self.selected.payload(),
        );
        if let Some(position) = position {
            let fact = match self.selected.payload().tool() {
                VoxelTool::Remove => VoxelFact::Remove { position },
                VoxelTool::Fire => VoxelFact::SpawnFire { position },
            };
            self.publish(fact)?;
            Ok(true)
        } else {
            Ok(true)
        }
    }

    fn synchronize_world_interaction_time(&mut self) -> Result<(), VoxelPackageError> {
        let authoring_time = self
            .writer
            .current_time()
            .max(self.controls.logical_time());
        self.writer.advance_to(authoring_time)?;
        if self.controls.logical_time() != authoring_time {
            self.controls.set_logical_time(authoring_time);
            self.controls.reset_tau();
            self.controls.resume_from_world();
            self.refresh_selected();
        }
        Ok(())
    }

    fn select_tool(&mut self, tool: VoxelTool) -> Result<bool, VoxelPackageError> {
        if self.selected.payload().tool() == tool {
            Ok(false)
        } else {
            self.publish(VoxelFact::SelectTool { tool })?;
            Ok(true)
        }
    }

    fn publish(&mut self, fact: VoxelFact) -> Result<(), VoxelPackageError> {
        if self.writer.current_time() < self.controls.logical_time() {
            self.writer.advance_to(self.controls.logical_time())?;
        }
        self.worldline = publish(&self.worldline, &mut self.writer, fact);
        self.controls.reset_tau();
        self.refresh_selected();
        Ok(())
    }

    fn point(&self, x: u32, y: u32) -> NormalizedPoint {
        NormalizedPoint::from_screen(self.viewport, ScreenPoint::new(x, y))
    }

    fn refresh_selected(&mut self) {
        self.selected = state(&self.worldline, self.controls.logical_time());
    }

    fn refresh_selected_if_needed(&mut self) -> bool {
        if self.selected.logical_time() == self.controls.logical_time() {
            false
        } else {
            self.refresh_selected();
            true
        }
    }

    /// Advances downstream visual time without querying or mutating the worldline.
    pub fn advance_visual_time(&mut self, delta: Tau) -> Result<Tau, PresentationError> {
        self.controls
            .advance_tau(delta)
            .map_err(|_| PresentationError::VisualTimeOverflow)
    }

    /// Returns the current downstream visual time.
    pub const fn visual_time(&self) -> Tau {
        self.controls.tau()
    }

    /// Returns the logical time of the currently selected complete state.
    pub fn logical_time(&self) -> LogicalTime {
        self.controls.logical_time()
    }

    pub fn selected_tool(&self) -> VoxelTool {
        self.selected.payload().tool()
    }

    /// Presents an explicit logical and presentation-time sample without changing package cursors.
    pub fn present_at(&self, logical_time: LogicalTime, tau: Tau) -> VoxelFrame {
        let sampled = state(&self.worldline, logical_time);
        let mut controls = self.controls;
        controls.set_logical_time(logical_time);
        controls.set_tau(tau);
        frame_with_camera_and_controls(&sampled, self.camera, tau, &controls)
    }

    pub const fn camera(&self) -> Camera {
        self.camera
    }

    pub fn controls(&self) -> TimelineControls {
        self.controls
    }
}

impl Default for VoxelPackage {
    fn default() -> Self {
        Self::new()
    }
}

impl GamePackage for VoxelPackage {
    type InputBatch = OrderedInputBatch<VoxelInputPacket>;
    type Frame = VoxelFrame;
    type Error = VoxelPackageError;
    type SaveError = VoxelPackageError;
    type LoadError = VoxelPackageError;

    fn declaration() -> PackageDeclaration {
        PackageDeclaration::new(
            "voxel-sample",
            SemanticVersion::new(0, 1, 0),
            &[],
            PersistenceRequirement::new("voxel-worldline", SchemaVersion::new(0)),
            HostVersionRequirement::new(SemanticVersion::new(0, 1, 0)),
            RenderVocabularyRequirement::new("triangle-list-rgba", SemanticVersion::new(1, 0, 0)),
        )
    }

    fn ingest_batch(&mut self, batch: Self::InputBatch) -> Result<(), Self::Error> {
        self.pending.extend(batch.packets());
        Ok(())
    }

    fn update(&mut self) -> Result<bool, Self::Error> {
        let mut changed = self.controls.advance_automatic()?;
        changed |= self.refresh_selected_if_needed();
        let pending = core::mem::take(&mut self.pending);
        let mut authoritative_changed = false;
        for packet in pending {
            let previous_worldline = self.worldline.clone();
            let packet_changed = self.apply_packet(packet)?;
            changed |= packet_changed;
            authoritative_changed |= packet_changed && self.worldline != previous_worldline;
        }
        if authoritative_changed {
            self.refresh_selected();
        }
        Ok(changed)
    }

    fn present(&self) -> Result<Self::Frame, Self::Error> {
        Ok(frame_with_camera_and_controls(
            &self.selected,
            self.camera,
            self.controls.tau(),
            &self.controls,
        ))
    }

    fn save_selected(&self) -> Result<Vec<u8>, Self::SaveError> {
        Err(VoxelPackageError::PersistenceUnavailable)
    }

    fn load_selected(&mut self, _bytes: &[u8]) -> Result<(), Self::LoadError> {
        Err(VoxelPackageError::PersistenceUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use engine_api::{LogicalTime, Tau};
    use nworlds_host::{GamePackage, InputObservation, ObservationId, OrderedInputBatch};

    use super::{VoxelInputPacket, VoxelPackage};
    use crate::world::{VoxelFact, VoxelTool};

    fn batch(packet: VoxelInputPacket) -> OrderedInputBatch<VoxelInputPacket> {
        OrderedInputBatch::from_observations([InputObservation::new(
            ObservationId::new(0, 0),
            packet,
        )])
        .expect("test packet should form a batch")
    }

    #[test]
    fn center_click_publishes_a_removal() {
        let mut package = VoxelPackage::new();
        package.controls.pause();
        let before = crate::engine_integration::state_at_zero(&package.worldline)
            .payload()
            .voxels()
            .len();
        package
            .ingest_batch(batch(VoxelInputPacket::PrimaryClick { x: 480, y: 360 }))
            .expect("click batch should ingest");

        assert!(package.step().expect("click should step").0);
        assert_eq!(
            crate::engine_integration::state_at_zero(&package.worldline)
                .payload()
                .voxels()
                .len(),
            before - 1
        );
    }

    #[test]
    fn wheel_changes_the_fixed_point_scale() {
        let mut package = VoxelPackage::new();
        package
            .advance_visual_time(engine_api::Tau::from_ticks(7))
            .expect("visual time should advance");
        package
            .ingest_batch(batch(VoxelInputPacket::Wheel { milli_delta: 1 }))
            .expect("wheel batch should ingest");

        assert!(package.update().expect("wheel should update"));
        assert_eq!(package.visual_time(), engine_api::Tau::zero());
        assert_eq!(
            crate::engine_integration::state(&package.worldline, package.logical_time())
                .payload()
                .scale()
                .milli(),
            1_001
        );
    }

    #[test]
    fn selecting_fire_publishes_authoritative_tool_state() {
        let mut package = VoxelPackage::new();
        package.controls.pause();
        let parent = package.worldline.clone();
        let original = package.present().expect("default tool should present");

        package
            .ingest_batch(batch(VoxelInputPacket::SelectTool {
                tool: VoxelTool::Fire,
            }))
            .expect("tool selection should ingest");

        assert!(package.update().expect("tool selection should update"));
        assert_eq!(package.selected_tool(), VoxelTool::Fire);
        assert_eq!(
            package.worldline.journal().len(),
            parent.journal().len() + 1
        );
        assert_eq!(
            crate::engine_integration::state_at_zero(&package.worldline)
                .payload()
                .tool(),
            VoxelTool::Fire
        );
        assert_ne!(
            package.present().expect("fire tool should present"),
            original
        );
    }

    #[test]
    fn fire_selection_publishes_a_spawn_fire_fact_on_world_click() {
        let mut package = VoxelPackage::new();
        package.controls.pause();
        package
            .ingest_batch(batch(VoxelInputPacket::SelectTool {
                tool: VoxelTool::Fire,
            }))
            .expect("tool selection should ingest");
        package.update().expect("tool selection should update");
        let parent = package.worldline.clone();

        package
            .ingest_batch(batch(VoxelInputPacket::PrimaryClick { x: 480, y: 360 }))
            .expect("fire click should ingest");
        assert!(package.step().expect("fire click should step").0);

        assert_eq!(
            parent.journal().len() + 1,
            package.worldline.journal().len()
        );
        assert_eq!(
            crate::engine_integration::state_at_zero(&package.worldline)
                .payload()
                .fires()
                .len(),
            1
        );
    }

    #[test]
    fn palette_click_selects_fire_through_the_worldline() {
        let mut package = VoxelPackage::new();
        let parent = package.worldline.clone();
        package
            .ingest_batch(batch(VoxelInputPacket::PrimaryClick { x: 120, y: 47 }))
            .expect("palette click should ingest");

        assert!(package.update().expect("palette click should update"));
        assert_eq!(package.selected_tool(), VoxelTool::Fire);
        assert_eq!(
            package.worldline.journal().len(),
            parent.journal().len() + 1
        );
    }

    #[test]
    fn camera_controls_change_presentation_without_changing_the_worldline() {
        let mut package = VoxelPackage::new();
        let parent = package.worldline.clone();
        let original = package.present().expect("default camera should present");

        package
            .ingest_batch(batch(VoxelInputPacket::CameraOrbit {
                horizontal: 20,
                vertical: -10,
            }))
            .expect("camera batch should ingest");
        assert!(package.update().expect("camera update should succeed"));
        let rotated = package.present().expect("rotated camera should present");

        assert_eq!(package.worldline, parent);
        assert_ne!(original.payload(), rotated.payload());
    }

    #[test]
    fn camera_reset_restores_default_projection_and_authority_stays_unchanged() {
        let mut package = VoxelPackage::new();
        package.controls.pause();
        let original = package.present().expect("default camera should present");
        let parent = package.worldline.clone();

        package
            .ingest_batch(batch(VoxelInputPacket::CameraOrbit {
                horizontal: 20,
                vertical: -10,
            }))
            .expect("camera batch should ingest");
        package.update().expect("camera orbit should update");
        package
            .ingest_batch(batch(VoxelInputPacket::CameraReset))
            .expect("reset batch should ingest");
        package.update().expect("camera reset should update");

        assert_eq!(package.worldline, parent);
        assert_eq!(
            package.present().expect("reset camera should present"),
            original
        );
    }

    #[test]
    fn presentation_projects_the_selected_state_without_requerying_the_worldline() {
        let mut package = VoxelPackage::new();
        let position = crate::world::VoxelPosition::new(0, 1, -3);
        let child = crate::engine_integration::publish(
            &package.worldline,
            &mut package.writer,
            VoxelFact::Remove { position },
        );
        let selected = crate::engine_integration::state_at_zero(&child);
        package.selected = selected.clone();

        let frame = package.present().expect("selected state should present");
        let expected = crate::engine_integration::frame_with_camera_and_controls(
            &selected,
            package.camera(),
            package.visual_time(),
            &package.controls,
        );
        let parent_frame = crate::engine_integration::frame_with_camera_and_controls(
            &crate::engine_integration::state_at_zero(&package.worldline),
            package.camera(),
            package.visual_time(),
            &package.controls,
        );

        assert_eq!(frame, expected);
        assert_ne!(frame, parent_frame);
    }

    #[test]
    fn explicit_redraw_samples_logical_time_and_tau_without_mutating_cursors() {
        let package = VoxelPackage::new();
        let logical_time = engine_api::LogicalTime::from_ticks(17);
        let tau = engine_api::Tau::from_ticks(9);

        let frame = package.present_at(logical_time, tau);

        assert_eq!(frame.tau(), tau);
        assert_eq!(package.logical_time(), engine_api::LogicalTime::zero());
        assert_eq!(package.visual_time(), engine_api::Tau::zero());
    }

    #[test]
    fn programmatic_visual_time_advance_does_not_pause_automatic_mode() {
        let mut package = VoxelPackage::new();

        package
            .advance_visual_time(Tau::from_ticks(7))
            .expect("visual time should advance");

        assert_eq!(package.controls.mode(), engine_controls::PlaybackMode::Automatic);
        package.update().expect("automatic update should succeed");
        assert_eq!(package.logical_time(), LogicalTime::from_ticks(16));
        assert_eq!(package.visual_time(), Tau::from_ticks(23));
    }

    #[test]
    fn world_click_resynchronizes_a_scrubbed_view_to_authoring_time() {
        let mut package = VoxelPackage::new();
        package
            .writer
            .advance_to(LogicalTime::from_ticks(100))
            .expect("authoring time should advance");
        package
            .publish(VoxelFact::SelectTool {
                tool: VoxelTool::Remove,
            })
            .expect("late setup fact should publish");
        package.controls.set_logical_time(LogicalTime::from_ticks(50));
        package.refresh_selected();

        package
            .ingest_batch(batch(VoxelInputPacket::PrimaryClick { x: 480, y: 360 }))
            .expect("world click should ingest");
        package.update().expect("world click should update");

        assert_eq!(package.logical_time(), LogicalTime::from_ticks(100));
        assert_eq!(
            package
                .worldline
                .journal()
                .iter()
                .last()
                .expect("world click should append a fact")
                .logical_time(),
            LogicalTime::from_ticks(100)
        );
    }
}
