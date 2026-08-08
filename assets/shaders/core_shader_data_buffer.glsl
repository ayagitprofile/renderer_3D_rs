#ifndef GLOBAL_DATA_BUFFER_INCLUDED
#define GLOBAL_DATA_BUFFER_INCLUDED
layout(std430, binding = 0) buffer shared_data_buffer {
    mat4 camera_vp_matrix;
    mat4 camera_view_matrix;
    mat4 camera_projection_matrix;
    vec4 camera_position;
    vec4 camera_forward;
} shared_data;

#define CORE_CAMERA_VP_MATRIX (shared_data.camera_vp_matrix)
#define CORE_CAMERA_VIEW_MATRIX (shared_data.camera_view_matrix)
#define CORE_CAMERA_PROJECTION_MATRIX (shared_data.camera_projection_matrix)

#endif