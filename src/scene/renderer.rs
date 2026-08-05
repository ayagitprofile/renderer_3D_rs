use std::rc::Rc;

use glam::Mat4;

use super::{data, scene::Scene};
use crate::gl;
use crate::graphics::material_properties::{CullMode, DepthTestMode, MaterialProperties};
use crate::graphics::mesh::Mesh;
use crate::graphics::texture::Texture;

pub struct Renderer {
    current_mat_props: MaterialProperties,
}

impl Renderer {
    pub fn new() -> Self {
        let renderer = Renderer {
            current_mat_props: MaterialProperties::DEFAULT,
        };

        set_depth_test_mode(renderer.current_mat_props.depth_test_mode);
        set_depth_writing(renderer.current_mat_props.depth_writing_enabled);
        set_cull_mode(renderer.current_mat_props.cull_mode);

        renderer
    }

    pub fn render(&mut self, scene: &Scene) {
        for root_node_id_ref in scene.root_node_iter() {
            let root_node_id = *root_node_id_ref;

            let node = scene.get_node(root_node_id);

            self.render_node(scene, root_node_id, &Mat4::IDENTITY);

            for child_id in node.children_iter() {
                self.render_node(scene, *child_id, node.transform.model_matrix());
            }
        }
    }

    fn render_node(&self, scene: &Scene, child_id: data::NodeID, parent_model_matrix: &Mat4) {
        let child = scene.get_node(child_id);

        let world_space_matrix = parent_model_matrix * child.transform.model_matrix();

        let child_mesh = scene.get_mesh(child.mesh_id);

        let child_material = scene.get_material(child.material_id);

        self.render_mesh(scene, child_mesh, child_material, &world_space_matrix);
    }

    fn render_mesh(&self, scene: &Scene, mesh: &Mesh, material: &data::Material, model_matrix: &glam::Mat4) {
        mesh.vao().bind();

        let shader = scene.get_shader(material.shader_id);
        shader.bind();

        if let Some(location) = shader.find_uniform_location("u_model_matrix") {
            shader.set_uniform_mat4(location, &model_matrix.to_cols_array());
        }

        for texture_id in material.texture_ids.iter() {
            let texture = scene.get_texture(*texture_id);
            let texture_name = scene.get_texture_name(*texture_id);

            shader.map_bindless_texture(
                shader.find_uniform_location(texture_name).unwrap(),
                texture.bindless_handle(),
            );
        }

        // self.set_mat_props(&material.material_properties);

        unsafe {
            gl::DrawElements(
                gl::TRIANGLES,
                mesh.index_count(),
                mesh.index_format().to_gl_format(),
                std::ptr::null(),
            );
        }
    }

    fn set_mat_props(&mut self, mat_props: &MaterialProperties) {
        if *mat_props == self.current_mat_props {
            return;
        }

        self.current_mat_props = *mat_props;

        set_depth_test_mode(self.current_mat_props.depth_test_mode);
        set_depth_writing(self.current_mat_props.depth_writing_enabled);
        set_cull_mode(self.current_mat_props.cull_mode);
    }
}

fn set_depth_writing(value: bool) {
    unsafe {
        gl::DepthMask(value as u8);
    }
}

fn set_depth_test_mode(value: DepthTestMode) {
    unsafe {
        match value {
            DepthTestMode::LessEqual => gl::DepthFunc(gl::LEQUAL),
            DepthTestMode::Equal => gl::DepthFunc(gl::EQUAL),
        }
    }
}

fn set_cull_mode(value: CullMode) {
    unsafe {
        if value == CullMode::Disabled {
            gl::Disable(gl::CULL_FACE);
            return;
        }

        gl::Enable(gl::CULL_FACE);

        match value {
            CullMode::Back => gl::CullFace(gl::BACK),
            CullMode::Front => gl::CullFace(gl::FRONT),
            CullMode::Both => gl::CullFace(gl::FRONT_AND_BACK),
            _ => {}
        }
    }
}
