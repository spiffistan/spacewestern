// Fluid pressure solver — Jacobi iteration with hybrid boundary conditions.
// Interior walls: Neumann BC (dp/dn=0) — pressure builds up in sealed rooms.
// Domain edges (out-of-bounds): Dirichlet BC (p=0) — absolute reference anchor.
// This is physically correct: outdoor atmosphere = zero pressure reference,
// sealed rooms can accumulate pressure above atmospheric.

struct FluidParams {
    sim_w: f32, sim_h: f32, dye_w: f32, dye_h: f32,
    dt: f32, dissipation: f32, vorticity_strength: f32, pressure_iterations: f32,
    splat_x: f32, splat_y: f32, splat_vx: f32, splat_vy: f32,
    splat_radius: f32, splat_active: f32, time: f32, wind_x: f32,
    wind_y: f32, smoke_rate: f32, fan_speed: f32, rain_intensity: f32,
};

@group(0) @binding(0) var pressure_in: texture_2d<f32>;
@group(0) @binding(1) var pressure_out: texture_storage_2d<r32float, write>;
@group(0) @binding(2) var divergence_tex: texture_2d<f32>;
@group(0) @binding(3) var obstacle_tex: texture_2d<f32>;
@group(0) @binding(4) var<uniform> params: FluidParams;

fn in_bounds(pos: vec2<i32>) -> bool {
    return pos.x >= 0 && pos.y >= 0 && pos.x < i32(params.sim_w) && pos.y < i32(params.sim_h);
}

fn is_solid(pos: vec2<i32>) -> bool {
    if !in_bounds(pos) { return true; }
    let obs_pos = vec2<i32>(pos.x * 512 / i32(params.sim_w), pos.y * 512 / i32(params.sim_h));
    return textureLoad(obstacle_tex, obs_pos, 0).r > 0.5;
}

// Pure Neumann BC: solid/OOB neighbors use center cell's pressure.
// dp/dn = 0 at all boundaries — pressure is relative, not absolute.
// Combined with zero-clear each frame, this prevents temporal oscillation
// while allowing pressure gradients within rooms.
fn neighbor_pressure(n: vec2<i32>, center_p: f32) -> f32 {
    if is_solid(n) { return center_p; }  // Neumann at walls and edges
    return textureLoad(pressure_in, n, 0).r;
}

// Jacobi iteration with hybrid BCs
@compute @workgroup_size(8, 8)
fn main_pressure(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = vec2<i32>(gid.xy);
    if !in_bounds(pos) { return; }

    if is_solid(pos) {
        // Wall cells store zero pressure (they're not fluid)
        textureStore(pressure_out, gid.xy, vec4(0.0));
        return;
    }

    let center = textureLoad(pressure_in, pos, 0).r;

    let pL = neighbor_pressure(pos + vec2(-1, 0), center);
    let pR = neighbor_pressure(pos + vec2( 1, 0), center);
    let pB = neighbor_pressure(pos + vec2( 0,-1), center);
    let pT = neighbor_pressure(pos + vec2( 0, 1), center);
    let div = textureLoad(divergence_tex, pos, 0).r;

    // Clamp pressure to prevent unbounded growth in fully sealed rooms
    let p = clamp((pL + pR + pB + pT + div) * 0.25, -50.0, 50.0);
    textureStore(pressure_out, gid.xy, vec4(p, 0.0, 0.0, 0.0));
}

// Pressure clear/damp — temporal coherence with mild damping
@compute @workgroup_size(8, 8)
fn main_pressure_clear(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = vec2<i32>(gid.xy);
    if !in_bounds(pos) { return; }

    let p = textureLoad(pressure_in, pos, 0).r;
    textureStore(pressure_out, gid.xy, vec4(p * 0.6, 0.0, 0.0, 0.0));
}
