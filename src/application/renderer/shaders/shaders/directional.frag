#version 450

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_color;

layout(set = 0, binding = 1) uniform DirectionalData {
    vec2 direction;
    vec3 color;
    float intensity;
} directional;

layout(location = 0) out vec4 f_color;

void main() {
    vec3 result_color = subpassLoad(u_color).rgb;
    vec3 directional_color = directional.color * directional.intensity;
    f_color = vec4(result_color, 1.0);
}
