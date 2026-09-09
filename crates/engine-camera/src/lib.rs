#![forbid(unsafe_code)]

pub type Vec3 = [f32; 3];
pub type Matrix4 = [[f32; 4]; 4];

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    origin: Vec3,
    direction: Vec3,
}

impl Ray {
    pub const fn origin(self) -> Vec3 {
        self.origin
    }

    pub const fn direction(self) -> Vec3 {
        self.direction
    }
}

/// View pose shared by target-neutral presentation clients.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraPose {
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
}

impl CameraPose {
    pub const MIN_PITCH: f32 = -1.45;
    pub const MAX_PITCH: f32 = 1.45;
    pub const MIN_DISTANCE: f32 = 6.0;
    pub const MAX_DISTANCE: f32 = 40.0;

    pub fn new(target: Vec3, yaw: f32, pitch: f32, distance: f32) -> Self {
        Self {
            target,
            yaw,
            pitch: pitch.clamp(Self::MIN_PITCH, Self::MAX_PITCH),
            distance: distance.clamp(Self::MIN_DISTANCE, Self::MAX_DISTANCE),
        }
    }

    pub const fn target(self) -> Vec3 {
        self.target
    }

    pub const fn yaw(self) -> f32 {
        self.yaw
    }

    pub const fn pitch(self) -> f32 {
        self.pitch
    }

    pub const fn distance(self) -> f32 {
        self.distance
    }
}

impl Default for CameraPose {
    fn default() -> Self {
        Self::new([0.0, 2.2, 0.0], 0.8, 0.52, 18.0)
    }
}

/// A target-neutral orbit camera used by presentation clients.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pose: CameraPose,
    aspect: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pose: CameraPose::default(),
            aspect: 4.0 / 3.0,
        }
    }
}

impl Camera {
    pub const fn from_pose(pose: CameraPose) -> Self {
        Self {
            pose,
            aspect: 4.0 / 3.0,
        }
    }

    pub const fn pose(self) -> CameraPose {
        self.pose
    }

    pub const fn aspect(self) -> f32 {
        self.aspect
    }

    pub fn set_pose(&mut self, pose: CameraPose) {
        self.pose = pose;
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect.max(0.1);
    }

    pub fn orbit(&mut self, yaw_delta: f32, pitch_delta: f32) {
        self.pose = CameraPose::new(
            self.pose.target,
            self.pose.yaw + yaw_delta,
            self.pose.pitch + pitch_delta,
            self.pose.distance,
        );
    }

    pub fn zoom(&mut self, distance_delta: f32) {
        self.pose = CameraPose::new(
            self.pose.target,
            self.pose.yaw,
            self.pose.pitch,
            self.pose.distance + distance_delta,
        );
    }

    pub fn reset(&mut self) {
        let aspect = self.aspect;
        *self = Self::default();
        self.aspect = aspect;
    }

    pub fn view_projection(self) -> Matrix4 {
        multiply(
            perspective(45.0_f32.to_radians(), self.aspect, 0.1, 100.0),
            self.view(),
        )
    }

    pub fn project_point(self, point: Vec3) -> [f32; 3] {
        let matrix = self.view_projection();
        let vector = [point[0], point[1], point[2], 1.0];
        let clip = [
            (0..4)
                .map(|index| matrix[index][0] * vector[index])
                .sum::<f32>(),
            (0..4)
                .map(|index| matrix[index][1] * vector[index])
                .sum::<f32>(),
            (0..4)
                .map(|index| matrix[index][2] * vector[index])
                .sum::<f32>(),
            (0..4)
                .map(|index| matrix[index][3] * vector[index])
                .sum::<f32>(),
        ];
        [clip[0] / clip[3], clip[1] / clip[3], clip[2] / clip[3]]
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

    fn basis(self) -> (Vec3, Vec3, Vec3, Vec3) {
        let cos_pitch = self.pose.pitch.cos();
        let eye = add(
            self.pose.target,
            [
                self.pose.distance * cos_pitch * self.pose.yaw.sin(),
                self.pose.distance * self.pose.pitch.sin(),
                self.pose.distance * cos_pitch * self.pose.yaw.cos(),
            ],
        );
        let forward = normalize(sub(self.pose.target, eye));
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

    #[test]
    fn orbit_changes_projection_and_reset_restores_default_view() {
        let camera = Camera::default();
        let point = [0.0, 2.2, 0.0];
        let original = camera.project_point(point);
        let mut rotated = camera;

        rotated.orbit(0.4, -0.2);
        assert_ne!(rotated.project_point(point), original);

        rotated.reset();
        assert_eq!(rotated.project_point(point), original);
    }

    #[test]
    fn pose_limits_are_clamped() {
        let pose = super::CameraPose::new([0.0, 0.0, 0.0], 0.0, 100.0, 0.0);
        assert_eq!(pose.pitch(), super::CameraPose::MAX_PITCH);
        assert_eq!(pose.distance(), super::CameraPose::MIN_DISTANCE);
    }
}
