mod ambient_occlusion;
mod app;
mod camera;
mod camera_controller;
mod debug_gizmos;
mod gl;
mod graphics;
mod input;
mod obj_loader;
mod scene;
mod shader_data;
mod shader_source;
mod timer;
mod transform;

fn main() {
    let mut app = app::App::new();
    app.run();
}
