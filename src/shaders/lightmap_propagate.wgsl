// Lightmap propagation pass — iterative flood-fill with viewport culling.
// Reads from source lightmap, writes to destination lightmap.
// Each open cell takes the brightest neighbor minus falloff.
// Walls block propagation. Glass attenuates.
// Works at lightmap resolution (lm_scale × grid resolution).
// Threads outside the viewport+margin region early-return for efficiency.

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
    use_shadow_map: f32, shadow_map_scale: f32, sound_speed: f32, sound_damping: f32,
    sound_coupling: f32, enable_terrain_detail: f32, terrain_ao_strength: f32, _pad4_c: f32,
};

@group(0) @binding(0) var lightmap_in: texture_2d<f32>;
@group(0) @binding(1) var lightmap_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> camera: Camera;
@group(0) @binding(3) var<storage, read> grid: array<u32>;
@group(0) @binding(4) var<storage, read> materials: array<GpuMaterial>;

struct GpuMaterial {
    color_r: f32, color_g: f32, color_b: f32, render_style: f32,
    is_solid: f32, light_transmission: f32, fluid_obstacle: f32, default_height: f32,
    light_intensity: f32, light_color_r: f32, light_color_g: f32, light_color_b: f32,
    light_radius: f32, light_height: f32, is_emissive: f32, is_furniture: f32,
    heat_capacity: f32, conductivity: f32, solar_absorption: f32, is_flammable: f32,
    ignition_temp: f32, walkable: f32, is_removable: f32, _pad: f32,
};

fn get_material(bt: u32) -> GpuMaterial { return materials[min(bt, 54u)]; }

// --- Block unpacking ---
fn block_type(b: u32) -> u32 { return b & 0xFFu; }
fn block_height(b: u32) -> u32 { return (b >> 8u) & 0xFFu; }
fn is_door(b: u32) -> bool { return ((b >> 16u) & 1u) != 0u; }
fn is_open(b: u32) -> bool { return ((b >> 16u) & 4u) != 0u; }

fn get_block(x: i32, y: i32) -> u32 {
    if x < 0 || y < 0 || x >= i32(camera.grid_w) || y >= i32(camera.grid_h) {
        return 0u;
    }
    return grid[u32(y) * u32(camera.grid_w) + u32(x)];
}

// Is this block a solid wall that blocks light propagation?
// Uses material properties: solid blocks with no light transmission are walls.
// Open doors always transmit.
fn is_wall(b: u32) -> bool {
    let bh = block_height(b);
    if bh == 0u { return false; }
    if is_door(b) && is_open(b) { return false; }
    let mat = get_material(block_type(b));
    // Blocks with any light transmission are not walls (glass, trees)
    if mat.light_transmission > 0.01 { return false; }
    return mat.is_solid > 0.5;
}
// Base propagation falloff per grid cell (scaled by lm_scale for per-texel step)
const BASE_FALLOFF: f32 = 0.08;

