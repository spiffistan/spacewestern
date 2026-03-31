// 2D wave equation solver for sound propagation.
// Uses velocity formulation: pressure (R) + velocity (G) in Rg32Float textures.
// Walls are hard boundaries (p=0, v=0). Open doors transmit sound.
// Sound sources inject pressure pulses at specified positions.

struct Camera {
    center_x: f32, center_y: f32, zoom: f32, show_roofs: f32,
    screen_w: f32, screen_h: f32, grid_w: f32, grid_h: f32,
    time: f32, glass_light_mul: f32, indoor_glow_mul: f32, light_bleed_mul: f32,
    foliage_opacity: f32, foliage_variation: f32, oblique_strength: f32,
    lm_vp_min_x: f32, lm_vp_min_y: f32, lm_vp_max_x: f32, lm_vp_max_y: f32, lm_scale: f32,
    fluid_overlay: f32,
    sun_dir_x: f32, sun_dir_y: f32, sun_elevation: f32,
    sun_intensity: f32, sun_color_r: f32, sun_color_g: f32, sun_color_b: f32,
    ambient_r: f32, ambient_g: f32, ambient_b: f32,
    enable_prox_glow: f32, enable_dir_bleed: f32, force_refresh: f32,
    pleb_x: f32, pleb_y: f32, pleb_angle: f32, pleb_selected: f32,
    pleb_torch: f32, pleb_headlight: f32,
    prev_center_x: f32, prev_center_y: f32, prev_zoom: f32, prev_time: f32,
    rain_intensity: f32, cloud_cover: f32, wind_magnitude: f32, wind_angle: f32,
    use_shadow_map: f32, shadow_map_scale: f32, sound_speed: f32, sound_damping: f32,
    sound_coupling: f32, enable_terrain_detail: f32, terrain_ao_strength: f32, fog_enabled: f32, hover_x: f32, hover_y: f32, shadow_intensity: f32, pleb_scale: f32, contour_opacity: f32, contour_interval: f32, contour_major_mul: f32,
};

// Sound source: packed as 8 f32 per source. First f32 of buffer = source count.
// Layout per source: [x, y, amplitude, frequency, phase, pattern, duration, _pad]
// pattern: 0=impulse, 1=sine, 2=noise

@group(0) @binding(0) var sound_in: texture_2d<f32>;       // current state (R=pressure, G=velocity)
@group(0) @binding(1) var sound_out: texture_storage_2d<rg32float, write>; // next state
@group(0) @binding(2) var<storage, read> grid: array<u32>;
@group(0) @binding(3) var<uniform> camera: Camera;
@group(0) @binding(4) var<storage, read> sources: array<f32>;
@group(0) @binding(5) var<storage, read> wall_buf: array<u32>;

fn block_type(b: u32) -> u32 { return b & 0xFFu; }
fn block_height(b: u32) -> u32 {
    let h = (b >> 8u) & 0xFFu;
    let bt = b & 0xFFu;
    // Wall blocks: bits 4-7 of height = edge bitmask, not visual height
    if bt == 1u || bt == 4u || bt == 5u || bt == 14u || (bt >= 21u && bt <= 25u) || bt == 35u || bt == 44u { return h & 0xFu; }
    return h;
}
fn is_door(b: u32) -> bool { return ((b >> 16u) & 1u) != 0u; }
fn is_open(b: u32) -> bool { return ((b >> 16u) & 4u) != 0u; }

// --- Wall data helpers (DN-008) ---
fn read_wall_data_s(idx: u32) -> u32 {
    let word = wall_buf[idx >> 1u];
    if (idx & 1u) == 0u { return word & 0xFFFFu; } else { return (word >> 16u) & 0xFFFFu; }
}
fn wd_has_edge_s(wd: u32, edge: u32) -> bool { return (wd & (1u << edge)) != 0u; }

// --- Thin wall helpers ---
fn wall_thickness_raw_s(flags: u32) -> u32 { return (flags >> 5u) & 3u; }
fn is_thin_wall_s(b: u32) -> bool {
    let bh = block_height(b);
    if bh == 0u { return false; }
    return wall_thickness_raw_s((b >> 16u) & 0xFFu) != 0u;
}
fn has_wall_on_edge_s(height: u32, flags: u32, edge: u32) -> bool {
    let thick_raw = (flags >> 5u) & 3u;
    if thick_raw == 0u { return true; }
    let mask = (height >> 4u) & 0xFu;
    if mask == 0u { return true; }
    return (mask & (1u << edge)) != 0u;
}

