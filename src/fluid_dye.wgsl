// Fluid dye advection — 512x512 density/color field.
// Advects dye through the 256x256 velocity field (manual bilinear sampling).
// Walls block dye: obstacle-aware bilinear sampling prevents wall bleeding.
// Also injects smoke at fire block positions and mouse splat dye.

struct FluidParams {
    sim_w: f32, sim_h: f32, dye_w: f32, dye_h: f32,
    dt: f32, dissipation: f32, vorticity_strength: f32, pressure_iterations: f32,
    splat_x: f32, splat_y: f32, splat_vx: f32, splat_vy: f32,
    splat_radius: f32, splat_active: f32, time: f32, wind_x: f32,
    wind_y: f32, smoke_rate: f32, fan_speed: f32, _pad3: f32,
};

@group(0) @binding(0) var dye_in: texture_2d<f32>;
@group(0) @binding(1) var dye_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var vel_tex: texture_2d<f32>;
@group(0) @binding(3) var<uniform> params: FluidParams;
@group(0) @binding(4) var<storage, read> grid: array<u32>;
@group(0) @binding(5) var obstacle_tex: texture_2d<f32>;

// Scale from dye-space to sim-space (computed per-invocation, but constant)
fn dye_to_sim() -> vec2<f32> {
    return vec2(params.sim_w, params.sim_h) / vec2(params.dye_w, params.dye_h);
}

fn sim_to_dye() -> vec2<f32> {
    return vec2(params.dye_w, params.dye_h) / vec2(params.sim_w, params.sim_h);
}

// Check if a sim-space cell is an obstacle
fn is_obstacle(sim_pos: vec2<i32>) -> bool {
    let sw = i32(params.sim_w);
    let sh = i32(params.sim_h);
    if sim_pos.x < 0 || sim_pos.y < 0 || sim_pos.x >= sw || sim_pos.y >= sh {
        return true; // out of bounds = wall
    }
    return textureLoad(obstacle_tex, sim_pos, 0).r > 0.5;
}

// Manual bilinear sample of velocity at fractional sim-space position
// Returns zero velocity at obstacle cells
fn bilinear_vel(pos: vec2<f32>) -> vec2<f32> {
    let p = pos - 0.5;
    let f = fract(p);
    let base = vec2<i32>(floor(p));
    let sw = i32(params.sim_w);
    let sh = i32(params.sim_h);

    let p00 = clamp(base, vec2(0), vec2(sw - 1, sh - 1));
    let p10 = clamp(base + vec2(1, 0), vec2(0), vec2(sw - 1, sh - 1));
    let p01 = clamp(base + vec2(0, 1), vec2(0), vec2(sw - 1, sh - 1));
    let p11 = clamp(base + vec2(1, 1), vec2(0), vec2(sw - 1, sh - 1));

    // Zero velocity at obstacles
    var v00 = textureLoad(vel_tex, p00, 0).xy;
    var v10 = textureLoad(vel_tex, p10, 0).xy;
    var v01 = textureLoad(vel_tex, p01, 0).xy;
    var v11 = textureLoad(vel_tex, p11, 0).xy;
    if is_obstacle(p00) { v00 = vec2(0.0); }
    if is_obstacle(p10) { v10 = vec2(0.0); }
    if is_obstacle(p01) { v01 = vec2(0.0); }
    if is_obstacle(p11) { v11 = vec2(0.0); }

    return mix(mix(v00, v10, f.x), mix(v01, v11, f.x), f.y);
}

// Obstacle-aware bilinear dye sampling.
// Dye at wall cells is treated as zero — prevents smoke bleeding through walls.
fn bilinear_dye(pos: vec2<f32>) -> vec4<f32> {
    let p = pos - 0.5;
    let f = fract(p);
    let base = vec2<i32>(floor(p));
    let dw = i32(params.dye_w);
    let dh = i32(params.dye_h);
    let scale = dye_to_sim();

    let p00 = clamp(base, vec2(0), vec2(dw - 1, dh - 1));
    let p10 = clamp(base + vec2(1, 0), vec2(0), vec2(dw - 1, dh - 1));
    let p01 = clamp(base + vec2(0, 1), vec2(0), vec2(dw - 1, dh - 1));
    let p11 = clamp(base + vec2(1, 1), vec2(0), vec2(dw - 1, dh - 1));

    // Read dye, but zero it if the sample is inside a wall
    var d00 = textureLoad(dye_in, p00, 0);
    var d10 = textureLoad(dye_in, p10, 0);
    var d01 = textureLoad(dye_in, p01, 0);
    var d11 = textureLoad(dye_in, p11, 0);

    let obs00 = is_obstacle(vec2<i32>(vec2<f32>(p00) * scale));
    let obs10 = is_obstacle(vec2<i32>(vec2<f32>(p10) * scale));
    let obs01 = is_obstacle(vec2<i32>(vec2<f32>(p01) * scale));
    let obs11 = is_obstacle(vec2<i32>(vec2<f32>(p11) * scale));

    if obs00 { d00 = vec4(0.0, 1.0, 0.0, 0.0); }
    if obs10 { d10 = vec4(0.0, 1.0, 0.0, 0.0); }
    if obs01 { d01 = vec4(0.0, 1.0, 0.0, 0.0); }
    if obs11 { d11 = vec4(0.0, 1.0, 0.0, 0.0); }

    return mix(mix(d00, d10, f.x), mix(d01, d11, f.x), f.y);
}

