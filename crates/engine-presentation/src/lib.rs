#![forbid(unsafe_code)]

mod animation;
mod playback;
mod presentation;
mod renderer;

pub use animation::{animate, Animation};
pub use playback::LinearPlayback;
pub use presentation::{present, present_with_animation, AnimatedFrame, QueryAdapter};
pub use renderer::Renderer;
