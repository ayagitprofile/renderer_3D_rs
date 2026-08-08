#ifndef CORE_UNIFORMS_INCLUDED
#define CORE_UNIFORMS_INCLUDED

uniform mat4 u_model_matrix = mat4(1.0);
#define CORE_MODEL_MATRIX (u_model_matrix)

#ifdef FRAGMENT_SHADER

uniform sampler2D albedo_map;
uniform sampler2D normal_map;
uniform sampler2D mettalic_roughness_map;
uniform samplerCube cubemap_texture;

#define CORE_ALBEDO_MAP (albedo_map)
#define CORE_NORMAL_MAP (normal_map)
#define CORE_METALLIC_ROUGHNESS_MAP (mettalic_roughness_map)
#define CORE_CUBEMAP (cubemap_texture)

#endif //#ifdef FRAGMENT_SHADER

#endif //#ifndef CORE_UNIFORMS_INCLUDED