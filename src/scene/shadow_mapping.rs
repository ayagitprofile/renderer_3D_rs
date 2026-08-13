use glam::Mat4;

use crate::{
    graphics::{buffer::GraphicsBuffer, framebuffer::Framebuffer, shader::Shader, texture},
    shader_source::ShaderSource,
};

pub struct ShadowMapper {
    pub framebuffer: Framebuffer,
    pub shadow_caster_shader: Shader,

    uniform_buffer: GraphicsBuffer,

    os_to_ws_mat_location: i32,
}

impl ShadowMapper {
    pub const UNIFORM_DATA_BUFFER_BINDING: u32 = 0;

    const LIGHT_SPACE_MATRIX_NAME: &str = "u_light_vp_matrix";
    const MODEL_MATRIX_NAME: &str = "u_model_matrix";

    pub fn set_uiform_buffer_ws_to_light_space_matrix(&self, matrix: &Mat4) {
        self.uniform_buffer.upload_data(&[*matrix]);
    }

    pub fn bind_uniform_buffer(&self) {
        self.uniform_buffer.set_binding(
            crate::graphics::buffer::BindingTarget::UniformBuffer,
            ShadowMapper::UNIFORM_DATA_BUFFER_BINDING,
        );
    }

    pub fn set_shader_os_to_ws_matrix(&self, matrix: &Mat4) {
        self.shadow_caster_shader
            .set_uniform_mat4(self.os_to_ws_mat_location, &matrix.to_cols_array());
    }

    pub fn new(resolution: (u32, u32)) -> Self {
        let mut framebuffer = Framebuffer::new(resolution);

        framebuffer.create_depth_attachment(
            texture::StorageFormat::Depth32,
            texture::WrappingMode::ClampWithBorderColor { color: [1f32; 4] },
        );

        let shader_source =
            ShaderSource::load_from_file(std::path::Path::new("assets/shaders/scene_shadow_caster_shader.glsl"));

        let shader = shader_source.compile();
        
        let mut buffer = GraphicsBuffer::new();
        buffer.allocate(&[Mat4::IDENTITY], crate::graphics::buffer::Usage::Dynamic);

        Self {
            uniform_buffer: buffer,
            os_to_ws_mat_location: shader.find_uniform_location(ShadowMapper::MODEL_MATRIX_NAME).unwrap(),

            framebuffer,
            shadow_caster_shader: shader,
        }
    }
}
