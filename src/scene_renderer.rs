use glam::Mat4;

use crate::{
    gl,
    graphics::{
        material_properties::{self, CullMode, DepthTestMode, MaterialProperties},
        mesh::{self, Mesh},
        shader::Shader,
    },
    scene::{Material, MeshNodeID, Scene},
};

pub struct SceneRenderer<'a> {
    scene: &'a Scene,
    current_mat_props: material_properties::MaterialProperties,
    test_scene_shader: Shader,
}

impl<'a> SceneRenderer<'a> {
    pub fn new(scene: &'a Scene) -> Self {
        let renderer = SceneRenderer {
            scene: scene,
            current_mat_props: MaterialProperties::DEFAULT,
            test_scene_shader: Scene::create_test_shader(),
        };

        set_depth_test_mode(renderer.current_mat_props.depth_test_mode);
        set_depth_writing(renderer.current_mat_props.depth_writing_enabled);
        set_cull_mode(renderer.current_mat_props.cull_mode);

        renderer
    }

    pub fn render(&mut self) {
        let scene = self.scene;

        for root_node_ref in scene.test_get_root_nodes_slice() {
            let node = scene.get_node(root_node_ref);

            self.render_node(root_node_ref, &Mat4::IDENTITY);

            for child_ref in node.children() {
                self.render_node(child_ref, node.transform.model_matrix());
            }
        }
    }

    fn render_node(&mut self, child_ref: &MeshNodeID, parent_model_matrix: &Mat4) {
        let child = self.scene.get_node(child_ref);

        let world_space_matrix = parent_model_matrix * child.transform.model_matrix();

        let child_mesh = self.scene.get_mesh(&child.mesh_ref);

        let child_material = self.scene.get_material(&child.material_ref);

        self.render_mesh(child_mesh, child_material, &world_space_matrix);
    }

    fn render_mesh(&mut self, mesh: &Mesh, material: &Material, model_matrix: &glam::Mat4) {
        mesh.vao().bind();

        let shader = &self.scene.get_shader(&material.shader_ref).shader;
        shader.bind();

        shader.set_uniform_mat4(
            shader.find_uniform_location("u_model_matrix"),
            &model_matrix.to_cols_array(),
        );

        self.set_mat_props(&material.material_properties);

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
