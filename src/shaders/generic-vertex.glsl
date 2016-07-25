# version 140

in vec2 position;
uniform vec2 position_offset;
uniform float zoom;
uniform vec3 uniform_rgb;
out vec3 rgb;

void main() {
    rgb = uniform_rgb;
    vec2 pos = (position + position_offset) * zoom;
    gl_Position = vec4(pos, 0.0, 1.0);
}
