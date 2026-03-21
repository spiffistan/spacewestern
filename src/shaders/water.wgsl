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
};

@group(0) @binding(0) var water_in: texture_2d<f32>;
@group(0) @binding(1) var water_out: texture_storage_2d<r32float, write>;
@group(0) @binding(2) var<storage, read> grid: array<u32>;
@group(0) @binding(3) var<uniform> camera: Camera;

const W: u32 = 256u;
const H: u32 = 256u;

// Flow rate per frame (lower = more viscous). Tuned for running every 4 frames.
const FLOW_RATE: f32 = 0.12;
// Rain input rate (water units per frame per outdoor tile)
const RAIN_RATE: f32 = 0.003;
// Evaporation rate multiplier (scaled by sun_intensity)
const EVAP_RATE: f32 = 0.001;
// Water table seep rate for deep dug ground
const SEEP_RATE: f32 = 0.0005;

fn block_type(b: u32) -> u32 { return b & 0xFFu; }
fn block_height(b: u32) -> u32 { return (b >> 8u) & 0xFFu; }
fn has_roof(b: u32) -> bool { return ((b >> 16u) & 2u) != 0u; }

// Get ground elevation for a tile. Surface = 0, dug ground = negative.
fn get_elevation(b: u32) -> f32 {
    let bt = block_type(b);
    let bh = block_height(b);
    if bt == 32u {
        // Dug ground: depth stored in height byte, negative elevation
        return -f32(bh);
    }
    // Walls and solid blocks: treated as very high (water can't flow onto them)
    if bh > 0u && bt != 8u && bt != 6u && bt != 7u && bt != 10u && bt != 31u
        && bt != 33u && bt != 34u && bt != 36u && bt != 43u && bt != 47u
        && bt != 45u && bt != 46u {
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

    // Water table seep: deep dug ground slowly fills from below
    if bt == 32u && block_height(block) >= 2u {
        water += SEEP_RATE;
    }

    // --- Sinks ---
    // Evaporation: sun removes surface water (not underground)
    if elev >= 0.0 && !has_roof(block) {
        water -= EVAP_RATE * camera.sun_intensity;
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
