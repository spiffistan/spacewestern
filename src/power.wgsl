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
    rain_intensity: f32, cloud_cover: f32, wind_magnitude: f32, wind_angle: f32,
};

@group(0) @binding(0) var<storage, read_write> voltage: array<f32>;
@group(0) @binding(1) var<uniform> camera: Camera;
@group(0) @binding(2) var<storage, read> grid: array<u32>;
@group(0) @binding(3) var<storage, read_write> block_temps: array<f32>;

fn block_type(b: u32) -> u32 { return b & 0xFFu; }

// Is this block part of the power network?
fn is_conductor(bt: u32, flags: u32) -> bool {
    // Wire=36, Solar=37, Battery=38/39/40, Wind=41, Electric light=7, Fan=12, Standing lamp=10
    // Also: any block with wire overlay flag (bit 7 of flags)
    let has_wire = (flags & 0x80u) != 0u;
    return bt == 36u || bt == 37u || bt == 38u || bt == 39u || bt == 40u || bt == 41u
        || bt == 7u || bt == 12u || bt == 10u || bt == 11u || bt == 16u || has_wire;
}

fn is_battery(bt: u32) -> bool {
    return bt == 38u || bt == 39u || bt == 40u;
}

// Is this block a power source?
fn is_generator(bt: u32) -> bool {
    return bt == 37u || bt == 41u; // Solar panel, Wind turbine
}

// Is this block a power consumer?
fn is_consumer(bt: u32) -> bool {
    // Electric light=7, Standing lamp=10, Table lamp=11, Fan=12, Pump=16
    return bt == 7u || bt == 10u || bt == 11u || bt == 12u || bt == 16u;
}

