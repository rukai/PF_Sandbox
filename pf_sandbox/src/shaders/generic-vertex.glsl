#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in float edge;
layout(location = 2) in uint render_id;
layout(location = 0) out float v_edge;
layout(location = 1) out uint v_render_id;

layout(set = 0, binding = 0) uniform Data {
    vec4 edge_color;
    vec4 color;
    mat4 transformation;
} uniforms;

void main() {
    vec4 result = uniforms.transformation * vec4(position, 0.0, 1.0);
    gl_Position = vec4(result[0], result[1] * -1.0, result[2], result[3]); // positive is up

    v_edge = edge;
    v_render_id = render_id;
}
