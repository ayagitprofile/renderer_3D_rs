use crate::gl;

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum AttribFormat {
    F32 = gl::FLOAT,
}

#[derive(Copy, Clone)]
pub struct AttribDescriptor {
    pub format: AttribFormat,
    pub count: u32,
    pub index: u32,
    pub relative_offset: u32,
}

pub struct VertexLayout {
    layout: std::vec::Vec<AttribDescriptor>,
    pub vertex_byte_size: usize,
}

pub struct Attrib {
    pub format: AttribFormat,
    pub count: u32,
}

impl VertexLayout {
    pub fn layout(&self) -> &[AttribDescriptor] {
        &self.layout.as_slice()
    }

    pub fn from_attribs(vertex_attributes: &[Attrib]) -> Self {
        let mut offset = 0;

        let layout = vertex_attributes
            .iter()
            .enumerate()
            .map(|(index, attrib)| {
                let value = AttribDescriptor {
                    format: attrib.format,
                    count: attrib.count,
                    index: index as u32,
                    relative_offset: offset,
                };

                offset += attrib.count * attrib.format.byte_size() as u32;

                value
            })
            .collect();

        Self {
            layout: layout,
            vertex_byte_size: offset as usize,
        }
    }
}

impl Attrib {
    pub const fn new(format: AttribFormat, count: u32) -> Self {
        Self { format, count }
    }

    pub const POSITION: Attrib = Attrib::new(AttribFormat::F32, 3);

    pub const NORMAL: Attrib = Attrib::new(AttribFormat::F32, 3);

    pub const UV: Attrib = Attrib::new(AttribFormat::F32, 2);
}

impl AttribFormat {
    pub const fn to_gl_format(&self) -> u32 {
        *self as u32
    }

    pub const fn byte_size(&self) -> usize {
        match self {
            AttribFormat::F32 => size_of::<f32>(),
        }
    }
}
