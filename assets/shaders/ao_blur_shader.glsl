#shader compute

layout(local_size_x = 16, local_size_y = 16) in;

layout(binding = 0, r16f) uniform readonly  image2D u_input_ao_texture;
layout(binding = 1, r16f) uniform writeonly image2D u_output_ao_texture;

uniform sampler2D fb_depth_texture;

uniform uint u_horizontal = 1;

const float weights[9] = float[](
    0.227027,
    0.1945946,
    0.1216216,
    0.054054,
    0.016216,
    0.003,
    0.0004,
    0.00005,
    0.000004
);

// Linearize standard OpenGL depth.
// Returns positive view-space distance.
// float linearizeDepth(float depth, float nearPlane, float farPlane)
// {
    // float z = depth * 2.0 - 1.0;
    // return (2.0 * nearPlane * farPlane) /
        //    (farPlane + nearPlane - z * (farPlane - nearPlane));
// }

// void main()
// {
//     const ivec2 pixel_coord = ivec2(gl_GlobalInvocationID.xy);
//     const ivec2 size = imageSize(u_output_ao_texture);

//     if (any(greaterThanEqual(pixel_coord, size)))
//         return;

//     // Horizontal / vertical pass.
//     const ivec2 axis =
//         (u_horizontal == 0)
//         ? ivec2(0, 1)
//         : ivec2(1, 0);

//     float centerAO = imageLoad(
//         u_input_ao_texture,
//         pixel_coord
//     ).r;

//     float centerDepth = texelFetch(
//         fb_depth_texture,
//         pixel_coord,
//         0
//     ).r;

//     float centerLinearDepth = linearizeDepth(
//         centerDepth,
//         u_near_plane,
//         u_far_plane
//     );

//     float result = centerAO * weights[0];
//     float total  = weights[0];

//     // Controls how quickly AO stops crossing depth discontinuities.
//     // This should generally be related to your AO radius / scene scale.
//     const float depthSigma = 0.15;

//     for (int i = 1; i < 5; ++i)
//     {
//         ivec2 offset = axis * i;

//         ivec2 p1 = clamp(
//             pixel_coord + offset,
//             ivec2(0),
//             size - 1
//         );

//         ivec2 p2 = clamp(
//             pixel_coord - offset,
//             ivec2(0),
//             size - 1
//         );

//         float ao1 = imageLoad(
//             u_input_ao_texture,
//             p1
//         ).r;

//         float ao2 = imageLoad(
//             u_input_ao_texture,
//             p2
//         ).r;

//         float depth1 = texelFetch(
//             fb_depth_texture,
//             p1,
//             0
//         ).r;

//         float depth2 = texelFetch(
//             fb_depth_texture,
//             p2,
//             0
//         ).r;

//         float linearDepth1 = linearizeDepth(
//             depth1,
//             u_near_plane,
//             u_far_plane
//         );

//         float linearDepth2 = linearizeDepth(
//             depth2,
//             u_near_plane,
//             u_far_plane
//         );

//         float depthDiff1 =
//             abs(linearDepth1 - centerLinearDepth);

//         float depthDiff2 =
//             abs(linearDepth2 - centerLinearDepth);

//         // Bilateral depth weighting.
//         float depthWeight1 =
//             exp(-depthDiff1 * depthDiff1 /
//                 (2.0 * depthSigma * depthSigma));

//         float depthWeight2 =
//             exp(-depthDiff2 * depthDiff2 /
//                 (2.0 * depthSigma * depthSigma));

//         // Spatial Gaussian weight.
//         float spatialWeight = weights[i];

//         float w1 = spatialWeight * depthWeight1;
//         float w2 = spatialWeight * depthWeight2;

//         result += ao1 * w1;
//         result += ao2 * w2;

//         total += w1 + w2;
//     }

//     result /= max(total, 1e-5);

//     imageStore(
//         u_output_ao_texture,
//         pixel_coord,
//         vec4(result, 0.0, 0.0, 0.0)
//     );
// }

void main() {
    const ivec2 pixel_coord = ivec2(gl_GlobalInvocationID.xy);
    const ivec2 output_image_size = imageSize(u_output_ao_texture);

    if (any(greaterThanEqual(pixel_coord, output_image_size)))
        return;

    vec2 ss_uv = (vec2(pixel_coord) + vec2(0.5)) / vec2(output_image_size);

    float center_ao = imageLoad(u_input_ao_texture, pixel_coord).r;
    float center_depth = textureLod(fb_depth_texture, ss_uv, 0).r;

    float result = center_ao * weights[0];
    float total  = weights[0];

    ivec2 axis = (u_horizontal == 0) ? ivec2(0, 1) : ivec2(1, 0);

    for (uint i = 1; i < 7; i++) {
        ivec2 offset = ivec2(axis * i);

        ivec2 p1 = clamp(pixel_coord + offset, ivec2(0), output_image_size - 1);
        ivec2 p2 = clamp(pixel_coord - offset, ivec2(0), output_image_size - 1);

        float ao1 = imageLoad(u_input_ao_texture, p1).r;
        float ao2 = imageLoad(u_input_ao_texture, p2).r;

        float d1 = textureLod(fb_depth_texture, (vec2(p1) + 0.5) / vec2(output_image_size), 0).r;
        float d2 = textureLod(fb_depth_texture, (vec2(p2) + 0.5) / vec2(output_image_size), 0).r;

        // Prevent AO from bleeding across depth discontinuities.
        float depth_sigma = 10.0;
#if 1
        float weight = exp(-float(i * i) / (2.0 * depth_sigma * depth_sigma));
#else
        float weight = weights[i];
#endif
        float w1 = weight * exp(-abs(d1 - center_depth) * depth_sigma);
        float w2 = weight * exp(-abs(d2 - center_depth) * depth_sigma);

        result += ao1 * w1;
        result += ao2 * w2;

        total += w1 + w2;
    }

    result /= total;

    imageStore(u_output_ao_texture, pixel_coord, vec4(result, 0, 0, 0));
}