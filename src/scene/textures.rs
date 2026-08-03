pub const ALBEDO_TEXTURE_NAME: &str = "albedo_map";
pub const NORMAL_TEXTURE_NAME: &str = "normal_map";
/// G channel stores roughness value, B channel stores metallic value, other channels are ignored
pub const METALLIC_ROUGHNESS_TEXTURE_NAME: &str = "mettalic_roughness_map";

pub const TEXTURE_NAME_ARRAY: [&str; 3] = [
    ALBEDO_TEXTURE_NAME,
    NORMAL_TEXTURE_NAME,
    METALLIC_ROUGHNESS_TEXTURE_NAME,
];
