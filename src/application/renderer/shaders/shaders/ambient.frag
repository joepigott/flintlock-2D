#version 450

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_color;

layout(set = 0, binding = 0) uniform AmbientData {
    vec3 color;
    float intensity;
} ambient;

void main() {

}
