#shader vertex

layout(location = 0) in vec3 a_position;

#include "core_shader_data_buffer.glsl"

ZTest LEqual
ZWrite Off
Cull Front

out vec3 v_sample_direction;

void main() {
    mat4 view_no_translation = mat4(mat3(CORE_CAMERA_VIEW_MATRIX));

    vec4 position_ws = CORE_CAMERA_PROJECTION_MATRIX * view_no_translation * vec4(a_position, 1.0);

    gl_Position = position_ws.xyww;

    v_sample_direction = a_position;
}

#shader frag

#include "core_uniforms.glsl"

layout(location = 0) out vec4 out_color;

in vec3 v_sample_direction;

void main() {
    vec3 sample_direction = normalize(v_sample_direction);
    vec3 cubemap_sample = texture(CORE_CUBEMAP, v_sample_direction).rgb;
    out_color = vec4(cubemap_sample, 1);
}