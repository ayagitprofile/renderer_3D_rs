use std::{collections::HashMap, ffi::CString};

use super::material_properties as mat_props;
use crate::{
    gl,
    graphics::{
        shader::Shader,
        texture::{self, Cubemap, StorageFormat},
    },
};

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

/// Use for debugging
pub fn try_set_bindless_texture(shader: &Shader, texture_name: &str, handle: u64) {
    if let Some(location) = shader.find_uniform_location(texture_name) {
        shader.map_bindless_texture(location, handle);
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

#[repr(u32)]
#[derive(Hash, Eq, PartialEq)]
pub enum CubemapSide {
    Right = 0,
    Left = 1,
    Top = 2,
    Bottom = 3,
    Front = 4,
    Back = 5,
}

impl CubemapSide {
    pub fn from_u32(val: u32) -> Option<Self> {
        match val {
            0 => Some(CubemapSide::Right),
            1 => Some(CubemapSide::Left),
            2 => Some(CubemapSide::Top),
            3 => Some(CubemapSide::Bottom),
            4 => Some(CubemapSide::Front),
            5 => Some(CubemapSide::Back),
            _ => None,
        }
    }

    #[rustfmt::skip]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "skybox_right"  | "right"  | "cubemap_right"  | "+x" => Some(Self::Right),
            "skybox_left"   | "left"   | "cubemap_left"   | "-x" => Some(Self::Left),
            "skybox_top"    | "top"    | "cubemap_top"    | "+y" => Some(Self::Top),
            "skybox_bottom" | "bottom" | "cubemap_bottom" | "-y" => Some(Self::Bottom),
            "skybox_front"  | "front"  | "cubemap_front"  | "+z" => Some(Self::Front),
            "skybox_back"   | "back"   | "cubemap_back"   | "-z" => Some(Self::Back),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct TextureData {
    pub data: *const u8,
    pub width: i32,
    pub height: i32,
    pub channels: i32,
}

impl TextureData {
    const fn zero() -> Self {
        Self {
            data: std::ptr::null() as *const u8,
            width: 0,
            height: 0,
            channels: 0,
        }
    }
    fn texture_meta_data_cmp(&self, other: &Self) -> bool {
        self.width == other.width && self.height == other.height && self.channels == other.channels
    }
}

pub fn load_cubemap_from_sub_textures(directory_path: &std::path::Path, storage_format: StorageFormat) -> Cubemap {
    let _timer = crate::timer::ScopedTimer::start(format!("Loading cubemap: {}", directory_path.display()).as_str());

    assert!(directory_path.exists() && directory_path.is_dir());

    let mut sides = HashMap::with_capacity(6);

    for entry in std::fs::read_dir(directory_path).unwrap() {
        let file_path = entry.unwrap().path();

        if !file_path.is_file() {
            continue;
        }

        if let Some(side) = CubemapSide::from_str(file_path.file_stem().unwrap().to_str().unwrap()) {
            sides.insert(side, file_path.clone());
        }
    }

    assert!(sides.len() == 6, "Not enough cubemap textures");

    let mut side_data = HashMap::new();

    for (side, path) in sides.iter() {
        let cstr = CString::new(path.to_str().unwrap()).unwrap();

        unsafe {
            let mut width = 0;
            let mut height = 0;
            let mut channels = 0;

            let data = stb_image::stb_image::stbi_load(
                cstr.as_ptr(),
                &mut width as *mut i32,
                &mut height as *mut i32,
                &mut channels as *mut i32,
                0,
            );

            assert!(!data.is_null());

            side_data.insert(
                side,
                TextureData {
                    data: data,
                    width: width,
                    height: height,
                    channels: channels,
                },
            );
        }
    }

    assert!(
        side_data.iter().is_sorted_by(|a, b| { a.1.texture_meta_data_cmp(b.1) }),
        "Cubemap textures have different dimensions"
    );

    let left = side_data[&CubemapSide::Left];
    let right = side_data[&CubemapSide::Right];
    let top = side_data[&CubemapSide::Top];
    let bottom = side_data[&CubemapSide::Bottom];
    let front = side_data[&CubemapSide::Front];
    let back = side_data[&CubemapSide::Back];

    let (width, height, channels) = (left.width as u32, left.height as u32, left.channels as u32);

    let cubemap = texture::Cubemap::load_from_memory(
        width,
        height,
        channels,
        storage_format,
        texture::FilteringMode::Trilinear,
        left.data,
        right.data,
        top.data,
        bottom.data,
        front.data,
        back.data,
    );

    for data in side_data {
        unsafe {
            stb_image::stb_image::stbi_image_free(data.1.data as *mut std::ffi::c_void);
        }
    }

    cubemap
}
