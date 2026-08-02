use std::collections::HashMap;

use glam::{Vec2, Vec3, vec2, vec3};
use imgui::TreeNodeFlags;
use sdl2::{event::Event, video::GLProfile};

use crate::{
    camera, camera_controller, gl, graphics, input, scene, scene_renderer::SceneRenderer,
    shader_data,
};

const VERT_SHADER_SRC: &str = "
#version 460 core
layout (location = 0) in vec3 a_pos;
layout (location = 1) in vec2 a_uv;
layout(std430, binding = 0) buffer shared_data_buffer {
    mat4 camera_vp_matrix;
    mat4 camera_view_matrix;
    mat4 camera_projection_matrix;
    vec4 camera_position;
    vec4 camera_forward;
} shared_data;
out vec2 v_uv;
void main() {
    vec4 position_os = vec4(a_pos, 1);
    gl_Position = shared_data.camera_vp_matrix * position_os;
    v_uv = a_uv;
}
";

const FRAG_SHADER_SRC: &str = "
#version 460 core
layout (location = 0) out vec4 out_color;
in vec2 v_uv;
void main() {
    out_color = vec4(v_uv, 1, 1);
}
";

impl App {
    pub fn run(&mut self) {
        let mut shader_data = shader_data::GlobalShaderData::new();

        shader_data.set_storage_buffer_binding(0);

        self.camera
            .transform
            .set_position(-self.camera.transform.forward() * 6f32);

        let mut camera_controller = camera_controller::CameraController::new();

        let mut scene = scene::Scene::new();

        scene.load_shaders(&[std::path::Path::new("assets/shaders/monkey_mat.glsl").to_path_buf()]);

        scene.load_data_from_file(std::path::Path::new("assets/scenes/scene.glb"));

        let mut scene_renderer = SceneRenderer::new(&scene);

        'main_loop: loop {
            let delta_time = self.get_delta_time().as_secs_f32();
            self.input_container.new_frame();

            for event in self.sdl_event_pump.poll_iter() {
                self.imgui_sdl_platform
                    .handle_event(&mut self.imgui_context, &event);

                match event {
                    Event::Quit { .. } => {
                        break 'main_loop;
                    }

                    Event::KeyDown { keycode, .. } => {
                        self.input_container.add_pressed_key(keycode.unwrap());
                    }

                    Event::KeyUp { keycode, .. } => {
                        self.input_container.remove_pressed_key(keycode.unwrap());
                    }

                    Event::MouseButtonDown { mouse_btn, .. } => {
                        self.input_container.add_pressed_mouse_button(mouse_btn);
                    }

                    Event::MouseButtonUp { mouse_btn, .. } => {
                        self.input_container.remove_pressed_mouse_button(mouse_btn);
                    }

                    Event::MouseMotion {
                        x, y, xrel, yrel, ..
                    } => {
                        self.input_container.set_cursor_position(x as f32, y as f32);
                        self.input_container
                            .set_mouse_delta(xrel as f32, yrel as f32);
                    }

                    Event::Window { win_event, .. } => match win_event {
                        sdl2::event::WindowEvent::Resized(..) => {
                            resize_viewport(&self.sdl_window);
                            self.camera.aspect_ratio = get_window_aspect_ratio(&self.sdl_window);
                        }
                        _ => {}
                    },

                    _ => {}
                }
            }

            let input = self.input_container.as_input();

            if input.get_key(input::Keycode::ESCAPE) {
                break 'main_loop;
            }

            camera_controller.update(&mut self.camera, delta_time, input);

            shader_data
                .set_camera_matrices(&self.camera.view_matrix(), &self.camera.projection_matrix());
            shader_data.upload_data();

            unsafe {
                gl::Clear(gl::DEPTH_BUFFER_BIT | gl::COLOR_BUFFER_BIT);
            }

            scene_renderer.render();

            // let model_matrix_location = scene_shader.find_uniform_location("u_model_matrix");

            // for root_node_ref in scene.test_get_root_nodes_slice() {
            //     let node = scene.get_node(root_node_ref);

            //     scene_shader.set_uniform_mat4(
            //         model_matrix_location,
            //         &node.transform.model_matrix().to_cols_array(),
            //     );

            //     let mesh = scene.get_mesh(&node.mesh_ref);

            //     render_mesh(mesh, &scene_shader);

            //     for child_ref in node.children() {
            //         let child = scene.get_node(child_ref);

            //         let world_space_matrix =
            //             node.transform.model_matrix() * child.transform.model_matrix();

            //         scene_shader.set_uniform_mat4(
            //             model_matrix_location,
            //             &world_space_matrix.to_cols_array(),
            //         );

            //         let child_mesh = scene.get_mesh(&child.mesh_ref);

            //         render_mesh(child_mesh, &scene_shader);
            //     }
            // }

            self.imgui_sdl_platform.prepare_frame(
                &mut self.imgui_context,
                &self.sdl_window,
                &self.sdl_event_pump,
            );

            let frame = self.imgui_context.new_frame();

            frame
                .window("Window")
                .size([120f32, 80f32], imgui::Condition::Appearing)
                .always_auto_resize(true)
                .movable(false)
                .scrollable(false)
                .scroll_bar(false)
                .title_bar(false)
                .position([0f32, 0f32], imgui::Condition::Always)
                .build(|| {
                    frame.text(format!("FPS: {}", (1f32 / delta_time) as i32));
                    if frame.collapsing_header("Camera", TreeNodeFlags::DEFAULT_OPEN) {
                        let pos = vec3_to_string(self.camera.transform.position());
                        let fwd = vec3_to_string(self.camera.transform.forward());
                        frame.text(format!("Position: {}", pos));
                        frame.text(format!("Forward: {}", fwd));
                    }
                });

            self.imgui_opengl_renderer.render(&mut self.imgui_context);

            self.sdl_window.gl_swap_window();
        }
    }

    pub fn new() -> Self {
        let sdl_context = sdl2::init().unwrap();

        let video = sdl_context.video().unwrap();

        {
            let attr = video.gl_attr();
            attr.set_context_profile(GLProfile::Core);
            attr.set_context_version(4, 6);

            attr.set_context_flags().debug().set();

            attr.set_multisample_buffers(1);
            attr.set_multisample_samples(4);
        }

        let window = video
            .window(
                "Renderer / Hold TAB to enable cursor / F1 toggle UI",
                1280,
                720,
            )
            .opengl()
            .position_centered()
            .resizable()
            .build()
            .unwrap();

        let opengl_context = window.gl_create_context().unwrap();

        gl::load_with(|name| video.gl_get_proc_address(name) as *const _);

        unsafe {
            let c = 0.3f32;
            gl::ClearColor(c, c, c, 0f32);
            let (width, height) = window.size();
            gl::Viewport(0, 0, width as i32, height as i32);
        }

        let event_pump = sdl_context.event_pump().unwrap();

        window
            .subsystem()
            .gl_set_swap_interval(sdl2::video::SwapInterval::Immediate)
            .unwrap();

        unsafe {
            gl::Enable(gl::DEBUG_OUTPUT);
            gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
            gl::DebugMessageCallback(Some(gl_debug_callback), std::ptr::null());
            gl::FrontFace(gl::CW);
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);
        }

        let mut imgui_context = imgui::Context::create();
        imgui_context.set_ini_filename(None);

        let platform = imgui_sdl2_support::SdlPlatform::new(&mut imgui_context);

        let imgui_renderer = imgui_opengl_renderer_rs::Renderer::new(&mut imgui_context, |s| {
            video.gl_get_proc_address(s) as *const _
        })
        .unwrap();

        sdl_context.mouse().set_relative_mouse_mode(true);

        let window_aspect_ratio = get_window_aspect_ratio(&window);

        Self {
            sdl_context: sdl_context,
            sdl_video_subsystem: video,
            sdl_window: window,
            sdl_opengl_context: opengl_context,
            sdl_event_pump: event_pump,
            imgui_context: imgui_context,
            imgui_sdl_platform: platform,
            imgui_opengl_renderer: imgui_renderer,
            time_last_frame: std::time::Instant::now(),
            input_container: input::InputContainer::new(),
            camera: camera::Camera::new(90f32, window_aspect_ratio, [0.1f32, 100f32]),
        }
    }

    fn get_delta_time(&mut self) -> std::time::Duration {
        let time_now = std::time::Instant::now();
        let delta = time_now.duration_since(self.time_last_frame);
        self.time_last_frame = time_now;

        delta
    }
}

