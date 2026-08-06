#ifndef CORE_VERTEX_LAYOUT
#define CORE_VERTEX_LAYOUT

#ifdef VERTEX_SHADER

layout (location = 0) in vec3 a_position;
layout (location = 1) in vec3 a_normal;
layout (location = 2) in vec4 a_tangent;
layout (location = 3) in vec2 a_uv;

#define CORE_ATTRIB_POSITION (a_position)
#define CORE_ATTRIB_NORMAL (a_normal)
#define CORE_ATTRIB_TANGENT (a_tangent.xyz)
#define CORE_ATTRIB_TANGENT_HANDEDNESS (a_tangent.w)
#define CORE_ATTRIB_UV (a_uv)

#endif // ifdef VERTEX_SHADER

#endif //ifndef CORE_VERTEX_LAYOUT