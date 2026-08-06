#shader vertex

#include "core_vertex_layout.glsl"
#include "core_shader_data_buffer.glsl"
#include "core_uniforms.glsl"

void main() {
    vec4 position_ws = CORE_MODEL_MATRIX * vec4(CORE_ATTRIB_POSITION, 1);
    gl_Position = shared_data.camera_vp_matrix * position_ws;
}

#shader frag

layout(location = 0) out vec4 out_color;

void main() {
    out_color = vec4(1, 0, 1, 1);
}