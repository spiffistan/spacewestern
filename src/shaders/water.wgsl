// Ground water simulation — height-based diffusion on GPU.
// Water flows from high (elevation + water_level) to low neighbors.
// Runs every N frames (water is slow/viscous).
//
// Bindings:
//   0: water_in (read) — current water level per tile
//   1: water_out (write) — next frame water level
//   2: grid (read) — block data for elevation + obstacle detection
//   3: camera (uniform) — for rain_intensity, sun_intensity, time

struct Camera {
    center_x: f32, center_y: f32, zoom: f32, show_roofs: f32,
    screen_w: f32, screen_h: f32, grid_w: f32, grid_h: f32,
    time: f32, glass_light_mul: f32, indoor_glow_mul: f32, light_bleed_mul: f32,
    foliage_opacity: f32, foliage_variation: f32, oblique_strength: f32,
    lm_vp_min_x: f32, lm_vp_min_y: f32, lm_vp_max_x: f32, lm_vp_max_y: f32,
    lm_scale: f32, fluid_overlay: f32,
    sun_dir_x: f32, sun_dir_y: f32, sun_elevation: f32,
    sun_intensity: f32, sun_color_r: f32, sun_color_g: f32, sun_color_b: f32,
    ambient_r: f32, ambient_g: f32, ambient_b: f32,
    enable_prox_glow: f32, enable_dir_bleed: f32, force_refresh: f32,
    pleb_x: f32, pleb_y: f32, pleb_angle: f32, pleb_selected: f32,
    pleb_torch: f32, pleb_headlight: f32,
    prev_center_x: f32, prev_center_y: f32, prev_zoom: f32, prev_time: f32,
    rain_intensity: f32, cloud_cover: f32, wind_magnitude: f32, wind_angle: f32,
    use_shadow_map: f32, shadow_map_scale: f32, sound_speed: f32, sound_damping: f32,
    sound_coupling: f32, enable_terrain_detail: f32, terrain_ao_strength: f32, fog_enabled: f32, hover_x: f32, hover_y: f32, shadow_intensity: f32, pleb_scale: f32, contour_opacity: f32, contour_interval: f32, contour_major_mul: f32, water_table_offset: f32, aim_mode: f32,
};

@group(0) @binding(0) var water_in: texture_2d<f32>;
@group(0) @binding(1) var water_out: texture_storage_2d<r32float, write>;
@group(0) @binding(2) var<storage, read> grid: array<u32>;
@group(0) @binding(3) var<uniform> camera: Camera;
@group(0) @binding(4) var<storage, read> water_table: array<f32>;
@group(0) @binding(5) var elevation_tex: texture_2d<f32>; // 1024x1024 sub-tile elevation

const W: u32 = 256u;
const H: u32 = 256u;

// Flow rate must be < 0.25 for stability (4 neighbors × rate < 1.0).
// At 0.15, max drain per step = 60%, leaving headroom for numerical safety.
const FLOW_RATE: f32 = 0.15;
const RAIN_RATE: f32 = 0.003;
const EVAP_BASE: f32 = 0.00005;
// Seep rate factor: how fast water wells up from the water table
const SEEP_FACTOR: f32 = 0.002;

fn block_type(b: u32) -> u32 { return b & 0xFFu; }
fn block_height(b: u32) -> u32 {
    let h = (b >> 8u) & 0xFFu;
    let bt = b & 0xFFu;
    // Wall blocks: bits 4-7 of height = edge bitmask, not visual height
    if bt == 1u || bt == 4u || bt == 5u || bt == 14u || (bt >= 21u && bt <= 25u) || bt == 35u || bt == 44u { return h & 0xFu; }
    return h;
}
fn has_roof(b: u32) -> bool { return ((b >> 16u) & 2u) != 0u; }

// Sample sub-tile elevation for a grid cell (center of 4x4 patch)
fn sample_sub_elevation(x: i32, y: i32) -> f32 {
    let ep = vec2<i32>(x * 4 + 2, y * 4 + 2); // center of 4x4 sub-tile patch
    let clamped = clamp(ep, vec2(0), vec2(1023));
    return textureLoad(elevation_tex, clamped, 0).r;
}

// Get ground elevation for a tile — uses sub-tile heightmap when available,
// falls back to block data for walls/obstacles.
fn get_elevation(b: u32, x: i32, y: i32) -> f32 {
    let bt = block_type(b);
    let bh = block_height(b);
    // Walls and solid blocks: treated as very high (water can't flow onto them)
    if bh > 0u && bt != BT_TREE && bt != BT_FIREPLACE && bt != BT_CEILING_LIGHT && bt != BT_FLOOR_LAMP && bt != BT_BERRY_BUSH
        && bt != BT_CRATE && bt != BT_ROCK && bt != BT_WIRE && bt != BT_DIMMER && bt != BT_CROP
        && bt != BT_BREAKER && bt != BT_RESTRICTOR && bt != BT_WALL_TORCH && bt != BT_WALL_LAMP
        && !(bt >= BT_PIPE && bt <= BT_INLET) // gas pipe components
        && bt != BT_LIQUID_PIPE && bt != BT_PIPE_BRIDGE && bt != BT_LIQUID_INTAKE && bt != BT_LIQUID_PUMP && bt != BT_LIQUID_OUTPUT
        && bt != BT_DUG_GROUND {
        return 10.0; // effectively a wall — water won't flow here
    }
    // Use sub-tile elevation heightmap (includes dug terrain)
    return sample_sub_elevation(x, y);
}

