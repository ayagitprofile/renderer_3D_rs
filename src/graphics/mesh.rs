use crate::graphics::{buffer, vao};
use crate::{gl, graphics};

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum IndexFormat {
    U32 = gl::UNSIGNED_INT,
    U16 = gl::UNSIGNED_SHORT,
    U8 = gl::UNSIGNED_BYTE,
}

pub trait IndexFormatDataType: Sized {
    const FORMAT: IndexFormat;
}

impl IndexFormatDataType for u8 {
    const FORMAT: IndexFormat = IndexFormat::U8;
}

impl IndexFormatDataType for u16 {
    const FORMAT: IndexFormat = IndexFormat::U16;
}

impl IndexFormatDataType for u32 {
    const FORMAT: IndexFormat = IndexFormat::U32;
}

impl IndexFormat {
    pub fn byte_size(&self) -> usize {
        match self {
            IndexFormat::U32 => size_of::<u32>(),
            IndexFormat::U16 => size_of::<u16>(),
            IndexFormat::U8 => size_of::<u8>(),
        }
    }

    pub const fn to_gl_format(&self) -> u32 {
        *self as u32
    }
}

pub struct Mesh {
    vertex_buffer: buffer::GraphicsBuffer,
    index_buffer: buffer::GraphicsBuffer,
    vao: vao::VAO,
    vertex_count: i32,
    index_count: i32,
    index_format: IndexFormat,
}

impl Mesh {
    const VERTEX_BUFFER_BINDING: u32 = 0;

    pub fn new() -> Self {
        Self {
            vertex_buffer: buffer::GraphicsBuffer::new(),
            index_buffer: buffer::GraphicsBuffer::new(),
            vao: vao::VAO::new(),
            vertex_count: 0,
            index_count: 0,
            index_format: IndexFormat::U8,
        }
    }

    pub fn upload_vertex_buffer_data<T>(
        &mut self,
        slice: &[T],
        layout: &graphics::vertex::VertexLayout,
        usage: buffer::Usage,
    ) {
        self.vertex_buffer.allocate(slice, usage);
        self.vertex_count = (self.vertex_buffer.byte_size() / layout.vertex_byte_size) as i32;

        self.vao
            .set_vertex_buffer(Mesh::VERTEX_BUFFER_BINDING, &self.vertex_buffer, layout);
    }

    pub fn upload_index_buffer_data<T: IndexFormatDataType>(
        &mut self,
        slice: &[T],
        usage: buffer::Usage,
    ) {
        self.vao.set_index_buffer(&self.index_buffer);
        self.index_format = T::FORMAT;
        self.index_buffer.allocate(slice, usage);
        self.index_count = (self.index_buffer.byte_size() / self.index_format.byte_size()) as i32;
    }

    pub const fn vao(&self) -> &vao::VAO {
        &self.vao
    }

    pub const fn index_count(&self) -> i32 {
        self.index_count
    }

    pub const fn vertex_count(&self) -> i32 {
        self.vertex_count
    }

    pub const fn index_format(&self) -> IndexFormat {
        self.index_format
    }
}
