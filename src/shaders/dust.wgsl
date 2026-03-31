// Dust density simulation — advected by fluid velocity, wall-blocked, slow decay.
// Separate from fluid sim: independent decay/diffusion rates.

struct DustParams {
    grid_w: f32, grid_h: f32, dt: f32,
    decay_rate: f32,
    diffusion: f32,
    wind_follow: f32,
    wind_x: f32, wind_y: f32,
    storm_active: f32,
    storm_edge: f32,
    storm_density: f32,
    _pad: f32,
};

@group(0) @binding(0) var dust_in: texture_2d<f32>;
@group(0) @binding(1) var dust_out: texture_storage_2d<r32float, write>;
@group(0) @binding(2) var vel_tex: texture_2d<f32>;
@group(0) @binding(3) var obstacle_tex: texture_2d<f32>;
@group(0) @binding(4) var<uniform> params: DustParams;

fn is_solid(pos: vec2<i32>) -> bool {
    // Both dust and obstacle textures are 512x512 — 1:1 mapping
    let obs = textureLoad(obstacle_tex, pos, 0).r;
    return obs > 0.5;
}

fn sample_dust(pos: vec2<i32>) -> f32 {
    let w = i32(params.grid_w);
    let h = i32(params.grid_h);
    if pos.x < 0 || pos.y < 0 || pos.x >= w || pos.y >= h { return 0.0; }
    if is_solid(pos) { return 0.0; }
    return textureLoad(dust_in, pos, 0).r;
}

// Obstacle-aware bilinear interpolation
fn bilinear_dust(p: vec2<f32>) -> f32 {
    let w = params.grid_w;
    let h = params.grid_h;
    let fp = p - 0.5; // center-to-corner offset
    let ip = vec2<i32>(floor(fp));
    let f = fract(fp);

    let s00 = sample_dust(ip);
    let s10 = sample_dust(ip + vec2(1, 0));
    let s01 = sample_dust(ip + vec2(0, 1));
    let s11 = sample_dust(ip + vec2(1, 1));

    return mix(mix(s00, s10, f.x), mix(s01, s11, f.x), f.y);
}

@compute @workgroup_size(8, 8)
fn main_dust(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = vec2<i32>(gid.xy);
    let w = i32(params.grid_w);
    let h = i32(params.grid_h);
    if pos.x >= w || pos.y >= h { return; }

    // 1. Obstacle: solid tiles have no dust
    if is_solid(pos) {
        textureStore(dust_out, gid.xy, vec4(0.0));
        return;
    }

    // 2. Read velocity — velocity sim runs at 256x256, dust at 512x512
    // Scale dust coords to sim coords (0.5x)
    let vel_pos = vec2<i32>(pos.x / 2, pos.y / 2);
    let vel = textureLoad(vel_tex, vel_pos, 0).xy;
    // Dust follows wind at reduced rate (heavier than air)
    // Scale velocity from sim-space (256) to dust-space (512): multiply by 2
    let dust_vel = vel * params.wind_follow * 2.0;

    // 3. Semi-Lagrangian advection: backtrace through velocity
    let fpos = vec2<f32>(pos) + 0.5; // texel center
    let backtraced = fpos - dust_vel * params.dt;
    var density = bilinear_dust(backtraced);

    // 4. Diffusion: blend toward 4-neighbor average (very slow)
    let n = sample_dust(pos + vec2(0, -1));
    let s = sample_dust(pos + vec2(0, 1));
    let e = sample_dust(pos + vec2(1, 0));
    let ww = sample_dust(pos + vec2(-1, 0));
    let avg = (n + s + e + ww) * 0.25;
    density = mix(density, avg, params.diffusion);

    // 5. Decay (slow — dust lingers)
    density *= params.decay_rate;

    // 6. Dust storm: inject along windward edge
    if params.storm_active > 0.5 {
        let edge = i32(params.storm_edge);
        let inject = select(false, true,
            (edge == 0 && pos.y < 3) ||   // North edge
            (edge == 1 && pos.x >= w - 3) || // East edge
            (edge == 2 && pos.y >= h - 3) || // South edge
            (edge == 3 && pos.x < 3)         // West edge
        );
        if inject {
            density += params.storm_density * params.dt;
        }
    }

    // 7. Edge dissipation: fade at map borders
    let border = 2.0;
    let fx = f32(pos.x);
    let fy = f32(pos.y);
    let fw = f32(w);
    let fh = f32(h);
    let edge_fade = min(min(fx, fw - fx - 1.0), min(fy, fh - fy - 1.0)) / border;
    density *= clamp(edge_fade, 0.0, 1.0);

    // 8. Clamp and store
    density = clamp(density, 0.0, 2.0);
    textureStore(dust_out, gid.xy, vec4(density, 0.0, 0.0, 0.0));
}
