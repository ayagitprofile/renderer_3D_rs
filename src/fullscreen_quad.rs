use crate::{
    gl,
    graphics::{self, material_properties::MaterialProperties, mesh::Mesh, shader::Shader, vertex::VertexLayout},
    shader_source::ShaderSource,
};

#[rustfmt::skip]
const VERTEX_BUFFER_DATA: [[f32; 5]; 4] = [
    // vec3 position        vec2 uv
    [ 1.0,  1.0,  0.0,      1.0, 1.0],
    [ 1.0, -1.0,  0.0,      1.0, 0.0],
    [-1.0, -1.0,  0.0,      0.0, 0.0],
    [-1.0,  1.0,  0.0,      0.0, 1.0],
];

#[rustfmt::skip]
const INDEX_BUFFER_DATA: [u8; 6] = [
    0, 1, 2,
    0, 2, 3
];

pub const VERTEX_LAYOUT_ATTRIBS: [graphics::vertex::Attrib; 2] =
    [graphics::vertex::Attrib::POSITION, graphics::vertex::Attrib::UV];

pub struct FullscreenQuad {
    pub mesh: Mesh,
    pub shader: Shader,
    shader_mat_props: MaterialProperties,
}

impl FullscreenQuad {
    pub fn new(shader_source: &std::path::Path) -> Self {
        let mut mesh = Mesh::new();

        mesh.upload_vertex_buffer_data(
            VERTEX_BUFFER_DATA.as_slice(),
            &VertexLayout::from_attribs(&VERTEX_LAYOUT_ATTRIBS),
            graphics::buffer::Usage::Static,
        );

        mesh.upload_index_buffer_data(INDEX_BUFFER_DATA.as_slice(), graphics::buffer::Usage::Static);

        let shader_source = ShaderSource::load_from_file(shader_source);

        Self {
            mesh: mesh,
            shader: shader_source.compile(),
            shader_mat_props: *shader_source.mat_props(),
        }
    }

    pub fn render(&self) {
        graphics::utility::apply_mat_props(&self.shader_mat_props);

        self.shader.bind();
        self.mesh.vao().bind();

        unsafe {
            gl::DrawElements(
                gl::TRIANGLES,
                self.mesh.index_count(),
                self.mesh.index_format().to_gl_format(),
                std::ptr::null(),
            );
        }
    }
}
