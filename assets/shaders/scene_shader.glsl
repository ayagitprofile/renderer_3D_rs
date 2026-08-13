#shader vertex

#include "core_shader_data_buffer.glsl"
#include "core_vertex_layout.glsl"
#include "core_uniforms.glsl"

uniform sampler2D fb_normal_texture;
uniform mat4 u_light_vp_matrix;

out vec4 v_position_light_space;
out vec3 v_position_ws;
out vec2 v_uv;
out mat3 v_TBN;

Cull Back
ZWrite Off
ZTest Equal
Surface Opaque

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
    v_position_light_space = u_light_vp_matrix * position_ws;
}

#shader frag

#include "core_uniforms.glsl"
#include "core_light_data_buffer.glsl"
#include "core_shader_data_buffer.glsl"

const float PI = 3.14159265359;

float distribution_GGX(vec3 N, vec3 H, float rough)
{
    float a = rough * rough;
    float a2 = a * a;

    float NdotH = max(dot(N, H), 0.0);
    float NdotH2 = NdotH * NdotH;

    float denom = NdotH2 * (a2 - 1.0) + 1.0;

    return a2 / max(PI * denom * denom, 0.0001);
}

float geometry_schlick_GGX(float NdotV, float rough)
{
    float r = rough + 1.0;
    float k = (r * r) / 8.0;

    return NdotV / (NdotV * (1.0 - k) + k);
}

float geometry_smith(vec3 N, vec3 V, vec3 L, float rough)
{
    float NdotV = max(dot(N,V),0.0);
    float NdotL = max(dot(N,L),0.0);

    return geometry_schlick_GGX(NdotV, rough) *
           geometry_schlick_GGX(NdotL, rough);
}

float radical_inverse_VdC(uint bits)
{
    bits = (bits << 16u) | (bits >> 16u);
    bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
    bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
    bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
    bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);

    return float(bits) * 2.3283064365386963e-10;
}

vec2 hammersley(uint i, uint N)
{
    return vec2(float(i) / float(N), radical_inverse_VdC(i));
}

vec3 importance_sample_GGX(vec2 Xi, vec3 N, float roughness)
{
    float a = roughness * roughness;

    float phi = 2.0 * PI * Xi.x;

    float cosTheta =
        sqrt((1.0 - Xi.y) /
             (1.0 + (a * a - 1.0) * Xi.y));

    float sinTheta = sqrt(1.0 - cosTheta * cosTheta);

    vec3 H;
    H.x = cos(phi) * sinTheta;
    H.y = sin(phi) * sinTheta;
    H.z = cosTheta;

    vec3 up =
        abs(N.z) < 0.999
        ? vec3(0.0,0.0,1.0)
        : vec3(1.0,0.0,0.0);

    vec3 tangent   = normalize(cross(up, N));
    vec3 bitangent = cross(N, tangent);

    return normalize(
        tangent   * H.x +
        bitangent * H.y +
        N         * H.z);
}

vec3 fresnel_schlick_roughness(float cosTheta, vec3 F0, float roughness)
{
    return F0 +
        (max(vec3(1.0 - roughness), F0) - F0) *
        pow(1.0 - cosTheta, 5.0);
}

vec3 ACES_film(vec3 x)
{
    const float a = 2.51;
    const float b = 0.03;
    const float c = 2.43;
    const float d = 0.59;
    const float e = 0.14;

    return clamp(
        (x * (a * x + b)) /
        (x * (c * x + d) + e),
        0.0,
        1.0
    );
}

vec3 evaluate_BRDF(
    vec3 N,
    vec3 V,
    vec3 L,
    vec3 radiance,
    vec3 albedo,
    float metallic,
    float roughness,
    vec3 F0)
{
    vec3 H = normalize(V + L);

    float NDF = distribution_GGX(N, H, roughness);
    float G   = geometry_smith(N, V, L, roughness);
    vec3  F   = fresnel_schlick_roughness(max(dot(H, V), 0.0), F0, roughness);

    vec3 numerator = NDF * G * F;

    float denominator =
        4.0 *
        max(dot(N, V), 0.0) *
        max(dot(N, L), 0.0) +
        0.001;

    vec3 specular = numerator / denominator;

    vec3 kS = F;
    vec3 kD = (vec3(1.0) - kS) * (1.0 - metallic);

    float NdotL = max(dot(N, L), 0.0);

    return
        (kD * albedo / PI + specular) *
        radiance *
        NdotL;
}

