#shader vertex

layout(location = 0) in vec3 a_position;
layout(location = 1) in vec2 a_uv;

out vec2 v_uv;

Cull Back
ZTest ALways
ZWrite off

void main() {
    gl_Position = vec4(a_position, 1);
    v_uv = a_uv;
}

#shader fragment

uniform sampler2D fb_color_texture;
uniform sampler2D fb_depth_texture;
uniform sampler2D fb_normal_texture;
uniform sampler2D ao_texture;
uniform sampler2D shadow_map;

vec3 chromatic_abberation(const vec2 uv, const float abberation) {
    const vec2 uv_center = vec2(0.5, 0.5);

    const vec2 uv_offset = uv - uv_center;
    const float r = length(uv_offset);
    const vec2 dir = normalize(uv_offset);

    const vec2 red_offset = dir * abberation * r * 1.2;
    const vec2 green_offset = dir * abberation * r * 0.2;
    const vec2 blue_offset = dir * abberation * r * -1.0;

    const float red = texture(fb_color_texture, uv + red_offset).r;
    const float green = texture(fb_color_texture, uv + green_offset).g;
    const float blue = texture(fb_color_texture, uv + blue_offset).b;

    return vec3(red, green, blue);
}

float noise(vec2 uv)
{
    return fract(sin(dot(uv, vec2(12.9898, 78.233))) * 43758.5453);
}

vec3 vignette(vec2 uv, vec3 color, float strength)
{
    vec2 p = uv - 0.5;

    float dist = length(p) * 1.414;
    float v = 1.0 - smoothstep(0.2, 1.0, dist);

    v += (noise(uv) - 0.5) / 255.0;

    v = mix(1.0, v, strength);

    return color * clamp(v, 0.0, 1.0);
}

#include "normal_compression.glsl"

layout(location = 0) out vec4 out_color;

layout(std140, binding = 1) uniform config_uniforms {
    float chromatic_abberation;
    float vignette;
} post_processing_config;

in vec2 v_uv;

void main() {
    const vec2 uv = v_uv;
    vec3 color_ab = chromatic_abberation(uv, post_processing_config.chromatic_abberation);

    float shadow_map_sample = texture(shadow_map, uv).r;

    out_color = vec4(vignette(uv, color_ab, post_processing_config.vignette) + vec3(shadow_map_sample)  * 0.001, 1);
    // out_color = vec4(vignette(uv, color_ab, post_processing_config.vignette) * 0.001  + vec3(shadow_map_sample), 1);
}