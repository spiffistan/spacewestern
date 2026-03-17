// Fullscreen blit shader — draws a fullscreen triangle, samples the compute output,
// and applies sRGB gamma correction for consistent appearance across platforms.

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    // Fullscreen triangle trick: 3 vertices cover the entire screen
    var out: VertexOutput;
    let x = f32(i32(vi) / 2) * 4.0 - 1.0;
    let y = f32(i32(vi) % 2) * 4.0 - 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    // UV: map from clip space to [0,1]
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@group(0) @binding(0) var t_output: texture_2d<f32>;
@group(0) @binding(1) var s_output: sampler;

// Linear to sRGB gamma curve
fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    let low = c * 12.92;
    let high = pow(c, vec3<f32>(1.0 / 2.4)) * 1.055 - vec3<f32>(0.055);
    return select(high, low, c <= vec3<f32>(0.0031308));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_output, s_output, in.uv);
    return vec4<f32>(linear_to_srgb(color.rgb), color.a);
}
