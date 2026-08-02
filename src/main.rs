mod app;
mod camera;
mod camera_controller;
mod gl;
mod graphics;
mod input;
mod scene;
mod shader_data;
mod timer;
mod transform;
mod shader_source;

fn main() {
    let mut app = app::App::new();

    app.run();
}

// fn lol() {
//     struct Vertex {
//         pos: Vec3,
//         uv: Vec2,
//     }

//     #[rustfmt::skip]
//     let vb_data: [Vertex; _] =
//     [
//         Vertex { pos: vec3(-0.5, -0.5, 0.0), uv: vec2(0.0, 0.0) },
//         Vertex { pos: vec3(0.5, -0.5, 0.0), uv: vec2(1.0, 0.0) },
//         Vertex { pos: vec3(0.5,  0.5, 0.0), uv: vec2(1.0, 1.0) },
//     ];

//     #[rustfmt::skip]
//     let ib_data: [u8; _] = [
//         0, 1, 2,
//     ];

//     let sdl_context = sdl2::init().unwrap();

//     let video = sdl_context.video().unwrap();

//     {
//         let attr = video.gl_attr();
//         attr.set_context_profile(GLProfile::Core);
//         attr.set_context_version(4, 6);

//         attr.set_context_flags().debug().set();

//         attr.set_multisample_buffers(1);
//         attr.set_multisample_samples(4);
//     }

//     let window = video
//         .window("Renderer / Hold TAB to enable cursor", 800, 600)
//         .opengl()
//         .position_centered()
//         .resizable()
//         .build()
//         .unwrap();

//     let _opengl_context = window.gl_create_context().unwrap();

//     gl::load_with(|name| video.gl_get_proc_address(name) as *const _);

//     unsafe {
//         gl::ClearColor(0f32, 0f32, 0f32, 0f32);
//         let (width, height) = window.size();
//         gl::Viewport(0, 0, width as i32, height as i32);
//     }

//     let mut event_pump = sdl_context.event_pump().unwrap();

//     window
//         .subsystem()
//         .gl_set_swap_interval(sdl2::video::SwapInterval::Immediate)
//         .unwrap();

//     unsafe {
//         gl::Enable(gl::DEBUG_OUTPUT);
//         gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
//         gl::DebugMessageCallback(Some(gl_debug_callback), std::ptr::null());
//         gl::Enable(gl::DEPTH_TEST);
//     }

//     let mut imgui_context = imgui::Context::create();
//     imgui_context.set_ini_filename(None);

//     let mut platform = SdlPlatform::new(&mut imgui_context);

//     let imgui_renderer = imgui_opengl_renderer_rs::Renderer::new(&mut imgui_context, |s| {
//         video.gl_get_proc_address(s) as *const _
//     })
//     .unwrap();

//     let mut mesh = mesh::Mesh::new();

//     let layout = vertex::layout_from_attribs(&[vertex::Attrib::POSITION, vertex::Attrib::UV]);

//     mesh.set_vertex_layout(&layout);
//     mesh.set_vertex_buffer_data_raw(
//         vb_data.as_ptr() as *const std::ffi::c_void,
//         vb_data.len() * std::mem::size_of::<Vertex>(),
//         buffer::Usage::Static,
//     );
//     mesh.set_index_buffer_data(ib_data.as_slice(), buffer::Usage::Static);

//     let mut shader = shader::Shader::compile_from_strings(VERT_SHADER_SRC, FRAG_SHADER_SRC);

//     let texture = texture::Texture2D::load_from_file(
//         std::path::Path::new("image.png"),
//         texture::StorageFormat::RGB,
//         texture::FilteringMode::AnisotropicX16,
//     );

//     let location = shader.find_uniform_location("tex");

//     shader.map_bindless_texture(location, texture.bindless_handle());

//     let mut shader_data = shader_data::GlobalShaderData::new();

//     let aspect_ratio = window.size().0 as f32 / window.size().1 as f32;

//     shader_data.bind_to(0);

//     let mut camera = Camera::new(90f32, aspect_ratio, [0.1f32, 100f32]);

//     camera.transform.set_position(-camera.transform.forward());

//     shader_data.set_camera_matrices(&camera.view_matrix(), &camera.projection_matrix());

//     let mut time_last_frame = std::time::Instant::now();

//     let mut input_container = input::InputContainer::new();

//     let mut camera_controller = camera_controller::CameraController::new();

//     sdl_context.mouse().set_relative_mouse_mode(true);

