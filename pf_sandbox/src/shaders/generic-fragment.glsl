#version 450

layout(location = 0) in float v_edge;
layout(location = 1) flat in uint v_render_id;
layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform Data {
    vec4  edge_color;
    vec4  color;
    mat3  transformation;
} uniforms;

void main() {
    if (v_render_id == 0) {
        f_color = uniforms.color;
    }
    else if (v_render_id == 1) {
        if (v_edge > 0.8) {
            f_color = uniforms.edge_color;
        }
        else {
            f_color = uniforms.color;
        }
    }
    else if (v_render_id == 2) {
        f_color = vec4(1.0, 0.0, 0.0, 1.0);
    }
    else if (v_render_id == 3) {
        f_color = vec4(0.76, 0.106, 0.843, 1.0);
    }
    else if (v_render_id == 4) {
        if (v_edge > 0.8) {
            vec4 a = uniforms.edge_color;
            f_color = vec4(a[0], a[1], a[2], 0.5);
        }
        else {
            vec4 a = uniforms.color;
            f_color = vec4(a[0], a[1], a[3], 0.3);
        }
    }
    else if (v_render_id == 5) {
        f_color = vec4(0.52, 0.608, 0.756, 1.0);
    }
    else if (v_render_id == 6) {
        f_color = vec4(0.0, 0.64, 0.0, 1.0);
    }
    else if (v_render_id == 7) {
        f_color = vec4(0.8, 0.8, 0.8, 1.0);
    }
    else if (v_render_id == 8) {
        f_color = vec4(0.0, 0.0, 1.0, 1.0);
    }
}
