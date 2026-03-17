// Fluid pressure solver — Jacobi iteration.
// Reads pressure + divergence, writes pressure. Run N times ping-ponging.

struct FluidParams {
    sim_w: f32,
    sim_h: f32,
    dye_w: f32,
    dye_h: f32,
    dt: f32,
    dissipation: f32,
    vorticity_strength: f32,
    pressure_iterations: f32,
    splat_x: f32,
    splat_y: f32,
    splat_vx: f32,
    splat_vy: f32,
    splat_radius: f32,
    splat_active: f32,
    time: f32,
    _pad: f32,
};

@group(0) @binding(0) var pressure_in: texture_2d<f32>;
@group(0) @binding(1) var pressure_out: texture_storage_2d<r32float, write>;
@group(0) @binding(2) var divergence_tex: texture_2d<f32>;
@group(0) @binding(3) var obstacle_tex: texture_2d<f32>;
@group(0) @binding(4) var<uniform> params: FluidParams;

fn in_bounds(pos: vec2<i32>) -> bool {
    return pos.x >= 0 && pos.y >= 0 && pos.x < i32(params.sim_w) && pos.y < i32(params.sim_h);
}

fn is_fluid(pos: vec2<i32>) -> bool {
    if !in_bounds(pos) { return false; }
    return textureLoad(obstacle_tex, pos, 0).r < 0.5;
}

fn pressure_at(pos: vec2<i32>) -> f32 {
    if !in_bounds(pos) { return 0.0; }
    return textureLoad(pressure_in, pos, 0).r;
}

// Jacobi iteration: solve pressure Poisson equation
// p_new = (pL + pR + pB + pT + div) * 0.25
@compute @workgroup_size(8, 8)
fn main_pressure(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = vec2<i32>(gid.xy);
    if !in_bounds(pos) { return; }

    if !is_fluid(pos) {
        textureStore(pressure_out, gid.xy, vec4(0.0));
        return;
    }

    let pL = pressure_at(pos + vec2(-1, 0));
    let pR = pressure_at(pos + vec2(1, 0));
    let pB = pressure_at(pos + vec2(0, -1));
    let pT = pressure_at(pos + vec2(0, 1));
    let div = textureLoad(divergence_tex, pos, 0).r;

    let p = (pL + pR + pB + pT + div) * 0.25;
    textureStore(pressure_out, gid.xy, vec4(p, 0.0, 0.0, 0.0));
}

// Pressure clear/damp — multiply existing pressure by a factor (temporal coherence)
@compute @workgroup_size(8, 8)
fn main_pressure_clear(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = vec2<i32>(gid.xy);
    if !in_bounds(pos) { return; }

    let p = textureLoad(pressure_in, pos, 0).r;
    textureStore(pressure_out, gid.xy, vec4(p * 0.8, 0.0, 0.0, 0.0));
}
