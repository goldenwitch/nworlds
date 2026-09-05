use engine_api::{PresentationDriver, PresentationError, Tau};
use nworlds_host::{
    GamePackage, HostVersionRequirement, InputBatchError, OrderedInputBatch, PackageDeclaration,
    PersistenceRequirement, RenderVocabularyRequirement, SchemaVersion, SemanticVersion,
};

use crate::camera::Camera;
use crate::engine_integration::{
    cottage_worldline, publish, state_at_zero, VoxelFrame, VoxelJournalWriter, VoxelWorldline,
};
use crate::world::VoxelFact;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VoxelInputPacket {
    PrimaryClick { x: u32, y: u32 },
    Wheel { milli_delta: i32 },
    ViewportResized { width: u32, height: u32 },
    CameraOrbit { horizontal: i32, vertical: i32 },
    CameraZoom { distance_milli: i32 },
    CameraReset,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VoxelPackageError {
    PersistenceUnavailable,
}

pub struct VoxelPackage {
    worldline: VoxelWorldline,
    writer: VoxelJournalWriter,
    pending: Vec<VoxelInputPacket>,
    viewport: (u32, u32),
    camera: Camera,
    presentation: PresentationDriver<crate::world::VoxelState>,
}

impl VoxelPackage {
    pub fn new() -> Self {
        let (worldline, writer) = cottage_worldline();
        let presentation = PresentationDriver::new(state_at_zero(&worldline));
        let mut camera = Camera::default();
        camera.set_aspect(960.0 / 720.0);
        Self {
            worldline,
            writer,
            pending: Vec::new(),
            viewport: (960, 720),
            camera,
            presentation,
        }
    }

    fn apply_packet(&mut self, packet: VoxelInputPacket) -> bool {
        match packet {
            VoxelInputPacket::PrimaryClick { x, y } => {
                let sampled = state_at_zero(&self.worldline);
                let position = self.camera.pick(
                    x as f32,
                    y as f32,
                    self.viewport.0.max(1) as f32,
                    self.viewport.1.max(1) as f32,
                    sampled.payload(),
                );
                if let Some(position) = position {
                    self.publish(VoxelFact::Remove { position });
                    true
                } else {
                    false
                }
            }
            VoxelInputPacket::Wheel { milli_delta } => {
                let sampled = state_at_zero(&self.worldline);
                let current = sampled.payload().scale();
                let next = current.saturating_add_milli(milli_delta);
                if next == current {
                    false
                } else {
                    self.publish(VoxelFact::SetScale { scale: next });
                    true
                }
            }
            VoxelInputPacket::ViewportResized { width, height } => {
                self.viewport = (width, height);
                self.camera
                    .set_aspect(width.max(1) as f32 / height.max(1) as f32);
                true
            }
            VoxelInputPacket::CameraOrbit {
                horizontal,
                vertical,
            } => {
                self.camera
                    .orbit(horizontal as f32 * 0.01, -(vertical as f32) * 0.01);
                true
            }
            VoxelInputPacket::CameraZoom { distance_milli } => {
                self.camera.zoom(distance_milli as f32 / 1_000.0);
                true
            }
            VoxelInputPacket::CameraReset => {
                self.camera.reset();
                true
            }
        }
    }

    fn publish(&mut self, fact: VoxelFact) {
        self.worldline = publish(&self.worldline, &mut self.writer, fact);
    }

    /// Advances downstream visual time without querying or mutating the worldline.
    pub fn advance_visual_time(&mut self, delta: Tau) -> Result<Tau, PresentationError> {
        self.presentation.advance_visual_time(delta)
    }

    /// Returns the current downstream visual time.
    pub const fn visual_time(&self) -> Tau {
        self.presentation.visual_time()
    }

    pub const fn camera(&self) -> Camera {
        self.camera
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
    type Error = InputBatchError;
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
        let pending = core::mem::take(&mut self.pending);
        let mut changed = false;
        let mut authoritative_changed = false;
        for packet in pending {
            let authoritative_packet = matches!(
                packet,
                VoxelInputPacket::PrimaryClick { .. } | VoxelInputPacket::Wheel { .. }
            );
            let packet_changed = self.apply_packet(packet);
            changed |= packet_changed;
            authoritative_changed |= packet_changed && authoritative_packet;
        }
        if authoritative_changed {
            self.presentation.select(state_at_zero(&self.worldline));
        }
        Ok(changed)
    }

    fn present(&self) -> Result<Self::Frame, Self::Error> {
        Ok(crate::engine_integration::frame_with_camera(
            self.presentation.selected(),
            self.camera,
            self.presentation.visual_time(),
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
    use nworlds_host::{GamePackage, InputObservation, ObservationId, OrderedInputBatch};

    use super::{VoxelInputPacket, VoxelPackage};

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
            crate::engine_integration::state_at_zero(&package.worldline)
                .payload()
                .scale()
                .milli(),
            1_001
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
}
