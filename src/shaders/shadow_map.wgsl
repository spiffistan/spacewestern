// Shadow map pre-pass: computes sun shadows at grid resolution (256×256).
// The raytrace shader samples this instead of per-pixel ray marching,
// reducing shadow cost from O(pixels × steps) to O(grid × steps).
//
// Output: Rgba8Unorm texture where:
//   RGB = shadow tint (glass color, tree absorption)
//   A   = light factor (1.0 = full sun, 0.0 = full shadow)

struct Camera {
    center_x: f32,
    center_y: f32,
    zoom: f32,
    show_roofs: f32,
    screen_w: f32,
    screen_h: f32,
    grid_w: f32,
    grid_h: f32,
    time: f32,
    glass_light_mul: f32,
    indoor_glow_mul: f32,
    light_bleed_mul: f32,
    foliage_opacity: f32,
    foliage_variation: f32,
    oblique_strength: f32,
    lm_vp_min_x: f32,
    lm_vp_min_y: f32,
    lm_vp_max_x: f32,
    lm_vp_max_y: f32,
    lm_scale: f32,
    fluid_overlay: f32,
    sun_dir_x: f32,
    sun_dir_y: f32,
    sun_elevation: f32,
    sun_intensity: f32,
    sun_color_r: f32,
    sun_color_g: f32,
    sun_color_b: f32,
    ambient_r: f32,
    ambient_g: f32,
    ambient_b: f32,
    enable_prox_glow: f32,
    enable_dir_bleed: f32,
    force_refresh: f32,
    pleb_x: f32,
    pleb_y: f32,
    pleb_angle: f32,
    pleb_selected: f32,
    pleb_torch: f32,
    pleb_headlight: f32,
    prev_center_x: f32,
    prev_center_y: f32,
    prev_zoom: f32,
    prev_time: f32,
    rain_intensity: f32,
    cloud_cover: f32,
    wind_magnitude: f32,
    wind_angle: f32,
    use_shadow_map: f32,
    shadow_map_scale: f32,
    sound_speed: f32,
    sound_damping: f32,
    sound_coupling: f32,
    enable_terrain_detail: f32,
    terrain_ao_strength: f32,
    fog_enabled: f32,
    hover_x: f32,
    hover_y: f32,
};

@group(0) @binding(0) var shadow_out: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<uniform> camera: Camera;
@group(0) @binding(2) var<storage, read> grid: array<u32>;
@group(0) @binding(3) var<storage, read> elevation: array<f32>;

fn block_type(b: u32) -> u32 { return b & 0xFFu; }
fn block_height(b: u32) -> u32 { return (b >> 8u) & 0xFFu; }
fn block_flags(b: u32) -> u32 { return (b >> 16u) & 0xFFu; }
fn has_roof(b: u32) -> bool { return ((b >> 16u) & 2u) != 0u; }
fn is_door(b: u32) -> bool { return ((b >> 16u) & 1u) != 0u; }
fn is_open(b: u32) -> bool { return ((b >> 16u) & 4u) != 0u; }

fn get_block(x: i32, y: i32) -> u32 {
    if x < 0 || y < 0 || x >= i32(camera.grid_w) || y >= i32(camera.grid_h) {
        return 0u;
    }
    return grid[u32(y) * u32(camera.grid_w) + u32(x)];
}

fn get_roof_height(bx: i32, by: i32) -> f32 {
    let block = get_block(bx, by);
    return f32((block >> 24u) & 0xFFu);
}

fn get_elev(x: i32, y: i32) -> f32 {
    if x < 0 || y < 0 || x >= i32(camera.grid_w) || y >= i32(camera.grid_h) { return 0.0; }
    return elevation[(u32(y) * u32(camera.grid_w) + u32(x)) * 2u]; // stride 2: [elev, ao, ...]
}

const SHADOW_MAX_DIST: f32 = 12.0;
const SHADOW_STEP: f32 = 0.25; // finer steps than raytrace (this runs at grid scale, not per-pixel)

// Glass tint
const GLASS_TINT: vec3<f32> = vec3<f32>(0.7, 0.85, 0.95);
const WINDOW_SILL_FRAC: f32 = 0.25;
const WINDOW_LINTEL_FRAC: f32 = 0.15;