// Sound sim runs at 2x grid resolution (512x512 for 256x256 grid)
const SOUND_SCALE: i32 = 2;

fn grid_idx(sx: i32, sy: i32) -> u32 {
    let gx = sx / SOUND_SCALE;
    let gy = sy / SOUND_SCALE;
    return u32(gy) * u32(camera.grid_w) + u32(gx);
}

fn is_wall(x: i32, y: i32) -> bool {
    let gx = x / SOUND_SCALE;
    let gy = y / SOUND_SCALE;
    if gx < 0 || gy < 0 || gx >= i32(camera.grid_w) || gy >= i32(camera.grid_h) { return true; }
    let b = grid[u32(gy) * u32(camera.grid_w) + u32(gx)];
    let bt = block_type(b);
    let bh = block_height(b);
    if bh == 0u { return false; }
    if is_door(b) && is_open(b) { return false; }
    // Thin walls: not fully blocking (edge check handles them)
    if is_thin_wall_s(b) { return false; }
    if bt == BT_GLASS { return false; }
    if (bt >= BT_PIPE && bt <= BT_INLET) || bt == BT_RESTRICTOR || bt == BT_LIQUID_PIPE || bt == BT_PIPE_BRIDGE || bt == BT_LIQUID_INTAKE || bt == BT_LIQUID_PUMP || bt == BT_LIQUID_OUTPUT { return false; }
    if bt == BT_WIRE || bt == BT_WIRE_BRIDGE { return false; }
    if bt == BT_DIMMER || bt == BT_BREAKER { return false; }
    if bt == BT_FIREPLACE { return false; }
    if bt == BT_DUG_GROUND || bt == BT_CRATE || bt == BT_ROCK { return false; }
    if bt == BT_BENCH || bt == BT_FLOOR_LAMP || bt == BT_TABLE_LAMP || bt == BT_BED { return false; }
    return true;
}

// Edge-blocked for sound: check thin wall edge between two adjacent tiles
fn sound_edge_blocked(ax: i32, ay: i32, bx: i32, by: i32) -> bool {
    // Convert sound coords to grid coords for edge checks
    let gax = ax / SOUND_SCALE;
    let gay = ay / SOUND_SCALE;
    let gbx = bx / SOUND_SCALE;
    let gby = by / SOUND_SCALE;
    // If both sound texels map to the same grid cell, no edge to check
    if gax == gbx && gay == gby { return false; }
    let ddx = gbx - gax;
    let ddy = gby - gay;
    var dir_a = 0u;
    if ddy < 0 { dir_a = 0u; }
    else if ddx > 0 { dir_a = 1u; }
    else if ddy > 0 { dir_a = 2u; }
    else { dir_a = 3u; }
    let dir_b = (dir_a + 2u) % 4u;
    let gw = i32(camera.grid_w);
    let gh = i32(camera.grid_h);

    // Check wall_data layer first (DN-008) — use grid coords
    if gax >= 0 && gay >= 0 && gax < gw && gay < gh {
        let a_wd = read_wall_data_s(u32(gay) * u32(gw) + u32(gax));
        if wd_has_edge_s(a_wd, dir_a) { return true; }
    }
    if gbx >= 0 && gby >= 0 && gbx < gw && gby < gh {
        let b_wd = read_wall_data_s(u32(gby) * u32(gw) + u32(gbx));
        if wd_has_edge_s(b_wd, dir_b) { return true; }
    }

    // Fall back to block grid (legacy)
    if gax >= 0 && gay >= 0 && gax < gw && gay < gh {
        let ab = grid[u32(gay) * u32(gw) + u32(gax)];
        let abh = block_height(ab);
        if abh > 0u && !(is_door(ab) && is_open(ab)) {
            let af = (ab >> 16u) & 0xFFu;
            if has_wall_on_edge_s(abh, af, dir_a) { return true; }
        }
    }
    if gbx >= 0 && gby >= 0 && gbx < gw && gby < gh {
        let bb = grid[u32(gby) * u32(gw) + u32(gbx)];
        let bbh = block_height(bb);
        if bbh > 0u && !(is_door(bb) && is_open(bb)) {
            let bf = (bb >> 16u) & 0xFFu;
            if has_wall_on_edge_s(bbh, bf, dir_b) { return true; }
        }
    }
    return false;
}

