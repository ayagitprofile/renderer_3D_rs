use super::material_properties as mat_props;
use crate::gl;

pub fn apply_mat_props(mat_props: &mat_props::MaterialProperties) {
    set_depth_test_mode(mat_props.depth_test_mode);
    set_depth_writing(mat_props.depth_writing_enabled);
    set_cull_mode(mat_props.cull_mode);
}

pub fn set_depth_writing(value: bool) {
    unsafe {
        gl::DepthMask(value as u8);
    }
}

pub fn set_depth_test_mode(value: mat_props::DepthTestMode) {
    unsafe {
        // match value {
        //     mat_props::DepthTestMode::Always => gl::DepthFunc(gl::ALWAYS),
        //     mat_props::DepthTestMode::LessEqual => gl::DepthFunc(gl::LEQUAL),
        //     mat_props::DepthTestMode::Equal => gl::DepthFunc(gl::EQUAL),
        //     mat_props::DepthTestMode::Less => gl::DepthFunc(gl::LESS),
        // }

        gl::DepthFunc(value.to_gl_format());
    }
}

pub fn set_cull_mode(value: mat_props::CullMode) {
    unsafe {
        if value == mat_props::CullMode::Disabled {
            gl::Disable(gl::CULL_FACE);
            return;
        }

        gl::Enable(gl::CULL_FACE);

        match value {
            mat_props::CullMode::Back => gl::CullFace(gl::BACK),
            mat_props::CullMode::Front => gl::CullFace(gl::FRONT),
            mat_props::CullMode::Both => gl::CullFace(gl::FRONT_AND_BACK),
            _ => {}
        }
    }
}
