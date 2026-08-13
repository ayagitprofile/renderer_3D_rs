#ifndef SHADOW_MAPPING_DATA_INCLUDED
#define SHADOW_MAPPING_DATA_INCLUDED
layout(std140, binding = 0) uniform shadow_uniforms {
    mat4 ws_to_ls_matrix;
} shadow_mapping_data;
#define CORE_WS_TO_LIGHT_SPACE_MATRIX (shadow_mapping_data.ws_to_ls_matrix)
#endif//SHADOW_MAPPING_DATA_INCLUDED