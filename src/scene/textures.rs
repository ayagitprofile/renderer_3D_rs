pub const ALBEDO_TEXTURE_NAME: &str = "albedo_map";
pub const NORMAL_TEXTURE_NAME: &str = "normal_map";
/// G channel stores roughness value, B channel stores metallic value, other channels are ignored
pub const METALLIC_ROUGHNESS_TEXTURE_NAME: &str = "mettalic_roughness_map";

pub const FRAMEBUFFER_COLOR_TEXTURE: &str = "fb_color_texture";
pub const FRAMEBUFFER_DEPTH_TEXTURE: &str = "fb_depth_texture";
pub const FRAMEBUFFER_NORMAL_TEXTURE: &str = "fb_normal_texture";

pub const CUBEMAP_TEXTURE: &str = "cubemap_texture";

pub const AO_TEXTURE: &str = "ao_texture";

pub const TEXTURE_NAME_ARRAY: [&str; 3] = [
    ALBEDO_TEXTURE_NAME,
    NORMAL_TEXTURE_NAME,
    METALLIC_ROUGHNESS_TEXTURE_NAME,
];
