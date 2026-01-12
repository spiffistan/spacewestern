#version 460
#extension GL_EXT_ray_tracing : require

layout(location = 0) rayPayloadInEXT vec3 hitValue;
hitAttributeEXT vec3 hitNormal;

layout(push_constant) uniform PushConstants {
    float time;
    uint width;
    uint height;
    uint _padding;
} pc;

// Get voxel color based on primitive ID
vec3 getVoxelColor(int index) {
    if (index == 0) {
        // The main voxel cube - nice blue
        return vec3(0.2, 0.6, 0.9);
    } else {
        // Ground plane - checkerboard
        vec3 hitPoint = gl_WorldRayOriginEXT + gl_WorldRayDirectionEXT * gl_HitTEXT;
        float scale = 1.0;
        bool white = (mod(floor(hitPoint.x * scale), 2.0) == mod(floor(hitPoint.z * scale), 2.0));
        return white ? vec3(0.9) : vec3(0.3);
    }
}

void main() {
    int primitiveID = gl_PrimitiveID;
    vec3 color = getVoxelColor(primitiveID);
    vec3 normal = hitNormal;

    // Circling light source
    float lightRadius = 3.0;
    float lightHeight = 2.0;
    vec3 lightPos = vec3(
        cos(pc.time) * lightRadius,
        lightHeight,
        -3.0 + sin(pc.time) * lightRadius
    );

    // Calculate intersection point
    vec3 hitPoint = gl_WorldRayOriginEXT + gl_WorldRayDirectionEXT * gl_HitTEXT;

    // Point light
    vec3 toLight = lightPos - hitPoint;
    float lightDist = length(toLight);
    vec3 lightDir = toLight / lightDist;

    // Attenuation
    float attenuation = 1.0 / (1.0 + 0.1 * lightDist * lightDist);

    // Diffuse lighting
    float diffuse = max(dot(normal, lightDir), 0.0);
    float ambient = 0.15;

    // Light color (warm white)
    vec3 lightColor = vec3(1.0, 0.95, 0.8);

    // TODO: Shadow rays would require a separate trace
    // For now, just do direct lighting
    hitValue = color * (ambient + diffuse * attenuation) * lightColor;
}
