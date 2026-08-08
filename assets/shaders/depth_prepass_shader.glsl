#shader vertex

#include "core_shader_data_buffer.glsl"
#include "core_vertex_layout.glsl"
#include "core_uniforms.glsl"

Cull Back
ZWrite On
ZTest Less

out vec3 v_normal_ws;

void main() {
    vec4 position_ws = CORE_MODEL_MATRIX * vec4(CORE_ATTRIB_POSITION, 1);
    gl_Position = shared_data.camera_vp_matrix * position_ws;

    mat3 normal_matrix = transpose(inverse(mat3(CORE_MODEL_MATRIX)));
    vec3 N = normalize(normal_matrix * CORE_ATTRIB_NORMAL);

    v_normal_ws = N;
}

#shader frag

vec2 normal_oct_encoding(vec3 n)
{
    n /= (abs(n.x) + abs(n.y) + abs(n.z));

    vec2 p = n.xy;

    if (n.z < 0.0)
    {
        p = (1.0 - abs(p.yx)) * sign(p);
    }

    return p * 0.5 + 0.5;
}

in vec3 v_normal_ws;

layout(location = 0) out vec2 out_compressed_normal;

void main() {
    out_compressed_normal = normal_oct_encoding(v_normal_ws);
}