use glam::{Mat4, Vec3, vec3};

use crate::transform::Transform;

pub struct Camera {
    pub transform: Transform,
    pub horizontal_fov: f32,
    pub clipping_range: [f32; 2],
    pub aspect_ratio: f32,
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

    fn calculate_projection_matrix(
        horizontal_fov: f32,
        aspect_ratio: f32,
        z_near: f32,
        z_far: f32,
    ) -> Mat4 {
        let vertical_fov = Camera::horizontal_to_vertical_fov(horizontal_fov, aspect_ratio);
        glam::Mat4::perspective_rh_gl(vertical_fov, aspect_ratio, z_near, z_far)
    }

    pub fn new(fov: f32, aspect_ratio: f32, clipping_range: [f32; 2]) -> Self {
        let transform = Transform::identity();

        Self {
            transform: transform,
            horizontal_fov: fov,
            clipping_range: clipping_range,
            aspect_ratio: aspect_ratio,
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        Camera::calculate_view_matrix(self.transform.position(), self.transform.forward())
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Camera::calculate_projection_matrix(
            self.horizontal_fov,
            self.aspect_ratio,
            self.clipping_range[0],
            self.clipping_range[1],
        )
    }
}
