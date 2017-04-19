#version 450

#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_450pack : enable

layout(location = 0) in vec2 position;
layout(set = 0, binding = 0) uniform Data {
    vec2  position_offset;
    float zoom;
    float aspect_ratio;
    vec4  color;
} uniforms;

void main() {
    vec2 pos_camera = (position + uniforms.position_offset) * uniforms.zoom;
    vec2 pos_aspect = vec2(pos_camera[0], pos_camera[1] * uniforms.aspect_ratio * -1);
    gl_Position = vec4(pos_aspect, 0.0, 1.0);
}
