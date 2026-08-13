use glam::{Mat4, Quat, Vec3, vec3};

use crate::{
    gl,
    graphics::{self, material_properties::MaterialProperties, mesh::Mesh, shader::Shader},
    obj_loader,
    shader_source::ShaderSource,
    transform::Transform,
};

pub struct DebugGizmoRenderer {
    lightbulb_mesh: Mesh,
    sun_mesh: Mesh,
    cylinder_mesh: Mesh,
    shader: (Shader, MaterialProperties),
}

const ATTRIBS: [graphics::vertex::Attrib; 1] = [graphics::vertex::Attrib::POSITION];

impl DebugGizmoRenderer {
    pub fn new() -> Self {
        let layout = graphics::vertex::VertexLayout::from_attribs(&ATTRIBS);

        let mut lightbulb_mesh = graphics::mesh::Mesh::new();

        let lightbulb_obj = obj_loader::load_obj(std::path::Path::new("assets/debug/lightbulb.obj"));

        lightbulb_mesh.upload_vertex_buffer_data(&lightbulb_obj.positions, &layout, graphics::buffer::Usage::Static);
        lightbulb_mesh.upload_index_buffer_data(&lightbulb_obj.indices, graphics::buffer::Usage::Static);

        let shader_source =
            ShaderSource::load_from_file(std::path::Path::new("assets/shaders/debug_gizmo_shader.glsl"));

        let mut sun_mesh = graphics::mesh::Mesh::new();

        let sun_obj = obj_loader::load_obj(std::path::Path::new("assets/debug/sun.obj"));

        sun_mesh.upload_vertex_buffer_data(&sun_obj.positions, &layout, graphics::buffer::Usage::Static);
        sun_mesh.upload_index_buffer_data(&sun_obj.indices, graphics::buffer::Usage::Static);

        let cylinder_obj = obj_loader::load_obj(std::path::Path::new("assets/debug/cylinder.obj"));

        let mut cylinder_mesh = graphics::mesh::Mesh::new();

        cylinder_mesh.upload_vertex_buffer_data(&cylinder_obj.positions, &layout, graphics::buffer::Usage::Static);
        cylinder_mesh.upload_index_buffer_data(&cylinder_obj.indices, graphics::buffer::Usage::Static);

        Self {
            lightbulb_mesh,
            shader: (shader_source.compile(), *shader_source.mat_props()),
            sun_mesh,
            cylinder_mesh,
        }
    }

    pub fn render_directional_light(&self, color: Vec3, direction: Vec3, camera_position: Vec3) {
        let shader = &self.shader.0;
        graphics::utility::apply_mat_props(&self.shader.1);

        shader.set_uniform_vec4(
            shader.find_uniform_location("u_color").unwrap(),
            &[color.x, color.y, color.z, 0.5f32],
        );

        {
            let mut transform = Transform::from_model_matrix(Mat4::from_scale_rotation_translation(
                vec3(0.1f32, 0.1f32, 12.5f32),
                Quat::IDENTITY,
                -6.25f32 * direction,
            ));

            transform.look_at(direction);

            shader.set_uniform_mat4(
                shader.find_uniform_location("u_model_matrix").unwrap(),
                &transform.model_matrix().to_cols_array(),
            );

            self.cylinder_mesh.vao().bind();

            unsafe {
                gl::DrawElements(
                    gl::TRIANGLES,
                    self.sun_mesh.index_count(),
                    self.sun_mesh.index_format().to_gl_format(),
                    std::ptr::null(),
                );
            }
        }

        shader.bind();
        {
            let mut transform = Transform::from_model_matrix(Mat4::from_scale_rotation_translation(
                Vec3::ONE * 0.5f32,
                Quat::IDENTITY,
                Vec3::ZERO,
            ));

            transform.look_at(camera_position);

            shader.set_uniform_mat4(
                shader.find_uniform_location("u_model_matrix").unwrap(),
                &transform.model_matrix().to_cols_array(),
            );

            self.sun_mesh.vao().bind();

            unsafe {
                gl::Enable(gl::BLEND);

                gl::DrawElements(
                    gl::TRIANGLES,
                    self.sun_mesh.index_count(),
                    self.sun_mesh.index_format().to_gl_format(),
                    std::ptr::null(),
                );
            }
        }

        {
            let color = [0.25f32, 0.25f32, 0.25f32, 0.5f32];
            shader.set_uniform_vec4(shader.find_uniform_location("u_color").unwrap(), &color);

            let mut transform = Transform::from_model_matrix(Mat4::from_scale_rotation_translation(
                vec3(0.1f32, 0.1f32, 12.5f32),
                Quat::IDENTITY,
                6.25f32 * direction,
            ));

            transform.look_at(direction);

            shader.set_uniform_mat4(
                shader.find_uniform_location("u_model_matrix").unwrap(),
                &transform.model_matrix().to_cols_array(),
            );

            self.cylinder_mesh.vao().bind();

            unsafe {
                gl::DrawElements(
                    gl::TRIANGLES,
                    self.sun_mesh.index_count(),
                    self.sun_mesh.index_format().to_gl_format(),
                    std::ptr::null(),
                );
            }
        }
    }

    pub fn render_lightbulb(&self, position: Vec3, scale: Vec3, color: Vec3) {
        let shader = &self.shader.0;

        graphics::utility::apply_mat_props(&self.shader.1);

        let transform = Transform::from_position_and_scale(position, scale);

        shader.set_uniform_mat4(
            shader.find_uniform_location("u_model_matrix").unwrap(),
            &transform.model_matrix().to_cols_array(),
        );

        shader.set_uniform_vec4(
            shader.find_uniform_location("u_color").unwrap(),
            &[color.x, color.y, color.z, 0.5f32],
        );

        shader.bind();
        self.lightbulb_mesh.vao().bind();

        unsafe {
            gl::Enable(gl::BLEND);

            gl::DrawElements(
                gl::TRIANGLES,
                self.lightbulb_mesh.index_count(),
                self.lightbulb_mesh.index_format().to_gl_format(),
                std::ptr::null(),
            );
        }
    }
}
