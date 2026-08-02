use std::{ffi::CStr, io::Read};

use crate::graphics::shader::Shader;

pub enum ShaderType {
    Vertex,
    Fragment,
}

pub struct ShaderSource {
    vertex_source: String,
    fragment_source: String,
}

impl ShaderSource {
    pub fn load_and_compile(file_path: &std::path::Path) -> Shader {
        let source = ShaderSource::load_from_file(file_path);

        let vert_cstr = CStr::from_bytes_with_nul(source.vertex_source.as_bytes()).unwrap();
        let frag_cstr = CStr::from_bytes_with_nul(source.fragment_source.as_bytes()).unwrap();

        Shader::compile_from_c_strings(vert_cstr, frag_cstr)
    }

    pub fn load_from_file(file_path: &std::path::Path) -> ShaderSource {
        let mut file = std::fs::File::open(file_path).expect("Failed to open shader source file");

        let mut file_string = String::new();
        let file_size = file.read_to_string(&mut file_string).unwrap();

        let mut vertex_source = String::with_capacity(file_size / 4);
        let mut fragment_source = String::with_capacity(file_size / 2);

        vertex_source.push_str(ShaderSource::SHADER_VERSION_DIRECTIVE_KV);
        vertex_source.push_str(ShaderSource::VERTEX_SHADER_TYPE_MACRO);
        vertex_source.push_str(ShaderSource::ENABLE_BINDLESS_TEXTURES_DIRECTIVE);

        fragment_source.push_str(ShaderSource::SHADER_VERSION_DIRECTIVE_KV);
        fragment_source.push_str(ShaderSource::FRAGMENT_SHADER_TYPE_MACRO);
        fragment_source.push_str(ShaderSource::ENABLE_BINDLESS_TEXTURES_DIRECTIVE);

        let mut current_shader_type = None;

        for line in file_string.lines() {
            if line.contains(ShaderSource::SHADER_VERSION_DIRECTIVE) {
                continue;
            }

            if line.contains(ShaderSource::SHADER_TYPE_DIRECTIVE) {
                current_shader_type = ShaderSource::extract_shader_type(line);
                continue;
            }

            match current_shader_type {
                None => continue,
                Some(ShaderType::Fragment) => {
                    fragment_source.push_str(line);
                    fragment_source.push('\n');
                }
                Some(ShaderType::Vertex) => {
                    vertex_source.push_str(line);
                    vertex_source.push('\n');
                }
            }
        }

        vertex_source.push('\0');
        fragment_source.push('\0');

        ShaderSource {
            vertex_source: vertex_source,
            fragment_source: fragment_source,
        }
    }

    fn extract_shader_type(line: &str) -> Option<ShaderType> {
        match ShaderSource::extract_directive_value(ShaderSource::SHADER_TYPE_DIRECTIVE, line)
            .expect("Failed to parse shader type directive")
            .to_lowercase()
            .as_str()
        {
            "vertex" | "vert" => Some(ShaderType::Vertex),
            "fragment" | "frag" | "pixel" => Some(ShaderType::Fragment),
            _ => {
                println!(
                    "[ShaderSource] Warning: ignored unknown shader type: {}",
                    line
                );
                None
            }
        }
    }

    fn extract_directive_value<'a>(directive: &'a str, input: &'a str) -> Option<&'a str> {
        input.trim().strip_prefix(directive).map(str::trim)
    }

    const SHADER_TYPE_DIRECTIVE: &str = "#shader";
    const SHADER_VERSION_DIRECTIVE: &str = "#version";
    const SHADER_VERSION_DIRECTIVE_KV: &str = "#version 460 core\n";

    const VERTEX_SHADER_TYPE_MACRO: &str = "#define VERTEX_SHADER\n";
    const FRAGMENT_SHADER_TYPE_MACRO: &str = "#define FRAGMENT_SHADER\n";

    const ENABLE_BINDLESS_TEXTURES_DIRECTIVE: &str =
        "#extension GL_ARB_bindless_texture : require\n";
}
