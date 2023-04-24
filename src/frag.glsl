#version 330 core
layout (location = 0) out vec4 FragColor;

in vec3 Position;
in vec3 Normal;

uniform vec3 camera;
uniform vec3 lightDirection;
uniform vec3 materialColor;


void main() {
    // Material properties
//    vec3 materialColor = vec3(0.18823, 0.51372, 0.86274);
    float shininess = 0.5;
    float ambientLight = 0.1;

    vec3 viewDirection = normalize(camera - Position);
    vec3 halfwayDir = normalize(-lightDirection + viewDirection);

    float spec = pow(max(dot(Normal, halfwayDir), 0.0), shininess);
    vec3 specular = materialColor * spec;

    vec3 ambient = materialColor * ambientLight;

    vec3 totalLight = specular + ambient;


//    vec3 normalColor = normalize(Normal / 2.5 + 0.25);
//    FragColor = vec4(Normal / 2.5 + 0.25, 1.0);
//    FragColor = vec4(0.0, gl_FragCoord.z, dot(normalColor, normalColor), 1.0);

    // Gamma correction
    float gamma = 2.2;
    FragColor = vec4(pow(totalLight, vec3(1.0 / gamma)), 1.0);
}
