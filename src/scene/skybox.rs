use crate::{
    graphics::{
        buffer::Usage,
        material_properties::MaterialProperties,
        mesh::Mesh,
        shader::Shader,
        vertex::{self, Attrib},
    },
    shader_source::ShaderSource,
};

pub struct Skybox {
    pub mesh: Mesh,
    pub shader: Shader,
    pub mat_props: MaterialProperties,
}

impl Skybox {
    pub fn new() -> Self {
        let mut mesh = Mesh::new();

        mesh.upload_index_buffer_data(CUBE_INDICES.as_slice(), Usage::Static);

        mesh.upload_vertex_buffer_data(
            CUBE_VERTICES.as_slice(),
            &vertex::VertexLayout::from_attribs(&ATTRIBS),
            Usage::Static,
        );

        let shader_source = ShaderSource::load_from_file(std::path::Path::new("assets/shaders/skybox_shader.glsl"));

        let shader = shader_source.compile();

        Skybox {
            mesh: mesh,
            shader: shader,
            mat_props: *shader_source.mat_props(),
        }
    }
}

const ATTRIBS: [Attrib; 1] = [vertex::Attrib::POSITION];

#[rustfmt::skip]
const CUBE_VERTICES: [f32; 3 * 8] = [
    // Front (+Z)
    -1.0, -1.0,  1.0, // 0
     1.0, -1.0,  1.0, // 1
     1.0,  1.0,  1.0, // 2
    -1.0,  1.0,  1.0, // 3

    // Back (-Z)
    -1.0, -1.0, -1.0, // 4
     1.0, -1.0, -1.0, // 5
     1.0,  1.0, -1.0, // 6
    -1.0,  1.0, -1.0, // 7
];

#[rustfmt::skip]
const CUBE_INDICES: [u8; 6 * 6] = [
    // Front (+Z)
    0, 2, 3,
    0, 1, 2,

    // Back (-Z)
    7, 6, 4,
    6, 5, 4,

    // Left (-X)
    0, 7, 4,
    0, 3, 7,

    // Right (+X)
    1, 6, 2,
    1, 5, 6,

    // Top (+Y)
    3, 6, 7,
    3, 2, 6,

    // Bottom (-Y)
    0, 5, 1,
    0, 4, 5,
];
