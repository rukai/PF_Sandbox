# version 140

in vec2 position;
uniform mat4 matrix;

void main() {
    vec2 pos = position;
    gl_Position = vec4(pos, 0.0, 1.0) * matrix;
}
