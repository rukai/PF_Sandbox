#version 450

#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_450pack : enable

layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform Data {
    vec2  position_offset;
    float zoom;
    float aspect_ratio;
    vec4  color;
} uniforms;

void main() {
    f_color = uniforms.color;
}
