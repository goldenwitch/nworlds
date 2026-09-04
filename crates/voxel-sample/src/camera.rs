use crate::world::{VoxelPosition, VoxelState};

type Vec3 = [f32; 3];
type Matrix4 = [[f32; 4]; 4];

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    origin: Vec3,
    direction: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
    aspect: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            target: [0.0, 2.2, 0.0],
            yaw: 0.8,
            pitch: 0.52,
            distance: 18.0,
            aspect: 4.0 / 3.0,
        }
    }
}

impl Camera {
    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect.max(0.1);
    }

    pub fn view_projection(self) -> Matrix4 {
        multiply(
            perspective(45.0_f32.to_radians(), self.aspect, 0.1, 100.0),
            self.view(),
        )
    }

    pub fn ray_from_screen(self, x: f32, y: f32, width: f32, height: f32) -> Ray {
        let (eye, forward, right, up) = self.basis();
        let ndc_x = (2.0 * x / width.max(1.0)) - 1.0;
        let ndc_y = 1.0 - (2.0 * y / height.max(1.0));
        let half_fov = 45.0_f32.to_radians() * 0.5;
        let direction = normalize(add(
            add(forward, scale(right, ndc_x * half_fov.tan() * self.aspect)),
            scale(up, ndc_y * half_fov.tan()),
        ));
        Ray {
            origin: eye,
            direction,
        }
    }

    pub fn pick(
        self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        state: &VoxelState,
    ) -> Option<VoxelPosition> {
        let ray = self.ray_from_screen(x, y, width, height);
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

    fn basis(self) -> (Vec3, Vec3, Vec3, Vec3) {
        let cos_pitch = self.pitch.cos();
        let eye = add(
            self.target,
            [
                self.distance * cos_pitch * self.yaw.sin(),
                self.distance * self.pitch.sin(),
                self.distance * cos_pitch * self.yaw.cos(),
            ],
        );
        let forward = normalize(sub(self.target, eye));
        let right = normalize(cross(forward, [0.0, 1.0, 0.0]));
        let up = cross(right, forward);
        (eye, forward, right, up)
    }

    fn view(self) -> Matrix4 {
        let (eye, forward, right, up) = self.basis();
        [
            [right[0], up[0], -forward[0], 0.0],
            [right[1], up[1], -forward[1], 0.0],
            [right[2], up[2], -forward[2], 0.0],
            [-dot(right, eye), -dot(up, eye), dot(forward, eye), 1.0],
        ]
    }
}

fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Matrix4 {
    let focal = 1.0 / (fov_y * 0.5).tan();
    [
        [focal / aspect, 0.0, 0.0, 0.0],
        [0.0, focal, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), -1.0],
        [0.0, 0.0, (near * far) / (near - far), 0.0],
    ]
}

fn multiply(left: Matrix4, right: Matrix4) -> Matrix4 {
    let mut result = [[0.0; 4]; 4];
    for column in 0..4 {
        for row in 0..4 {
            result[column][row] = (0..4)
                .map(|index| left[index][row] * right[column][index])
                .sum();
        }
    }
    result
}

fn ray_box_intersection(ray: Ray, min: Vec3, max: Vec3) -> Option<f32> {
    let mut near: f32 = 0.0;
    let mut far = f32::INFINITY;

    for axis in 0..3 {
        let origin = ray.origin[axis];
        let direction = ray.direction[axis];
        if direction.abs() < f32::EPSILON {
            if origin < min[axis] || origin > max[axis] {
                return None;
            }
            continue;
        }

        let inverse = 1.0 / direction;
        let mut near_axis = (min[axis] - origin) * inverse;
        let mut far_axis = (max[axis] - origin) * inverse;
        if near_axis > far_axis {
            std::mem::swap(&mut near_axis, &mut far_axis);
        }
        near = near.max(near_axis);
        far = far.min(far_axis);
        if near > far {
            return None;
        }
    }

    if far < 0.0 {
        None
    } else {
        Some(near)
    }
}

fn add(left: Vec3, right: Vec3) -> Vec3 {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn sub(left: Vec3, right: Vec3) -> Vec3 {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn scale(vector: Vec3, amount: f32) -> Vec3 {
    [vector[0] * amount, vector[1] * amount, vector[2] * amount]
}

fn dot(left: Vec3, right: Vec3) -> f32 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross(left: Vec3, right: Vec3) -> Vec3 {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn normalize(vector: Vec3) -> Vec3 {
    let length = dot(vector, vector).sqrt();
    scale(vector, 1.0 / length.max(f32::EPSILON))
}

#[cfg(test)]
mod tests {
    use super::Camera;
    use crate::engine_integration::{cottage_worldline, state_at_zero};

    #[test]
    fn center_ray_hits_the_cottage() {
        let (worldline, _) = cottage_worldline();
        let sampled = state_at_zero(&worldline);

        let position = Camera::default().pick(480.0, 360.0, 960.0, 720.0, sampled.payload());

        assert!(position.is_some());
    }
}
