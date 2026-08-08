#ifndef GLOBAL_DATA_BUFFER_INCLUDED
#define GLOBAL_DATA_BUFFER_INCLUDED
layout(std430, binding = 0) buffer shared_data_buffer {
    mat4 camera_vp_matrix;
    mat4 camera_view_matrix;
    mat4 camera_projection_matrix;
    vec4 camera_position;
    vec4 camera_forward;
} shared_data;

struct Light {
    vec4 position;    // xyz = world space position, w = LightType
    vec4 direction;   // xyz = world space direction, w is ignored
    vec4 color;       // rgb = colro, w = intensity
    vec4 attenuation; // x = constant, y = linear, z = quadratic, w = range

    vec4 spot_light_data;
};

layout(std430, binding = 1) buffer light_data_buffer {
    Light lights[];
} light_data;

#define CORE_CAMERA_VP_MATRIX (shared_data.camera_vp_matrix)
#define CORE_CAMERA_VIEW_MATRIX (shared_data.camera_view_matrix)
#define CORE_CAMERA_PROJECTION_MATRIX (shared_data.camera_projection_matrix)

#endif