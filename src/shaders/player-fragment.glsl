# version 140

out vec4 color;
in vec3 rgb;

void main() {
    color = vec4(rgb, 0.5);
}
