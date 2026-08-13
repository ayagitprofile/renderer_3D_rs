#shader vertex

layout (location = 0) in vec3 a_position;

Cull Back
ZWrite On
ZTest LEqual

uniform mat4 u_light_vp_matrix;
uniform mat4 u_model_matrix;

void main() {
    vec4 position_ws = u_model_matrix * vec4(a_position, 1.0);
    vec4 position_cs = u_light_vp_matrix * position_ws;
    gl_Position = position_cs;
}

#shader frag

void main() {}
