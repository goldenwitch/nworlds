#![forbid(unsafe_code)]

mod application;
mod debug_console;
mod input;
mod wgpu;

pub use application::DesktopApplication;
pub use input::{DesktopInputAdapter, NoopInputAdapter};
pub use wgpu::{WgpuRenderError, WgpuRenderSink};
