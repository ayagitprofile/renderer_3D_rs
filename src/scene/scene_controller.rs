use imgui::TreeNodeFlags;

use super::scene::Scene;
use crate::{
    scene::scene::{CustomShaders, ShaderMaterialMapping},
    transform::Transform,
};
use std::path::Path;

pub trait SceneController {
    fn scene(&self) -> &Scene;
    fn update(&mut self);
    fn draw_ui(&mut self, ui: &imgui::Ui);
    fn new() -> Self;
}

pub struct TestSceneController {
    scene: Scene,
    rotate_objects: bool,
    time_on_start: std::time::Instant,
}

impl SceneController for TestSceneController {
    fn scene(&self) -> &Scene {
        &self.scene
    }

    fn new() -> Self {
        let mut scene = Scene::new(Some(CustomShaders {
            shader_file_paths: &[Path::new("assets/shaders/monkey.glsl").to_path_buf()],
            mapping: &[ShaderMaterialMapping::new("monkey", &["monkey_mat"])],
        }));

        scene.load_data_from_file(Path::new("assets/scenes/scene.glb"));

        Self {
            scene: scene,
            rotate_objects: false,
            time_on_start: std::time::Instant::now(),
        }
    }

    fn update(&mut self) {
        let scene = &mut self.scene;

        if self.rotate_objects {
            for i in 0..scene.root_nodes().len() {
                let node = scene.get_node_mut(scene.root_nodes()[i]);

                if node.transform.position().y < 0.25f32 {
                    continue;
                }

                let time = std::time::Instant::now()
                    .duration_since(self.time_on_start)
                    .as_secs_f32();

                let (s, _, t) = node.transform.model_matrix().to_scale_rotation_translation();

                let rotation = glam::Quat::from_axis_angle(Transform::UP, time);

                node.transform =
                    Transform::from_model_matrix(glam::Mat4::from_scale_rotation_translation(s, rotation, t));
            }
        }
    }

    fn draw_ui(&mut self, ui: &imgui::Ui) {
        if ui.collapsing_header("Scene", TreeNodeFlags::DEFAULT_OPEN) {
            ui.checkbox("Rotate objects", &mut self.rotate_objects);
        }
    }
}