fn trace_shadow(wx: f32, wy: f32, surface_height: f32, sun_dir: vec2<f32>, sun_elev: f32) -> vec4<f32> {
    if camera.sun_intensity < 0.001 { return vec4<f32>(1.0, 1.0, 1.0, 0.0); }

    let dir2d = normalize(sun_dir);
    let step_x = dir2d.x * SHADOW_STEP;
    let step_y = dir2d.y * SHADOW_STEP;
    let step_h = sun_elev * SHADOW_STEP;

    var current_h = surface_height;
    var sx = wx;
    var sy = wy;
    var light = 1.0;
    var tint = vec3<f32>(1.0);

    let max_steps = i32(SHADOW_MAX_DIST / SHADOW_STEP);
    for (var i: i32 = 0; i < max_steps; i++) {
        sx += step_x;
        sy += step_y;
        current_h += step_h;

        let bx = i32(floor(sx));
        let by = i32(floor(sy));
        let block = get_block(bx, by);
        let bh = f32(block_height(block)) + get_elev(bx, by); // block height + terrain elevation
        let bt = block_type(block);

        let rh = get_roof_height(bx, by);
        let is_roofed_floor = has_roof(block) && bh < 0.5;

        // Skip non-shadow-casting blocks
        let is_pipe = (bt >= BT_PIPE && bt <= BT_INLET) || bt == BT_RESTRICTOR || bt == BT_LIQUID_PIPE || bt == BT_PIPE_BRIDGE || bt == BT_LIQUID_INTAKE || bt == BT_LIQUID_PUMP || bt == BT_LIQUID_OUTPUT;
        let is_skip = bt == BT_DUG_GROUND || bt == BT_CRATE || bt == BT_ROCK || bt == BT_WIRE || bt == BT_DIMMER || bt == BT_BREAKER || bt == BT_WIRE_BRIDGE || bt == BT_FIREPLACE;
        if is_pipe || is_skip { continue; }

        // Diagonal wall
        if bt == BT_DIAGONAL {
            let sfx = fract(sx);
            let sfy = fract(sy);
            let svar = (block_flags(block) >> 3u) & 3u;
            var is_wall_half = false;
            if svar == 0u { is_wall_half = sfy > (1.0 - sfx); }
            else if svar == 1u { is_wall_half = sfy > sfx; }
            else if svar == 2u { is_wall_half = sfy < (1.0 - sfx); }
            else { is_wall_half = sfy < sfx; }
            if !is_wall_half { continue; }
        }

        if is_door(block) {
            if is_open(block) { continue; } else { return vec4<f32>(tint, 0.0); }
        }

        // Thin wall: only casts shadow in wall sub-cells
        let bf = block_flags(block);
        let tw_raw = (bf >> 5u) & 3u;
        if tw_raw != 0u && bh > 0.1 {
            let sfx = fract(sx);
            let sfy = fract(sy);
            let tw_thick = 4u - tw_raw;
            let tw_frac = f32(tw_thick) * 0.25;
            let tw_edge = (bf >> 3u) & 3u;
            let tw_corner = (bf & 4u) != 0u;
            var in_wall = false;
            if tw_edge == 0u && sfy < tw_frac { in_wall = true; }
            if tw_edge == 1u && sfx > (1.0 - tw_frac) { in_wall = true; }
            if tw_edge == 2u && sfy > (1.0 - tw_frac) { in_wall = true; }
            if tw_edge == 3u && sfx < tw_frac { in_wall = true; }
            if tw_corner {
                let tw_next = (tw_edge + 1u) % 4u;
                if tw_next == 0u && sfy < tw_frac { in_wall = true; }
                if tw_next == 1u && sfx > (1.0 - tw_frac) { in_wall = true; }
                if tw_next == 2u && sfy > (1.0 - tw_frac) { in_wall = true; }
                if tw_next == 3u && sfx < tw_frac { in_wall = true; }
            }
            if !in_wall { continue; }
        }

        // Roofed floor: roof blocks ray at roof height
        if is_roofed_floor {
            if current_h < rh { continue; }
            return vec4<f32>(tint, 0.0);
        }

        // Glass: partial tinted transmission
        if bt == BT_GLASS {
            let window_frac = 1.0 - WINDOW_SILL_FRAC - WINDOW_LINTEL_FRAC;
            let wall_frac = 1.0 - window_frac;
            if current_h < bh {
                light *= (1.0 - 0.35 * SHADOW_STEP * window_frac);
                tint *= mix(vec3<f32>(1.0), GLASS_TINT, SHADOW_STEP * 0.8 * window_frac);
                light *= (1.0 - wall_frac * SHADOW_STEP * 1.5);
                if light < 0.02 { return vec4<f32>(tint, 0.0); }
                continue;
            }
            if current_h >= bh { continue; }
        }

        // Trees: partial shadow
        if bt == BT_TREE {
            if current_h < bh {
                light *= 0.6;
                tint *= vec3<f32>(0.85, 0.95, 0.85);
                if light < 0.02 { return vec4<f32>(tint, 0.0); }
            }
            continue;
        }

        // Berry bush / crop: soft shadow
        if bt == BT_BERRY_BUSH || bt == BT_CROP {
            light *= 0.7;
            if light < 0.02 { return vec4<f32>(tint, 0.0); }
            continue;
        }

        // Block with height: check if ray passes above it
        if bh > 0.0 {
            let effective_h = select(bh, max(bh, rh), rh > 0.5);
            if current_h >= effective_h { continue; }
            return vec4<f32>(tint, 0.0);
        }

        // Roof plane blocks ray
        if rh > 0.5 && current_h < rh {
            continue; // Under the roof plane but no wall — interior airspace
        }
        if rh > 0.5 && current_h >= rh {
            return vec4<f32>(tint, 0.0);
        }
    }

    return vec4<f32>(tint, light);
}

@compute @workgroup_size(8, 8)
fn main_shadow(@builtin(global_invocation_id) gid: vec3<u32>) {
    let sx = gid.x;
    let sy = gid.y;
    let scale = camera.shadow_map_scale;
    let sm_w = u32(camera.grid_w * scale);
    let sm_h = u32(camera.grid_h * scale);
    if sx >= sm_w || sy >= sm_h { return; }

    // Convert shadow map pixel to world coordinate (sub-tile precision)
    let wx = (f32(sx) + 0.5) / scale;
    let wy = (f32(sy) + 0.5) / scale;

    let bx = i32(floor(wx));
    let by = i32(floor(wy));
    let block = get_block(bx, by);
    let bh = f32(block_height(block)) + get_elev(bx, by); // include terrain elevation

    let sun_dir = vec2<f32>(camera.sun_dir_x, camera.sun_dir_y);
    let sun_elev = camera.sun_elevation;

    let result = trace_shadow(wx, wy, bh, sun_dir, sun_elev);
    textureStore(shadow_out, vec2<u32>(sx, sy), result);
}
