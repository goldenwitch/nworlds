#![forbid(unsafe_code)]

mod application;
mod input;
mod wgpu;

pub use application::DesktopApplication;
pub use input::NoopInputAdapter;
pub use wgpu::{WgpuRenderError, WgpuRenderSink};
