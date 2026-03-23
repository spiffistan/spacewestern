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
    sound_coupling: f32, enable_terrain_detail: f32, terrain_ao_strength: f32, fog_enabled: f32,
};

@group(0) @binding(0) var water_in: texture_2d<f32>;
@group(0) @binding(1) var water_out: texture_storage_2d<r32float, write>;
@group(0) @binding(2) var<storage, read> grid: array<u32>;
@group(0) @binding(3) var<uniform> camera: Camera;
@group(0) @binding(4) var<storage, read> water_table: array<f32>;

const W: u32 = 256u;
const H: u32 = 256u;

const FLOW_RATE: f32 = 0.12;
const RAIN_RATE: f32 = 0.003;
const EVAP_BASE: f32 = 0.00005;
// Seep rate factor: how fast water wells up from the water table
const SEEP_FACTOR: f32 = 0.002;

fn block_type(b: u32) -> u32 { return b & 0xFFu; }
fn block_height(b: u32) -> u32 { return (b >> 8u) & 0xFFu; }
fn has_roof(b: u32) -> bool { return ((b >> 16u) & 2u) != 0u; }

// Get ground elevation for a tile. Surface = 0, dug ground = negative.
fn get_elevation(b: u32) -> f32 {
    let bt = block_type(b);
    let bh = block_height(b);
    if bt == BT_DUG_GROUND {
        // Dug ground: depth stored in height byte, negative elevation
        return -f32(bh);
    }
    // Walls and solid blocks: treated as very high (water can't flow onto them)
    if bh > 0u && bt != BT_TREE && bt != BT_FIREPLACE && bt != BT_CEILING_LIGHT && bt != BT_FLOOR_LAMP && bt != BT_BERRY_BUSH
        && bt != BT_CRATE && bt != BT_ROCK && bt != BT_WIRE && bt != BT_DIMMER && bt != BT_CROP
        && bt != BT_BREAKER && bt != BT_RESTRICTOR && bt != BT_WALL_TORCH && bt != BT_WALL_LAMP
        && !(bt >= BT_PIPE && bt <= BT_INLET) // gas pipe components
        && bt != BT_LIQUID_PIPE && bt != BT_PIPE_BRIDGE && bt != BT_LIQUID_INTAKE && bt != BT_LIQUID_PUMP && bt != BT_LIQUID_OUTPUT {
        return 10.0; // effectively a wall — water won't flow here
    }
    return 0.0; // ground level
}

fn is_wall_for_water(b: u32) -> bool {
    return get_elevation(b) >= 10.0;
}

@compute @workgroup_size(8, 8)
fn main_water(@builtin(global_invocation_id) gid: vec3<u32>) {
    if gid.x >= W || gid.y >= H { return; }

    let x = i32(gid.x);
    let y = i32(gid.y);
    let idx = gid.y * W + gid.x;

    let block = grid[idx];
    let bt = block_type(block);
    let elev = get_elevation(block);

    // Walls: no water
    if is_wall_for_water(block) {
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
    let wt = water_table[idx];
    let seep_head = wt - elev; // positive = water table above this tile's ground
    if seep_head > 0.0 {
        water += seep_head * SEEP_FACTOR;
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
        water -= evap * wind_factor;
    }

    // --- Flow: water moves from high to low total height ---
    let my_total = elev + water;
    var total_outflow = 0.0;
    var total_inflow = 0.0;

    // Check 4 neighbors
    let dirs = array<vec2<i32>, 4>(vec2(1, 0), vec2(-1, 0), vec2(0, 1), vec2(0, -1));
    for (var d = 0u; d < 4u; d++) {
        let nx = x + dirs[d].x;
        let ny = y + dirs[d].y;

        if nx < 0 || ny < 0 || nx >= i32(W) || ny >= i32(H) {
            // Edge: water drains off the map
            total_outflow += max(water * FLOW_RATE, 0.0);
            continue;
        }

        let nidx = u32(ny) * W + u32(nx);
        let nb = grid[nidx];
        if is_wall_for_water(nb) { continue; }

        let n_elev = get_elevation(nb);
        let n_water = textureLoad(water_in, vec2<i32>(nx, ny), 0).r;
        let n_total = n_elev + n_water;

        let diff = my_total - n_total;
        if diff > 0.0 {
            // Outflow: we're higher, water flows out
            let flow = min(diff * FLOW_RATE, water * 0.25); // cap at 25% per neighbor
            total_outflow += flow;
        } else if diff < 0.0 {
            // Inflow: neighbor is higher, water flows in
            let flow = min(-diff * FLOW_RATE, n_water * 0.25);
            total_inflow += flow;
        }
    }

    water = water - total_outflow + total_inflow;
    water = clamp(water, 0.0, 5.0); // cap at 5 units (full deep pool)

    textureStore(water_out, vec2<i32>(x, y), vec4<f32>(water));
}
