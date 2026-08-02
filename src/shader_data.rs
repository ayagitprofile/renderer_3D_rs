use glam::{Mat4, Vec4};

use crate::graphics;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GPUSideData {
    camera_vp_matrix: Mat4,
    camera_view_matrix: Mat4,
    camera_projection_matrix: Mat4,
    camera_position: Vec4,
    camera_forward: Vec4,
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
    pub fn set_storage_buffer_binding(&self, index: u32) {
        self.global_data_buffer
            .set_binding(graphics::buffer::BindingTarget::ShaderStorageBuffer, index);
    }

    pub fn upload_data(&self) {
        self.global_data_buffer
            .upload_data(self.gpu_side_data.as_slice());
    }

    pub fn set_camera_matrices(&mut self, view_matrix: &Mat4, projection_matrix: &Mat4) {
        self.gpu_side_data.camera_view_matrix = *view_matrix;
        self.gpu_side_data.camera_projection_matrix = *projection_matrix;
        self.gpu_side_data.camera_vp_matrix =
            self.gpu_side_data.camera_projection_matrix * self.gpu_side_data.camera_view_matrix;
    }

    pub fn new() -> Self {
        let mut buffer = graphics::buffer::GraphicsBuffer::new();

        let data = GPUSideData {
            camera_vp_matrix: Mat4::IDENTITY,
            camera_view_matrix: Mat4::IDENTITY,
            camera_projection_matrix: Mat4::IDENTITY,
            camera_forward: Vec4::ZERO,
            camera_position: Vec4::ZERO,
        };

        buffer.allocate(data.as_slice(), graphics::buffer::Usage::Dynamic);

        Self {
            global_data_buffer: buffer,
            gpu_side_data: data,
        }
    }
}
