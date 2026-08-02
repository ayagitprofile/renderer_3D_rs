use crate::gl;

#[repr(u32)]
#[derive(Clone, Copy, Hash)]
pub enum CullMode {
    Disabled = 0u32,
    Back = gl::BACK,
    Front = gl::FRONT,
    Both = gl::FRONT_AND_BACK,
}

#[repr(u32)]
#[derive(Clone, Copy, Hash)]
pub enum DepthTestMode {
    LessEqual = gl::LEQUAL,
    Equal = gl::EQUAL,
}

#[derive(Hash)]
pub struct MaterialProperties {
    cull_mode: CullMode,
    depth_test_mode: DepthTestMode,
    depth_writing_enabled: bool,
}

impl Default for MaterialProperties {
    fn default() -> Self {
        Self {
            cull_mode: CullMode::Back,
            depth_test_mode: DepthTestMode::LessEqual,
            depth_writing_enabled: true,
        }
    }
}

impl MaterialProperties {
    pub const fn new(cull: CullMode, depth_test: DepthTestMode, depth_writing: bool) -> Self {
        Self {
            cull_mode: cull,
            depth_test_mode: depth_test,
            depth_writing_enabled: depth_writing,
        }
    }
}

impl DepthTestMode {
    pub const fn to_gl_format(&self) -> u32 {
        *self as u32
    }
}

impl CullMode {
    pub const fn to_gl_format(&self) -> u32 {
        *self as u32
    }
}