const float LIGHT_UNIT_SCALE = 0.001;

vec3 calculate_directional_light(
    Core_Light light,
    vec3 N,
    vec3 V,
    vec3 albedo,
    float metallic,
    float roughness,
    vec3 F0)
{
    vec3 L = normalize(-light.direction.xyz);

    // glTF KHR_lights_punctual directional intensity is lux.
    float illuminance = light.color.w;

    // Renderer exposure / unit conversion.
    float irradiance = illuminance * LIGHT_UNIT_SCALE;

    vec3 radiance = light.color.rgb * irradiance;

    return evaluate_BRDF(
        N,
        V,
        L,
        radiance,
        albedo,
        metallic,
        roughness,
        F0
    );
}

vec3 calculate_point_light(
    Core_Light light,
    vec3 position_ws,
    vec3 N,
    vec3 V,
    vec3 albedo,
    float metallic,
    float roughness,
    vec3 F0)
{
    vec3 toLight = light.position.xyz - position_ws;

    float distanceToLight = length(toLight);
    float light_range = light.attenuation.w;

    if (distanceToLight >= light_range)
        return vec3(0.0);

    vec3 L = toLight / max(distanceToLight, 0.0001);

    float attenuation =
        1.0 / max(distanceToLight * distanceToLight, 0.0001);

    // Smoothly fade to zero at range.
    float range_fade =
        pow(
            clamp(1.0 - distanceToLight / light_range, 0.0, 1.0),
            2.0
        );

    attenuation *= range_fade;

    // glTF point-light intensity is candela.
    float intensity = light.color.w;

    vec3 irradiance =
        light.color.rgb *
        intensity *
        LIGHT_UNIT_SCALE *
        attenuation;

    return evaluate_BRDF(
        N,
        V,
        L,
        irradiance,
        albedo,
        metallic,
        roughness,
        F0
    );
}

vec3 calculate_spot_light(
    Core_Light light,
    vec3 position_ws,
    vec3 N,
    vec3 V,
    vec3 albedo,
    float metallic,
    float roughness,
    vec3 F0)
{
    vec3 toLight = light.position.xyz - position_ws;

    float distanceToLight = length(toLight);
    float light_range = light.attenuation.w;

    if (distanceToLight >= light_range)
        return vec3(0.0);

    vec3 L = toLight / max(distanceToLight, 0.0001);

    // Inverse-square attenuation.
    float attenuation =
        1.0 / max(distanceToLight * distanceToLight, 0.0001);

    // Fade out at the specified range.
    float range_fade =
        pow(
            clamp(1.0 - distanceToLight / light_range, 0.0, 1.0),
            2.0
        );

    attenuation *= range_fade;

    // Spot cone.
    float theta =
        dot(L, normalize(-light.direction.xyz));

    float inner_cos = light.spot_light_data.x;
    float outer_cos = light.spot_light_data.y;

    float epsilon = max(inner_cos - outer_cos, 0.0001);

    float spot =
        clamp(
            (theta - outer_cos) / epsilon,
            0.0,
            1.0
        );

    // glTF spot-light intensity is candela.
    float intensity = light.color.w;

    vec3 irradiance =
        light.color.rgb *
        intensity *
        LIGHT_UNIT_SCALE *
        attenuation *
        spot;

    return evaluate_BRDF(
        N,
        V,
        L,
        irradiance,
        albedo,
        metallic,
        roughness,
        F0
    );
}