@compute @workgroup_size(8, 8)
fn main_power(@builtin(global_invocation_id) gid: vec3<u32>) {
    let gw = u32(camera.grid_w);
    let gh = u32(camera.grid_h);
    if gid.x >= gw || gid.y >= gh { return; }

    let idx = gid.y * gw + gid.x;
    let block = grid[idx];
    let bt = block_type(block);
    let flags = (block >> 16u) & 0xFFu;

    // Non-conductor cells: voltage = 0 (insulator)
    if !is_conductor(bt, flags) {
        voltage[idx] = 0.0;
        return;
    }

    // --- Generators: inject voltage ---
    if is_generator(bt) {
        var target_v = 0.0;
        if bt == 37u {
            // Solar panel: output from sun intensity and clouds
            let solar_output = camera.sun_intensity * (1.0 - camera.cloud_cover * 0.8);
            target_v = solar_output * 12.0;
        } else if bt == 41u {
            // Wind turbine: output from wind perpendicular to blade axis
            // Rotation stored in flags bit 6 (0x40): 0=N-S wind, 1=E-W wind
            let wt_flags = (block >> 16u) & 0xFFu;
            let wt_ew = (wt_flags & 0x40u) != 0u;
            // Compute perpendicular wind component (only positive = forward)
            let wind_x = camera.wind_magnitude * cos(camera.wind_angle);
            let wind_y = camera.wind_magnitude * sin(camera.wind_angle);
            var perp_wind = 0.0;
            if wt_ew {
                // E-W facing: wind from X axis drives it
                perp_wind = abs(wind_x);
            } else {
                // N-S facing: wind from Y axis drives it
                perp_wind = abs(wind_y);
            }
            // Cut-in at 2, rated at 12, max 12V
            let wind_factor = clamp((perp_wind - 2.0) / 10.0, 0.0, 1.0);
            target_v = wind_factor * 12.0;
        }
        let current_v = voltage[idx];
        voltage[idx] = mix(current_v, target_v, 0.3);
        return;
    }

    // --- Batteries: actual energy storage with charge/discharge dynamics ---
    // All batteries output 12V max. Capacity = how slowly they drain.
    // Charge rate: slow (takes time to fill from solar)
    // Discharge rate: moderate (responsive as power source)
    // Self-discharge: very slow (holds charge overnight)
    if is_battery(bt) {
        // Capacity scaling via charge/discharge rates
        // Small (38):  fast drain  — runs a few lights for a short while
        // Medium (39): 1.8x capacity — good for a small base
        // Large (40):  3.5x capacity — powers a full colony overnight
        var charge_rate = 0.02;       // how fast it absorbs energy
        var discharge_rate = 0.15;    // how fast it supplies energy (must keep up with consumers)
        var self_discharge = 0.99995; // per-frame voltage retention
        if bt == 39u {
            charge_rate = 0.011;
            discharge_rate = 0.083;
            self_discharge = 0.99998;
        }
        if bt == 40u {
            charge_rate = 0.006;
            discharge_rate = 0.043;
            self_discharge = 0.99999;
        }

        // Find average neighbor voltage (same scan as below but battery-specific)
        let bbx = i32(gid.x);
        let bby = i32(gid.y);
        var bneigh_sum = 0.0;
        var bneigh_count = 0.0;
        for (var bd = 0; bd < 4; bd++) {
            var bndx = 0; var bndy = 0;
            if bd == 0 { bndx = 1; } else if bd == 1 { bndx = -1; }
            else if bd == 2 { bndy = 1; } else { bndy = -1; }
            let bnx = bbx + bndx;
            let bny = bby + bndy;
            if bnx < 0 || bny < 0 || bnx >= i32(gw) || bny >= i32(gh) { continue; }
            let bnidx = u32(bny) * gw + u32(bnx);
            let bnb = grid[bnidx];
            let bnbt = block_type(bnb);
            let bnflags = (bnb >> 16u) & 0xFFu;
            if is_conductor(bnbt, bnflags) {
                bneigh_sum += voltage[bnidx];
                bneigh_count += 1.0;
            }
        }

        var bat_v = voltage[idx];
        if bneigh_count > 0.0 {
            let bavg = bneigh_sum / bneigh_count;
            if bavg > bat_v {
                // Charging: network voltage higher than battery → absorb slowly
                bat_v = mix(bat_v, bavg, charge_rate);
            } else {
                // Discharging: battery voltage higher than network → supply power
                bat_v = mix(bat_v, bavg, discharge_rate);
            }
        }

        // Self-discharge (very slow — retains ~95% overnight at 60fps)
        bat_v *= self_discharge;

        voltage[idx] = clamp(bat_v, 0.0, 12.0);
        return; // batteries handle their own relaxation, skip the generic path
    }

    // --- Consumers: draw current (reduce voltage) ---
    // Load in watts (relative units). Applied as small voltage drop per frame.
    var load = 0.0;
    if bt == 7u { load = 0.05; }   // Ceiling light: 5W
    if bt == 10u { load = 0.05; }  // Standing lamp: 5W
    if bt == 11u { load = 0.03; }  // Table lamp: 3W
    if bt == 12u { load = 0.10; }  // Fan: 10W
    if bt == 16u { load = 0.08; }  // Pump: 8W

    // --- Voltage relaxation ---
    // Wires connect to direct 4-neighbors only.
    // Consumers (lights/fans) connect to nearest wire within 3 tiles (cable).
    let bx = i32(gid.x);
    let by = i32(gid.y);
    var neighbor_sum = 0.0;
    var neighbor_count = 0.0;

    let has_wire = (flags & 0x80u) != 0u;
    if bt == 36u || has_wire {
        // Wire (standalone or overlay): direct 4-neighbor connections
        for (var d = 0; d < 4; d++) {
            var ndx = 0; var ndy = 0;
            if d == 0 { ndx = 1; } else if d == 1 { ndx = -1; }
            else if d == 2 { ndy = 1; } else { ndy = -1; }
            let nx = bx + ndx;
            let ny = by + ndy;
            if nx < 0 || ny < 0 || nx >= i32(gw) || ny >= i32(gh) { continue; }
            let nidx = u32(ny) * gw + u32(nx);
            let nb = grid[nidx];
            let nbt = block_type(nb);
            let nflags = (nb >> 16u) & 0xFFu;
            if is_conductor(nbt, nflags) {
                neighbor_sum += voltage[nidx];
                neighbor_count += 1.0;
            }
        }
    } else {
        // Consumer (light/fan): search 3-tile radius for nearest wire
        var best_dist = 100.0;
        var best_v = 0.0;
        for (var dy = -3; dy <= 3; dy++) {
            for (var dx = -3; dx <= 3; dx++) {
                let nx = bx + dx;
                let ny = by + dy;
                if nx < 0 || ny < 0 || nx >= i32(gw) || ny >= i32(gh) { continue; }
                let nidx = u32(ny) * gw + u32(nx);
                let nbt = block_type(grid[nidx]);
                if nbt == 36u { // wire
                    let dist = sqrt(f32(dx * dx + dy * dy));
                    if dist < best_dist {
                        best_dist = dist;
                        best_v = voltage[nidx];
                    }
                }
            }
        }
        if best_dist < 4.0 {
            neighbor_sum = best_v;
            neighbor_count = 1.0;
        }
    }

    if neighbor_count == 0.0 {
        // Isolated conductor: voltage decays
        voltage[idx] *= 0.9;
        return;
    }

    // Relaxation step: move toward neighbor average (fast for wires)
    let avg = neighbor_sum / neighbor_count;
    var new_v = mix(voltage[idx], avg, 0.6); // fast relaxation for wires

    // Apply consumer load (small voltage drop per frame)
    new_v -= load;
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
        let nb2 = grid[nidx2];
        let nbt2 = block_type(nb2);
        let nflags2 = (nb2 >> 16u) & 0xFFu;
        if is_conductor(nbt2, nflags2) {
            let dv = abs(voltage[nidx2] - new_v);
            total_current += dv;
        }
    }

    // I²R heating: wire resistance is low, so only significant at high currents
    let resistance = select(0.01, 0.1, bt == 36u); // wire=low, others=higher
    let heat = total_current * total_current * resistance * 0.001;
    block_temps[idx] += heat;
}
