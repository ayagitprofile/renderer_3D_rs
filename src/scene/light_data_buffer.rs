use crate::graphics::buffer::GraphicsBuffer;

const BUFFER_HEADER_SIZE: usize = size_of::<[u32; 4]>();
const LIGHT_DATA_SIZE: usize = size_of::<super::light::GPULightData>();

pub struct LightDataBuffer {
    data: Vec<u8>,
    gpu_buffer: GraphicsBuffer,
}

impl LightDataBuffer {
    pub fn new(data: &[super::light::GPULightData]) -> Self {
        let header = [data.len() as u32, 0, 0, 0];

        let buffer_byte_size = BUFFER_HEADER_SIZE + LIGHT_DATA_SIZE * data.len();

        let mut buffer = vec![0u8; buffer_byte_size];

        let header_slice = unsafe { std::slice::from_raw_parts(header.as_ptr() as *const u8, BUFFER_HEADER_SIZE) };

        buffer[..BUFFER_HEADER_SIZE].copy_from_slice(header_slice);

        let src = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * LIGHT_DATA_SIZE) };

        buffer[BUFFER_HEADER_SIZE..].copy_from_slice(src);

        let mut gpu_buffer = GraphicsBuffer::new();

        gpu_buffer.allocate(buffer.as_slice(), crate::graphics::buffer::Usage::Dynamic);

        gpu_buffer.set_binding(
            crate::graphics::buffer::BindingTarget::ShaderStorageBuffer,
            super::buffers::SHADER_LIGHT_DATA_BUFFER_BINDING_INDEX,
        );

        Self {
            gpu_buffer: gpu_buffer,
            data: buffer,
        }
    }

    pub fn clear_cpu_side_buffer(&mut self) {
        self.data = Vec::new();
    }
}
