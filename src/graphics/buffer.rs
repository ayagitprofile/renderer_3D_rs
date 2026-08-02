use crate::gl;

#[repr(u32)]
pub enum BindingTarget {
    ShaderStorageBuffer = gl::SHADER_STORAGE_BUFFER,
    UniformBuffer = gl::UNIFORM_BUFFER,
    AtomicCounterBuffer = gl::ATOMIC_COUNTER_BUFFER,
    TransformFeedbackBuffer = gl::TRANSFORM_FEEDBACK_BUFFER,
}

pub enum Usage {
    Static,
    Dynamic,
}

impl Usage {
    fn to_gl_usage(&self) -> u32 {
        match self {
            Usage::Static => gl::STATIC_DRAW,
            Usage::Dynamic => gl::DYNAMIC_DRAW,
        }
    }
}

pub struct GraphicsBuffer {
    id: u32,
    byte_size: usize,
}

impl GraphicsBuffer {
    pub fn set_binding(&self, binding_target: BindingTarget, index: u32) {
        unsafe {
            gl::BindBufferBase(binding_target as u32, index, self.id);
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn byte_size(&self) -> usize {
        self.byte_size
    }

    pub fn allocate<T>(&mut self, slice: &[T], usage: Usage)
    {
        self.byte_size = std::mem::size_of_val(slice);
        unsafe {
            gl::NamedBufferData(
                self.id,
                self.byte_size as isize,
                slice.as_ptr() as *const std::ffi::c_void,
                usage.to_gl_usage(),
            );
        }
    }

    pub fn upload_data<T>(&self, slice: &[T]) {
        let data_size = std::mem::size_of_val(slice);
        debug_assert!(
            data_size <= self.byte_size,
            "GraphicsBuffer.upload_data expects input data size to be <= than existing buffer size"
        );

        unsafe {
            gl::NamedBufferSubData(
                self.id,
                0,
                data_size as isize,
                slice.as_ptr() as *const std::ffi::c_void,
            );
        }
    }

    pub fn new() -> Self {
        let mut id = 0;
        unsafe {
            gl::CreateBuffers(1, &mut id);
        }
        Self {
            id: id,
            byte_size: 0,
        }
    }
}

impl Drop for GraphicsBuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &mut self.id);
        }
    }
}
