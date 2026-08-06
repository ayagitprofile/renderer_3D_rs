use crate::gl;
use stb_image::stb_image;

#[derive(PartialEq)]
pub enum FilteringMode {
    Nearest,
    Bilinear,
    Trilinear,
    AnisotropicX16,
}

pub enum WrappingMode {
    Repeat,
    Clamp,
}

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum StorageFormat {
    R = gl::R8,
    RG = gl::RG8,
    RGB = gl::RGB8,
    RGBA = gl::RGBA8,

    SRGB = gl::SRGB8,
    SRGBA = gl::SRGB8_ALPHA8,

    R16F = gl::R16F,
    RG16F = gl::RG16F,
    RGB16F = gl::RGB16F,
    RGBA16F = gl::RGBA16F,

    Depth16F = gl::DEPTH_COMPONENT16,
    Depth24F = gl::DEPTH_COMPONENT24,
    Depth32F = gl::DEPTH_COMPONENT32,

    Depth24FStencil = gl::DEPTH24_STENCIL8,
    Depth32FStencil = gl::DEPTH32F_STENCIL8,
}

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum PixelDataType {
    U8 = gl::UNSIGNED_BYTE,
    U16 = gl::UNSIGNED_SHORT,
    U32 = gl::UNSIGNED_INT,
}

impl PixelDataType {
    pub fn to_gl_format(&self) -> u32 {
        *self as u32
    }
}

impl StorageFormat {
    pub fn to_gl_format(&self) -> u32 {
        *self as u32
    }
}

pub trait Texture {
    fn id(&self) -> u32;

    fn bindless_handle(&self) -> u64;
}

pub struct Texture2D {
    id: u32,
    width: i32,
    height: i32,
    channels: i32,
    storage_format: StorageFormat,
    bindless_handle: u64,
}

impl Texture for Texture2D {
    fn id(&self) -> u32 {
        self.id
    }

    fn bindless_handle(&self) -> u64 {
        self.bindless_handle
    }
}

fn mip_level_count(width: u32, height: u32) -> u32 {
    assert!(width > 0 && height > 0);

    width.max(height).ilog2() + 1
}

fn release_texture<T>(texture: &T)
where
    T: Texture,
{
    let id = texture.id();

    if id == 0 {
        return;
    }

    unsafe {
        let handle = texture.bindless_handle();
        if handle != 0 {
            gl::MakeTextureHandleNonResidentARB(handle);
        }
        gl::DeleteTextures(1, std::ptr::addr_of!(id));
    }
}

fn calulate_mip_count(width: i32, height: i32) -> i32 {
    let max_dimension = width.max(height);
    return 32 as i32 - max_dimension.leading_zeros() as i32;
}

fn get_data_format(channels: i32) -> u32 {
    match channels {
        1 => gl::R8,
        2 => gl::RG8,
        3 => gl::RGB8,
        4 => gl::RGBA8,
        _ => panic!("Incorrect number of channels{}", channels),
    }
}

fn get_data_format_srgb(channels: i32) -> u32 {
    match channels {
        1 => gl::R8,
        2 => gl::RG8,
        3 => gl::SRGB8,
        4 => gl::SRGB8_ALPHA8,
        _ => panic!("Incorrect number of channels{}", channels),
    }
}

fn get_input_data_format(channels: i32) -> u32 {
    match channels {
        1 => gl::RED,
        2 => gl::RG,
        3 => gl::RGB,
        4 => gl::RGBA,
        _ => panic!("Incorrect number of channels{}", channels),
    }
}

