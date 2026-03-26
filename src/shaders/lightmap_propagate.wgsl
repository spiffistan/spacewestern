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
    sound_coupling: f32, enable_terrain_detail: f32, terrain_ao_strength: f32, fog_enabled: f32, hover_x: f32, hover_y: f32,
};

@group(0) @binding(0) var lightmap_in: texture_2d<f32>;
@group(0) @binding(1) var lightmap_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> camera: Camera;
@group(0) @binding(3) var<storage, read> grid: array<u32>;
@group(0) @binding(4) var<storage, read> materials: array<GpuMaterial>;
@group(0) @binding(5) var<storage, read> wall_buf: array<u32>;

struct GpuMaterial {
    color_r: f32, color_g: f32, color_b: f32, render_style: f32,
    is_solid: f32, light_transmission: f32, fluid_obstacle: f32, default_height: f32,
    light_intensity: f32, light_color_r: f32, light_color_g: f32, light_color_b: f32,
    light_radius: f32, light_height: f32, is_emissive: f32, is_furniture: f32,
    heat_capacity: f32, conductivity: f32, solar_absorption: f32, is_flammable: f32,
    ignition_temp: f32, walkable: f32, is_removable: f32, _pad: f32,
};

fn get_material(bt: u32) -> GpuMaterial { return materials[min(bt, 61u)]; }

// --- Block unpacking ---
fn block_type(b: u32) -> u32 { return b & 0xFFu; }
fn block_height(b: u32) -> u32 {
    let h = (b >> 8u) & 0xFFu;
    let bt = b & 0xFFu;
    // Wall blocks: bits 4-7 of height = edge bitmask, not visual height
    if bt == 1u || bt == 4u || bt == 5u || bt == 14u || (bt >= 21u && bt <= 25u) || bt == 35u || bt == 44u { return h & 0xFu; }
    return h;
}
fn is_door(b: u32) -> bool { return ((b >> 16u) & 1u) != 0u; }
fn is_open(b: u32) -> bool { return ((b >> 16u) & 4u) != 0u; }

fn get_block(x: i32, y: i32) -> u32 {
    if x < 0 || y < 0 || x >= i32(camera.grid_w) || y >= i32(camera.grid_h) {
        return 0u;
    }
    return grid[u32(y) * u32(camera.grid_w) + u32(x)];
}

// --- Wall data helpers (DN-008) ---
fn read_wall_data(idx: u32) -> u32 {
    let word = wall_buf[idx >> 1u];
    if (idx & 1u) == 0u { return word & 0xFFFFu; } else { return (word >> 16u) & 0xFFFFu; }
}
fn wd_edges(wd: u32) -> u32 { return wd & 0xFu; }
fn wd_has_edge(wd: u32, edge: u32) -> bool { return (wd & (1u << edge)) != 0u; }

// Check if wall_data blocks edge crossing from (ax,ay) to (bx,by)
fn wd_edge_blocked(ax: i32, ay: i32, bx: i32, by: i32) -> bool {
    let dx = bx - ax;
    let dy = by - ay;
    var dir_a = 0u;
    if dy < 0 { dir_a = 0u; }
    else if dx > 0 { dir_a = 1u; }
    else if dy > 0 { dir_a = 2u; }
    else { dir_a = 3u; }
    let dir_b = (dir_a + 2u) % 4u;
    let gw = u32(camera.grid_w);
    // Check tile A's wall_data for outgoing edge
    if ax >= 0 && ay >= 0 && ax < i32(camera.grid_w) && ay < i32(camera.grid_h) {
        let a_wd = read_wall_data(u32(ay) * gw + u32(ax));
        if wd_has_edge(a_wd, dir_a) { return true; }
    }
    // Check tile B's wall_data for incoming edge
    if bx >= 0 && by >= 0 && bx < i32(camera.grid_w) && by < i32(camera.grid_h) {
        let b_wd = read_wall_data(u32(by) * gw + u32(bx));
        if wd_has_edge(b_wd, dir_b) { return true; }
    }
    return false;
}

// --- Thin wall helpers ---
fn wall_thickness_raw(flags: u32) -> u32 { return (flags >> 5u) & 3u; }
fn is_thin_wall_block(b: u32) -> bool {
    let bh = block_height(b);
    if bh == 0u { return false; }
    return wall_thickness_raw((b >> 16u) & 0xFFu) != 0u;
}
fn has_wall_on_edge(height: u32, flags: u32, edge: u32) -> bool {
    let thick_raw = (flags >> 5u) & 3u;
    if thick_raw == 0u { return true; } // full wall
    let mask = (height >> 4u) & 0xFu;
    if mask == 0u { return true; } // no edges = full wall (backward compat)
    return (mask & (1u << edge)) != 0u;
}

