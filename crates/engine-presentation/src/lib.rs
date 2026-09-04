#![forbid(unsafe_code)]

mod presentation;
mod render_batch;
mod renderer;

pub use presentation::present;
pub use render_batch::{RenderBatch, RenderVertex};
pub use renderer::Renderer;
