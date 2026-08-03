use std::{
    ffi::{CStr, CString},
    str::FromStr,
};

use crate::gl;

struct UniformData {
    name: smol_str::SmolStr,
    location: i32,
}

type UniformDataStorage = smallvec::SmallVec<[UniformData; 4]>;

pub struct Shader {
    id: u32,
    uniform_data_storage: UniformDataStorage,
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}

impl Shader {
    pub fn map_bindless_texture(&self, texture_location: i32, bindless_texture_handle: u64) {
        unsafe {
            gl::ProgramUniformHandleui64ARB(self.id, texture_location, bindless_texture_handle);
        }
    }

    pub fn bind(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    pub fn compile_from_c_strings(vertex_source: &CStr, fragment_source: &CStr) -> Self {
        let id = compile_shader(vertex_source, fragment_source);

        Self {
            id: id,
            uniform_data_storage: Shader::load_uniform_data(id),
        }
    }

    pub fn compile_from_strings(vertex_source: &str, fragment_source: &str) -> Self {
        let v_cstr = CString::new(vertex_source.trim_end_matches('\0')).expect("String conversion failure");

        let f_cstr = CString::new(fragment_source.trim_end_matches('\0')).expect("String conversion failure");

        Shader::compile_from_c_strings(v_cstr.as_c_str(), f_cstr.as_c_str())
    }

    pub fn find_uniform_location(&self, name: &str) -> Option<i32> {
        for element in &self.uniform_data_storage {
            if element.name == name {
                return Some(element.location);
            }
        }

        None

        // panic!("{}", format!("Uniform with such name: {} not found", name));
    }

    pub fn map_texture_to_unit(&self, texture_uniform_location: i32, unit_index: i32) {
        unsafe {
            gl::ProgramUniform1i(self.id, texture_uniform_location, unit_index);
        }
    }

    pub fn set_uniform_mat4(&self, location: i32, value: &[f32; 16]) {
        unsafe {
            gl::ProgramUniformMatrix4fv(self.id, location, 1, gl::FALSE, value.as_ptr() as *const f32);
        }
    }

    fn load_uniform_data(id: u32) -> UniformDataStorage {
        let mut storage = UniformDataStorage::new();

        let uniform_count = unsafe {
            let mut count = 0;
            gl::GetProgramInterfaceiv(id, gl::UNIFORM, gl::ACTIVE_RESOURCES, &mut count);
            count as usize
        };

        storage.reserve_exact(uniform_count);

        const PROPS: [u32; 2] = [gl::NAME_LENGTH, gl::BLOCK_INDEX];

        for index in 0..uniform_count {
            let mut values = [0i32; 2];

            unsafe {
                gl::GetProgramResourceiv(
                    id,
                    gl::UNIFORM,
                    index as u32,
                    PROPS.len() as i32,
                    PROPS.as_ptr(),
                    values.len() as i32,
                    std::ptr::null_mut(),
                    values.as_mut_ptr(),
                );
            }

            let [name_len, block_index] = values;

            // Ignore uniforms belonging to UBOs
            if block_index != -1 {
                println!("[Shader] Warning: Failed to preload uniform data because UBOs are not supported");
                continue;
            }

            if name_len <= 0 {
                continue;
            }

            let mut name_buffer = vec![0u8; name_len as usize];

            let actual_len = unsafe {
                let mut len = 0;

                gl::GetProgramResourceName(
                    id,
                    gl::UNIFORM,
                    index as u32,
                    name_len,
                    &mut len,
                    name_buffer.as_mut_ptr() as *mut i8,
                );

                len as usize
            };

            let name = &name_buffer[..actual_len];

            let location = unsafe { gl::GetUniformLocation(id, name_buffer.as_ptr() as *const i8) };

            storage.push(UniformData {
                name: smol_str::SmolStr::new(String::from_utf8_lossy(name)),
                location,
            });
        }

        if storage.len() > storage.inline_size() {
            println!(
                "[Shader] Warning: uniform data inline storage overflow, {} elements are heap allocated",
                storage.len()
            );
        }

        storage
    }
}

fn compile_sub_shader(shader_type: u32, source: &CStr) -> u32 {
    use std::ffi::c_char;

    const COMPILATION_SUCCESS: i32 = gl::TRUE as i32;

    let shader;

    unsafe {
        shader = gl::CreateShader(shader_type);
        gl::ShaderSource(shader, 1, &source.as_ptr(), std::ptr::null());
        gl::CompileShader(shader);

        let mut comp_status = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut comp_status);

        if comp_status != COMPILATION_SUCCESS {
            let mut buffer: [c_char; 2048] = [0; 2048];
            let mut string_len: i32 = 0;

            gl::GetShaderInfoLog(shader, buffer.len() as i32, &mut string_len, buffer.as_mut_ptr());
            gl::DeleteShader(shader);

            let bytes = std::slice::from_raw_parts(buffer.as_ptr() as *const u8, string_len as usize);

            println!("Shader compilation failed: {}", String::from_utf8_lossy(bytes));

            println!("Source dump:\n {}", source.to_string_lossy());

            return 0;
        }
    }

    return shader;
}

fn compile_shader(vertex_source: &CStr, fragment_source: &CStr) -> u32 {
    use std::ffi::c_char;

    let vertex_sub = compile_sub_shader(gl::VERTEX_SHADER, vertex_source);
    let fragment_sub = compile_sub_shader(gl::FRAGMENT_SHADER, fragment_source);

    if vertex_sub == 0 || fragment_sub == 0 {
        return 0;
    }

    let shader;

    unsafe {
        shader = gl::CreateProgram();

        gl::AttachShader(shader, vertex_sub);
        gl::AttachShader(shader, fragment_sub);

        gl::LinkProgram(shader);

        let mut success = 0;
        gl::GetProgramiv(shader, gl::LINK_STATUS, &mut success);

        if success == gl::FALSE as i32 {
            let mut buffer: [c_char; 2048] = [0; 2048];
            let mut string_len: i32 = 0;

            gl::GetShaderInfoLog(shader, buffer.len() as i32, &mut string_len, buffer.as_mut_ptr());
            gl::DeleteProgram(shader);

            let bytes = std::slice::from_raw_parts(buffer.as_ptr() as *const u8, string_len as usize);

            println!("Shader linking failed: {}", String::from_utf8_lossy(bytes));

            return 0;
        }

        gl::DeleteShader(vertex_sub);
        gl::DeleteShader(fragment_sub);
    }

    return shader;
}
