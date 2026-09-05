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
    presentation: PresentationDriver<crate::world::VoxelState>,
}

impl VoxelPackage {
    pub fn new() -> Self {
        let (worldline, writer) = cottage_worldline();
        let presentation = PresentationDriver::new(state_at_zero(&worldline));
        Self {
            worldline,
            writer,
            pending: Vec::new(),
            viewport: (960, 720),
            presentation,
        }
    }

    fn apply_packet(&mut self, packet: VoxelInputPacket) -> bool {
        match packet {
            VoxelInputPacket::PrimaryClick { x, y } => {
                let sampled = state_at_zero(&self.worldline);
                let mut camera = Camera::default();
                camera.set_aspect(self.viewport.0.max(1) as f32 / self.viewport.1.max(1) as f32);
                let position = camera.pick(
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
        Ok(self
            .presentation
            .present::<crate::engine_integration::VoxelRenderer>())
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
}
