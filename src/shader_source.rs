#![allow(dead_code)]

use std::{
    collections::HashMap,
    ffi::CStr,
    io::Read,
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

use crate::{
    graphics::{
        material_properties::{CullMode, DepthTestMode, MaterialProperties},
        shader::{ComputeShader, Shader},
    },
    timer::ScopedTimer,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderParsingTarget {
    None,
    Vertex,
    Fragment,
    Compute,
}

static INCLUDE_FILE_CACHE: LazyLock<Mutex<HashMap<PathBuf, String>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub struct ShaderSource {
    vertex_source: String,
    fragment_source: String,
    compute_source: String,
    material_properties: MaterialProperties,
    name: String,
}

impl ShaderSource {
    pub fn compile(&self) -> Shader {
        let vert_cstr = CStr::from_bytes_with_nul(self.vertex_source.as_bytes()).unwrap();
        let frag_cstr = CStr::from_bytes_with_nul(self.fragment_source.as_bytes()).unwrap();

        Shader::compile_from_c_strings(vert_cstr, frag_cstr)
    }

    pub fn compile_compute(&self) -> ComputeShader {
        let compute_cstr = CStr::from_bytes_with_nul(self.compute_source.as_bytes()).unwrap();

        ComputeShader::compile_compute_from_c_string(compute_cstr)
    }

    pub fn load_and_compile(file_path: &std::path::Path) -> Shader {
        let _timer = ScopedTimer::start(
            format!(
                "Loading shader: {}",
                file_path.file_stem().unwrap_or_default().to_str().unwrap_or_default()
            )
            .as_str(),
        );

        let source = ShaderSource::load_from_file(file_path);

        let vert_cstr = CStr::from_bytes_with_nul(source.vertex_source.as_bytes()).unwrap();
        let frag_cstr = CStr::from_bytes_with_nul(source.fragment_source.as_bytes()).unwrap();

        let shader = Shader::compile_from_c_strings(vert_cstr, frag_cstr);

        if source.material_properties != MaterialProperties::DEFAULT {
            println!(
                "[ShaderSource] Compiling shader source directly into shader, ignored material properties:\n{:?}\n",
                source.material_properties
            );
        }

        shader
    }

    pub fn load_from_file(file_path: &std::path::Path) -> ShaderSource {
        let mut file = std::fs::File::open(file_path).expect("Failed to open shader source file");

        let mut file_string = String::new();
        let file_size = file.read_to_string(&mut file_string).unwrap();

        let mut vertex_source = String::with_capacity(file_size / 4);
        let mut fragment_source = String::with_capacity(file_size / 2);
        let mut compute_source = String::new();

        vertex_source.push_str(ShaderSource::SHADER_VERSION_DIRECTIVE_KV);
        vertex_source.push_str(ShaderSource::VERTEX_SHADER_TYPE_MACRO);
        vertex_source.push_str(ShaderSource::ENABLE_BINDLESS_TEXTURES_DIRECTIVE);

        fragment_source.push_str(ShaderSource::SHADER_VERSION_DIRECTIVE_KV);
        fragment_source.push_str(ShaderSource::FRAGMENT_SHADER_TYPE_MACRO);
        fragment_source.push_str(ShaderSource::ENABLE_BINDLESS_TEXTURES_DIRECTIVE);

        compute_source.push_str(ShaderSource::SHADER_VERSION_DIRECTIVE_KV);
        compute_source.push_str(ShaderSource::COMPUTE_SHADER_TYPE_MACRO);
        compute_source.push_str(ShaderSource::ENABLE_BINDLESS_TEXTURES_DIRECTIVE);

        let name = file_path.file_stem().unwrap_or_default().to_string_lossy().to_string();

        let mut current_parsing_target = ShaderParsingTarget::None;

        let mut mat_props = MaterialProperties::DEFAULT;

        let mut curly_counter = 0;

        for line in file_string.lines() {
            if line.contains('{') {
                curly_counter += 1;
            } else if line.contains('}') {
                curly_counter -= 1;
            }

            if line.contains(ShaderSource::SHADER_VERSION_DIRECTIVE) {
                continue;
            }

            if line.contains(ShaderSource::SHADER_TYPE_DIRECTIVE) {
                current_parsing_target = ShaderSource::extract_shader_type(line);
                continue;
            }

            if line.contains(ShaderSource::SHADER_INCLUDE_DIRECTIVE)
                && current_parsing_target != ShaderParsingTarget::None
            {
                let include_path_str = Self::extract_directive_value(Self::SHADER_INCLUDE_DIRECTIVE, line)
                    .unwrap()
                    .trim_matches('"');

                let root_path = file_path.parent().unwrap();
                let include_path = root_path.join(include_path_str);

                let include_string = {
                    let cache = INCLUDE_FILE_CACHE.lock().unwrap();

                    if let Some(value) = cache.get(&include_path) {
                        Some(value.clone())
                    } else {
                        drop(cache);

                        if let Some(value) = ShaderSource::load_include(&include_path) {
                            let mut cache = INCLUDE_FILE_CACHE.lock().unwrap();
                            cache.insert(include_path.clone(), value.clone());
                            Some(value)
                        } else {
                            None
                        }
                    }
                };

                if let Some(include_string) = include_string {
                    match current_parsing_target {
                        ShaderParsingTarget::Fragment => {
                            fragment_source.push_str(&include_string);
                            fragment_source.push('\n');
                        }
                        ShaderParsingTarget::Vertex => {
                            vertex_source.push_str(&include_string);
                            vertex_source.push('\n');
                        }
                        ShaderParsingTarget::Compute => {
                            compute_source.push_str(&include_string);
                            compute_source.push('\n');
                        }
                        ShaderParsingTarget::None => {}
                    }
                }

                continue;
            }

            if curly_counter == 0 {
                if line.contains(ShaderSource::SHADER_Z_WRITE_COMMAND) {
                    mat_props.depth_writing_enabled = ShaderSource::parse_z_write_command(line);
                    continue;
                }

                if line.contains(ShaderSource::SHADER_Z_TEST_COMMAND) {
                    mat_props.depth_test_mode = ShaderSource::parse_z_test_command(line);
                    continue;
                }

                if line.contains(ShaderSource::SHADER_CULL_COMMAND) {
                    mat_props.cull_mode = ShaderSource::parse_cull_command(line);
                    continue;
                }
            }

            match current_parsing_target {
                ShaderParsingTarget::None => continue,
                ShaderParsingTarget::Fragment => {
                    fragment_source.push_str(line);
                    fragment_source.push('\n');
                }
                ShaderParsingTarget::Vertex => {
                    vertex_source.push_str(line);
                    vertex_source.push('\n');
                }
                ShaderParsingTarget::Compute => {
                    compute_source.push_str(line);
                    compute_source.push('\n');
                }
            }
        }

        vertex_source.push('\0');
        fragment_source.push('\0');
        compute_source.push('\0');

        ShaderSource {
            vertex_source: vertex_source,
            fragment_source: fragment_source,
            compute_source: compute_source,
            name: name,
            material_properties: mat_props,
        }
    }

    fn load_include(include_path: &std::path::Path) -> Option<String> {
        if !include_path.exists() {
            println!(
                "[ShaderSource] Error: failed to include file, file doesn't exist: {}",
                include_path.display()
            );

            return None;
        }

        std::fs::read_to_string(include_path).ok()
    }

    fn parse_z_write_command(line: &str) -> bool {
        if let Some(command) = ShaderSource::extract_directive_value(ShaderSource::SHADER_Z_WRITE_COMMAND, line) {
            if command.eq_ignore_ascii_case("on") {
                return true;
            } else if command.eq_ignore_ascii_case("off") {
                return false;
            } else {
                println!("Unknown ZWrite command value: {}", command);
            }
        }

        MaterialProperties::DEFAULT.depth_writing_enabled
    }

    fn parse_z_test_command(line: &str) -> DepthTestMode {
        if let Some(command) = ShaderSource::extract_directive_value(ShaderSource::SHADER_Z_TEST_COMMAND, line) {
            match command.to_lowercase().as_str() {
                "lequal" | "lessequal" => return DepthTestMode::LessEqual,
                "equal" => return DepthTestMode::Equal,
                "always" => return DepthTestMode::Always,
                "less" => return DepthTestMode::Less,
                _ => {
                    println!("Unknown ZTest command value: {}", command);
                }
            }
        }

        MaterialProperties::DEFAULT.depth_test_mode
    }

    fn parse_cull_command(line: &str) -> CullMode {
        if let Some(command) = ShaderSource::extract_directive_value(ShaderSource::SHADER_CULL_COMMAND, line) {
            match command.to_lowercase().as_str() {
                "back" => return CullMode::Back,
                "front" => return CullMode::Front,
                "off" | "disabled" => return CullMode::Disabled,
                "both" => return CullMode::Both,
                _ => {
                    println!("Unknown Cull command value: {}", command);
                }
            }
        }

        MaterialProperties::DEFAULT.cull_mode
    }

    fn extract_shader_type(line: &str) -> ShaderParsingTarget {
        match ShaderSource::extract_directive_value(ShaderSource::SHADER_TYPE_DIRECTIVE, line)
            .expect("Failed to parse shader type directive")
            .to_lowercase()
            .as_str()
        {
            "vertex" | "vert" => ShaderParsingTarget::Vertex,
            "fragment" | "frag" | "pixel" => ShaderParsingTarget::Fragment,
            "compute" => ShaderParsingTarget::Compute,
            _ => {
                println!("[ShaderSource] Warning: ignored unknown shader type: {}", line);
                ShaderParsingTarget::None
            }
        }
    }

    fn extract_directive_value<'a>(directive: &'a str, input: &'a str) -> Option<&'a str> {
        input.trim().strip_prefix(directive).map(str::trim)
    }

    const SHADER_TYPE_DIRECTIVE: &str = "#shader";
    const SHADER_VERSION_DIRECTIVE: &str = "#version";

    const SHADER_INCLUDE_DIRECTIVE: &str = "#include";

    const SHADER_VERSION_DIRECTIVE_KV: &str = "#version 460 core\n";

    const VERTEX_SHADER_TYPE_MACRO: &str = "#define VERTEX_SHADER\n";
    const FRAGMENT_SHADER_TYPE_MACRO: &str = "#define FRAGMENT_SHADER\n";
    const COMPUTE_SHADER_TYPE_MACRO: &str = "#define COMPUTE_SHADER\n";

    const ENABLE_BINDLESS_TEXTURES_DIRECTIVE: &str = "#extension GL_ARB_bindless_texture : require\n";

    const SHADER_CULL_COMMAND: &str = "Cull";
    const SHADER_Z_WRITE_COMMAND: &str = "ZWrite";
    const SHADER_Z_TEST_COMMAND: &str = "ZTest";

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn mat_props(&self) -> &MaterialProperties {
        &self.material_properties
    }
}
