use core::f32;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub fovy: f32,
    pub aspect: f32,
    pub znear: f32,
    pub zfar: f32,
    pub zoom: f32,
}

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Self {
            position: [0.0, 0.0, 10.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            fovy: std::f32::consts::PI / 4.0,
            aspect,
            znear: 0.01,
            zfar: 1000.0,
            zoom: 1.0,
        }
    }

    pub fn map_view(center_lon: f32, center_lat: f32, distance: f32, aspect: f32) -> Self {
        Self {
            position: [center_lon, center_lat + distance * 0.5, distance],
            target: [center_lon, center_lat, 0.0],
            up: [0.0, 1.0, 0.0],
            fovy: std::f32::consts::PI / 4.0,
            aspect,
            znear: 0.01,
            zfar: 1000.0,
            zoom: 1.0,
        }
    }

    pub fn fit_bounds(
        min_lon: f32,
        max_lon: f32,
        min_lat: f32,
        max_lat: f32,
        min_alt: f32,
        max_alt: f32,
        aspect: f32,
    ) -> Self {
        let center_lon = (min_lon + max_lon) / 2.0;
        let center_lat = (min_lat + max_lat) / 2.0;
        let center_alt = (min_alt + max_alt) / 2.0;

        let lon_span = max_lon - min_lon;
        let lat_span = max_lat - min_lat;
        let alt_span = max_alt - min_alt;

        let span = lon_span.max(lat_span).max(alt_span);
        let distance = span * 2.0;

        Self {
            position: [center_lon, center_lat - distance * 0.3, distance * 0.7],
            target: [center_lon, center_lat, center_alt],
            up: [0.0, 1.0, 0.0],
            fovy: std::f32::consts::PI / 4.0,
            aspect,
            znear: 0.01,
            zfar: distance * 3.0,
            zoom: 1.0,
        }
    }

    pub fn build_view_projection_matrix(&self) -> CameraUniform {
        let eye = Vec3::from(self.position);
        let target = Vec3::from(self.target);
        let up = Vec3::from(self.up);
        let view = Mat4::look_at_rh(eye, target, up);

        let proj = Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar);

        let view_proj = proj * view;

        CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
        }
    }

    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        let eye = Vec3::from(self.position);
        let target = Vec3::from(self.target);
        let world_up = Vec3::from(self.up);

        let forward = (target - eye).normalize();
        let right = forward.cross(world_up).normalize();

        let right = if right.length_squared() < 0.001 {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            right
        };

        let _up = right.cross(forward).normalize();

        let rotation_horizontal = Quat::from_axis_angle(world_up, delta_x);
        let rotation_vertical = Quat::from_axis_angle(right, delta_y);
        let combined_rotation = rotation_horizontal * rotation_vertical;

        let offset = eye - target;
        let rotated_offset = combined_rotation * offset;

        self.position = (target + rotated_offset).to_array();
    }

    pub fn dolly(&mut self, factor: f32) {
        let eye = Vec3::from(self.position);
        let target = Vec3::from(self.target);

        let direction = target - eye;
        let distance = direction.length();

        // todo: probably make these into constants
        let min_distance = 0.1;
        let max_distance = 100.0;

        let new_distance = (distance * (1.0 - factor)).clamp(min_distance, max_distance);
        let direction_normalized = direction.normalize();

        self.position = (target - direction_normalized * new_distance).to_array();
    }

    pub fn dolly_to_point(&mut self, world_point: Vec3, factor: f32) {
        let target = Vec3::from(self.target);

        let new_target = target.lerp(world_point, factor.abs() * 0.1);

        self.target = new_target.to_array();

        self.dolly(factor);
    }

    pub fn pan(&mut self, right: f32, up: f32) {
        let eye = Vec3::from(self.position);
        let target = Vec3::from(self.target);
        let up_vec = Vec3::from(self.up);

        let f = (target - eye).normalize();
        let s = f.cross(up_vec).normalize();
        let u = s.cross(f);

        let offset = s * right + u * up;

        self.position[0] += offset.x;
        self.position[1] += offset.y;
        self.position[2] += offset.z;

        self.target[0] += offset.x;
        self.target[1] += offset.y;
        self.target[2] += offset.z;
    }

    pub fn update_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }

    // forward and back (this is kinda jank atm)
    pub fn fly_forward(&mut self, amount: f32) {
        let eye = Vec3::from(self.position);
        let target = Vec3::from(self.target);

        let mut forward = target - eye;
        forward.z = 0.0;

        if forward.length_squared() > 0.001 {
            forward = forward.normalize();

            let offset = forward * amount;
            self.position[0] += offset.x;
            self.position[1] += offset.y;
            self.target[0] += offset.x;
            self.target[1] += offset.y;
        }
    }

    /// strafing (this is kinda jank atm)
    pub fn fly_right(&mut self, amount: f32) {
        let eye = Vec3::from(self.position);
        let target = Vec3::from(self.target);

        let mut forward = target - eye;
        forward.z = 0.0;

        if forward.length_squared() > 0.001 {
            forward = forward.normalize();

            let right = Vec3::new(forward.y, -forward.x, 0.0);

            let offset = right * amount;
            self.position[0] += offset.x;
            self.position[1] += offset.y;
            self.target[0] += offset.x;
            self.target[1] += offset.y;
        }
    }

    pub fn set_top_down_view(&mut self, distance: f32) {
        self.position = [self.target[0], self.target[1], self.target[2] + distance];
        self.up = [0.0, 1.0, 0.0];
    }

    pub fn set_perspective_view(&mut self, distance: f32) {
        let angle = f32::consts::PI / 4.0;
        let cos_45 = angle.cos();
        let sin_45 = angle.sin();

        self.position = [
            self.target[0] + distance * cos_45,
            self.target[1] - distance * cos_45,
            self.target[2] + distance * sin_45,
        ];
        self.up = [0.0, 0.0, 1.0];
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}
