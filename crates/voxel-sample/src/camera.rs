use crate::world::{VoxelPosition, VoxelState};

pub use engine_api::{Camera, CameraPose, Matrix4, Ray, Vec3};

/// Voxel-specific picking over the engine camera's screen ray.
pub fn pick(
    camera: Camera,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    state: &VoxelState,
) -> Option<VoxelPosition> {
    let ray = camera.ray_from_screen(x, y, width, height);
    let scale = state.scale().as_f32();
    state
        .voxels()
        .iter()
        .filter_map(|voxel| {
            let position = voxel.position();
            let min = [
                position.x() as f32 * scale,
                position.y() as f32 * scale,
                position.z() as f32 * scale,
            ];
            let max = [min[0] + scale, min[1] + scale, min[2] + scale];
            ray_box_intersection(ray, min, max).map(|distance| (distance, position))
        })
        .min_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, position)| position)
}

fn ray_box_intersection(ray: Ray, min: Vec3, max: Vec3) -> Option<f32> {
    let origin = ray.origin();
    let direction = ray.direction();
    let mut near: f32 = 0.0;
    let mut far = f32::INFINITY;

    for axis in 0..3 {
        if direction[axis].abs() < f32::EPSILON {
            if origin[axis] < min[axis] || origin[axis] > max[axis] {
                return None;
            }
            continue;
        }

        let inverse = 1.0 / direction[axis];
        let mut near_axis = (min[axis] - origin[axis]) * inverse;
        let mut far_axis = (max[axis] - origin[axis]) * inverse;
        if near_axis > far_axis {
            std::mem::swap(&mut near_axis, &mut far_axis);
        }
        near = near.max(near_axis);
        far = far.min(far_axis);
        if near > far {
            return None;
        }
    }

    (far >= 0.0).then_some(near)
}

#[cfg(test)]
mod tests {
    use super::{pick, Camera};
    use crate::engine_integration::{cottage_worldline, state_at_zero};

    #[test]
    fn center_ray_hits_the_cottage() {
        let (worldline, _) = cottage_worldline();
        let sampled = state_at_zero(&worldline);

        let position = pick(
            Camera::default(),
            480.0,
            360.0,
            960.0,
            720.0,
            sampled.payload(),
        );

        assert!(position.is_some());
    }
}
