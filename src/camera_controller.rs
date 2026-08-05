use glam::{Quat, Vec3, vec2};

use crate::{
    camera::Camera,
    input::{self, Input},
    transform::Transform,
};

pub struct CameraController {
    camera_velocity: Vec3,
    pitch: f32,
    yaw: f32,
    pub ignore_input: bool,
}

impl CameraController {
    pub fn new() -> Self {
        Self {
            camera_velocity: Vec3::ZERO,
            pitch: 0f32,
            yaw: 0f32,
            ignore_input: false,
        }
    }

    pub fn update(&mut self, camera: &mut Camera, delta_time: f32, input: input::Input) {
        if !self.ignore_input {
            camera
                .transform
                .set_rotation(self.get_camera_rotation(input, delta_time));
        }

        camera
            .transform
            .translate(self.get_camera_translation(&camera.transform, input, delta_time));
    }

    fn apply_friction(velocity: Vec3, friction: f32, delta_time: f32) -> Vec3 {
        const STOP_SPEED: f32 = 0.1;

        let speed = velocity.length();

        if speed < 0.01 {
            return Vec3::ZERO;
        }

        let control = if speed < STOP_SPEED { STOP_SPEED } else { speed };

        let drop = control * friction * delta_time;

        let mut new_speed = speed - drop;

        if new_speed < 0.01 {
            new_speed = 0.0;
        }

        new_speed /= speed;

        return velocity * new_speed;
    }

    fn accelerate(velocity: Vec3, wish_dir: Vec3, wish_speed: f32, accel: f32, delta_time: f32) -> Vec3 {
        let current_speed = velocity.dot(wish_dir);

        let add_speed = wish_speed - current_speed;

        if add_speed <= 0.0 {
            return velocity;
        }

        let mut accel_speed = accel * wish_speed * delta_time;

        if accel_speed > add_speed {
            accel_speed = add_speed;
        }

        return velocity + wish_dir * accel_speed;
    }

    fn get_camera_rotation(&mut self, input: Input, delta_time: f32) -> Quat {
        const SENSITIVITY: f32 = 0.1;

        let mut mouse_delta = vec2(input.mouse_delta().0, input.mouse_delta().1);

        if input.get_key(input::Keycode::RIGHT) {
            mouse_delta.x += 1000f32 * delta_time;
        }
        if input.get_key(input::Keycode::LEFT) {
            mouse_delta.x -= 1000f32 * delta_time;
        }
        if input.get_key(input::Keycode::UP) {
            mouse_delta.y -= 1000f32 * delta_time;
        }
        if input.get_key(input::Keycode::DOWN) {
            mouse_delta.y += 1000f32 * delta_time;
        }

        self.pitch += mouse_delta.y * SENSITIVITY;
        self.pitch = self.pitch.clamp(-89f32, 89f32);

        self.yaw += mouse_delta.x * SENSITIVITY;
        self.yaw = self.yaw.rem_euclid(360f32);

        Quat::from_euler(glam::EulerRot::YXZ, self.yaw.to_radians(), self.pitch.to_radians(), 0.0)
    }

    fn get_camera_translation(&mut self, camera_transform: &Transform, input: Input, delta_time: f32) -> Vec3 {
        let speed = if input.get_key(input::Keycode::LSHIFT) {
            30f32
        } else if input.get_key(input::Keycode::LCTRL) {
            4f32
        } else {
            7f32
        };

        let mut movement = Vec3::ZERO;

        if input.get_key(input::Keycode::A) {
            movement.x -= 1.;
        }
        if input.get_key(input::Keycode::D) {
            movement.x += 1.;
        }
        if input.get_key(input::Keycode::W) {
            movement.z += 1.;
        }
        if input.get_key(input::Keycode::S) {
            movement.z -= 1.;
        }
        if input.get_key(input::Keycode::E) {
            movement.y += 1.;
        }
        if input.get_key(input::Keycode::Q) {
            movement.y -= 1.;
        }

        if movement.length_squared() > 0.01 {
            movement = movement.normalize();
        }

        if self.ignore_input {
            movement = Vec3::ZERO;
        }

        const FRICTION: f32 = 8f32;
        const ACCEL: f32 = 10f32;

        let mut wish_dir = camera_transform.right() * movement.x
            + camera_transform.forward() * movement.z
            + camera_transform.up() * movement.y;

        if wish_dir.length_squared() > 0.01 {
            wish_dir = wish_dir.normalize();
        }

        self.camera_velocity = CameraController::apply_friction(self.camera_velocity, FRICTION, delta_time);
        self.camera_velocity = CameraController::accelerate(self.camera_velocity, wish_dir, speed, ACCEL, delta_time);

        self.camera_velocity * delta_time
    }
}