impl Texture2D {
    pub fn storage_format(&self) -> StorageFormat {
        self.storage_format
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn bind_to_unit(&self, unit_index: u32) {
        unsafe {
            gl::BindTextureUnit(unit_index, self.id);
        }
    }

    pub fn create_texture_from_memory(
        width: i32,
        height: i32,
        storage_format: StorageFormat,
        filtering: FilteringMode,
        channels: i32,
        data: *const std::ffi::c_void,
    ) -> Texture2D {
        let mut id = 0;

        let data_format = match channels {
            1 => gl::RED,
            2 => gl::RG,
            3 => gl::RGB,
            4 => gl::RGBA,
            _ => panic!(""),
        };

        unsafe {
            gl::CreateTextures(gl::TEXTURE_2D, 1, &mut id);
            gl::TextureStorage2D(
                id,
                mip_level_count(width as u32, height as u32) as i32,
                storage_format.to_gl_format(),
                width,
                height,
            );
            gl::TextureSubImage2D(id, 0, 0, 0, width, height, data_format, gl::UNSIGNED_BYTE, data);
            gl::GenerateTextureMipmap(id);
        }

        let mut texture = Texture2D {
            id: id,
            width: width,
            height: height,
            channels: channels,
            storage_format: storage_format,
            bindless_handle: 0,
        };

        texture.set_filtering_mode(filtering);
        texture.set_wrapping_mode(WrappingMode::Repeat);

        unsafe {
            texture.bindless_handle = gl::GetTextureHandleARB(id);
        }

        texture.make_resident();

        texture
    }

    pub fn create_texture(
        width: i32,
        height: i32,
        storage_format: StorageFormat,
        filtering: FilteringMode,
        wrapping: WrappingMode,
        generate_mip_maps: bool,
    ) -> Texture2D {
        let mut texture = Texture2D::create_empty_texture(storage_format);

        texture.width = width;
        texture.height = height;

        unsafe {
            gl::TextureStorage2D(
                texture.id,
                mip_level_count(width as u32, height as u32) as i32,
                storage_format.to_gl_format(),
                width,
                height,
            );
        }

        if generate_mip_maps {
            texture.regenerate_mip_maps();
        }

        texture.set_wrapping_mode(wrapping);
        texture.set_filtering_mode(filtering);

        unsafe {
            texture.bindless_handle = gl::GetTextureHandleARB(texture.id);
        }

        texture.make_resident();

        texture
    }

    pub fn create_single_color_texture(
        width: i32,
        height: i32,
        storage_format: StorageFormat,
        color: &[f32; 4],
        filtering: FilteringMode,
    ) -> Texture2D {
        let mut texture = Texture2D::create_empty_texture(storage_format);

        texture.width = width;
        texture.height = height;

        unsafe {
            gl::TextureStorage2D(
                texture.id,
                mip_level_count(width as u32, height as u32) as i32,
                storage_format.to_gl_format(),
                width,
                height,
            );

            gl::ClearTexImage(
                texture.id,
                0,
                gl::RGBA,
                gl::FLOAT,
                color.as_ptr() as *const std::ffi::c_void,
            );
        }

        texture.regenerate_mip_maps();

        texture.set_wrapping_mode(WrappingMode::Repeat);
        texture.set_filtering_mode(filtering);

        unsafe {
            texture.bindless_handle = gl::GetTextureHandleARB(texture.id);
        }

        texture.make_resident();

        texture
    }

    pub fn regenerate_mip_maps(&self) {
        unsafe {
            gl::GenerateTextureMipmap(self.id);
        }
    }

    pub fn load_from_file(path: &std::path::Path, storage_format: StorageFormat, filtering: FilteringMode) -> Self {
        let path_c_str = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).expect("Incorrect path");
        let str_ptr: *const i8 = path_c_str.as_ptr();

        let mut texture = Texture2D::create_empty_texture(storage_format);

        unsafe {
            stb_image::stbi_set_flip_vertically_on_load(1);

            let data = stb_image::stbi_load(
                str_ptr,
                &mut texture.width,
                &mut texture.height,
                &mut texture.channels,
                0,
            );

            if data == std::ptr::null_mut() {
                panic!("[Texture] Failed to load image: {}", path.to_string_lossy());
            }

            let id = texture.id;

            gl::TextureStorage2D(
                id,
                calulate_mip_count(texture.width, texture.height),
                storage_format.to_gl_format(),
                texture.width,
                texture.height,
            );

            gl::TextureSubImage2D(
                id,
                0,
                0,
                0,
                texture.width,
                texture.height,
                get_input_data_format(texture.channels),
                gl::UNSIGNED_BYTE,
                data as *const std::ffi::c_void,
            );

            gl::GenerateTextureMipmap(id);

            texture.set_wrapping_mode(WrappingMode::Repeat);
            texture.set_filtering_mode(filtering);

            stb_image::stbi_image_free(data as *mut std::ffi::c_void);

            texture.bindless_handle = gl::GetTextureHandleARB(id);
            texture.make_resident();
        }

        return texture;
    }

