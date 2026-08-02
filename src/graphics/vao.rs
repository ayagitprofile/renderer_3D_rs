use crate::graphics::vertex;
use crate::{gl, graphics};

pub struct VAO {
    id: u32,
    vertex_size_bytes: usize,
}

impl Drop for VAO {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &mut self.id);
        }
    }
}

impl VAO {
    pub fn new() -> Self {
        let mut id = 0;

        unsafe {
            gl::CreateVertexArrays(1, &mut id);
        }

        Self {
            id: id,
            vertex_size_bytes: 0,
        }
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindVertexArray(self.id);
        }
    }

    pub const fn id(&self) -> u32 {
        self.id
    }

    pub const fn vertex_size_bytes(&self) -> usize {
        self.vertex_size_bytes
    }

    pub fn set_index_buffer(&self, index_buffer: &graphics::buffer::GraphicsBuffer) {
        unsafe {
            gl::VertexArrayElementBuffer(self.id, index_buffer.id());
        }
    }

    pub fn set_vertex_buffer(
        &mut self,
        buffer_binding_index: u32,
        vertex_buffer: &graphics::buffer::GraphicsBuffer,
        vertex_layout: &graphics::vertex::VertexLayout,
    ) {
        self.vertex_size_bytes = vertex_layout.vertex_byte_size;

        unsafe {
            gl::VertexArrayVertexBuffer(
                self.id,
                buffer_binding_index,
                vertex_buffer.id(),
                0,
                vertex_layout.vertex_byte_size as i32,
            );
        }

        for attrib_descriptor in vertex_layout.layout() {
            unsafe {
                gl::EnableVertexArrayAttrib(self.id, attrib_descriptor.index);

                match attrib_descriptor.format {
                    vertex::AttribFormat::F32 => {
                        gl::VertexArrayAttribFormat(
                            self.id,
                            attrib_descriptor.index,
                            attrib_descriptor.count as i32,
                            attrib_descriptor.format.to_gl_format(),
                            gl::FALSE,
                            attrib_descriptor.relative_offset,
                        );
                    }
                }

                gl::VertexArrayAttribBinding(
                    self.id,
                    attrib_descriptor.index,
                    buffer_binding_index,
                );
            }
        }

        debug_assert!(
            (vertex_buffer.byte_size() % self.vertex_size_bytes) == 0,
            "[graphics::VAO] Error: Incorrect vertex layout"
        );
    }
}
