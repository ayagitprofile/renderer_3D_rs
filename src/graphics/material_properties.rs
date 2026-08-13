use crate::gl;

#[repr(u32)]
#[derive(Clone, Copy, Hash, PartialEq, Debug)]
pub enum CullMode {
    Disabled = 0u32,
    Back = gl::BACK,
    Front = gl::FRONT,
    Both = gl::FRONT_AND_BACK,
}

#[repr(u32)]
#[derive(Clone, Copy, Hash, PartialEq, Debug)]
pub enum DepthTestMode {
    LessEqual = gl::LEQUAL,
    Equal = gl::EQUAL,
    Less = gl::LESS,
    Always = gl::ALWAYS,
}

#[derive(Clone, Copy, Hash, PartialEq, Debug)]
pub enum ShadowCasting {
    Disabled,
    Enabled,
}

#[derive(Clone, Copy, Hash, PartialEq, Debug)]
pub enum SurfaceType {
    Opaque,
    Transparent,
}

#[derive(Hash, PartialEq, Debug, Clone, Copy)]
pub struct MaterialProperties {
    pub cull_mode: CullMode,
    pub depth_test_mode: DepthTestMode,
    pub surface_type: SurfaceType,
    pub depth_writing_enabled: bool,
    pub shadow_casting: ShadowCasting,
}

impl MaterialProperties {
    pub const fn new(
        cull: CullMode,
        depth_test: DepthTestMode,
        depth_writing: bool,
        surface_type: SurfaceType,
        shadow_casting: ShadowCasting,
    ) -> Self {
        Self {
            cull_mode: cull,
            depth_test_mode: depth_test,
            depth_writing_enabled: depth_writing,
            surface_type,
            shadow_casting,
        }
    }

    pub const DEFAULT: MaterialProperties = MaterialProperties::new(
        CullMode::Back,
        DepthTestMode::LessEqual,
        true,
        SurfaceType::Opaque,
        ShadowCasting::Disabled,
    );
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
