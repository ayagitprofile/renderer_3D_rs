pub struct ShaderSource {
    vertex_source: String,
    fragment_source: String,
}

impl ShaderSource {
    pub fn load_from_file(file_path: &std::path::Path) -> ShaderSource {
        let file = std::fs::File::open(file_path).expect("Failed to open a shader source file");

        

        ShaderSource {
            vertex_source: "".to_string(),
            fragment_source: "".to_string(),
        }
    }
}
