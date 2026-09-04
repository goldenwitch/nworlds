#![forbid(unsafe_code)]

use nworlds_desktop::DesktopApplication;
use voxel_sample::{VoxelInputAdapter, VoxelPackage};
use winit::event_loop::EventLoop;

fn main() {
    let event_loop = EventLoop::new().expect("the voxel event loop should initialize");
    event_loop
        .run_app(&mut DesktopApplication::new(
            VoxelPackage::new(),
            VoxelInputAdapter::default(),
        ))
        .expect("the voxel event loop should run");
}