// Edge-blocked: is the crossing from (ax,ay) to (bx,by) blocked by a thin wall?
fn edge_blocked_lm(ax: i32, ay: i32, bx: i32, by: i32) -> bool {
    // Check wall_data layer first (DN-008)
    if wd_edge_blocked(ax, ay, bx, by) { return true; }

    // Fall back to block grid (legacy)
    let dx = bx - ax;
    let dy = by - ay;
    var dir_a = 0u;
    if dy < 0 { dir_a = 0u; }
    else if dx > 0 { dir_a = 1u; }
    else if dy > 0 { dir_a = 2u; }
    else { dir_a = 3u; }
    let dir_b = (dir_a + 2u) % 4u;

    let a_block = get_block(ax, ay);
    let a_bt = block_type(a_block);
    let a_flags = (a_block >> 16u) & 0xFFu;
    let a_bh = block_height(a_block);
    let a_mat = get_material(a_bt);
    if a_bh > 0u && a_mat.is_solid > 0.5 && a_mat.light_transmission < 0.01 {
        if !((a_flags & 1u) != 0u && (a_flags & 4u) != 0u) { // not open door
            if has_wall_on_edge(a_bh, a_flags, dir_a) { return true; }
        }
    }

    let b_block = get_block(bx, by);
    let b_bt = block_type(b_block);
    let b_flags = (b_block >> 16u) & 0xFFu;
    let b_bh = block_height(b_block);
    let b_mat = get_material(b_bt);
    if b_bh > 0u && b_mat.is_solid > 0.5 && b_mat.light_transmission < 0.01 {
        if !((b_flags & 1u) != 0u && (b_flags & 4u) != 0u) {
            if has_wall_on_edge(b_bh, b_flags, dir_b) { return true; }
        }
    }

    return false;
}

// Is this block a solid wall that blocks light propagation?
fn is_wall(b: u32) -> bool {
    let bh = block_height(b);
    if bh == 0u { return false; }
    if is_door(b) && is_open(b) { return false; }
    // Thin walls: not fully blocking (handled by edge_blocked)
    if is_thin_wall_block(b) { return false; }
    let mat = get_material(block_type(b));
    if mat.light_transmission > 0.01 { return false; }
    return mat.is_solid > 0.5;
}
// Base propagation falloff per grid cell (scaled by lm_scale for per-texel step)
const BASE_FALLOFF: f32 = 0.08;

// Try to take light from a neighbor at lightmap texel (ntx, nty).
// cur_bx, cur_by = current tile's block coords (for edge blocking check).
// Returns the attenuated value, or zero if blocked.
fn sample_neighbor(ntx: i32, nty: i32, cur_bx: i32, cur_by: i32, falloff: f32, lm_w: i32, lm_h: i32) -> vec4<f32> {
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
    // Edge blocking: thin wall between current tile and neighbor
    if nbx != cur_bx || nby != cur_by {
        if edge_blocked_lm(cur_bx, cur_by, nbx, nby) {
            return vec4<f32>(0.0);
        }
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
    let n0 = sample_neighbor(tx + 1, ty, bx, by, prop_falloff, lm_w, lm_h);
    let n1 = sample_neighbor(tx - 1, ty, bx, by, prop_falloff, lm_w, lm_h);
    let n2 = sample_neighbor(tx, ty + 1, bx, by, prop_falloff, lm_w, lm_h);
    let n3 = sample_neighbor(tx, ty - 1, bx, by, prop_falloff, lm_w, lm_h);

    if n0.w > best.w { best = n0; }
    if n1.w > best.w { best = n1; }
    if n2.w > best.w { best = n2; }
    if n3.w > best.w { best = n3; }

    // Diagonal neighbors (sqrt(2) falloff for circular spread)
    let d0 = sample_neighbor(tx + 1, ty + 1, bx, by, diag_falloff, lm_w, lm_h);
    let d1 = sample_neighbor(tx - 1, ty + 1, bx, by, diag_falloff, lm_w, lm_h);
    let d2 = sample_neighbor(tx + 1, ty - 1, bx, by, diag_falloff, lm_w, lm_h);
    let d3 = sample_neighbor(tx - 1, ty - 1, bx, by, diag_falloff, lm_w, lm_h);

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