vec3 calculate_IBL(
    vec3 N,
    vec3 V,
    vec3 albedo,
    float metallic,
    float roughness,
    vec3 F0,
    float ao)
{
// #define HDR_CUBEMAP
    float NdotV = max(dot(N, V), 0.0);

    vec3 F = fresnel_schlick_roughness(
        NdotV,
        F0,
        roughness
    );

    vec3 kS = F;
    vec3 kD = (1.0 - kS) * (1.0 - metallic);

    // Diffuse IBL
    // Approximate irradiance by sampling the environment using
    // the surface normal.

    float maxLod = float(textureQueryLevels(CORE_CUBEMAP) - 1);

    vec3 irradiance = textureLod(
        CORE_CUBEMAP,
        N,
        maxLod
    ).rgb;

#ifndef HDR_CUBEMAP
    irradiance = pow(irradiance, vec3(2.2));
#endif

    vec3 diffuse_ibl =
        irradiance * albedo * kD;

    // SSAO affects diffuse ambient illumination.
    diffuse_ibl *= ao;

    // Specular IBL

    vec3 R = reflect(-V, N);

    vec3 env = textureLod(
        CORE_CUBEMAP,
        R,
        roughness * maxLod
    ).rgb;

#ifndef HDR_CUBEMAP
    env = pow(env, vec3(2.2));
#endif

    vec3 specular_ibl = env * kS;

    return diffuse_ibl + specular_ibl;
}

uniform sampler2D shadow_map;

float sample_shadow_map(vec4 position_light_space) {
    // perspective division and remapping from [-1;1] to [0;1]
    vec3 proj_coords = (position_light_space.xyz / position_light_space.w) * 0.5 + 0.5;

    // closest depth to light source
    float closest_depth = texture(shadow_map, proj_coords.xy).r;

    float current_depth = proj_coords.z;

    return current_depth > closest_depth ? 1.0 : 0.0;
}

layout (location = 0) out vec4 out_color;

uniform sampler2D ao_texture;

in vec4 v_position_light_space;
in vec3 v_position_ws;
in vec2 v_uv;
in mat3 v_TBN;

void main() {
    const vec2 uv = v_uv;

    const vec4 mettalic_roughness_sample = texture(CORE_METALLIC_ROUGHNESS_MAP, uv);
    const float metallic = mettalic_roughness_sample.b;
    const float roughness = clamp(mettalic_roughness_sample.g, 0.04, 1.0);

    const vec4 albedo_sample = texture(CORE_ALBEDO_MAP, uv);
    const vec3 albedo = albedo_sample.rgb;
    const float alpha = albedo_sample.w;

    const vec3 normal_ts = texture(CORE_NORMAL_MAP, uv).xyz * 2.0 - 1.0;

    vec3 normal_ws = normalize(v_TBN * normal_ts);

    vec3 N = normal_ws;
    vec3 V = normalize(CORE_CAMERA_POSITION - v_position_ws);

    vec3 F0 = mix(vec3(0.04), albedo, metallic);

    vec3 direct_light = vec3(0);

    float dir_light_shadow = sample_shadow_map(v_position_light_space);

    for (uint i = 0; i < CORE_LIGHT_COUNT; i++) {
        Core_Light light = CORE_LIGHT_ARRAY[i];

        uint light_type = uint(light.position.w);

        switch (light_type) {
            case CORE_LIGHT_TYPE_DIRECTIONAL: {
                direct_light += (1.0 - dir_light_shadow) * calculate_directional_light(light, N, V, albedo, metallic, roughness, F0);
            } break;

            case CORE_LIGHT_TYPE_POINT: {
                direct_light += calculate_point_light(light, v_position_ws, N, V, albedo, metallic, roughness, F0);
            } break;

            default: break;
        };
    }

    vec2 ss_uv = vec2(gl_FragCoord.xy + vec2(0.5)) / vec2(CORE_SCREEN_SIZE);
    float ao = textureLod(ao_texture, ss_uv, 0).r;

    vec3 indirect_light = calculate_IBL(N, V, albedo, metallic, roughness, F0, 1.0);

    vec3 color = (direct_light + indirect_light) * ao;

    // HDR tonemap
    color = ACES_film(color);

    // gamma
    color = pow(color, vec3(1.0/2.2));

    out_color = vec4(color, alpha);
}