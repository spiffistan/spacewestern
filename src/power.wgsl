// Power grid voltage relaxation compute shader.
// Simulates electrical flow through wire network using iterative Jacobi relaxation.
// Voltage propagates from generators (solar panels) through wires to consumers (lights).
// Current = voltage difference / resistance. Heat = I²R (fed into block_temps).

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
    rain_intensity: f32, cloud_cover: f32, _cam_pad0: f32, _cam_pad1: f32,
};

@group(0) @binding(0) var<storage, read_write> voltage: array<f32>;
@group(0) @binding(1) var<uniform> camera: Camera;
@group(0) @binding(2) var<storage, read> grid: array<u32>;
@group(0) @binding(3) var<storage, read_write> block_temps: array<f32>;

fn block_type(b: u32) -> u32 { return b & 0xFFu; }

// Is this block part of the power network?
fn is_conductor(bt: u32) -> bool {
    // Wire=36, Solar=37, Battery=38, Electric light=7, Fan=12
    return bt == 36u || bt == 37u || bt == 38u || bt == 7u || bt == 12u;
}

// Is this block a power source?
fn is_generator(bt: u32) -> bool {
    return bt == 37u; // Solar panel
}

// Is this block a power consumer?
fn is_consumer(bt: u32) -> bool {
    return bt == 7u || bt == 12u; // Electric light, Fan
}

@compute @workgroup_size(8, 8)
fn main_power(@builtin(global_invocation_id) gid: vec3<u32>) {
    let gw = u32(camera.grid_w);
    let gh = u32(camera.grid_h);
    if gid.x >= gw || gid.y >= gh { return; }

    let idx = gid.y * gw + gid.x;
    let block = grid[idx];
    let bt = block_type(block);

    // Non-conductor cells: voltage = 0 (insulator)
    if !is_conductor(bt) {
        voltage[idx] = 0.0;
        return;
    }

    // --- Generators: inject voltage based on sun ---
    if is_generator(bt) {
        // Solar panel: voltage proportional to sun intensity and cloud cover
        let solar_output = camera.sun_intensity * (1.0 - camera.cloud_cover * 0.8);
        let target_v = solar_output * 12.0; // max 12V at full sun
        // Gradually approach target (don't instant-set, allows network to stabilize)
        let current_v = voltage[idx];
        voltage[idx] = mix(current_v, target_v, 0.2);
        return;
    }

    // --- Consumers: draw current (reduce voltage) ---
    var load = 0.0;
    if bt == 7u { load = 0.3; }  // Electric light: moderate draw
    if bt == 12u { load = 0.5; } // Fan: higher draw

    // --- Voltage relaxation: average of connected conductor neighbors ---
    let bx = i32(gid.x);
    let by = i32(gid.y);
    var neighbor_sum = 0.0;
    var neighbor_count = 0.0;
    for (var d = 0; d < 4; d++) {
        var ndx = 0; var ndy = 0;
        if d == 0 { ndx = 1; } else if d == 1 { ndx = -1; }
        else if d == 2 { ndy = 1; } else { ndy = -1; }
        let nx = bx + ndx;
        let ny = by + ndy;
        if nx < 0 || ny < 0 || nx >= i32(gw) || ny >= i32(gh) { continue; }
        let nidx = u32(ny) * gw + u32(nx);
        let nbt = block_type(grid[nidx]);
        if is_conductor(nbt) {
            neighbor_sum += voltage[nidx];
            neighbor_count += 1.0;
        }
    }

    if neighbor_count == 0.0 {
        // Isolated conductor: voltage decays
        voltage[idx] *= 0.9;
        return;
    }

    // Relaxation step: move toward neighbor average
    let avg = neighbor_sum / neighbor_count;
    var new_v = mix(voltage[idx], avg, 0.4); // relaxation rate

    // Apply consumer load (voltage drop proportional to load)
    new_v -= load * 0.1;
    new_v = max(new_v, 0.0);

    voltage[idx] = new_v;

    // --- Heat generation from current (I²R) ---
    // Current ~ voltage difference with neighbors
    var total_current = 0.0;
    for (var d2 = 0; d2 < 4; d2++) {
        var ndx2 = 0; var ndy2 = 0;
        if d2 == 0 { ndx2 = 1; } else if d2 == 1 { ndx2 = -1; }
        else if d2 == 2 { ndy2 = 1; } else { ndy2 = -1; }
        let nx2 = bx + ndx2;
        let ny2 = by + ndy2;
        if nx2 < 0 || ny2 < 0 || nx2 >= i32(gw) || ny2 >= i32(gh) { continue; }
        let nidx2 = u32(ny2) * gw + u32(nx2);
        let nbt2 = block_type(grid[nidx2]);
        if is_conductor(nbt2) {
            let dv = abs(voltage[nidx2] - new_v);
            total_current += dv;
        }
    }

    // I²R heating: wire resistance is low, so only significant at high currents
    let resistance = select(0.01, 0.1, bt == 36u); // wire=low, others=higher
    let heat = total_current * total_current * resistance * 0.001;
    block_temps[idx] += heat;
}
