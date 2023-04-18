#version 330 core
out vec4 FragColor;

in vec3 Normal;

void main() {
    FragColor = vec4(Normal / 2.0 + 0.5, 1.0);
}
