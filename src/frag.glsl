#version 330 core
layout (location = 0) out vec4 FragColor;

in vec3 Position;
in vec3 Normal;

uniform vec3 camera;
uniform vec3 lightDirection;


void main() {
    // Material properties
    vec3 materialColor = vec3(0.18823, 0.51372, 0.86274);
    float shininess = 0.5;

    vec3 viewDirection = normalize(camera - Position);
    vec3 halfwayDir = normalize(-lightDirection + viewDirection);

    float spec = pow(max(dot(Normal, halfwayDir), 0.0), shininess);
    vec3 specular = materialColor * spec;


//    vec3 normalColor = normalize(Normal / 2.5 + 0.25);
//    FragColor = vec4(Normal / 2.5 + 0.25, 1.0);
//    FragColor = vec4(0.0, gl_FragCoord.z, dot(normalColor, normalColor), 1.0);
    FragColor = vec4(specular, 1.0);
}
