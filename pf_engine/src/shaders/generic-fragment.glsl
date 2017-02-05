#version 450

#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_450pack : enable

layout(location = 0) in float v_edge;
layout(location = 1) in float render_id;
layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform Data {
    vec2  position_offset;
    float zoom;
    float aspect_ratio;
    float direction;
    vec4  edge_color;
    vec4  color;
} uniforms;

void main() {
    if (render_id == 0.0) {
        f_color = uniforms.color;
    }
    else if (render_id == 1.0) {
        if (v_edge > 0.8) {
            f_color = uniforms.edge_color;
        }
        else {
            f_color = uniforms.color;
        }
    }
    else if (render_id == 2.0) {
        f_color = vec4(1.0, 0.0, 0.0, 1.0);
    }
    else if (render_id == 3.0) {
        f_color = vec4(0.76, 0.106, 0.843, 1.0);
    }
    else if (render_id == 4.0) {
        f_color = vec4(0.52, 0.608, 0.756, 1.0);
    }
    else if (render_id == 5.0) {
        f_color = vec4(0.0, 0.64, 0.0, 1.0);
    }
    else if (render_id == 6.0) {
        f_color = vec4(0.8, 0.8, 0.8, 1.0);
    }
    else if (render_id == 7.0) {
        f_color = vec4(0.0, 0.0, 1.0, 1.0);
    }
}
