// Navier-Stokes fluid simulation — 256x256 velocity/pressure passes.
// Entry points: curl, vorticity, divergence, gradient_subtract, advect_velocity, splat.
// All share one bind group layout. Unused bindings are bound to dummies.

struct FluidParams {
    sim_w: f32, sim_h: f32, dye_w: f32, dye_h: f32,
    dt: f32, dissipation: f32, vorticity_strength: f32, pressure_iterations: f32,
    splat_x: f32, splat_y: f32, splat_vx: f32, splat_vy: f32,
    splat_radius: f32, splat_active: f32, time: f32, wind_x: f32,
    wind_y: f32, smoke_rate: f32, fan_speed: f32, rain_intensity: f32,
};

// --- Bind group layout (shared by all entry points) ---
@group(0) @binding(0) var vel_in: texture_2d<f32>;
@group(0) @binding(1) var vel_out: texture_storage_2d<rg32float, write>;
@group(0) @binding(2) var aux_tex: texture_2d<f32>;     // curl (vorticity pass), pressure (gradient pass)
@group(0) @binding(3) var scalar_out: texture_storage_2d<r32float, write>; // curl or divergence output
@group(0) @binding(4) var obstacle_tex: texture_2d<f32>;
@group(0) @binding(5) var<uniform> params: FluidParams;
@group(0) @binding(6) var<storage, read> grid: array<u32>;
@group(0) @binding(7) var dye_tex: texture_2d<f32>;  // for temperature readback (buoyancy)

// --- Helpers ---
fn in_bounds(pos: vec2<i32>) -> bool {
    return pos.x >= 0 && pos.y >= 0 && pos.x < i32(params.sim_w) && pos.y < i32(params.sim_h);
}