    fn create_empty_texture(storage_format: StorageFormat) -> Texture2D {
        let mut id = 0;

        unsafe {
            gl::CreateTextures(gl::TEXTURE_2D, 1, &mut id);
        }

        let texture = Texture2D {
            id: id,
            width: 0,
            height: 0,
            channels: 0,
            storage_format: storage_format,
            bindless_handle: 0,
        };

        return texture;
    }

    pub fn set_filtering_mode(&self, filtering: FilteringMode) {
        debug_assert!(
            self.bindless_handle == 0,
            "Cant call functions that modify texture's sampler if the bindless handle for this texture has been generated, because the sampler becomes immutable, if you want to modify the sampler of a bindless texture, use sampler object"
        );

        const GL_TEXTURE_MAX_ANISOTROPY_EXT: u32 = 0x84FE;

        let (min, mag) = match filtering {
            FilteringMode::Nearest => (gl::NEAREST, gl::NEAREST),
            FilteringMode::Bilinear => (gl::LINEAR, gl::LINEAR),
            FilteringMode::Trilinear | FilteringMode::AnisotropicX16 => (gl::LINEAR_MIPMAP_LINEAR, gl::LINEAR),
        };

        let id = self.id();

        unsafe {
            gl::TextureParameteri(id, gl::TEXTURE_MIN_FILTER, min as i32);
            gl::TextureParameteri(id, gl::TEXTURE_MAG_FILTER, mag as i32);

            if filtering == FilteringMode::AnisotropicX16 {
                gl::TextureParameterf(id, GL_TEXTURE_MAX_ANISOTROPY_EXT, 16f32);
            }
        }
    }

    pub fn set_wrapping_mode(&self, wrapping: WrappingMode) {
        debug_assert!(
            self.bindless_handle == 0,
            "Cant call functions that modify texture's sampler if the bindless handle for this texture has been generated, because the sampler becomes immutable, if you want to modify the sampler of a bindless texture, use sampler object"
        );

        let mode = match wrapping {
            WrappingMode::Clamp => gl::CLAMP_TO_EDGE,
            WrappingMode::Repeat => gl::REPEAT,
        };

        unsafe {
            gl::TextureParameteri(self.id(), gl::TEXTURE_WRAP_S, mode as i32);
            gl::TextureParameteri(self.id(), gl::TEXTURE_WRAP_T, mode as i32);
        }
    }

    fn make_resident(&self) {
        debug_assert!(self.id() != 0 && self.bindless_handle() != 0);

        unsafe {
            gl::MakeTextureHandleResidentARB(self.bindless_handle());
        }
    }

    fn make_non_resident(&self) {
        debug_assert!(self.id() != 0 && self.bindless_handle() != 0);

        unsafe {
            gl::MakeTextureHandleNonResidentARB(self.bindless_handle());
        }
    }
}

impl Drop for Texture2D {
    fn drop(&mut self) {
        unsafe {
            if self.bindless_handle != 0 {
                gl::MakeTextureHandleNonResidentARB(self.bindless_handle);
            }

            gl::DeleteTextures(1, &self.id);
        }
    }
}
