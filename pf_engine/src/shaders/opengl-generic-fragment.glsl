#version 140

// TODO: The only reason this is seperate from the vulkan shaders is because
// TODO: block uniforms are really annoying to handle in glium

in float v_edge;
in float render_id;
out vec4 f_color;

uniform vec2  position_offset;
uniform float zoom;
uniform float aspect_ratio;
uniform float direction;
uniform vec4  edge_color;
uniform vec4  color;

void main() {
    if (render_id == 0.0) {
        f_color = color;
    }
    else if (render_id == 1.0) {
        if (v_edge > 0.8) {
            f_color = edge_color;
        }
        else {
            f_color = color;
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
