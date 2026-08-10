use glam::{Mat4, Quat, Vec3};

use crate::{
    gl,
    graphics::{self, material_properties::MaterialProperties, mesh::Mesh, shader::Shader},
    obj_loader,
    shader_source::ShaderSource,
};

pub struct DebugGizmoRenderer {
    lightbulb_mesh: Mesh,
    shader: (Shader, MaterialProperties),
}

const ATTRIBS: [graphics::vertex::Attrib; 1] = [graphics::vertex::Attrib::POSITION];

impl DebugGizmoRenderer {
    pub fn new() -> Self {
        let mut mesh = graphics::mesh::Mesh::new();

        let lightbulb_obj = obj_loader::load_obj(std::path::Path::new("assets/debug/lightbulb.obj"));

        mesh.upload_vertex_buffer_data(
            &lightbulb_obj.positions,
            &graphics::vertex::VertexLayout::from_attribs(&ATTRIBS),
            graphics::buffer::Usage::Static,
        );
        mesh.upload_index_buffer_data(&lightbulb_obj.indices, graphics::buffer::Usage::Static);

        let shader_source =
            ShaderSource::load_from_file(std::path::Path::new("assets/shaders/debug_gizmo_shader.glsl"));

        Self {
            lightbulb_mesh: mesh,
            shader: (shader_source.compile(), *shader_source.mat_props()),
        }
    }

    pub fn render_lightbulb(&self, position: Vec3, scale: Vec3, color: Vec3) {
        let shader = &self.shader.0;

        unsafe {
            gl::Enable(gl::BLEND);
        }

        graphics::utility::apply_mat_props(&self.shader.1);

        let mat = Mat4::from_scale_rotation_translation(scale, Quat::IDENTITY, position);

        shader.set_uniform_mat4(
            shader.find_uniform_location("u_model_matrix").unwrap(),
            &mat.to_cols_array(),
        );

        shader.set_uniform_vec4(
            shader.find_uniform_location("u_color").unwrap(),
            &[color.x, color.y, color.z, 0.5f32],
        );

        shader.bind();
        self.lightbulb_mesh.vao().bind();

        unsafe {
            gl::DrawElements(
                gl::TRIANGLES,
                self.lightbulb_mesh.index_count(),
                self.lightbulb_mesh.index_format().to_gl_format(),
                std::ptr::null(),
            );
        }
    }
}
