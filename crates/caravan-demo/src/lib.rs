#![forbid(unsafe_code)]

pub mod input;
pub mod orchestrator;
pub mod package;
pub mod publication;
pub mod render;
pub mod stage;
pub mod transformation;

pub mod host {
    pub mod application;
    pub mod input;
    pub mod render;
    pub mod storage;
}

pub use orchestrator::{CaravanInteraction, CaravanOrchestrator, OrchestratorError};
pub use package::{demo_package, CaravanPackage};
pub use render::{CaravanRenderer, RenderActor, RenderOutput, RenderTile};
pub use stage::CaravanStage;
