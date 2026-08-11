#ifndef CORE_LIGHT_DATA_BUFFER_INCLUDED
#define CORE_LIGHT_DATA_BUFFER_INCLUDED

struct Core_Light {
    vec4 position;    // xyz = world space position, w = LightType
    vec4 direction;   // xyz = world space direction, w is ignored
    vec4 color;       // rgb = colro, w = intensity
    vec4 attenuation; // x = constant, y = linear, z = quadratic, w = range

    vec4 spot_light_data; // x = inner, y = outer
};

layout(std430, binding = 1) buffer light_data_buffer {
    Core_Light lights[];
} light_data;

#define CORE_LIGHT_TYPE_DIRECTIONAL (0)
#define CORE_LIGHT_TYPE_POINT (1)
#define CORE_LIGHT_TYPE_SPOT (2)

#define CORE_LIGHT_COUNT (light_data.lights.length())
#define CORE_LIGHT_ARRAY (light_data.lights)

#endif // CORE_LIGHT_DATA_BUFFER_INCLUDED