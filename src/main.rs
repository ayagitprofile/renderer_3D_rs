mod ambient_occlusion;
mod app;
mod camera;
mod camera_controller;
mod fullscreen_quad;
mod gl;
mod graphics;
mod input;
mod scene;
mod shader_data;
mod shader_source;
mod timer;
mod transform;

fn main() {
    let mut app = app::App::new();
    app.run();
}