fn read_pressure(from_x: i32, from_y: i32, x: i32, y: i32) -> f32 {
    let sw = i32(camera.grid_w) * SOUND_SCALE;
    let sh = i32(camera.grid_h) * SOUND_SCALE;
    if x < 0 || y < 0 || x >= sw || y >= sh { return 0.0; }
    if is_wall(x, y) { return 0.0; }
    // Edge blocking for thin walls
    if sound_edge_blocked(from_x, from_y, x, y) { return 0.0; }
    return textureLoad(sound_in, vec2(x, y), 0).r;
}

// Wave speed squared and damping from camera uniform
// sound_speed = wave_speed (c), sound_damping = damping
// These are set from the CPU side

@compute @workgroup_size(8, 8)
fn main_sound(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = i32(gid.x);
    let y = i32(gid.y);
    let sw = i32(camera.grid_w) * SOUND_SCALE;
    let sh = i32(camera.grid_h) * SOUND_SCALE;
    if x >= sw || y >= sh { return; }

    // Wall cells: zero pressure and velocity
    if is_wall(x, y) {
        textureStore(sound_out, vec2(gid.xy), vec4(0.0, 0.0, 0.0, 0.0));
        return;
    }

    let curr = textureLoad(sound_in, vec2(x, y), 0);
    let p = curr.r;  // pressure
    let v = curr.g;  // velocity (dp/dt)

    // 5-point Laplacian of pressure (from_x/y = current cell for edge blocking)
    let p_left  = read_pressure(x, y, x - 1, y);
    let p_right = read_pressure(x, y, x + 1, y);
    let p_up    = read_pressure(x, y, x, y - 1);
    let p_down  = read_pressure(x, y, x, y + 1);
    let laplacian = p_left + p_right + p_up + p_down - 4.0 * p;

    // Wave equation: velocity formulation
    let c = camera.sound_speed;  // wave speed
    let damping = camera.sound_damping;  // damping factor
    let c2 = c * c;
    var v_new = v + c2 * laplacian;
    v_new *= (1.0 - damping);  // energy loss per step
    var p_new = p + v_new;

    // Glass blocks: attenuate sound passing through (partial transmission)
    let gx_s = x / SOUND_SCALE;
    let gy_s = y / SOUND_SCALE;
    let block = grid[u32(gy_s) * u32(camera.grid_w) + u32(gx_s)];
    if block_type(block) == 5u {
        p_new *= 0.7;
        v_new *= 0.7;
    }

    // Inject sound sources
    let source_count = i32(sources[0]);
    for (var i: i32 = 0; i < source_count; i++) {
        let base = 1 + i * 8;
        let sx = sources[base + 0];
        let sy = sources[base + 1];
        let amp = sources[base + 2];
        let freq = sources[base + 3];
        let phase = sources[base + 4];
        let pattern = sources[base + 5];
        // Check if this source is at this sound cell (source coords are grid-space)
        let dx = f32(x) + 0.5 - sx * f32(SOUND_SCALE);
        let dy = f32(y) + 0.5 - sy * f32(SOUND_SCALE);
        let dist = dx * dx + dy * dy;
        if dist < 2.0 * f32(SOUND_SCALE * SOUND_SCALE) {  // radius scales with resolution
            let falloff = max(0.0, 1.0 - sqrt(dist));
            if pattern < 0.5 {
                // Impulse: amplitude applied once (duration handles timing on CPU)
                p_new += amp * falloff;
            } else if pattern < 1.5 {
                // Sine wave: continuous oscillation
                p_new += amp * sin(phase) * falloff;
            } else {
                // Noise burst
                let h = fract(sin(camera.time * 127.1 + f32(x) * 311.7 + f32(y) * 113.3) * 43758.5);
                p_new += amp * (h - 0.5) * 2.0 * falloff;
            }
        }
    }

    // Clamp to prevent explosion
    p_new = clamp(p_new, -50.0, 50.0);
    v_new = clamp(v_new, -50.0, 50.0);

    textureStore(sound_out, vec2(gid.xy), vec4(p_new, v_new, 0.0, 0.0));
}
