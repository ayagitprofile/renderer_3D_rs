use std::sync::{Mutex, OnceLock};

use glam::{Vec3, vec3};
use imgui::TreeNodeFlags;
use sdl2::{
    event::Event,
    keyboard::Keycode,
    video::{FullscreenType, GLProfile},
};

use crate::{
    camera, camera_controller,
    debug_gizmos::DebugGizmoRenderer,
    gl,
    graphics::{self},
    input,
    scene::{self, light::LightType, scene_controller::SceneController, value_range},
    shader_data, timer,
    transform::Transform,
};

static OPENGL_ERROR: OnceLock<Mutex<Option<String>>> = OnceLock::new();

impl App {
    pub fn run(&mut self) {
        let load_timer = timer::Timer::start("");

        let mut shader_data = shader_data::GlobalShaderData::new();

        self.camera
            .transform
            .set_position(-self.camera.transform.forward() * 4f32 + Transform::UP * 3f32);

        let mut camera_controller = camera_controller::CameraController::new();

        let mut scene_controller = scene::scene_controller::TestSceneController::new();

        let (window_size_x, window_size_y) = (self.sdl_window.size().0 as i32, self.sdl_window.size().1 as i32);

        let mut scene_renderer =
            scene::renderer::Renderer::new((window_size_x as u32, window_size_y as u32), scene_controller.scene());

        let mut draw_ui = true;

        let mut draw_gizmos = true;

        let gizmo_renderer = DebugGizmoRenderer::new();

        println!("Loading took: {} ms", load_timer.elapsed().as_millis());

        'main_loop: loop {
            if let Some(error) = OPENGL_ERROR.get().and_then(|m| m.lock().unwrap().take()) {
                panic!("{}", error);
            }

            let delta_time = self.get_delta_time().as_secs_f32();
            self.input_container.new_frame();

            for event in self.sdl_event_pump.poll_iter() {
                if !matches!(
                    event,
                    Event::KeyDown {
                        keycode: Some(Keycode::Tab),
                        ..
                    }
                ) {
                    self.imgui_sdl_platform.handle_event(&mut self.imgui_context, &event);
                }

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
                            self.camera
                                .projection
                                .set_aspect_ratio(get_window_aspect_ratio(&self.sdl_window));
                            scene_renderer =
                                scene::renderer::Renderer::new(self.sdl_window.size(), scene_controller.scene())
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

            if input.get_key_down(input::Keycode::SPACE) {
                let state = self.sdl_window.fullscreen_state();
                let result = self.sdl_window.set_fullscreen(if state == FullscreenType::Off {
                    FullscreenType::Desktop
                } else {
                    FullscreenType::Off
                });

                result.unwrap();
            }

            if input.get_key_down(input::Keycode::TAB) {
                self.sdl_context.mouse().set_relative_mouse_mode(false);
                camera_controller.ignore_input = true;
            } else if input.get_key_up(input::Keycode::TAB) {
                self.sdl_context.mouse().set_relative_mouse_mode(true);
                camera_controller.ignore_input = false;
            }

            if input.get_key_down(input::Keycode::F1) {
                draw_ui = !draw_ui;
            }

            camera_controller.update(&mut self.camera, delta_time, input);

            scene_controller.update();

            shader_data.set_screen_size(self.sdl_window.size().0, self.sdl_window.size().1);

            shader_data.set_camera_data(
                &self.camera.view_matrix(),
                &self.camera.projection_matrix(),
                &self.camera.transform,
                self.camera.clipping_range,
            );

            shader_data.upload_data();

            let scene = scene_controller.scene();

            let rendering_stats = scene_renderer.prepare_rendering_data(scene, &self.camera);
            graphics::utility::poll_error("Render prep");

            scene_renderer.new_frame();
            graphics::utility::poll_error("New frame");

            scene_renderer.render_depth_prepass(scene);
            graphics::utility::poll_error("Depth prepass");

            scene_renderer.ssao().compute_ambient_occlusion();
            graphics::utility::poll_error("SSAO pass");

            scene_renderer.render_forward_lighting(scene);
            graphics::utility::poll_error("Forward pass");

            scene_renderer.render_post_processing();
            graphics::utility::poll_error("Post process pass");

            if draw_gizmos {
                for light in scene_controller.scene().lights.lights() {
                    match light.type_of_light {
                        LightType::Directional => {
                            gizmo_renderer.render_directional_light(
                                Vec3::from_array(light.color),
                                Vec3::from_array(light.direction),
                            );
                        }
                        LightType::Point => {
                            gizmo_renderer.render_lightbulb(
                                Vec3::from_array(light.position),
                                vec3(0.3, 0.3, 0.3),
                                Vec3::from_array(light.color),
                            );
                        }
                        LightType::Spot => {}
                    }
                }
            }

            if draw_ui {
                self.imgui_sdl_platform
                    .prepare_frame(&mut self.imgui_context, &self.sdl_window, &self.sdl_event_pump);

                let ui = self.imgui_context.new_frame();

                ui.window("Window")
                    .size([120f32, 80f32], imgui::Condition::Appearing)
                    .always_auto_resize(true)
                    .movable(true)
                    .scrollable(false)
                    .scroll_bar(false)
                    .title_bar(false)
                    .position([0f32, 0f32], imgui::Condition::FirstUseEver)
                    .build(|| {
                        if ui.collapsing_header("Rendering stats", TreeNodeFlags::DEFAULT_OPEN) {
                            ui.text(format!(
                                "Resolution: ({} by {})",
                                self.sdl_window.size().0,
                                self.sdl_window.size().1
                            ));
                            ui.text(format!("FPS: {}", (1f32 / delta_time) as i32));
                            ui.text(format!("Frame time: {:.2} ms", delta_time * 1000f32));
                        }
                        if ui.collapsing_header("Camera", TreeNodeFlags::DEFAULT_OPEN) {
                            let pos = vec3_to_string(self.camera.transform.position());
                            let fwd = vec3_to_string(self.camera.transform.forward());
                            ui.text(format!("Position: {}", pos));
                            ui.text(format!("Forward: {}", fwd));
                        }
                        if ui.collapsing_header("Rendering", TreeNodeFlags::DEFAULT_OPEN) {
                            ui.text(format!("Object count: {}", rendering_stats.total_objects));
                            ui.text(format!("Visible objects: {}", rendering_stats.visible_objects));
                        }
                        if ui.collapsing_header("SSAO", TreeNodeFlags::DEFAULT_OPEN) {
                            let (rx, ry) = (
                                scene_renderer.ssao().ao_compute_result_texture().width(),
                                scene_renderer.ssao().ao_compute_result_texture().height(),
                            );

                            let ratio = (rx * ry) as f32 / (self.sdl_window.size().0 * self.sdl_window.size().1) as f32;

                            ui.text(format!("Resolution: ({} by {})", rx, ry));

                            ui.text(format!("Ratio to screen size: {:.2}", ratio));

                            let ssao = scene_renderer.ssao_mut();

                            let mut blur = *ssao.enable_blur_pass();
                            if ui.checkbox("Enable blur pass", &mut blur) {
                                *ssao.enable_blur_pass() = blur;
                            }
                            let mut r = ssao.radius();
                            if ui.slider("Radius", 0f32, 1f32, &mut r) {
                                ssao.set_radius(r);
                            }
                            let mut db = ssao.depth_bias();
                            if ui.slider("Depth bias", 0f32, 0.1f32, &mut db) {
                                ssao.set_depth_bias(db);
                            }
                            let mut s = ssao.strength();
                            if ui.slider("Strength", 0f32, 10f32, &mut s) {
                                ssao.set_strength(s);
                            }
                        }
                        if ui.collapsing_header("Post processing", TreeNodeFlags::DEFAULT_OPEN) {
                            let config = scene_renderer.post_processing_config_mut();
                            App::draw_value_range_ui(ui, "Chromatic abberation", &mut config.chromatic_abberation);
                            App::draw_value_range_ui(ui, "Vignette", &mut config.vignette);
                        }
                        if ui.collapsing_header("Debug", TreeNodeFlags::DEFAULT_OPEN) {
                            ui.checkbox("Draw gizmos", &mut draw_gizmos);
                        }
                        scene_controller.draw_ui(ui);
                    });

                self.imgui_opengl_renderer.render(&mut self.imgui_context);
            }

            self.sdl_window.gl_swap_window();
        }
    }

    fn draw_value_range_ui(ui: &imgui::Ui, label: &str, value_range: &mut value_range::ValueRange) {
        let mut value = value_range.value();
        if ui.slider(label, value_range.min(), value_range.max(), &mut value) {
            value_range.set_value(value);
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
            camera: camera::Camera::new_perspective_camera(90f32, window_aspect_ratio, [0.1f32, 100f32]),
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

    graphics::framebuffer::bind_default_framebuffer();

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
