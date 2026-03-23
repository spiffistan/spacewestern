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
    sound_coupling: f32, enable_terrain_detail: f32, terrain_ao_strength: f32, fog_enabled: f32,
};

// Sound source: packed as 8 f32 per source. First f32 of buffer = source count.
// Layout per source: [x, y, amplitude, frequency, phase, pattern, duration, _pad]
// pattern: 0=impulse, 1=sine, 2=noise

@group(0) @binding(0) var sound_in: texture_2d<f32>;       // current state (R=pressure, G=velocity)
@group(0) @binding(1) var sound_out: texture_storage_2d<rg32float, write>; // next state
@group(0) @binding(2) var<storage, read> grid: array<u32>;
@group(0) @binding(3) var<uniform> camera: Camera;
@group(0) @binding(4) var<storage, read> sources: array<f32>;

fn block_type(b: u32) -> u32 { return b & 0xFFu; }
fn block_height(b: u32) -> u32 { return (b >> 8u) & 0xFFu; }
fn is_door(b: u32) -> bool { return ((b >> 16u) & 1u) != 0u; }
fn is_open(b: u32) -> bool { return ((b >> 16u) & 4u) != 0u; }

fn is_wall(x: i32, y: i32) -> bool {
    if x < 0 || y < 0 || x >= i32(camera.grid_w) || y >= i32(camera.grid_h) { return true; }
    let b = grid[u32(y) * u32(camera.grid_w) + u32(x)];
    let bt = block_type(b);
    let bh = block_height(b);
    if bh == 0u { return false; }
    // Open doors transmit sound
    if is_door(b) && is_open(b) { return false; }
    // Glass transmits some sound (partial attenuation handled elsewhere)
    if bt == BT_GLASS { return false; }
    // Pipes, restrictors, bridges, liquid pipes/equipment: height = connection mask, not wall
    if (bt >= BT_PIPE && bt <= BT_INLET) || bt == BT_RESTRICTOR || bt == BT_LIQUID_PIPE || bt == BT_PIPE_BRIDGE || bt == BT_LIQUID_INTAKE || bt == BT_LIQUID_PUMP || bt == BT_LIQUID_OUTPUT { return false; }
    // Wires, wire bridges: height = connection mask
    if bt == BT_WIRE || bt == BT_WIRE_BRIDGE { return false; }
    // Dimmer/varistor, breaker: height = level/threshold
    if bt == BT_DIMMER || bt == BT_BREAKER { return false; }
    // Fireplace: height = intensity
    if bt == BT_FIREPLACE { return false; }
    // Crates, rocks, dug ground: not real walls
    if bt == BT_DUG_GROUND || bt == BT_CRATE || bt == BT_ROCK { return false; }
    // Furniture (bench, bed, lamps): sound passes over
    if bt == BT_BENCH || bt == BT_FLOOR_LAMP || bt == BT_TABLE_LAMP || bt == BT_BED { return false; }
    return true;
}

fn read_pressure(x: i32, y: i32) -> f32 {
    if x < 0 || y < 0 || x >= i32(camera.grid_w) || y >= i32(camera.grid_h) { return 0.0; }
    if is_wall(x, y) { return 0.0; }
    return textureLoad(sound_in, vec2(x, y), 0).r;
}

// Wave speed squared and damping from camera uniform
// sound_speed = wave_speed (c), sound_damping = damping
// These are set from the CPU side

@compute @workgroup_size(8, 8)
fn main_sound(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = i32(gid.x);
    let y = i32(gid.y);
    let gw = i32(camera.grid_w);
    let gh = i32(camera.grid_h);
    if x >= gw || y >= gh { return; }

    // Wall cells: zero pressure and velocity
    if is_wall(x, y) {
        textureStore(sound_out, vec2(gid.xy), vec4(0.0, 0.0, 0.0, 0.0));
        return;
    }

    let curr = textureLoad(sound_in, vec2(x, y), 0);
    let p = curr.r;  // pressure
    let v = curr.g;  // velocity (dp/dt)

    // 5-point Laplacian of pressure
    let p_left  = read_pressure(x - 1, y);
    let p_right = read_pressure(x + 1, y);
    let p_up    = read_pressure(x, y - 1);
    let p_down  = read_pressure(x, y + 1);
    let laplacian = p_left + p_right + p_up + p_down - 4.0 * p;

    // Wave equation: velocity formulation
    let c = camera.sound_speed;  // wave speed
    let damping = camera.sound_damping;  // damping factor
    let c2 = c * c;
    var v_new = v + c2 * laplacian;
    v_new *= (1.0 - damping);  // energy loss per step
    var p_new = p + v_new;

    // Glass blocks: attenuate sound passing through (partial transmission)
    let block = grid[u32(y) * u32(camera.grid_w) + u32(x)];
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
        // Check if this source is at this grid cell
        let dx = f32(x) + 0.5 - sx;
        let dy = f32(y) + 0.5 - sy;
        let dist = dx * dx + dy * dy;
        if dist < 2.0 {  // within ~1.4 tiles
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