fn is_fluid(pos: vec2<i32>) -> bool {
    if !in_bounds(pos) { return false; }
    // Obstacle texture is always 256x256; scale if sim is hires (512)
    let obs_pos = vec2<i32>(pos.x * 256 / i32(params.sim_w), pos.y * 256 / i32(params.sim_h));
    return textureLoad(obstacle_tex, obs_pos, 0).r < 0.5;
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

    // Read curl magnitude from neighbors — zero for walls (no cross-wall vorticity)
    let cL = select(abs(textureLoad(aux_tex, max(pos + vec2(-1, 0), vec2(0)), 0).r), 0.0, !is_fluid(pos + vec2(-1, 0)));
    let cR = select(abs(textureLoad(aux_tex, min(pos + vec2(1, 0), vec2<i32>(i32(params.sim_w) - 1, i32(params.sim_h) - 1)), 0).r), 0.0, !is_fluid(pos + vec2(1, 0)));
    let cB = select(abs(textureLoad(aux_tex, max(pos + vec2(0, -1), vec2(0)), 0).r), 0.0, !is_fluid(pos + vec2(0, -1)));
    let cT = select(abs(textureLoad(aux_tex, min(pos + vec2(0, 1), vec2<i32>(i32(params.sim_w) - 1, i32(params.sim_h) - 1)), 0).r), 0.0, !is_fluid(pos + vec2(0, 1)));
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

    // Backtrace — bilinear_vel handles walls by zeroing velocity at obstacle cells
    let back_pos = vec2<f32>(gid.xy) + 0.5 - v * params.dt;
    var new_v = bilinear_vel(back_pos);

    // Velocity dissipation (slight damping)
    new_v *= 0.998;

    // Temperature-driven buoyancy via gradient method.
    // Compute temperature gradient from neighbors — velocity flows from hot to cold,
    // naturally creating radial expansion from heat sources in all directions.
    let dye_scale = i32(params.dye_w / params.sim_w); // dye texels per sim cell
    let dye_cx = pos.x * dye_scale + dye_scale / 2;
    let dye_cy = pos.y * dye_scale + dye_scale / 2;

    let temp_c = textureLoad(dye_tex, vec2<i32>(dye_cx, dye_cy), 0).a;
    // Temperature gradient: use center temp for wall neighbors (Neumann BC — no gradient across walls)
    let temp_l = select(textureLoad(dye_tex, vec2<i32>(max(dye_cx - dye_scale, 0), dye_cy), 0).a, temp_c, !is_fluid(pos + vec2(-1, 0)));
    let temp_r = select(textureLoad(dye_tex, vec2<i32>(min(dye_cx + dye_scale, i32(params.dye_w) - 1), dye_cy), 0).a, temp_c, !is_fluid(pos + vec2(1, 0)));
    let temp_u = select(textureLoad(dye_tex, vec2<i32>(dye_cx, max(dye_cy - dye_scale, 0)), 0).a, temp_c, !is_fluid(pos + vec2(0, -1)));
    let temp_d = select(textureLoad(dye_tex, vec2<i32>(dye_cx, min(dye_cy + dye_scale, i32(params.dye_h) - 1)), 0).a, temp_c, !is_fluid(pos + vec2(0, 1)));

    // Read smoke and CO2 density at this cell
    let dye_c = textureLoad(dye_tex, vec2<i32>(dye_cx, dye_cy), 0);
    let smoke_density = dye_c.r;
    let co2_density = dye_c.b;

    // Temperature gradient (central differences)
    let grad_x = (temp_r - temp_l) * 0.5;
    let grad_y = (temp_d - temp_u) * 0.5;

    // Ambient temperature
    let day_frac = fract(params.time / 60.0);
    let sun_t = clamp((day_frac - 0.15) / 0.7, 0.0, 1.0);
    let ambient_temp = mix(5.0, 25.0, sin(sun_t * 3.14159));
    let temp_delta = temp_c - ambient_temp;

    // --- Buoyancy: thermal lift vs smoke/gas weight ---
    // Hot air rises (negative gradient force = expansion from heat)
    // Dense smoke and CO2 are heavier than clean air and settle downward
    // The balance creates realistic turbulent plumes: hot smoke initially
    // rises, then cools and sinks, creating rolling vortices.

    // Thermal gradient force (radial expansion from heat sources)
    let grad_mag = length(vec2(grad_x, grad_y));
    var buoyancy_force = vec2(0.0, 0.0);
    if grad_mag > 0.5 {
        let buoyancy_coeff = clamp(abs(temp_delta) * 0.10, 0.0, 25.0);
        buoyancy_force = -vec2(grad_x, grad_y) / grad_mag * buoyancy_coeff;
    }

    // Smoke weight: dense smoke sinks (in top-down: settles southward + spreads)
    // CO2 is 1.5x heavier than air — also sinks
    let smoke_weight = smoke_density * 3.0 + co2_density * 2.0;

    // Net buoyancy: thermal lift minus particle/gas weight
    // In top-down 2D, "sinking" manifests as slight southward drift + lateral spread
    let net_buoyancy = buoyancy_force - vec2(0.0, smoke_weight * 0.5);

    // Add turbulent wobble for natural convection (stronger with smoke for rolling effect)
    let center = vec2<f32>(f32(pos.x) + 0.5, f32(pos.y) + 0.5);
    let phase = fract(sin(dot(center, vec2(127.1, 311.7))) * 43758.5) * 6.28;
    let wobble_strength = 2.0 + smoke_density * 1.5; // more turbulence in smoky air
    let wobble = vec2(
        sin(params.time * 5.3 + phase) * wobble_strength,
        cos(params.time * 4.1 + phase) * wobble_strength
    );

    new_v += (net_buoyancy + wobble) * params.dt;

    // Fire source: extra turbulent kick at fire blocks
    let block = grid[u32(pos.y) * u32(params.sim_w) + u32(pos.x)];
    let bt = block & 0xFFu;
    if bt == BT_FIREPLACE {
        let fire_intensity = f32((block >> 8u) & 0xFFu) / 10.0;
        let center = vec2<f32>(f32(pos.x) + 0.5, f32(pos.y) + 0.5);
        let phase = fract(sin(dot(center, vec2(127.1, 311.7))) * 43758.5) * 6.28;
        let wobble = vec2(
            sin(params.time * 5.3 + phase) * 4.0 * fire_intensity,
            cos(params.time * 4.1 + phase) * 4.0 * fire_intensity
        );
        new_v += wobble * params.dt;
    }

    // Fan: force velocity in fan direction (overrides pressure correction)
    // Acts as a one-way valve — always pushes forward, resists reverse flow
    if bt == BT_FAN {
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
