// Navier-Stokes fluid simulation — 256x256 velocity/pressure passes.
// Entry points: curl, vorticity, divergence, gradient_subtract, advect_velocity, splat.
// All share one bind group layout. Unused bindings are bound to dummies.

struct FluidParams {
    sim_w: f32, sim_h: f32, dye_w: f32, dye_h: f32,
    dt: f32, dissipation: f32, vorticity_strength: f32, pressure_iterations: f32,
    splat_x: f32, splat_y: f32, splat_vx: f32, splat_vy: f32,
    splat_radius: f32, splat_active: f32, time: f32, wind_x: f32,
    wind_y: f32, smoke_rate: f32, fan_speed: f32, _pad3: f32,
};

// --- Bind group layout (shared by all entry points) ---
@group(0) @binding(0) var vel_in: texture_2d<f32>;
@group(0) @binding(1) var vel_out: texture_storage_2d<rg32float, write>;
@group(0) @binding(2) var aux_tex: texture_2d<f32>;     // curl (vorticity pass), pressure (gradient pass)
@group(0) @binding(3) var scalar_out: texture_storage_2d<r32float, write>; // curl or divergence output
@group(0) @binding(4) var obstacle_tex: texture_2d<f32>;
@group(0) @binding(5) var<uniform> params: FluidParams;
@group(0) @binding(6) var<storage, read> grid: array<u32>;

// --- Helpers ---
fn in_bounds(pos: vec2<i32>) -> bool {
    return pos.x >= 0 && pos.y >= 0 && pos.x < i32(params.sim_w) && pos.y < i32(params.sim_h);
}

fn is_fluid(pos: vec2<i32>) -> bool {
    if !in_bounds(pos) { return false; }
    return textureLoad(obstacle_tex, pos, 0).r < 0.5;
}

// Read velocity, returning zero for out-of-bounds or solid cells
fn vel_at(pos: vec2<i32>) -> vec2<f32> {
    if !in_bounds(pos) { return vec2(0.0); }
    return textureLoad(vel_in, pos, 0).xy;
}

// Manual bilinear sample of velocity field at fractional position
fn bilinear_vel(pos: vec2<f32>) -> vec2<f32> {
    let p = pos - 0.5; // shift to texel centers
    let f = fract(p);
    let base = vec2<i32>(floor(p));
    let v00 = vel_at(base);
    let v10 = vel_at(base + vec2(1, 0));
    let v01 = vel_at(base + vec2(0, 1));
    let v11 = vel_at(base + vec2(1, 1));
    return mix(mix(v00, v10, f.x), mix(v01, v11, f.x), f.y);
}

// --- 1. Compute curl of velocity field ---
@compute @workgroup_size(8, 8)
fn main_curl(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = vec2<i32>(gid.xy);
    if !in_bounds(pos) { return; }

    let vL = vel_at(pos + vec2(-1, 0));
    let vR = vel_at(pos + vec2(1, 0));
    let vB = vel_at(pos + vec2(0, -1));
    let vT = vel_at(pos + vec2(0, 1));

    // curl = dVy/dx - dVx/dy (scalar in 2D)
    let curl = (vR.y - vL.y - vT.x + vB.x) * 0.5;

    textureStore(scalar_out, gid.xy, vec4(curl, 0.0, 0.0, 0.0));
}

// --- 2. Vorticity confinement ---
@compute @workgroup_size(8, 8)
fn main_vorticity(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = vec2<i32>(gid.xy);
    if !in_bounds(pos) {
        textureStore(vel_out, gid.xy, vec4(0.0));
        return;
    }

    let v = textureLoad(vel_in, pos, 0).xy;

    if !is_fluid(pos) {
        textureStore(vel_out, gid.xy, vec4(0.0, 0.0, 0.0, 0.0));
        return;
    }

    // Read curl magnitude from neighbors
    let cL = abs(textureLoad(aux_tex, max(pos + vec2(-1, 0), vec2(0)), 0).r);
    let cR = abs(textureLoad(aux_tex, min(pos + vec2(1, 0), vec2<i32>(i32(params.sim_w) - 1, i32(params.sim_h) - 1)), 0).r);
    let cB = abs(textureLoad(aux_tex, max(pos + vec2(0, -1), vec2(0)), 0).r);
    let cT = abs(textureLoad(aux_tex, min(pos + vec2(0, 1), vec2<i32>(i32(params.sim_w) - 1, i32(params.sim_h) - 1)), 0).r);
    let c = textureLoad(aux_tex, pos, 0).r;

    // Gradient of curl magnitude → points toward vortex center
    var force = vec2(cT - cB, cL - cR);
    let len = length(force);
    if len > 0.0001 {
        force = force / len;
    }
    force *= params.vorticity_strength * c;

    let new_v = v + force * params.dt;
    textureStore(vel_out, gid.xy, vec4(new_v, 0.0, 0.0));
}

// --- 3. Compute divergence ---
@compute @workgroup_size(8, 8)
fn main_divergence(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = vec2<i32>(gid.xy);
    if !in_bounds(pos) { return; }

    if !is_fluid(pos) {
        textureStore(scalar_out, gid.xy, vec4(0.0));
        return;
    }

    let vL = vel_at(pos + vec2(-1, 0)).x;
    let vR = vel_at(pos + vec2(1, 0)).x;
    let vB = vel_at(pos + vec2(0, -1)).y;
    let vT = vel_at(pos + vec2(0, 1)).y;

    let div = (vR - vL + vT - vB) * 0.5;
    textureStore(scalar_out, gid.xy, vec4(-div, 0.0, 0.0, 0.0));
}

