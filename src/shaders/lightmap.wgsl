// Lightmap compute shader — seed pass.
// Writes light source values at fire/electric light positions, zeros everywhere else.
// Works at lightmap resolution (lm_scale × grid resolution).
// The main raytrace shader bilinearly samples this for smooth gradients.

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
    sun_dir_x: f32, sun_dir_y: f32, sun_elevation: f32,
    sun_intensity: f32, sun_color_r: f32, sun_color_g: f32, sun_color_b: f32,
    ambient_r: f32, ambient_g: f32, ambient_b: f32,
    enable_prox_glow: f32, enable_dir_bleed: f32,
    force_refresh: f32,
    pleb_x: f32, pleb_y: f32, pleb_angle: f32, pleb_selected: f32,
    pleb_torch: f32, pleb_headlight: f32,
    prev_center_x: f32, prev_center_y: f32, prev_zoom: f32, prev_time: f32,
    rain_intensity: f32, cloud_cover: f32, wind_magnitude: f32, wind_angle: f32,
};

// --- Seed pass bindings ---
@group(0) @binding(0) var lightmap_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(1) var<uniform> camera: Camera;
@group(0) @binding(2) var<storage, read> grid: array<u32>;
@group(0) @binding(3) var<storage, read> materials: array<GpuMaterial>;
@group(0) @binding(4) var<storage, read> voltage: array<f32>;

struct GpuMaterial {
    color_r: f32, color_g: f32, color_b: f32, render_style: f32,
    is_solid: f32, light_transmission: f32, fluid_obstacle: f32, default_height: f32,
    light_intensity: f32, light_color_r: f32, light_color_g: f32, light_color_b: f32,
    light_radius: f32, light_height: f32, is_emissive: f32, is_furniture: f32,
    heat_capacity: f32, conductivity: f32, solar_absorption: f32, is_flammable: f32,
    ignition_temp: f32, walkable: f32, is_removable: f32, _pad: f32,
};

fn get_material(bt: u32) -> GpuMaterial { return materials[min(bt, 47u)]; }

// --- Block unpacking ---
fn block_type(b: u32) -> u32 { return b & 0xFFu; }
fn block_height(b: u32) -> u32 { return (b >> 8u) & 0xFFu; }
fn has_roof(b: u32) -> bool { return ((b >> 16u) & 2u) != 0u; }
fn is_door(b: u32) -> bool { return ((b >> 16u) & 1u) != 0u; }
fn is_open(b: u32) -> bool { return ((b >> 16u) & 4u) != 0u; }

fn get_block(x: i32, y: i32) -> u32 {
    if x < 0 || y < 0 || x >= i32(camera.grid_w) || y >= i32(camera.grid_h) {
        return 0u;
    }
    return grid[u32(y) * u32(camera.grid_w) + u32(x)];
}

// --- Light source constants ---
const FIRE_COLOR: vec3<f32> = vec3<f32>(1.0, 0.55, 0.15);
const FIRE_COLOR_HOT: vec3<f32> = vec3<f32>(1.0, 0.85, 0.4);
const FIRE_BASE_INTENSITY: f32 = 0.90;
const FIRE_FLICKER_AMP: f32 = 0.20;

const ELIGHT_COLOR: vec3<f32> = vec3<f32>(0.95, 0.92, 0.85);
const ELIGHT_INTENSITY: f32 = 1.0;

fn fire_hash(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453);
}

fn fire_flicker(time: f32) -> f32 {
    let f1 = sin(time * 8.3) * 0.3;
    let f2 = sin(time * 13.7 + 2.1) * 0.2;
    let f3 = sin(time * 23.1 + 0.7) * 0.15;
    let f4 = sin(time * 37.9 + 4.3) * 0.1;
    let gutter = sin(time * 3.1) * sin(time * 1.7);
    let gutter_pulse = max(0.0, gutter) * 0.25;
    return clamp(0.5 + f1 + f2 + f3 + f4 - gutter_pulse, 0.0, 1.0);
}

// --- Seed pass: write light source values, zero everything else ---
// gid indexes lightmap texels (not grid cells)
@compute @workgroup_size(8, 8)
fn main_lightmap_seed(@builtin(global_invocation_id) gid: vec3<u32>) {
    let lm_w = u32(camera.grid_w * camera.lm_scale);
    let lm_h = u32(camera.grid_h * camera.lm_scale);

    if gid.x >= lm_w || gid.y >= lm_h {
        return;
    }

    // Convert lightmap texel to block coordinates
    let bx = u32(f32(gid.x) / camera.lm_scale);
    let by = u32(f32(gid.y) / camera.lm_scale);

    let block = get_block(i32(bx), i32(by));
    let bt = block_type(block);

    var value = vec4<f32>(0.0);

    let mat = get_material(bt);
    if mat.light_intensity > 0.0 {
        var intensity = mat.light_intensity;
        var color = vec3<f32>(mat.light_color_r, mat.light_color_g, mat.light_color_b);

        // Electric lights: OFF without power (voltage < 2V)
        if bt == 7u || bt == 10u || bt == 11u {
            let vidx = u32(by) * u32(camera.grid_w) + u32(bx);
            let lv = voltage[vidx];
            if lv < 2.0 {
                intensity = 0.0;
            } else {
                intensity *= clamp((lv - 2.0) / 6.0, 0.0, 1.0);
            }
        }

        // Fireplace: apply flicker
        if bt == 6u {
            let wx = f32(bx) + 0.5;
            let wy = f32(by) + 0.5;
            let phase = fire_hash(vec2<f32>(wx, wy)) * 6.28;
            let flicker = fire_flicker(camera.time + phase);
            intensity = FIRE_BASE_INTENSITY + (flicker - 0.5) * 2.0 * FIRE_FLICKER_AMP;
            color = mix(FIRE_COLOR, FIRE_COLOR_HOT, flicker * 0.3);
        }

        value = vec4<f32>(color, intensity);
    }

    textureStore(lightmap_out, vec2<u32>(gid.x, gid.y), value);
}
