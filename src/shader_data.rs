use glam::{Mat4, Vec2, Vec4, vec2, vec4};

use crate::{graphics, transform::Transform};

#[repr(C)]
#[derive(Clone, Copy)]
struct GPUSideData {
    camera_vp_matrix: Mat4,
    camera_view_matrix: Mat4,
    camera_projection_matrix: Mat4,
    camera_inverse_projection_matrix: Mat4,
    camera_position: Vec4,
    camera_forward: Vec4,
    screen_size: Vec2,
    camera_panes: Vec2,
}

impl GPUSideData {
    pub fn as_slice(&self) -> &[GPUSideData] {
        unsafe { std::slice::from_raw_parts(self as *const GPUSideData, 1) }
    }
}

pub struct GlobalShaderData {
    gpu_side_data: GPUSideData,
    global_data_buffer: graphics::buffer::GraphicsBuffer,
}

impl GlobalShaderData {
    fn set_storage_buffer_binding(&self, index: u32) {
        self.global_data_buffer
            .set_binding(graphics::buffer::BindingTarget::ShaderStorageBuffer, index);
    }

    pub fn upload_data(&self) {
        self.global_data_buffer.upload_data(self.gpu_side_data.as_slice());
    }

    pub fn set_screen_size(&mut self, width: u32, height: u32) {
        self.gpu_side_data.screen_size = vec2(width as f32, height as f32);
    }

    fn set_camera_matrices(&mut self, view_matrix: &Mat4, projection_matrix: &Mat4) {
        self.gpu_side_data.camera_inverse_projection_matrix = projection_matrix.inverse();
        self.gpu_side_data.camera_view_matrix = *view_matrix;
        self.gpu_side_data.camera_projection_matrix = *projection_matrix;
        self.gpu_side_data.camera_vp_matrix =
            self.gpu_side_data.camera_projection_matrix * self.gpu_side_data.camera_view_matrix;
    }

    pub fn set_camera_data(
        &mut self,
        view_matrix: &Mat4,
        projection_matrix: &Mat4,
        camera_transform: &Transform,
        clip_range: [f32; 2],
    ) {
        self.set_camera_matrices(view_matrix, projection_matrix);

        let position = camera_transform.position();

        self.gpu_side_data.camera_position = vec4(position.x, position.y, position.z, 0f32);

        let forward = camera_transform.forward();

        self.gpu_side_data.camera_forward = vec4(forward.x, forward.y, forward.z, 0f32);

        self.gpu_side_data.camera_panes = Vec2::from_array(clip_range);
    }

    pub fn new() -> Self {
        let mut buffer = graphics::buffer::GraphicsBuffer::new();

        let data = GPUSideData {
            camera_vp_matrix: Mat4::IDENTITY,
            camera_view_matrix: Mat4::IDENTITY,
            camera_projection_matrix: Mat4::IDENTITY,
            camera_inverse_projection_matrix: Mat4::IDENTITY,
            camera_forward: Vec4::ZERO,
            camera_position: Vec4::ZERO,
            screen_size: Vec2::ZERO,
            camera_panes: Vec2::ZERO,
        };

        buffer.allocate(data.as_slice(), graphics::buffer::Usage::Dynamic);

        let data = GlobalShaderData {
            global_data_buffer: buffer,
            gpu_side_data: data,
        };

        data.set_storage_buffer_binding(crate::scene::buffers::SHADER_SHARED_DATA_BUFFER_BINDING_INDEX);

        data
    }
}
