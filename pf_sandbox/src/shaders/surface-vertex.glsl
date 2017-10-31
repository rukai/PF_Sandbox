#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec4 color;
layout(location = 0) out vec4 v_color;

layout(set = 0, binding = 0) uniform Data {
    mat4  transformation;
} uniforms;

void main() {
    vec4 result = uniforms.transformation * vec4(position, 0.0, 1.0);
    gl_Position = vec4(result[0], result[1] * -1.0, result[2], result[3]); // positive is up

    v_color = color;
}
