// Lightmap compute shader — two-pass iterative light propagation.
// Pass 1 (seed): Write light source values at fire/electric light positions.
// Pass 2 (propagate): Flood-fill light through open tiles, stop at walls.
//   Runs N iterations ping-ponging between two textures.
// Output: Rgba16Float texture where RGB = light color, A = light intensity.
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
    _pad1: f32,
};

// --- Seed pass bindings ---
@group(0) @binding(0) var lightmap_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(1) var<uniform> camera: Camera;
@group(0) @binding(2) var<storage, read> grid: array<u32>;

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
@compute @workgroup_size(8, 8)
fn main_lightmap_seed(@builtin(global_invocation_id) gid: vec3<u32>) {
    let bx = gid.x;
    let by = gid.y;

    if bx >= u32(camera.grid_w) || by >= u32(camera.grid_h) {
        return;
    }

    let block = get_block(i32(bx), i32(by));
    let bt = block_type(block);

    var value = vec4<f32>(0.0);

    if bt == 6u {
        // Fireplace
        let wx = f32(bx) + 0.5;
        let wy = f32(by) + 0.5;
        let phase = fire_hash(vec2<f32>(wx, wy)) * 6.28;
        let flicker = fire_flicker(camera.time + phase);
        let intensity = FIRE_BASE_INTENSITY + (flicker - 0.5) * 2.0 * FIRE_FLICKER_AMP;
        let color = mix(FIRE_COLOR, FIRE_COLOR_HOT, flicker * 0.3);
        value = vec4<f32>(color, intensity);
    } else if bt == 7u {
        // Electric light
        value = vec4<f32>(ELIGHT_COLOR, ELIGHT_INTENSITY);
    } else if bt == 10u {
        // Standing lamp: large warm glow
        value = vec4<f32>(0.95, 0.85, 0.60, 1.0);
    } else if bt == 11u {
        // Table lamp: smaller warm glow
        value = vec4<f32>(0.95, 0.80, 0.50, 0.35);
    }

    textureStore(lightmap_out, vec2<u32>(bx, by), value);
}