// Try to take light from a neighbor at lightmap texel (ntx, nty).
// Returns the attenuated value, or zero if blocked.
fn sample_neighbor(ntx: i32, nty: i32, falloff: f32, lm_w: i32, lm_h: i32) -> vec4<f32> {
    if ntx < 0 || nty < 0 || ntx >= lm_w || nty >= lm_h {
        return vec4<f32>(0.0);
    }
    // Convert texel to block coordinates for grid lookup
    let nbx = i32(f32(ntx) / camera.lm_scale);
    let nby = i32(f32(nty) / camera.lm_scale);
    let nb = get_block(nbx, nby);
    // Can't receive light from a solid wall
    if is_wall(nb) {
        return vec4<f32>(0.0);
    }
    let nval = textureLoad(lightmap_in, vec2<i32>(ntx, nty), 0);
    var intensity = nval.w - falloff;
    // Apply material light transmission (glass=0.4, tree=0.5, etc.)
    let nmat = get_material(block_type(nb));
    if nmat.light_transmission > 0.01 && nmat.light_transmission < 0.99 {
        intensity *= nmat.light_transmission;
    }
    if intensity <= 0.0 {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(nval.xyz, intensity);
}

@compute @workgroup_size(8, 8)
fn main_lightmap_propagate(@builtin(global_invocation_id) gid: vec3<u32>) {
    let lm_w = i32(camera.grid_w * camera.lm_scale);
    let lm_h = i32(camera.grid_h * camera.lm_scale);
    let tx = i32(gid.x);
    let ty = i32(gid.y);

    if tx >= lm_w || ty >= lm_h {
        return;
    }

    // Viewport culling: skip propagation outside the viewport+margin region.
    // Both textures were seeded with clean values, so untouched texels are correct.
    let vp_min_tx = i32(camera.lm_vp_min_x * camera.lm_scale);
    let vp_min_ty = i32(camera.lm_vp_min_y * camera.lm_scale);
    let vp_max_tx = i32(camera.lm_vp_max_x * camera.lm_scale);
    let vp_max_ty = i32(camera.lm_vp_max_y * camera.lm_scale);

    if tx < vp_min_tx || tx >= vp_max_tx || ty < vp_min_ty || ty >= vp_max_ty {
        return;
    }

    // Convert texel to block coordinates
    let bx = i32(f32(tx) / camera.lm_scale);
    let by = i32(f32(ty) / camera.lm_scale);
    let block = get_block(bx, by);
    let bt = block_type(block);

    // Walls stay at zero — light doesn't enter them
    if is_wall(block) {
        textureStore(lightmap_out, vec2<u32>(gid.xy), vec4<f32>(0.0));
        return;
    }

    // Light sources always keep their seed value
    if get_material(bt).light_intensity > 0.0 {
        let self_val = textureLoad(lightmap_in, vec2<i32>(tx, ty), 0);
        textureStore(lightmap_out, vec2<u32>(gid.xy), self_val);
        return;
    }

    // Falloff scaled by lightmap resolution (finer steps = less falloff per texel)
    let prop_falloff = BASE_FALLOFF / camera.lm_scale;
    let diag_falloff = prop_falloff * 1.414;

    // Start with own current value
    var best = textureLoad(lightmap_in, vec2<i32>(tx, ty), 0);

    // Cardinal neighbors
    let n0 = sample_neighbor(tx + 1, ty, prop_falloff, lm_w, lm_h);
    let n1 = sample_neighbor(tx - 1, ty, prop_falloff, lm_w, lm_h);
    let n2 = sample_neighbor(tx, ty + 1, prop_falloff, lm_w, lm_h);
    let n3 = sample_neighbor(tx, ty - 1, prop_falloff, lm_w, lm_h);

    if n0.w > best.w { best = n0; }
    if n1.w > best.w { best = n1; }
    if n2.w > best.w { best = n2; }
    if n3.w > best.w { best = n3; }

    // Diagonal neighbors (sqrt(2) falloff for circular spread)
    let d0 = sample_neighbor(tx + 1, ty + 1, diag_falloff, lm_w, lm_h);
    let d1 = sample_neighbor(tx - 1, ty + 1, diag_falloff, lm_w, lm_h);
    let d2 = sample_neighbor(tx + 1, ty - 1, diag_falloff, lm_w, lm_h);
    let d3 = sample_neighbor(tx - 1, ty - 1, diag_falloff, lm_w, lm_h);

    if d0.w > best.w { best = d0; }
    if d1.w > best.w { best = d1; }
    if d2.w > best.w { best = d2; }
    if d3.w > best.w { best = d3; }

    // Blocks with partial light transmission attenuate light passing through
    let self_mat = get_material(bt);
    if self_mat.light_transmission > 0.01 && self_mat.light_transmission < 0.99 {
        best = vec4<f32>(best.xyz, best.w * self_mat.light_transmission);
    }

    textureStore(lightmap_out, vec2<u32>(gid.xy), max(best, vec4<f32>(0.0)));
}
