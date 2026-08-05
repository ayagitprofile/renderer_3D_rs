#ifndef GLOBAL_DATA_BUFFER_INCLUDED
#define GLOBAL_DATA_BUFFER_INCLUDED
layout(std430, binding = 0) buffer shared_data_buffer {
    mat4 camera_vp_matrix;
    mat4 camera_view_matrix;
    mat4 camera_projection_matrix;
    vec4 camera_position;
    vec4 camera_forward;
} shared_data;
#endif