fn fire_hash(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453);
}

@compute @workgroup_size(8, 8)
fn main_advect_dye(@builtin(global_invocation_id) gid: vec3<u32>) {
    if gid.x >= u32(params.dye_w) || gid.y >= u32(params.dye_h) {
        return;
    }

    let dye_pos = vec2<f32>(gid.xy) + 0.5;
    let scale = dye_to_sim();
    let inv_scale = sim_to_dye();

    // Check if this dye texel is inside an obstacle — walls have no smoke, atmospheric O2, ambient temp
    let sim_cell = vec2<i32>(vec2<f32>(gid.xy) * scale);
    if is_obstacle(sim_cell) {
        let day_frac_obs = fract(params.time / 60.0);
        let sun_obs = sin(clamp((day_frac_obs - 0.15) / 0.7, 0.0, 1.0) * 3.14159);
        let amb_temp_obs = mix(5.0, 25.0, sun_obs);
        textureStore(dye_out, gid.xy, vec4(0.0, 1.0, 0.0, amb_temp_obs));
        return;
    }

    // Map dye-space position to sim-space for velocity lookup
    let sim_pos = dye_pos * scale;

    // Sample velocity at this position (obstacle-aware)
    let vel = bilinear_vel(sim_pos);

    // Backtrace in dye-space: scale velocity from sim-space to dye-space
    let back_pos = dye_pos - vel * inv_scale * params.dt;

    // Obstacle-aware bilinear sample of dye at backtraced position
    var result = bilinear_dye(back_pos);

    // Per-channel dissipation: smoke fades, O2 and CO2 are conserved
    result.r *= params.dissipation;  // smoke fades over time
    // O2 (G) and CO2 (B) are conserved gases — no dissipation

    // --- Fire source injection ---
    let bx = sim_cell.x;
    let by = sim_cell.y;
    if bx >= 0 && by >= 0 && bx < i32(params.sim_w) && by < i32(params.sim_h) {
        let block = grid[u32(by) * u32(params.sim_w) + u32(bx)];
        let bt = block & 0xFFu;
        if bt == 6u {
            // Fire block: O2-dependent combustion
            let fire_o2 = result.g;
            let fire_strength = clamp(fire_o2 * 3.0 - 0.5, 0.0, 1.0);
            let wx = f32(bx) + 0.5;
            let wy = f32(by) + 0.5;
            let phase = fire_hash(vec2(wx, wy)) * 6.28;
            let flicker = sin(params.time * 8.3 + phase) * 0.3 + 0.7;
            // Produce smoke (scaled by smoke_rate slider)
            result.r += params.smoke_rate * flicker * fire_strength;
            // Consume O2
            result.g -= 0.03 * fire_strength;
            // Produce CO2
            result.b += 0.02 * fire_strength;
        }
    }

    // Compost: anaerobic decomposition produces CO2 without consuming O2
    if bx >= 0 && by >= 0 && bx < i32(params.sim_w) && by < i32(params.sim_h) {
        let block_c = grid[u32(by) * u32(params.sim_w) + u32(bx)];
        if (block_c & 0xFFu) == 13u {
            result.b += 0.05;  // steady CO2 output
            result.r += 0.005; // trace haze (decomposition gases)
        }
    }

    // Outdoor cells: fresh air exchange (O2 recovers, CO2 dissipates)
    if bx >= 0 && by >= 0 && bx < i32(params.sim_w) && by < i32(params.sim_h) {
        let block_full = grid[u32(by) * u32(params.sim_w) + u32(bx)];
        let has_roof = ((block_full >> 16u) & 2u) != 0u;
        let btype = block_full & 0xFFu;
        if !has_roof && (btype == 0u || btype == 2u) {
            result.g += (1.0 - result.g) * 0.008;  // O2 recovery toward 1.0
            result.b *= 0.992;                       // CO2 dissipates outdoors
        }
    }

    // --- Mouse splat dye injection (smoke channel only) ---
    if params.splat_active > 0.5 {
        let splat_dye_pos = vec2(params.splat_x, params.splat_y) * inv_scale;
        let dx = dye_pos - splat_dye_pos;
        let d2 = dot(dx, dx);
        let r = params.splat_radius * inv_scale.x;
        let r2 = r * r;
        let factor = exp(-d2 / r2);
        result.r += factor * 0.5;
    }

    // --- Diffusion: smoke spreads to adjacent cells (physical mixing in still air) ---
    // Strong enough to fill a 15-block room in ~3 seconds via spread alone
    let d_l = bilinear_dye(dye_pos + vec2(-1.0, 0.0));
    let d_r = bilinear_dye(dye_pos + vec2( 1.0, 0.0));
    let d_u = bilinear_dye(dye_pos + vec2( 0.0,-1.0));
    let d_d = bilinear_dye(dye_pos + vec2( 0.0, 1.0));
    let avg_neighbors = (d_l + d_r + d_u + d_d) * 0.25;
    result += (avg_neighbors - result) * 0.1;

    // --- Accumulation: smoke gains density only when smoke_rate > 0 ---
    if params.smoke_rate > 0.01 && result.r > 0.05 {
        result.r += 0.005 * params.smoke_rate;
    }

    // --- Edge zone: gases reset to atmospheric at map borders ---
    let edge_dist = min(
        min(f32(bx), params.sim_w - f32(bx)),
        min(f32(by), params.sim_h - f32(by))
    );
    if edge_dist < 20.0 {
        let edge_fade = clamp(edge_dist / 20.0, 0.0, 1.0);
        result.r *= edge_fade;               // smoke dissipates at edges
        result.b *= edge_fade;               // CO2 dissipates at edges
        result.g += (1.0 - result.g) * (1.0 - edge_fade) * 0.05; // O2 recovers at edges
    }

    // --- Windward edge: inject fresh O2 from upwind direction ---
    // Wind blows FROM the upwind edge, carrying fresh air into the map
    let wind_mag = length(vec2(params.wind_x, params.wind_y));
    if wind_mag > 0.1 {
        let wind_dir = vec2(params.wind_x, params.wind_y) / wind_mag;
        // Distance from upwind edge (where wind enters the map)
        let upwind_x = select(f32(bx), params.sim_w - f32(bx), wind_dir.x > 0.0);
        let upwind_y = select(f32(by), params.sim_h - f32(by), wind_dir.y > 0.0);
        let upwind_dist = min(upwind_x * abs(wind_dir.x) + upwind_y * abs(wind_dir.y), 30.0);
        if upwind_dist < 15.0 {
            let fresh_air = (1.0 - upwind_dist / 15.0) * 0.02;
            result.g += (1.0 - result.g) * fresh_air;  // inject O2
            result.r *= 1.0 - fresh_air;                // clear smoke
            result.b *= 1.0 - fresh_air;                // clear CO2
        }
    }

    // Per-channel clamping
    result.r = clamp(result.r, 0.0, 2.0);   // smoke
    result.g = clamp(result.g, 0.0, 1.0);   // O2
    result.b = clamp(result.b, 0.0, 1.5);   // CO2
    // --- Temperature (A channel, in Celsius) ---
    // Ambient temperature varies with time of day (approx from sun intensity)
    // params.time cycles 0..60. Dawn=0.15*60=9, dusk=0.85*60=51
    let day_frac = fract(params.time / 60.0);
    let sun_t = clamp((day_frac - 0.15) / 0.7, 0.0, 1.0); // 0 at night, 1 at peak day
    let sun_curve = sin(sun_t * 3.14159);
    let ambient_temp = mix(5.0, 25.0, sun_curve); // 5°C night, 25°C midday

    // Temperature dissipation (slight cooling toward ambient)
    result.a += (ambient_temp - result.a) * 0.005;

    // Fire injects heat
    if bx >= 0 && by >= 0 && bx < i32(params.sim_w) && by < i32(params.sim_h) {
        let block_t = grid[u32(by) * u32(params.sim_w) + u32(bx)];
        if (block_t & 0xFFu) == 6u {
            // Fire: inject ~300°C, scaled by O2 availability
            let fire_o2_t = clamp(result.g * 3.0 - 0.5, 0.0, 1.0);
            result.a = max(result.a, 300.0 * fire_o2_t);
        }
    }

    // Outdoor cells: temperature trends toward ambient
    if bx >= 0 && by >= 0 && bx < i32(params.sim_w) && by < i32(params.sim_h) {
        let block_temp = grid[u32(by) * u32(params.sim_w) + u32(bx)];
        let has_roof_t = ((block_temp >> 16u) & 2u) != 0u;
        let btype_t = block_temp & 0xFFu;
        if !has_roof_t && (btype_t == 0u || btype_t == 2u) {
            result.a += (ambient_temp - result.a) * 0.02; // outdoor air resets faster
        }
    }

    // Edge zone: temperature resets to ambient
    if edge_dist < 20.0 {
        let edge_t = 1.0 - clamp(edge_dist / 20.0, 0.0, 1.0);
        result.a += (ambient_temp - result.a) * edge_t * 0.05;
    }

    // Temperature diffusion (heat spreads through air)
    result.a += (avg_neighbors.a - result.a) * 0.08;

    result.a = clamp(result.a, -20.0, 500.0);

    textureStore(dye_out, gid.xy, result);
}
