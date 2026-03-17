// Fluid dye advection — 512x512 density/color field.
// Advects dye through the 256x256 velocity field (manual bilinear sampling).
// Also injects smoke at fire block positions and mouse splat dye.

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

@group(0) @binding(0) var dye_in: texture_2d<f32>;
@group(0) @binding(1) var dye_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var vel_tex: texture_2d<f32>;
@group(0) @binding(3) var<uniform> params: FluidParams;
@group(0) @binding(4) var<storage, read> grid: array<u32>;

// Manual bilinear sample of velocity at fractional sim-space position
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

    let v00 = textureLoad(vel_tex, p00, 0).xy;
    let v10 = textureLoad(vel_tex, p10, 0).xy;
    let v01 = textureLoad(vel_tex, p01, 0).xy;
    let v11 = textureLoad(vel_tex, p11, 0).xy;

    return mix(mix(v00, v10, f.x), mix(v01, v11, f.x), f.y);
}

// Manual bilinear sample of dye at fractional dye-space position
fn bilinear_dye(pos: vec2<f32>) -> vec4<f32> {
    let p = pos - 0.5;
    let f = fract(p);
    let base = vec2<i32>(floor(p));
    let dw = i32(params.dye_w);
    let dh = i32(params.dye_h);

    let p00 = clamp(base, vec2(0), vec2(dw - 1, dh - 1));
    let p10 = clamp(base + vec2(1, 0), vec2(0), vec2(dw - 1, dh - 1));
    let p01 = clamp(base + vec2(0, 1), vec2(0), vec2(dw - 1, dh - 1));
    let p11 = clamp(base + vec2(1, 1), vec2(0), vec2(dw - 1, dh - 1));

    let d00 = textureLoad(dye_in, p00, 0);
    let d10 = textureLoad(dye_in, p10, 0);
    let d01 = textureLoad(dye_in, p01, 0);
    let d11 = textureLoad(dye_in, p11, 0);

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

    // Map dye-space position to sim-space for velocity lookup
    let scale = vec2(params.sim_w, params.sim_h) / vec2(params.dye_w, params.dye_h);
    let sim_pos = dye_pos * scale;

    // Sample velocity at this position
    let vel = bilinear_vel(sim_pos);

    // Backtrace in dye-space: scale velocity from sim-space to dye-space
    let inv_scale = vec2(params.dye_w, params.dye_h) / vec2(params.sim_w, params.sim_h);
    let back_pos = dye_pos - vel * inv_scale * params.dt;

    // Bilinear sample dye at backtraced position
    var result = bilinear_dye(back_pos);

    // Dissipation
    result *= params.dissipation;

    // --- Fire source injection ---
    let bx = i32(f32(gid.x) * scale.x);
    let by = i32(f32(gid.y) * scale.y);
    if bx >= 0 && by >= 0 && bx < i32(params.sim_w) && by < i32(params.sim_h) {
        let block = grid[u32(by) * u32(params.sim_w) + u32(bx)];
        let bt = block & 0xFFu;
        if bt == 6u {
            // Fire block: inject warm smoke
            let wx = f32(bx) + 0.5;
            let wy = f32(by) + 0.5;
            let phase = fire_hash(vec2(wx, wy)) * 6.28;
            let flicker = sin(params.time * 8.3 + phase) * 0.3 + 0.7;
            result += vec4(0.6 * flicker, 0.4 * flicker, 0.25 * flicker, 0.4 * flicker);
        }
    }

    // --- Mouse splat dye injection ---
    if params.splat_active > 0.5 {
        let splat_dye_pos = vec2(params.splat_x, params.splat_y) * inv_scale;
        let dx = dye_pos - splat_dye_pos;
        let d2 = dot(dx, dx);
        let r = params.splat_radius * inv_scale.x; // scale radius to dye space
        let r2 = r * r;
        let factor = exp(-d2 / r2);
        // Color based on velocity direction for visual variety
        let hue = atan2(params.splat_vy, params.splat_vx) * 0.159 + 0.5;
        let splat_color = vec3(
            abs(hue * 6.0 - 3.0) - 1.0,
            2.0 - abs(hue * 6.0 - 2.0),
            2.0 - abs(hue * 6.0 - 4.0)
        );
        result += vec4(clamp(splat_color, vec3(0.2), vec3(1.0)) * factor, factor * 0.5);
    }

    textureStore(dye_out, gid.xy, result);
}
