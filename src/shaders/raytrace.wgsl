// Rayworld — pixel-level top-down raytrace compute shader
// Features:
//   - Sub-block shadow ray marching (pixel resolution, not block resolution)
//   - Glass blocks: partial absorption + color tinting of transmitted light
//   - Roofed buildings: opaque roof surface hides interior and walls from above
//   - Height-based 3D shadow casting from directional sunlight

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
    shadow_intensity: f32,
    pleb_scale: f32,
    contour_opacity: f32,
    contour_interval: f32,
    contour_major_mul: f32, water_table_offset: f32, aim_mode: f32,
};

@group(0) @binding(0) var output: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<uniform> camera: Camera;
@group(0) @binding(2) var<storage, read> grid: array<u32>;
@group(0) @binding(3) var lightmap_tex: texture_2d<f32>;
@group(0) @binding(4) var lightmap_sampler: sampler;
@group(0) @binding(5) var<storage, read> sprites: array<u32>;
@group(0) @binding(6) var fluid_dye_tex: texture_2d<f32>;
@group(0) @binding(7) var fluid_dye_sampler: sampler;
@group(0) @binding(8) var fluid_vel_tex: texture_2d<f32>;
@group(0) @binding(9) var fluid_pres_tex: texture_2d<f32>;
@group(0) @binding(10) var prev_output: texture_2d<f32>;
@group(0) @binding(11) var<storage, read> materials: array<GpuMaterial>;
@group(0) @binding(12) var<storage, read> plebs: array<GpuPleb>;
@group(0) @binding(13) var<storage, read> block_temps: array<f32>;
@group(0) @binding(14) var<storage, read> voltage: array<f32>;
@group(0) @binding(15) var<storage, read> pipe_flow: array<f32>;
@group(0) @binding(16) var water_tex: texture_2d<f32>;
@group(0) @binding(17) var<storage, read> water_table_buf: array<f32>;
@group(0) @binding(18) var shadow_map_tex: texture_2d<f32>;
@group(0) @binding(19) var sound_tex: texture_2d<f32>;
@group(0) @binding(20) var<storage, read> elevation_buf: array<f32>;
@group(0) @binding(21) var fog_tex: texture_2d<f32>;
@group(0) @binding(22) var fog_sampler: sampler;
@group(0) @binding(23) var<storage, read> terrain_buf: array<u32>;
@group(0) @binding(24) var<storage, read> wall_buf: array<u32>; // u16 packed as u32 pairs
@group(0) @binding(25) var<storage, read> door_buf: array<u32>; // [count, w0, angle, w0, angle, ...]

// Alien fauna
struct GpuCreature {
    x: f32, y: f32, angle: f32, health: f32,
    color_r: f32, color_g: f32, color_b: f32, body_radius: f32,
    hop_offset: f32, eye_r: f32, eye_g: f32, eye_b: f32,
};
const MAX_CREATURES: u32 = 32u;
@group(0) @binding(26) var<storage, read> creature_buf: array<GpuCreature>;
@group(0) @binding(27) var dust_tex: texture_2d<f32>;
@group(0) @binding(28) var elevation_tex: texture_2d<f32>;

// Bush sprites packed after tree sprites in the same buffer
const BUSH_SPRITE_SIZE: u32 = 64u;
const BUSH_SPRITE_VARIANTS: u32 = 16u;
const BUSH_OFFSET: u32 = SPRITE_VARIANTS * SPRITE_SIZE * SPRITE_SIZE; // where bushes start

fn sample_bush_sprite(variant: u32, fx: f32, fy: f32) -> vec4<f32> {
    let lx = clamp(u32(fx * f32(BUSH_SPRITE_SIZE)), 0u, BUSH_SPRITE_SIZE - 1u);
    let ly = clamp(u32(fy * f32(BUSH_SPRITE_SIZE)), 0u, BUSH_SPRITE_SIZE - 1u);
    let idx = BUSH_OFFSET + variant * BUSH_SPRITE_SIZE * BUSH_SPRITE_SIZE + ly * BUSH_SPRITE_SIZE + lx;
    let packed = sprites[idx]; // same buffer as trees
    let r = f32(packed & 0xFFu) / 255.0;
    let g = f32((packed >> 8u) & 0xFFu) / 255.0;
    let b = f32((packed >> 16u) & 0xFFu) / 255.0;
    let h = f32((packed >> 24u) & 0xFFu) / 255.0;
    return vec4(r, g, b, h);
}

// Rock sprites packed after bushes
const ROCK_SPRITE_SIZE: u32 = 64u;
const ROCK_SPRITE_VARIANTS: u32 = 32u;
const ROCK_OFFSET: u32 = BUSH_OFFSET + BUSH_SPRITE_VARIANTS * BUSH_SPRITE_SIZE * BUSH_SPRITE_SIZE;

fn sample_rock_sprite(variant: u32, fx: f32, fy: f32) -> vec4<f32> {
    let lx = clamp(u32(fx * f32(ROCK_SPRITE_SIZE)), 0u, ROCK_SPRITE_SIZE - 1u);
    let ly = clamp(u32(fy * f32(ROCK_SPRITE_SIZE)), 0u, ROCK_SPRITE_SIZE - 1u);
    let idx = ROCK_OFFSET + variant * ROCK_SPRITE_SIZE * ROCK_SPRITE_SIZE + ly * ROCK_SPRITE_SIZE + lx;
    let packed = sprites[idx];
    let r = f32(packed & 0xFFu) / 255.0;
    let g = f32((packed >> 8u) & 0xFFu) / 255.0;
    let b = f32((packed >> 16u) & 0xFFu) / 255.0;
    let h = f32((packed >> 24u) & 0xFFu) / 255.0;
    return vec4(r, g, b, h);
}

// --- Wall data helpers (DN-008 wall edge layer) ---
// wall_buf stores u16 per tile packed as u32 (two tiles per u32 entry).
// Read a single u16 wall_data value for tile at grid index idx.
fn read_wall_data(idx: u32) -> u32 {
    let word = wall_buf[idx / 2u];
    if (idx & 1u) == 0u {
        return word & 0xFFFFu;
    } else {
        return (word >> 16u) & 0xFFFFu;
    }
}

// Wall data bit layout:
// bits 0-3: edge mask (bit0=N, bit1=E, bit2=S, bit3=W)
// bits 4-5: thickness raw (0=full, 1→3, 2→2, 3→1)
// bits 6-9: material index
// bit 10: has_door
// bit 11: door_open
// bit 12: has_window
fn wd_edges_s(wd: u32) -> u32 { return wd & 0xFu; }
fn wd_thickness_raw_s(wd: u32) -> u32 { return (wd >> 4u) & 3u; }
fn wd_thickness_s(wd: u32) -> u32 {
    let raw = wd_thickness_raw_s(wd);
    return select(4u - raw, 4u, raw == 0u);
}
fn wd_material_s(wd: u32) -> u32 { return (wd >> 6u) & 0xFu; }
fn wd_has_door(wd: u32) -> bool { return (wd & 0x400u) != 0u; }
fn wd_door_open(wd: u32) -> bool { return (wd & 0x800u) != 0u; }
fn wd_has_window(wd: u32) -> bool { return (wd & 0x1000u) != 0u; }

// Check if wall_data has a wall on given edge
fn wd_has_edge_s(wd: u32, edge: u32) -> bool {
    if wd == 0u { return false; }
    let edges = wd_edges_s(wd);
    if edges == 0u && wd_thickness_raw_s(wd) == 0u { return true; } // full wall compat
    return (edges & (1u << edge)) != 0u;
}

// Check if pixel is in the wall area using wall_data
fn wd_pixel_is_wall(fx: f32, fy: f32, wd: u32) -> bool {
    if wd == 0u { return false; }
    // Open doors are not walls
    if wd_has_door(wd) && wd_door_open(wd) { return false; }
    let thick = wd_thickness_s(wd);
    if thick >= 4u {
        let edges = wd_edges_s(wd);
        if edges == 0u { return true; } // full wall compat
    }
    let wall_frac = f32(thick) * 0.25;
    let edges = wd_edges_s(wd);
    if edges == 0u && wd_thickness_raw_s(wd) == 0u { return true; }
    var hit = false;
    if (edges & 1u) != 0u && edge_covers_pixel(fx, fy, 0u, wall_frac) { hit = true; }
    if (edges & 2u) != 0u && edge_covers_pixel(fx, fy, 1u, wall_frac) { hit = true; }
    if (edges & 4u) != 0u && edge_covers_pixel(fx, fy, 2u, wall_frac) { hit = true; }
    if (edges & 8u) != 0u && edge_covers_pixel(fx, fy, 3u, wall_frac) { hit = true; }
    return hit;
}

// --- Physical door rendering (DN-009) ---
// Door data: door_buf[0] = count, then pairs of (packed_w0, angle_bits)
struct DoorInfo {
    edge: u32,
    hinge_side: u32,
    angle: f32,
    material: u32,
    found: bool,
};

fn find_door(tx: u32, ty: u32) -> DoorInfo {
    let count = min(door_buf[0], 64u);
    for (var i = 0u; i < count; i++) {
        let w0 = door_buf[1u + i * 2u];
        let dx = w0 & 0xFFu;
        let dy = (w0 >> 8u) & 0xFFu;
        if dx == tx && dy == ty {
            return DoorInfo(
                (w0 >> 16u) & 3u,
                (w0 >> 18u) & 1u,
                bitcast<f32>(door_buf[2u + i * 2u]),
                (w0 >> 20u) & 0xFu,
                true
            );
        }
    }
    return DoorInfo(0u, 0u, 0.0, 0u, false);
}

// Door geometry constants
const DOOR_JAMB_FRAC: f32 = 0.15;  // each jamb = 15% of tile width
const DOOR_GAP_FRAC: f32 = 0.70;   // doorway = 70% of tile width
const DOOR_LEAF_THICK: f32 = 0.06;  // leaf thickness

// Render a door pixel. Returns vec4(color, 1.0) if on door/jamb, vec4(0) if gap.
fn render_door(fx: f32, fy: f32, wd: u32, door: DoorInfo) -> vec4<f32> {
    let wall_frac = f32(wd_thickness_s(wd)) * 0.25;
    let edge = door.edge;

    // Transform to edge-local: u=along edge(0..1), v=perpendicular (0=edge, positive=into tile)
    var u: f32; var v: f32;
    if edge == 0u { u = fx; v = fy; }                       // N: wall at top
    else if edge == 1u { u = fy; v = 1.0 - fx; }            // E: wall at right
    else if edge == 2u { u = 1.0 - fx; v = 1.0 - fy; }     // S: wall at bottom
    else { u = 1.0 - fy; v = fx; }                          // W: wall at left

    let wall_color = wall_material_color(wd_material_s(wd));
    let door_color = vec3<f32>(0.45, 0.32, 0.18); // wood brown

    // Jambs: solid wall material at edges (only within wall strip)
    if v <= wall_frac && (u < DOOR_JAMB_FRAC || u > (1.0 - DOOR_JAMB_FRAC)) {
        return vec4<f32>(wall_color, 1.0);
    }

    // Door leaf: rotated line from hinge — extends beyond wall strip when open
    let gap_start = DOOR_JAMB_FRAC;
    let gap_width = DOOR_GAP_FRAC;
    let hinge_u = select(gap_start, gap_start + gap_width, door.hinge_side == 1u);
    let hinge_v = wall_frac * 0.5;

    let swing = select(1.0, -1.0, door.hinge_side == 1u);
    let leaf_end_u = hinge_u + swing * cos(door.angle) * gap_width;
    let leaf_end_v = hinge_v + sin(door.angle) * gap_width;

    // Distance from pixel to line segment (hinge → leaf_end)
    let seg_du = leaf_end_u - hinge_u;
    let seg_dv = leaf_end_v - hinge_v;
    let seg_len_sq = seg_du * seg_du + seg_dv * seg_dv;
    let pu = u - hinge_u;
    let pv = v - hinge_v;
    let t = clamp((pu * seg_du + pv * seg_dv) / max(seg_len_sq, 0.0001), 0.0, 1.0);
    let closest_u = hinge_u + t * seg_du;
    let closest_v = hinge_v + t * seg_dv;
    let dist = length(vec2<f32>(u - closest_u, v - closest_v));

    if dist < DOOR_LEAF_THICK * 0.5 {
        let edge_t = abs(t - 0.5) * 2.0;
        let leaf_color = door_color * (0.85 + 0.15 * (1.0 - edge_t));
        return vec4<f32>(leaf_color, 1.0);
    }

    // Beyond wall strip and not on leaf = not a door pixel
    if v > wall_frac { return vec4<f32>(0.0); }

    // In the gap (floor visible)
    return vec4<f32>(0.0);
}

// Wall material color table (indexed by wd_material_s)
fn wall_material_color(mat_idx: u32) -> vec3<f32> {
    switch mat_idx {
        case 0u: { return vec3<f32>(0.52, 0.50, 0.48); }  // stone
        case 1u: { return vec3<f32>(0.58, 0.56, 0.52); }  // generic wall
        case 2u: { return vec3<f32>(0.65, 0.78, 0.88); }  // glass
        case 3u: { return vec3<f32>(0.70, 0.68, 0.65); }  // insulated
        case 4u: { return vec3<f32>(0.50, 0.38, 0.22); }  // wood
        case 5u: { return vec3<f32>(0.55, 0.57, 0.60); }  // steel
        case 6u: { return vec3<f32>(0.62, 0.58, 0.50); }  // sandstone
        case 7u: { return vec3<f32>(0.50, 0.48, 0.46); }  // granite
        case 8u: { return vec3<f32>(0.65, 0.63, 0.58); }  // limestone
        case 9u: { return vec3<f32>(0.52, 0.40, 0.25); }  // mud
        default: { return vec3<f32>(0.55, 0.53, 0.50); }  // fallback
    }
}

// Wall material height (for shadow casting)
fn wall_material_height(mat_idx: u32) -> f32 {
    if mat_idx == 2u { return 3.0; } // glass
    return 3.0; // all walls default to height 3
}

// --- Fog of war helper (bilinear-sampled for smooth edges) ---
fn sample_fog(wx: f32, wy: f32) -> f32 {
    // Map world coords to UV (center of each tile = center of texel)
    let uv = vec2<f32>((wx + 0.5) / camera.grid_w, (wy + 0.5) / camera.grid_h);
    return textureSampleLevel(fog_tex, fog_sampler, uv, 0.0).r;
}

fn apply_fog(color: vec3<f32>, wx: f32, wy: f32) -> vec3<f32> {
    if camera.fog_enabled < 0.5 { return color; }
    let fog = sample_fog(wx, wy);
    if fog < 0.01 {
        return vec3(0.0); // shrouded
    } else if fog < 0.5 {
        // explored: desaturate + dim, fade smoothly at edges
        let t = fog / 0.5; // 0 at deep explored, 1 at visible edge
        let gray = dot(color, vec3(0.299, 0.587, 0.114));
        let dimmed = mix(vec3(gray), color, 0.3) * 0.35;
        return mix(dimmed, color, t * t); // smooth transition
    }
    return color; // visible
}

// --- Pleb struct (must match Rust GpuPleb layout exactly) ---
struct GpuPleb {
    x: f32, y: f32, angle: f32, selected: f32,
    torch: f32, headlight: f32, carrying: f32, health: f32,
    skin_r: f32, skin_g: f32, skin_b: f32, hair_style: f32,
    hair_r: f32, hair_g: f32, hair_b: f32, aim_progress: f32,
    shirt_r: f32, shirt_g: f32, shirt_b: f32, weapon_type: f32,
    pants_r: f32, pants_g: f32, pants_b: f32, swing_progress: f32,
    crouch: f32, stress: f32, _pad2: f32, _pad3: f32,
};

const MAX_PLEBS: u32 = 16u;

// --- Material struct (must match Rust GpuMaterial layout exactly) ---
struct GpuMaterial {
    color_r: f32, color_g: f32, color_b: f32,
    render_style: f32,
    is_solid: f32,
    light_transmission: f32,
    fluid_obstacle: f32,
    default_height: f32,
    light_intensity: f32,
    light_color_r: f32, light_color_g: f32, light_color_b: f32,
    light_radius: f32,
    light_height: f32,
    is_emissive: f32,
    is_furniture: f32,
    heat_capacity: f32,
    conductivity: f32,
    solar_absorption: f32,
    is_flammable: f32,
    ignition_temp: f32,
    walkable: f32,
    is_removable: f32,
    shows_wall_face: f32,
};

fn get_material(bt: u32) -> GpuMaterial {
    return materials[min(bt, 64u)];
}

// --- Diagonal wall helpers ---
// Variant 0: / solid below-right, 1: \ solid below-left,
// 2: / solid above-left, 3: \ solid above-right
fn diag_is_wall(fx: f32, fy: f32, variant: u32) -> bool {
    if variant == 0u { return fy > (1.0 - fx); }
    if variant == 1u { return fy > fx; }
    if variant == 2u { return fy < (1.0 - fx); }
    return fy < fx; // variant 3
}

fn diag_dist_to_edge(fx: f32, fy: f32, variant: u32) -> f32 {
    if variant == 0u || variant == 2u {
        return abs(fx + fy - 1.0) * 0.7071; // 1/sqrt(2)
    }
    return abs(fy - fx) * 0.7071;
}

// --- Sprite constants ---
const SPRITE_SIZE: u32 = 256u;
const SPRITE_VARIANTS: u32 = 8u; // 8 conifer

// Sample a tree sprite. Returns vec4(r, g, b, height_normalized).
// height_normalized: 0 = transparent (show ground), >0 = canopy/trunk.
fn sample_sprite(variant: u32, fx: f32, fy: f32) -> vec4<f32> {
    let lx = clamp(u32(fx * f32(SPRITE_SIZE)), 0u, SPRITE_SIZE - 1u);
    let ly = clamp(u32(fy * f32(SPRITE_SIZE)), 0u, SPRITE_SIZE - 1u);
    let idx = variant * SPRITE_SIZE * SPRITE_SIZE + ly * SPRITE_SIZE + lx;
    let packed = sprites[idx];
    let r = f32(packed & 0xFFu) / 255.0;
    let g = f32((packed >> 8u) & 0xFFu) / 255.0;
    let b = f32((packed >> 16u) & 0xFFu) / 255.0;
    let h = f32((packed >> 24u) & 0xFFu) / 255.0;
    return vec4<f32>(r, g, b, h);
}

// --- Lightmap sampling ---
// Sample the pre-computed lightmap at a world position.
// Returns vec4(light_color_rgb, light_intensity) with bilinear interpolation.
fn sample_lightmap(wx: f32, wy: f32) -> vec4<f32> {
    let uv = vec2<f32>(wx / camera.grid_w, wy / camera.grid_h);
    return textureSampleLevel(lightmap_tex, lightmap_sampler, uv, 0.0);
}

// --- Shadow map sampling ---
// Sample the pre-computed shadow map (variable resolution, bilinear interpolated).
// The active region is (grid_w * scale) × (grid_h * scale) within the max-size texture.
// Returns vec4(tint_rgb, light_factor) — same format as trace_shadow_ray.
fn sample_shadow_map(wx: f32, wy: f32) -> vec4<f32> {
    // UV maps world coords to the active portion of the shadow map texture.
    // Texture is allocated at max_scale but only scale×grid is populated.
    // scale/max_scale gives the fraction of the texture that's active.
    let max_scale = 8.0; // must match SHADOW_MAP_MAX_SCALE in gpu_init.rs
    let frac = camera.shadow_map_scale / max_scale;
    let uv = vec2<f32>(wx / camera.grid_w * frac, wy / camera.grid_h * frac);
    return textureSampleLevel(shadow_map_tex, lightmap_sampler, uv, 0.0);
}

// --- Block unpacking ---
// type: 0=air, 1=stone, 2=dirt, 3=water, 4=wall, 5=glass, 6=fireplace, 7=electric_light, 8=tree, 9=bench, 10=standing_lamp, 11=table_lamp, 12=fan
// height: 0-255
// flags: bit0=is_door/scorched, bit1=has_roof, bit2=is_open/switch_state
// Wall-specific: bits3-4=wall_edge (0=N,1=E,2=S,3=W), bits5-6=wall_thickness (0=full,1=3,2=2,3=1 sub-cell)
fn block_type(b: u32) -> u32 { return b & 0xFFu; }
fn block_height_raw(b: u32) -> u32 { return (b >> 8u) & 0xFFu; }
// Wall blocks store edge bitmask in height bits 4-7. Mask to lower 4 for visual height.
fn is_wall_type_h(bt: u32) -> bool {
    return bt == BT_STONE || bt == BT_WALL || bt == BT_GLASS || bt == BT_INSULATED
        || (bt >= BT_WOOD_WALL && bt <= BT_LIMESTONE) || bt == BT_MUD_WALL || bt == BT_DIAGONAL || bt == BT_LOW_WALL;
}
fn block_height(b: u32) -> u32 {
    let h = (b >> 8u) & 0xFFu;
    if is_wall_type_h(b & 0xFFu) { return h & 0xFu; }
    return h;
}
fn block_flags(b: u32) -> u32 { return (b >> 16u) & 0xFFu; }
fn has_roof(b: u32) -> bool { return ((b >> 16u) & 2u) != 0u; }
fn is_door(b: u32) -> bool { return ((b >> 16u) & 1u) != 0u; }
fn is_open(b: u32) -> bool { return ((b >> 16u) & 4u) != 0u; }
fn is_glass(b: u32) -> bool { return block_type(b) == 5u; }

// --- Thin wall helpers ---
// Edge bitmask in height byte: bit4=N, bit5=E, bit6=S, bit7=W
// Thickness in flags byte: bits 5-6 (0=full, 1→3, 2→2, 3→1 sub-cell)
fn wall_edge_mask(height: u32) -> u32 { return (height >> 4u) & 0xFu; }
fn wall_thickness_raw(flags: u32) -> u32 { return (flags >> 5u) & 3u; }
fn wall_thickness(flags: u32) -> u32 {
    let raw = wall_thickness_raw(flags);
    return select(4u - raw, 4u, raw == 0u);
}
fn is_thin_wall(flags: u32) -> bool { return wall_thickness_raw(flags) != 0u; }

// Check if a single edge covers this pixel
fn edge_covers_pixel(fx: f32, fy: f32, edge: u32, wall_frac: f32) -> bool {
    if edge == 0u { return fy < wall_frac; }           // N: wall at top
    if edge == 1u { return fx > (1.0 - wall_frac); }   // E: wall at right
    if edge == 2u { return fy > (1.0 - wall_frac); }   // S: wall at bottom
    return fx < wall_frac;                               // W: wall at left
}

// Check if pixel position (fx, fy) falls within the wall portion of a thin wall.
// Uses edge bitmask (any combination of N/E/S/W including T-junctions and crosses).
fn pixel_is_wall(fx: f32, fy: f32, height: u32, flags: u32) -> bool {
    let thick = wall_thickness(flags);
    if thick >= 4u { return true; } // full wall, entire tile is wall
    let mask = wall_edge_mask(height);
    if mask == 0u { return true; } // no edges = full wall (backward compat)
    let wall_frac = f32(thick) * 0.25;
    // Check each edge in the bitmask
    if (mask & 1u) != 0u && edge_covers_pixel(fx, fy, 0u, wall_frac) { return true; } // N
    if (mask & 2u) != 0u && edge_covers_pixel(fx, fy, 1u, wall_frac) { return true; } // E
    if (mask & 4u) != 0u && edge_covers_pixel(fx, fy, 2u, wall_frac) { return true; } // S
    if (mask & 8u) != 0u && edge_covers_pixel(fx, fy, 3u, wall_frac) { return true; } // W
    return false;
}
// Structural wall types that form the building envelope (not equipment/furniture)
fn matches_wall_type(bt: u32) -> bool {
    return bt == BT_STONE || bt == BT_WALL || bt == BT_GLASS || bt == BT_INSULATED
        || (bt >= BT_WOOD_WALL && bt <= BT_LIMESTONE) || bt == BT_MUD_WALL || bt == BT_DIAGONAL || bt == BT_LOW_WALL;
}

fn get_block(x: i32, y: i32) -> u32 {
    if x < 0 || y < 0 || x >= i32(camera.grid_w) || y >= i32(camera.grid_h) {
        return 0u;
    }
    return grid[u32(y) * u32(camera.grid_w) + u32(x)];
}

fn get_block_f(wx: f32, wy: f32) -> u32 {
    return get_block(i32(floor(wx)), i32(floor(wy)));
}

// --- Roof detection ---
// Roof height is precomputed on the CPU and stored in bits 24-31 of each block.
// Returns 0 if the block is not part of a roofed building.
fn get_roof_height(bx: i32, by: i32) -> f32 {
    let block = get_block(bx, by);
    return f32((block >> 24u) & 0xFFu);
}

// ============================================================
// Procedural terrain detail — decoupled noise + biome system.
// These functions layer visual variation onto flat block colors.
// When sprites replace per-pixel detail, the noise functions
// remain useful for variant selection and placement decisions.
// ============================================================

// --- Noise utilities (self-contained, no game-specific logic) ---

// Fast 2D hash → [0, 1]
fn hash2(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453);
}

// 2D hash → vec2 [0, 1]
fn hash2v(p: vec2<f32>) -> vec2<f32> {
    return vec2(
        fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453),
        fract(sin(dot(p, vec2(269.5, 183.3))) * 27183.6142),
    );
}

// Smooth value noise (bilinear interpolation of hash values)
fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    // Smooth hermite interpolation
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash2(i);
    let b = hash2(i + vec2(1.0, 0.0));
    let c = hash2(i + vec2(0.0, 1.0));
    let d = hash2(i + vec2(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Fractional Brownian motion — layered noise at decreasing scales
fn fbm(p: vec2<f32>, octaves: i32) -> f32 {
    var sum = 0.0;
    var amp = 0.5;
    var pos = p;
    for (var i: i32 = 0; i < octaves; i++) {
        sum += value_noise(pos) * amp;
        pos *= 2.1; // non-integer to avoid axis alignment
        amp *= 0.5;
    }
    return sum;
}

// --- Terrain detail function ---
// Layers procedural variation onto a flat base color.
// Inputs:
//   base_col:    flat block color from block_base_color()
//   wx, wy:      world-space pixel position (sub-tile precision)
//   bx, by:      grid cell integer coords
//   water_table: water table depth at this cell (-3..0.5)
//   rain:        current rain intensity (0..1)
//   wind_angle:  wind direction for grass sway
//   time:        for subtle animation
// Returns: modified color with terrain detail applied.
//
// This function is the single point to replace with sprite sampling later.
// Terrain hue palette — indexed by terrain type (bits 0-3 of terrain_buf)
// 0=grass, 1=sand, 2=rocky, 3=clay, 4=gravel, 5=snow, 6=marsh, 7=loam
fn terrain_base_color(terrain_type: u32) -> vec3<f32> {
    if terrain_type == 1u { return vec3(0.68, 0.66, 0.60); }  // chalky: pale grey-white
    if terrain_type == 2u { return vec3(0.45, 0.42, 0.38); }  // rocky: grey
    if terrain_type == 3u { return vec3(0.50, 0.38, 0.25); }  // clay: reddish brown
    if terrain_type == 4u { return vec3(0.48, 0.46, 0.42); }  // gravel: grey-brown
    if terrain_type == 5u { return vec3(0.22, 0.18, 0.12); }  // peat: dark brown-black
    if terrain_type == 6u { return vec3(0.30, 0.35, 0.22); }  // marsh: dark green-brown
    if terrain_type == 7u { return vec3(0.38, 0.30, 0.18); }  // loam: dark fertile
    return vec3(0.42, 0.36, 0.22);                            // grass: default earth
}

fn terrain_detail(
    base_col: vec3<f32>,
    wx: f32, wy: f32,
    bx: i32, by: i32,
    water_table: f32,
    rain: f32,
    wind_angle: f32,
    time: f32,
) -> vec3<f32> {
    let pos = vec2(wx, wy);

    // Read terrain parameters from buffer
    let tidx = u32(by) * u32(camera.grid_w) + u32(bx);
    let tdata = terrain_buf[tidx];
    let t_type = tdata & 0xFu;
    let t_compact = f32((tdata >> 24u) & 0x1Fu) / 31.0;   // 0..1 compaction (foot traffic)
    let t_veg_raw = f32((tdata >> 4u) & 0x1Fu) / 31.0;    // 0..1 base vegetation density
    let t_veg = t_veg_raw * (1.0 - t_compact * 0.9);      // compaction kills vegetation
    let t_grain = f32((tdata >> 9u) & 0xFu) / 15.0;       // 0..1 texture grain
    let t_rough = f32((tdata >> 13u) & 0x3u) / 3.0;        // 0..1 roughness

    // --- 1. Base soil color — bilinear blend between neighboring terrain types ---
    // This eliminates hard tile-edge transitions between terrain types.
    let bl_gx = wx - 0.5;
    let bl_gy = wy - 0.5;
    let bl_ix = i32(floor(bl_gx));
    let bl_iy = i32(floor(bl_gy));
    let bl_fx = fract(bl_gx);
    let bl_fy = fract(bl_gy);
    let bl_ux = bl_fx * bl_fx * (3.0 - 2.0 * bl_fx);
    let bl_uy = bl_fy * bl_fy * (3.0 - 2.0 * bl_fy);
    let bl_gw = i32(camera.grid_w);
    let bl_gh = i32(camera.grid_h);
    let bl_w = u32(camera.grid_w);
    let bl_c00 = terrain_base_color(terrain_buf[u32(clamp(bl_iy, 0, bl_gh-1)) * bl_w + u32(clamp(bl_ix, 0, bl_gw-1))] & 0xFu);
    let bl_c10 = terrain_base_color(terrain_buf[u32(clamp(bl_iy, 0, bl_gh-1)) * bl_w + u32(clamp(bl_ix+1, 0, bl_gw-1))] & 0xFu);
    let bl_c01 = terrain_base_color(terrain_buf[u32(clamp(bl_iy+1, 0, bl_gh-1)) * bl_w + u32(clamp(bl_ix, 0, bl_gw-1))] & 0xFu);
    let bl_c11 = terrain_base_color(terrain_buf[u32(clamp(bl_iy+1, 0, bl_gh-1)) * bl_w + u32(clamp(bl_ix+1, 0, bl_gw-1))] & 0xFu);
    var soil_base = mix(mix(bl_c00, bl_c10, bl_ux), mix(bl_c01, bl_c11, bl_ux), bl_uy);
    // Per-pixel noise variation
    let soil_noise = value_noise(pos * 0.4 + vec2(97.3, 41.2));
    soil_base = mix(soil_base, soil_base * (0.85 + soil_noise * 0.3), 0.5);
    var color = soil_base;

    // Compacted soil: darker, smoother, worn path appearance
    if t_compact > 0.05 {
        let path_col = soil_base * 0.72; // darker packed earth
        color = mix(color, path_col, t_compact * 0.6);
    }

    // --- 2. Moisture influence (from water table) ---
    let moisture = clamp((water_table + 1.5) / 2.5, 0.0, 1.0);
    color = mix(color, color * vec3(0.80, 0.88, 0.75), moisture * 0.35);
    color *= (1.0 - rain * 0.15);

    // --- 3. Vegetation (density from terrain buffer, not hardcoded noise) ---
    // Vegetation is scaled by t_veg: 0 = barren, 1 = lush
    if t_veg > 0.05 {
        // Per-pixel grass noise modulates local presence within the vegetation density
        let grass_noise = fbm(pos * 0.25 + vec2(31.7, 73.1), 3);
        let grass_threshold = 1.0 - t_veg; // higher veg = lower threshold = more grass
        let grass_amount = smoothstep(grass_threshold - 0.1, grass_threshold + 0.1, grass_noise);

        if grass_amount > 0.01 {
            // Grass color varies by terrain type
            let grass_hue = value_noise(pos * 0.4 + vec2(97.3, 41.2));
            var grass_green = vec3(0.22, 0.40, 0.12);
            var grass_yellow = vec3(0.38, 0.42, 0.15);
            // Marsh/loam: darker, richer greens
            if t_type == 6u || t_type == 7u {
                grass_green = vec3(0.15, 0.32, 0.08);
                grass_yellow = vec3(0.28, 0.35, 0.12);
            }
            // Chalky/gravel: sparse, yellowed scrub
            if t_type == 1u || t_type == 4u {
                grass_green = vec3(0.35, 0.38, 0.18);
                grass_yellow = vec3(0.45, 0.42, 0.22);
            }
            // Peat: dark mossy tones
            if t_type == 5u {
                grass_green = vec3(0.18, 0.28, 0.10);
                grass_yellow = vec3(0.25, 0.30, 0.15);
            }
            var grass_col = mix(grass_green, grass_yellow, grass_hue * 0.5);
            grass_col = mix(grass_col, grass_col * 0.7, moisture * 0.3);

            // Grass blade detail — mix of short and long grass
            let blade_seed = hash2(floor(pos * 6.0));
            let blade_angle = blade_seed * 6.28 + wind_angle * 0.3;
            let blade_dir = vec2(cos(blade_angle), sin(blade_angle));
            let blade_pos = fract(pos * 6.0);
            let along_blade = dot(blade_pos - 0.5, blade_dir);
            let across_blade = abs(dot(blade_pos - 0.5, vec2(-blade_dir.y, blade_dir.x)));

            // Grass height categories
            let is_tall_grass = t_veg > 0.65; // veg 20+/31
            let long_seed = hash2(floor(pos * 3.0));
            let is_long_grass = long_seed > 0.7 && t_veg > 0.4;

            // Tall grass: bigger blades, more sway, denser
            var b_width = select(0.08, 0.06, is_long_grass);
            var b_min = select(-0.15, -0.2, is_long_grass);
            var b_max = select(0.25, 0.4, is_long_grass);
            if is_tall_grass {
                b_width = 0.04;  // thin individual strands
                b_min = -0.25;
                b_max = 0.48;    // very tall
            }

            // Wind sway: more pronounced in tall grass
            let wind_sway = sin(camera.time * 1.2 + wx * 2.5 + wy * 1.8)
                          * camera.wind_magnitude * select(0.02, 0.08, is_tall_grass);
            let swayed_along = along_blade + wind_sway;

            let on_blade = f32(across_blade < b_width && swayed_along > b_min && swayed_along < b_max);
            let blade_t = clamp((swayed_along - b_min) / (b_max - b_min), 0.0, 1.0);
            var blade_col = mix(grass_col * 0.7, grass_col * 1.2, blade_t);

            // Tall grass tips: sun-bleached golden
            if (is_long_grass || is_tall_grass) && blade_t > 0.65 {
                let tip_blend = (blade_t - 0.65) * 2.85;
                blade_col = mix(blade_col, vec3(0.55, 0.50, 0.28), tip_blend);
                // Sun catch: blades leaning toward sun are brighter
                let sun_catch = max(0.0, wind_sway * 5.0 * camera.sun_dir_x + 0.2)
                              * camera.sun_intensity * 0.3;
                blade_col += vec3(camera.sun_color_r, camera.sun_color_g, camera.sun_color_b) * sun_catch;
            }

            // Density: tall grass fills more of the tile
            let density_boost = select(0.0, 0.25, is_tall_grass);
            let grass_vis = grass_amount * mix(0.5 + density_boost, on_blade * 0.8 + 0.2 + density_boost, 0.6);
            color = mix(color, blade_col, clamp(grass_vis, 0.0, 0.90));

            // Tall grass overlay: dense canopy of bright tips over dark understory
            if is_tall_grass {
                // Dark understory (light doesn't reach the ground through dense grass)
                color = mix(color, grass_col * 0.45, grass_amount * 0.3);

                // Bright grass tip layer: high-frequency noise field
                let tip_n1 = value_noise(pos * 8.0 + vec2(time * 0.3, time * 0.2));
                let tip_n2 = value_noise(pos * 12.0 + vec2(77.0, 33.0) + vec2(wind_sway * 2.0, 0.0));
                let tip_bright = smoothstep(0.35, 0.65, tip_n1 * 0.6 + tip_n2 * 0.4);

                // Tips color: bright yellow-green, catching sun
                let tip_col = mix(
                    grass_col * 1.15,
                    vec3(0.55, 0.52, 0.28),
                    tip_bright * 0.5
                );
                let tip_sun = max(0.0, wind_sway * 4.0 * camera.sun_dir_x + 0.15)
                            * camera.sun_intensity * 0.25;

                color = mix(color, tip_col + vec3(camera.sun_color_r, camera.sun_color_g, camera.sun_color_b) * tip_sun,
                    grass_amount * tip_bright * 0.55);
            }

            // Wildflowers (only in medium-high vegetation areas)
            if t_veg > 0.4 {
                let flower_hash = hash2(floor(pos * 4.0) + vec2(173.1, 291.7));
                if flower_hash > 0.93 && grass_amount > 0.4 {
                    let flower_sub = fract(pos * 4.0) - 0.5;
                    let flower_dist = length(flower_sub);
                    if flower_dist < 0.12 {
                        let flower_type = hash2(floor(pos * 4.0) + vec2(0.0, 500.0));
                        var flower_col = vec3(0.85, 0.25, 0.20);
                        if flower_type > 0.7 { flower_col = vec3(0.90, 0.80, 0.15); }
                        else if flower_type > 0.4 { flower_col = vec3(0.70, 0.30, 0.75); }
                        else if flower_type > 0.2 { flower_col = vec3(0.90, 0.88, 0.82); }
                        color = mix(color, flower_col, smoothstep(0.12, 0.04, flower_dist));
                    }
                }
            }
        }
    }

    // --- 4. Fine soil grain (frequency from terrain buffer) ---
    let grain_freq = 4.0 + t_grain * 12.0; // 4..16 — fine to coarse
    let grain_val = value_noise(pos * grain_freq);
    let grain_strength = 0.03 + t_rough * 0.05; // rougher = more variation
    color += vec3((grain_val - 0.5) * grain_strength);

    // Freshly dug earth: max roughness creates a disturbed look
    if t_rough > 0.9 {
        // Clumpy, dark, uneven — visible soil chunks and exposed subsurface
        let clump = value_noise(pos * 8.0);
        let streak = value_noise(pos * vec2(2.0, 12.0)); // directional streaks (shovel marks)
        // Darken overall (exposed subsurface is darker)
        color *= 0.8 + clump * 0.15;
        // Add warm brown tint (fresh earth vs weathered surface)
        color = mix(color, vec3(0.25, 0.18, 0.10), 0.15 + streak * 0.1);
        // Heightmap-style clumps: light/dark variation for 3D feel
        let lump = value_noise(pos * 6.0 + vec2(97.0, 41.0));
        color += vec3((lump - 0.5) * 0.08);
    }

    // --- 5. Surface detail per terrain type ---
    // Pebbles/stones: more common with high roughness
    let pebble_density = t_rough * (1.0 - t_veg * 0.7);
    if pebble_density > 0.1 {
        let pebble_hash = hash2(floor(pos * 5.0) + vec2(419.7, 137.3));
        if pebble_hash > 1.0 - pebble_density * 0.2 {
            let pebble_sub = fract(pos * 5.0) - 0.5;
            let pebble_offset = hash2v(floor(pos * 5.0) + vec2(77.0, 33.0)) * 0.3 - 0.15;
            let pebble_dist = length(pebble_sub - pebble_offset);
            let pebble_r = 0.04 + hash2(floor(pos * 5.0) + vec2(200.0, 0.0)) * 0.04;
            if pebble_dist < pebble_r {
                let pebble_shade = hash2(floor(pos * 5.0) + vec2(500.0, 300.0));
                let pebble_col = mix(vec3(0.45, 0.43, 0.40), vec3(0.58, 0.55, 0.50), pebble_shade);
                color = mix(color, pebble_col, smoothstep(pebble_r, pebble_r * 0.3, pebble_dist));
            }
        }
    }

    // Chalky soil: pale streaks and exposed chalk fragments
    if t_type == 1u {
        let chalk_streak = sin(wx * 2.5 + wy * 1.2 + value_noise(pos * 0.4) * 3.0) * 0.5 + 0.5;
        color = mix(color, vec3(0.78, 0.76, 0.70), chalk_streak * 0.25);
        // White chalk fragments
        let frag = value_noise(pos * 12.0);
        if frag > 0.88 {
            color = mix(color, vec3(0.82, 0.80, 0.75), (frag - 0.88) * 8.0 * 0.4);
        }
    }

    // Peat: dark wet sheen, waterlogged patches
    if t_type == 5u {
        let wet_patch = value_noise(pos * 3.0 + vec2(17.3, 41.7));
        if wet_patch > 0.5 {
            let wet_t = (wet_patch - 0.5) * 2.0;
            color = mix(color, color * vec3(0.7, 0.75, 0.8), wet_t * 0.4); // dark wet sheen
        }
    }

    // Marsh: water glints near ponds
    if t_type == 6u {
        let glint = value_noise(pos * 8.0 + vec2(time * 0.3, 0.0));
        if glint > 0.92 {
            color = mix(color, vec3(0.4, 0.55, 0.7), (glint - 0.92) * 12.0 * 0.3);
        }
    }

    return color;
}

// Terrain detail for stone blocks — cracks, mineral veins, color banding
fn stone_detail(base_col: vec3<f32>, wx: f32, wy: f32) -> vec3<f32> {
    let pos = vec2(wx, wy);
    var color = base_col;

    // Regional color variation (warm gray vs cool gray)
    let region = value_noise(pos * 0.1);
    let warm = vec3(0.54, 0.50, 0.44); // brownish gray
    let cool = vec3(0.48, 0.50, 0.54); // bluish gray
    color = mix(warm, cool, region);

    // Layered strata (horizontal banding)
    let strata = sin(wy * 3.0 + value_noise(pos * 0.5) * 2.0) * 0.03;
    color += vec3(strata);

    // Crack lines
    let crack_noise = fbm(pos * 3.0 + vec2(57.0, 13.0), 3);
    let crack = smoothstep(0.48, 0.50, crack_noise) * smoothstep(0.52, 0.50, crack_noise);
    color = mix(color, color * 0.6, crack * 0.8);

    // Mineral specks (rare bright/dark spots)
    let mineral = hash2(floor(pos * 8.0));
    if mineral > 0.95 {
        let spec_sub = fract(pos * 8.0) - 0.5;
        if length(spec_sub) < 0.06 {
            color = mix(color, vec3(0.70, 0.68, 0.60), 0.5); // quartz-like
        }
    } else if mineral < 0.03 {
        let spec_sub = fract(pos * 8.0) - 0.5;
        if length(spec_sub) < 0.05 {
            color = mix(color, vec3(0.25, 0.22, 0.20), 0.5); // dark mineral
        }
    }

    // Fine grain texture
    let grain = value_noise(pos * 12.0);
    color += vec3((grain - 0.5) * 0.025);

    return color;
}

// --- Wood floor detail: finished planks with grain, knots, and nail dots ---
fn wood_floor_detail(wx: f32, wy: f32) -> vec3<f32> {
    let pos = vec2(wx, wy);

    // Plank layout: 3 planks per tile running east-west
    // Stagger plank seams between rows (brick-like offset)
    let plank_row = floor(wy * 3.0);
    let row_offset = fract(plank_row * 0.5) * 0.33; // stagger seams
    let plank_col = floor((wx + row_offset) * 2.0);

    // Per-plank color variation (warm wood tones)
    let plank_id = plank_row * 7.0 + plank_col * 13.0;
    let plank_hash = fract(sin(plank_id * 127.1 + 311.7) * 43758.5);
    let base_warm = vec3<f32>(0.58, 0.42, 0.22);  // honey oak
    let base_cool = vec3<f32>(0.48, 0.35, 0.18);  // darker oak
    var color = mix(base_warm, base_cool, plank_hash);
    // Slight reddish or golden tint per plank
    let tint = fract(sin(plank_id * 73.3) * 21753.1);
    color += vec3(tint * 0.03, tint * 0.01, -tint * 0.01);

    // Plank seam lines (dark gaps between planks)
    let py_in_plank = fract(wy * 3.0);
    let seam_y = smoothstep(0.0, 0.04, py_in_plank) * smoothstep(0.0, 0.04, 1.0 - py_in_plank);
    color *= 0.7 + seam_y * 0.3;

    // End seams (where planks butt together along the row)
    let px_in_plank = fract((wx + row_offset) * 2.0);
    let seam_x = smoothstep(0.0, 0.03, px_in_plank) * smoothstep(0.0, 0.03, 1.0 - px_in_plank);
    color *= 0.85 + seam_x * 0.15;

    // Wood grain: flowing lines along plank length
    let grain_freq = 8.0 + plank_hash * 4.0;
    let grain_offset = fract(sin(plank_id * 41.7) * 9371.3) * 10.0;
    let grain = sin((wx * grain_freq + grain_offset) + value_noise(pos * 2.0 + vec2(plank_id, 0.0)) * 2.0);
    let grain_line = smoothstep(0.6, 0.8, abs(grain)) * 0.06;
    color -= vec3(grain_line * 0.5, grain_line * 0.3, grain_line * 0.1);

    // Knot holes (rare, per plank)
    let knot_hash = fract(sin(plank_id * 193.7 + 47.1) * 43758.5);
    if knot_hash > 0.75 {
        let knot_cx = fract(sin(plank_id * 83.1) * 7531.3) * 0.6 + 0.2;
        let knot_cy = fract(sin(plank_id * 131.7) * 3917.1) * 0.5 + 0.25;
        let knot_wx = (plank_col + knot_cx - row_offset) / 2.0;
        let knot_wy = (plank_row + knot_cy) / 3.0;
        let knot_dist = length(vec2(wx - knot_wx, wy - knot_wy));
        let knot_r = 0.03 + knot_hash * 0.02;
        if knot_dist < knot_r * 2.5 {
            // Concentric ring around knot
            let ring = abs(knot_dist - knot_r) / knot_r;
            let knot_dark = mix(color * 0.5, color * 0.7, ring);
            color = mix(knot_dark, color, smoothstep(0.0, knot_r * 2.5, knot_dist));
        }
    }

    // Nail dots at plank ends (near seams)
    if px_in_plank < 0.06 || px_in_plank > 0.94 {
        let nail_y = fract(wy * 3.0);
        if abs(nail_y - 0.25) < 0.02 || abs(nail_y - 0.75) < 0.02 {
            color = vec3(0.25, 0.23, 0.22); // dark iron nail head
        }
    }

    // Subtle sheen variation (simulates wear/polish)
    let wear = value_noise(pos * 1.5 + vec2(77.0, 33.0));
    color *= 0.95 + wear * 0.10;

    return color;
}

// --- Rough floor detail: early-game unfinished planks with gaps and dirt ---
fn rough_floor_detail(wx: f32, wy: f32) -> vec3<f32> {
    let pos = vec2(wx, wy);
    let dirt_color = vec3<f32>(0.38, 0.30, 0.18);

    // Irregular plank layout: 2-4 planks per tile, varying widths
    // Use world-position hash to vary plank count per row
    let row_id = floor(wy * 2.5);
    let row_hash = fract(sin(row_id * 127.1 + 311.7) * 43758.5);

    // Plank boundaries defined by accumulated widths (irregular)
    let plank_y = fract(wy * 2.5);
    let width_var = fract(sin(row_id * 73.3) * 43758.5) * 0.15;

    // Gap between planks (wider and more irregular than finished floor)
    let gap_width = 0.06 + row_hash * 0.04;
    let near_gap = plank_y < gap_width || plank_y > (1.0 - gap_width);
    if near_gap {
        // Dirt visible through gaps
        let dirt_noise = value_noise(pos * 6.0);
        return dirt_color * (0.8 + dirt_noise * 0.2);
    }

    // Raw wood color — less uniform than finished, more weathered
    let plank_id = row_id * 7.0 + floor(wx * 1.5) * 13.0;
    let plank_hash = fract(sin(plank_id * 127.1) * 43758.5);
    let base_light = vec3<f32>(0.52, 0.40, 0.22);  // raw pale wood
    let base_dark = vec3<f32>(0.38, 0.28, 0.14);   // weathered dark
    var color = mix(base_light, base_dark, plank_hash * 0.6);

    // Rough grain: coarser and more visible than finished floor
    let grain = sin(wx * 12.0 + value_noise(pos * 1.5) * 4.0 + plank_hash * 20.0);
    let grain_strength = 0.08 + plank_hash * 0.04;
    color -= vec3(smoothstep(0.5, 0.9, abs(grain)) * grain_strength);

    // Saw marks: perpendicular cuts visible in rough-hewn wood
    let saw = fract(wx * 8.0 + plank_hash * 3.0);
    let saw_line = smoothstep(0.0, 0.03, saw) * smoothstep(0.0, 0.03, 1.0 - saw);
    color *= 0.92 + saw_line * 0.08;

    // Splinters / rough edges at plank boundaries
    let edge_dist = min(plank_y - gap_width, (1.0 - gap_width) - plank_y);
    let edge_noise = value_noise(pos * 15.0 + vec2(plank_id, 0.0));
    if edge_dist < 0.08 && edge_noise > 0.6 {
        // Splintered edge: darker, irregular
        color = mix(color, dirt_color, (0.08 - edge_dist) * 5.0 * (edge_noise - 0.6) * 2.5);
    }

    // Occasional missing section (shows dirt)
    let hole_hash = fract(sin(plank_id * 193.7 + wx * 47.1) * 43758.5);
    if hole_hash > 0.95 {
        let hole_pos = fract(pos * 4.0) - 0.5;
        if length(hole_pos) < 0.08 {
            return dirt_color * (0.7 + value_noise(pos * 8.0) * 0.3);
        }
    }

    // Weathering: darker patches and stains
    let stain = value_noise(pos * 2.0 + vec2(41.0, 97.0));
    if stain > 0.65 {
        color *= 0.85 + (stain - 0.65) * 0.3;
    }

    return color;
}

// --- Smooth elevation sampling (bilinear interpolation across tile boundaries) ---
// elevation_buf is interleaved: [elev, ao, elev, ao, ...] — stride 2 per cell.
fn sample_elevation(wx: f32, wy: f32) -> f32 {
    let gx = wx - 0.5;
    let gy = wy - 0.5;
    let ix = i32(floor(gx));
    let iy = i32(floor(gy));
    let fx = fract(gx);
    let fy = fract(gy);
    let gw = i32(camera.grid_w);
    let gh = i32(camera.grid_h);
    let cx0 = u32(clamp(ix, 0, gw - 1));
    let cx1 = u32(clamp(ix + 1, 0, gw - 1));
    let cy0 = u32(clamp(iy, 0, gh - 1));
    let cy1 = u32(clamp(iy + 1, 0, gh - 1));
    let w = u32(camera.grid_w);
    let a = elevation_buf[(cy0 * w + cx0) * 2u];
    let b = elevation_buf[(cy0 * w + cx1) * 2u];
    let c = elevation_buf[(cy1 * w + cx0) * 2u];
    let d = elevation_buf[(cy1 * w + cx1) * 2u];
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);
    return mix(mix(a, b, ux), mix(c, d, ux), uy);
}

// Sample terrain ambient occlusion (bilinear interpolated, from interleaved buffer).
// Returns 0.6–1.0: lower = more occluded (valley), higher = exposed (hilltop).
fn sample_terrain_ao(wx: f32, wy: f32) -> f32 {
    let gx = wx - 0.5;
    let gy = wy - 0.5;
    let ix = i32(floor(gx));
    let iy = i32(floor(gy));
    let fx = fract(gx);
    let fy = fract(gy);
    let gw = i32(camera.grid_w);
    let gh = i32(camera.grid_h);
    let cx0 = u32(clamp(ix, 0, gw - 1));
    let cx1 = u32(clamp(ix + 1, 0, gw - 1));
    let cy0 = u32(clamp(iy, 0, gh - 1));
    let cy1 = u32(clamp(iy + 1, 0, gh - 1));
    let w = u32(camera.grid_w);
    let a = elevation_buf[(cy0 * w + cx0) * 2u + 1u];
    let b = elevation_buf[(cy0 * w + cx1) * 2u + 1u];
    let c = elevation_buf[(cy1 * w + cx0) * 2u + 1u];
    let d = elevation_buf[(cy1 * w + cx1) * 2u + 1u];
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);
    return mix(mix(a, b, ux), mix(c, d, ux), uy);
}

// ============================================================
// End terrain detail
// ============================================================

// --- Lighting ---
// Sun arc: dawn (east/right) → noon (overhead) → dusk (west/left) → night → dawn
// Full cycle = DAY_DURATION seconds. Daylight occupies the middle portion.
const DAY_DURATION: f32 = 60.0; // 60 seconds for full day/night cycle
const DAWN_START: f32 = 0.15;   // sun rises at 15% through cycle
const DUSK_END: f32 = 0.85;     // sun sets at 85% through cycle

// Returns (sun_dir_xy, unused) — sun direction in ground plane
// sun_dir_xy: unit-ish vector FROM pixel TOWARD the sun
fn get_sun(time: f32) -> vec3<f32> {
    let t = fract(time / DAY_DURATION);
    // Map daytime portion [DAWN_START..DUSK_END] to [0..1]
    let day_t = clamp((t - DAWN_START) / (DUSK_END - DAWN_START), 0.0, 1.0);
    // Sun angle: 0 = east, π = west
    let angle = day_t * 3.14159265;
    let sun_x = -cos(angle);
    let sun_y = -sin(angle) * 0.6 - 0.2;
    return vec3<f32>(sun_x, sun_y, 0.0);
}

fn get_sun_elevation(time: f32) -> f32 {
    let t = fract(time / DAY_DURATION);
    let day_t = clamp((t - DAWN_START) / (DUSK_END - DAWN_START), 0.0, 1.0);
    // Smooth rise and fall — use smoothstep at edges to avoid sudden changes
    let noon_factor = sin(day_t * 3.14159265);
    // Fade elevation smoothly to near-zero at dawn/dusk
    let edge_fade = smoothstep(0.0, 0.15, day_t) * smoothstep(1.0, 0.85, day_t);
    // High elevation range: sun is very far away, shadows are short and parallel
    return mix(1.0, 4.0, noon_factor) * edge_fade;
}

// Returns 0..1 indicating how much sunlight is active (0 = night, 1 = full day)
fn get_sun_intensity(time: f32) -> f32 {
    let t = fract(time / DAY_DURATION);
    // Smooth fade in at dawn, smooth fade out at dusk
    let fade_in = smoothstep(DAWN_START - 0.05, DAWN_START + 0.05, t);
    let fade_out = smoothstep(DUSK_END + 0.05, DUSK_END - 0.05, t);
    return fade_in * fade_out;
}

fn get_sun_color(time: f32) -> vec3<f32> {
    let t = fract(time / DAY_DURATION);
    let day_t = clamp((t - DAWN_START) / (DUSK_END - DAWN_START), 0.0, 1.0);
    let noon_factor = sin(day_t * 3.14159265);
    let dawn_color = vec3<f32>(1.0, 0.55, 0.25);
    let noon_color = vec3<f32>(1.0, 0.97, 0.90);
    let sun_col = mix(dawn_color, noon_color, smoothstep(0.0, 0.6, noon_factor));
    // Scale by sun intensity so color fades to zero at night
    return sun_col * get_sun_intensity(time);
}

fn get_ambient(time: f32) -> vec3<f32> {
    let intensity = get_sun_intensity(time);
    let night_ambient = vec3<f32>(0.008, 0.008, 0.02);
    let day_ambient = vec3<f32>(0.10, 0.10, 0.13);
    return mix(night_ambient, day_ambient, intensity);
}

const SHADOW_MAX_DIST: f32 = 12.0;
const SHADOW_STEP: f32 = 0.35;  // coarser steps for performance (was 0.20)

// Glass properties
const GLASS_TINT: vec3<f32> = vec3<f32>(0.7, 0.85, 0.95);
const GLASS_ABSORPTION: f32 = 0.35;
const GLASS_REFRACT_OFFSET: f32 = 0.08;

// Window geometry: glass blocks only transmit light between sill and lintel
// On a wall of height H, the window occupies [WINDOW_SILL_FRAC*H, (1-WINDOW_LINTEL_FRAC)*H]
const WINDOW_SILL_FRAC: f32 = 0.25;   // bottom 25% of wall is solid sill
const WINDOW_LINTEL_FRAC: f32 = 0.15;  // top 15% of wall is solid lintel

// Window thickness: how much of the block the glass occupies perpendicular to the wall
// 0.4 = window is 40% of block width, centered (30% wall on each side)
const WINDOW_THICKNESS: f32 = 0.35;

// Interior indirect light: interiors get a base level of bounced light
// so they're not pitch black behind walls
const INTERIOR_INDIRECT: f32 = 0.06;
// Direct sunbeam strength through windows
const INTERIOR_SUNBEAM: f32 = 0.70;
// Ambient bounce near windows (even when not in direct beam)
const INTERIOR_WINDOW_AMBIENT: f32 = 0.12;
// Trace a 2D ray from an interior floor pixel toward the sun.
// The ray walks through the ground plane until it hits the building envelope.
// Returns: vec4(tint_rgb, light_factor)
//   light_factor ~1.0 if ray exits through glass (sunbeam), ~0.0 if hits wall.
fn trace_interior_sun_ray(wx: f32, wy: f32, sun_dir: vec2<f32>) -> vec4<f32> {
    // Skip interior sun ray at night — no sunbeams when sun is down
    if camera.sun_intensity < 0.001 { return vec4<f32>(1.0, 1.0, 1.0, 0.0); }
    let dir2d = normalize(sun_dir);
    let step_size = 0.25;
    let step_x = dir2d.x * step_size;
    let step_y = dir2d.y * step_size;
    // Sun elevation for the ray height — starts at floor level, rises as it traces toward sun
    let sun_elev = get_sun_elevation(camera.time);
    let step_h = sun_elev * step_size;

    var sx = wx;
    var sy = wy;
    var ray_h = 0.0; // ray starts at floor level (looking up toward the window)
    var tint = vec3<f32>(1.0);
    var light = 1.0;

    // Walk until we exit the building (hit a non-roofed tile) or run out of steps
    let max_steps = 64; // ~16 blocks at 0.25 step
    for (var i: i32 = 0; i < max_steps; i++) {
        sx += step_x;
        sy += step_y;
        ray_h += step_h;

        let bx = i32(floor(sx));
        let by = i32(floor(sy));
        let block = get_block(bx, by);
        let bt = block_type(block);
        let bh = block_height(block);
        let fbh = f32(bh);

        // Still on a roofed floor tile — ray is in interior airspace, continue
        if has_roof(block) && bh == 0u {
            continue;
        }

        // Light sources: interior fixtures, ray passes over them
        if get_material(bt).is_emissive > 0.5 {
            continue;
        }

        // Hit a glass block — ray passes through with tint/absorption
        if bt == BT_GLASS {
            let window_open_frac = 1.0 - WINDOW_SILL_FRAC - WINDOW_LINTEL_FRAC;
            let absorption = GLASS_ABSORPTION * step_size * window_open_frac;
            light *= (1.0 - absorption);
            tint *= mix(vec3<f32>(1.0), GLASS_TINT, step_size * 0.8 * window_open_frac);

            // Wall portion (sill + lintel)
            let wall_frac = 1.0 - window_open_frac;
            light *= (1.0 - wall_frac * step_size * 1.5);

            if light < 0.02 {
                return vec4<f32>(tint, 0.0);
            }
            continue;
        }

        // Hit a door — open doors let light through, closed doors block most
        if is_door(block) {
            if is_open(block) {
                continue;
            }
            light *= 0.4;
            continue;
        }

        // Block with height
        if bh > 0u {
            // Furniture (benches): sun passes over if ray is above furniture height
            if bt == BT_BENCH && fbh <= ray_h {
                continue;
            }
            // Walls and everything else: always block the interior sun ray
            return vec4<f32>(tint, 0.0);
        }

        // Reached an open-air tile (no roof, no height) — ray has exited the building
        // This means the pixel is in a sunbeam
        return vec4<f32>(tint, light);
    }

    // Ran out of steps — assume blocked
    return vec4<f32>(tint, 0.0);
}

// Full interior lighting: combines ambient base, window ambient fill, and direct sunbeams.
// --- Interior light sources (constants for render_fireplace / render_electric_light) ---
const FIRE_COLOR: vec3<f32> = vec3<f32>(1.0, 0.55, 0.15);       // warm orange
const FIRE_COLOR_HOT: vec3<f32> = vec3<f32>(1.0, 0.85, 0.4);    // bright yellow-white core

// Electric light: cool-white ceiling light
const ELIGHT_COLOR: vec3<f32> = vec3<f32>(0.95, 0.92, 0.85);    // warm white (~4000K)

// Point light proximity glow radius and intensity
const GLOW_RADIUS: f32 = 6.0;
const FIRE_GLOW_INTENSITY: f32 = 0.70;
const ELIGHT_GLOW_INTENSITY: f32 = 0.85;
const STANDING_LAMP_GLOW_INTENSITY: f32 = 1.0;
const STANDING_LAMP_GLOW_RADIUS: f32 = 6.0;
const TABLE_LAMP_GLOW_INTENSITY: f32 = 0.35;
const TABLE_LAMP_GLOW_RADIUS: f32 = 4.0;
const STANDING_LAMP_COLOR: vec3<f32> = vec3<f32>(0.95, 0.85, 0.60);
const TABLE_LAMP_COLOR: vec3<f32> = vec3<f32>(0.95, 0.80, 0.50);

// Simple pseudo-random hash for fire flicker
fn fire_hash(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453);
}

// Multi-octave flicker noise for natural fire variation
fn fire_flicker(time: f32) -> f32 {
    // Several overlapping sine waves at different frequencies = organic flicker
    let f1 = sin(time * 8.3) * 0.3;
    let f2 = sin(time * 13.7 + 2.1) * 0.2;
    let f3 = sin(time * 23.1 + 0.7) * 0.15;
    let f4 = sin(time * 37.9 + 4.3) * 0.1;
    // Occasional bigger gutters
    let gutter = sin(time * 3.1) * sin(time * 1.7);
    let gutter_pulse = max(0.0, gutter) * 0.25;
    return clamp(0.5 + f1 + f2 + f3 + f4 - gutter_pulse, 0.0, 1.0);
}

// Trace a line from (x0,y0) to (x1,y1) checking for wall occlusion.
// light_h: height of the light source. Blocks shorter than this are skipped.
// Returns 1.0 if clear, 0.0 if blocked by a wall, partial for glass/trees.
fn trace_glow_visibility(x0: f32, y0: f32, x1: f32, y1: f32, light_h: f32) -> f32 {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let dist_sq = dx * dx + dy * dy;
    if dist_sq < 0.25 { return 1.0; } // 0.5^2

    let dist = sqrt(dist_sq);
    let steps = i32(ceil(dist * 3.0)); // 3 samples per tile for thin wall accuracy
    var vis = 1.0;
    var prev_bx = i32(floor(x0));
    var prev_by = i32(floor(y0));

    for (var i: i32 = 1; i < steps; i++) {
        let t = f32(i) / f32(steps);
        let sx = x0 + dx * t;
        let sy = y0 + dy * t;
        let sbx = i32(floor(sx));
        let sby = i32(floor(sy));
        let sb = get_block(sbx, sby);
        let sbt = block_type(sb);
        let sbh = block_height(sb);

        // Skip light source blocks
        if get_material(sbt).light_intensity > 0.0 { prev_bx = sbx; prev_by = sby; continue; }

        // Wall_data: check edges on tile boundary crossings (cardinal + diagonal)
        if sbx >= 0 && sby >= 0 && sbx < i32(camera.grid_w) && sby < i32(camera.grid_h)
            && prev_bx >= 0 && prev_by >= 0 && prev_bx < i32(camera.grid_w) && prev_by < i32(camera.grid_h) {
            let sdx = sbx - prev_bx;
            let sdy = sby - prev_by;
            // X boundary crossing
            if sdx != 0 {
                let dir_a = select(3u, 1u, sdx > 0);
                let dir_b = (dir_a + 2u) % 4u;
                let a_wd = read_wall_data(u32(prev_by) * u32(camera.grid_w) + u32(prev_bx));
                let b_wd = read_wall_data(u32(prev_by) * u32(camera.grid_w) + u32(sbx));
                if ((a_wd & (1u << dir_a)) != 0u && !(wd_has_door(a_wd) && wd_door_open(a_wd)))
                    || ((b_wd & (1u << dir_b)) != 0u && !(wd_has_door(b_wd) && wd_door_open(b_wd))) {
                    return 0.0;
                }
            }
            // Y boundary crossing
            if sdy != 0 {
                let dir_a = select(0u, 2u, sdy > 0);
                let dir_b = (dir_a + 2u) % 4u;
                let a_wd = read_wall_data(u32(prev_by) * u32(camera.grid_w) + u32(prev_bx));
                let b_wd = read_wall_data(u32(sby) * u32(camera.grid_w) + u32(prev_bx));
                if ((a_wd & (1u << dir_a)) != 0u && !(wd_has_door(a_wd) && wd_door_open(a_wd)))
                    || ((b_wd & (1u << dir_b)) != 0u && !(wd_has_door(b_wd) && wd_door_open(b_wd))) {
                    return 0.0;
                }
            }
            // Diagonal: check destination tile for wall edges facing back toward source
            if sdx != 0 && sdy != 0 {
                let dest_wd = read_wall_data(u32(sby) * u32(camera.grid_w) + u32(sbx));
                if dest_wd != 0u && !(wd_has_door(dest_wd) && wd_door_open(dest_wd)) {
                    // face_x: entering from East(1) if moving left, West(3) if moving right
                    let face_x = select(1u, 3u, sdx > 0);
                    // face_y: entering from South(2) if moving up, North(0) if moving down
                    let face_y = select(2u, 0u, sdy > 0);
                    if ((dest_wd & (1u << face_x)) != 0u) || ((dest_wd & (1u << face_y)) != 0u) {
                        return 0.0;
                    }
                }
                // Check intermediate corner tiles only for perpendicular wall pairs
                // (two walls meeting at a right angle to seal a corner)
                let mid1_wd = read_wall_data(u32(prev_by) * u32(camera.grid_w) + u32(sbx));
                let mid2_wd = read_wall_data(u32(sby) * u32(camera.grid_w) + u32(prev_bx));
                // Only block if BOTH intermediate tiles have wall edges (sealed corner)
                if (mid1_wd & 0xFu) != 0u && (mid2_wd & 0xFu) != 0u
                    && !(wd_has_door(mid1_wd) && wd_door_open(mid1_wd))
                    && !(wd_has_door(mid2_wd) && wd_door_open(mid2_wd))
                {
                    return 0.0;
                }
            }
        }
        prev_bx = sbx;
        prev_by = sby;
        if sbh == 0u { continue; } // open floor
        if (sbt >= BT_PIPE && sbt <= BT_INLET) || sbt == BT_RESTRICTOR || sbt == BT_LIQUID_PIPE || sbt == BT_PIPE_BRIDGE || sbt == BT_LIQUID_INTAKE || sbt == BT_LIQUID_PUMP || sbt == BT_LIQUID_OUTPUT { continue; }
        if sbt == BT_WIRE_BRIDGE { continue; } // wire bridge
        if sbt == BT_DUG_GROUND { continue; } // dug ground doesn't block light
        if sbt == BT_WIRE { continue; } // wire (height = connection mask, not visual)
        if sbt == BT_DIMMER { continue; } // dimmer/varistor (height = level, not visual)
        if sbt == BT_BREAKER { continue; } // breaker (height = threshold, not visual)

        // Doors: open = pass through, closed = block (regardless of height)
        if is_door(sb) {
            if is_open(sb) { continue; } else { return 0.0; }
        }

        // Light is above this block — passes over (furniture below light height)
        if f32(sbh) <= light_h {
            continue;
        }

        // Glass: partial transmission
        if sbt == BT_GLASS {
            vis *= 0.5;
            if vis < 0.02 { return 0.0; }
            continue;
        }

        // Trees: check if ray passes through the circular canopy, not just the tile
        if sbt == BT_TREE {
            let th = block_height(sb);
            let canopy_r = select(select(select(0.35, 0.5, th >= 3u), 0.65, th >= 4u), 0.75, th >= 5u);
            // Tree center with per-tree offset (matches render_tree)
            let tid = f32(sbx) * 137.0 + f32(sby) * 311.0;
            let tox = (fract(sin(tid * 1.3 + 7.1) * 31415.9) - 0.5) * 0.3;
            let toy = (fract(sin(tid * 2.7 + 3.9) * 27183.6) - 0.5) * 0.3;
            let tcx = f32(sbx) + 0.5 + tox;
            let tcy = f32(sby) + 0.5 + toy;
            let d = length(vec2(sx - tcx, sy - tcy));
            if d < canopy_r {
                vis *= 0.35; // inside canopy: strong attenuation
            } else {
                // outside canopy circle but in tile: no blocking
                prev_bx = sbx;
                prev_by = sby;
                continue;
            }
            if vis < 0.02 { return 0.0; }
            continue;
        }

        // Berry bushes + crops: soft dappled shadow
        if sbt == BT_BERRY_BUSH || sbt == BT_CROP {
            vis *= 0.6;
            if vis < 0.02 { return 0.0; }
            continue;
        }

        // Solid wall: blocked
        return 0.0;
    }

    return vis;
}

// Per-pixel point light proximity glow.
// Searches a small radius for light sources, traces visibility, returns additive glow color.
// This provides the bright "hot spot" near sources that the block-level
// lightmap propagation can't capture.
fn compute_proximity_glow(wx: f32, wy: f32, time: f32) -> vec3<f32> {
    var glow = vec3<f32>(0.0);
    let max_search = 7; // covers floodlight range (directional, most energy within 7 tiles)
    let bx = i32(floor(wx));
    let by = i32(floor(wy));

    for (var dy: i32 = -max_search; dy <= max_search; dy++) {
        for (var dx: i32 = -max_search; dx <= max_search; dx++) {
            // Early reject: skip corners outside circular radius (avoid sqrt)
            let dsq = dx * dx + dy * dy;
            if dsq > max_search * max_search { continue; }

            let nx = bx + dx;
            let ny = by + dy;
            let nb = get_block(nx, ny);
            let bt = block_type(nb);

            // Check if this is a light source OR a burning flammable block
            let glow_mat = get_material(bt);
            var is_burning_glow = false;
            var burn_glow_i = 0.0;
            if glow_mat.is_flammable > 0.5 && glow_mat.light_intensity <= 0.0 {
                let b_idx = u32(ny) * u32(camera.grid_w) + u32(nx);
                let b_temp = block_temps[b_idx];
                if b_temp > glow_mat.ignition_temp {
                    is_burning_glow = true;
                    burn_glow_i = clamp((b_temp - glow_mat.ignition_temp) / 300.0, 0.0, 1.0);
                }
            }
            if glow_mat.light_intensity <= 0.0 && !is_burning_glow {
                continue;
            }

            let lcx = f32(nx) + 0.5;
            let lcy = f32(ny) + 0.5;
            let fdx = wx - lcx;
            let fdy = wy - lcy;
            let dist = sqrt(fdx * fdx + fdy * fdy);

            // Get material properties for this light source
            var radius = glow_mat.light_radius;
            var intensity = glow_mat.light_intensity;
            var light_col = vec3<f32>(glow_mat.light_color_r, glow_mat.light_color_g, glow_mat.light_color_b);
            let light_h = glow_mat.light_height;

            // Burning block glow: synthesize light properties from fire state
            if is_burning_glow {
                let phase = fire_hash(vec2<f32>(lcx, lcy)) * 6.28;
                let flicker = fire_flicker(time + phase);
                intensity = burn_glow_i * 0.7 * (0.6 + 0.4 * flicker);
                radius = 4.0 + burn_glow_i * 3.0;
                light_col = mix(FIRE_COLOR, FIRE_COLOR_HOT, burn_glow_i * flicker * 0.5);
            }

            // Wall torch: fire flicker (same as fireplace pattern)
            if bt == BT_WALL_TORCH {
                let phase = fire_hash(vec2<f32>(lcx, lcy)) * 6.28;
                let flicker = fire_flicker(time + phase);
                intensity *= (0.7 + 0.3 * flicker);
                let heat = clamp(1.0 - dist / 3.0, 0.0, 1.0);
                light_col = mix(light_col, FIRE_COLOR_HOT, heat * flicker);
            }

            // Electric lights: brightness proportional to voltage, off only at 0V
            if bt == BT_CEILING_LIGHT || bt == BT_FLOOR_LAMP || bt == BT_TABLE_LAMP || bt == BT_FLOODLIGHT || bt == BT_WALL_LAMP {
                let light_idx = u32(ny) * u32(camera.grid_w) + u32(nx);
                let lv = voltage[light_idx];
                if lv < 0.1 {
                    intensity = 0.0;
                } else {
                    // Smooth ramp: sqrt curve gives visible light even at low voltage
                    // 3.3V → 64%, 6V → 87%, 8V+ → 100%
                    let power_factor = sqrt(clamp(lv / 8.0, 0.0, 1.0));
                    intensity *= power_factor;
                    // Flicker when voltage is marginal (<2V)
                    if lv < 2.0 {
                        let pf = sin(time * 15.0 + f32(light_idx) * 3.7) * 0.4 + 0.6;
                        intensity *= pf;
                    }
                }
            }

            // Fire blocks: apply flicker animation
            if bt == BT_FIREPLACE || bt == BT_CAMPFIRE {
                let phase = fire_hash(vec2<f32>(lcx, lcy)) * 6.28;
                let flicker = fire_flicker(time + phase);
                intensity *= (0.7 + 0.3 * flicker);
                let heat = clamp(1.0 - dist / 3.0, 0.0, 1.0);
                light_col = mix(light_col, FIRE_COLOR_HOT, heat * flicker);
            }

            if dist > radius { continue; }

            // Floodlight: directional cone (rotation in flags bits 3-4)
            var dir_atten = 1.0;
            if bt == BT_FLOODLIGHT {
                let fl_flags = block_flags(nb);
                let fl_dir = (fl_flags >> 3u) & 3u;
                var light_dx = 0.0;
                var light_dy = 0.0;
                if fl_dir == 0u { light_dy = -1.0; } // N
                else if fl_dir == 1u { light_dx = 1.0; } // E
                else if fl_dir == 2u { light_dy = 1.0; } // S
                else { light_dx = -1.0; } // W
                // Cosine cone: dot product of normalized pixel direction with light direction
                if dist > 0.3 {
                    let to_pixel_x = fdx / dist;
                    let to_pixel_y = fdy / dist;
                    let cos_angle = to_pixel_x * light_dx + to_pixel_y * light_dy;
                    // Narrow cone: ~70° half-angle, sharp falloff at edges
                    dir_atten = smoothstep(-0.1, 0.4, cos_angle);
                    // Extra intensity boost in the center of the beam
                    dir_atten *= 1.0 + max(cos_angle, 0.0) * 0.5;
                }
                if dir_atten < 0.01 { continue; }
            }

            let vis = trace_glow_visibility(wx, wy, lcx, lcy, light_h);
            if vis < 0.01 { continue; }

            let atten = (1.0 / (1.0 + dist * 0.6 + dist * dist * 0.15))
                      * smoothstep(radius, radius * 0.15, dist);

            glow += light_col * intensity * atten * vis * dir_atten;
        }
    }

    return glow;
}

// Render fireplace block from top-down: stone hearth with animated fire
// Directional light bleeding through windows/doors.
// For outdoor pixels, scans nearby for glass/open doors, determines window orientation,
// samples interior light from the propagated lightmap behind the window, and projects
// it outward with angular + distance falloff — creating realistic light pools.
fn compute_directional_bleed(wx: f32, wy: f32) -> vec4<f32> {
    var total_light = 0.0;
    var total_color = vec3<f32>(0.0);
    let bx = i32(floor(wx));
    let by = i32(floor(wy));
    let search = 4;
    let max_range = 5.0;

    for (var dy: i32 = -search; dy <= search; dy++) {
        for (var dx: i32 = -search; dx <= search; dx++) {
            // Circular cull
            if dx * dx + dy * dy > search * search { continue; }

            let nx = bx + dx;
            let ny = by + dy;
            let nb = get_block(nx, ny);
            let bt = block_type(nb);

            // Only windows (glass) and open doors are portals
            let is_window = bt == BT_GLASS;
            let is_open_door = is_door(nb) && is_open(nb);
            if !is_window && !is_open_door { continue; }

            let wcx = f32(nx) + 0.5;
            let wcy = f32(ny) + 0.5;
            let to_pixel = vec2<f32>(wx - wcx, wy - wcy);
            let dist = length(to_pixel);
            if dist < 0.5 || dist > max_range { continue; }
            let dir = to_pixel / dist;

            // Determine window orientation from wall neighbors
            let left_h = block_height(get_block(nx - 1, ny));
            let right_h = block_height(get_block(nx + 1, ny));
            let top_h = block_height(get_block(nx, ny - 1));
            let bot_h = block_height(get_block(nx, ny + 1));

            let h_wall = (left_h > 0u) || (right_h > 0u);
            let v_wall = (top_h > 0u) || (bot_h > 0u);

            // Window normal candidates: perpendicular to wall run
            var normal1: vec2<f32>;
            var normal2: vec2<f32>;
            if h_wall && !v_wall {
                normal1 = vec2<f32>(0.0, 1.0);
                normal2 = vec2<f32>(0.0, -1.0);
            } else if v_wall && !h_wall {
                normal1 = vec2<f32>(1.0, 0.0);
                normal2 = vec2<f32>(-1.0, 0.0);
            } else {
                // Corner or standalone — use direction to pixel
                normal1 = dir;
                normal2 = -dir;
            }

            // Sample interior light behind the window in both directions.
            // The brighter side is the interior.
            let behind1 = sample_lightmap(wcx - normal1.x * 1.5, wcy - normal1.y * 1.5);
            let behind2 = sample_lightmap(wcx - normal2.x * 1.5, wcy - normal2.y * 1.5);

            var outward_normal: vec2<f32>;
            var interior_light: vec4<f32>;
            if behind1.w > behind2.w {
                outward_normal = normal1; // interior is behind normal1 → normal1 points outward
                interior_light = behind1;
            } else {
                outward_normal = normal2;
                interior_light = behind2;
            }

            if interior_light.w < 0.01 { continue; }

            // Angular falloff: bright directly in front of window, fades to sides
            let angle_factor = max(0.0, dot(dir, outward_normal));
            let angle_shaped = pow(angle_factor, 1.5); // focus into a cone

            // Distance falloff with gentle fade to zero
            let atten = (1.0 / (1.0 + dist * 0.4 + dist * dist * 0.15))
                      * smoothstep(max_range, max_range * 0.15, dist);

            // Open doors let through more light than glass panes
            var portal_mul = select(0.6, 1.0, is_open_door);

            let contribution = interior_light.w * angle_shaped * atten * portal_mul * 0.4;
            total_light += contribution;
            total_color += interior_light.xyz * contribution;
        }
    }

    if total_light > 0.001 {
        total_color /= total_light;
    }

    return vec4<f32>(total_color, total_light);
}

fn render_fireplace(wx: f32, wy: f32, fx: f32, fy: f32, time: f32) -> vec3<f32> {
    // Stone hearth base
    let hearth_color = vec3<f32>(0.32, 0.28, 0.25);
    let stone_var = fire_hash(vec2<f32>(floor(fx * 4.0), floor(fy * 4.0))) * 0.06;
    var color = hearth_color + vec3<f32>(stone_var);

    // Inner fire pit (circular, centered)
    let cx = fx - 0.5;
    let cy = fy - 0.5;
    let dist_center = sqrt(cx * cx + cy * cy);

    // Stone rim
    let rim_outer = 0.46;
    let rim_inner = 0.38;
    if dist_center < rim_outer && dist_center > rim_inner {
        let rim_t = (dist_center - rim_inner) / (rim_outer - rim_inner);
        color = mix(vec3<f32>(0.38, 0.34, 0.30), vec3<f32>(0.28, 0.24, 0.22), rim_t);
    }

    // Fire inside the pit
    if dist_center < rim_inner {
        // Ember bed — dark red/orange base
        let ember_base = vec3<f32>(0.25, 0.06, 0.02);

        // Animated embers: use noise to make glowing patches
        let ember_t = time * 1.5;
        let e1 = sin(wx * 17.0 + ember_t * 2.3) * sin(wy * 19.0 + ember_t * 1.7);
        let e2 = sin(wx * 31.0 - ember_t * 3.1) * sin(wy * 23.0 + ember_t * 2.9);
        let ember_glow = clamp(e1 * 0.5 + e2 * 0.3 + 0.3, 0.0, 1.0);
        let ember_color = mix(ember_base, vec3<f32>(0.9, 0.3, 0.05), ember_glow);

        // Flame tongues — bright dancing spots
        let flame_t = time * 3.0;
        let angle = atan2(cy, cx);
        let flame1 = sin(angle * 3.0 + flame_t * 4.7) * 0.5 + 0.5;
        let flame2 = sin(angle * 5.0 - flame_t * 6.3 + 1.5) * 0.5 + 0.5;
        let flame_intensity = flame1 * flame2;
        // Flames are strongest near center, fade toward rim
        let flame_radial = 1.0 - dist_center / rim_inner;
        let flame = flame_intensity * flame_radial * flame_radial;

        let flame_color = mix(FIRE_COLOR, FIRE_COLOR_HOT, flame);
        color = mix(ember_color, flame_color, clamp(flame * 1.5, 0.0, 1.0));

        // Overall flicker modulation
        let flicker = fire_flicker(time);
        color *= (0.7 + 0.3 * flicker);

        // Bright hot core at very center
        let core_t = max(0.0, 1.0 - dist_center / (rim_inner * 0.4));
        color = mix(color, FIRE_COLOR_HOT * (0.8 + 0.2 * flicker), core_t * core_t);
    }

    return color;
}

// Small campfire: 2x2 subtiles centered, half the power of a full fireplace
fn render_campfire(wx: f32, wy: f32, fx: f32, fy: f32, time: f32) -> vec3<f32> {
    // Ground color (dirt/stone base)
    let ground = vec3<f32>(0.28, 0.24, 0.20);
    let stone_var = fire_hash(vec2<f32>(floor(fx * 4.0), floor(fy * 4.0))) * 0.04;
    var color = ground + vec3<f32>(stone_var);

    // Small fire pit centered in tile, 2x2 subtiles = 0.5 tile width
    let cx = fx - 0.5;
    let cy = fy - 0.5;
    let dist_center = sqrt(cx * cx + cy * cy);

    // Small stone ring
    let rim_outer = 0.24;
    let rim_inner = 0.18;
    if dist_center < rim_outer && dist_center > rim_inner {
        let rim_t = (dist_center - rim_inner) / (rim_outer - rim_inner);
        color = mix(vec3<f32>(0.35, 0.31, 0.27), vec3<f32>(0.25, 0.22, 0.20), rim_t);
    }

    // Fire inside
    if dist_center < rim_inner {
        let ember_base = vec3<f32>(0.22, 0.05, 0.02);
        let ember_t = time * 1.5;
        let e1 = sin(wx * 17.0 + ember_t * 2.3) * sin(wy * 19.0 + ember_t * 1.7);
        let e2 = sin(wx * 31.0 - ember_t * 3.1) * sin(wy * 23.0 + ember_t * 2.9);
        let ember_glow = clamp(e1 * 0.5 + e2 * 0.3 + 0.3, 0.0, 1.0);
        let ember_color = mix(ember_base, vec3<f32>(0.85, 0.28, 0.05), ember_glow);

        let flame_t = time * 3.0;
        let angle = atan2(cy, cx);
        let flame1 = sin(angle * 3.0 + flame_t * 4.7) * 0.5 + 0.5;
        let flame2 = sin(angle * 5.0 - flame_t * 6.3 + 1.5) * 0.5 + 0.5;
        let flame_intensity = flame1 * flame2;
        let flame_radial = 1.0 - dist_center / rim_inner;
        let flame = flame_intensity * flame_radial * flame_radial;

        let flame_color = mix(FIRE_COLOR, FIRE_COLOR_HOT, flame);
        color = mix(ember_color, flame_color, clamp(flame * 1.5, 0.0, 1.0));

        let flicker = fire_flicker(time);
        color *= (0.7 + 0.3 * flicker);

        let core_t = max(0.0, 1.0 - dist_center / (rim_inner * 0.4));
        color = mix(color, FIRE_COLOR_HOT * (0.7 + 0.2 * flicker), core_t * core_t);
    }

    return color;
}

// Wall torch: iron bracket + flame rendered at the edge nearest the wall
fn render_wall_torch(fx: f32, fy: f32, flags: u32, time: f32, wx: f32, wy: f32) -> vec3<f32> {
    let dir = (flags >> 3u) & 3u; // 0=N, 1=E, 2=S, 3=W
    // Transform coordinates so the wall edge is always at local_y = 0
    var lx = 0.0; var ly = 0.0;
    if dir == 0u      { lx = fx; ly = fy; }           // N: wall at top, ly=0 is wall edge
    else if dir == 1u { lx = fy; ly = 1.0 - fx; }     // E: wall at right
    else if dir == 2u { lx = 1.0 - fx; ly = 1.0 - fy; } // S: wall at bottom
    else              { lx = 1.0 - fy; ly = fx; }     // W: wall at left

    var color = block_base_color(2u, 0u); // ground base

    // Bracket: iron mount at wall edge (ly < 0.2, centered on lx)
    let bracket_cx = abs(lx - 0.5);
    if ly < 0.18 && bracket_cx < 0.08 {
        color = vec3(0.30, 0.28, 0.25); // dark iron
        // Bracket arm
        if ly > 0.06 && bracket_cx < 0.04 {
            color = vec3(0.35, 0.32, 0.28);
        }
    }

    // Flame: small flickering fire at tip of bracket
    let flame_cx = lx - 0.5;
    let flame_cy = ly - 0.12;
    let flame_dist = length(vec2(flame_cx, flame_cy));
    if flame_dist < 0.10 {
        let phase = fire_hash(vec2(wx, wy)) * 6.28;
        let flicker = fire_flicker(time + phase);
        let flame_t = 1.0 - flame_dist / 0.10;
        let flame_color = mix(FIRE_COLOR, FIRE_COLOR_HOT, flame_t * flicker);
        color = mix(color, flame_color, flame_t * (0.7 + 0.3 * flicker));
    }

    return color;
}

// Wall lamp: metallic fixture with warm-white glow at wall edge
fn render_wall_lamp(fx: f32, fy: f32, flags: u32, bx: i32, by: i32) -> vec3<f32> {
    let dir = (flags >> 3u) & 3u;
    // Transform so wall edge is at local_y = 0
    var lx = 0.0; var ly = 0.0;
    if dir == 0u      { lx = fx; ly = fy; }
    else if dir == 1u { lx = fy; ly = 1.0 - fx; }
    else if dir == 2u { lx = 1.0 - fx; ly = 1.0 - fy; }
    else              { lx = 1.0 - fy; ly = fx; }

    var color = block_base_color(2u, 0u); // ground base

    // Fixture body: compact metallic housing at wall edge
    let body_cx = abs(lx - 0.5);
    let body_cy = ly;
    if body_cy < 0.20 && body_cx < 0.12 {
        color = vec3(0.55, 0.53, 0.50); // brushed metal
        // Metallic sheen
        let sheen = 1.0 - body_cy / 0.20;
        color += vec3(0.06) * sheen;
        // Rim
        if body_cx > 0.09 || body_cy > 0.17 {
            color = vec3(0.40, 0.38, 0.36);
        }
    }

    // Lens/glow: bright when powered
    let lens_cx = lx - 0.5;
    let lens_cy = ly - 0.10;
    let lens_dist = length(vec2(lens_cx, lens_cy));
    if lens_dist < 0.07 {
        let vidx = u32(by) * u32(camera.grid_w) + u32(bx);
        let lv = voltage[vidx];
        let power = sqrt(clamp(lv / 8.0, 0.0, 1.0));
        let lens_t = 1.0 - lens_dist / 0.07;
        color = mix(vec3(0.45, 0.43, 0.40), vec3(0.95, 0.92, 0.85) * (0.5 + power), lens_t * lens_t);
    }

    return color;
}

// --- Noise functions for fire rendering ---
// 2D value noise with smooth interpolation
fn noise2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f); // smoothstep interpolation
    let a = fire_hash(i);
    let b = fire_hash(i + vec2<f32>(1.0, 0.0));
    let c = fire_hash(i + vec2<f32>(0.0, 1.0));
    let d = fire_hash(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Fractal Brownian motion — layered noise for turbulent fire shapes
fn fbm_fire(p: vec2<f32>, octaves: i32) -> f32 {
    var val = 0.0;
    var amp = 0.5;
    var freq = 1.0;
    var pos = p;
    for (var i = 0; i < octaves; i++) {
        val += amp * noise2d(pos * freq);
        freq *= 2.0;
        amp *= 0.5;
        pos += vec2<f32>(1.7, 3.1); // offset each octave to reduce axis-alignment
    }
    return val;
}

// Fire color ramp: maps noise value 0-1 to fire color
// 0.0 = transparent/black, 0.3 = deep red, 0.5 = orange, 0.7 = yellow, 0.9 = white
fn fire_color_ramp(t: f32) -> vec3<f32> {
    if t < 0.25 {
        // Black → deep red
        let s = t / 0.25;
        return mix(vec3<f32>(0.05, 0.0, 0.0), vec3<f32>(0.6, 0.05, 0.0), s);
    } else if t < 0.5 {
        // Deep red → orange
        let s = (t - 0.25) / 0.25;
        return mix(vec3<f32>(0.6, 0.05, 0.0), vec3<f32>(1.0, 0.4, 0.0), s);
    } else if t < 0.75 {
        // Orange → yellow
        let s = (t - 0.5) / 0.25;
        return mix(vec3<f32>(1.0, 0.4, 0.0), vec3<f32>(1.0, 0.8, 0.1), s);
    } else {
        // Yellow → white-hot
        let s = (t - 0.75) / 0.25;
        return mix(vec3<f32>(1.0, 0.8, 0.1), vec3<f32>(1.0, 0.95, 0.8), s);
    }
}

// Render fire overlay for burning blocks. Returns RGBA where A = fire opacity.
// Uses FBM noise scrolling upward to create realistic turbulent flame shapes.
fn render_fire_overlay(wx: f32, wy: f32, fx: f32, fy: f32, time: f32, intensity: f32) -> vec4<f32> {
    // Per-tile offset so adjacent fires don't look identical
    let tile_offset = fire_hash(vec2<f32>(floor(wx), floor(wy))) * 100.0;

    // --- Flame shape: noise field in WORLD space (no tile boundaries) ---
    let noise_scale = 4.0;
    let rise_speed = 3.0 * (0.8 + 0.4 * intensity);
    let noise_uv = vec2<f32>(
        wx * noise_scale,
        wy * noise_scale + time * rise_speed
    );

    // Multi-octave turbulent noise for flame shape
    let flame_noise = fbm_fire(noise_uv, 4);

    // --- Flame mask: radial from tile center, noise-modulated edge ---
    // Small fire = small circle. Full fire = fills tile.
    let cx = fx - 0.5;
    let cy = fy - 0.5;
    let dist = sqrt(cx * cx + cy * cy);

    // Fire radius grows with intensity: 0.12 (tiny) → 0.48 (full tile)
    let base_radius = 0.12 + intensity * 0.36;

    // Noise wobbles the edge so it's organic, not a perfect circle
    let edge_noise = fbm_fire(vec2<f32>(
        wx * 6.0 + tile_offset * 0.3,
        wy * 6.0 - time * 1.5
    ), 3);
    let noisy_radius = base_radius + (edge_noise - 0.5) * 0.12;

    // Smooth falloff from center
    let irregular_edge = smoothstep(noisy_radius, noisy_radius * 0.4, dist);

    // Combine noise with radial mask
    let threshold = 0.55 - intensity * 0.3;
    let raw_flame = (flame_noise - threshold) / (1.0 - threshold);
    let flame = clamp(raw_flame * irregular_edge, 0.0, 1.0);

    // --- Color: map flame intensity through the color ramp ---
    let color_val = flame * (0.7 + 0.3 * intensity); // boost toward white at high intensity
    var fire_col = fire_color_ramp(color_val);

    // Flicker modulation (whole-tile brightness variation)
    let flicker = fire_flicker(time + tile_offset);
    fire_col *= (0.75 + 0.25 * flicker);

    // --- Ember base: glowing patches beneath the flames, within fire radius ---
    let ember_noise = fbm_fire(vec2<f32>(wx * 3.0 + time * 0.5, wy * 3.0 + time * 0.3), 3);
    let ember_glow = clamp(ember_noise * 2.0 - 0.6, 0.0, 1.0) * intensity * irregular_edge;
    let ember_col = mix(vec3<f32>(0.15, 0.02, 0.0), vec3<f32>(0.8, 0.2, 0.02), ember_glow);

    // Blend embers under flames
    var final_color = mix(ember_col, fire_col, clamp(flame * 2.0, 0.0, 1.0));

    // --- Sparks: bright points drifting upward ---
    let spark_noise = noise2d(vec2<f32>(wx * 12.0 + tile_offset, wy * 12.0 - time * 8.0));
    let spark = smoothstep(0.92, 0.96, spark_noise) * intensity;
    if spark > 0.01 {
        final_color = mix(final_color, vec3<f32>(1.0, 0.95, 0.7), spark);
    }

    // --- Opacity: fire is visible where flame or embers are ---
    let opacity = clamp(flame * 1.5 + ember_glow * 0.5 + spark, 0.0, 1.0);

    // --- Charring: darken the block underneath (applied via reduced opacity at edges) ---
    // Blocks should look charred even where flames aren't directly visible
    let char_amount = intensity * 0.3 * (1.0 - flame);
    final_color = mix(final_color, vec3<f32>(0.05, 0.03, 0.02), char_amount * irregular_edge);

    return vec4(final_color, clamp(opacity, 0.0, 1.0));
}

// Render electric light block from top-down: ceiling-mounted fixture seen from above
fn render_electric_light(wx: f32, wy: f32, fx: f32, fy: f32, time: f32) -> vec3<f32> {
    // Floor beneath the light
    let floor_color = vec3<f32>(0.45, 0.35, 0.20); // dirt floor base
    var color = floor_color;

    // Ceiling fixture: small bright disk in center
    let cx = fx - 0.5;
    let cy = fy - 0.5;
    let dist = sqrt(cx * cx + cy * cy);

    // Outer glow ring (light spill on ceiling seen from above)
    let glow_radius = 0.4;
    if dist < glow_radius {
        let glow_t = 1.0 - dist / glow_radius;
        color = mix(color, ELIGHT_COLOR * 0.5, glow_t * glow_t * 0.4);
    }

    // Fixture body: small circular plate
    let fixture_radius = 0.18;
    if dist < fixture_radius {
        let fixture_t = 1.0 - dist / fixture_radius;
        let fixture_color = vec3<f32>(0.7, 0.7, 0.72); // metallic grey
        color = mix(fixture_color, ELIGHT_COLOR * 1.1, fixture_t * fixture_t);
    }

    // Bright center bulb
    let bulb_radius = 0.08;
    if dist < bulb_radius {
        color = ELIGHT_COLOR * 1.2;
    }

    return color;
}

// Render tree from top-down using sprite atlas.
// Returns vec4(color_rgb, is_canopy): is_canopy > 0 if this pixel is tree, 0 if ground beneath.
// Render bench from top-down: wooden plank seat with visible legs
fn render_bench(fx: f32, fy: f32, flags: u32) -> vec3<f32> {
    let segment = (flags >> 3u) & 3u;  // 0=left/top, 1=center, 2=right/bottom
    let rotation = (flags >> 5u) & 3u; // 0=horizontal, 1=vertical

    // Transform local coords based on rotation
    var lx = fx; // along bench length
    var ly = fy; // across bench width
    if rotation == 1u {
        lx = fy;
        ly = fx;
    }

    let wood_color = vec3<f32>(0.55, 0.38, 0.18);
    let wood_dark = vec3<f32>(0.40, 0.28, 0.12);
    let leg_color = vec3<f32>(0.35, 0.24, 0.10);
    let ground = vec3<f32>(0.45, 0.35, 0.20); // dirt beneath

    // Bench seat: centered strip across the width
    let seat_min = 0.2;
    let seat_max = 0.8;
    let on_seat = ly >= seat_min && ly <= seat_max;

    if !on_seat {
        // Ground visible on either side of the bench
        return ground;
    }

    // Plank grain lines running along the bench length
    let grain = fract(ly * 6.0);
    let grain_line = f32(grain < 0.08) * 0.04;
    var color = wood_color - vec3<f32>(grain_line);

    // Slight color variation per plank
    let plank_id = floor(ly * 3.0);
    let plank_var = fract(sin(plank_id * 127.1) * 43758.5453) * 0.06 - 0.03;
    color += vec3<f32>(plank_var);

    // Legs: small dark squares at the ends and middle of each segment
    let leg_size = 0.12;
    let at_edge_x = lx < leg_size || lx > (1.0 - leg_size);
    let at_edge_y = ly < (seat_min + leg_size) || ly > (seat_max - leg_size);

    // End segments (0, 2) have legs at the outer end; center (1) has no end legs
    var show_leg = false;
    if segment == 0u && lx < leg_size && at_edge_y {
        show_leg = true;
    }
    if segment == 2u && lx > (1.0 - leg_size) && at_edge_y {
        show_leg = true;
    }
    // All segments have a subtle cross-brace shadow in the middle
    if segment == 1u {
        let mid_brace = abs(lx - 0.5) < 0.04 && at_edge_y;
        if mid_brace {
            color = wood_dark;
        }
    }

    if show_leg {
        color = leg_color;
    }

    return color;
}

// Render bed from top-down: 2-tile piece with pillow (head) and blanket (foot)
// segment 0 = head (pillow end), segment 1 = foot (blanket end)
fn render_bed(fx: f32, fy: f32, flags: u32) -> vec3<f32> {
    let segment = (flags >> 3u) & 1u;  // 0=head, 1=foot
    let rotation = (flags >> 5u) & 3u; // 0=horizontal, 1=vertical

    // Transform local coords based on rotation
    var lx = fx; // along bed length
    var ly = fy; // across bed width
    if rotation == 1u {
        lx = fy;
        ly = fx;
    }

    let frame_color = vec3<f32>(0.42, 0.30, 0.16);  // dark wood frame
    let sheet_color = vec3<f32>(0.82, 0.80, 0.75);   // white-ish sheets
    let blanket_color = vec3<f32>(0.28, 0.22, 0.48); // purple blanket
    let pillow_color = vec3<f32>(0.90, 0.88, 0.82);  // off-white pillow
    let ground = vec3<f32>(0.45, 0.35, 0.20);

    // Bed frame: 0.08 border on sides, 0.12 at head/foot ends
    let frame_side = 0.08;
    let frame_end = 0.12;
    let on_frame_side = ly < frame_side || ly > (1.0 - frame_side);
    let on_frame_head = segment == 0u && lx < frame_end;
    let on_frame_foot = segment == 1u && lx > (1.0 - frame_end);

    if on_frame_side || on_frame_head || on_frame_foot {
        // Frame with wood grain
        let grain = fract(lx * 8.0);
        let grain_var = f32(grain < 0.06) * 0.03;
        return frame_color - vec3<f32>(grain_var);
    }

    // Interior: sheets and pillow/blanket
    if segment == 0u {
        // Head segment: pillow + upper sheet
        // Pillow: plump rounded rectangle in the first 0.45 of the segment
        let pillow_end = 0.50;
        if lx < pillow_end {
            let px = (lx - pillow_end * 0.5) / (pillow_end * 0.5);
            let py = (ly - 0.5) / 0.42;
            let pdist = px * px + py * py;
            if pdist < 1.0 {
                // Pillow: soft rounded shape with subtle crease
                let puff = 1.0 - pdist;
                let crease = abs(py) < 0.15 && px > -0.3 && px < 0.3;
                var pc = pillow_color * (0.92 + puff * 0.08);
                if crease {
                    pc *= 0.96; // subtle indent
                }
                return pc;
            }
        }
        // Sheet area (rest of head segment)
        let wrinkle = sin(lx * 15.0 + ly * 3.0) * 0.015;
        return sheet_color + vec3<f32>(wrinkle);
    } else {
        // Foot segment: blanket with folds
        let fold1 = sin(lx * 6.0 + 0.5) * 0.02;
        let fold2 = sin(ly * 8.0 + lx * 4.0) * 0.015;
        let edge_fade = smoothstep(0.0, 0.15, lx); // blanket starts folded at the join
        var bc = mix(sheet_color, blanket_color, edge_fade);
        bc += vec3<f32>(fold1 + fold2);
        // Subtle stitch pattern along blanket
        let stitch_x = fract(lx * 12.0);
        let stitch_y = fract(ly * 12.0);
        if stitch_x < 0.06 || stitch_y < 0.06 {
            bc *= 0.97;
        }
        return bc;
    }
}

// Render berry bush from top-down: leafy green mound with red berry dots
fn render_berry_bush(fx: f32, fy: f32, world_x: f32, world_y: f32, time: f32) -> vec4<f32> {
    let cx = fx - 0.5;
    let cy = fy - 0.5;
    let dist = sqrt(cx * cx + cy * cy);
    let ground = vec3<f32>(0.45, 0.35, 0.20);

    // Bush is a circular mound, radius ~0.42
    let bush_r = 0.42;
    if dist > bush_r {
        return vec4<f32>(ground, 0.0); // ground beneath, no canopy
    }

    // Leafy base color with variation
    let leaf_dark = vec3<f32>(0.15, 0.32, 0.10);
    let leaf_light = vec3<f32>(0.28, 0.48, 0.18);
    let noise1 = fract(sin(world_x * 17.3 + world_y * 23.1) * 43758.5);
    let noise2 = fract(sin(fx * 31.0 + fy * 47.0 + world_x * 7.0) * 27183.6);
    let leaf_mix = noise1 * 0.5 + noise2 * 0.5;
    var color = mix(leaf_dark, leaf_light, leaf_mix);

    // Rounded mound shading (brighter at center, darker at edges)
    let height_factor = 1.0 - (dist / bush_r);
    color *= (0.75 + height_factor * 0.25);

    // Leaf texture: small irregular patches
    let leaf_cell = fract(vec2(fx * 5.0, fy * 5.0));
    let leaf_edge = min(leaf_cell.x, min(leaf_cell.y, min(1.0 - leaf_cell.x, 1.0 - leaf_cell.y)));
    if leaf_edge < 0.08 {
        color *= 0.88; // dark gaps between leaf clusters
    }

    // Berries: scattered red dots
    // Use world position hash for stable placement
    let berry_hash1 = fract(sin(world_x * 127.1 + world_y * 311.7) * 43758.5);
    let berry_hash2 = fract(sin(world_x * 269.5 + world_y * 183.3) * 27183.6);
    let berry_hash3 = fract(sin(world_x * 419.2 + world_y * 97.4) * 31415.9);

    // 4-5 berry positions per bush tile
    let berry_positions = array<vec2<f32>, 5>(
        vec2(0.3, 0.35),
        vec2(0.65, 0.3),
        vec2(0.45, 0.6),
        vec2(0.25, 0.55),
        vec2(0.6, 0.65)
    );
    let berry_r = 0.04;
    let berry_color = vec3<f32>(0.75, 0.12, 0.15); // deep red
    let berry_highlight = vec3<f32>(0.90, 0.35, 0.25); // bright spot

    for (var b = 0u; b < 5u; b++) {
        // Offset each berry slightly by world hash for variety
        let bp = berry_positions[b] + vec2(
            fract(sin(f32(b) * 73.0 + berry_hash1 * 100.0) * 438.5) * 0.08 - 0.04,
            fract(sin(f32(b) * 91.0 + berry_hash2 * 100.0) * 271.8) * 0.08 - 0.04
        );
        let bdist = length(vec2(fx, fy) - bp);
        if bdist < berry_r {
            // Berry with highlight
            let bt = bdist / berry_r;
            color = mix(berry_highlight, berry_color, bt);
            // Tiny specular dot
            if bdist < berry_r * 0.3 {
                color = mix(vec3(1.0, 0.9, 0.8), color, bdist / (berry_r * 0.3));
            }
        }
    }

    // Subtle wind sway
    let sway = sin(world_x * 3.0 + time * 1.5) * sin(world_y * 2.7 + time * 1.2) * 0.02;
    color += vec3<f32>(sway, sway * 0.5, 0.0);

    // Return with height for shadow casting (low bush, ~0.4 height equivalent)
    return vec4<f32>(color, 0.3);
}

// Render standing lamp from top-down: thin pole with circular shade
fn render_standing_lamp(fx: f32, fy: f32, time: f32) -> vec3<f32> {
    let cx = fx - 0.5;
    let cy = fy - 0.5;
    let dist = sqrt(cx * cx + cy * cy);
    let floor_color = vec3<f32>(0.45, 0.35, 0.20);

    // Lamp shade: warm glowing circle
    let shade_r = 0.30;
    if dist < shade_r {
        let shade_t = 1.0 - dist / shade_r;
        let warm = vec3<f32>(0.95, 0.85, 0.60);
        let bright = vec3<f32>(1.0, 0.95, 0.80);
        return mix(warm, bright, shade_t * shade_t);
    }

    // Pole shadow on floor (thin dark line)
    let pole_r = 0.04;
    if dist < pole_r {
        return floor_color * 0.6;
    }

    return floor_color;
}

// Render table lamp from top-down: warm glowing circle on bench surface
// Returns vec4(color, is_lamp_bulb). is_lamp_bulb > 0 = emissive circle, 0 = bench surface.
fn render_table_lamp(fx: f32, fy: f32) -> vec4<f32> {
    let cx = fx - 0.5;
    let cy = fy - 0.5;
    let dist = sqrt(cx * cx + cy * cy);

    let shade_r = 0.18;
    if dist > shade_r {
        // Bench surface — not emissive, receives normal lighting
        let bench_color = vec3<f32>(0.55, 0.38, 0.18);
        let grain = fract(fy * 6.0);
        let grain_line = f32(grain < 0.08) * 0.03;
        return vec4<f32>(bench_color - vec3<f32>(grain_line), 0.0);
    }

    // Warm glowing circle (emissive)
    let t = 1.0 - dist / shade_r;
    let outer = vec3<f32>(0.85, 0.65, 0.35);
    let inner = vec3<f32>(1.0, 0.92, 0.75);
    return vec4<f32>(mix(outer, inner, t * t), 1.0);
}

fn render_tree(wx: f32, wy: f32, tree_tx: f32, tree_ty: f32, height: u32, flags: u32) -> vec4<f32> {
    let tree_id = tree_tx * 137.0 + tree_ty * 311.0;
    let id_hash = fract(sin(tree_id) * 43758.5453);
    // Pick conifer variant (0-7)
    let variant = u32(id_hash * 8.0) % 8u;

    // Size: tiles the sprite covers
    let size_factor = select(
        select(select(2.0, 2.8, height >= 3u), 3.5, height >= 4u),
        4.0, height >= 5u
    );

    // Random offset ±0.25 tiles
    let offset_x = (fract(sin(tree_id * 1.3 + 7.1) * 31415.9) - 0.5) * 0.5;
    let offset_y = (fract(sin(tree_id * 2.7 + 3.9) * 27183.6) - 0.5) * 0.5;
    let center_x = tree_tx + 0.5 + offset_x;
    let center_y = tree_ty + 0.5 + offset_y;

    // World pixel → sprite UV
    let cu = 0.5 + (wx - center_x) / size_factor;
    let cv = 0.5 - (wy - center_y) / size_factor;

    if cu < 0.0 || cu > 1.0 || cv < 0.0 || cv > 1.0 {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let sprite = sample_sprite(variant, cu, cv);
    if sprite.w < 0.05 {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let is_trunk = sprite.w < 0.4;
    var color = sprite.xyz;

    color *= (0.88 + id_hash * 0.24);

    if is_trunk {
        return vec4<f32>(color * 0.6, 0.05);
    }
    return vec4<f32>(color, sprite.w);
}

fn compute_interior_light(wx: f32, wy: f32, sun_intensity: f32, sun_dir: vec2<f32>, window_amb: f32) -> vec4<f32> {
    // Sun-only interior lighting (fire is handled separately in the main shader)
    // window_amb is pre-computed from the lightmap (window proximity fill)

    // Direct sunbeam: trace toward sun to see if we exit through glass (per-pixel)
    let beam = trace_interior_sun_ray(wx, wy, sun_dir);
    let beam_tint = beam.xyz;
    let beam_light = beam.w;

    // Combine: base ambient + window proximity fill + direct sunbeam
    let ambient_light = INTERIOR_INDIRECT + window_amb * INTERIOR_WINDOW_AMBIENT;
    let direct_light = beam_light * INTERIOR_SUNBEAM * sun_intensity;
    let sun_total = ambient_light + direct_light;

    // Compute tint from sunbeam (glass color when sun comes through windows)
    let sun_tint = mix(vec3<f32>(1.0), beam_tint, clamp(direct_light / max(sun_total, 0.01), 0.0, 1.0));

    return vec4<f32>(sun_tint, sun_total);
}

// --- Color palette ---
fn block_base_color(btype: u32, flags: u32) -> vec3<f32> {
    switch btype {
        case 0u: { return vec3<f32>(0.05, 0.05, 0.08); }   // air/void
        case 1u: { return vec3<f32>(0.52, 0.50, 0.48); }    // stone
        case 2u: { return vec3<f32>(0.45, 0.35, 0.20); }    // dirt
        case 3u: { return vec3<f32>(0.12, 0.30, 0.55); }    // water
        case 4u: {
            if (flags & 1u) != 0u {
                return vec3<f32>(0.50, 0.38, 0.22);          // door
            }
            return vec3<f32>(0.58, 0.56, 0.52);              // wall
        }
        case 5u: { return vec3<f32>(0.65, 0.78, 0.88); }    // glass
        case 6u: { return vec3<f32>(0.35, 0.30, 0.28); }    // fireplace stone
        case 7u: { return vec3<f32>(0.45, 0.35, 0.20); }    // electric light (floor beneath)
        case 8u: { return vec3<f32>(0.18, 0.35, 0.12); }    // tree (canopy green)
        case 9u: { return vec3<f32>(0.55, 0.38, 0.18); }    // bench (wood)
        case 10u: { return vec3<f32>(0.45, 0.35, 0.20); }   // standing lamp (floor beneath)
        case 11u: { return vec3<f32>(0.55, 0.38, 0.18); }   // table lamp (bench beneath)
        default: {
            let m = get_material(btype);
            return vec3<f32>(m.color_r, m.color_g, m.color_b);
        }
    }
}

// Roof color with tile pattern
// Corrugated steel roof: dark gray with parallel ridges
fn roof_color(wx: f32, wy: f32) -> vec3<f32> {
    let base = vec3<f32>(0.28, 0.27, 0.26); // dark steel gray
    // Corrugation ridges running east-west (along X)
    let ridge = sin(wy * 3.14159265 * 4.0) * 0.5 + 0.5; // 4 ridges per block
    let highlight = ridge * ridge * 0.06; // subtle bright line on ridge crests
    return base + vec3<f32>(highlight);
}

// --- Pixel-level shadow ray ---
// Traces from a world position toward the sun, accumulating occlusion.
// Returns: vec4(tint_rgb, light_factor)
fn trace_shadow_ray(wx: f32, wy: f32, surface_height: f32, sun_dir: vec2<f32>, sun_elev: f32) -> vec4<f32> {
    // At night (no sun), everything is in shadow — light_factor = 0
    if camera.sun_intensity < 0.001 { return vec4<f32>(1.0, 1.0, 1.0, 0.0); }
    let dir2d = normalize(sun_dir);
    let step_x = dir2d.x * SHADOW_STEP;
    let step_y = dir2d.y * SHADOW_STEP;
    let step_h = sun_elev * SHADOW_STEP;

    // Include terrain elevation in the starting height
    let start_elev = sample_elevation(wx, wy);
    var current_h = surface_height + start_elev;
    var sx = wx;
    var sy = wy;
    var light = 1.0;
    var tint = vec3<f32>(1.0, 1.0, 1.0);

    let max_steps = i32(SHADOW_MAX_DIST / SHADOW_STEP);
    for (var i: i32 = 0; i < max_steps; i++) {
        sx += step_x;
        sy += step_y;
        current_h += step_h;

        let bx = i32(floor(sx));
        let by = i32(floor(sy));
        let block = get_block(bx, by);
        // Include terrain elevation in obstacle height
        let sample_elev = sample_elevation(sx, sy);
        let bh = f32(block_height(block)) + sample_elev;
        let bt = block_type(block);

        // Shadow occlusion logic:
        // - Roofed floor tiles: roof plane blocks ray when it climbs to roof height,
        //   but interior airspace below the roof is open for lateral light.
        // - Glass blocks: only the window opening (between sill and lintel) transmits
        //   tinted light; the wall below sill and above lintel is opaque.
        // - Other structural blocks use max(block_height, roof_height).
        let rh = get_roof_height(bx, by);
        let is_roofed_floor = has_roof(block) && bh < 0.5;

        // Pipe components (15-20), dug ground (32), crates (33), rocks (34) don't cast shadows
        let is_pipe_block = (bt >= BT_PIPE && bt <= BT_INLET) || bt == BT_RESTRICTOR || bt == BT_LIQUID_PIPE || bt == BT_PIPE_BRIDGE || bt == BT_LIQUID_INTAKE || bt == BT_LIQUID_PUMP || bt == BT_LIQUID_OUTPUT;
        let is_dug_block = bt == BT_DUG_GROUND;
        let is_crate_block = bt == BT_CRATE; // height = item count, not visual
        let is_rock_block = bt == BT_ROCK;
        let is_wire_block = bt == BT_WIRE || bt == BT_WIRE_BRIDGE; // height = connection mask, not visual
        let is_dimmer_block = bt == BT_DIMMER || bt == BT_FIREPLACE; // height = level/intensity, not visual
        let is_breaker_block = bt == BT_BREAKER; // height = trip threshold, not visual
        let is_plant_block = false; // berry bush + crop now cast soft shadows (handled below)

        // Diagonal wall: only occlude if ray is on the wall half
        let is_diag_block = bt == BT_DIAGONAL;
        var diag_open = false;
        if is_diag_block {
            let sfx = fract(sx);
            let sfy = fract(sy);
            let svar = (block_flags(block) >> 3u) & 3u;
            diag_open = !diag_is_wall(sfx, sfy, svar);
        }

        var effective_h = select(bh, 0.0, is_pipe_block || is_dug_block || is_crate_block || is_rock_block || is_wire_block || is_dimmer_block || is_breaker_block || is_plant_block || diag_open);

        // Wall_data shadow: sub-pixel check at TWO positions per step
        // (endpoint + midpoint) to never miss thin wall strips (≥0.25 wide).
        let gw = i32(camera.grid_w);
        let gh = i32(camera.grid_h);
        for (var si = 0u; si < 2u; si++) {
            // si=0: midpoint, si=1: endpoint
            let t = select(0.5, 1.0, si == 1u);
            let sample_sx = sx - step_x * (1.0 - t);
            let sample_sy = sy - step_y * (1.0 - t);
            let sbx = i32(floor(sample_sx));
            let sby = i32(floor(sample_sy));
            if sbx < 0 || sby < 0 || sbx >= gw || sby >= gh { continue; }
            let s_wd = read_wall_data(u32(sby) * u32(gw) + u32(sbx));
            if (s_wd & 0xFu) == 0u { continue; }
            let s_open = (s_wd & 0x400u) != 0u && (s_wd & 0x800u) != 0u;
            if s_open { continue; }
            if wd_pixel_is_wall(fract(sample_sx), fract(sample_sy), s_wd) {
                let s_elev = sample_elevation(sample_sx, sample_sy);
                let wall_h = wall_material_height(wd_material_s(s_wd)) + s_elev;
                if wall_h > effective_h { effective_h = wall_h; }
            }
        }

        if is_roofed_floor {
            // The roof is a thin plane at height rh. Rather than a hard threshold
            // that flickers, always set effective_h to rh but apply a smooth
            // attenuation based on how close the ray is to the roof plane.
            // This is handled below in the occlusion test.
            effective_h = rh;
        } else if rh > effective_h {
            effective_h = rh;
        }

        // --- Tree: sprite-shaped semi-transparent shadow ---
        if bt == BT_TREE {
            let tree_flags = block_flags(block);
            let is_large = (tree_flags & 32u) != 0u;
            let quadrant = (tree_flags >> 3u) & 3u;

            var tree_fx = fract(sx);
            var tree_fy = fract(sy);
            var origin_x = f32(bx);
            var origin_y = f32(by);

            if is_large {
                if (quadrant & 1u) != 0u { origin_x -= 1.0; }
                if (quadrant & 2u) != 0u { origin_y -= 1.0; }
                let qx = f32(quadrant & 1u);
                let qy = f32((quadrant >> 1u) & 1u);
                tree_fx = (qx + fract(sx)) * 0.5;
                tree_fy = (qy + fract(sy)) * 0.5;
            }

            let tree_id = origin_x * 137.0 + origin_y * 311.0;
            let tree_hash = fract(sin(tree_id) * 43758.5453);
            let variant = u32(tree_hash * f32(SPRITE_VARIANTS)) % SPRITE_VARIANTS;

            // Same rotation as rendering (full 360°)
            let rot_hash = fract(sin(tree_id * 1.7 + 5.3) * 27183.6142);
            let angle = rot_hash * 6.2832;
            var ru = tree_fx - 0.5;
            var rv = tree_fy - 0.5;
            let cos_a = cos(angle);
            let sin_a = sin(angle);
            let rotated_u = ru * cos_a - rv * sin_a;
            let rotated_v = ru * sin_a + rv * cos_a;
            ru = rotated_u;
            rv = rotated_v;
            let sprite = sample_sprite(variant, ru + 0.5, rv + 0.5);

            if sprite.w > 0.01 {
                // Sprite height scaled to block height
                let sprite_h = sprite.w * bh;

                if sprite_h > current_h {
                    // Per-tree density variation (some trees denser than others)
                    let tree_density = 1.0 - camera.foliage_variation + tree_hash * camera.foliage_variation * 2.0;

                    // Dappled light: high-frequency noise simulating gaps between leaves
                    // Varies with position AND time for subtle wind shimmer
                    let dapple_seed = sx * 127.1 + sy * 311.7 + camera.time * 0.5;
                    let dapple1 = fract(sin(dapple_seed) * 43758.5453);
                    let dapple2 = fract(sin(dapple_seed * 1.7 + 3.1) * 27183.6142);
                    let dapple = 0.5 + 0.5 * (dapple1 * 0.7 + dapple2 * 0.3);

                    // Center of canopy is denser (sprite UV center = 0.5, 0.5)
                    let center_dist = length(vec2<f32>(tree_fx - 0.5, tree_fy - 0.5)) * 2.0;
                    let depth_factor = 1.0 - center_dist * 0.4; // center=1.0, edge=0.6

                    // Combined opacity per step
                    let opacity = camera.foliage_opacity * tree_density * dapple * depth_factor
                                * sprite.w * SHADOW_STEP * 3.0;
                    light *= (1.0 - clamp(opacity, 0.0, 0.4));

                    // Light filtering through green leaves: warm-green tint
                    let tint_strength = clamp(opacity * 0.5, 0.0, 0.08);
                    tint *= mix(vec3<f32>(1.0), vec3<f32>(0.88, 1.0, 0.82), tint_strength);

                    if light < 0.02 {
                        return vec4<f32>(tint, 0.0);
                    }
                }
            }
            // Skip the normal block shadow test for trees
        } else if bt == BT_BERRY_BUSH || bt == BT_CROP {
            // Berry bush / crop: soft dappled shadow (lighter than trees)
            if effective_h > current_h {
                let plant_seed = sx * 97.3 + sy * 213.5 + camera.time * 0.3;
                let dapple = 0.6 + 0.4 * fract(sin(plant_seed) * 43758.5453);
                let opacity = 0.25 * dapple * SHADOW_STEP * 2.0;
                light *= (1.0 - clamp(opacity, 0.0, 0.2));
                tint *= mix(vec3<f32>(1.0), vec3<f32>(0.85, 1.0, 0.78), opacity * 0.3);
                if light < 0.02 { return vec4<f32>(tint, 0.0); }
            }
        } else

        // Does this block/roof intersect the ray?
        if effective_h > current_h {
            if is_roofed_floor {
                // Roof is a hard opaque surface — fully blocks the shadow ray.
                // (Indoor pixels never reach here; they use compute_interior_light instead.)
                return vec4<f32>(tint, 0.0);
            } else if is_door(block) && is_open(block) {
                // Open door: ray passes through freely (doorway is an opening)
                // continue stepping
            } else {
                // Opaque block (wall, roof, etc.): shadow with soft penumbra
                let overlap = effective_h - current_h;
                let shadow_strength = clamp(overlap * 1.2, 0.0, 1.0);
                light *= (1.0 - shadow_strength);
                if light < 0.02 {
                    return vec4<f32>(tint, 0.0);
                }
            }
        }
    }

    return vec4<f32>(tint, light);
}

// --- Wall side face detection (3D bevel) ---
// Get effective height at a tile, including wall_data walls
fn effective_tile_height(nx: i32, ny: i32) -> u32 {
    let bh = block_height(get_block(nx, ny));
    if bh > 0u { return bh; }
    // Check wall_data for wall height
    if nx >= 0 && ny >= 0 && nx < i32(camera.grid_w) && ny < i32(camera.grid_h) {
        let wd = read_wall_data(u32(ny) * u32(camera.grid_w) + u32(nx));
        if (wd & 0xFu) != 0u {
            return u32(wall_material_height(wd_material_s(wd)));
        }
    }
    return 0u;
}

fn wall_side_shade(wx: f32, wy: f32, bx: i32, by: i32, height: u32) -> vec3<f32> {
    let fx = fract(wx);
    let fy = fract(wy);
    var shade = vec3<f32>(0.0);

    if height == 0u { return shade; }

    let fh = f32(height);
    let edge_width = clamp(0.12 * fh, 0.04, 0.25);

    // Top edge (sun-facing: sun is upper-left)
    if effective_tile_height(bx, by - 1) < height && fy < edge_width {
        let t = 1.0 - fy / edge_width;
        shade += vec3<f32>(0.15, 0.14, 0.12) * t;
    }
    // Left edge (sun-facing)
    if effective_tile_height(bx - 1, by) < height && fx < edge_width {
        let t = 1.0 - fx / edge_width;
        shade += vec3<f32>(0.12, 0.11, 0.10) * t;
    }
    // Bottom edge (shadowed)
    if effective_tile_height(bx, by + 1) < height && fy > (1.0 - edge_width) {
        let t = (fy - (1.0 - edge_width)) / edge_width;
        shade -= vec3<f32>(0.10, 0.10, 0.08) * t;
    }
    // Right edge (shadowed)
    if effective_tile_height(bx + 1, by) < height && fx > (1.0 - edge_width) {
        let t = (fx - (1.0 - edge_width)) / edge_width;
        shade -= vec3<f32>(0.08, 0.08, 0.06) * t;
    }

    return shade;
}

// --- Glass rendering ---
// Returns: vec4(color_rgb, is_glass)
// is_glass = 1.0 if pixel is in the glass portion, 0.0 if it's in the wall surround
fn render_glass_block(wx: f32, wy: f32, fx: f32, fy: f32, bx: i32, by: i32) -> vec4<f32> {
    // Detect wall orientation by checking neighbors
    let left_t = block_type(get_block(bx - 1, by));
    let right_t = block_type(get_block(bx + 1, by));
    let top_t = block_type(get_block(bx, by - 1));
    let bot_t = block_type(get_block(bx, by + 1));

    // Wall neighbors: stone, generic, glass, insulated, or any named wall type (21-25)
    let left_w = left_t == 1u || left_t == 4u || left_t == 5u || left_t == 14u || (left_t >= 21u && left_t <= 25u);
    let right_w = right_t == 1u || right_t == 4u || right_t == 5u || right_t == 14u || (right_t >= 21u && right_t <= 25u);
    let top_w = top_t == 1u || top_t == 4u || top_t == 5u || top_t == 14u || (top_t >= 21u && top_t <= 25u);
    let bot_w = bot_t == 1u || bot_t == 4u || bot_t == 5u || bot_t == 14u || (bot_t >= 21u && bot_t <= 25u);
    let h_wall = left_w || right_w;
    let v_wall = top_w || bot_w;

    // Determine thin axis: if in a horizontal wall run, thin in Y; if vertical, thin in X
    // If both or neither, default to thinner in both axes
    let margin = (1.0 - WINDOW_THICKNESS) * 0.5;
    var in_glass = true;
    if h_wall && !v_wall {
        // Horizontal wall — window is thin in Y
        in_glass = fy >= margin && fy <= (1.0 - margin);
    } else if v_wall && !h_wall {
        // Vertical wall — window is thin in X
        in_glass = fx >= margin && fx <= (1.0 - margin);
    } else {
        // Corner or standalone — thin in both
        in_glass = fx >= margin && fx <= (1.0 - margin) &&
                   fy >= margin && fy <= (1.0 - margin);
    }

    if !in_glass {
        // Wall surround: stone color with subtle edge darkening toward the opening
        let wall_col = vec3<f32>(0.55, 0.53, 0.50);
        // Darken the inner edge (near the glass) to give depth
        var depth_shade = 0.0;
        if h_wall && !v_wall {
            let dist_to_glass = min(abs(fy - margin), abs(fy - (1.0 - margin)));
            depth_shade = smoothstep(margin, 0.0, dist_to_glass) * 0.15;
        } else if v_wall && !h_wall {
            let dist_to_glass = min(abs(fx - margin), abs(fx - (1.0 - margin)));
            depth_shade = smoothstep(margin, 0.0, dist_to_glass) * 0.15;
        }
        return vec4<f32>(wall_col - vec3<f32>(depth_shade), 0.0);
    }

    // Remap fx/fy to be relative to the glass inset area
    var gx = fx;
    var gy = fy;
    if h_wall && !v_wall {
        gy = (fy - margin) / WINDOW_THICKNESS;
    } else if v_wall && !h_wall {
        gx = (fx - margin) / WINDOW_THICKNESS;
    } else {
        gx = (fx - margin) / WINDOW_THICKNESS;
        gy = (fy - margin) / WINDOW_THICKNESS;
    }

    let frame_w = 0.10;
    let is_frame = f32(gx < frame_w || gx > (1.0 - frame_w) ||
                       gy < frame_w || gy > (1.0 - frame_w));

    let frame_color = vec3<f32>(0.4, 0.42, 0.44);
    let glass_color = vec3<f32>(0.55, 0.72, 0.82);

    let highlight = smoothstep(0.3, 0.5, gx) * smoothstep(0.7, 0.5, gx) *
                    smoothstep(0.2, 0.4, gy) * smoothstep(0.8, 0.6, gy);

    let base = mix(glass_color, frame_color, is_frame);
    return vec4<f32>(base + vec3<f32>(highlight * 0.15), 1.0);
}


@compute @workgroup_size(8, 8)
fn main_raytrace(@builtin(global_invocation_id) gid: vec3<u32>) {
    let px = gid.x;
    let py = gid.y;
    let sw = u32(camera.screen_w);
    let sh = u32(camera.screen_h);

    if px >= sw || py >= sh {
        return;
    }

    // Screen pixel → world coordinate (pixel-level precision)
    let world_x = camera.center_x + (f32(px) - camera.screen_w * 0.5) / camera.zoom;
    let world_y = camera.center_y + (f32(py) - camera.screen_h * 0.5) / camera.zoom;

    let bx = i32(floor(world_x));
    let by = i32(floor(world_y));
    let fx = fract(world_x);
    let fy = fract(world_y);

    // --- Fog of war early exit: skip all work for shrouded tiles ---
    if camera.fog_enabled > 0.5 {
        let fog_early = sample_fog(world_x, world_y);
        if fog_early < 0.01 {
            textureStore(output, vec2<u32>(px, py), vec4(0.0, 0.0, 0.0, 1.0));
            return;
        }
    }

    // --- Temporal reprojection: compute blend weight for TAA-style accumulation ---
    // Instead of binary reproject-or-don't, we ALWAYS render the current frame
    // but blend with the previous frame at the end. This accumulates shadow jitter
    // into smooth shadows over 2-3 frames.
    var temporal_blend = 0.0; // 0 = fully current, higher = mix more previous
    var temporal_prev_px = 0.0;
    var temporal_prev_py = 0.0;
    {
        let prev_px = (world_x - camera.prev_center_x) * camera.prev_zoom + camera.screen_w * 0.5;
        let prev_py = (world_y - camera.prev_center_y) * camera.prev_zoom + camera.screen_h * 0.5;

        let zoom_stable = abs(camera.zoom - camera.prev_zoom) < 0.01;
        let in_prev_bounds = prev_px >= 0.0 && prev_py >= 0.0
            && prev_px < camera.screen_w && prev_py < camera.screen_h;
        let camera_delta = length(vec2(camera.center_x - camera.prev_center_x,
                                        camera.center_y - camera.prev_center_y));

        // Check fluid activity at this position
        let fluid_uv_check = vec2(world_x / camera.grid_w, world_y / camera.grid_h);
        let dye_check = textureSampleLevel(fluid_dye_tex, fluid_dye_sampler, fluid_uv_check, 0.0);
        let near_fluid = dye_check.r > 0.001 || abs(dye_check.g - 1.0) > 0.01 || dye_check.b > 0.005;

        let time_delta = abs(camera.time - camera.prev_time);
        let time_stable = time_delta < 0.0005;

        let can_blend = zoom_stable && in_prev_bounds && camera_delta < 0.001
            && camera.force_refresh < 0.5;

        if can_blend {
            temporal_prev_px = prev_px;
            temporal_prev_py = prev_py;
            if time_stable && !near_fluid {
                // Fully static: use previous frame entirely (fast path)
                let prev_color = textureLoad(prev_output, vec2<i32>(i32(px), i32(py)), 0).rgb;
                textureStore(output, vec2<u32>(px, py), vec4(prev_color, 1.0));
                return;
            }
            // Slow time progression: blend for shadow smoothing (TAA)
            // Higher blend = smoother shadows but more ghosting
            let fluid_factor = select(1.0, 0.4, near_fluid); // less blend near smoke
            temporal_blend = 0.35 * fluid_factor;
        }
    }

    var block = get_block(bx, by);
    var btype = block_type(block);
    var bheight = block_height(block);
    var bheight_raw = block_height_raw(block); // includes edge mask for pixel_is_wall
    var bflags = block_flags(block);
    var fheight = f32(bheight);

    // --- Wall data layer (DN-008): check if this tile has walls from wall_buf ---
    let wd_idx = u32(by) * u32(camera.grid_w) + u32(bx);
    let wd = read_wall_data(wd_idx);

    // Physical door check: if this tile has a door, use door rendering
    var is_door_pixel = false;
    var is_door_gap = false;
    var door_pixel_color = vec3<f32>(0.0);
    if wd_has_door(wd) {
        let door_info = find_door(u32(bx), u32(by));
        if door_info.found {
            let door_result = render_door(fx, fy, wd, door_info);
            if door_result.w > 0.5 {
                is_door_pixel = true;
                door_pixel_color = door_result.xyz;
            } else {
                // Check if we're in the wall strip but in the gap
                let wall_frac = f32(wd_thickness_s(wd)) * 0.25;
                var in_strip = false;
                let edge = door_info.edge;
                if edge == 0u && fy < wall_frac { in_strip = true; }
                else if edge == 1u && fx > (1.0 - wall_frac) { in_strip = true; }
                else if edge == 2u && fy > (1.0 - wall_frac) { in_strip = true; }
                else if edge == 3u && fx < wall_frac { in_strip = true; }
                if in_strip { is_door_gap = true; }
            }
        }
    }

    let wd_is_wall_pixel = wd_pixel_is_wall(fx, fy, wd) && !is_door_gap;

    // If the wall_data says this pixel is wall, override effective height for shadows
    // and set a flag for the renderer to use wall material color instead of block color.
    var is_wd_wall = false;
    var wd_wall_height = 0.0;
    if wd_is_wall_pixel || is_door_pixel {
        is_wd_wall = true;
        let wmat = wd_material_s(wd);
        wd_wall_height = wall_material_height(wmat);
        // Override block height so shadows/oblique work correctly
        if bheight == 0u || !matches_wall_type(btype) {
            bheight = u32(wd_wall_height);
            bheight_raw = bheight;
            fheight = wd_wall_height;
        }
    }

    // --- Oblique projection: show south face within the wall's own tile ---
    // The camera looks slightly from the south. If this block is tall and the
    // block to the south is shorter, the bottom strip of THIS tile shows the
    // south wall face. The wall stays entirely within its own block boundary.
    var is_wall_face = false;
    var wall_face_t = 0.0; // 0=top of face, 1=bottom of face

    let south_h = effective_tile_height(bx, by + 1);
    let mat = get_material(btype);
    let has_wall_face = mat.shows_wall_face > 0.5 || is_wd_wall;
    let any_door_open = (is_door(block) && is_open(block)) || (wd_has_door(wd) && wd_door_open(wd));
    // Show face even when door is open (open door shows dark opening in face)
    let is_exterior_south = bheight > south_h && has_wall_face;

    if is_exterior_south {
        let height_diff = f32(bheight - south_h);
        // Always show at least a baseline face (0.08) + oblique slider adds more
        let face_height = min(height_diff * max(camera.oblique_strength, 0.08), 0.35);
        let face_start = 1.0 - face_height; // face occupies bottom strip of tile
        if fy > face_start {
            // Thin wall: only show face where wall sub-cells exist.
            // Check at the pixel's actual position — the face only shows
            // where the pixel is within the wall portion of the tile.
            var show_face = true;
            if is_thin_wall(bflags) {
                show_face = pixel_is_wall(fx, fy, bheight_raw, bflags);
            }
            if show_face {
                is_wall_face = true;
                wall_face_t = (fy - face_start) / face_height;
            }
        }
    }

    // (slope face disabled — edge darkening handles depth perception)

    // Sun parameters precomputed on CPU (no per-pixel trig)
    let sun_dir = vec2<f32>(camera.sun_dir_x, camera.sun_dir_y);
    let sun_elev = camera.sun_elevation;
    let sun_color = vec3<f32>(camera.sun_color_r, camera.sun_color_g, camera.sun_color_b);
    let ambient = vec3<f32>(camera.ambient_r, camera.ambient_g, camera.ambient_b);

    // --- Determine if this pixel is covered by a roof ---
    let roof_h = get_roof_height(bx, by);
    let is_roofed = roof_h > 0.5;

    // --- If roofed AND show_roofs is on, render corrugated steel roof surface ---
    // Roof is a solid opaque layer that covers everything (walls, faces, doors).
    if is_roofed && camera.show_roofs > 0.5 {
        let roof_col = roof_color(world_x, world_y);

        // Shadow on roof surface (per-pixel ray trace)
        let roof_shadow = trace_shadow_ray(world_x, world_y, roof_h, sun_dir, sun_elev);
        let roof_light = roof_shadow.w;
        let roof_tint = roof_shadow.xyz;

        var color = roof_col * (ambient + sun_color * roof_light * 0.8 * roof_tint);

        color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
        textureStore(output, vec2<u32>(px, py), vec4<f32>(color, 1.0));
        return;
    }

    // --- Not roofed (or roofs transparent): render the actual block surface ---
    var color: vec3<f32>;
    var is_glass_pixel = false;
    var is_tree_pixel = false;
    var is_table_lamp_bulb = false;

    // If under a roof but transparent mode, add subtle indoor tint
    // Only ground-level tiles under a roof are truly indoor.
    // Wall blocks (height > 0) that are part of a roofed building should NOT
    // get interior lighting — they're the building envelope, not the interior.
    let is_indoor = is_roofed && camera.show_roofs < 0.5 && bheight == 0u;

    // --- Wall face rendering (south face, oblique projection) ---
    if is_wall_face {
        // Use wall_data material color if available, otherwise block color
        var face_color = select(block_base_color(btype, bflags), wall_material_color(wd_material_s(wd)), is_wd_wall);

        // Open door: dark opening in the face
        let face_is_open_door = wd_has_door(wd) && wd_door_open(wd);
        if face_is_open_door {
            // Show dark interior through the opening
            face_color = vec3<f32>(0.08, 0.07, 0.06);
            // Door frame on sides
            if fx < 0.08 || fx > 0.92 {
                face_color = vec3<f32>(0.30, 0.22, 0.12);
            }
            // Lintel (top of doorway)
            if wall_face_t < 0.08 {
                face_color = vec3<f32>(0.30, 0.22, 0.12);
            }
        }

        // Closed door face: darker wood with vertical plank lines and handle
        let face_is_door = wd_has_door(wd) && !wd_door_open(wd);
        if face_is_door {
            // Dark wood door
            face_color = vec3<f32>(0.35, 0.25, 0.15);
            // Vertical plank lines (3 planks)
            let plank = fract(fx * 3.0);
            if plank < 0.04 || plank > 0.96 {
                face_color *= 0.7; // dark gap between planks
            }
            // Handle/knob (small bright dot on the right side)
            let knob_x = fx - 0.65;
            let knob_y = wall_face_t - 0.55;
            if knob_x * knob_x + knob_y * knob_y < 0.003 {
                face_color = vec3<f32>(0.55, 0.50, 0.35); // brass knob
            }
            // Slight frame around door edges
            if fx < 0.06 || fx > 0.94 || wall_face_t < 0.05 {
                face_color = vec3<f32>(0.30, 0.22, 0.12); // darker frame
            }
        }

        // Darken toward bottom of face (ambient occlusion at ground junction)
        face_color *= (0.60 + 0.40 * (1.0 - wall_face_t));

        // Subtle mortar/plank lines along the face (skip for doors — already has planks)
        if !face_is_door {
            let line = fract(fx * 4.0);
            let mortar = f32(line < 0.06) * 0.04;
            face_color -= vec3<f32>(mortar);
        }

        // Glass face: window between sill and lintel with frame detail
        if btype == BT_GLASS {
            let sill_t = WINDOW_SILL_FRAC;
            let lintel_t = 1.0 - WINDOW_LINTEL_FRAC;
            let in_window = wall_face_t > sill_t && wall_face_t < lintel_t;
            if in_window {
                // Window frame (thin border around glass)
                let frame_w = 0.04;
                let near_sill = wall_face_t < sill_t + frame_w;
                let near_lintel = wall_face_t > lintel_t - frame_w;
                let near_side = fx < 0.08 || fx > 0.92;
                // Cross bar in the middle
                let cross_bar = abs(wall_face_t - (sill_t + lintel_t) * 0.5) < frame_w * 0.5;
                let mid_bar = abs(fx - 0.5) < 0.02;
                if near_sill || near_lintel || near_side || cross_bar || mid_bar {
                    // Dark wood/metal frame
                    face_color = vec3<f32>(0.28, 0.25, 0.22);
                } else {
                    // Glass pane — blue-tinted, slight reflection gradient
                    let glass_t = (wall_face_t - sill_t) / (lintel_t - sill_t);
                    face_color = vec3<f32>(0.35, 0.50, 0.65) * (0.85 + 0.15 * (1.0 - glass_t));
                    // Slight specular highlight near top
                    if glass_t < 0.15 {
                        face_color += vec3<f32>(0.08, 0.08, 0.10) * (1.0 - glass_t / 0.15);
                    }
                }
            } else {
                // Sill below window: stone ledge, slightly lighter
                if wall_face_t < sill_t && wall_face_t > sill_t - 0.06 {
                    face_color = vec3<f32>(0.58, 0.56, 0.52);
                }
            }
        }

        // Insulated wall face: show insulation core between outer panels
        if btype == BT_INSULATED {
            let core_top = 0.2;
            let core_bot = 0.85;
            let in_core_face = wall_face_t > core_top && wall_face_t < core_bot;
            if in_core_face {
                let ft = (wall_face_t - core_top) / (core_bot - core_top);
                let fiber = fract(sin(fx * 47.3 + ft * 89.1) * 43758.5);
                face_color = vec3<f32>(0.85, 0.72, 0.52);
                if fiber > 0.5 {
                    face_color = mix(face_color, vec3<f32>(0.88, 0.62, 0.58), 0.3);
                }
                // Panel divider lines
                if abs(ft - 0.5) < 0.02 { face_color -= vec3<f32>(0.06); }
            } else {
                face_color = vec3<f32>(0.86, 0.86, 0.88);
            }
        }

        // Mud wall face: rounded profile, craggy with straw
        if btype == BT_MUD_WALL {
            // Rounded profile: bulges out in the middle
            let bulge = sin(wall_face_t * 3.14159) * 0.12;
            let mud_v = fract(sin(fx * 37.1 + wall_face_t * 73.7) * 43758.5) * 0.06 - 0.03;
            face_color = vec3<f32>(0.50 + mud_v, 0.38 + mud_v * 0.8, 0.24 + mud_v * 0.5);
            // Brightness from bulge (center of face is closer to viewer = brighter)
            face_color *= 0.85 + bulge;
            // Craggy cracks
            let fc = abs(fract(fx * 4.0 + wall_face_t * 0.5 + mud_v * 3.0) - 0.5);
            if fc < 0.05 { face_color *= 0.8; }
            // Straw fiber
            let fs = fract(sin(fx * 211.1 + wall_face_t * 137.3) * 43758.5);
            if fs > 0.93 { face_color = mix(face_color, vec3<f32>(0.62, 0.55, 0.33), 0.4); }
            // Rounded top: taper at the top of the wall
            if wall_face_t < 0.08 {
                let top_round = wall_face_t / 0.08;
                face_color *= 0.7 + top_round * 0.3;
            }
        }

        // Low wall face: earthy, shorter, sandy mud color
        if btype == BT_LOW_WALL {
            let mud_v = fract(sin(fx * 37.1 + wall_face_t * 73.7) * 43758.5) * 0.06 - 0.03;
            face_color = vec3<f32>(0.55 + mud_v, 0.42 + mud_v * 0.8, 0.28 + mud_v * 0.5);
            let bulge = sin(wall_face_t * 3.14159) * 0.08;
            face_color *= 0.88 + bulge;
            // Rounded top edge
            if wall_face_t < 0.12 {
                face_color *= 0.7 + (wall_face_t / 0.12) * 0.3;
            }
        }

        // South face is always mostly in shade (sun stays north in our model).
        // Slight indirect bounce: south face catches a tiny bit of reflected ground light.
        let indirect = sun_color * camera.sun_intensity * 0.12;
        color = face_color * (ambient * 0.6 + indirect);

        // Interior light glow on wall faces (proximity-traced, no lightmap bleed)
        if is_roofed {
            let face_glow = compute_proximity_glow(world_x, world_y, camera.time);
            let night_boost = 1.0 - camera.sun_intensity * 0.7;
            let face_glow_mul = camera.indoor_glow_mul * (0.5 + night_boost);
            color = color * (vec3(1.0) + face_glow * face_glow_mul * 3.0)
                  + face_glow * face_glow_mul * 0.08;
        }

        color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
        textureStore(output, vec2<u32>(px, py), vec4<f32>(color, 1.0));
        return;
    }

    // Pre-pass: draw wire connections underneath power equipment
    // For battery, solar, wind, switch, dimmer — show wire entering from adjacent wire blocks
    let is_power_equip = btype == BT_SOLAR || btype == BT_BATTERY_S || btype == BT_BATTERY_M || btype == BT_BATTERY_L
        || btype == BT_WIND_TURBINE || btype == BT_SWITCH || btype == BT_DIMMER;
    if is_power_equip {
        let ground_col = vec3<f32>(0.42, 0.35, 0.22);
        color = ground_col;
        // Check adjacent tiles for wires and draw connecting wire segments
        let pw = 0.06;
        for (var wd = 0; wd < 4; wd++) {
            var wdx = 0; var wdy = 0;
            if wd == 0 { wdx = 1; } else if wd == 1 { wdx = -1; }
            else if wd == 2 { wdy = 1; } else { wdy = -1; }
            let wnx = bx + wdx;
            let wny = by + wdy;
            if wnx < 0 || wny < 0 || wnx >= i32(camera.grid_w) || wny >= i32(camera.grid_h) { continue; }
            let wnbt = block_type(get_block(wnx, wny));
            let wnf = (get_block(wnx, wny) >> 16u) & 0x80u;
            if wnbt == 36u || wnf != 0u { // wire or wire overlay
                var on_seg = false;
                if wdx == 1 { on_seg = fx > 0.5 && abs(fy - 0.5) < pw; }
                if wdx == -1 { on_seg = fx < 0.5 && abs(fy - 0.5) < pw; }
                if wdy == 1 { on_seg = fy > 0.5 && abs(fx - 0.5) < pw; }
                if wdy == -1 { on_seg = fy < 0.5 && abs(fx - 0.5) < pw; }
                if on_seg {
                    var wc = vec3<f32>(0.55, 0.38, 0.20);
                    let wv = voltage[u32(by) * u32(camera.grid_w) + u32(bx)];
                    wc = mix(wc, vec3<f32>(1.0, 0.85, 0.3), clamp(wv / 12.0, 0.0, 1.0) * 0.5);
                    color = wc;
                }
            }
        }
    }

    // Multi-tile tree rendering: check 3x3 neighborhood for trees whose
    // sprites extend into this pixel (allows trees to visually cross tile boundaries)
    // Track winning tree tile for hover outline
    var tree_win_tx = 0.0;
    var tree_win_ty = 0.0;
    {
        var best_tree = vec4(0.0);
        for (var tdy: i32 = -2; tdy <= 2; tdy++) {
            for (var tdx: i32 = -2; tdx <= 2; tdx++) {
                let tx = bx + tdx;
                let ty = by + tdy;
                if tx < 0 || ty < 0 || tx >= i32(camera.grid_w) || ty >= i32(camera.grid_h) { continue; }
                let tb = get_block(tx, ty);
                if block_type(tb) != BT_TREE { continue; }
                let tr = render_tree(world_x, world_y, f32(tx), f32(ty), block_height(tb), block_flags(tb));
                if tr.w > best_tree.w {
                    best_tree = tr;
                    tree_win_tx = f32(tx);
                    tree_win_ty = f32(ty);
                }
            }
        }
        if best_tree.w > 0.04 {
            is_tree_pixel = true;
            color = best_tree.xyz;
        }
    }

    // Tree sprite shadow: silhouette anchored at trunk base, rotated opposite
    // sun, stretched by sun elevation. Ray-based search (no clipping).
    var tree_shadow = 1.0;
    if !is_tree_pixel && camera.sun_intensity > 0.05 {
        let elev = max(camera.sun_elevation, 0.08);
        let shd = normalize(vec2(-camera.sun_dir_x, -camera.sun_dir_y));
        let shd_p = vec2(-shd.y, shd.x);
        let sun_n = normalize(vec2(camera.sun_dir_x, camera.sun_dir_y));
        let stretch = 3.5 / elev;
        // Ray trace TOWARD the sun from this pixel — check trees along the way
        // Half-step ray: ensures no grid tiles are skipped at diagonal sun angles
        let max_steps = i32(min(stretch * 2.0 + 8.0, 50.0));
        for (var step: i32 = -4; step <= max_steps; step++) {
            let t = f32(step) * 0.5;
            let ray_x = world_x + sun_n.x * t;
            let ray_y = world_y + sun_n.y * t;
            for (var perp_off: i32 = -1; perp_off <= 1; perp_off++) {
                let check_x = i32(floor(ray_x + sun_n.y * f32(perp_off)));
                let check_y = i32(floor(ray_y - sun_n.x * f32(perp_off)));
                if check_x < 0 || check_y < 0 || check_x >= i32(camera.grid_w) || check_y >= i32(camera.grid_h) { continue; }
                let ttb = get_block(check_x, check_y);
                if block_type(ttb) != BT_TREE { continue; }
                let tid = f32(check_x) * 137.0 + f32(check_y) * 311.0;
                let tih = fract(sin(tid) * 43758.5453);
                let tsh = fract(sin(tid * 2.3 + 1.7) * 31415.9);
                var tvar: u32;
                if tsh < 0.5 { tvar = u32(tih * 8.0) % 8u; }
                else { tvar = 8u + u32(tih * 16.0) % 16u; }
                let th = block_height(ttb);
                let tsz = select(select(select(2.0, 2.8, th >= 3u), 3.5, th >= 4u), 4.0, th >= 5u);
                let tox = (fract(sin(tid * 1.3 + 7.1) * 31415.9) - 0.5) * 0.5;
                let toy = (fract(sin(tid * 2.7 + 3.9) * 27183.6) - 0.5) * 0.5;
                let tcx = f32(check_x) + 0.5 + tox;
                let tcy = f32(check_y) + 0.5 + toy;

                // Trunk base: slightly south of sprite center
                let trunk_x = tcx;
                let trunk_y = tcy + tsz * 0.15;

                // Pixel offset from trunk base
                let dx = world_x - trunk_x;
                let dy = world_y - trunk_y;

                // Decompose into shadow axes
                let along = dx * shd.x + dy * shd.y;
                let across = dx * shd_p.x + dy * shd_p.y;

                // Shadow extends from trunk (along≈0) away from sun (along>0)
                if along < -tsz * 0.15 { continue; }
                if abs(across) > tsz * 0.55 { continue; }

                // Map to sprite UV
                let su = 0.5 + across / tsz;
                let shadow_len = stretch * tsz * 0.4;
                let sv = 0.1 + (along / shadow_len) * 0.8;

                // Margin avoids sprite edge pixels (anti-aliasing fringe)
                if su < 0.03 || su > 0.97 || sv < 0.03 || sv > 0.97 { continue; }
                let sp = sample_sprite(tvar, su, sv);
                if sp.w > 0.15 {
                    let strength = smoothstep(0.1, 0.6, sp.w) * 0.45;
                    tree_shadow = min(tree_shadow, 1.0 - strength);
                }
            }
        }
    }

    // --- Pleb shadows (analytical ellipse, cast along sun direction) ---
    var pleb_shadow = 1.0;
    if camera.sun_intensity > 0.05 {
        let shd_elev = max(camera.sun_elevation, 0.08);
        let sun_n = normalize(vec2(camera.sun_dir_x, camera.sun_dir_y));
        let sun_p = vec2(-sun_n.y, sun_n.x); // perpendicular
        let shd_stretch = 1.5 / shd_elev; // longer shadow at low sun
        let ps = 0.55 * camera.pleb_scale; // match body scale

        let max_shadow_dist = shd_stretch * ps + 2.0; // max possible shadow reach

        for (var pi: u32 = 0u; pi < MAX_PLEBS; pi++) {
            let p = plebs[pi];
            if p.x < 0.5 && p.y < 0.5 { continue; }
            if p.health <= 0.0 { continue; }

            // Early distance cull: skip plebs too far from this pixel
            let qdx = world_x - p.x;
            let qdy = world_y - p.y;
            if qdx * qdx + qdy * qdy > max_shadow_dist * max_shadow_dist { continue; }

            // Pleb "trunk" (feet position)
            let foot_x = p.x;
            let foot_y = p.y;

            // Pixel offset from pleb feet
            let dx = world_x - foot_x;
            let dy = world_y - foot_y;

            // Decompose into shadow axis (along sun dir) and perpendicular
            let along = dx * sun_n.x + dy * sun_n.y;
            let across = dx * sun_p.x + dy * sun_p.y;

            // Shadow extends away from sun (along < 0 = toward sun = no shadow)
            let shadow_len = shd_stretch * ps * 0.7;
            if along < -ps * 0.1 || along > shadow_len { continue; }
            if abs(across) > ps * 0.28 { continue; }

            // Shadow shape: tapers with distance from body
            let t = along / shadow_len; // 0 at body, 1 at tip
            let width = ps * 0.26 * (1.0 - t * 0.6); // narrower at tip
            if abs(across) < width {
                let alpha = (1.0 - t) * 0.35 * camera.sun_intensity;
                pleb_shadow = min(pleb_shadow, 1.0 - alpha);
            }
        }
    }

    if is_tree_pixel {
        // Tree pixel already colored by multi-tile pre-pass — skip block-type switch
    } else if btype == BT_GLASS {
        // Glass block: render with thin inset
        let glass_result = render_glass_block(world_x, world_y, fx, fy, bx, by);
        color = glass_result.xyz;
        is_glass_pixel = glass_result.w > 0.5;
    } else if btype == BT_FIREPLACE {
        // Fireplace: animated emissive rendering
        color = render_fireplace(world_x, world_y, fx, fy, camera.time);
    } else if btype == BT_CAMPFIRE {
        // Small campfire: only render the 2x2 subtile fire area, preserve wall/ground elsewhere
        let sub_x = f32((bflags >> 3u) & 3u) * 0.25;
        let sub_y = f32((bflags >> 5u) & 3u) * 0.25;
        let cf_cx = sub_x + 0.25;
        let cf_cy = sub_y + 0.25;
        let cf_fx = fx - cf_cx + 0.5;
        let cf_fy = fy - cf_cy + 0.5;
        // Only overwrite if pixel is within the fire pit area
        let cf_dist = length(vec2(cf_fx - 0.5, cf_fy - 0.5));
        if cf_dist < 0.26 {
            color = render_campfire(world_x, world_y, cf_fx, cf_fy, camera.time);
        }
        // else: keep existing color (wall, ground, etc.)
    } else if btype == BT_CEILING_LIGHT {
        // Electric light: ceiling fixture rendering
        color = render_electric_light(world_x, world_y, fx, fy, camera.time);
    } else if btype == BT_BENCH {
        // Bench
        color = render_bench(fx, fy, bflags);
    } else if btype == BT_FLOOR_LAMP {
        // Standing lamp (emissive)
        color = render_standing_lamp(fx, fy, camera.time);
    } else if btype == BT_TABLE_LAMP {
        // Table lamp: bulb circle is emissive, bench surface is not
        let tl = render_table_lamp(fx, fy);
        color = tl.xyz;
        is_table_lamp_bulb = tl.w > 0.5;
    } else if btype == BT_FLOODLIGHT {
        // Floodlight: compact housing with bright directional lens
        let fl_cx = fx - 0.5;
        let fl_cy = fy - 0.5;
        let fl_dist = length(vec2(fl_cx, fl_cy));
        let fl_dir = (bflags >> 3u) & 3u;
        // Direction vector
        var fl_dx = 0.0; var fl_dy = 0.0;
        if fl_dir == 0u { fl_dy = -1.0; }
        else if fl_dir == 1u { fl_dx = 1.0; }
        else if fl_dir == 2u { fl_dy = 1.0; }
        else { fl_dx = -1.0; }
        // Housing: dark metallic box
        let body_r = 0.22;
        if fl_dist < body_r {
            color = vec3(0.30, 0.32, 0.35);
            // Metallic sheen
            let sheen = 1.0 - fl_dist / body_r;
            color += vec3(0.08) * sheen;
            // Lens: bright when powered
            let lens_cx = fl_cx - fl_dx * 0.08;
            let lens_cy = fl_cy - fl_dy * 0.08;
            let lens_dist = length(vec2(lens_cx, lens_cy));
            if lens_dist < 0.12 {
                let fl_v = voltage[u32(by) * u32(camera.grid_w) + u32(bx)];
                let fl_power = sqrt(clamp(fl_v / 8.0, 0.0, 1.0));
                let lens_t = 1.0 - lens_dist / 0.12;
                color = mix(vec3(0.4, 0.4, 0.45), vec3(1.0, 0.98, 0.95) * (0.5 + fl_power), lens_t * lens_t);
            }
            // Direction indicator: small notch on the front edge
            let front_dot = fl_cx * fl_dx + fl_cy * fl_dy;
            if front_dot > body_r - 0.06 && abs(fl_cx * (-fl_dy) + fl_cy * fl_dx) < 0.05 {
                color = vec3(0.7, 0.7, 0.75);
            }
            // Rim
            if fl_dist > body_r - 0.03 {
                color = vec3(0.22, 0.22, 0.25);
            }
        } else {
            color = block_base_color(2u, 0u); // ground
        }
    } else if btype == BT_WALL_TORCH {
        color = render_wall_torch(fx, fy, bflags, camera.time, world_x, world_y);
    } else if btype == BT_WALL_LAMP {
        color = render_wall_lamp(fx, fy, bflags, bx, by);
    } else if btype == BT_FAN {
        // Fan: metallic gray housing with grille and bold direction arrow
        // Outer frame (dark steel border)
        let edge = f32(fx < 0.08 || fx > 0.92 || fy < 0.08 || fy > 0.92);
        let base_metal = vec3(0.50, 0.52, 0.55); // metallic gray
        let frame_metal = vec3(0.35, 0.37, 0.40); // darker frame
        color = mix(base_metal, frame_metal, edge);

        // Grille bars (horizontal and vertical for cross-hatch)
        let bar_h = fract(fx * 5.0);
        let bar_v = fract(fy * 5.0);
        let is_bar = f32(bar_h < 0.12 || bar_h > 0.88 || bar_v < 0.12 || bar_v > 0.88);
        color = mix(color, vec3(0.38, 0.40, 0.43), is_bar * 0.6);

        // Specular highlight (metallic sheen)
        let spec = smoothstep(0.3, 0.5, fx) * smoothstep(0.7, 0.5, fx)
                 * smoothstep(0.3, 0.5, fy) * smoothstep(0.7, 0.5, fy);
        color += vec3(0.06) * spec;

        // Bold direction arrow
        let dir_bits = (block >> 19u) & 3u;
        var fan_dx = 0.0;
        var fan_dy = 0.0;
        if dir_bits == 0u { fan_dy = -1.0; }
        else if dir_bits == 1u { fan_dx = 1.0; }
        else if dir_bits == 2u { fan_dy = 1.0; }
        else { fan_dx = -1.0; }
        // Arrow shaft
        let along = (fx - 0.5) * fan_dx + (fy - 0.5) * fan_dy;
        let perp_dist = abs(-(fx - 0.5) * fan_dy + (fy - 0.5) * fan_dx);
        let on_shaft = along > -0.25 && along < 0.2 && perp_dist < 0.07;
        // Arrowhead (triangle)
        let head_t = (along - 0.1) / 0.25; // 0 at base, 1 at tip
        let on_head = head_t > 0.0 && head_t < 1.0 && perp_dist < 0.2 * (1.0 - head_t);
        if on_shaft || on_head {
            color = vec3(0.85, 0.88, 0.92); // bright white-silver arrow
        }

        // Power indicator LED: small green dot on the output side corner
        let fan_idx = u32(by) * u32(camera.grid_w) + u32(bx);
        let fan_v = voltage[fan_idx];
        if fan_v > 0.5 {
            // LED position: corner on the output side
            let led_x = 0.5 + fan_dx * 0.35 + fan_dy * 0.35;
            let led_y = 0.5 + fan_dy * 0.35 - fan_dx * 0.35;
            let led_dist = length(vec2(fx - led_x, fy - led_y));
            if led_dist < 0.06 {
                // Bright green with subtle pulse
                let pulse = 0.8 + 0.2 * sin(camera.time * 3.0);
                color = vec3(0.1, 0.9, 0.2) * pulse;
            }
        }
    } else if btype == BT_COMPOST {
        // Compost: brown-green organic heap with texture
        let noise = fract(sin(world_x * 13.7 + world_y * 7.3) * 43758.5);
        let heap = smoothstep(0.45, 0.2, length(vec2(fx - 0.5, fy - 0.5)));
        color = mix(vec3(0.30, 0.22, 0.12), vec3(0.25, 0.32, 0.10), noise * 0.5) * (0.7 + heap * 0.3);
        // Slight steam wisps
        let wisp = sin(world_x * 31.0 + camera.time * 2.0) * sin(world_y * 29.0 + camera.time * 1.7);
        color += vec3(0.05) * max(wisp, 0.0) * heap;
    } else if (btype >= BT_PIPE && btype <= BT_INLET) || btype == BT_RESTRICTOR || btype == BT_LIQUID_PIPE || btype == BT_PIPE_BRIDGE || btype == BT_LIQUID_INTAKE || btype == BT_LIQUID_PUMP || btype == BT_LIQUID_OUTPUT {
        // Piping system: auto-connected thin pipe rendering (gas, liquid, bridge, liquid equipment)
        // Background: wall-mounted inlets on stone, liquid intake water-side on water, rest on dirt
        if (btype == BT_OUTLET || btype == BT_INLET) && bheight > 1u {
            color = block_base_color(1u, 0u); // stone wall background for wall-mounted
        } else if btype == BT_LIQUID_INTAKE && ((bflags >> 3u) & 3u) == 1u {
            // Liquid intake water-side segment: render water underneath
            color = block_base_color(3u, 0u); // water background
            // Animated water surface
            let wave1 = sin(world_x * 6.0 + camera.time * 1.5) * 0.02;
            let wave2 = sin(world_y * 8.0 + camera.time * 2.1) * 0.015;
            color += vec3(wave1 + wave2, (wave1 + wave2) * 1.5, (wave1 + wave2) * 2.0);
        } else {
            color = block_base_color(2u, 0u); // dirt floor for ground pipes
        }

        // Connection mask for pipes: height byte bits 4-7 (same encoding as wires)
        let pipe_conn_mask = bheight >> 4u;
        let n_n = block_type(get_block(bx, by - 1));
        let n_s = block_type(get_block(bx, by + 1));
        let n_e = block_type(get_block(bx + 1, by));
        let n_w = block_type(get_block(bx - 1, by));
        // If mask is set, use it; otherwise auto-detect (backward compatible)
        // Liquid pipes only connect to liquid pipes/bridges; gas to gas/restrictors/bridges
        let is_liquid = btype == BT_LIQUID_PIPE || btype == BT_LIQUID_INTAKE || btype == BT_LIQUID_PUMP || btype == BT_LIQUID_OUTPUT;
        // Gas neighbors: standard pipe types. Liquid neighbors: liquid pipe types.
        let ln_n = n_n == BT_LIQUID_PIPE || n_n == BT_PIPE_BRIDGE || n_n == BT_LIQUID_INTAKE || n_n == BT_LIQUID_PUMP || n_n == BT_LIQUID_OUTPUT;
        let ln_s = n_s == BT_LIQUID_PIPE || n_s == BT_PIPE_BRIDGE || n_s == BT_LIQUID_INTAKE || n_s == BT_LIQUID_PUMP || n_s == BT_LIQUID_OUTPUT;
        let ln_e = n_e == BT_LIQUID_PIPE || n_e == BT_PIPE_BRIDGE || n_e == BT_LIQUID_INTAKE || n_e == BT_LIQUID_PUMP || n_e == BT_LIQUID_OUTPUT;
        let ln_w = n_w == BT_LIQUID_PIPE || n_w == BT_PIPE_BRIDGE || n_w == BT_LIQUID_INTAKE || n_w == BT_LIQUID_PUMP || n_w == BT_LIQUID_OUTPUT;
        let gn_n = (n_n >= BT_PIPE && n_n <= BT_INLET) || n_n == BT_RESTRICTOR || n_n == BT_PIPE_BRIDGE;
        let gn_s = (n_s >= BT_PIPE && n_s <= BT_INLET) || n_s == BT_RESTRICTOR || n_s == BT_PIPE_BRIDGE;
        let gn_e = (n_e >= BT_PIPE && n_e <= BT_INLET) || n_e == BT_RESTRICTOR || n_e == BT_PIPE_BRIDGE;
        let gn_w = (n_w >= BT_PIPE && n_w <= BT_INLET) || n_w == BT_RESTRICTOR || n_w == BT_PIPE_BRIDGE;
        var cn = select(gn_n, ln_n, is_liquid);
        var cs = select(gn_s, ln_s, is_liquid);
        var ce = select(gn_e, ln_e, is_liquid);
        var cw = select(gn_w, ln_w, is_liquid);
        if pipe_conn_mask != 0u && (btype == BT_PIPE || btype == BT_RESTRICTOR || btype == BT_LIQUID_PIPE) {
            cn = cn && (pipe_conn_mask & 1u) != 0u; // N
            ce = ce && (pipe_conn_mask & 2u) != 0u; // E
            cs = cs && (pipe_conn_mask & 4u) != 0u; // S
            cw = cw && (pipe_conn_mask & 8u) != 0u; // W
        }

        let cx = fx - 0.5;
        let cy = fy - 0.5;

        if btype == BT_PUMP {
            // --- Pump: square block connected to pipes ---
            let pump_r = 0.30;
            let pipe_r = 0.15;
            // Draw pipe stubs connecting to neighbors BEHIND the pump
            if cn && abs(cx) < pipe_r && cy < -pump_r { color = vec3(0.38, 0.40, 0.43); }
            if cs && abs(cx) < pipe_r && cy > pump_r { color = vec3(0.38, 0.40, 0.43); }
            if ce && abs(cy) < pipe_r && cx > pump_r { color = vec3(0.38, 0.40, 0.43); }
            if cw && abs(cy) < pipe_r && cx < -pump_r { color = vec3(0.38, 0.40, 0.43); }
            // Pump body on top
            if abs(cx) < pump_r && abs(cy) < pump_r {
                let shade = 1.0 - max(abs(cx), abs(cy)) / pump_r * 0.3;
                color = vec3(0.35, 0.52, 0.38) * shade;
                let pulse = sin(camera.time * 6.0) * 0.5 + 0.5;
                color += vec3(0.0, pulse * 0.08, 0.0);
                if abs(cx) > pump_r - 0.03 || abs(cy) > pump_r - 0.03 {
                    color = vec3(0.28, 0.35, 0.30);
                }
            }
        } else if btype == BT_TANK {
            // --- Tank: rounded cylinder with fill indicator ---
            let tank_rx = 0.42; // wider
            let tank_ry = 0.35;
            let ellipse = (cx / tank_rx) * (cx / tank_rx) + (cy / tank_ry) * (cy / tank_ry);

            if ellipse < 1.0 {
                // Tank body: metallic dark cylinder
                let rim = smoothstep(1.0, 0.7, ellipse);
                let body = vec3<f32>(0.38, 0.40, 0.45);
                let highlight = vec3<f32>(0.55, 0.57, 0.62);
                color = mix(body, highlight, rim * rim);

                // Pipe connections: thin lines extending from tank edges
                if cn && abs(cx) < 0.08 && cy < -tank_ry + 0.1 { color = vec3(0.38, 0.40, 0.43); }
                if cs && abs(cx) < 0.08 && cy > tank_ry - 0.1 { color = vec3(0.38, 0.40, 0.43); }
                if ce && abs(cy) < 0.08 && cx > tank_rx - 0.1 { color = vec3(0.38, 0.40, 0.43); }
                if cw && abs(cy) < 0.08 && cx < -tank_rx + 0.1 { color = vec3(0.38, 0.40, 0.43); }

                // Fill level indicator (vertical bar on the left side)
                // TODO: read actual pressure from pipe state buffer
                let fill = 0.5; // placeholder
                let gauge_x = cx + 0.28;
                let gauge_y = (cy + tank_ry * 0.7) / (tank_ry * 1.4);
                if abs(gauge_x) < 0.04 && gauge_y >= 0.0 && gauge_y <= 1.0 {
                    let gauge_bg = vec3(0.2, 0.2, 0.25);
                    let gauge_fill_color = mix(vec3(0.2, 0.7, 0.3), vec3(0.8, 0.2, 0.1), gauge_y);
                    color = select(gauge_bg, gauge_fill_color, gauge_y < fill);
                }

                // Rivets/bolts around the rim
                let angle = atan2(cy, cx);
                let rivet = fract(angle * 4.0 / 6.283);
                if ellipse > 0.75 && ellipse < 0.95 && (rivet < 0.1 || rivet > 0.9) {
                    color = vec3(0.3, 0.32, 0.35);
                }
            }
            // else: background (dirt floor) already set above
        } else {
            // --- Pipe / Pump / Valve / Outlet / Inlet / Restrictor: round pipe ---
            var pipe_r = select(0.15, 0.12, btype == BT_LIQUID_PIPE); // gas pipes wider, liquid slightly thinner
            // Restrictor (46): constriction — narrower in the middle
            let is_restrictor = btype == BT_RESTRICTOR;
            if is_restrictor {
                // Narrow in center, wider at edges (hourglass shape)
                let center_dist = length(vec2(cx, cy));
                let constrict = smoothstep(0.0, 0.35, center_dist);
                let r_level_vis = f32(bheight & 0xFu) / 10.0;
                let narrow_r = 0.05 + r_level_vis * 0.05; // min 0.05, max 0.10
                pipe_r = mix(narrow_r, 0.15, constrict);
            }
            var on_pipe = false;
            var pipe_dist = 1.0; // distance from pipe center (for rounded shading)

            // Horizontal segment (E-W)
            if ce || cw {
                let x_min = select(-pipe_r, -0.5, cw);
                let x_max = select(pipe_r, 0.5, ce);
                // For restrictor, compute local pipe_r at this x position
                var local_r = pipe_r;
                if is_restrictor {
                    let along_dist = abs(cx);
                    let constrict_h = smoothstep(0.0, 0.35, along_dist);
                    let r_lv = f32(bheight & 0xFu) / 10.0;
                    local_r = mix(0.05 + r_lv * 0.05, 0.15, constrict_h);
                }
                if cx >= x_min && cx <= x_max && abs(cy) < local_r {
                    on_pipe = true;
                    pipe_dist = abs(cy) / local_r;
                }
            }
            // Vertical segment (N-S)
            if cn || cs {
                let y_min = select(-pipe_r, -0.5, cn);
                let y_max = select(pipe_r, 0.5, cs);
                var local_r = pipe_r;
                if is_restrictor {
                    let along_dist = abs(cy);
                    let constrict_v = smoothstep(0.0, 0.35, along_dist);
                    let r_lv = f32(bheight & 0xFu) / 10.0;
                    local_r = mix(0.05 + r_lv * 0.05, 0.15, constrict_v);
                }
                if cy >= y_min && cy <= y_max && abs(cx) < local_r {
                    on_pipe = true;
                    pipe_dist = min(pipe_dist, abs(cx) / local_r);
                }
            }
            // Center hub (junction)
            let cdist = length(vec2(cx, cy));
            if cdist < pipe_r && (cn || cs || ce || cw) {
                on_pipe = true;
                pipe_dist = min(pipe_dist, cdist / pipe_r);
            }
            // Isolated: small dot
            if !cn && !cs && !ce && !cw && cdist < pipe_r {
                on_pipe = true;
                pipe_dist = cdist / pipe_r;
            }

            if on_pipe {
                // Rounded pipe shading: bright center, dark edges (cylindrical)
                let shade = 1.0 - pipe_dist * pipe_dist;
                // Pipe color varies by type — gas pipes are darker/industrial
                var pipe_base = vec3<f32>(0.30, 0.32, 0.35);
                var pipe_bright = vec3<f32>(0.48, 0.50, 0.54);
                if is_restrictor {
                    pipe_base = vec3<f32>(0.55, 0.40, 0.25);
                    pipe_bright = vec3<f32>(0.75, 0.58, 0.35);
                } else if btype == BT_LIQUID_PIPE {
                    // Liquid pipe: blue tint
                    pipe_base = vec3<f32>(0.30, 0.40, 0.55);
                    pipe_bright = vec3<f32>(0.50, 0.62, 0.78);
                } else if btype == BT_PIPE_BRIDGE {
                    // Pipe bridge: slightly different shade to indicate crossing
                    pipe_base = vec3<f32>(0.42, 0.44, 0.48);
                    pipe_bright = vec3<f32>(0.64, 0.67, 0.72);
                }
                color = mix(pipe_base, pipe_bright, shade);

                // Traveling dots along pipes — direction follows actual gas/liquid flow
                let flow_idx = (u32(by) * u32(camera.grid_w) + u32(bx)) * 2u;
                let flow_x = pipe_flow[flow_idx];
                let flow_y = pipe_flow[flow_idx + 1u];
                let flow_mag = length(vec2(flow_x, flow_y));
                // Only animate when there's actual flow
                if flow_mag > 0.001 {
                    let dot_r = pipe_r * 0.55;
                    let flow_speed = clamp(flow_mag * 0.5, 0.3, 4.0);
                    let norm_fx = flow_x / flow_mag;
                    let norm_fy = flow_y / flow_mag;
                    // Project pixel position onto flow direction
                    let along = cx * norm_fx + cy * norm_fy;
                    let perp = abs(cx * (-norm_fy) + cy * norm_fx);
                    // Only draw dots within the pipe cross-section
                    if perp < pipe_r {
                        for (var di: i32 = 0; di < 3; di++) {
                            let dot_pos = fract(camera.time * flow_speed + f32(di) * 0.333) - 0.5;
                            let d = abs(along - dot_pos);
                            if d < dot_r {
                                let alpha = smoothstep(dot_r, dot_r * 0.3, d) * 0.6;
                                color = mix(color, vec3(0.28, 0.30, 0.33), alpha);
                            }
                        }
                    }
                }

                // Pipe bridge: segment-specific visual overlay
                if btype == BT_PIPE_BRIDGE {
                    let br_seg = (bflags >> 3u) & 3u;
                    let br_rot = (bflags >> 5u) & 3u;
                    let bridge_is_ns = br_rot % 2u == 0u;
                    let along = select(cx, cy, bridge_is_ns);
                    if br_seg == 0u || br_seg == 2u {
                        // Entry/exit: ramp going underground — darker toward middle
                        let toward_middle = select(along, -along, br_seg == 0u);
                        if toward_middle > 0.1 {
                            let ramp = smoothstep(0.1, 0.5, toward_middle);
                            color = mix(color, vec3(0.20, 0.22, 0.25), ramp * 0.6);
                        }
                    } else {
                        // Middle: surface shows "underground" indicator — dashed line
                        let perp = select(abs(cy), abs(cx), bridge_is_ns);
                        if perp < 0.04 {
                            let dash = fract(select(cx, cy, bridge_is_ns) * 3.0);
                            if dash < 0.5 {
                                color = mix(color, vec3(0.25, 0.28, 0.30), 0.5);
                            }
                        }
                    }
                }

                // Valve overlay
                if btype == BT_VALVE {
                    let valve_open = is_open(block);
                    let vc = select(vec3(0.65, 0.15, 0.15), vec3(0.15, 0.55, 0.15), valve_open);
                    let bar_along = select(abs(cy), abs(cx), ce || cw);
                    let bar_perp = select(abs(cx), abs(cy), ce || cw);
                    if bar_along < pipe_r * 1.8 && bar_perp < 0.04 {
                        color = vc;
                    }
                    if cdist < 0.04 { color = vc; }
                }

                // Liquid Intake (52): blue/teal box straddling water
                if btype == BT_LIQUID_INTAKE {
                    let seg52 = (bflags >> 3u) & 3u;
                    if cdist < 0.25 {
                        color = vec3(0.25, 0.40, 0.55);
                        if cdist > 0.20 { color = vec3(0.18, 0.30, 0.42); } // rim
                        if seg52 == 1u {
                            // Water-side segment: wave pattern
                            let wave = sin(fx * 12.0 + camera.time * 3.0) * 0.03;
                            if abs(cy + wave) < 0.08 { color = vec3(0.20, 0.50, 0.70); }
                        } else {
                            // Ground-side: pipe connection indicator
                            if cdist < 0.06 { color = vec3(0.35, 0.55, 0.70); }
                        }
                    }
                }

                // Liquid Pump (53): blue-green pump body
                if btype == BT_LIQUID_PUMP {
                    let pump53_r = 0.28;
                    if cdist < pump53_r {
                        let shade53 = 1.0 - cdist / pump53_r * 0.3;
                        color = vec3(0.25, 0.45, 0.50) * shade53;
                        let pulse53 = sin(camera.time * 5.0) * 0.5 + 0.5;
                        color += vec3(0.0, pulse53 * 0.06, pulse53 * 0.08);
                        if cdist > pump53_r - 0.03 { color = vec3(0.20, 0.32, 0.38); }
                    }
                }

                // Liquid Output (54): nozzle spraying water
                if btype == BT_LIQUID_OUTPUT {
                    if cdist < 0.20 {
                        color = vec3(0.30, 0.45, 0.55);
                        if cdist > 0.16 { color = vec3(0.22, 0.35, 0.45); }
                        // Animated spray dots
                        let spray_t = camera.time * 4.0;
                        let spray_r = fract(spray_t) * 0.3 + 0.05;
                        let spray_a = fract(spray_t * 0.7) * 6.28;
                        let sx = cos(spray_a) * spray_r;
                        let sy = sin(spray_a) * spray_r;
                        if length(vec2(cx - sx, cy - sy)) < 0.04 {
                            color = vec3(0.40, 0.65, 0.85);
                        }
                    }
                }
            }
            // Inlet/Outlet: rendered AFTER and ON TOP of everything (overlays wall sprite)
            if btype == BT_OUTLET || btype == BT_INLET {
                let is_outlet = btype == BT_OUTLET;
                let dir_bits = (bflags >> 3u) & 3u;
                var fan_dx = 0.0;
                var fan_dy = 0.0;
                if dir_bits == 0u { fan_dy = -1.0; }
                else if dir_bits == 1u { fan_dx = 1.0; }
                else if dir_bits == 2u { fan_dy = 1.0; }
                else { fan_dx = -1.0; }
                // Inlet: flip direction (sucks IN)
                if !is_outlet { fan_dx = -fan_dx; fan_dy = -fan_dy; }

                // Fan circle with radial blades
                let fan_r = 0.32;
                let fan_dist = length(vec2(cx, cy));
                if fan_dist < fan_r {
                    // Fan body
                    let accent = select(vec3(0.65, 0.45, 0.25), vec3(0.30, 0.45, 0.65), is_outlet);
                    let rim = smoothstep(fan_r, fan_r * 0.7, fan_dist);
                    color = mix(accent * 0.6, accent, rim);

                    // Rotating blades (4 blades)
                    let blade_angle = atan2(cy, cx) + camera.time * select(3.0, -3.0, is_outlet);
                    let blade = abs(sin(blade_angle * 2.0));
                    if blade > 0.7 && fan_dist > 0.05 {
                        color = mix(color, accent * 1.4, 0.4);
                    }

                    // Center hub
                    if fan_dist < 0.08 {
                        color = vec3(0.35, 0.37, 0.40);
                    }

                    // Border ring
                    if fan_dist > fan_r - 0.03 {
                        color = accent * 0.4;
                    }

                    // Direction arrow (small, in center area)
                    let along = cx * fan_dx + cy * fan_dy;
                    let perp_d = abs(-cx * fan_dy + cy * fan_dx);
                    if along > 0.0 && along < 0.15 && perp_d < 0.06 * (1.0 - along / 0.15) {
                        color = vec3(0.9, 0.9, 0.95);
                    }
                }
            }
        }
    } else if btype == BT_BED {
        // Bed: 2-tile piece
        color = render_bed(fx, fy, bflags);
    } else if btype == BT_BERRY_BUSH {
        // Berry bush: sprite-based rendering
        let bush_id = floor(world_x) * 73.0 + floor(world_y) * 197.0;
        let bush_hash = fract(sin(bush_id) * 43758.5453);
        let bush_var = u32(bush_hash * f32(BUSH_SPRITE_VARIANTS)) % BUSH_SPRITE_VARIANTS;
        let bush_su = fx;
        let bush_sv = 1.0 - fy;
        let bush_sp = sample_bush_sprite(bush_var, bush_su, bush_sv);
        is_tree_pixel = bush_sp.w > 0.05;
        if is_tree_pixel {
            var bush_col = bush_sp.xyz * (0.90 + bush_hash * 0.2);
            // Depleted bush (height=0): desaturated, browner — bare branches look
            if bheight == 0u {
                let gray = dot(bush_col, vec3(0.299, 0.587, 0.114));
                bush_col = mix(bush_col, vec3(gray * 0.9, gray * 0.75, gray * 0.5), 0.6);
                bush_col *= 0.75;
            }
            color = bush_col;
        } else {
            color = block_base_color(BT_GROUND, 0u);
        }
    } else if btype == BT_SNARE {
        // Snare trap: small loop on the ground
        color = block_base_color(BT_GROUND, 0u); // ground underneath
        let cx = fx - 0.5;
        let cy = fy - 0.5;
        let d = length(vec2(cx, cy));
        // Rope ring
        let ring_outer = 0.28;
        let ring_inner = 0.20;
        if d < ring_outer && d > ring_inner {
            color = vec3(0.45, 0.38, 0.25); // rope color
        }
        // Trigger stick crossing the ring
        if abs(cx - cy) < 0.03 && d < ring_outer {
            color = vec3(0.35, 0.28, 0.15); // dark stick
        }
        // Broken snare (height 0): faded, no ring
        if bheight == 0u {
            color = block_base_color(BT_GROUND, 0u);
            // Just a few scattered sticks
            let stick = abs(fract(fx * 3.0 + fy * 2.0) - 0.5) < 0.04;
            if stick && d < 0.3 {
                color = vec3(0.38, 0.30, 0.18);
            }
        }
    } else if btype == BT_DUG_GROUND {
        // Dug ground: excavated pit, 20% per depth level (max 5 = one full block)
        let depth = f32(bheight); // 1-5, each = 20% of a block
        let depth_frac = depth / 5.0; // 0.0-1.0
        let base_earth = vec3<f32>(0.35, 0.28, 0.15);
        let dark_earth = vec3<f32>(0.18, 0.14, 0.08);
        color = mix(base_earth, dark_earth, depth_frac);
        // Pit wall shadows at edges (deeper = more shadow)
        let edge_d = min(min(fx, 1.0 - fx), min(fy, 1.0 - fy));
        let pit_shadow = smoothstep(0.15, 0.0, edge_d) * 0.4 * depth_frac;
        color *= 1.0 - pit_shadow;
        // Moisture seeping in (visible before full water)
        if depth >= 1.0 {
            color *= 1.0 - depth_frac * 0.15;
        }
    } else if btype == BT_CRATE {
        // Storage crate: wooden box with planks and brackets
        // bheight = number of stored items (0-10)
        let ground = vec3<f32>(0.45, 0.35, 0.20);
        let crate_min = 0.1;
        let crate_max = 0.9;
        let on_crate = fx > crate_min && fx < crate_max && fy > crate_min && fy < crate_max;
        if on_crate {
            // Wood planks running horizontally
            let plank_y = fract(fy * 4.0);
            let plank_edge = f32(plank_y < 0.06) * 0.04;
            let wood = vec3<f32>(0.50, 0.38, 0.20);
            color = wood - vec3<f32>(plank_edge);
            // Plank variation
            let pid = floor(fy * 4.0);
            let pvar = fract(sin(pid * 127.1 + world_x * 17.0) * 43758.5) * 0.06 - 0.03;
            color += vec3<f32>(pvar);
            // Corner brackets (dark metal)
            let bracket_size = 0.12;
            let at_corner = (fx < crate_min + bracket_size || fx > crate_max - bracket_size)
                         && (fy < crate_min + bracket_size || fy > crate_max - bracket_size);
            if at_corner {
                color = vec3<f32>(0.25, 0.25, 0.28);
            }
            // Center cross brace
            let cross_h = abs(fx - 0.5) < 0.03;
            let cross_v = abs(fy - 0.5) < 0.03;
            if cross_h || cross_v {
                color = vec3<f32>(0.38, 0.30, 0.18);
            }
            // Stacked items inside crate (rocks shown as small circles)
            let item_count = bheight; // 0-10
            if item_count > 0u {
                // Arrange items in a grid pattern inside the crate
                let inner_min = 0.20;
                let inner_max = 0.80;
                let inner_fx = (fx - inner_min) / (inner_max - inner_min);
                let inner_fy = (fy - inner_min) / (inner_max - inner_min);
                if inner_fx > 0.0 && inner_fx < 1.0 && inner_fy > 0.0 && inner_fy < 1.0 {
                    // Up to 10 items in a roughly 4x3 grid
                    var item_drawn = false;
                    for (var it = 0u; it < item_count && it < 10u; it++) {
                        // Position each item with slight randomized offsets
                        let col = f32(it % 4u);
                        let row = f32(it / 4u);
                        let ox = (col + 0.5) / 4.0 + fract(sin(f32(it) * 73.1 + world_x * 3.0) * 437.5) * 0.06 - 0.03;
                        let oy = (row + 0.5) / 3.0 + fract(sin(f32(it) * 31.7 + world_y * 5.0) * 218.3) * 0.06 - 0.03;
                        let idist = length(vec2<f32>(inner_fx - ox, inner_fy - oy));
                        if idist < 0.10 {
                            // Rock item
                            let rv = fract(sin(f32(it) * 127.1) * 43758.5) * 0.06 - 0.03;
                            if idist < 0.07 {
                                color = vec3<f32>(0.34 + rv, 0.32 + rv, 0.28 + rv);
                            } else {
                                color = vec3<f32>(0.20, 0.19, 0.17); // outline
                            }
                            item_drawn = true;
                        }
                    }
                }
            }
        } else {
            color = ground;
        }
    } else if btype == BT_ROCK {
        // Rock: sprite-based rendering
        let rock_id = floor(world_x) * 59.0 + floor(world_y) * 173.0;
        let rock_hash = fract(sin(rock_id) * 43758.5453);
        let rock_var = u32(rock_hash * f32(ROCK_SPRITE_VARIANTS)) % ROCK_SPRITE_VARIANTS;
        let rock_su = fx;
        let rock_sv = 1.0 - fy;
        let rock_sp = sample_rock_sprite(rock_var, rock_su, rock_sv);
        is_tree_pixel = rock_sp.w > 0.05;
        if is_tree_pixel {
            color = rock_sp.xyz * (0.85 + rock_hash * 0.2);
        } else {
            color = block_base_color(BT_GROUND, 0u);
        }
    } else if btype == BT_WIRE || btype == BT_WIRE_BRIDGE {
        // Wire / Wire Bridge: copper conductor with directional segments
        let ground = vec3<f32>(0.42, 0.35, 0.22);
        // Connection mask stored in height byte bits 4-7: bit4=N, bit5=E, bit6=S, bit7=W
        let conn_mask = bheight >> 4u;
        // If mask is 0 (old wires or single-click), auto-detect from neighbors
        var conn_n = (conn_mask & 1u) != 0u;
        var conn_e = (conn_mask & 2u) != 0u;
        var conn_s = (conn_mask & 4u) != 0u;
        var conn_w = (conn_mask & 8u) != 0u;
        if conn_mask == 0u {
            // Fallback: auto-detect from power neighbors
            let n_w = block_type(get_block(bx - 1, by));
            let n_e = block_type(get_block(bx + 1, by));
            let n_n = block_type(get_block(bx, by - 1));
            let n_s = block_type(get_block(bx, by + 1));
            let wf_w2 = (get_block(bx - 1, by) >> 16u) & 0x80u;
            let wf_e2 = (get_block(bx + 1, by) >> 16u) & 0x80u;
            let wf_n2 = (get_block(bx, by - 1) >> 16u) & 0x80u;
            let wf_s2 = (get_block(bx, by + 1) >> 16u) & 0x80u;
            let pwr_w = n_w == BT_WIRE || n_w == BT_SOLAR || n_w == BT_BATTERY_S || n_w == BT_BATTERY_M || n_w == BT_BATTERY_L || n_w == BT_WIND_TURBINE || n_w == BT_SWITCH || n_w == BT_DIMMER || n_w == BT_BREAKER || n_w == BT_WIRE_BRIDGE || n_w == BT_FLOODLIGHT || n_w == BT_CEILING_LIGHT || n_w == BT_FAN || n_w == BT_FLOOR_LAMP || n_w == BT_WALL_LAMP;
            let pwr_e = n_e == BT_WIRE || n_e == BT_SOLAR || n_e == BT_BATTERY_S || n_e == BT_BATTERY_M || n_e == BT_BATTERY_L || n_e == BT_WIND_TURBINE || n_e == BT_SWITCH || n_e == BT_DIMMER || n_e == BT_BREAKER || n_e == BT_CEILING_LIGHT || n_e == BT_FAN || n_e == BT_FLOOR_LAMP || n_e == BT_WALL_LAMP;
            let pwr_n = n_n == BT_WIRE || n_n == BT_SOLAR || n_n == BT_BATTERY_S || n_n == BT_BATTERY_M || n_n == BT_BATTERY_L || n_n == BT_WIND_TURBINE || n_n == BT_SWITCH || n_n == BT_DIMMER || n_n == BT_BREAKER || n_n == BT_CEILING_LIGHT || n_n == BT_FAN || n_n == BT_FLOOR_LAMP || n_n == BT_WALL_LAMP;
            let pwr_s = n_s == BT_WIRE || n_s == BT_SOLAR || n_s == BT_BATTERY_S || n_s == BT_BATTERY_M || n_s == BT_BATTERY_L || n_s == BT_WIND_TURBINE || n_s == BT_SWITCH || n_s == BT_DIMMER || n_s == BT_BREAKER || n_s == BT_CEILING_LIGHT || n_s == BT_FAN || n_s == BT_FLOOR_LAMP || n_s == BT_WALL_LAMP;
            conn_w = pwr_w || wf_w2 != 0u;
            conn_e = pwr_e || wf_e2 != 0u;
            conn_n = pwr_n || wf_n2 != 0u;
            conn_s = pwr_s || wf_s2 != 0u;
        }
        let wire_w = 0.07;
        var on_wire = false;
        // Draw segments toward each connected neighbor
        if conn_w { on_wire = on_wire || (fx < 0.5 && abs(fy - 0.5) < wire_w); }
        if conn_e { on_wire = on_wire || (fx > 0.5 && abs(fy - 0.5) < wire_w); }
        if conn_n { on_wire = on_wire || (fy < 0.5 && abs(fx - 0.5) < wire_w); }
        if conn_s { on_wire = on_wire || (fy > 0.5 && abs(fx - 0.5) < wire_w); }
        // Center junction (always visible — this is where connections meet)
        let cdist = length(vec2<f32>(fx - 0.5, fy - 0.5));
        on_wire = on_wire || cdist < 0.10;
        // Corner radius: round the turns where two perpendicular segments meet
        if (conn_n || conn_s) && (conn_w || conn_e) {
            // Fill the inner corner of L-bends with a rounded shape
            let qx = abs(fx - 0.5);
            let qy = abs(fy - 0.5);
            if qx < wire_w + 0.03 && qy < wire_w + 0.03 {
                on_wire = true;
            }
        }
        if on_wire {
            // Copper wire color
            var wire_col = vec3<f32>(0.60, 0.42, 0.22);
            // Dark outline
            let outline_w = wire_w + 0.025;
            var near_edge = false;
            if conn_w && fx < 0.5 { near_edge = near_edge || abs(fy - 0.5) > wire_w - 0.02; }
            if conn_e && fx > 0.5 { near_edge = near_edge || abs(fy - 0.5) > wire_w - 0.02; }
            if conn_n && fy < 0.5 { near_edge = near_edge || abs(fx - 0.5) > wire_w - 0.02; }
            if conn_s && fy > 0.5 { near_edge = near_edge || abs(fx - 0.5) > wire_w - 0.02; }
            if near_edge { wire_col *= 0.7; }
            // Voltage glow
            let v = voltage[u32(by) * u32(camera.grid_w) + u32(bx)];
            let glow_intensity = clamp(v / 12.0, 0.0, 1.0);
            wire_col = mix(wire_col, vec3<f32>(1.0, 0.85, 0.3), glow_intensity * 0.5);
            // Animated pulse showing current flow
            let pulse = sin(world_x * 4.0 + world_y * 4.0 - camera.time * 8.0) * 0.5 + 0.5;
            wire_col += vec3<f32>(0.08, 0.06, 0.01) * pulse * glow_intensity;
            // Wire bridge: segment-specific visual
            if btype == BT_WIRE_BRIDGE {
                let wb_seg = (bflags >> 3u) & 3u;
                let wb_rot = (bflags >> 5u) & 3u;
                let bridge_is_ns = wb_rot % 2u == 0u;
                let along = select(fx - 0.5, fy - 0.5, bridge_is_ns);
                if wb_seg == 0u || wb_seg == 2u {
                    // Entry/exit: ramp going underground
                    let toward_mid = select(along, -along, wb_seg == 0u);
                    if toward_mid > 0.1 {
                        let ramp = smoothstep(0.1, 0.5, toward_mid);
                        wire_col = mix(wire_col, vec3(0.15, 0.12, 0.08), ramp * 0.6);
                    }
                } else {
                    // Middle: dashed underground indicator
                    let perp = select(abs(fy - 0.5), abs(fx - 0.5), bridge_is_ns);
                    if perp < 0.03 {
                        let dash = fract(select(fx, fy, bridge_is_ns) * 4.0);
                        if dash < 0.5 {
                            wire_col = mix(wire_col, vec3(0.3, 0.25, 0.15), 0.4);
                        }
                    }
                }
            }
            color = wire_col;
        } else {
            color = ground;
        }
    } else if btype == BT_SOLAR {
        // Solar panel: 3×3 tile panel. Segment info in flags: bits3-4=col, bits5-6=row
        let seg_col = f32((bflags >> 3u) & 3u);
        let seg_row = f32((bflags >> 5u) & 3u);
        // Global UV across the full 3×3 panel (0-1 over the whole panel)
        let gx = (seg_col + fx) / 3.0;
        let gy = (seg_row + fy) / 3.0;
        // Panel frame: dark aluminum border around the entire 3×3
        let frame_w = 0.02;
        let on_frame = gx < frame_w || gx > (1.0 - frame_w) || gy < frame_w || gy > (1.0 - frame_w);
        if on_frame {
            color = vec3<f32>(0.40, 0.40, 0.42); // aluminum frame
        } else {
            // Silicon cells: 6×6 grid across the full panel
            let cell_x = fract(gx * 6.0);
            let cell_y = fract(gy * 6.0);
            // Dark blue silicon with per-cell variation
            let cell_id = floor(gx * 6.0) * 7.0 + floor(gy * 6.0) * 13.0;
            let sv = fract(sin(cell_id * 17.1 + world_x * 0.01) * 43758.5) * 0.02;
            color = vec3<f32>(0.08 + sv, 0.12 + sv, 0.28 + sv);
            // Cell grid lines (thin silver conductors)
            if cell_x < 0.04 || cell_y < 0.04 {
                color = vec3<f32>(0.50, 0.50, 0.55);
            }
            // Horizontal bus bars (thicker, every 2 cells)
            let bus = fract(gy * 3.0);
            if bus < 0.015 || bus > 0.985 {
                color = vec3<f32>(0.55, 0.55, 0.58);
            }
            // Sun reflection: slight specular sheen
            let reflect = camera.sun_intensity * 0.08;
            color += vec3<f32>(reflect * 0.5, reflect * 0.5, reflect);
        }
        // Voltage indicator: subtle green glow on active panel
        let v = voltage[u32(by) * u32(camera.grid_w) + u32(bx)];
        if v > 0.5 {
            color += vec3<f32>(0.02, 0.06, 0.01) * clamp(v / 12.0, 0.0, 1.0);
        }
    } else if btype == BT_BATTERY_S {
        // Battery: green casing with charge indicator
        let margin = 0.12;
        let on_case = fx > margin && fx < (1.0 - margin) && fy > margin && fy < (1.0 - margin);
        if on_case {
            color = vec3<f32>(0.28, 0.40, 0.25);
            // Terminal bumps at top
            if fy < margin + 0.12 && (abs(fx - 0.35) < 0.06 || abs(fx - 0.65) < 0.06) {
                color = vec3<f32>(0.50, 0.50, 0.55); // metal terminals
            }
            // Charge level bar
            let v = voltage[u32(by) * u32(camera.grid_w) + u32(bx)];
            let charge = clamp(v / 12.0, 0.0, 1.0);
            let bar_y = (fy - margin - 0.15) / 0.55;
            if bar_y > 0.0 && bar_y < 1.0 && fx > 0.3 && fx < 0.7 {
                if bar_y < charge {
                    let g = mix(vec3<f32>(0.8, 0.2, 0.1), vec3<f32>(0.2, 0.8, 0.2), charge);
                    color = g;
                } else {
                    color = vec3<f32>(0.15, 0.15, 0.15);
                }
            }
        } else {
            color = vec3<f32>(0.42, 0.35, 0.22); // ground
        }
    } else if btype == BT_BATTERY_M {
        // Medium battery (2 tiles): darker green, seamless across tiles
        let seg = (bflags >> 3u) & 1u;
        let rot = (bflags >> 5u) & 3u;
        var gx39 = fx; var gy39 = fy;
        if rot == 0u { gx39 = (f32(seg) + fx) / 2.0; }
        else { gy39 = (f32(seg) + fy) / 2.0; }
        let m39 = 0.06;
        let on39 = gx39 > m39 && gx39 < (1.0 - m39) && gy39 > m39 && gy39 < (1.0 - m39);
        if on39 {
            color = vec3<f32>(0.24, 0.36, 0.22);
            // Terminals at one end
            if gy39 < m39 + 0.08 && (abs(gx39 - 0.35) < 0.04 || abs(gx39 - 0.65) < 0.04) {
                color = vec3<f32>(0.50, 0.50, 0.55);
            }
            let v39 = voltage[u32(by) * u32(camera.grid_w) + u32(bx)];
            let ch39 = clamp(v39 / 12.0, 0.0, 1.0);
            let bar39 = (gy39 - m39 - 0.10) / 0.74;
            if bar39 > 0.0 && bar39 < 1.0 && gx39 > 0.25 && gx39 < 0.75 {
                if bar39 < ch39 { color = mix(vec3<f32>(0.8, 0.2, 0.1), vec3<f32>(0.2, 0.8, 0.2), ch39); }
                else { color = vec3<f32>(0.12, 0.12, 0.12); }
            }
        } else { color = vec3<f32>(0.42, 0.35, 0.22); }
    } else if btype == BT_BATTERY_L {
        // Large battery (2×2): industrial green, seamless across 4 tiles
        let col40 = (bflags >> 3u) & 1u;
        let row40 = (bflags >> 5u) & 1u;
        let gx40 = (f32(col40) + fx) / 2.0;
        let gy40 = (f32(row40) + fy) / 2.0;
        let m40 = 0.04;
        let on40 = gx40 > m40 && gx40 < (1.0 - m40) && gy40 > m40 && gy40 < (1.0 - m40);
        if on40 {
            color = vec3<f32>(0.20, 0.32, 0.20);
            // Heavy terminals
            if gy40 < m40 + 0.06 && (abs(gx40 - 0.30) < 0.04 || abs(gx40 - 0.70) < 0.04) {
                color = vec3<f32>(0.48, 0.48, 0.52);
            }
            // Bolts at corners
            let cb40 = (gx40 < m40 + 0.06 || gx40 > 1.0 - m40 - 0.06)
                     && (gy40 < m40 + 0.06 || gy40 > 1.0 - m40 - 0.06);
            if cb40 { color = vec3<f32>(0.45, 0.45, 0.48); }
            let v40 = voltage[u32(by) * u32(camera.grid_w) + u32(bx)];
            let ch40 = clamp(v40 / 12.0, 0.0, 1.0);
            let bar40 = (gy40 - m40 - 0.08) / 0.80;
            if bar40 > 0.0 && bar40 < 1.0 && gx40 > 0.20 && gx40 < 0.80 {
                if bar40 < ch40 { color = mix(vec3<f32>(0.8, 0.2, 0.1), vec3<f32>(0.1, 0.7, 0.2), ch40); }
                else { color = vec3<f32>(0.10, 0.10, 0.10); }
            }
            // Panel lines
            if abs(gx40 - 0.5) < 0.008 { color *= 0.85; }
        } else { color = vec3<f32>(0.42, 0.35, 0.22); }
    } else if btype == BT_WIND_TURBINE {
        // Wind turbine: 2×2 with 3 long blades spinning from center hub
        // Segment: bits 3-4 = col (0-1), bits 5-6 = row (0-1), bit 6 (0x40) = rotation
        let wt_col = f32((bflags >> 3u) & 1u);
        let wt_row = f32((bflags >> 5u) & 1u);
        let wt_rotated = (bflags & 0x40u) != 0u; // true = E-W wind orientation
        // Global UV across the full 2×2 turbine (0-1)
        let wt_gx = (wt_col + fx) / 2.0;
        let wt_gy = (wt_row + fy) / 2.0;
        let wt_cx = wt_gx - 0.5;
        let wt_cy = wt_gy - 0.5;
        let wt_dist = length(vec2<f32>(wt_cx, wt_cy));
        // Compute wind component perpendicular to the turbine blades
        // N-S orientation (rot=0): blades face E-W, wind from N or S powers it
        // E-W orientation (rot=1): blades face N-S, wind from E or W powers it
        var wind_perp = 0.0;
        if wt_rotated {
            wind_perp = abs(camera.sun_dir_x); // approximate with sun_dir... no, use wind_magnitude
            // Wind perpendicular component for E-W facing
        } else {
            // Wind perpendicular component for N-S facing
        }
        // Blade spin speed: proportional to wind magnitude, only forward
        let spin_speed = max(camera.wind_magnitude * 0.3, 0.0);
        // Ground base
        color = vec3<f32>(0.42, 0.35, 0.22);
        // Tower base: concrete pad
        if wt_dist < 0.42 {
            let pad_dist = wt_dist / 0.42;
            color = mix(vec3<f32>(0.48, 0.47, 0.45), vec3<f32>(0.42, 0.35, 0.22), pad_dist);
        }
        // Spinning blades: 3 blades, long and tapered
        if wt_dist < 0.46 && wt_dist > 0.04 {
            let blade_angle = atan2(wt_cy, wt_cx) + camera.time * spin_speed;
            let blade_phase = fract(blade_angle / 2.094); // 2π/3 = 3 blades
            // Tapered blade: wider near hub, narrow at tip
            let taper = 0.04 + (1.0 - wt_dist / 0.46) * 0.08;
            let on_blade = blade_phase < taper && wt_dist > 0.06;
            if on_blade {
                // Blade color: white with subtle gradient
                let blade_t = wt_dist / 0.46;
                color = mix(vec3<f32>(0.90, 0.90, 0.92), vec3<f32>(0.75, 0.75, 0.78), blade_t);
                // Leading edge highlight
                if blade_phase < taper * 0.3 {
                    color += vec3<f32>(0.05);
                }
                // Motion blur at high speed
                if spin_speed > 2.0 {
                    let blur_alpha = clamp((spin_speed - 2.0) * 0.1, 0.0, 0.4);
                    color = mix(color, vec3<f32>(0.42, 0.35, 0.22), blur_alpha * wt_dist);
                }
            }
        }
        // Center hub
        if wt_dist < 0.06 {
            color = vec3<f32>(0.55, 0.55, 0.58); // steel hub
            if wt_dist < 0.03 {
                color = vec3<f32>(0.45, 0.45, 0.48); // hub center bolt
            }
        }
        // Nacelle housing (elongated behind hub based on orientation)
        var nacelle_fx = wt_cx;
        var nacelle_fy = wt_cy;
        if wt_rotated {
            // E-W wind: nacelle points along X
            if abs(nacelle_fy) < 0.04 && nacelle_fx > -0.02 && nacelle_fx < 0.10 {
                color = vec3<f32>(0.52, 0.52, 0.55);
            }
        } else {
            // N-S wind: nacelle points along Y
            if abs(nacelle_fx) < 0.04 && nacelle_fy > -0.02 && nacelle_fy < 0.10 {
                color = vec3<f32>(0.52, 0.52, 0.55);
            }
        }
        // Voltage glow on hub
        let wv = voltage[u32(by) * u32(camera.grid_w) + u32(bx)];
        if wv > 0.5 && wt_dist < 0.06 {
            color += vec3<f32>(0.1, 0.15, 0.05) * clamp(wv / 12.0, 0.0, 1.0);
        }
    } else if btype == BT_SWITCH {
        // Switch: toggle box on wire
        let ground = vec3<f32>(0.42, 0.35, 0.22);
        let sw_on = (bflags & 4u) != 0u;
        let sw_cdist = length(vec2<f32>(fx - 0.5, fy - 0.5));
        let on_sw_wire = abs(fy - 0.5) < 0.06 || abs(fx - 0.5) < 0.06 || sw_cdist < 0.10;
        let box_size = 0.18;
        let in_box = abs(fx - 0.5) < box_size && abs(fy - 0.5) < box_size;
        if in_box {
            color = vec3<f32>(0.40, 0.38, 0.35);
            let toggle_y = select(0.5 + 0.08, 0.5 - 0.08, sw_on);
            let toggle_dist = length(vec2<f32>(fx - 0.5, fy - toggle_y));
            if toggle_dist < 0.06 {
                color = select(vec3<f32>(0.4, 0.15, 0.15), vec3<f32>(0.2, 0.8, 0.2), sw_on);
            }
            if abs(fx - 0.5) > box_size - 0.025 || abs(fy - 0.5) > box_size - 0.025 {
                color = vec3<f32>(0.30, 0.28, 0.25);
            }
        } else if on_sw_wire {
            var wc = vec3<f32>(0.55, 0.38, 0.20);
            let sv = voltage[u32(by) * u32(camera.grid_w) + u32(bx)];
            wc = mix(wc, vec3<f32>(1.0, 0.85, 0.3), clamp(sv / 12.0, 0.0, 1.0) * 0.5);
            color = wc;
        } else {
            color = ground;
        }
    } else if btype == BT_DIMMER {
        // Dimmer: rotary knob on wire
        let ground = vec3<f32>(0.42, 0.35, 0.22);
        let dim_level = f32(bheight) / 10.0;
        let dm_cdist = length(vec2<f32>(fx - 0.5, fy - 0.5));
        let on_dm_wire = abs(fy - 0.5) < 0.06 || abs(fx - 0.5) < 0.06 || dm_cdist < 0.10;
        if dm_cdist < 0.20 {
            color = vec3<f32>(0.32, 0.30, 0.28);
            let knob_angle = atan2(fy - 0.5, fx - 0.5);
            let norm_angle = (knob_angle + 3.14159) / 6.28318;
            if norm_angle < dim_level && dm_cdist > 0.08 && dm_cdist < 0.17 {
                color = mix(vec3<f32>(0.3, 0.15, 0.05), vec3<f32>(1.0, 0.7, 0.2), dim_level);
            }
            if dm_cdist < 0.05 { color = vec3<f32>(0.50, 0.48, 0.45); }
            if dm_cdist > 0.17 { color = vec3<f32>(0.25, 0.23, 0.20); }
        } else if on_dm_wire {
            var wc = vec3<f32>(0.55, 0.38, 0.20);
            let dv = voltage[u32(by) * u32(camera.grid_w) + u32(bx)];
            wc = mix(wc, vec3<f32>(1.0, 0.85, 0.3), clamp(dv / 12.0, 0.0, 1.0) * 0.5);
            color = wc;
        } else {
            color = ground;
        }
    } else if btype == BT_BREAKER {
        // Circuit breaker: box with trip indicator on wire
        let ground = vec3<f32>(0.42, 0.35, 0.22);
        let breaker_on = (bflags & 4u) != 0u;
        let cb_cdist = length(vec2<f32>(fx - 0.5, fy - 0.5));
        let on_cb_wire = abs(fy - 0.5) < 0.06 || abs(fx - 0.5) < 0.06 || cb_cdist < 0.10;
        // Breaker box
        let box_w = 0.22;
        let box_h = 0.18;
        let in_box = abs(fx - 0.5) < box_w && abs(fy - 0.5) < box_h;
        if in_box {
            // Housing
            color = vec3<f32>(0.35, 0.33, 0.30);
            // Toggle lever
            let lever_x = select(0.5 - 0.08, 0.5 + 0.08, breaker_on);
            let lever_dist = length(vec2<f32>(fx - lever_x, fy - 0.5));
            if lever_dist < 0.07 {
                color = select(vec3<f32>(0.8, 0.15, 0.1), vec3<f32>(0.15, 0.6, 0.15), breaker_on);
            }
            // Warning stripe (yellow/black) at top
            if fy < 0.5 - box_h + 0.05 {
                let stripe = fract(fx * 6.0);
                color = select(vec3<f32>(0.15, 0.15, 0.12), vec3<f32>(0.85, 0.75, 0.1), stripe > 0.5);
            }
            // Box outline
            if abs(fx - 0.5) > box_w - 0.02 || abs(fy - 0.5) > box_h - 0.02 {
                color = vec3<f32>(0.25, 0.23, 0.20);
            }
            // Tripped flash: red pulse when recently tripped
            if !breaker_on {
                let pulse = sin(camera.time * 6.0) * 0.3 + 0.7;
                color = mix(color, vec3<f32>(0.8, 0.1, 0.05), pulse * 0.3);
            }
        } else if on_cb_wire {
            var wc = vec3<f32>(0.55, 0.38, 0.20);
            let cv = voltage[u32(by) * u32(camera.grid_w) + u32(bx)];
            wc = mix(wc, vec3<f32>(1.0, 0.85, 0.3), clamp(cv / 12.0, 0.0, 1.0) * 0.5);
            color = wc;
        } else {
            color = ground;
        }
    } else if btype == BT_INSULATED {
        // Insulated wall: outer shell with fiberglass insulation core
        let edge = 0.15; // outer shell thickness
        let in_core = fx > edge && fx < (1.0 - edge) && fy > edge && fy < (1.0 - edge);
        if in_core {
            // Insulation core: pink/yellow fiberglass with fibrous texture
            let fiber1 = fract(sin(world_x * 47.3 + world_y * 89.1) * 43758.5);
            let fiber2 = fract(sin(world_x * 131.7 + world_y * 53.9) * 27183.6);
            let fiber_line = abs(fract(fx * 8.0 + fiber1 * 0.3) - 0.5) +
                             abs(fract(fy * 6.0 + fiber2 * 0.3) - 0.5);
            let fiber_shade = smoothstep(0.3, 0.7, fiber_line) * 0.08;
            color = vec3<f32>(0.88, 0.75, 0.55) + vec3<f32>(fiber_shade, -fiber_shade * 0.5, -fiber_shade);
            // Pink tint patches
            let pink = fract(sin(world_x * 23.1 + world_y * 67.3) * 31415.9);
            if pink > 0.6 {
                color = mix(color, vec3<f32>(0.90, 0.65, 0.60), 0.3);
            }
        } else {
            // Outer shell: light gray with subtle panel lines
            color = vec3<f32>(0.88, 0.88, 0.90);
            let panel_x = abs(fract(fx * 2.0) - 0.5);
            let panel_y = abs(fract(fy * 2.0) - 0.5);
            if panel_x < 0.03 || panel_y < 0.03 {
                color -= vec3<f32>(0.05);
            }
        }
    } else if btype == BT_MUD_WALL {
        // Mud wall: organic rounded shape with craggy surface texture
        let cx = fx - 0.5;
        let cy = fy - 0.5;
        // Rounded shape — distance from center with noise for cragginess
        let crag1 = fract(sin(world_x * 37.1 + world_y * 73.7) * 43758.5) * 0.06;
        let crag2 = fract(sin(world_x * 91.3 + world_y * 29.1) * 27183.6) * 0.04;
        let crag3 = fract(sin(world_x * 17.9 - world_y * 113.3) * 61283.1) * 0.03;
        let dist = length(vec2<f32>(cx, cy)) + crag1 + crag2 - 0.05;
        // Rounded top: center is highest, edges are lower
        let height_fade = 1.0 - smoothstep(0.2, 0.45, dist);
        // Base mud color with variation
        let mud_var = (crag1 - 0.03) * 3.0;
        color = vec3<f32>(0.52 + mud_var, 0.40 + mud_var * 0.8, 0.25 + mud_var * 0.5);
        // Craggy surface: darker cracks
        let crack1 = abs(fract(fx * 5.0 + crag2 * 4.0 + fy * 0.5) - 0.5);
        let crack2 = abs(fract(fy * 4.0 + crag1 * 3.0 + fx * 0.3) - 0.5);
        let crack = min(crack1, crack2);
        if crack < 0.06 {
            color *= 0.82;
        }
        // Subtle straw/fiber inclusions
        let straw = fract(sin(world_x * 211.1 + world_y * 137.3) * 43758.5);
        if straw > 0.92 {
            color = mix(color, vec3<f32>(0.65, 0.58, 0.35), 0.4);
        }
        // Rounded brightness (center brighter, edges darker)
        color *= 0.85 + height_fade * 0.15;
        // Edge darkening
        if dist > 0.38 {
            color *= 0.7;
        }
    } else if btype == BT_LOW_WALL {
        // Low cover wall: earthy mound, visually shorter/lighter than mud wall
        let cx = fx - 0.5;
        let cy = fy - 0.5;
        let crag = fract(sin(world_x * 37.1 + world_y * 73.7) * 43758.5) * 0.05;
        let dist = length(vec2<f32>(cx, cy)) + crag - 0.03;
        let mud_var = (crag - 0.025) * 3.0;
        color = vec3<f32>(0.56 + mud_var, 0.44 + mud_var * 0.8, 0.30 + mud_var * 0.5);
        // Subtler cracks than full mud wall
        let crack = abs(fract(fx * 4.0 + crag * 3.0 + fy * 0.4) - 0.5);
        if crack < 0.05 {
            color *= 0.85;
        }
        // Rounded brightness
        let height_fade = 1.0 - smoothstep(0.15, 0.42, dist);
        color *= 0.88 + height_fade * 0.12;
        if dist > 0.36 {
            color *= 0.75;
        }
    } else if btype == BT_DIAGONAL {
        // Diagonal wall: half-cell wall, half floor
        let diag_variant = (bflags >> 3u) & 3u;
        let on_wall = diag_is_wall(fx, fy, diag_variant);
        if on_wall {
            // Wall half: stone color with diagonal mortar lines
            let m44 = get_material(44u);
            color = vec3(m44.color_r, m44.color_g, m44.color_b);
            let mortar_coord = select(fx + fy, fy - fx + 1.0, diag_variant < 2u);
            let mortar = fract(mortar_coord * 3.0);
            let mortar_line = f32(mortar < 0.06) * 0.04;
            color -= vec3(mortar_line);
            // Perpendicular mortar
            let mortar2 = fract((fx - fy + 1.0) * 3.0);
            color -= vec3(f32(mortar2 < 0.04) * 0.03);
        } else {
            // Open half: show underlying floor
            color = block_base_color(2u, 0u); // dirt
            // Diagonal face (oblique along the diagonal edge)
            let d = diag_dist_to_edge(fx, fy, diag_variant);
            let face_w = min(f32(bheight) * camera.oblique_strength * 0.5, 0.18);
            if d < face_w {
                let ft = d / face_w; // 0=at edge, 1=far from edge
                let m44 = get_material(44u);
                var fc = vec3(m44.color_r, m44.color_g, m44.color_b);
                fc *= (0.55 + 0.45 * ft); // darker near ground
                let mortar_c = select(fx + fy, fy - fx + 1.0, diag_variant < 2u);
                fc -= vec3(f32(fract(mortar_c * 3.0) < 0.06) * 0.03);
                color = fc;
            }
        }
    } else if btype == BT_CROP {
        // Crop: growth stages shown as increasingly green/tall plants
        let stage = bheight; // 0=planted, 1=sprout, 2=growing, 3=mature
        let ground = vec3<f32>(0.40, 0.32, 0.18); // tilled soil (darker than dirt)
        var crop_color = ground;

        let cx = fx - 0.5;
        let cy = fy - 0.5;

        if stage == 0u {
            // Planted: just tilled soil with seed dots
            let seed = fract(sin(world_x * 17.3 + world_y * 31.7) * 43758.5);
            if seed > 0.85 {
                crop_color = vec3(0.35, 0.30, 0.15);
            }
        } else if stage == 1u {
            // Sprout: tiny green dots
            let sprout_r = 0.12;
            let dist = length(vec2(cx, cy));
            if dist < sprout_r {
                crop_color = vec3(0.25, 0.50, 0.15);
            }
        } else if stage == 2u {
            // Growing: larger green cross shape
            let on_plant = (abs(cx) < 0.08 || abs(cy) < 0.08) && length(vec2(cx, cy)) < 0.25;
            if on_plant {
                crop_color = vec3(0.20, 0.55, 0.12);
            } else {
                // Leaves around the stem
                let leaf_dist = length(vec2(cx, cy));
                if leaf_dist < 0.2 {
                    crop_color = mix(ground, vec3(0.22, 0.45, 0.15), 0.5);
                }
            }
        } else {
            // Mature: full green with golden wheat tips
            let on_plant = length(vec2(cx, cy)) < 0.35;
            if on_plant {
                let gold = fract(sin(world_x * 23.0 + world_y * 41.0) * 43758.5);
                let plant_green = vec3(0.22, 0.55, 0.10);
                let plant_gold = vec3(0.65, 0.55, 0.20);
                crop_color = mix(plant_green, plant_gold, gold * 0.4);
            }
        }
        color = crop_color;
    } else {
        // Wall rendering: check wall_data layer first (DN-008), fall back to legacy block
        if is_door_pixel {
            // Physical door: jamb or leaf color
            color = door_pixel_color;
        } else if is_wd_wall {
            // Wall pixel from wall_data layer — use wall material color
            let wmat = wd_material_s(wd);
            color = wall_material_color(wmat);
        } else if wd != 0u && !wd_is_wall_pixel {
            // Open portion of a tile with walls — show the underlying block
            color = block_base_color(btype, bflags);
        } else if is_thin_wall(bflags) && bheight > 0u && matches_wall_type(btype) && !pixel_is_wall(fx, fy, bheight_raw, bflags) {
            // Legacy: open portion of thin wall from block grid
            color = block_base_color(2u, 0u);
        } else {
            color = block_base_color(btype, bflags);
        }
    }

    // --- Level indicator bar for adjustable blocks ---
    // Generalized: any block with a variable level gets a thin bar at the bottom.
    // Level is read from the height byte, normalized to 0..1 by max_level.
    {
        var adj_level = -1.0;   // negative = not adjustable
        var adj_color = vec3(1.0, 0.8, 0.2); // default: warm yellow
        var adj_max = 10.0;
        if btype == BT_DIMMER {
            adj_level = f32(bheight); adj_max = 10.0;
            adj_color = vec3(1.0, 0.9, 0.3); // brightness yellow
        } else if btype == BT_RESTRICTOR {
            adj_level = f32(bheight & 0xFu); adj_max = 10.0;
            adj_color = vec3(0.3, 0.7, 1.0); // flow blue
        } else if btype == BT_FIREPLACE || btype == BT_CAMPFIRE {
            adj_level = f32(bheight); adj_max = 10.0;
            adj_color = vec3(1.0, 0.5, 0.15); // fire orange
        } else if btype == BT_FAN {
            // Fan speed stored in height byte
            adj_level = f32(bheight); adj_max = 10.0;
            adj_color = vec3(0.3, 0.8, 0.9); // teal
        }
        if adj_level >= 0.0 {
            let bar_y = fy - 0.92; // bottom of tile
            let bar_x = (fx - 0.1) / 0.8; // 80% width centered
            if bar_y > 0.0 && bar_y < 0.06 && bar_x > 0.0 && bar_x < 1.0 {
                let fill = adj_level / adj_max;
                if bar_x < fill {
                    color = mix(color, adj_color, 0.8);
                } else {
                    color = mix(color, vec3(0.1), 0.5); // dark empty portion
                }
            }
        }
    }

    // --- Procedural terrain detail (ground-level blocks only) ---
    // Replace this block with sprite sampling when migrating to sprites.
    let is_scorched_dirt = btype == BT_GROUND && (bflags & 8u) != 0u; // bit 3 = scorched
    let is_transparent_vegetation = (btype == BT_TREE || btype == BT_BERRY_BUSH) && !is_tree_pixel;
    let is_ground_tile = btype == BT_GROUND || btype == BT_AIR || btype == BT_DUG_GROUND
        || btype == BT_ROCK || is_transparent_vegetation;
    if camera.enable_terrain_detail > 0.5 && !is_tree_pixel && (bheight == 0u || is_transparent_vegetation) && is_ground_tile && !is_scorched_dirt {
        // Dirt / air (ground): full terrain detail with grass, flowers, pebbles
        let td_idx = u32(by) * u32(camera.grid_w) + u32(bx);
        let td_wt = water_table_buf[td_idx];
        let td_elev = sample_elevation(world_x, world_y); // smooth interpolated
        // Hilltops are drier (reduce effective water table with elevation)
        let effective_wt = td_wt - td_elev * 0.3;
        color = terrain_detail(color, world_x, world_y, bx, by,
            effective_wt, camera.rain_intensity, camera.wind_angle, camera.time);
    } else if is_scorched_dirt {
        // Scorched dirt (grass burned away): dark charred earth, no vegetation
        let char_noise = value_noise(vec2(world_x * 3.0, world_y * 3.0));
        let ash_col = mix(vec3<f32>(0.15, 0.12, 0.08), vec3<f32>(0.25, 0.20, 0.14), char_noise);
        // Scattered ash flecks
        let ash_fleck = value_noise(vec2(world_x * 8.0 + 17.3, world_y * 8.0 + 41.7));
        let fleck_col = mix(ash_col, vec3<f32>(0.35, 0.32, 0.28), smoothstep(0.7, 0.8, ash_fleck));
        color = fleck_col;
    } else if camera.enable_terrain_detail > 0.5 && bheight > 0u && btype == BT_STONE {
        // Stone block surface: cracks, veins, strata
        color = stone_detail(color, world_x, world_y);
    } else if camera.enable_terrain_detail > 0.5 && btype == BT_WOOD_FLOOR {
        // Finished wood floor: planks with grain, knots, nails
        color = wood_floor_detail(world_x, world_y);
    } else if camera.enable_terrain_detail > 0.5 && btype == BT_ROUGH_FLOOR {
        // Early-game rough floor: unfinished planks with gaps and dirt
        color = rough_floor_detail(world_x, world_y);
    }

    // (slope face disabled — terrain AO handles depth)

    // --- Elevation visual cues (ground-level blocks only) ---
    // Uses bilinear-interpolated elevation for smooth gradients (no tile-edge jaggies).
    if camera.enable_terrain_detail > 0.5 && !is_tree_pixel && bheight == 0u && btype != BT_TREE && btype != BT_BERRY_BUSH {
        let elev = sample_elevation(world_x, world_y);

        // 1. Altitude brightness: higher = lighter, lower = darker (topographic convention)
        let elev_bright = 1.0 + elev * 0.06;
        color *= elev_bright;

        // 2. Hillshade: terrain normal dotted with sun direction
        // Creates directional lighting on slopes — sun-facing bright, shadow-facing dark.
        let e_dx = sample_elevation(world_x + 0.5, world_y) - sample_elevation(world_x - 0.5, world_y);
        let e_dy = sample_elevation(world_x, world_y + 0.5) - sample_elevation(world_x, world_y - 0.5);
        let terrain_normal = normalize(vec3(-e_dx, -e_dy, 1.5));
        // Use a fixed northwest illumination for consistent depth perception,
        // blended with the actual sun direction when sun is up.
        let fixed_light = normalize(vec3(-0.5, -0.7, 1.5)); // NW light (cartographic standard)
        let sun3d = normalize(vec3(sun_dir.x, sun_dir.y, max(sun_elev, 0.5)));
        let light_dir = normalize(mix(fixed_light, sun3d, camera.sun_intensity * 0.6));
        let hillshade = dot(terrain_normal, light_dir);
        // Apply ±15% brightness based on slope facing
        color *= 0.92 + hillshade * 0.15;

        // 3. Terrain ambient occlusion: pre-computed structural shadows from dawn+dusk rays.
        // Valleys and terrain folds get permanent soft darkening; hilltops stay bright.
        if camera.terrain_ao_strength > 0.01 {
            let terrain_ao = sample_terrain_ao(world_x, world_y);
            color *= mix(1.0, terrain_ao, camera.terrain_ao_strength);
        }
    }

    // --- Elevation depth cues (edge darkening + hypsometric tinting) ---
    if bheight == 0u && !is_wall_face && !is_tree_pixel && btype != BT_TREE && btype != BT_BERRY_BUSH && camera.enable_terrain_detail > 0.5 {
        let e_here = sample_elevation(world_x, world_y);

        // Edge darkening: darken where terrain drops to neighbors (crevice shadows)
        let e_n = sample_elevation(world_x, world_y - 0.5);
        let e_s = sample_elevation(world_x, world_y + 0.5);
        let e_e = sample_elevation(world_x + 0.5, world_y);
        let e_w = sample_elevation(world_x - 0.5, world_y);
        let drop = max(
            max(max(e_here - e_n, 0.0), max(e_here - e_s, 0.0)),
            max(max(e_here - e_e, 0.0), max(e_here - e_w, 0.0))
        );
        let edge_shadow = smoothstep(0.0, 2.0, drop) * 0.20;
        color *= 1.0 - edge_shadow;

        // Hypsometric tinting: valleys cooler, hills warmer
        let hyp = clamp(e_here / 5.0, 0.0, 1.0);
        color *= mix(vec3(0.96, 0.97, 1.0), vec3(1.02, 1.0, 0.97), hyp);

        // Contour lines (controlled by camera uniforms)
        // Only draw where there's actual elevation — skip flat ground near zero
        if camera.contour_opacity > 0.01 && e_here > 0.15 {
            let c_iv = camera.contour_interval;
            let c_phase = fract(e_here / c_iv);
            let c_dist = abs(c_phase - 0.5) - 0.48;
            let c_line = smoothstep(0.01, -0.01, c_dist) * 0.05;
            let m_iv = c_iv * camera.contour_major_mul;
            let m_phase = fract(e_here / m_iv);
            let m_dist = abs(m_phase - 0.5) - 0.47;
            let m_line = smoothstep(0.015, -0.015, m_dist) * 0.10;
            let line = max(c_line, m_line) * camera.contour_opacity;
            color = mix(color, vec3(0.15, 0.12, 0.08), line);
        }
    }

    // Open door: treat as floor-level opening (overrides wall type)
    // Check both grid_data doors and wall_data doors (DN-008)
    let door_is_open = (is_door(block) && is_open(block)) || (wd_has_door(wd) && wd_door_open(wd));
    // Trees: transparent sprite pixels are ground-level; canopy keeps height for shadows
    let is_tree_ground = (btype == BT_TREE || btype == BT_BERRY_BUSH) && !is_tree_pixel;
    let is_pipe = (btype >= BT_PIPE && btype <= BT_INLET) || btype == BT_RESTRICTOR || btype == BT_LIQUID_PIPE || btype == BT_PIPE_BRIDGE || btype == BT_LIQUID_INTAKE || btype == BT_LIQUID_PUMP || btype == BT_LIQUID_OUTPUT;
    let is_dug = btype == BT_DUG_GROUND; // dug ground: height = depth, not visual height
    let is_rock = btype == BT_ROCK;
    let is_crate = btype == BT_CRATE; // crate height = item count, not visual height
    let is_wire = btype == BT_WIRE || btype == BT_WIRE_BRIDGE; // wire height = connection mask, not visual
    let is_dimmer = btype == BT_DIMMER || btype == BT_FIREPLACE || btype == BT_CAMPFIRE; // height = level/intensity, not visual
    let is_breaker = btype == BT_BREAKER; // breaker height = threshold, not visual
    let is_plant = btype == BT_CROP; // crop height = growth stage, not visual (berry bush handled by is_tree_ground)
    let is_diag_open = btype == BT_DIAGONAL && !diag_is_wall(fx, fy, (bflags >> 3u) & 3u);
    let is_thin_open = is_thin_wall(bflags) && bheight > 0u && matches_wall_type(btype) && !pixel_is_wall(fx, fy, bheight_raw, bflags);
    let effective_height = select(bheight, 0u, door_is_open || is_tree_ground || is_pipe || is_dug || is_rock || is_crate || is_wire || is_dimmer || is_breaker || is_plant || is_diag_open || is_thin_open);
    let effective_fheight = f32(effective_height);

    // Height-based brightness (skip for trees — they have their own shading)
    if btype != BT_TREE {
        color += vec3<f32>(effective_fheight * 0.03);
    }

    // Ground water rendering: edge-blended for smooth shorelines
    let wmax = vec2<i32>(i32(camera.grid_w) - 1, i32(camera.grid_h) - 1);
    let wc = textureLoad(water_tex, clamp(vec2<i32>(bx, by), vec2(0), wmax), 0).r;
    let wn = textureLoad(water_tex, clamp(vec2<i32>(bx, by - 1), vec2(0), wmax), 0).r;
    let ws = textureLoad(water_tex, clamp(vec2<i32>(bx, by + 1), vec2(0), wmax), 0).r;
    let we = textureLoad(water_tex, clamp(vec2<i32>(bx + 1, by), vec2(0), wmax), 0).r;
    let ww = textureLoad(water_tex, clamp(vec2<i32>(bx - 1, by), vec2(0), wmax), 0).r;
    let edge_w = smoothstep(0.4, 0.0, fx);
    let edge_e = smoothstep(0.6, 1.0, fx);
    let edge_n = smoothstep(0.4, 0.0, fy);
    let edge_s = smoothstep(0.6, 1.0, fy);
    let water_level = wc
        + (ww - wc) * edge_w * 0.5
        + (we - wc) * edge_e * 0.5
        + (wn - wc) * edge_n * 0.5
        + (ws - wc) * edge_s * 0.5;
    let wt_idx = u32(by) * 256u + u32(bx);
    let wt_depth = water_table_buf[wt_idx]; // negative = below ground, positive = spring
    let is_floor_tile = btype == BT_GROUND || btype == BT_WOOD_FLOOR || btype == BT_STONE_FLOOR || btype == BT_CONCRETE_FLOOR || btype == BT_ROUGH_FLOOR || btype == BT_DUG_GROUND;
    if is_floor_tile && !is_tree_pixel && effective_height == 0u {
        // Water table coloring: subtle moisture for high water table (even without surface water)
        let wt_moisture = clamp((wt_depth + 1.5) / 2.0, 0.0, 0.5); // 0 at -1.5, 0.5 at +0.5
        if wt_moisture > 0.02 && water_level < 0.1 {
            let damp_earth = vec3<f32>(0.32, 0.26, 0.16);
            color = mix(color, damp_earth, wt_moisture * 0.3);
        }
        // Combine water sim level with rain for immediate visual feedback
        let wet = clamp(water_level + camera.rain_intensity * 0.3, 0.0, 1.0);
        if wet > 0.01 {
            // Wet soil: mix toward absolute dark brown (not relative scaling)
            let wet_earth = vec3<f32>(0.18, 0.13, 0.07); // dark wet mud
            color = mix(color, wet_earth, wet * 0.6);

            // Puddle effect: when water level is significant, show reflective surface
            if water_level > 0.15 {
                let puddle_strength = clamp((water_level - 0.15) * 3.0, 0.0, 1.0);
                // Muddy water: dark brown-tinted, not gray
                let muddy_water = vec3<f32>(0.12, 0.09, 0.05);
                color = mix(color, muddy_water, puddle_strength * 0.3);
                // Subtle sky reflection (only a hint, mostly opaque muddy water)
                let rip1 = sin(world_x * 8.0 + world_y * 5.0 + camera.time * 2.0) * 0.015;
                let rip2 = sin(world_x * 4.0 - world_y * 9.0 + camera.time * 1.3) * 0.01;
                let sky_ref = vec3(0.3, 0.35, 0.45) * (camera.sun_intensity * 0.3 + 0.05);
                color = mix(color, sky_ref, puddle_strength * 0.12);
                // Small specular highlight
                let spec = pow(max(rip1 + rip2 + 0.5, 0.0), 12.0) * camera.sun_intensity * 0.08;
                color += vec3(spec) * puddle_strength;
            }
        }
    }

    // Water overlay is rendered in the main overlay chain below (layer 12)

    // Wall side faces (3D bevel) — skip for doors and trees
    if effective_height > 0u && btype != BT_TREE && btype != BT_DIAGONAL {
        color += wall_side_shade(world_x, world_y, bx, by, effective_height);
    }

    // Wire overlay: draw wire on top of walls/blocks that have wire flag (bit 7)
    let has_wire_flag = (bflags & 0x80u) != 0u;
    if has_wire_flag && btype != BT_WIRE {
        // Same wire rendering logic as standalone wire, but overlaid
        let wn_w = block_type(get_block(bx - 1, by));
        let wn_e = block_type(get_block(bx + 1, by));
        let wn_n = block_type(get_block(bx, by - 1));
        let wn_s = block_type(get_block(bx, by + 1));
        let wf_w = (get_block(bx - 1, by) >> 16u) & 0x80u;
        let wf_e = (get_block(bx + 1, by) >> 16u) & 0x80u;
        let wf_n = (get_block(bx, by - 1) >> 16u) & 0x80u;
        let wf_s = (get_block(bx, by + 1) >> 16u) & 0x80u;
        let wc_w = wn_w == BT_WIRE || wn_w == BT_SOLAR || wn_w == BT_BATTERY_S || wf_w != 0u;
        let wc_e = wn_e == BT_WIRE || wn_e == BT_SOLAR || wn_e == BT_BATTERY_S || wf_e != 0u;
        let wc_n = wn_n == BT_WIRE || wn_n == BT_SOLAR || wn_n == BT_BATTERY_S || wf_n != 0u;
        let wc_s = wn_s == BT_WIRE || wn_s == BT_SOLAR || wn_s == BT_BATTERY_S || wf_s != 0u;
        let ww = 0.06;
        var on_overlay_wire = false;
        if wc_w { on_overlay_wire = on_overlay_wire || (fx < 0.5 && abs(fy - 0.5) < ww); }
        if wc_e { on_overlay_wire = on_overlay_wire || (fx > 0.5 && abs(fy - 0.5) < ww); }
        if wc_n { on_overlay_wire = on_overlay_wire || (fy < 0.5 && abs(fx - 0.5) < ww); }
        if wc_s { on_overlay_wire = on_overlay_wire || (fy > 0.5 && abs(fx - 0.5) < ww); }
        let ow_cdist = length(vec2<f32>(fx - 0.5, fy - 0.5));
        on_overlay_wire = on_overlay_wire || ow_cdist < 0.08;
        if on_overlay_wire {
            var ow_col = vec3<f32>(0.55, 0.38, 0.20);
            let ow_v = voltage[u32(by) * u32(camera.grid_w) + u32(bx)];
            let ow_glow = clamp(ow_v / 12.0, 0.0, 1.0);
            ow_col = mix(ow_col, vec3<f32>(1.0, 0.85, 0.3), ow_glow * 0.5);
            color = ow_col;
        }
    }

    // Sub-tile elevation: sample 1024x1024 heightmap for slope shading
    let elev_uv_x = world_x * 4.0;
    let elev_uv_y = world_y * 4.0;
    let elev_x0 = i32(floor(elev_uv_x));
    let elev_y0 = i32(floor(elev_uv_y));
    let elev_max = vec2<i32>(i32(camera.grid_w) * 4 - 1, i32(camera.grid_h) * 4 - 1);
    // Bilinear sample elevation
    let e00 = textureLoad(elevation_tex, clamp(vec2(elev_x0, elev_y0), vec2(0), elev_max), 0).r;
    let e10 = textureLoad(elevation_tex, clamp(vec2(elev_x0 + 1, elev_y0), vec2(0), elev_max), 0).r;
    let e01 = textureLoad(elevation_tex, clamp(vec2(elev_x0, elev_y0 + 1), vec2(0), elev_max), 0).r;
    let e11 = textureLoad(elevation_tex, clamp(vec2(elev_x0 + 1, elev_y0 + 1), vec2(0), elev_max), 0).r;
    let efx = fract(elev_uv_x);
    let efy = fract(elev_uv_y);
    let sub_elev = mix(mix(e00, e10, efx), mix(e01, e11, efx), efy);
    // Slope shading: darken steep terrain (ditch walls, hillsides)
    // Use wider sample distance (2 sub-cells) for smoother, less noisy slopes
    let e_right = textureLoad(elevation_tex, clamp(vec2(elev_x0 + 2, elev_y0), vec2(0), elev_max), 0).r;
    let e_down = textureLoad(elevation_tex, clamp(vec2(elev_x0, elev_y0 + 2), vec2(0), elev_max), 0).r;
    let slope_dx = (e_right - e00) * 0.5;
    let slope_dy = (e_down - e00) * 0.5;
    let slope_mag = length(vec2(slope_dx, slope_dy));
    // Only darken significant slopes (not micro-noise), max 25% darkening
    let slope_shade = 1.0 - clamp((slope_mag - 0.01) * 2.0, 0.0, 0.25);
    color *= slope_shade;

    // Save pre-lighting base color for pleb torch/headlight illumination
    var base_color_prelit = color;

    // Shadow / interior lighting
    var shadow_tint = vec3<f32>(1.0);
    var light_factor = 1.0;
    var light_color_out = vec3<f32>(0.0);
    var light_intensity_out = 0.0;

    // Roofed wall tops: under the roof, no direct sun — just ambient
    let is_roofed_wall = is_roofed && bheight > 0u && camera.show_roofs < 0.5;

    if is_indoor {
        // Indoor floor: skip shadow ray entirely. The roof blocks all direct sun.
        // Sample lightmap for point lights (pre-computed per block).
        let lm = sample_lightmap(world_x, world_y);
        light_color_out = lm.xyz;
        light_intensity_out = lm.w;

        // Interior sun lighting still needs per-pixel sunbeam tracing.
        let sun_int = camera.sun_intensity;
        let interior = compute_interior_light(world_x, world_y, sun_int, sun_dir, 0.0);
        shadow_tint = interior.xyz;
        light_factor = interior.w;
    } else if is_roofed_wall {
        // Wall under a roof: no sun, no shadow ray. Just ambient + interior indirect.
        light_factor = INTERIOR_INDIRECT;
    } else {
        // Per-pixel shadow ray trace with IGN dithering for temporal AA
        let frame_offset = fract(camera.time * 5.3) * 5.0;
        let ign_x = f32(px) + frame_offset;
        let ign_y = f32(py) + frame_offset * 0.7;
        let ign1 = fract(52.9829189 * fract(0.06711056 * ign_x + 0.00583715 * ign_y));
        let ign2 = fract(52.9829189 * fract(0.00583715 * ign_x + 0.06711056 * ign_y));
        let shadow_dither = 0.06; // subtle penumbra softening (±0.03 tiles)
        let shadow_result = trace_shadow_ray(
            world_x + (ign1 - 0.5) * shadow_dither,
            world_y + (ign2 - 0.5) * shadow_dither,
            effective_fheight, sun_dir, sun_elev);
        shadow_tint = shadow_result.xyz;
        // Shadow intensity: 1.0 = full shadow, 0.0 = no shadow (always lit)
        light_factor = mix(1.0, shadow_result.w, camera.shadow_intensity);

        // DEBUG: output roof height as color — red channel = roof_h/5

        // Outdoor lighting: directional window bleed only.
        // Outdoor point lights are handled by proximity glow (line-of-sight traced)
        // rather than the lightmap (which floods around obstacles unrealistically).
        if camera.enable_dir_bleed > 0.5 && effective_height == 0u {
            let bleed = compute_directional_bleed(world_x, world_y);
            light_color_out = bleed.xyz;
            light_intensity_out = bleed.w;
        }
    }

    if get_material(btype).is_emissive > 0.5 && (btype != BT_TABLE_LAMP || is_table_lamp_bulb) {
        // Emissive block (fireplace/electric light): not affected by shadow/lighting.
        // Just clamp and output directly.
        color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
        textureStore(output, vec2<u32>(px, py), vec4<f32>(color, 1.0));
        return;
    }

    if btype == BT_GLASS && is_glass_pixel {
        // Glass pixel: translucent with tinted light + additive fire
        let glass_light = light_factor * 0.6 + 0.4;
        color = color * (ambient + sun_color * glass_light * 0.9 * shadow_tint);

        // Light glow through glass: use lightmap for point lights behind this window
        let glass_pt = sample_lightmap(world_x, world_y);
        let glass_pt_intensity = glass_pt.w;
        let glass_pt_color = glass_pt.xyz;
        if glass_pt_intensity > 0.01 {
            // Strong warm glow — backlit window effect, not multiplied by base glass color
            // Stronger at night when sun intensity is low
            let night_boost = 1.0 - camera.sun_intensity * 0.7;
            color = mix(color, glass_pt_color * 0.9, clamp(glass_pt_intensity * 1.5 * night_boost, 0.0, 0.85));
        }

        // Refraction distortion
        let refracted_wx = world_x + GLASS_REFRACT_OFFSET * sin(world_y * 12.0);
        let refracted_wy = world_y + GLASS_REFRACT_OFFSET * cos(world_x * 12.0);
        let beneath = get_block_f(refracted_wx, refracted_wy);
        let beneath_type = block_type(beneath);
        if beneath_type != 5u && block_height(beneath) < bheight {
            let beneath_col = block_base_color(beneath_type, block_flags(beneath));
            color = mix(color, beneath_col * GLASS_TINT, 0.25);
        }
    } else {
        // Normal block: apply shadow + additive point light
        let lit = color * (ambient + sun_color * light_factor * 0.85 * shadow_tint * tree_shadow * pleb_shadow);
        // Point light is additive (not multiplied by base color) so it illuminates
        // even when ambient/sun is very low (e.g. at night)
        let pl_mul = select(camera.light_bleed_mul, camera.indoor_glow_mul, is_indoor);
        color = lit + light_color_out * light_intensity_out * pl_mul;
    }

    // --- Per-pixel water rendering ---
    // water_level from the sim is DEPTH (water above terrain), not absolute height.
    var water_depth_here = water_level;
    if btype == BT_WATER {
        water_depth_here = max(water_depth_here, 0.5);
    }
    if btype == BT_DUG_GROUND && bheight >= 1u {
        water_depth_here = max(water_depth_here, f32(bheight) * 0.2);
    }

    let pixel_water_depth = water_depth_here;

    if pixel_water_depth > 0.005 {
        let t = camera.time;
        let depth = pixel_water_depth;
        let terrain_below = color; // save terrain color for transparency blending

        // --- Ripples: wind-aligned + depth-scaled ---
        let wind_dir = vec2(cos(camera.wind_angle), sin(camera.wind_angle));
        let wind_along = dot(vec2(world_x, world_y), wind_dir);
        // Large swells (wind-driven)
        let swell = sin(wind_along * 4.0 - t * 1.5) * 0.015 * min(depth * 3.0, 1.0);
        // Small chop (multi-directional)
        let chop1 = sin(world_x * 11.0 + world_y * 7.0 + t * 2.0) * 0.008;
        let chop2 = sin(world_x * 5.3 - world_y * 13.0 + t * 1.3) * 0.006;
        let chop3 = sin(world_x * 23.0 + world_y * 3.0 + t * 3.1) * 0.004;
        let ripple = swell + (chop1 + chop2 + chop3) * min(depth * 5.0, 1.0);

        // Surface normal from ripple (for specular/reflection)
        let nx = cos(world_x * 11.0 + t * 2.0) * 0.3 + cos(wind_along * 4.0 - t * 1.5) * 0.5;
        let ny = cos(world_y * 13.0 + t * 1.3) * 0.3;

        // --- Depth-dependent water color ---
        // Depth-dependent color: shallow = lighter cyan, deep = dark navy
        let shallow_col = vec3<f32>(0.20, 0.42, 0.56);
        let mid_col = vec3<f32>(0.10, 0.26, 0.44);
        let deep_col = vec3<f32>(0.04, 0.12, 0.28);
        let abyss_col = vec3<f32>(0.015, 0.04, 0.14);
        var water_col = mix(shallow_col, mid_col, smoothstep(0.02, 0.15, depth));
        water_col = mix(water_col, deep_col, smoothstep(0.15, 0.4, depth));
        water_col = mix(water_col, abyss_col, smoothstep(0.4, 1.0, depth));

        // --- Caustics (only visible in shallow water, modulated by sunlight) ---
        if depth < 0.4 {
            let caust_strength = (1.0 - depth / 0.4) * light_factor * 0.8;
            let c1 = abs(sin(world_x * 17.0 + t * 0.7) * sin(world_y * 19.0 + t * 0.9));
            let c2 = abs(sin(world_x * 11.0 - t * 0.5) * sin(world_y * 13.0 + t * 1.1));
            let caustic = c1 * c2;
            water_col += vec3(0.03, 0.08, 0.12) * caustic * caust_strength;
        }

        // --- Sky reflection (fresnel: more reflection at shallow viewing angle) ---
        let sky_col = vec3<f32>(0.45, 0.55, 0.75) * (camera.sun_intensity * 0.4 + 0.15);
        let fresnel = 0.1 + 0.15 * (1.0 + nx * 0.1); // simplified top-down fresnel
        water_col = mix(water_col, sky_col, clamp(fresnel + ripple * 1.5, 0.0, 0.35));

        // --- Specular highlight from sun ---
        let spec_base = nx * 0.5 + 0.5;
        let spec = pow(max(spec_base, 0.0), 32.0) * camera.sun_intensity * 0.4;
        water_col += vec3(spec * 0.8, spec * 0.9, spec);

        // --- Shore foam (where depth approaches zero) ---
        let foam_t = smoothstep(0.04, 0.0, depth);
        // Animated foam line: oscillates with time
        let foam_wave = sin(t * 1.5 + world_x * 3.0 + world_y * 5.0) * 0.01;
        let foam_depth = smoothstep(0.06 + foam_wave, 0.0, depth);
        let foam = max(foam_t, foam_depth * 0.7);
        water_col = mix(water_col, vec3(0.85, 0.88, 0.92), foam * 0.6);

        // --- Lighting: ambient + sun ---
        water_col = water_col * (ambient + sun_color * light_factor * 0.85);
        let water_pl_mul = select(camera.light_bleed_mul, camera.indoor_glow_mul, is_indoor);
        water_col += light_color_out * light_intensity_out * water_pl_mul * 0.5;

        // --- Transparency gradient: thin film → see-through → opaque ---
        let alpha = smoothstep(0.0, 0.35, depth); // wider gradient for gentler transition
        // Tint terrain below with blue (looking through water)
        let tinted_terrain = terrain_below * vec3(0.7, 0.8, 0.9);
        color = mix(tinted_terrain, water_col, clamp(alpha, 0.08, 0.97));
    } else if pixel_water_depth > -0.08 {
        // Wet terrain near waterline (moisture darkening)
        let moisture = smoothstep(-0.08, 0.0, pixel_water_depth);
        color *= 1.0 - moisture * 0.25;
    }

    // Update prelit color to include water (so torch/headlight illuminates water too)
    base_color_prelit = color;

    // Door detail: open doors show floor, closed doors show planks + handle
    if is_door(block) {
        if !is_open(block) {
            let plank = fract(fx * 3.0);
            let plank_edge = f32(plank < 0.08 || plank > 0.92) * 0.06;
            color -= vec3<f32>(plank_edge);
            let hx = fx - 0.75;
            let hy = fy - 0.5;
            if hx * hx + hy * hy < 0.008 {
                // Door handle: modulate by current scene lighting so it darkens at night
                let handle_base = vec3<f32>(0.7, 0.6, 0.3);
                let scene_light = ambient + sun_color * light_factor * shadow_tint;
                color = handle_base * (scene_light * 0.8 + 0.2);
            }
        } else {
            // Open door: show doorway floor with subtle threshold marks
            let threshold_color = vec3<f32>(0.42, 0.38, 0.32);
            let edge = f32(fx < 0.06 || fx > 0.94 || fy < 0.06 || fy > 0.94);
            color = mix(threshold_color, threshold_color * 0.8, edge);
        }
    }

    // Indoor tint when viewing through transparent roof
    if is_indoor {
        // Slightly desaturate and cool-shift to convey "indoors"
        color = mix(color, color * vec3<f32>(0.85, 0.88, 0.95), 0.3);
    }

    // --- Pleb rendering (all plebs from buffer) ---
    for (var pi: u32 = 0u; pi < MAX_PLEBS; pi++) {
        let p = plebs[pi];
        if p.x < 0.5 && p.y < 0.5 { continue; } // empty slot

        // Early distance cull: skip plebs beyond max light/body range
        // Headlight focus can reach ~40 tiles, so use generous cull distance
        let qdx = world_x - p.x;
        let qdy = world_y - p.y;
        let qdist_sq = qdx * qdx + qdy * qdy;
        // Use tight cull only when pleb has no headlight; otherwise allow up to 50 tiles
        let cull_sq = select(144.0, 2500.0, p.headlight > 0.5);
        if qdist_sq > cull_sq { continue; }

        let pdx = world_x - p.x;
        let pdy = world_y - p.y;
        let pdist = length(vec2(pdx, pdy));

        // Skip pleb lights on elevated block tops (not walls or sprite-rendered blocks)
        let is_sprite_block = btype == BT_TREE || btype == BT_BERRY_BUSH || btype == BT_CROP
            || btype == BT_ROCK || btype == BT_CRATE;
        let is_elevated = effective_height > 0u && !is_wall_face && !is_wd_wall && !is_sprite_block;

        // Per-pixel jitter for soft light edges (noise-based, varies by pixel + pleb)
        let jitter_seed = world_x * 127.1 + world_y * 311.7 + f32(pi) * 53.3 + camera.time * 7.1;
        let jx = (fract(sin(jitter_seed) * 43758.5) - 0.5) * 0.12;
        let jy = (fract(sin(jitter_seed * 1.37 + 2.1) * 27183.6) - 0.5) * 0.12;

        // Torch: warm point light — illuminates terrain base color, not additive overlay
        if p.torch > 0.5 && pdist < 12.0 && !is_elevated {
            let vis = trace_glow_visibility(world_x + jx, world_y + jy, p.x, p.y, 1.0);
            if vis > 0.01 {
                // Gentle inverse-distance falloff with soft outer fade
                let torch_atten = 1.0 / (1.0 + pdist * 0.25 + pdist * pdist * 0.03);
                let edge_fade = smoothstep(12.0, 5.0, pdist); // gradual fade from 5-12 tiles
                let flicker = sin(camera.time * 8.3 + f32(pi) * 2.0) * 0.15 + sin(camera.time * 13.1) * 0.1 + 0.85;
                let torch_tint = vec3(1.0, 0.55, 0.15);
                let torch_strength = torch_atten * edge_fade * 0.7 * flicker * vis;
                color += base_color_prelit * torch_tint * torch_strength;
            }
        }

        // Headlight: directional cone — mode 1=wide, 2=normal, 3=focused
        // No hard distance cutoff — attenuation alone handles falloff
        if p.headlight > 0.5 && pdist > 0.5 && pdist < 50.0 && !is_elevated {
            let mode = i32(p.headlight);
            // Beam parameters per mode
            var cone_inner = 0.0f;
            var cone_outer = 0.5f;
            var hl_intensity = 0.8f;
            var dist_linear = 0.08f;   // 1/(1 + linear*d + quad*d²)
            var dist_quad = 0.008f;
            if mode == 1 {
                // Wide: broad flood, shorter range, dimmer
                cone_inner = -0.3;
                cone_outer = 0.3;
                hl_intensity = 0.6;
                dist_linear = 0.12;
                dist_quad = 0.015;
            } else if mode == 3 {
                // Focused: tight pencil beam, very long range, bright
                cone_inner = 0.7;
                cone_outer = 0.92;
                hl_intensity = 1.6;
                dist_linear = 0.03;
                dist_quad = 0.002;
            } else {
                // Normal (mode 2): balanced
                cone_inner = 0.05;
                cone_outer = 0.55;
                hl_intensity = 0.8;
                dist_linear = 0.06;
                dist_quad = 0.006;
            }
            let vis = trace_glow_visibility(world_x + jx, world_y + jy, p.x, p.y, 1.5);
            if vis > 0.01 {
                let to_pixel = normalize(vec2(pdx, pdy));
                let light_dir = vec2(cos(p.angle), sin(p.angle));
                let cone_dot = dot(to_pixel, light_dir);
                let cone = smoothstep(cone_inner, cone_outer, cone_dot);
                let dist_atten = 1.0 / (1.0 + pdist * dist_linear + pdist * pdist * dist_quad);
                let headlight_tint = vec3(0.9, 0.92, 1.0);
                let headlight_strength = cone * dist_atten * hl_intensity * vis;
                color += base_color_prelit * headlight_tint * headlight_strength;
            }
        }

        // Body rendering — matches chargen preview exactly
        // s = scale factor (tile units). Proportions identical to UI preview.
        let s = 0.55 * camera.pleb_scale;
        var lx = pdx;
        var ly = -pdy; // flip Y

        let is_corpse = p.health <= 0.0;
        // Smooth crouch amount (0=standing, 1=fully crouched, 0-0.5 during peek)
        let crouch_t = p.crouch;

        let dir_x = cos(p.angle);
        let walk_phase = fract((p.x + p.y) * 2.0 + camera.time * 4.0);
        let bob = select(sin(walk_phase * 6.28) * 0.008, 0.0, is_corpse || crouch_t > 0.3);
        let head_dx = select(dir_x * 0.02 * s, 0.0, is_corpse);

        // Corpse: rotate body to lay flat in facing direction
        if is_corpse {
            let ca = cos(p.angle);
            let sa = sin(p.angle);
            lx = pdx * ca + pdy * sa;
            ly = (-pdx * sa + pdy * ca) * 1.3; // slight squash: body seen from above when lying
        }

        var drew_pleb = false;

        // Selection: bright pulsing ring at feet (green=normal, red=drafted)
        if p.selected > 0.5 {
            let ring_d = length(vec2(lx / (s * 0.42), ly / (s * 0.18)));
            let ring_w = 0.08;
            if ring_d > 1.0 - ring_w && ring_d < 1.0 + ring_w {
                let pulse = sin(camera.time * 4.0) * 0.25 + 0.75;
                // hair_style padding _pad2 is repurposed: >100 = drafted
                // (we'll encode this in the carrying field or a spare)
                color = mix(color, vec3(0.3, 0.95, 0.3), pulse);
                drew_pleb = true;
            }
        }

        // Direction chevron on ground (always visible, not just selected)
        {
            // Vector from pleb feet to pixel, in world space
            let dx = pdx;
            let dy = pdy;
            let fwd = vec2(cos(p.angle), sin(p.angle));
            let side = vec2(-fwd.y, fwd.x);

            // Project pixel onto forward/side axes
            let along = dx * fwd.x + dy * fwd.y;
            let across = dx * side.x + dy * side.y;

            // Chevron sits in front of the pleb at distance ~0.5*s from feet
            let chev_dist = s * 0.50;
            let chev_len = s * 0.15;
            let chev_w = s * 0.12;
            let t = along - chev_dist; // distance from chevron center along forward axis

            if t > -chev_len && t < chev_len && abs(across) < chev_w {
                // V shape: two angled lines meeting at the front
                let arm = chev_w * (1.0 - (t + chev_len) / (chev_len * 2.0));
                let thickness = s * 0.025;
                if abs(abs(across) - arm) < thickness {
                    let alpha = select(0.5, 0.8, p.selected > 0.5);
                    color = mix(color, vec3(0.85, 0.85, 0.80), alpha);
                    drew_pleb = true;
                }
            }
        }

        // Aiming cone: light transparent gray fill that narrows with aim_progress
        if p.aim_progress > 0.01 && !is_corpse {
            let fwd = vec2(cos(p.angle), sin(p.angle));
            let side_a = vec2(-fwd.y, fwd.x);
            let along = pdx * fwd.x + pdy * fwd.y;
            let across = pdx * side_a.x + pdy * side_a.y;

            let cone_len = 5.0;
            // Aim mode affects cone: snap=narrow start, precise=wide start but tighter end
            let aim_m = camera.aim_mode;
            let half_angle_start = select(select(0.35, 0.50, aim_m > 1.5), 0.25, aim_m < 0.5);
            let half_angle_end = select(select(0.04, 0.015, aim_m > 1.5), 0.08, aim_m < 0.5);
            let progress = clamp(p.aim_progress, 0.0, 1.0);
            let half_angle = mix(half_angle_start, half_angle_end, progress);

            if along > 0.2 && along < cone_len {
                let max_width = along * tan(half_angle);
                if abs(across) < max_width {
                    // Filled translucent cone — fades with distance and progress
                    let dist_fade = 1.0 - along / cone_len;
                    let edge_fade = 1.0 - abs(across) / max_width; // brighter at center
                    let alpha = dist_fade * edge_fade * mix(0.12, 0.04, progress);
                    color = mix(color, vec3(0.85, 0.85, 0.85), alpha);
                }
            }
        }

        // Shadow
        {
            let sd = (lx / (s * 0.35)) * (lx / (s * 0.35)) + (ly / (s * 0.12)) * (ly / (s * 0.12));
            if sd < 1.0 {
                color = mix(color, vec3(0.0), 0.22 * (1.0 - sd));
                drew_pleb = true;
            }
        }

        // Per-part crouch offsets: feet/pants compress up, shirt squashes, head drops
        // crouch_t: 0=standing, 1=fully crouched
        let ct = crouch_t;
        let feet_shift = ct * s * 0.05;      // feet shift up slightly
        let pants_shift = ct * s * 0.14;      // pants shift up
        let pants_squash = 1.0 - ct * 0.4;    // pants compress vertically
        let shirt_shift = ct * s * 0.22;      // shirt shifts up (body bends)
        let shirt_squash = 1.0 - ct * 0.35;   // shirt compresses
        let head_drop = ct * s * 0.35;        // head drops down toward body

        // Feet (dark pants)
        {
            let fy = ly - s * 0.07 + bob + feet_shift;
            let fd = (lx / (s * 0.12)) * (lx / (s * 0.12)) + (fy / (s * 0.06)) * (fy / (s * 0.06));
            if fd < 1.0 {
                color = vec3(p.pants_r, p.pants_g, p.pants_b) * 0.55;
                drew_pleb = true;
            }
        }

        // Pants (compressed when crouching)
        {
            let py_off = ly - s * 0.21 + bob + pants_shift;
            let py_h = s * 0.15 * pants_squash;
            let pd = (lx / (s * 0.22)) * (lx / (s * 0.22)) + (py_off / py_h) * (py_off / py_h);
            if pd < 1.0 {
                let shade = 0.80 + 0.20 * (1.0 - pd);
                color = vec3(p.pants_r, p.pants_g, p.pants_b) * shade;
                drew_pleb = true;
            }
        }

        // Shirt (compressed when crouching)
        {
            let sx_l = lx - head_dx * 0.3;
            let sy = ly - s * 0.43 + bob + shirt_shift;
            let sy_h = s * 0.20 * shirt_squash;
            let sd = (sx_l / (s * 0.26)) * (sx_l / (s * 0.26)) + (sy / sy_h) * (sy / sy_h);
            if sd < 1.0 {
                let shade = 0.80 + 0.20 * (1.0 - sd);
                color = vec3(p.shirt_r, p.shirt_g, p.shirt_b) * shade;
                drew_pleb = true;
            }
        }

        // Head (same size, drops down when crouching)
        {
            let hx = lx - head_dx;
            let hy = ly - s * 0.67 + bob + head_drop;
            let hd = length(vec2(hx, hy));
            let hr = s * 0.16;
            if hd < hr {
                let shade = 0.85 + 0.15 * (1.0 - hd / hr);
                color = vec3(p.skin_r, p.skin_g, p.skin_b) * shade;
                drew_pleb = true;
            }
        }

        // Hair (drops with head)
        {
            let hx = lx - head_dx * 1.5;
            let hy = ly - s * 0.77 + bob + head_drop;
            let hr = select(select(select(s * 0.04, s * 0.08, p.hair_style > 0.5), s * 0.10, p.hair_style > 1.5), s * 0.14, p.hair_style > 2.5);
            let hd = length(vec2(hx, hy * 1.4));
            if hd < hr {
                color = vec3(p.hair_r, p.hair_g, p.hair_b);
                drew_pleb = true;
            }
        }

        // Carried item: rendered at torso height, offset to the side
        if drew_pleb && p.carrying > 0.5 {
            let carry_id = u32(p.carrying - 0.5); // item_id (carrying encodes id+1)
            // Offset to the left side of the body (opposite weapon hand)
            let carry_ox = -s * 0.22 - head_dx * 0.2;
            let carry_oy = -s * 0.40 + bob + shirt_shift;
            let cx_c = lx + carry_ox;
            let cy_c = ly + carry_oy;
            let carry_r = s * 0.12;
            let cd = length(vec2(cx_c, cy_c));
            if cd < carry_r {
                // Item-appropriate color
                var item_col = vec3(0.30, 0.28, 0.26); // default: brown bundle
                if carry_id == 201u { // ITEM_ROCK
                    item_col = vec3(0.50, 0.48, 0.45); // gray rock
                } else if carry_id == 200u { // ITEM_WOOD
                    item_col = vec3(0.45, 0.32, 0.18); // brown wood
                } else if carry_id == 0u { // ITEM_BERRIES
                    item_col = vec3(0.30, 0.15, 0.35); // purple berries
                } else if carry_id == 202u { // ITEM_FIBER
                    item_col = vec3(0.30, 0.42, 0.20); // green fiber
                } else if carry_id == 1u { // ITEM_RAW_MEAT
                    item_col = vec3(0.55, 0.22, 0.18); // red meat
                } else if carry_id == 2u { // ITEM_RAW_FISH
                    item_col = vec3(0.40, 0.50, 0.55); // silvery fish
                }
                // Shading: darker at edges
                let shade = 1.0 - cd / carry_r * 0.3;
                color = item_col * shade;
                // Dark outline ring
                if cd > carry_r * 0.8 {
                    color *= 0.6;
                }
            }
        }

        // Weapon rendering: anchored to right side at torso height
        if p.weapon_type > 0.5 && !is_corpse && !drew_pleb {
            let wpn_ox = s * 0.20 + head_dx * 0.3;
            let wpn_oy = -s * 0.48 + bob + shirt_shift; // shifts with torso when crouching

            let is_pistol = p.weapon_type > 3.5; // type 4 = pistol

            // Melee: swing arc animation. Ranged: steady aim along facing.
            let wpn_angle = select(
                mix(-0.8, 0.8, p.swing_progress) + p.angle,  // melee swing
                p.angle,                                       // ranged aim
                is_pistol
            );
            let wc = cos(wpn_angle);
            let ws = sin(wpn_angle);
            let wlx = (lx - wpn_ox) * wc + (ly - wpn_oy) * ws;
            let wly = -(lx - wpn_ox) * ws + (ly - wpn_oy) * wc;

            if is_pistol {
                // Pistol: short barrel + grip
                let barrel_len = s * 0.22;
                let barrel_w = s * 0.02;
                let grip_len = s * 0.08;
                let grip_w = s * 0.025;

                // Barrel
                if wlx > 0.0 && wlx < barrel_len && abs(wly) < barrel_w {
                    color = vec3(0.25, 0.22, 0.20); // dark metal
                    drew_pleb = true;
                }
                // Grip (above barrel in screen space — negative wly)
                if wlx > -grip_len * 0.3 && wlx < grip_len * 0.7
                    && wly < -barrel_w && wly > -barrel_w - grip_len {
                    color = vec3(0.35, 0.25, 0.15); // wood grip
                    drew_pleb = true;
                }
                // Muzzle flash (when aim_progress near 1.0)
                if p.aim_progress > 0.9 {
                    let flash_d = length(vec2(wlx - barrel_len, wly));
                    if flash_d < s * 0.06 {
                        let flash_bright = (p.aim_progress - 0.9) * 10.0;
                        color = mix(vec3(1.0, 0.8, 0.3), vec3(1.0, 1.0, 0.9), flash_bright);
                        drew_pleb = true;
                    }
                }
            } else {
                // Melee weapons: handle + head
                let handle_len = s * 0.35;
                let handle_w = s * 0.025;
                let head_len = s * 0.10;
                let head_w = select(s * 0.05, s * 0.07, p.weapon_type > 2.5);

                if wlx > 0.0 && wlx < handle_len && abs(wly) < handle_w {
                    color = vec3(0.35, 0.25, 0.15);
                    drew_pleb = true;
                }
                if wlx > handle_len - head_len * 0.2 && wlx < handle_len + head_len
                    && abs(wly) < head_w {
                    if p.weapon_type < 1.5 {
                        color = vec3(0.45, 0.42, 0.40); // stone axe
                    } else if p.weapon_type < 2.5 {
                        color = vec3(0.50, 0.48, 0.45); // stone pick
                    } else {
                        color = vec3(0.30, 0.25, 0.18); // wooden shovel
                    }
                    drew_pleb = true;
                }
            }
        }

        // Black outline: check if pixel is just outside any body zone
        if !drew_pleb {
            // Scale outline with zoom: thicker when zoomed out so plebs stay visible
            let pixels_per_tile = camera.zoom / camera.screen_w * camera.grid_w;
            let outline_w = 0.025 + 0.04 / max(pixels_per_tile, 1.0);
            // Check expanded versions of main body zones
            let o_feet = (lx / (s * 0.12 + outline_w)) * (lx / (s * 0.12 + outline_w))
                + ((ly - s * 0.07 + bob + feet_shift) / (s * 0.06 + outline_w)) * ((ly - s * 0.07 + bob + feet_shift) / (s * 0.06 + outline_w));
            let o_py = ly - s * 0.21 + bob + pants_shift;
            let o_ph = s * 0.15 * pants_squash + outline_w;
            let o_pants = (lx / (s * 0.22 + outline_w)) * (lx / (s * 0.22 + outline_w))
                + (o_py / o_ph) * (o_py / o_ph);
            let o_sy = ly - s * 0.43 + bob + shirt_shift;
            let o_sh = s * 0.20 * shirt_squash + outline_w;
            let o_shirt = ((lx - head_dx * 0.3) / (s * 0.26 + outline_w)) * ((lx - head_dx * 0.3) / (s * 0.26 + outline_w))
                + (o_sy / o_sh) * (o_sy / o_sh);
            let hx_o = lx - head_dx;
            let hy_o = ly - s * 0.67 + bob + head_drop;
            let o_head = length(vec2(hx_o, hy_o)) / (s * 0.16 + outline_w);
            if o_feet < 1.0 || o_pants < 1.0 || o_shirt < 1.0 || o_head < 1.0 {
                color = vec3(0.02, 0.02, 0.02);
                drew_pleb = true;
            }
        }

        // Corpse
        if drew_pleb && p.health <= 0.0 {
            let gray = dot(color, vec3(0.3, 0.5, 0.2));
            color = mix(vec3(gray * 0.5), vec3(0.25, 0.18, 0.15), 0.3);
            let xc_x = lx + s * 0.2;
            let xc_y = ly - s * 0.3;
            let x1 = abs(xc_x - xc_y);
            let x2 = abs(xc_x + xc_y);
            if (x1 < s * 0.05 || x2 < s * 0.05) && abs(xc_x) < s * 0.15 && abs(xc_y) < s * 0.15 {
                color = vec3(0.85, 0.15, 0.10);
            }
        }

        // Health bar (below feet)
        if p.health > 0.0 && p.health < 0.95 {
            let bar_cx = lx;
            let bar_cy = ly + s * 0.10;
            let bar_hw = s * 0.28;  // half-width
            let bar_hh = 0.045;     // half-height
            let outline = 0.015;
            // Dark outline
            if abs(bar_cy) < bar_hh + outline && abs(bar_cx) < bar_hw + outline {
                color = vec3(0.05, 0.05, 0.05);
                drew_pleb = true;
            }
            // Bar fill
            if abs(bar_cy) < bar_hh && abs(bar_cx) < bar_hw {
                let bar_t = (bar_cx + bar_hw) / (bar_hw * 2.0);
                if bar_t < p.health {
                    var bar_col = vec3(0.2, 0.8, 0.2);
                    if p.health < 0.5 { bar_col = mix(vec3(0.9, 0.2, 0.1), vec3(0.9, 0.8, 0.1), p.health * 2.0); }
                    else { bar_col = mix(vec3(0.9, 0.8, 0.1), vec3(0.2, 0.8, 0.2), (p.health - 0.5) * 2.0); }
                    color = bar_col;
                } else {
                    color = vec3(0.15, 0.12, 0.10);
                }
            }
        }

        // Stress bar (below health bar, visible when stress > 0.25)
        if p.health > 0.0 && p.stress > 0.25 {
            let sbar_cx = lx;
            let sbar_cy = ly + s * 0.10 + 0.12; // below health bar
            let sbar_hw = s * 0.22; // slightly narrower than health bar
            let sbar_hh = 0.025;    // thinner than health bar
            let sbar_outline = 0.01;
            // Dark outline
            if abs(sbar_cy) < sbar_hh + sbar_outline && abs(sbar_cx) < sbar_hw + sbar_outline {
                color = vec3(0.04, 0.04, 0.04);
                drew_pleb = true;
            }
            // Bar fill: blue (calm) → yellow (stressed) → red (breaking)
            if abs(sbar_cy) < sbar_hh && abs(sbar_cx) < sbar_hw {
                let sbar_t = (sbar_cx + sbar_hw) / (sbar_hw * 2.0);
                if sbar_t < p.stress {
                    var scol = vec3(0.3, 0.5, 0.8); // blue-ish calm
                    if p.stress > 0.7 {
                        scol = mix(vec3(0.85, 0.6, 0.1), vec3(0.9, 0.2, 0.1), (p.stress - 0.7) / 0.3);
                    } else if p.stress > 0.4 {
                        scol = mix(vec3(0.3, 0.5, 0.8), vec3(0.85, 0.6, 0.1), (p.stress - 0.4) / 0.3);
                    }
                    // Pulse when near breaking (>0.85)
                    if p.stress > 0.85 {
                        let pulse = sin(camera.time * 6.0) * 0.3 + 0.7;
                        scol *= pulse;
                    }
                    color = scol;
                } else {
                    color = vec3(0.08, 0.08, 0.10);
                }
            }
        }
    }

    // --- Creature rendering (alien fauna) ---
    for (var ci: u32 = 0u; ci < MAX_CREATURES; ci++) {
        let cr = creature_buf[ci];
        if cr.body_radius < 0.01 { continue; }
        // Hop offset shifts body "upward" on screen (negative Y in world = up on screen)
        let cr_y = cr.y - cr.hop_offset;
        // Early distance cull
        let cdx = world_x - cr.x;
        let cdy = world_y - cr_y;
        if cdx * cdx + cdy * cdy > 4.0 { continue; } // >2 tiles away
        let cdist = length(vec2(cdx, cdy));

        // Hover outline: soft ring when cursor is near this creature
        let hover_dx = camera.hover_x - cr.x;
        let hover_dy = camera.hover_y - cr_y;
        let hover_near = camera.hover_x >= 0.0
            && hover_dx * hover_dx + hover_dy * hover_dy < cr.body_radius * cr.body_radius * 4.0;
        if hover_near && cr.health > 0.0 {
            let outline_inner = cr.body_radius;
            let outline_outer = cr.body_radius + 0.04;
            if cdist > outline_inner && cdist < outline_outer {
                let t = 1.0 - (cdist - outline_inner) / (outline_outer - outline_inner);
                let outline_col = vec3(
                    min(cr.color_r + 0.4, 1.0),
                    min(cr.color_g + 0.4, 1.0),
                    min(cr.color_b + 0.4, 1.0),
                );
                color = mix(color, outline_col, t * 0.7);
            }
        }

        if cdist < cr.body_radius {
            // Body: shaded circle with subtle rim lighting
            let shade = 1.0 - cdist / cr.body_radius * 0.4;
            var cr_color = vec3(cr.color_r, cr.color_g, cr.color_b) * shade;
            // Hover brighten
            if hover_near && cr.health > 0.0 {
                cr_color = cr_color * 1.15;
            }
            color = cr_color;
            // Shadow blob on ground when hopping (darker spot below)
            if cr.hop_offset > 0.01 {
                let shadow_dy = world_y - cr.y; // distance from ground position
                let shadow_dx = world_x - cr.x;
                let shadow_d = length(vec2(shadow_dx, shadow_dy));
                if shadow_d < cr.body_radius * 0.6 {
                    color *= 0.7; // darken for shadow
                }
            }
            // Eye dots: two small bright spots toward facing direction
            let eye_spread = cr.body_radius * 0.35;
            let eye_fwd = cr.body_radius * 0.3;
            let eye_sz = cr.body_radius * 0.12;
            let fwd = vec2(cos(cr.angle), sin(cr.angle));
            let side = vec2(-fwd.y, fwd.x);
            let eye1 = vec2(cr.x, cr_y) + fwd * eye_fwd + side * eye_spread;
            let eye2 = vec2(cr.x, cr_y) + fwd * eye_fwd - side * eye_spread;
            let ed1 = length(vec2(world_x, world_y) - eye1);
            let ed2 = length(vec2(world_x, world_y) - eye2);
            if ed1 < eye_sz || ed2 < eye_sz {
                color = vec3(cr.eye_r, cr.eye_g, cr.eye_b);
            }
            // Corpse: red X overlay
            if cr.health <= 0.0 {
                let ax = abs(cdx) - abs(cdy);
                if abs(ax) < cr.body_radius * 0.15 {
                    color = mix(color, vec3(0.6, 0.1, 0.1), 0.7);
                }
            }
        }
    }

    // Per-pixel proximity glow from nearby light sources.
    // Applies to all visible surfaces EXCEPT structural wall tops (which would bleed onto roof).
    // Equipment, furniture, batteries, pipes etc. inside buildings all receive light.
    let is_furniture = get_material(btype).is_furniture > 0.5 && !(btype == BT_TABLE_LAMP && is_table_lamp_bulb);
    let is_structural_wall = is_roofed && camera.show_roofs < 0.5
        && matches_wall_type(btype) && bheight > 0u;
    let receives_glow = is_indoor || is_furniture
        || (is_roofed && camera.show_roofs < 0.5 && !is_structural_wall) // indoor equipment/pipes
        || (!is_roofed && effective_height == 0u); // outdoor ground
    if camera.enable_prox_glow > 0.5 && receives_glow {
        // Conditional glow: skip expensive 13x13 scan if lightmap shows no nearby light
        let lm_gate = sample_lightmap(world_x, world_y);
        if lm_gate.w > 0.02 {
            let prox_glow = compute_proximity_glow(world_x, world_y, camera.time);
            let night_boost = 1.0 - camera.sun_intensity * 0.7;
            let glow_mul = camera.indoor_glow_mul * (0.5 + night_boost);
            // Multiplicative illumination: surfaces get brighter proportionally,
            // preserving texture detail. Small additive term ensures even very
            // dark surfaces show some light (simulates subsurface/bounce).
            color = color * (vec3(1.0) + prox_glow * glow_mul * 3.0)
                  + prox_glow * glow_mul * 0.08;
        }
    }

    // Outdoor light glow: warm light spilling through windows/doors onto ground
    if !is_indoor && light_intensity_out > 0.01 {
        // Stronger at night — warm pools of light on the ground outside windows
        let night_boost = 1.0 - camera.sun_intensity * 0.8;
        color += light_color_out * light_intensity_out * camera.light_bleed_mul * night_boost;
    }

    // Two-zone border:
    // Buffer zone (0-10 tiles from edge): completely invisible, solid fog
    // Gray zone (10-25 tiles from edge): terrain + gas fade out gradually
    let border_dist = min(
        min(world_x, camera.grid_w - world_x),
        min(world_y, camera.grid_h - world_y)
    );
    let border_fade = clamp((border_dist - 10.0) / 15.0, 0.0, 1.0);  // 0 at edge, 1 at 25+ tiles in

    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));

    // Fluid sim overlay
    let fluid_uv = vec2<f32>(world_x / camera.grid_w, world_y / camera.grid_h);
    let smoke = textureSampleLevel(fluid_dye_tex, fluid_dye_sampler, fluid_uv, 0.0);

    if camera.fluid_overlay < 0.5 {
        // Normal mode: smoke (R), O2 depletion, CO2 effects
        let smoke_density = clamp(smoke.r * 0.4, 0.0, 0.85);
        // Toxic gas: green tint when both smoke and CO2 are high
        let toxic = clamp(min(smoke.r, smoke.b * 1.5) * 0.8, 0.0, 1.0);
        let smoke_color = mix(vec3(0.75, 0.73, 0.72), vec3(0.15, 1.0, 0.05), toxic);
        color = mix(color, smoke_color, smoke_density);

        // O2 depletion: darkening + slight blue tint
        let o2_deficit = clamp(1.0 - smoke.g, 0.0, 1.0);
        color *= 1.0 - o2_deficit * 0.3;
        color = mix(color, vec3(0.05, 0.05, 0.15), o2_deficit * 0.2);

        // CO2: slight darkening
        let co2 = clamp(smoke.b, 0.0, 1.0);
        color *= 1.0 - co2 * 0.15;

        // Temperature effects: heat shimmer near fire, cold blue tint
        let air_t = smoke.a;
        if air_t > 80.0 {
            // Heat shimmer: warm orange glow in very hot air
            let heat = clamp((air_t - 80.0) / 300.0, 0.0, 0.5);
            color = mix(color, vec3(1.0, 0.6, 0.2), heat * 0.3);
        } else if air_t < 0.0 {
            // Cold: blue tint
            let cold = clamp(-air_t / 20.0, 0.0, 0.4);
            color = mix(color, vec3(0.5, 0.6, 0.9), cold * 0.2);
        }
        // Subtle sound ripple (always-on when sound is active) — bilinear interpolated
        {
            let sfx = world_x * 2.0;
            let sfy = world_y * 2.0;
            let sx0 = i32(floor(sfx));
            let sy0 = i32(floor(sfy));
            let sfrac_x = sfx - floor(sfx);
            let sfrac_y = sfy - floor(sfy);
            let smax = vec2<i32>(i32(camera.grid_w) * 2 - 1, i32(camera.grid_h) * 2 - 1);
            let s00 = textureLoad(sound_tex, clamp(vec2(sx0, sy0), vec2(0), smax), 0).r;
            let s10 = textureLoad(sound_tex, clamp(vec2(sx0 + 1, sy0), vec2(0), smax), 0).r;
            let s01 = textureLoad(sound_tex, clamp(vec2(sx0, sy0 + 1), vec2(0), smax), 0).r;
            let s11 = textureLoad(sound_tex, clamp(vec2(sx0 + 1, sy0 + 1), vec2(0), smax), 0).r;
            let sp = mix(mix(s00, s10, sfrac_x), mix(s01, s11, sfrac_x), sfrac_y);
            let ripple = clamp(sp * 0.06, -0.04, 0.04);
            color += vec3(ripple * 0.5, ripple * 0.3, ripple * 0.8);
        }
    } else if camera.fluid_overlay < 1.5 {
        // Gases: all gases with distinct colors, composited together
        let bg = color * 0.25;
        var gas_color = vec3(0.0);
        var gas_alpha = 0.0;
        // Smoke: white
        let s = clamp(smoke.r * 0.5, 0.0, 1.0);
        gas_color += vec3(0.9, 0.9, 0.92) * s;
        gas_alpha = max(gas_alpha, s);
        // O2 deficit: blue (shows where O2 is low)
        let o2d = clamp((1.0 - smoke.g) * 2.0, 0.0, 1.0);
        gas_color += vec3(0.2, 0.4, 1.0) * o2d;
        gas_alpha = max(gas_alpha, o2d);
        // CO2: yellow-green
        let co2 = clamp(smoke.b * 2.0, 0.0, 1.0);
        gas_color += vec3(0.7, 0.8, 0.1) * co2;
        gas_alpha = max(gas_alpha, co2);
        // Normalize and blend
        let total = s + o2d + co2 + 0.001;
        gas_color /= total;
        color = mix(bg, gas_color, clamp(gas_alpha, 0.0, 0.9));
    } else if camera.fluid_overlay < 2.5 {
        // Smoke overlay: R channel density as heat map
        let density = clamp(smoke.r, 0.0, 1.0);
        let heat = vec3(
            clamp(density * 3.0, 0.0, 1.0),
            clamp(density * 3.0 - 1.0, 0.0, 1.0),
            clamp(density * 3.0 - 2.0, 0.0, 1.0)
        );
        color = mix(color * 0.3, heat, clamp(density * 2.0, 0.0, 0.9));
    } else if camera.fluid_overlay < 3.5 {
        // Velocity: magnitude as brightness with subtle directional tint, per-block arrows
        let sim_pos = vec2<i32>(vec2<f32>(world_x, world_y));
        let sim_clamped = clamp(sim_pos, vec2(0), vec2(i32(camera.grid_w) - 1, i32(camera.grid_h) - 1));
        let vel = textureLoad(fluid_vel_tex, sim_clamped, 0).xy;
        let mag = length(vel);
        // Logarithmic scaling so small velocities are still visible
        let norm_mag = clamp(log(1.0 + mag * 0.5) * 0.5, 0.0, 1.0);
        // Calm blue base → brighter cyan/white at high velocity
        let vel_color = mix(
            vec3<f32>(0.1, 0.15, 0.3),    // calm: dark blue
            vec3<f32>(0.4, 0.85, 1.0),    // fast: bright cyan
            norm_mag
        );
        color = mix(color * 0.3, vel_color, clamp(norm_mag + 0.15, 0.0, 0.8));

        // Procedural arrow per block (no extra texture reads)
        if mag > 0.3 {
            let fx = fract(world_x) - 0.5;  // -0.5 to 0.5 within block
            let fy = fract(world_y) - 0.5;
            let dir = vel / mag;
            // Project pixel offset onto arrow direction and perpendicular
            let along = fx * dir.x + fy * dir.y;     // distance along arrow
            let perp = abs(-fx * dir.y + fy * dir.x); // distance from arrow axis
            let arrow_len = clamp(mag * 0.02, 0.1, 0.4);
            // Shaft: thin line from center toward tip
            let on_shaft = along > -0.05 && along < arrow_len && perp < 0.06;
            // Arrowhead: wider near the tip
            let head_t = (along - arrow_len + 0.12) / 0.12; // 0 at start of head, 1 at tip
            let on_head = head_t > 0.0 && head_t < 1.0 && perp < 0.15 * (1.0 - head_t);
            if on_shaft || on_head {
                color = mix(color, vec3(1.0), 0.8);
            }
        }
    } else if camera.fluid_overlay < 4.5 {
        // Pressure: signed divergence-free pressure field
        // Blue = negative (suction), white = zero, red = positive (push)
        // Bilinear interpolation for smooth display
        let fp = vec2<f32>(world_x, world_y) - 0.5;
        let f = fract(fp);
        let base = vec2<i32>(floor(fp));
        let maxc = vec2<i32>(i32(camera.grid_w) - 1, i32(camera.grid_h) - 1);
        let p00 = textureLoad(fluid_pres_tex, clamp(base, vec2(0), maxc), 0).r;
        let p10 = textureLoad(fluid_pres_tex, clamp(base + vec2(1, 0), vec2(0), maxc), 0).r;
        let p01 = textureLoad(fluid_pres_tex, clamp(base + vec2(0, 1), vec2(0), maxc), 0).r;
        let p11 = textureLoad(fluid_pres_tex, clamp(base + vec2(1, 1), vec2(0), maxc), 0).r;
        let p = mix(mix(p00, p10, f.x), mix(p01, p11, f.x), f.y);

        // Signed pressure: -1..+1 range with soft scaling
        let signed_p = clamp(p * 0.05, -1.0, 1.0);
        // Blue (negative) → gray (zero) → red (positive)
        var pcolor: vec3<f32>;
        if signed_p < 0.0 {
            let t = -signed_p; // 0..1
            pcolor = mix(vec3<f32>(0.5, 0.5, 0.55), vec3<f32>(0.15, 0.25, 0.8), t);
        } else {
            let t = signed_p;  // 0..1
            pcolor = mix(vec3<f32>(0.5, 0.5, 0.55), vec3<f32>(0.85, 0.2, 0.15), t);
        }

        let strength = clamp(abs(signed_p) * 1.5 + 0.2, 0.0, 0.8);
        color = mix(color * 0.3, pcolor, strength);
    } else if camera.fluid_overlay < 5.5 {
        // O2: blue (high/atmospheric) to red (depleted)
        let o2 = clamp(smoke.g, 0.0, 1.0);
        let o2_color = mix(vec3(0.9, 0.1, 0.0), vec3(0.1, 0.4, 1.0), o2);
        color = mix(color * 0.3, o2_color, 0.7);
    } else if camera.fluid_overlay < 6.5 {
        // CO2: dark (none) to yellow-green (high)
        let co2 = clamp(smoke.b * 3.0, 0.0, 1.0);
        let co2_color = mix(vec3(0.05, 0.1, 0.05), vec3(0.85, 0.9, 0.2), co2);
        color = mix(color * 0.3, co2_color, clamp(co2 + 0.1, 0.0, 0.8));
    } else if camera.fluid_overlay < 7.5 {
        // Temperature overlay — expanded palette focused on useful range
        // For solid blocks (walls), read block temperature from block_temps buffer
        // For air/ground, read from dye texture
        let grid_idx = u32(by) * u32(camera.grid_w) + u32(bx);
        let bt_for_temp = btype;
        let bh_for_temp = bheight;
        let is_solid_block = bh_for_temp > 0u && (bt_for_temp == 1u || bt_for_temp == 4u || bt_for_temp == 5u
            || bt_for_temp == 14u || (bt_for_temp >= 21u && bt_for_temp <= 25u) || bt_for_temp == 35u);
        let is_pipe_block_t = (bt_for_temp >= 15u && bt_for_temp <= 20u) || bt_for_temp == 46u || bt_for_temp == 49u || bt_for_temp == 50u || bt_for_temp == 52u || bt_for_temp == 53u || bt_for_temp == 54u;
        let temp = select(smoke.a, block_temps[grid_idx], is_solid_block || is_pipe_block_t);
        // Remap: -30→0, 0→0.15, 15→0.30, 25→0.40, 40→0.55, 60→0.70, 100→0.85, 500→1.0
        var temp_norm: f32;
        if temp < -30.0 {
            temp_norm = 0.0;
        } else if temp < 0.0 {
            temp_norm = (temp + 30.0) / 30.0 * 0.15;           // -30..0 → 0..0.15
        } else if temp < 15.0 {
            temp_norm = 0.15 + temp / 15.0 * 0.15;             // 0..15 → 0.15..0.30
        } else if temp < 25.0 {
            temp_norm = 0.30 + (temp - 15.0) / 10.0 * 0.10;   // 15..25 → 0.30..0.40
        } else if temp < 40.0 {
            temp_norm = 0.40 + (temp - 25.0) / 15.0 * 0.15;   // 25..40 → 0.40..0.55
        } else if temp < 60.0 {
            temp_norm = 0.55 + (temp - 40.0) / 20.0 * 0.15;   // 40..60 → 0.55..0.70
        } else if temp < 100.0 {
            temp_norm = 0.70 + (temp - 60.0) / 40.0 * 0.15;   // 60..100 → 0.70..0.85
        } else {
            temp_norm = 0.85 + clamp((temp - 100.0) / 400.0, 0.0, 0.15); // 100..500 → 0.85..1.0
        }
        temp_norm = clamp(temp_norm, 0.0, 1.0);
        var temp_color: vec3<f32>;
        if temp_norm < 0.075 {
            // Deep freeze: dark blue-purple (-30 to -15°C)
            let t = temp_norm / 0.075;
            temp_color = mix(vec3(0.15, 0.0, 0.4), vec3(0.0, 0.1, 0.7), t);
        } else if temp_norm < 0.15 {
            // Freezing: blue (-15 to 0°C)
            let t = (temp_norm - 0.075) / 0.075;
            temp_color = mix(vec3(0.0, 0.1, 0.7), vec3(0.2, 0.4, 0.9), t);
        } else if temp_norm < 0.225 {
            // Cold: light blue (0 to ~8°C)
            let t = (temp_norm - 0.15) / 0.075;
            temp_color = mix(vec3(0.2, 0.4, 0.9), vec3(0.5, 0.7, 1.0), t);
        } else if temp_norm < 0.30 {
            // Cool: cyan-white (8 to 15°C)
            let t = (temp_norm - 0.225) / 0.075;
            temp_color = mix(vec3(0.5, 0.7, 1.0), vec3(0.7, 0.85, 0.7), t);
        } else if temp_norm < 0.40 {
            // Comfortable: green-white (15 to 25°C)
            let t = (temp_norm - 0.30) / 0.10;
            temp_color = mix(vec3(0.7, 0.85, 0.7), vec3(0.9, 0.92, 0.8), t);
        } else if temp_norm < 0.55 {
            // Warm: cream to yellow (25 to 40°C)
            let t = (temp_norm - 0.40) / 0.15;
            temp_color = mix(vec3(0.9, 0.92, 0.8), vec3(1.0, 0.85, 0.3), t);
        } else if temp_norm < 0.70 {
            // Hot: orange (40 to 60°C)
            let t = (temp_norm - 0.55) / 0.15;
            temp_color = mix(vec3(1.0, 0.85, 0.3), vec3(1.0, 0.45, 0.1), t);
        } else if temp_norm < 0.82 {
            // Very hot: deep orange-red (60 to 90°C)
            let t = (temp_norm - 0.70) / 0.12;
            temp_color = mix(vec3(1.0, 0.45, 0.1), vec3(0.9, 0.12, 0.05), t);
        } else if temp_norm < 0.89 {
            // Dangerous: cherry red to magenta (90 to 200°C)
            let t = (temp_norm - 0.82) / 0.07;
            temp_color = mix(vec3(0.9, 0.12, 0.05), vec3(0.85, 0.1, 0.55), t);
        } else if temp_norm < 0.95 {
            // Searing: magenta to violet (200 to 400°C)
            let t = (temp_norm - 0.89) / 0.06;
            temp_color = mix(vec3(0.85, 0.1, 0.55), vec3(0.6, 0.15, 0.85), t);
        } else {
            // Extreme: bright violet-blue (400°C+ — plasma/lightning)
            let t = (temp_norm - 0.95) / 0.05;
            temp_color = mix(vec3(0.6, 0.15, 0.85), vec3(0.5, 0.3, 1.0), t);
        }
        color = mix(color * 0.3, temp_color, 0.7);
    } else if camera.fluid_overlay < 9.5 {
        // Power overlay (9): show voltage, highlight infrastructure, dim terrain
        let grid_idx_p = u32(by) * u32(camera.grid_w) + u32(bx);
        let v = voltage[grid_idx_p];
        let norm_v = clamp(v / 12.0, 0.0, 1.0);
        // Is this block part of the power grid?
        let is_pwr_infra = btype == BT_WIRE || btype == BT_SOLAR || btype == BT_BATTERY_S || btype == BT_BATTERY_M
            || btype == BT_BATTERY_L || btype == BT_WIND_TURBINE || btype == BT_SWITCH || btype == BT_DIMMER || btype == BT_BREAKER || btype == BT_FLOODLIGHT || btype == BT_WIRE_BRIDGE
            || btype == BT_CEILING_LIGHT || btype == BT_FLOOR_LAMP || btype == BT_TABLE_LAMP || btype == BT_FAN || btype == BT_PUMP || btype == BT_WALL_LAMP
            || (bflags & 0x80u) != 0u; // wire overlay on wall
        if is_pwr_infra {
            if norm_v > 0.01 {
                var pwr_color: vec3<f32>;
                if norm_v < 0.5 {
                    pwr_color = mix(vec3<f32>(0.1, 0.3, 0.1), vec3<f32>(0.2, 0.8, 0.2), norm_v / 0.5);
                } else if norm_v < 0.8 {
                    pwr_color = mix(vec3<f32>(0.2, 0.8, 0.2), vec3<f32>(0.9, 0.8, 0.1), (norm_v - 0.5) / 0.3);
                } else {
                    pwr_color = mix(vec3<f32>(0.9, 0.8, 0.1), vec3<f32>(1.0, 0.2, 0.1), (norm_v - 0.8) / 0.2);
                }
                color = mix(color * 0.4, pwr_color, 0.7);
            } else {
                // Unpowered infrastructure: show as dim outline
                color = mix(color * 0.3, vec3<f32>(0.3, 0.3, 0.35), 0.5);
            }
        } else {
            // Non-power terrain: dim heavily
            color *= 0.25;
        }
    } else if camera.fluid_overlay < 10.5 {
        // Amps overlay: show current flow (voltage differences with neighbors)
        let gw_a = u32(camera.grid_w);
        let aidx = u32(by) * gw_a + u32(bx);
        let av = voltage[aidx];
        var total_current = 0.0;
        for (var ad = 0; ad < 4; ad++) {
            var adx = 0; var ady = 0;
            if ad == 0 { adx = 1; } else if ad == 1 { adx = -1; }
            else if ad == 2 { ady = 1; } else { ady = -1; }
            let anx = i32(bx) + adx;
            let any = i32(by) + ady;
            if anx >= 0 && any >= 0 && anx < i32(camera.grid_w) && any < i32(camera.grid_h) {
                let anidx = u32(any) * gw_a + u32(anx);
                total_current += abs(voltage[anidx] - av);
            }
        }
        let norm_a = clamp(total_current * 0.5, 0.0, 1.0);
        let is_amp_infra = btype == BT_WIRE || btype == BT_SOLAR || btype == BT_BATTERY_S || btype == BT_BATTERY_M
            || btype == BT_BATTERY_L || btype == BT_WIND_TURBINE || btype == BT_SWITCH || btype == BT_DIMMER || btype == BT_FLOODLIGHT || btype == BT_WIRE_BRIDGE
            || btype == BT_CEILING_LIGHT || btype == BT_FLOOR_LAMP || btype == BT_TABLE_LAMP || btype == BT_FAN || btype == BT_PUMP || btype == BT_WALL_LAMP
            || (bflags & 0x80u) != 0u;
        if norm_a > 0.005 {
            var amp_color = mix(vec3<f32>(0.05, 0.05, 0.15), vec3<f32>(0.4, 0.8, 1.0), clamp(norm_a * 2.0, 0.0, 1.0));
            if norm_a > 0.5 {
                amp_color = mix(amp_color, vec3<f32>(1.0, 1.0, 0.9), (norm_a - 0.5) * 2.0);
            }
            color = mix(color * 0.3, amp_color, 0.7);
        } else if is_amp_infra {
            color = mix(color * 0.3, vec3<f32>(0.3, 0.3, 0.35), 0.5);
        } else {
            color *= 0.25;
        }
    } else if camera.fluid_overlay < 11.5 {
        // Watts overlay (11): generation (green) vs consumption (red)
        let gw_w = u32(camera.grid_w);
        let widx = u32(by) * gw_w + u32(bx);
        let wbt = btype;
        let wv = voltage[widx];
        // Determine if generator, consumer, or passive
        let is_gen = wbt == 37u || wbt == 41u; // solar, wind
        let is_con = wbt == 7u || wbt == 10u || wbt == 11u || wbt == 12u || wbt == 16u;
        let is_bat = wbt == 38u || wbt == 39u || wbt == 40u;
        if wv > 0.01 {
            var watt_color: vec3<f32>;
            if is_gen {
                // Green glow for generators (brighter = more output)
                let gen_pwr = clamp(wv / 12.0, 0.0, 1.0);
                watt_color = mix(vec3<f32>(0.1, 0.3, 0.1), vec3<f32>(0.2, 0.9, 0.2), gen_pwr);
            } else if is_con {
                // Red/orange for consumers
                let con_pwr = clamp(wv / 12.0, 0.0, 1.0);
                watt_color = mix(vec3<f32>(0.3, 0.1, 0.05), vec3<f32>(1.0, 0.4, 0.1), con_pwr);
            } else if is_bat {
                // Blue/cyan for batteries (charging/discharging)
                watt_color = mix(vec3<f32>(0.1, 0.1, 0.3), vec3<f32>(0.3, 0.6, 0.9), clamp(wv / 12.0, 0.0, 1.0));
            } else {
                // Wire: dim white proportional to voltage
                watt_color = vec3<f32>(0.4, 0.4, 0.45) * clamp(wv / 12.0, 0.0, 1.0);
            }
            color = mix(color * 0.3, watt_color, 0.7);
        } else {
            let is_watt_infra = is_gen || is_con || is_bat || wbt == 36u || wbt == 42u || wbt == 43u
                || (bflags & 0x80u) != 0u;
            if is_watt_infra {
                color = mix(color * 0.3, vec3<f32>(0.3, 0.3, 0.35), 0.5);
            } else {
                color *= 0.25;
            }
        }
    } else if camera.fluid_overlay < 12.5 {
        // Water overlay (12): show surface water level
        let wl = clamp(water_level * 2.0, 0.0, 1.0);
        if wl > 0.01 {
            let water_ov_color = mix(vec3(0.08, 0.12, 0.25), vec3(0.15, 0.45, 0.9), wl);
            color = mix(color * 0.4, water_ov_color, 0.6 + wl * 0.3);
        } else {
            color *= 0.3;
        }
    } else if camera.fluid_overlay < 13.5 {
        // Water Table overlay (13): underground water depth
        let wt_norm = clamp((wt_depth + 3.0) / 3.5, 0.0, 1.0); // -3→0, 0.5→1
        // Brown (dry/deep) → blue (wet/spring)
        let dry_col = vec3(0.35, 0.22, 0.10);
        let wet_col = vec3(0.10, 0.30, 0.70);
        let spring_col = vec3(0.15, 0.50, 0.90);
        var wt_color = mix(dry_col, wet_col, clamp(wt_norm * 1.5, 0.0, 1.0));
        if wt_depth > 0.0 {
            // Active spring: bright blue pulse
            wt_color = mix(wt_color, spring_col, clamp(wt_depth * 2.0, 0.0, 1.0));
        }
        color = mix(color * 0.3, wt_color, 0.7);
    } else if camera.fluid_overlay < 14.5 {
        // Sound overlay (14): dB-scaled pressure visualization — bilinear interpolated
        // Colors match the decibel scale legend (green → yellow → orange → red → magenta → purple)
        let snd_fx = world_x * 2.0;
        let snd_fy = world_y * 2.0;
        let snd_x0 = i32(floor(snd_fx));
        let snd_y0 = i32(floor(snd_fy));
        let snd_fx_frac = snd_fx - floor(snd_fx);
        let snd_fy_frac = snd_fy - floor(snd_fy);
        let snd_max = vec2<i32>(i32(camera.grid_w) * 2 - 1, i32(camera.grid_h) * 2 - 1);
        let snd00 = textureLoad(sound_tex, clamp(vec2(snd_x0, snd_y0), vec2(0), snd_max), 0);
        let snd10 = textureLoad(sound_tex, clamp(vec2(snd_x0 + 1, snd_y0), vec2(0), snd_max), 0);
        let snd01 = textureLoad(sound_tex, clamp(vec2(snd_x0, snd_y0 + 1), vec2(0), snd_max), 0);
        let snd11 = textureLoad(sound_tex, clamp(vec2(snd_x0 + 1, snd_y0 + 1), vec2(0), snd_max), 0);
        let snd_top = mix(snd00, snd10, snd_fx_frac);
        let snd_bot = mix(snd01, snd11, snd_fx_frac);
        let sp = mix(snd_top, snd_bot, snd_fy_frac);
        let pressure = sp.r;
        let velocity = sp.g;
        let amp = abs(pressure);

        // Convert amplitude to approximate dB: dB = 80 + 40*log10(amp)
        // amp 0.001 → ~-40 dB, amp 0.01 → ~0 dB, amp 0.1 → ~40 dB,
        // amp 1.0 → 80 dB, amp 10 → 120 dB, amp 100 → 160 dB
        var db_val = 0.0;
        if amp > 0.0001 {
            db_val = 80.0 + 40.0 * log(amp) / log(10.0);
        }
        // Normalize to 0-1 range for color mapping (0 dB → 0.0, 180 dB → 1.0)
        let t = clamp(db_val / 180.0, 0.0, 1.0);

        // Color ramp matching the legend:
        // 0.0 (silent)     = dark gray  (40, 40, 40)
        // 0.22 (~40 dB)    = dark green (60, 70, 55)
        // 0.33 (~60 dB)    = yellow     (160, 140, 20)
        // 0.44 (~80 dB)    = orange     (200, 100, 10)
        // 0.55 (~100 dB)   = red        (255, 50, 10)
        // 0.66 (~120 dB)   = magenta    (220, 20, 80)
        // 0.77 (~140 dB)   = purple     (140, 20, 200)
        // 1.0  (~180 dB)   = white      (255, 255, 200)
        var sound_color: vec3<f32>;
        if t < 0.22 {
            let s = t / 0.22;
            sound_color = mix(vec3(0.15, 0.15, 0.15), vec3(0.24, 0.27, 0.22), s);
        } else if t < 0.33 {
            let s = (t - 0.22) / 0.11;
            sound_color = mix(vec3(0.24, 0.27, 0.22), vec3(0.63, 0.55, 0.08), s);
        } else if t < 0.44 {
            let s = (t - 0.33) / 0.11;
            sound_color = mix(vec3(0.63, 0.55, 0.08), vec3(0.78, 0.39, 0.04), s);
        } else if t < 0.55 {
            let s = (t - 0.44) / 0.11;
            sound_color = mix(vec3(0.78, 0.39, 0.04), vec3(1.0, 0.20, 0.04), s);
        } else if t < 0.66 {
            let s = (t - 0.55) / 0.11;
            sound_color = mix(vec3(1.0, 0.20, 0.04), vec3(0.86, 0.08, 0.31), s);
        } else if t < 0.77 {
            let s = (t - 0.66) / 0.11;
            sound_color = mix(vec3(0.86, 0.08, 0.31), vec3(0.55, 0.08, 0.78), s);
        } else {
            let s = (t - 0.77) / 0.23;
            sound_color = mix(vec3(0.55, 0.08, 0.78), vec3(1.0, 1.0, 0.78), s);
        }

        // Rarefaction tint: negative pressure gets a blue shift
        if pressure < 0.0 {
            sound_color = mix(sound_color, vec3(0.1, 0.2, 0.5), 0.3);
        }

        // Wavefront edges: velocity highlights the expanding ring
        let v_intensity = clamp(abs(velocity) * 8.0, 0.0, 1.0);
        sound_color += vec3(v_intensity * 0.12);

        let vis = max(t, v_intensity * 0.3);
        color = mix(color * 0.2, sound_color, clamp(vis * 1.5, 0.0, 0.95));
    } else if camera.fluid_overlay < 15.5 {
        // Terrain type overlay (15): color per terrain type from terrain_buf
        let t_idx = u32(by) * u32(camera.grid_w) + u32(bx);
        let td = terrain_buf[t_idx];
        let tt = td & 0xFu;
        // Match colors to legend
        var tc = vec3(0.42, 0.36, 0.22); // default grass
        if tt == 0u { tc = vec3(0.42, 0.36, 0.22); }      // Grass
        else if tt == 1u { tc = vec3(0.68, 0.66, 0.60); }  // Chalky
        else if tt == 2u { tc = vec3(0.45, 0.42, 0.38); }  // Rocky
        else if tt == 3u { tc = vec3(0.50, 0.38, 0.25); }  // Clay
        else if tt == 4u { tc = vec3(0.48, 0.46, 0.42); }  // Gravel
        else if tt == 5u { tc = vec3(0.22, 0.18, 0.12); }  // Peat
        else if tt == 6u { tc = vec3(0.30, 0.35, 0.22); }  // Marsh
        else if tt == 7u { tc = vec3(0.38, 0.30, 0.18); }  // Loam
        // Blend with subtle base color for depth
        let ground = bheight == 0u && (btype == BT_GROUND || btype == BT_AIR);
        if ground {
            color = mix(color * 0.3, tc, 0.75);
        } else {
            color *= 0.4; // dim non-ground
        }
    } else if camera.fluid_overlay < 16.5 {
        // Dust density overlay (16): bilinear sampled heatmap
        let duv = vec2<f32>(
            world_x / camera.grid_w * 512.0 - 0.5,
            world_y / camera.grid_h * 512.0 - 0.5
        );
        let dip = vec2<i32>(floor(duv));
        let dfp = fract(duv);
        let d00 = textureLoad(dust_tex, clamp(dip, vec2(0), vec2(511)), 0).r;
        let d10 = textureLoad(dust_tex, clamp(dip + vec2(1, 0), vec2(0), vec2(511)), 0).r;
        let d01 = textureLoad(dust_tex, clamp(dip + vec2(0, 1), vec2(0), vec2(511)), 0).r;
        let d11 = textureLoad(dust_tex, clamp(dip + vec2(1, 1), vec2(0), vec2(511)), 0).r;
        let d = mix(mix(d00, d10, dfp.x), mix(d01, d11, dfp.x), dfp.y);
        // Heatmap: black (0) → brown (low) → orange (mid) → yellow (high)
        var dc = vec3(0.0);
        if d > 0.005 {
            let t = clamp(d / 1.5, 0.0, 1.0);
            if t < 0.33 {
                dc = mix(vec3(0.1, 0.05, 0.02), vec3(0.55, 0.30, 0.12), t / 0.33);
            } else if t < 0.66 {
                dc = mix(vec3(0.55, 0.30, 0.12), vec3(0.85, 0.55, 0.15), (t - 0.33) / 0.33);
            } else {
                dc = mix(vec3(0.85, 0.55, 0.15), vec3(1.0, 0.95, 0.5), (t - 0.66) / 0.34);
            }
        }
        color = mix(color * 0.2, dc, 0.85);
    }

    // Velocity arrow overlay (when fractional part of fluid_overlay > 0.1)
    let show_arrows = fract(camera.fluid_overlay) > 0.1;
    if show_arrows && camera.fluid_overlay > 0.5 {
        let vel_cell = vec2<i32>(bx, by);
        let vel = textureLoad(fluid_vel_tex, vel_cell, 0).xy;
        let vel_mag = length(vel);
        if vel_mag > 0.5 {
            let dir = vel / vel_mag;
            let afx = fx - 0.5;
            let afy = fy - 0.5;
            // Arrow shaft
            let along = afx * dir.x + afy * dir.y;
            let perp = abs(-afx * dir.y + afy * dir.x);
            let arrow_len = clamp(vel_mag * 0.015, 0.08, 0.35);
            let on_shaft = along > -0.02 && along < arrow_len && perp < 0.04;
            // Arrowhead
            let head_t = (along - arrow_len + 0.08) / 0.08;
            let on_head = head_t > 0.0 && head_t < 1.0 && perp < 0.12 * (1.0 - head_t);
            if on_shaft || on_head {
                color = mix(color, vec3(1.0, 1.0, 1.0), 0.7);
            }
        }
    }

    // Rain overlay: wind-angled streaks with parallax layers + ground splashes
    if camera.rain_intensity > 0.01 {
        let ri = camera.rain_intensity;
        // Wind-driven rain: horizontal drift (never upward on screen = never negative Y drift)
        let raw_dx = cos(camera.wind_angle) * camera.wind_magnitude;
        let raw_dy = sin(camera.wind_angle) * camera.wind_magnitude;
        // Horizontal drift speed for streaks (wind X component)
        let drift_x = raw_dx * 0.4;
        // Slant factor: how much X position skews the Y phase (wind-angled streaks)
        let slant = raw_dx * 0.3;

        // --- Falling streaks: 3 parallax layers (near/mid/far) ---
        var rain_accum = 0.0;

        // Layer 0: near (large, slow, thick)
        {
            let sc = 4.0; let sp = 18.0; let w = 0.018; let wt = 0.45; let off = 0.0;
            let rx = (world_x - camera.time * drift_x * 1.0) * sc + off;
            let ry = (world_y + world_x * slant) * sc;
            let col = floor(rx);
            let col_rand = fract(sin(col * 127.1 + off * 311.7) * 43758.5453);
            let col_fx = fract(rx);
            let fall = fract(ry * 0.5 - camera.time * sp * 0.1 + col_rand * 10.0);
            let dash_len = 0.08 + col_rand * 0.06;
            let streak_v = smoothstep(dash_len, 0.0, fall) * smoothstep(0.0, dash_len * 0.3, fall);
            let streak_h = smoothstep(w, 0.0, abs(col_fx - 0.5 - (col_rand - 0.5) * 0.3));
            rain_accum += streak_v * streak_h * wt;
        }
        // Layer 1: mid
        {
            let sc = 7.0; let sp = 22.0; let w = 0.012; let wt = 0.35; let off = 37.7;
            let rx = (world_x - camera.time * drift_x * 1.3) * sc + off;
            let ry = (world_y + world_x * slant) * sc;
            let col = floor(rx);
            let col_rand = fract(sin(col * 127.1 + off * 311.7) * 43758.5453);
            let col_fx = fract(rx);
            let fall = fract(ry * 0.5 - camera.time * sp * 0.1 + col_rand * 10.0);
            let dash_len = 0.08 + col_rand * 0.06;
            let streak_v = smoothstep(dash_len, 0.0, fall) * smoothstep(0.0, dash_len * 0.3, fall);
            let streak_h = smoothstep(w, 0.0, abs(col_fx - 0.5 - (col_rand - 0.5) * 0.3));
            rain_accum += streak_v * streak_h * wt;
        }
        // Layer 2: far (fine, fast, thin)
        {
            let sc = 12.0; let sp = 28.0; let w = 0.008; let wt = 0.20; let off = 91.3;
            let rx = (world_x - camera.time * drift_x * 1.6) * sc + off;
            let ry = (world_y + world_x * slant) * sc;
            let col = floor(rx);
            let col_rand = fract(sin(col * 127.1 + off * 311.7) * 43758.5453);
            let col_fx = fract(rx);
            let fall = fract(ry * 0.5 - camera.time * sp * 0.1 + col_rand * 10.0);
            let dash_len = 0.08 + col_rand * 0.06;
            let streak_v = smoothstep(dash_len, 0.0, fall) * smoothstep(0.0, dash_len * 0.3, fall);
            let streak_h = smoothstep(w, 0.0, abs(col_fx - 0.5 - (col_rand - 0.5) * 0.3));
            rain_accum += streak_v * streak_h * wt;
        }

        // --- Ground splashes: tiny bright circles that pulse ---
        var splash = 0.0;
        if ri > 0.15 {
            let splash_sc = 6.0;
            let ssx = world_x * splash_sc;
            let ssy = world_y * splash_sc;
            let cell_x = floor(ssx);
            let cell_y = floor(ssy);
            let cell_fx = fract(ssx) - 0.5;
            let cell_fy = fract(ssy) - 0.5;
            let cell_r = fract(sin(cell_x * 127.1 + (cell_y + 100.0) * 311.7) * 43758.5453);
            let cell_r2 = fract(sin((cell_x + 50.0) * 127.1 + cell_y * 311.7) * 43758.5453);
            let scx = (cell_r - 0.5) * 0.6;
            let scy = (cell_r2 - 0.5) * 0.6;
            let d = length(vec2(cell_fx - scx, cell_fy - scy));
            let period = 0.4 + cell_r * 0.6;
            let phase = fract(camera.time / period + cell_r * 7.0);
            let ring_r = phase * 0.15;
            let ring = smoothstep(0.02, 0.0, abs(d - ring_r)) * smoothstep(1.0, 0.0, phase);
            let dot = smoothstep(0.03, 0.0, d) * smoothstep(0.3, 0.0, phase);
            splash = (ring + dot * 0.5) * ri;
        }

        // Combine streaks + splashes
        let rain_alpha = clamp(rain_accum * ri * 1.2 + splash * 0.3, 0.0, 0.55);
        let rain_color = vec3(0.82, 0.85, 0.92);
        color = mix(color, rain_color, rain_alpha);

        // Atmospheric haze: heavier rain → more fog/mist
        let haze = ri * ri * 0.12;
        let haze_color = vec3(0.55, 0.58, 0.65);
        color = mix(color, haze_color, haze);

        // Cloud dimming tint
        let cloud_tint = mix(vec3(1.0), vec3(0.75, 0.78, 0.85), camera.cloud_cover * 0.3);
        color *= cloud_tint;
    }

    // Final fog overlay: covers EVERYTHING (terrain + gas) in the border zone
    color = mix(vec3(0.12, 0.12, 0.15), color, border_fade);

    // Temporal blend: mix current frame with previous for TAA shadow smoothing
    if temporal_blend > 0.01 {
        let prev_color = textureLoad(prev_output, vec2<i32>(i32(temporal_prev_px), i32(temporal_prev_py)), 0).rgb;
        color = mix(color, prev_color, temporal_blend);
    }

    // Workgroup shadow blur removed — workgroupBarrier() requires uniform control flow
    // which is incompatible with early returns above (roof, emissive, wall face).
    // Temporal blend provides sufficient shadow smoothing.

    // --- Fire overlay for burning blocks (applied AFTER lighting so flames are emissive) ---
    // Skip scorched dirt (flags bit 3) — grass already burned away
    let skip_fire = btype == BT_GROUND && (bflags & 8u) != 0u;
    if mat.is_flammable > 0.5 && !skip_fire {
        let fire_tidx = u32(by) * u32(camera.grid_w) + u32(bx);
        let fire_temp = block_temps[fire_tidx];
        if fire_temp > mat.ignition_temp {
            let burn_i = clamp((fire_temp - mat.ignition_temp) / 300.0, 0.0, 1.0);
            let fire_ov = render_fire_overlay(world_x, world_y, fx, fy, camera.time, burn_i);
            // Fire is emissive — boost brightness above 1.0, not dimmed by shadows
            let emissive_fire = fire_ov.rgb * (1.2 + 0.8 * burn_i);
            color = mix(color, emissive_fire, fire_ov.a);
        }
    }

    // --- 2.5D wall cap: walls to the south project upward into this tile ---
    // Applied after pleb rendering so we can show pleb outlines through walls.
    if !is_wall_face && bheight == 0u && by + 1 < i32(camera.grid_h) {
        let south_wall_h2 = effective_tile_height(bx, by + 1);
        if south_wall_h2 > 0u {
            let cap_depth = min(f32(south_wall_h2) * 0.06, 0.12);
            let cap_start = 1.0 - cap_depth;
            if fy > cap_start {
                let t = smoothstep(0.0, 1.0, (fy - cap_start) / cap_depth);
                // Get wall material color for cap
                let s_wd_idx2 = u32(by + 1) * u32(camera.grid_w) + u32(bx);
                let s_wd2 = read_wall_data(s_wd_idx2);
                var cap_col: vec3<f32>;
                if s_wd2 != 0u && (s_wd2 & 0xFu) != 0u {
                    cap_col = wall_material_color(wd_material_s(s_wd2));
                } else {
                    let s_block2 = get_block(bx, by + 1);
                    cap_col = block_base_color(block_type(s_block2), block_flags(s_block2));
                }
                cap_col *= 1.05; // top surface catches more light
                // Apply shadow/lighting to cap
                cap_col = cap_col * (ambient + sun_color * light_factor * 0.7);

                if drew_pleb {
                    // Pleb behind wall: show outline through cap
                    // Blend cap on top but leave a bright outline edge around the pleb
                    let pleb_outline_col = vec3(0.85, 0.90, 0.75);
                    color = mix(color, pleb_outline_col, t * 0.4);
                } else {
                    // Normal cap: opaque wall top surface
                    color = mix(color, cap_col, t * 0.75);
                }
            }
        }
    }

    // Sprite hover outline: silhouette detection on hovered tree, bush, or rock
    if camera.hover_x >= 0.0 && is_tree_pixel {
        let hbx = i32(floor(camera.hover_x));
        let hby = i32(floor(camera.hover_y));
        var hover_tree_tx = -1.0;
        var hover_tree_ty = -1.0;
        var hover_best_w = 0.0;
        var hover_sprite_kind = 0u; // 0=tree, 1=bush, 2=rock
        for (var hdy: i32 = -2; hdy <= 2; hdy++) {
            for (var hdx: i32 = -2; hdx <= 2; hdx++) {
                let htx = hbx + hdx;
                let hty = hby + hdy;
                if htx < 0 || hty < 0 || htx >= i32(camera.grid_w) || hty >= i32(camera.grid_h) { continue; }
                let htb = get_block(htx, hty);
                let hbt = block_type(htb);
                if hbt == BT_TREE {
                    let hr = render_tree(camera.hover_x, camera.hover_y,
                        f32(htx), f32(hty), block_height(htb), block_flags(htb));
                    if hr.w > hover_best_w {
                        hover_best_w = hr.w;
                        hover_tree_tx = f32(htx);
                        hover_tree_ty = f32(hty);
                        hover_sprite_kind = 0u;
                    }
                } else if (hbt == BT_BERRY_BUSH || hbt == BT_ROCK) && htx == hbx && hty == hby {
                    // Tile-aligned sprite: check if hover is on sprite pixel
                    let sfx = camera.hover_x - f32(htx);
                    let sfy = camera.hover_y - f32(hty);
                    var sp_alpha = 0.0;
                    var kind = 1u;
                    if hbt == BT_BERRY_BUSH {
                        let bid = f32(htx) * 73.0 + f32(hty) * 197.0;
                        let bh2 = fract(sin(bid) * 43758.5453);
                        let bv2 = u32(bh2 * f32(BUSH_SPRITE_VARIANTS)) % BUSH_SPRITE_VARIANTS;
                        sp_alpha = sample_bush_sprite(bv2, sfx, 1.0 - sfy).w;
                        kind = 1u;
                    } else {
                        let rid = f32(htx) * 59.0 + f32(hty) * 173.0;
                        let rh2 = fract(sin(rid) * 43758.5453);
                        let rv2 = u32(rh2 * f32(ROCK_SPRITE_VARIANTS)) % ROCK_SPRITE_VARIANTS;
                        sp_alpha = sample_rock_sprite(rv2, sfx, 1.0 - sfy).w;
                        kind = 2u;
                    }
                    if sp_alpha > 0.05 {
                        hover_tree_tx = f32(htx);
                        hover_tree_ty = f32(hty);
                        hover_sprite_kind = kind;
                        hover_best_w = 1.0;
                    }
                }
            }
        }

        if hover_tree_tx >= 0.0 && hover_sprite_kind == 0u && tree_win_tx == hover_tree_tx && tree_win_ty == hover_tree_ty {
            // Tree sprite edge detection
            let ht_id = hover_tree_tx * 137.0 + hover_tree_ty * 311.0;
            let ht_hash = fract(sin(ht_id) * 43758.5453);
            let ht_var = u32(ht_hash * f32(SPRITE_VARIANTS)) % SPRITE_VARIANTS;
            let ht_block = get_block(i32(hover_tree_tx), i32(hover_tree_ty));
            let ht_h = block_height(ht_block);
            let ht_size = select(select(select(2.0, 2.8, ht_h >= 3u), 3.5, ht_h >= 4u), 4.0, ht_h >= 5u);
            let ht_ox = (fract(sin(ht_id * 1.3 + 7.1) * 31415.9) - 0.5) * 0.5;
            let ht_oy = (fract(sin(ht_id * 2.7 + 3.9) * 27183.6) - 0.5) * 0.5;
            let ht_cx = hover_tree_tx + 0.5 + ht_ox;
            let ht_cy = hover_tree_ty + 0.5 + ht_oy;
            let ht_cu = 0.5 + (world_x - ht_cx) / ht_size;
            let ht_cv = 0.5 - (world_y - ht_cy) / ht_size;
            let ps1 = 2.0 / f32(SPRITE_SIZE);
            let ps2 = 4.0 / f32(SPRITE_SIZE);
            let ps3 = 6.0 / f32(SPRITE_SIZE);
            let oc = vec3(0.9, 0.9, 0.85);
            let e1 = sample_sprite(ht_var, ht_cu + ps1, ht_cv).w < 0.05
                  || sample_sprite(ht_var, ht_cu - ps1, ht_cv).w < 0.05
                  || sample_sprite(ht_var, ht_cu, ht_cv + ps1).w < 0.05
                  || sample_sprite(ht_var, ht_cu, ht_cv - ps1).w < 0.05
                  || ht_cu < ps1 || ht_cu > 1.0 - ps1 || ht_cv < ps1 || ht_cv > 1.0 - ps1;
            let e2 = sample_sprite(ht_var, ht_cu + ps2, ht_cv).w < 0.05
                  || sample_sprite(ht_var, ht_cu - ps2, ht_cv).w < 0.05
                  || sample_sprite(ht_var, ht_cu, ht_cv + ps2).w < 0.05
                  || sample_sprite(ht_var, ht_cu, ht_cv - ps2).w < 0.05;
            let e3 = sample_sprite(ht_var, ht_cu + ps3, ht_cv).w < 0.05
                  || sample_sprite(ht_var, ht_cu - ps3, ht_cv).w < 0.05
                  || sample_sprite(ht_var, ht_cu, ht_cv + ps3).w < 0.05
                  || sample_sprite(ht_var, ht_cu, ht_cv - ps3).w < 0.05;
            if e1 { color = mix(color, oc, 0.6); }
            else if e2 { color = mix(color, oc, 0.35); }
            else if e3 { color = mix(color, oc, 0.15); }
        }

        // Bush or Rock sprite edge detection (tile-aligned sprites)
        if hover_sprite_kind >= 1u && hover_tree_tx >= 0.0 && bx == i32(hover_tree_tx) && by == i32(hover_tree_ty) {
            let su = fx;
            let sv = 1.0 - fy;
            let oc = vec3(0.9, 0.9, 0.85);
            if hover_sprite_kind == 1u {
                // Bush
                let bid3 = hover_tree_tx * 73.0 + hover_tree_ty * 197.0;
                let bh3 = fract(sin(bid3) * 43758.5453);
                let bv3 = u32(bh3 * f32(BUSH_SPRITE_VARIANTS)) % BUSH_SPRITE_VARIANTS;
                let ps1 = 2.0 / f32(BUSH_SPRITE_SIZE);
                let ps2 = 4.0 / f32(BUSH_SPRITE_SIZE);
                let e1 = sample_bush_sprite(bv3, su + ps1, sv).w < 0.05
                      || sample_bush_sprite(bv3, su - ps1, sv).w < 0.05
                      || sample_bush_sprite(bv3, su, sv + ps1).w < 0.05
                      || sample_bush_sprite(bv3, su, sv - ps1).w < 0.05;
                let e2 = sample_bush_sprite(bv3, su + ps2, sv).w < 0.05
                      || sample_bush_sprite(bv3, su - ps2, sv).w < 0.05
                      || sample_bush_sprite(bv3, su, sv + ps2).w < 0.05
                      || sample_bush_sprite(bv3, su, sv - ps2).w < 0.05;
                if e1 { color = mix(color, oc, 0.6); }
                else if e2 { color = mix(color, oc, 0.3); }
            } else {
                // Rock
                let rid3 = hover_tree_tx * 59.0 + hover_tree_ty * 173.0;
                let rh3 = fract(sin(rid3) * 43758.5453);
                let rv3 = u32(rh3 * f32(ROCK_SPRITE_VARIANTS)) % ROCK_SPRITE_VARIANTS;
                let ps1 = 2.0 / f32(ROCK_SPRITE_SIZE);
                let ps2 = 4.0 / f32(ROCK_SPRITE_SIZE);
                let e1 = sample_rock_sprite(rv3, su + ps1, sv).w < 0.05
                      || sample_rock_sprite(rv3, su - ps1, sv).w < 0.05
                      || sample_rock_sprite(rv3, su, sv + ps1).w < 0.05
                      || sample_rock_sprite(rv3, su, sv - ps1).w < 0.05;
                let e2 = sample_rock_sprite(rv3, su + ps2, sv).w < 0.05
                      || sample_rock_sprite(rv3, su - ps2, sv).w < 0.05
                      || sample_rock_sprite(rv3, su, sv + ps2).w < 0.05
                      || sample_rock_sprite(rv3, su, sv - ps2).w < 0.05;
                if e1 { color = mix(color, oc, 0.6); }
                else if e2 { color = mix(color, oc, 0.3); }
            }
        }
    }

    // Hover tint: highlight interactable blocks under the cursor
    if camera.hover_x >= 0.0 {
        let hbx = i32(floor(camera.hover_x));
        let hby = i32(floor(camera.hover_y));
        if bx == hbx && by == hby {
            // Determine if this block is interactable and choose tint color
            var hover_tint = vec3(0.0);
            var is_hover = false;
            // Doors: warm highlight
            if (bflags & 1u) != 0u && btype == BT_WALL {
                hover_tint = vec3(0.8, 0.6, 0.2); is_hover = true;
            }
            // Toggle blocks: switches, valves, breakers
            if btype == BT_SWITCH || btype == BT_VALVE || btype == BT_BREAKER {
                hover_tint = vec3(0.3, 0.7, 1.0); is_hover = true;
            }
            // Slider blocks: dimmers, restrictors, fireplaces
            if btype == BT_DIMMER || btype == BT_RESTRICTOR || btype == BT_FIREPLACE || btype == BT_CAMPFIRE {
                hover_tint = vec3(1.0, 0.6, 0.3); is_hover = true;
            }
            // Fan, pump: mechanical blue
            if btype == BT_FAN || btype == BT_PUMP {
                hover_tint = vec3(0.3, 0.8, 0.9); is_hover = true;
            }
            // Workbench, kiln, well: utility gold
            if btype == BT_WORKBENCH || btype == BT_KILN || btype == BT_WELL || btype == BT_SAW_HORSE {
                hover_tint = vec3(0.9, 0.7, 0.2); is_hover = true;
            }
            // Storage: crate
            if btype == BT_CRATE {
                hover_tint = vec3(0.7, 0.55, 0.3); is_hover = true;
            }
            if btype == BT_BED {
                hover_tint = vec3(0.5, 0.5, 0.8); is_hover = true;
            }
            if is_hover {
                // Subtle pulsing highlight
                let pulse = 0.08 + 0.04 * sin(camera.time * 3.0);
                color = mix(color, hover_tint, pulse);
                // Edge brighten for clarity
                let edge_x = min(fx, 1.0 - fx);
                let edge_y = min(fy, 1.0 - fy);
                let edge = smoothstep(0.08, 0.0, min(edge_x, edge_y));
                color = mix(color, hover_tint, edge * 0.25);
            }
        }
    }

    // Dust layer — reddish brown, GPU-simulated (manual bilinear for smooth look)
    {
        let dust_uv = vec2<f32>(
            world_x / camera.grid_w * 512.0 - 0.5,
            world_y / camera.grid_h * 512.0 - 0.5
        );
        let ip = vec2<i32>(floor(dust_uv));
        let fp = fract(dust_uv);
        let c00 = textureLoad(dust_tex, clamp(ip, vec2(0), vec2(511)), 0).r;
        let c10 = textureLoad(dust_tex, clamp(ip + vec2(1, 0), vec2(0), vec2(511)), 0).r;
        let c01 = textureLoad(dust_tex, clamp(ip + vec2(0, 1), vec2(0), vec2(511)), 0).r;
        let c11 = textureLoad(dust_tex, clamp(ip + vec2(1, 1), vec2(0), vec2(511)), 0).r;
        let dust_d = mix(mix(c00, c10, fp.x), mix(c01, c11, fp.x), fp.y);
        if dust_d > 0.005 {
            let dust_color = vec3(0.55, 0.35, 0.22);
            let dust_alpha = clamp(dust_d * 0.7, 0.0, 0.8);
            color = mix(color, dust_color, dust_alpha);
        }
    }

    // Apply fog of war
    color = apply_fog(color, world_x, world_y);

    textureStore(output, vec2<u32>(px, py), vec4<f32>(color, 1.0));
}
