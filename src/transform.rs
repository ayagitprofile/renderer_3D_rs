use glam::{Mat4, Quat, Vec3};

#[derive(Clone, Copy)]
pub struct Transform {
    model_matrix: Mat4,
    rotation: Quat,
}

impl Transform {
    pub const FORWARD: Vec3 = Vec3::new(0f32, 0f32, 1f32);
    pub const RIGHT: Vec3 = Vec3::new(1f32, 0f32, 0f32);
    pub const UP: Vec3 = Vec3::new(0f32, 1f32, 0f32);

    pub const fn identity() -> Self {
        Self {
            model_matrix: Mat4::IDENTITY,
            rotation: Quat::IDENTITY,
        }
    }

    pub fn from_model_matrix(model_matrix: Mat4) -> Self {
        Self {
            model_matrix: model_matrix,
            rotation: model_matrix.to_scale_rotation_translation().1,
        }
    }

    pub fn forward(&self) -> Vec3 {
        self.rotation * Transform::FORWARD
    }

    pub fn right(&self) -> Vec3 {
        self.rotation * Transform::RIGHT
    }

    pub fn up(&self) -> Vec3 {
        self.rotation * Transform::UP
    }

    pub fn set_position(&mut self, position: Vec3) {
        let (s, r, _) = self.model_matrix.to_scale_rotation_translation();
        self.model_matrix = Mat4::from_scale_rotation_translation(s, r, position);
    }

    pub fn position(&self) -> Vec3 {
        self.model_matrix.w_axis.truncate()
    }

    pub fn translate(&mut self, translation: Vec3) {
        let (s, r, t) = self.model_matrix.to_scale_rotation_translation();
        self.model_matrix = Mat4::from_scale_rotation_translation(s, r, t + translation);
    }

    pub fn set_euler_angles(&mut self, euler_angles_degrees: Vec3) {
        let (s, _, t) = self.model_matrix.to_scale_rotation_translation();

        self.rotation = Quat::from_euler(
            glam::EulerRot::XYZ,
            euler_angles_degrees.x.to_radians(),
            euler_angles_degrees.y.to_radians(),
            euler_angles_degrees.z.to_radians(),
        );

        self.model_matrix = Mat4::from_scale_rotation_translation(s, self.rotation, t);
    }

    pub fn set_rotation(&mut self, rotation: Quat) {
        let (s, _, t) = self.model_matrix.to_scale_rotation_translation();

        self.rotation = rotation;

        self.model_matrix = Mat4::from_scale_rotation_translation(s, self.rotation, t);
    }

    pub fn model_matrix(&self) -> &Mat4 {
        &self.model_matrix
    }

    pub fn model_rh_to_lh(m: Mat4) -> Mat4 {
        // let flip = Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0));
        #[rustfmt::skip]
        const RH_TO_LH_FLIP: Mat4 = Mat4::from_cols_array(&[
            1.0, 0.0,  0.0, 0.0,
            0.0, 1.0,  0.0, 0.0,
            0.0, 0.0, -1.0, 0.0,
            0.0, 0.0,  0.0, 1.0,
        ]);

        RH_TO_LH_FLIP * m * RH_TO_LH_FLIP
    }
}
