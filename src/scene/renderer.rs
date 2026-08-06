use glam::Mat4;

use super::{data, scene::Scene};
use crate::graphics::material_properties::MaterialProperties;
use crate::graphics::mesh::Mesh;
use crate::graphics::texture::Texture;
use crate::{gl, graphics};

pub struct Renderer {
    current_mat_props: MaterialProperties,
}

impl Renderer {
    pub fn new() -> Self {
        let renderer = Renderer {
            current_mat_props: MaterialProperties::DEFAULT,
        };

        graphics::utility::apply_mat_props(&renderer.current_mat_props);

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

        graphics::utility::apply_mat_props(&material.material_properties);

        unsafe {
            gl::DrawElements(
                gl::TRIANGLES,
                mesh.index_count(),
                mesh.index_format().to_gl_format(),
                std::ptr::null(),
            );
        }
    }
}
