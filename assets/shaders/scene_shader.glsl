#shader vertex

#include "core_shader_data_buffer.glsl"
#include "core_vertex_layout.glsl"
#include "core_uniforms.glsl"

uniform sampler2D fb_normal_texture;

out vec2 v_uv;
out mat3 v_TBN;

Cull Back
ZWrite Off
ZTest Equal

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
}

#shader frag
layout (location = 0) out vec4 out_color;

in vec2 v_uv;
in mat3 v_TBN;

#include "core_uniforms.glsl"

void main() {
    const vec3 LIGHT_DIR = normalize(-vec3(1.0, 1.0, 1.0));

    const vec2 uv = v_uv;

    const vec4 mettalic_roughness_sample = texture(CORE_METALLIC_ROUGHNESS_MAP, uv);
    const float metallic = mettalic_roughness_sample.b;
    const float roughness = mettalic_roughness_sample.g;

    const vec4 albedo_sample = texture(CORE_ALBEDO_MAP, uv);
    const vec3 albedo = albedo_sample.rgb;

    const vec3 normal_ts = texture(CORE_NORMAL_MAP, uv).xyz * 2.0 - 1.0;

    vec3 normal_ws = normalize(v_TBN * normal_ts);

    float l_dot_n = dot(-LIGHT_DIR, normal_ws);

    vec3 ambient = albedo * 1.5;

    out_color = vec4(l_dot_n * albedo + ambient, 1);
}