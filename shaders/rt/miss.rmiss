#version 460
#extension GL_EXT_ray_tracing : require

layout(location = 0) rayPayloadInEXT vec3 hitValue;

void main() {
    // Dark sky gradient
    vec3 direction = gl_WorldRayDirectionEXT;
    float t = 0.5 * (direction.y + 1.0);
    hitValue = mix(vec3(0.1, 0.1, 0.15), vec3(0.3, 0.4, 0.6), t);
}