//     // Main loop
//     'main_loop: loop {
//         let time_current_frame = std::time::Instant::now();
//         let delta_time = time_current_frame
//             .duration_since(time_last_frame)
//             .as_secs_f32();

//         time_last_frame = time_current_frame;

//         input_container.new_frame();

//         for event in event_pump.poll_iter() {
//             platform.handle_event(&mut imgui_context, &event);

//             match event {
//                 Event::Quit { .. } => {
//                     break 'main_loop;
//                 }

//                 Event::KeyDown { keycode, .. } => {
//                     input_container.add_pressed_key(keycode.unwrap());
//                 }

//                 Event::KeyUp { keycode, .. } => {
//                     input_container.remove_pressed_key(keycode.unwrap());
//                 }

//                 Event::MouseButtonDown { mouse_btn, .. } => {
//                     input_container.add_pressed_mouse_button(mouse_btn);
//                 }

//                 Event::MouseButtonUp { mouse_btn, .. } => {
//                     input_container.remove_pressed_mouse_button(mouse_btn);
//                 }

//                 Event::MouseMotion {
//                     x, y, xrel, yrel, ..
//                 } => {
//                     input_container.set_cursor_position(x as f32, y as f32);
//                     input_container.set_mouse_delta(xrel as f32, yrel as f32);
//                 }

//                 Event::Window { win_event, .. } => match win_event {
//                     sdl2::event::WindowEvent::Resized(width, height) => {
//                         resize_viewport(width, height);
//                         camera.aspect_ratio = (width as f32) / (height as f32);
//                     }
//                     _ => {}
//                 },

//                 _ => {}
//             }
//         }

//         let input = input_container.as_input();

//         if input.get_key_down(input::Keycode::TAB) {
//             sdl_context.mouse().set_relative_mouse_mode(false);
//             camera_controller.ignore_input = true;
//         } else if input.get_key_up(input::Keycode::TAB) {
//             sdl_context.mouse().set_relative_mouse_mode(true);
//             camera_controller.ignore_input = false;
//         }

//         if input.get_key(input::Keycode::ESCAPE) {
//             break 'main_loop;
//         }

//         camera_controller.update(&mut camera, delta_time, input);

//         shader_data.set_camera_matrices(&camera.view_matrix(), &camera.projection_matrix());
//         shader_data.upload_data();

//         unsafe {
//             gl::Clear(gl::DEPTH_BUFFER_BIT | gl::COLOR_BUFFER_BIT);
//             gl::FrontFace(gl::CW);
//             gl::Disable(gl::CULL_FACE);
//         }

//         unsafe {
//             shader.bind();
//             mesh.vao().bind();

//             gl::DrawElements(
//                 gl::TRIANGLES,
//                 mesh.index_count(),
//                 mesh.index_format().to_gl_format(),
//                 std::ptr::null(),
//             );
//         }

//         platform.prepare_frame(&mut imgui_context, &window, &event_pump);
//         let frame = imgui_context.new_frame();

//         frame
//             .window("Window")
//             .size([120f32, 80f32], imgui::Condition::Appearing)
//             .always_auto_resize(true)
//             .movable(false)
//             .scrollable(false)
//             .scroll_bar(false)
//             .title_bar(false)
//             .position([0f32, 0f32], imgui::Condition::Always)
//             .build(|| {
//                 frame.text(format!("FPS: {}", (1f32 / delta_time) as i32));
//                 if frame.collapsing_header("Camera", TreeNodeFlags::DEFAULT_OPEN) {
//                     frame.text(vec3_to_string(camera.transform.position()));
//                 }
//             });

//         imgui_renderer.render(&mut imgui_context);

//         window.gl_swap_window();
//     }
// }

// fn vec3_to_string(value: glam::Vec3) -> String {
//     format!("({:.1}, {:.1}, {:.1})", value.x, value.y, value.z)
// }

// extern "system" fn gl_debug_callback(
//     source: gl::types::GLenum,
//     kind: gl::types::GLenum,
//     id: gl::types::GLuint,
//     severity: gl::types::GLenum,
//     _length: gl::types::GLsizei,
//     message: *const gl::types::GLchar,
//     _user_param: *mut std::ffi::c_void,
// ) {
//     if severity == gl::DEBUG_SEVERITY_NOTIFICATION {
//         return;
//     }
//     unsafe {
//         let msg = std::ffi::CStr::from_ptr(message).to_string_lossy();

//         eprintln!(
//             "OpenGL debug:\n  source={:#x}\n  type={:#x}\n  id={}\n  severity={:#x}\n  message={}",
//             source, kind, id, severity, msg
//         );
//     }
// }
