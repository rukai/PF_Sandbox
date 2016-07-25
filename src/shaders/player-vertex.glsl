# version 140

in vec2 position;
uniform vec2 position_offset;
uniform float zoom;
uniform vec3 uniform_rgb;
uniform float direction;
out vec3 rgb;

void main() {
    rgb = uniform_rgb;
    vec2 pos_flipped = vec2(position[0] * direction, position[1]);
    vec2 pos_final = (pos_flipped + position_offset) * zoom;
    gl_Position = vec4(pos_final, 0.0, 1.0);
}
