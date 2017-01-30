#version 450

#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_450pack : enable

layout(location = 0) in vec2 position;
layout(location = 1) in float edge;
layout(location = 2) in float render_id;
layout(location = 0) out float v_edge;
layout(location = 1) out float v_render_id;
layout(set = 0, binding = 0) uniform Data {
    vec2  position_offset;
    float zoom;
    float aspect_ratio;
    float direction;
    vec3  edge_color;
    vec3  color;
} uniforms;

void main() {
    vec2 pos_flipped = vec2(position[0] * uniforms.direction, position[1]);
    vec2 pos_camera = (pos_flipped + uniforms.position_offset) * uniforms.zoom;
    vec2 pos_aspect = vec2(pos_camera[0], pos_camera[1] * uniforms.aspect_ratio * -1);
    gl_Position = vec4(pos_aspect, 0.0, 1.0);

    v_edge = edge;
    v_render_id = render_id;
}
