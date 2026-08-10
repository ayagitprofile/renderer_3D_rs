#shader compute

#include "core_shader_data_buffer.glsl"
#include "normal_compression.glsl"

layout(local_size_x = 16, local_size_y = 16) in;

layout(std430, binding = 2) readonly buffer kernel_sample_buffer {
    vec4 samples[];
} kernel_buffer;

layout(std140, binding = 0) uniform ssao_ubo {
    float radius;
    float depth_bias;
    float strength;
} ao_config;

uniform sampler2D fb_depth_texture;
uniform sampler2D fb_normal_texture;
uniform sampler2D random_direction_texture;

layout(binding = 0, r16f) uniform writeonly image2D out_ao_texture_image;

vec3 reconstruct_position_vs(vec2 uv, float depth)
{
    const vec4 ndc = vec4(
        uv * 2.0 - 1.0,
        depth * 2.0 - 1.0,
        1.0
    );

    vec4 view =
        CORE_CAMERA_INV_PROJECTION_MATRIX *
        ndc;

    return view.xyz / view.w;
}

mat3 create_TBN_matrix(vec3 normal, vec3 random_vec)
{
    vec3 tangent =
        random_vec -
        normal * dot(random_vec, normal);

    if (dot(tangent, tangent) < 0.001)
    {
        vec3 helper =
            abs(normal.y) < 0.999
                ? vec3(0.0, 1.0, 0.0)
                : vec3(1.0, 0.0, 0.0);

        tangent =
            helper -
            normal * dot(helper, normal);
    }

    tangent = normalize(tangent);

    vec3 bitangent =
        normalize(cross(normal, tangent));

    return mat3(
        tangent,
        bitangent,
        normal
    );
}

void main()
{
    const ivec2 pixel_coord = ivec2(gl_GlobalInvocationID.xy);

    const ivec2 output_image_size = imageSize(out_ao_texture_image);

    // prevent overruning image buffer
    if (any(greaterThanEqual(pixel_coord, output_image_size)))
        return;

    const ivec2 normal_texture_size = textureSize(fb_normal_texture, 0);
    const ivec2 depth_texture_size = textureSize(fb_depth_texture, 0);

    const uint kernel_sample_count = kernel_buffer.samples.length();

    const vec2 ao_uv = (vec2(pixel_coord) + 0.5) / vec2(output_image_size);

    const float depth = textureLod(fb_depth_texture, ao_uv, 0).r;

    // Background.
    if (depth >= 0.9999)
    {
        imageStore(
            out_ao_texture_image,
            pixel_coord,
            vec4(1.0)
        );

        return;
    }

    // Reconstruct position in VIEW SPACE

    const vec3 position_vs = reconstruct_position_vs(ao_uv, depth);

    // World-space normal from normal texture to view-space normal

    const vec3 normal_ws =
        normal_oct_decode(
            textureLod(
                fb_normal_texture,
                ao_uv,
                0
            ).xy
        );

    const vec3 normal_vs =
        normalize(
            mat3(CORE_CAMERA_VIEW_MATRIX) *
            normal_ws
        );

    // Random tangent direction
    const float random_dir_texture_width = 8.0;
    const vec2 noise_uv = fract(ao_uv * vec2(output_image_size) / random_dir_texture_width);

    // Assuming input data is in [-1..1] range, no remapping
    const vec3 random_vec = texture(random_direction_texture, noise_uv).xyz;

    // TBN in VIEW SPACE
    const mat3 TBN =
        create_TBN_matrix(
            normal_vs,
            random_vec
        );

    float occlusion = 0.0;

    for (uint i = 0u; i < kernel_sample_count; ++i)
    {
        // Kernel is already a hemisphere around +Z.
        const vec3 sample_direction =
            TBN * kernel_buffer.samples[i].xyz;

        const vec3 sample_position_vs =
            position_vs +
            sample_direction * ao_config.radius;

        // Project sample position back to screen space

        const vec4 sample_clip =
            CORE_CAMERA_PROJECTION_MATRIX *
            vec4(sample_position_vs, 1.0);

        if (sample_clip.w <= 0.0)
            continue;

        const vec3 sample_ndc =
            sample_clip.xyz / sample_clip.w;

        const vec2 sample_uv =
            sample_ndc.xy * 0.5 + 0.5;

        if (sample_uv.x < 0.0 ||
            sample_uv.x > 1.0 ||
            sample_uv.y < 0.0 ||
            sample_uv.y > 1.0)
        {
            continue;
        }

        // Read depth at projected sample location

        const float scene_depth =
            textureLod(
                fb_depth_texture,
                sample_uv,
                0
            ).r;

        if (scene_depth >= 0.9999)
            continue;

        // Reconstruct the actual geometry position in VIEW SPACE

        const vec3 scene_position_vs =
            reconstruct_position_vs(
                sample_uv,
                scene_depth
            );

        // Occlusion test

        // In OpenGL view space, objects in front of the camera have
        // negative Z. Therefore:
        //
        // scene_position.z > sample_position.z
        //
        // means the sampled geometry is behind our expected sample.
        //
        // For SSAO we want geometry that is closer to the surface than
        // our sample position.

        float depth_difference = scene_position_vs.z - sample_position_vs.z;

        if (depth_difference > ao_config.depth_bias)
        {
#if 1
            float sample_distance = length(sample_position_vs - position_vs);

            float scene_distance = length(scene_position_vs - position_vs);

            float range_weight = 1.0 - smoothstep(0.0, ao_config.radius, abs(sample_distance - scene_distance));
#else
            float range_weight =
                1.0 - smoothstep(
                    0.0,
                    ao_config.radius,
                    length(scene_position_vs - sample_position_vs)
                );
#endif

            occlusion += range_weight;
        }
    }

    // Normalize
    occlusion /= float(kernel_sample_count);

    const float ao_value = 1.0 - clamp(occlusion * ao_config.strength, 0.0, 1.0);

    imageStore(
        out_ao_texture_image,
        pixel_coord,
        vec4(
            ao_value,
            0.0,
            0.0,
            0.0
        )
    );
}