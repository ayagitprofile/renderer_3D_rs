#![allow(dead_code)]
use glam::{Mat4, Vec3, vec3};

use crate::transform::Transform;

pub enum CameraProjection {
    Perspective { horizontal_fov: f32, aspect_ratio: f32 },
    Orthographic { width: f32, height: f32 },
}

impl CameraProjection {
    pub fn set_aspect_ratio(&mut self, value: f32) {
        if let CameraProjection::Perspective { aspect_ratio, .. } = self {
            *aspect_ratio = value;
        }
    }
}

pub struct Camera {
    pub transform: Transform,
    pub clipping_range: [f32; 2],
    pub projection: CameraProjection,
}

impl Camera {
    fn horizontal_to_vertical_fov(horizontal_fov: f32, aspect_ratio: f32) -> f32 {
        2.0 * ((horizontal_fov.to_radians() * 0.5).tan() / aspect_ratio).atan()
    }

    fn calculate_view_matrix(position: Vec3, forward: Vec3) -> Mat4 {
        let view_matrix_lh = if false {
            glam::Mat4::look_at_rh(position, position - forward, Transform::UP)
        } else {
            glam::Mat4::look_to_lh(position, forward, Transform::UP)
        };

        let lh_to_rh_matrix = Mat4::from_scale(vec3(1f32, 1f32, -1f32));

        lh_to_rh_matrix * view_matrix_lh
    }

    fn calculate_projection_matrix(horizontal_fov: f32, aspect_ratio: f32, z_near: f32, z_far: f32) -> Mat4 {
        let vertical_fov = Camera::horizontal_to_vertical_fov(horizontal_fov, aspect_ratio);
        glam::Mat4::perspective_rh_gl(vertical_fov, aspect_ratio, z_near, z_far)
    }

    pub fn clipping_range_mut(&mut self) -> &mut [f32; 2] {
        &mut self.clipping_range
    }

    pub fn new_perspective_camera(horizontal_fov: f32, aspect_ratio: f32, clipping_range: [f32; 2]) -> Self {
        let transform = Transform::identity();

        Self {
            transform: transform,
            clipping_range: clipping_range,
            projection: CameraProjection::Perspective {
                horizontal_fov,
                aspect_ratio,
            },
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        Camera::calculate_view_matrix(self.transform.position(), self.transform.forward())
    }

    pub fn projection_matrix(&self) -> Mat4 {
        match self.projection {
            CameraProjection::Orthographic { .. } => {
                todo!()
            }
            CameraProjection::Perspective {
                horizontal_fov,
                aspect_ratio,
            } => Camera::calculate_projection_matrix(
                horizontal_fov,
                aspect_ratio,
                self.clipping_range[0],
                self.clipping_range[1],
            ),
        }
    }
}
