#shader vertex

#include "core_shader_data_buffer.glsl"
#include "core_vertex_layout.glsl"
#include "core_uniforms.glsl"

uniform sampler2D fb_normal_texture;

out vec3 v_position_ws;
out vec2 v_uv;
out mat3 v_TBN;

Cull Back
ZWrite Off
ZTest LEqual
Surface Transparent

vec3 normal_oct_decode(vec2 e)
{
    e = e * 2.0 - 1.0;

    vec3 n = vec3(e.x, e.y, 1.0 - abs(e.x) - abs(e.y));

    if (n.z < 0.0)
    {
        n.xy = (1.0 - abs(n.yx)) * sign(n.xy);
    }

    return normalize(n);
}

void main() {
    vec4 position_ws = CORE_MODEL_MATRIX * vec4(CORE_ATTRIB_POSITION, 1);
    gl_Position = shared_data.camera_vp_matrix * position_ws;

    mat3 normal_matrix = transpose(inverse(mat3(CORE_MODEL_MATRIX)));

    vec3 tangent = CORE_ATTRIB_TANGENT.xyz;

    vec3 N = normalize(normal_matrix * CORE_ATTRIB_NORMAL);
    vec3 T = normalize(normal_matrix * CORE_ATTRIB_TANGENT);
    T = normalize(T - N * dot(N, T));
    vec3 B = cross(N, T) * CORE_ATTRIB_TANGENT_HANDEDNESS;

    mat3 TBN = mat3(T, B, N);
    v_TBN = TBN;
    v_uv = a_uv;
    v_position_ws = position_ws.xyz;
}

#shader frag

layout(location = 0) out vec4 out_color;

void main() {
    out_color = vec4(1, 0, 1, 0.5);
}