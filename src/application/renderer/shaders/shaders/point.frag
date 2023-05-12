#version 450

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_color;

layout(set = 0, binding = 0) uniform PointData {
    vec3 position;
    vec3 color;
    float intensity;
} point;

void main() {

}
