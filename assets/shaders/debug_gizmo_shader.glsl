#shader vertex

layout(location = 0) in vec3 a_position;

#include "core_shader_data_buffer.glsl"

uniform mat4 u_model_matrix;

Cull Off
ZWrite On
ZTest Always
Surface Opaque

void main() {
    vec4 position_ws = u_model_matrix * vec4(a_position, 1);
    gl_Position = shared_data.camera_vp_matrix * position_ws;
}

#shader frag

layout(location = 0) out vec4 out_color;

uniform vec4 u_color;

void main() {
    out_color = u_color;
}