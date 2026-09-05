#![forbid(unsafe_code)]

pub mod engine_integration;
pub mod input;
pub mod interaction;
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

pub use interaction::CaravanInteraction;
pub use orchestrator::{CaravanOrchestrator, OrchestratorError};
pub use package::{sample_package, CaravanPackage};
pub use render::{CaravanRenderer, RenderActor, RenderOutput, RenderTile};
pub use stage::CaravanStage;
