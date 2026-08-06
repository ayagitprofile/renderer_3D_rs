use std::{
    path::Path,
    sync::{Mutex, OnceLock},
};

use imgui::TreeNodeFlags;
use sdl2::{event::Event, video::GLProfile};

use crate::{
    camera, camera_controller,
    fullscreen_quad::FullscreenQuad,
    gl,
    graphics::{self, texture::Texture},
    input,
    scene::{self, scene::Scene},
    shader_data,
    transform::Transform,
};

static OPENGL_ERROR: OnceLock<Mutex<Option<String>>> = OnceLock::new();

impl App {
    pub fn run(&mut self) {
        let mut shader_data = shader_data::GlobalShaderData::new();

        shader_data.set_storage_buffer_binding(0);

        self.camera
            .transform
            .set_position(-self.camera.transform.forward() * 6f32);

        let mut camera_controller = camera_controller::CameraController::new();

        let mut scene = Scene::new();

        scene.load_data_from_file(Path::new("assets/scenes/scene.glb"));

        let mut scene_renderer = scene::renderer::Renderer::new();

        let (window_size_x, window_size_y) = (self.sdl_window.size().0 as i32, self.sdl_window.size().1 as i32);

        let mut framebuffer = graphics::framebuffer::Framebuffer::new();

        let depth_texture = graphics::texture::Texture2D::create_texture(
            window_size_x,
            window_size_y,
            graphics::texture::StorageFormat::Depth24FStencil,
            graphics::texture::FilteringMode::Bilinear,
            graphics::texture::WrappingMode::Clamp,
            false,
        );

        framebuffer.set_depth_texture_render_target(depth_texture.id(), depth_texture.storage_format());

        let color_texture = graphics::texture::Texture2D::create_texture(
            window_size_x,
            window_size_y,
            graphics::texture::StorageFormat::SRGBA,
            graphics::texture::FilteringMode::Bilinear,
            graphics::texture::WrappingMode::Clamp,
            false,
        );

        framebuffer.set_color_texture_render_target(color_texture.id(), 0);

        let fs_quad = FullscreenQuad::new(std::path::Path::new("assets/shaders/post_process_shader.glsl"));

        if let Some(loc) = fs_quad
            .shader
            .find_uniform_location(scene::textures::FRAMEBUFFER_COLOR_TEXTURE)
        {
            fs_quad
                .shader
                .map_bindless_texture(loc, color_texture.bindless_handle());
        }

        if let Some(loc) = fs_quad
            .shader
            .find_uniform_location(scene::textures::FRAMEBUFFER_DEPTH_TEXTURE)
        {
            fs_quad
                .shader
                .map_bindless_texture(loc, depth_texture.bindless_handle());
        }

        'main_loop: loop {
            if let Some(error) = OPENGL_ERROR.get().and_then(|m| m.lock().unwrap().take()) {
                panic!("{}", error);
            }

            let delta_time = self.get_delta_time().as_secs_f32();
            self.input_container.new_frame();

            for event in self.sdl_event_pump.poll_iter() {
                self.imgui_sdl_platform.handle_event(&mut self.imgui_context, &event);

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

                    Event::MouseMotion { x, y, xrel, yrel, .. } => {
                        self.input_container.set_cursor_position(x as f32, y as f32);
                        self.input_container.set_mouse_delta(xrel as f32, yrel as f32);
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

            if input.get_key_down(input::Keycode::TAB) {
                self.sdl_context.mouse().set_relative_mouse_mode(false);
                camera_controller.ignore_input = true;
            } else if input.get_key_up(input::Keycode::TAB) {
                self.sdl_context.mouse().set_relative_mouse_mode(true);
                camera_controller.ignore_input = false;
            }

            camera_controller.update(&mut self.camera, delta_time, input);

            shader_data.set_camera_matrices(&self.camera.view_matrix(), &self.camera.projection_matrix());
            shader_data.upload_data();

            let rendering_stats = scene_renderer.prepare_rendering_data(&scene, &self.camera);

            framebuffer.set_active_render_targets(&[0]);
            framebuffer.clear();
            framebuffer.bind();

            scene_renderer.render_depth_prepass(&scene);

            scene_renderer.render(&scene);

            graphics::framebuffer::bind_default_framebuffer();

            fs_quad.render();

            self.imgui_sdl_platform
                .prepare_frame(&mut self.imgui_context, &self.sdl_window, &self.sdl_event_pump);

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
                    if frame.collapsing_header("Rendering", TreeNodeFlags::DEFAULT_OPEN) {
                        frame.text(format!("Object count: {}", rendering_stats.total_objects));
                        frame.text(format!("Visible objects: {}", rendering_stats.visible_objects));
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

            attr.set_multisample_buffers(0);
            attr.set_multisample_samples(0);
        }

        let window = video
            .window("Renderer / Hold TAB to enable cursor / F1 toggle UI", 1280, 720)
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

        let imgui_renderer =
            imgui_opengl_renderer_rs::Renderer::new(&mut imgui_context, |s| video.gl_get_proc_address(s) as *const _)
                .unwrap();

        sdl_context.mouse().set_relative_mouse_mode(true);

        let window_aspect_ratio = get_window_aspect_ratio(&window);
        let now = std::time::Instant::now();
        Self {
            sdl_context: sdl_context,
            sdl_video_subsystem: video,
            sdl_window: window,
            sdl_opengl_context: opengl_context,
            sdl_event_pump: event_pump,
            imgui_context: imgui_context,
            imgui_sdl_platform: platform,
            imgui_opengl_renderer: imgui_renderer,
            time_last_frame: now,
            time_on_app_startup: now,
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

#[allow(unused)]
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
    time_on_app_startup: std::time::Instant,
    input_container: input::InputContainer,
    camera: camera::Camera,
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

        if severity == gl::DEBUG_SEVERITY_HIGH {
            let storage = OPENGL_ERROR.get_or_init(|| Mutex::new(None));
            *storage.lock().unwrap() = Some(msg.to_string());
        }
    }
}
