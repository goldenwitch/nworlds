#![forbid(unsafe_code)]

use engine_api::{Frame, RenderBatch};
use nworlds_desktop::{DesktopApplication, NoopInputAdapter};
use nworlds_host::GamePackage;
use nworlds_host::{
    HostVersionRequirement, InputBatchError, OrderedInputBatch, PackageDeclaration,
    PersistenceRequirement, RenderVocabularyRequirement, SchemaVersion, SemanticVersion,
};
use winit::event_loop::EventLoop;

struct SyntheticPackage;

impl GamePackage for SyntheticPackage {
    type InputBatch = OrderedInputBatch<()>;
    type Frame = Frame<RenderBatch>;
    type Error = InputBatchError;
    type SaveError = core::convert::Infallible;
    type LoadError = core::convert::Infallible;

    fn declaration() -> PackageDeclaration {
        PackageDeclaration::new(
            "synthetic-desktop-host",
            SemanticVersion::new(0, 1, 0),
            &[],
            PersistenceRequirement::new("synthetic", SchemaVersion::new(1)),
            HostVersionRequirement::new(SemanticVersion::new(0, 1, 0)),
            RenderVocabularyRequirement::new("triangle-list-rgba", SemanticVersion::new(1, 0, 0)),
        )
    }

    fn ingest_batch(&mut self, _batch: Self::InputBatch) -> Result<(), Self::Error> {
        Ok(())
    }

    fn update(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }

    fn present(&self) -> Result<Self::Frame, Self::Error> {
        Ok(Frame::new(engine_api::Tau::zero(), RenderBatch::empty()))
    }

    fn save_selected(&self) -> Result<Vec<u8>, Self::SaveError> {
        Ok(Vec::new())
    }

    fn load_selected(&mut self, _bytes: &[u8]) -> Result<(), Self::LoadError> {
        Ok(())
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("the desktop event loop should initialize");
    event_loop
        .run_app(&mut DesktopApplication::new(
            SyntheticPackage,
            NoopInputAdapter,
        ))
        .expect("the desktop event loop should run");
}