fn is_wall_for_water(b: u32, x: i32, y: i32) -> bool {
    return get_elevation(b, x, y) >= 10.0;
}

@compute @workgroup_size(8, 8)
fn main_water(@builtin(global_invocation_id) gid: vec3<u32>) {
    if gid.x >= W || gid.y >= H { return; }

    let x = i32(gid.x);
    let y = i32(gid.y);
    let idx = gid.y * W + gid.x;

    let block = grid[idx];
    let bt = block_type(block);
    let elev = get_elevation(block, x, y);

    // Walls: no water
    if is_wall_for_water(block, x, y) {
        textureStore(water_out, vec2<i32>(x, y), vec4<f32>(0.0));
        return;
    }

    var water = textureLoad(water_in, vec2<i32>(x, y), 0).r;

    // --- Sources ---
    // Rain: add water to outdoor non-roofed tiles
    if !has_roof(block) && camera.rain_intensity > 0.0 {
        water += RAIN_RATE * camera.rain_intensity;
    }

    // Water table seep: water wells up where water table > elevation
    // water_table_offset shifts the global water table up/down (slider-controlled)
    let wt = water_table[idx] + camera.water_table_offset;
    let seep_head = wt - elev; // positive = water table above this tile's ground
    if seep_head > 0.0 {
        water += seep_head * SEEP_FACTOR;
    }
    // Percolation: water above the water table drains into the ground.
    // Rate scales with how far below the table — slight dip = slow drain, big gap = fast.
    // Lakes in bowls persist because flow refills what drains.
    if seep_head < 0.0 && water > 0.0 {
        let gap = abs(seep_head);
        let drain = water * 0.008 + gap * 0.003; // ~50% in 1.5s at 60fps for typical gap
        water = max(0.0, water - drain);
    }

    // --- Sinks ---
    // Evaporation: temperature-dependent, mostly daytime.
    // Approximate air temperature from day cycle (matches fluid_dye.wgsl ambient).
    // Real evaporation rate roughly doubles per 10°C (Clausius-Clapeyron).
    if elev >= 0.0 && !has_roof(block) {
        let day_frac = fract(camera.time / 60.0);
        let sun_t = clamp((day_frac - 0.15) / 0.7, 0.0, 1.0);
        let sun_curve = sin(sun_t * 3.14159);
        let approx_temp = 5.0 + 20.0 * sun_curve; // 5°C night, 25°C midday
        // Exponential scaling: near-zero below 5°C, ramps up above 15°C
        let temp_factor = max(approx_temp - 5.0, 0.0) / 20.0; // 0 at 5°C, 1 at 25°C
        let evap = EVAP_BASE * temp_factor * temp_factor * camera.sun_intensity;
        // Wind increases evaporation
        let wind_factor = 1.0 + camera.wind_magnitude * 0.05;
        // Shallow puddles evaporate much faster (high surface:volume ratio)
        let shallow_boost = 1.0 + 5.0 / max(water, 0.02); // 0.02 depth → 250x, 1.0 depth → 6x
        water -= evap * wind_factor * shallow_boost;
    }

    // --- Flow: symmetric uncapped scheme (volume-conserving) ---
    // Both outflow and inflow use identical formula: diff * FLOW_RATE.
    // No caps needed because FLOW_RATE < 0.25 guarantees max 4×rate < 100% drain.
    // What A sends to B = what B receives from A (symmetric, conserves volume).
    let my_total = elev + water;
    var net_flow = 0.0; // positive = net outflow, negative = net inflow

    let dirs = array<vec2<i32>, 4>(vec2(1, 0), vec2(-1, 0), vec2(0, 1), vec2(0, -1));
    for (var d = 0u; d < 4u; d++) {
        let nx = x + dirs[d].x;
        let ny = y + dirs[d].y;

        if nx < 0 || ny < 0 || nx >= i32(W) || ny >= i32(H) {
            // Edge: small drain
            net_flow += water * 0.02;
            continue;
        }

        let nidx = u32(ny) * W + u32(nx);
        let nb = grid[nidx];
        if is_wall_for_water(nb, nx, ny) { continue; }

        let n_elev = get_elevation(nb, nx, ny);
        let n_water = textureLoad(water_in, vec2<i32>(nx, ny), 0).r;
        let n_total = n_elev + n_water;

        // Symmetric: positive diff = I'm higher (send), negative = they're higher (receive)
        let diff = my_total - n_total;
        net_flow += diff * FLOW_RATE;
    }

    water = max(0.0, water - net_flow);
    water = min(water, 5.0);

    textureStore(water_out, vec2<i32>(x, y), vec4<f32>(water));
}