// --- 4. Gradient subtract (project velocity to divergence-free) ---
@compute @workgroup_size(8, 8)
fn main_gradient_subtract(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = vec2<i32>(gid.xy);
    if !in_bounds(pos) {
        textureStore(vel_out, gid.xy, vec4(0.0));
        return;
    }

    if !is_fluid(pos) {
        textureStore(vel_out, gid.xy, vec4(0.0, 0.0, 0.0, 0.0));
        return;
    }

    // Pure Neumann BC: solid/OOB neighbors use center pressure (no gradient at walls)
    let center_p = textureLoad(aux_tex, pos, 0).r;
    let maxc = vec2<i32>(i32(params.sim_w) - 1, i32(params.sim_h) - 1);

    let nL = pos + vec2(-1, 0);
    let nR = pos + vec2( 1, 0);
    let nB = pos + vec2( 0,-1);
    let nT = pos + vec2( 0, 1);

    let pL = select(textureLoad(aux_tex, clamp(nL, vec2(0), maxc), 0).r, center_p, !is_fluid(nL));
    let pR = select(textureLoad(aux_tex, clamp(nR, vec2(0), maxc), 0).r, center_p, !is_fluid(nR));
    let pB = select(textureLoad(aux_tex, clamp(nB, vec2(0), maxc), 0).r, center_p, !is_fluid(nB));
    let pT = select(textureLoad(aux_tex, clamp(nT, vec2(0), maxc), 0).r, center_p, !is_fluid(nT));

    let grad = vec2(pR - pL, pT - pB) * 0.5;
    var v = textureLoad(vel_in, pos, 0).xy;
    v -= grad;

    textureStore(vel_out, gid.xy, vec4(v, 0.0, 0.0));
}

// --- 5. Semi-Lagrangian advection of velocity + fire source injection ---
@compute @workgroup_size(8, 8)
fn main_advect_velocity(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = vec2<i32>(gid.xy);
    if !in_bounds(pos) {
        textureStore(vel_out, gid.xy, vec4(0.0));
        return;
    }

    if !is_fluid(pos) {
        textureStore(vel_out, gid.xy, vec4(0.0, 0.0, 0.0, 0.0));
        return;
    }

    let v = textureLoad(vel_in, pos, 0).xy;

    // Backtrace
    let back_pos = vec2<f32>(gid.xy) + 0.5 - v * params.dt;
    var new_v = bilinear_vel(back_pos);

    // Velocity dissipation (slight damping)
    new_v *= 0.998;

    // Fire source: inject outward velocity from fire blocks
    let block = grid[u32(pos.y) * u32(params.sim_w) + u32(pos.x)];
    let bt = block & 0xFFu;
    if bt == 6u {
        // Fireplace: hot air expands outward with turbulent wobble
        let center = vec2<f32>(f32(pos.x) + 0.5, f32(pos.y) + 0.5);
        let phase = fract(sin(dot(center, vec2(127.1, 311.7))) * 43758.5) * 6.28;
        let wobble = vec2(
            sin(params.time * 5.3 + phase) * 4.0,
            cos(params.time * 4.1 + phase) * 4.0
        );
        new_v += (vec2(0.0, -20.0) + wobble) * params.dt;
    }

    // Fan: force velocity in fan direction (overrides pressure correction)
    // Acts as a one-way valve — always pushes forward, resists reverse flow
    if bt == 12u {
        let dir_bits = (block >> 19u) & 3u;  // bits 3-4 of flags
        var fan_dir = vec2(0.0, 0.0);
        if dir_bits == 0u { fan_dir = vec2(0.0, -1.0); }
        else if dir_bits == 1u { fan_dir = vec2(1.0, 0.0); }
        else if dir_bits == 2u { fan_dir = vec2(0.0, 1.0); }
        else { fan_dir = vec2(-1.0, 0.0); }
        // Decompose into along-fan and perpendicular components
        let along = dot(new_v, fan_dir);
        let perp = new_v - fan_dir * along;
        // Force minimum forward velocity, prevent reverse flow
        let forced_along = max(along, params.fan_speed);
        new_v = fan_dir * forced_along + perp * 0.3; // dampen perpendicular flow
    }

    // Global wind: outdoor cells receive wind force
    let bh = (block >> 8u) & 0xFFu;
    let has_roof = ((block >> 16u) & 2u) != 0u;
    if bh == 0u && !has_roof {
        new_v += vec2(params.wind_x, params.wind_y) * params.dt;
    }

    textureStore(vel_out, gid.xy, vec4(new_v, 0.0, 0.0));
}

// --- 6. Splat injection (mouse drag) ---
@compute @workgroup_size(8, 8)
fn main_splat(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = vec2<i32>(gid.xy);
    if !in_bounds(pos) {
        textureStore(vel_out, gid.xy, vec4(0.0));
        return;
    }

    var v = textureLoad(vel_in, pos, 0).xy;

    if params.splat_active > 0.5 {
        let dx = vec2<f32>(gid.xy) + 0.5 - vec2(params.splat_x, params.splat_y);
        let d2 = dot(dx, dx);
        let r2 = params.splat_radius * params.splat_radius;
        let factor = exp(-d2 / r2) * params.dt;
        v += vec2(params.splat_vx, params.splat_vy) * factor;
    }

    textureStore(vel_out, gid.xy, vec4(v, 0.0, 0.0));
}
