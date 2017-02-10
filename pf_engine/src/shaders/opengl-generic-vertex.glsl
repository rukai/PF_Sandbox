#version 140

// TODO: The only reason this is seperate from the vulkan shaders is because
// TODO: block uniforms are really annoying to handle in glium

in vec2 position;
in float edge;
in float render_id;
out float v_edge;
out float v_render_id;

uniform vec2  position_offset;
uniform float zoom;
uniform float aspect_ratio;
uniform float direction;
uniform vec4  edge_color;
uniform vec4  color;

void main() {
    vec2 pos_flipped = vec2(position[0] * direction, position[1]);
    vec2 pos_camera = (pos_flipped + position_offset) * zoom;
    vec2 pos_aspect = vec2(pos_camera[0], pos_camera[1] * aspect_ratio);
    gl_Position = vec4(pos_aspect, 0.0, 1.0);

    v_edge = edge;
    v_render_id = render_id;
}
