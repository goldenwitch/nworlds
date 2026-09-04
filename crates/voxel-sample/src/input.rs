use nworlds_desktop::DesktopInputAdapter;
use nworlds_host::PacketIngress;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};

use crate::package::VoxelInputPacket;

#[derive(Clone, Copy, Debug, Default)]
pub struct VoxelInputAdapter {
    cursor: Option<(u32, u32)>,
}

impl DesktopInputAdapter for VoxelInputAdapter {
    type Packet = VoxelInputPacket;

    fn translate(&mut self, event: &WindowEvent, ingress: &mut dyn PacketIngress<Self::Packet>) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = Some((position.x.max(0.0) as u32, position.y.max(0.0) as u32));
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let Some((x, y)) = self.cursor {
                    ingress.push(VoxelInputPacket::PrimaryClick { x, y });
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let milli_delta = match delta {
                    MouseScrollDelta::LineDelta(_, lines) => (lines * 60.0).round() as i32,
                    MouseScrollDelta::PixelDelta(delta) => (delta.y as f32 / 4.0).round() as i32,
                };
                if milli_delta != 0 {
                    ingress.push(VoxelInputPacket::Wheel { milli_delta });
                }
            }
            WindowEvent::Resized(size) => {
                ingress.push(VoxelInputPacket::ViewportResized {
                    width: size.width,
                    height: size.height,
                });
            }
            _ => {}
        }
    }
}
