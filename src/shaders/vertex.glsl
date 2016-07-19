# version 140

in vec2 position;
uniform vec2 position_offset;
uniform float zoom;

void main() {
    vec2 pos = (position + position_offset) * zoom;
    gl_Position = vec4(pos, 0.0, 1.0);
}
