#shader vertex

#include "global_data_buffer.glsl"

layout (location = 0) in vec3 a_position;
layout (location = 1) in vec3 a_normal;
layout (location = 2) in vec4 a_tangent;
layout (location = 3) in vec2 a_uv;

uniform mat4 u_model_matrix = mat4(1.0);

out vec2 v_uv;
out mat3 v_TBN;

void main() {
    vec4 position_ws = u_model_matrix * vec4(a_position, 1);
    gl_Position = shared_data.camera_vp_matrix * position_ws;

    mat3 normal_matrix = transpose(inverse(mat3(u_model_matrix)));

    vec3 tangent = a_tangent.xyz;
    float handedness = a_tangent.w;

    vec3 N = normalize(normal_matrix * a_normal);
    vec3 T = normalize(normal_matrix * a_tangent.xyz);
    T = normalize(T - N * dot(N, T));
    vec3 B = cross(N, T) * handedness;

    mat3 TBN = mat3(T, B, N);
    v_TBN = TBN;
    v_uv = a_uv;
}

#shader frag
layout (location = 0) out vec4 out_color;

in vec2 v_uv;
in mat3 v_TBN;
uniform sampler2D albedo_map;
uniform sampler2D normal_map;
// G channel stores roughness value, B channel stores metallic value
uniform sampler2D mettalic_roughness_map;

void main() {
    const vec3 LIGHT_DIR = normalize(-vec3(1.0, 1.0, 1.0));
    const vec2 uv = v_uv;
    const vec4 mettalic_roughness_sample = texture(mettalic_roughness_map, uv);
    const float metallic = mettalic_roughness_sample.b;
    const float roughness = mettalic_roughness_sample.g;

    vec3 normal_ts = texture(normal_map, uv).xyz * 2.0 - 1.0;
    vec3 normal_ws = normalize(v_TBN * normal_ts);

    vec3 albedo = texture(albedo_map, uv).xyz;

    float l_dot_n = dot(-LIGHT_DIR, normal_ws);

    out_color = vec4(vec3(roughness), 1);
    // out_color = vec4(l_dot_n * albedo, 1);
}