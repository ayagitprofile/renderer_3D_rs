#shader vertex

#include "core_shader_data_buffer.glsl"
#include "core_vertex_layout.glsl"
#include "core_uniforms.glsl"

Cull Back
ZWrite On
ZTest Less

void main() {
    vec4 position_ws = CORE_MODEL_MATRIX * vec4(CORE_ATTRIB_POSITION, 1);
    gl_Position = shared_data.camera_vp_matrix * position_ws;
}

#shader frag

void main() {

}