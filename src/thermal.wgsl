// Thermal exchange compute shader — runs once per frame at grid resolution (256x256).
// Each block exchanges heat with adjacent air cells (from dye.a) based on material
// conductivity and heat capacity. Outdoor sunlit blocks gain solar heat.

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

struct GpuMaterial {
    color_r: f32, color_g: f32, color_b: f32, render_style: f32,
    is_solid: f32, light_transmission: f32, fluid_obstacle: f32, default_height: f32,
    light_intensity: f32, light_color_r: f32, light_color_g: f32, light_color_b: f32,
    light_radius: f32, light_height: f32, is_emissive: f32, is_furniture: f32,
    heat_capacity: f32, conductivity: f32, solar_absorption: f32, is_flammable: f32,
    ignition_temp: f32, walkable: f32, is_removable: f32, _pad: f32,
};

@group(0) @binding(0) var<storage, read_write> block_temps: array<f32>;
@group(0) @binding(1) var<uniform> camera: Camera;
@group(0) @binding(2) var<storage, read> grid: array<u32>;
@group(0) @binding(3) var<storage, read> materials: array<GpuMaterial>;
@group(0) @binding(4) var dye_tex: texture_2d<f32>;

fn block_type(b: u32) -> u32 { return b & 0xFFu; }
fn block_height(b: u32) -> u32 { return (b >> 8u) & 0xFFu; }
fn has_roof(b: u32) -> bool { return ((b >> 16u) & 2u) != 0u; }

fn get_material(bt: u32) -> GpuMaterial { return materials[min(bt, 39u)]; }

@compute @workgroup_size(8, 8)
fn main_thermal(@builtin(global_invocation_id) gid: vec3<u32>) {
    let gw = u32(camera.grid_w);
    let gh = u32(camera.grid_h);
    if gid.x >= gw || gid.y >= gh { return; }

    let idx = gid.y * gw + gid.x;
    let block = grid[idx];
    let bt = block_type(block);
    let mat = get_material(bt);
    var block_temp = block_temps[idx];

    // Skip pipe blocks (15-20) — their temperature is managed by CPU pipe network
    let is_pipe = bt >= 15u && bt <= 20u;
    if is_pipe { return; }

    // Skip air/dirt floor with no thermal mass — their temperature is the air temperature
    if mat.heat_capacity < 0.01 {
        // Read air temperature from dye texture and store as block temp
        // Scale grid coords to dye coords (dye is 2x grid resolution)
        let dye_pos = vec2<i32>(i32(gid.x) * 2 + 1, i32(gid.y) * 2 + 1);
        let air_temp = textureLoad(dye_tex, dye_pos, 0).a;
        block_temps[idx] = air_temp;
        return;
    }

    // --- Solar heating ---
    // Outdoor blocks (no roof) gain heat from sunlight based on sun intensity and absorption.
    // Rate is slow — stone takes many game-minutes to warm significantly.
    if !has_roof(block) && block_height(block) > 0u {
        let solar_heat = camera.sun_intensity * mat.solar_absorption * 0.005; // ~0.3°C/sec for stone
        block_temp += solar_heat;
    }

    // --- Radiative cooling ---
    // All blocks radiate heat toward ambient temperature (15°C).
    // Rate is slower for high thermal mass (stone retains heat longer).
    let ambient = 15.0;
    let cool_rate = 0.002 / max(mat.heat_capacity, 0.5); // high capacity = slow cooling
    block_temp += (ambient - block_temp) * cool_rate;

    // --- Heat exchange with adjacent air ---
    // Sample air temperature from dye texture at this block's position
    let dye_pos = vec2<i32>(i32(gid.x) * 2 + 1, i32(gid.y) * 2 + 1);
    let air_temp = textureLoad(dye_tex, dye_pos, 0).a;

    // Heat transfer: block ↔ air based on conductivity and heat capacity
    // Higher capacity = more thermal inertia = slower to change
    let temp_diff = air_temp - block_temp;
    let transfer_rate = mat.conductivity / max(mat.heat_capacity, 0.5);
    block_temp += temp_diff * transfer_rate * 0.3;

    // --- Heat conduction between adjacent blocks ---
    let bx = i32(gid.x);
    let by = i32(gid.y);
    var neighbor_heat = 0.0;
    var neighbor_count = 0.0;
    for (var dy: i32 = -1; dy <= 1; dy++) {
        for (var dx: i32 = -1; dx <= 1; dx++) {
            if dx == 0 && dy == 0 { continue; }
            let nx = bx + dx;
            let ny = by + dy;
            if nx < 0 || ny < 0 || nx >= i32(gw) || ny >= i32(gh) { continue; }
            let nidx = u32(ny) * gw + u32(nx);
            let nb = grid[nidx];
            let nmat = get_material(block_type(nb));
            if nmat.heat_capacity > 0.01 {
                neighbor_heat += block_temps[nidx];
                neighbor_count += 1.0;
            }
        }
    }
    if neighbor_count > 0.0 {
        let avg_neighbor = neighbor_heat / neighbor_count;
        block_temp += (avg_neighbor - block_temp) * transfer_rate * 0.2;
    }

    // Clamp temperature
    block_temp = clamp(block_temp, -30.0, 600.0);

    block_temps[idx] = block_temp;
}