pub struct App {
    // dependencies
    sdl_context: sdl2::Sdl,
    sdl_video_subsystem: sdl2::VideoSubsystem,
    sdl_window: sdl2::video::Window,
    sdl_opengl_context: sdl2::video::GLContext,
    sdl_event_pump: sdl2::EventPump,
    imgui_context: imgui::Context,
    imgui_sdl_platform: imgui_sdl2_support::SdlPlatform,
    imgui_opengl_renderer: imgui_opengl_renderer_rs::Renderer,
    // app data
    time_last_frame: std::time::Instant,
    input_container: input::InputContainer,
    camera: camera::Camera,
}

fn render_mesh(mesh: &graphics::mesh::Mesh, shader: &graphics::shader::Shader) {
    mesh.vao().bind();
    shader.bind();

    unsafe {
        gl::DrawElements(
            gl::TRIANGLES,
            mesh.index_count(),
            mesh.index_format().to_gl_format(),
            std::ptr::null(),
        );
    }
}

fn vec3_to_string(value: glam::Vec3) -> String {
    format!("({:.1}, {:.1}, {:.1})", value.x, value.y, value.z)
}

fn resize_viewport(window: &sdl2::video::Window) {
    let (width, height) = (window.size().0 as i32, window.size().1 as i32);

    unsafe {
        gl::Viewport(0, 0, width, height);
    }
}

fn get_window_aspect_ratio(window: &sdl2::video::Window) -> f32 {
    let (width, height) = (window.size().0 as f32, window.size().1 as f32);
    width / height
}

extern "system" fn gl_debug_callback(
    source: gl::types::GLenum,
    kind: gl::types::GLenum,
    id: gl::types::GLuint,
    severity: gl::types::GLenum,
    _length: gl::types::GLsizei,
    message: *const gl::types::GLchar,
    _user_param: *mut std::ffi::c_void,
) {
    if severity == gl::DEBUG_SEVERITY_NOTIFICATION {
        return;
    }
    unsafe {
        let msg = std::ffi::CStr::from_ptr(message).to_string_lossy();

        eprintln!(
            "OpenGL debug:\n  source={:#x}\n  type={:#x}\n  id={}\n  severity={:#x}\n  message={}",
            source, kind, id, severity, msg
        );
    }
}
