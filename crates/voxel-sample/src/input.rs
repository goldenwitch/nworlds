use nworlds_desktop::DesktopInputAdapter;
use nworlds_host::PacketIngress;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::package::VoxelInputPacket;

#[derive(Clone, Copy, Debug, Default)]
pub struct VoxelInputAdapter {
    cursor: Option<(i32, i32)>,
    orbit_anchor: Option<(i32, i32)>,
}

impl DesktopInputAdapter for VoxelInputAdapter {
    type Packet = VoxelInputPacket;

    fn translate(&mut self, event: &WindowEvent, ingress: &mut dyn PacketIngress<Self::Packet>) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let next = (position.x.round() as i32, position.y.round() as i32);
                if let Some(previous) = self.orbit_anchor {
                    ingress.push(VoxelInputPacket::CameraOrbit {
                        horizontal: next.0 - previous.0,
                        vertical: next.1 - previous.1,
                    });
                    self.orbit_anchor = Some(next);
                }
                self.cursor = Some(next);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let Some((x, y)) = self.cursor {
                    ingress.push(VoxelInputPacket::PrimaryClick {
                        x: x.max(0) as u32,
                        y: y.max(0) as u32,
                    });
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                self.orbit_anchor = self.cursor;
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Right,
                ..
            } => self.orbit_anchor = None,
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
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && !event.repeat =>
            {
                self.translate_key(event.physical_key, ingress);
            }
            _ => {}
        }
    }
}

impl VoxelInputAdapter {
    fn translate_key(
        &mut self,
        key: PhysicalKey,
        ingress: &mut dyn PacketIngress<VoxelInputPacket>,
    ) {
        let packet = match key {
            PhysicalKey::Code(KeyCode::KeyR) => Some(VoxelInputPacket::CameraReset),
            PhysicalKey::Code(KeyCode::Equal) | PhysicalKey::Code(KeyCode::NumpadAdd) => {
                Some(VoxelInputPacket::CameraZoom {
                    distance_milli: -500,
                })
            }
            PhysicalKey::Code(KeyCode::Minus) | PhysicalKey::Code(KeyCode::NumpadSubtract) => {
                Some(VoxelInputPacket::CameraZoom {
                    distance_milli: 500,
                })
            }
            _ => None,
        };
        if let Some(packet) = packet {
            ingress.push(packet);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VoxelInputAdapter;
    use crate::package::VoxelInputPacket;
    use nworlds_desktop::DesktopInputAdapter;
    use nworlds_host::{InputIngress, MemoryInputIngress};
    use winit::dpi::PhysicalPosition;
    use winit::event::{ElementState, MouseButton, WindowEvent};

    fn drain(adapter: &mut VoxelInputAdapter, event: WindowEvent) -> Vec<VoxelInputPacket> {
        let mut ingress = MemoryInputIngress::new();
        adapter.translate(&event, &mut ingress);
        ingress
            .drain()
            .expect("test input should normalize")
            .packets()
            .collect()
    }

    #[test]
    fn camera_keyboard_controls_emit_packets() {
        let mut adapter = VoxelInputAdapter::default();
        let mut ingress = MemoryInputIngress::new();

        adapter.translate_key(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyR),
            &mut ingress,
        );
        assert_eq!(
            ingress
                .drain()
                .expect("reset input should normalize")
                .packets()
                .collect::<Vec<_>>(),
            vec![VoxelInputPacket::CameraReset]
        );
        let mut ingress = MemoryInputIngress::new();
        adapter.translate_key(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Equal),
            &mut ingress,
        );
        assert_eq!(
            ingress
                .drain()
                .expect("zoom input should normalize")
                .packets()
                .collect::<Vec<_>>(),
            vec![VoxelInputPacket::CameraZoom {
                distance_milli: -500
            }]
        );
    }

    #[test]
    fn right_drag_emits_orbit_packets_between_press_and_release() {
        let mut adapter = VoxelInputAdapter::default();
        let moved = WindowEvent::CursorMoved {
            device_id: winit::event::DeviceId::dummy(),
            position: PhysicalPosition::new(100.0, 100.0),
        };
        let press = WindowEvent::MouseInput {
            device_id: winit::event::DeviceId::dummy(),
            state: ElementState::Pressed,
            button: MouseButton::Right,
        };
        let drag = WindowEvent::CursorMoved {
            device_id: winit::event::DeviceId::dummy(),
            position: PhysicalPosition::new(120.0, 90.0),
        };
        let release = WindowEvent::MouseInput {
            device_id: winit::event::DeviceId::dummy(),
            state: ElementState::Released,
            button: MouseButton::Right,
        };

        assert!(drain(&mut adapter, moved).is_empty());
        assert!(drain(&mut adapter, press).is_empty());
        assert_eq!(
            drain(&mut adapter, drag.clone()),
            vec![VoxelInputPacket::CameraOrbit {
                horizontal: 20,
                vertical: -10,
            }]
        );
        assert!(drain(&mut adapter, release).is_empty());
        assert!(drain(&mut adapter, drag).is_empty());
    }
}
