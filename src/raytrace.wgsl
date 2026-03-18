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
    _pad: f32,
};

fn get_material(bt: u32) -> GpuMaterial {
    return materials[min(bt, 13u)];
}

// --- Sprite constants ---
const SPRITE_SIZE: u32 = 16u;
const SPRITE_VARIANTS: u32 = 4u;

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

// --- Block unpacking ---
// type: 0=air, 1=stone, 2=dirt, 3=water, 4=wall, 5=glass, 6=fireplace, 7=electric_light, 8=tree, 9=bench, 10=standing_lamp, 11=table_lamp, 12=fan
// height: 0-255
// flags: bit0=is_door, bit1=has_roof
fn block_type(b: u32) -> u32 { return b & 0xFFu; }
fn block_height(b: u32) -> u32 { return (b >> 8u) & 0xFFu; }
fn block_flags(b: u32) -> u32 { return (b >> 16u) & 0xFFu; }
fn has_roof(b: u32) -> bool { return ((b >> 16u) & 2u) != 0u; }
fn is_door(b: u32) -> bool { return ((b >> 16u) & 1u) != 0u; }
fn is_open(b: u32) -> bool { return ((b >> 16u) & 4u) != 0u; }
fn is_glass(b: u32) -> bool { return block_type(b) == 5u; }

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
const SHADOW_STEP: f32 = 0.25;

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
        if bt == 5u {
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
            if bt == 9u && fbh <= ray_h {
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
    let dist = sqrt(dx * dx + dy * dy);
    if dist < 0.5 { return 1.0; }

    let steps = i32(ceil(dist * 2.0)); // ~2 samples per block
    var vis = 1.0;

    for (var i: i32 = 1; i < steps; i++) {
        let t = f32(i) / f32(steps);
        let sx = x0 + dx * t;
        let sy = y0 + dy * t;
        let sb = get_block(i32(floor(sx)), i32(floor(sy)));
        let sbt = block_type(sb);
        let sbh = block_height(sb);

        // Skip light source blocks
        if get_material(sbt).light_intensity > 0.0 { continue; } // skip light sources

        if sbh == 0u { continue; } // open floor

        // Light is above this block — passes over (furniture below light height)
        if f32(sbh) <= light_h {
            continue;
        }

        // Glass: partial transmission
        if sbt == 5u {
            vis *= 0.5;
            if vis < 0.02 { return 0.0; }
            continue;
        }

        // Trees: partial transmission through foliage
        if sbt == 8u {
            vis *= 0.4;
            if vis < 0.02 { return 0.0; }
            continue;
        }

        // Open door: passes through
        if is_door(sb) && is_open(sb) { continue; }

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
    let max_search = 6; // capped for performance
    let bx = i32(floor(wx));
    let by = i32(floor(wy));

    for (var dy: i32 = -max_search; dy <= max_search; dy++) {
        for (var dx: i32 = -max_search; dx <= max_search; dx++) {
            let nx = bx + dx;
            let ny = by + dy;
            let nb = get_block(nx, ny);
            let bt = block_type(nb);

            // All light source types
            if get_material(bt).light_intensity <= 0.0 {
                continue;
            }

            let lcx = f32(nx) + 0.5;
            let lcy = f32(ny) + 0.5;
            let fdx = wx - lcx;
            let fdy = wy - lcy;
            let dist = sqrt(fdx * fdx + fdy * fdy);

            // Get material properties for this light source
            let mat = get_material(bt);
            var radius = mat.light_radius;
            var intensity = mat.light_intensity;
            var light_col = vec3<f32>(mat.light_color_r, mat.light_color_g, mat.light_color_b);
            let light_h = mat.light_height;

            // Fireplace: apply flicker animation
            if bt == 6u {
                let phase = fire_hash(vec2<f32>(lcx, lcy)) * 6.28;
                let flicker = fire_flicker(time + phase);
                intensity *= (0.7 + 0.3 * flicker);
                let heat = clamp(1.0 - dist / 3.0, 0.0, 1.0);
                light_col = mix(light_col, FIRE_COLOR_HOT, heat * flicker);
            }

            if dist > radius { continue; }

            let vis = trace_glow_visibility(wx, wy, lcx, lcy, light_h);
            if vis < 0.01 { continue; }

            let atten = (1.0 / (1.0 + dist * 0.6 + dist * dist * 0.15))
                      * smoothstep(radius, radius * 0.15, dist);

            glow += light_col * intensity * atten * vis;
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
    let search = 6;
    let max_range = 6.0;

    for (var dy: i32 = -search; dy <= search; dy++) {
        for (var dx: i32 = -search; dx <= search; dx++) {
            let nx = bx + dx;
            let ny = by + dy;
            let nb = get_block(nx, ny);
            let bt = block_type(nb);

            // Only windows (glass) and open doors are portals
            let is_window = bt == 5u;
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

fn render_tree(wx: f32, wy: f32, fx: f32, fy: f32, height: u32, flags: u32) -> vec4<f32> {
    let is_large = (flags & 32u) != 0u;
    let quadrant = (flags >> 3u) & 3u; // 0=TL, 1=TR, 2=BL, 3=BR

    // For large (2x2) trees, compute UV across the 2x2 footprint
    var sprite_u = fx;
    var sprite_v = fy;
    // Use the top-left corner for the tree ID hash (consistent across all 4 tiles)
    var origin_x = floor(wx);
    var origin_y = floor(wy);

    if is_large {
        // Offset origin to top-left tile of the 2x2 group
        if (quadrant & 1u) != 0u { origin_x -= 1.0; } // TR or BR → shift left
        if (quadrant & 2u) != 0u { origin_y -= 1.0; } // BL or BR → shift up

        // Map fx/fy to 0..1 across the full 2x2 area
        let qx = f32(quadrant & 1u); // 0 or 1
        let qy = f32((quadrant >> 1u) & 1u); // 0 or 1
        sprite_u = (qx + fx) * 0.5;
        sprite_v = (qy + fy) * 0.5;
    }

    // Pick variant and rotation from origin position hash
    let tree_id = origin_x * 137.0 + origin_y * 311.0;
    let id_hash = fract(sin(tree_id) * 43758.5453);
    let variant = u32(id_hash * f32(SPRITE_VARIANTS)) % SPRITE_VARIANTS;

    // Random 90° rotation (0, 90, 180, 270) via UV transform around center
    let rot_hash = fract(sin(tree_id * 1.7 + 5.3) * 27183.6142);
    let rotation = u32(rot_hash * 4.0) % 4u;
    var ru = sprite_u - 0.5;
    var rv = sprite_v - 0.5;
    switch rotation {
        case 1u: { let tmp = ru; ru = -rv; rv = tmp; }   // 90°
        case 2u: { ru = -ru; rv = -rv; }                  // 180°
        case 3u: { let tmp = ru; ru = rv; rv = -tmp; }    // 270°
        default: { }                                       // 0°
    }
    let sprite = sample_sprite(variant, ru + 0.5, rv + 0.5);

    if sprite.w < 0.01 {
        return vec4<f32>(0.45, 0.35, 0.20, 0.0);
    }

    let color = sprite.xyz * (0.85 + id_hash * 0.3);
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
        default: { return vec3<f32>(1.0, 0.0, 1.0); }
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
    let dir2d = normalize(sun_dir);
    let step_x = dir2d.x * SHADOW_STEP;
    let step_y = dir2d.y * SHADOW_STEP;
    let step_h = sun_elev * SHADOW_STEP;

    var current_h = surface_height;
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
        let bh = f32(block_height(block));
        let bt = block_type(block);

        // Shadow occlusion logic:
        // - Roofed floor tiles: roof plane blocks ray when it climbs to roof height,
        //   but interior airspace below the roof is open for lateral light.
        // - Glass blocks: only the window opening (between sill and lintel) transmits
        //   tinted light; the wall below sill and above lintel is opaque.
        // - Other structural blocks use max(block_height, roof_height).
        let rh = get_roof_height(bx, by);
        let is_roofed_floor = has_roof(block) && bh < 0.5;

        var effective_h = bh;
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
        if bt == 8u {
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

            // Same rotation as rendering
            let rot_hash = fract(sin(tree_id * 1.7 + 5.3) * 27183.6142);
            let rotation = u32(rot_hash * 4.0) % 4u;
            var ru = tree_fx - 0.5;
            var rv = tree_fy - 0.5;
            switch rotation {
                case 1u: { let tmp = ru; ru = -rv; rv = tmp; }
                case 2u: { ru = -ru; rv = -rv; }
                case 3u: { let tmp = ru; ru = rv; rv = -tmp; }
                default: { }
            }
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
        } else

        // Does this block/roof intersect the ray?
        if effective_h > current_h {
            if bt == 5u {
                // Glass block: fixed-ratio split between window opening and
                // solid wall (sill + lintel). No current_h-dependent terms
                // to avoid flickering as sun angle changes frame-to-frame.
                let window_open_frac = 1.0 - WINDOW_SILL_FRAC - WINDOW_LINTEL_FRAC;

                // Glass portion: tint and partial absorption
                let absorption = GLASS_ABSORPTION * SHADOW_STEP * window_open_frac;
                light *= (1.0 - absorption);
                tint *= mix(vec3<f32>(1.0), GLASS_TINT, SHADOW_STEP * 0.8 * window_open_frac);

                // Wall portion (sill + lintel): fixed partial shadow per step
                let wall_frac = 1.0 - window_open_frac;
                light *= (1.0 - wall_frac * SHADOW_STEP * 1.5);

                if light < 0.02 {
                    return vec4<f32>(tint, 0.0);
                }
            } else if is_roofed_floor {
                // Roof is a hard opaque surface — fully blocks the shadow ray.
                // (Indoor pixels never reach here; they use compute_interior_light instead.)
                return vec4<f32>(tint, 0.0);
            } else if is_door(block) && is_open(block) {
                // Open door: ray passes through freely (doorway is an opening)
                // continue stepping
            } else {
                // Opaque block (wall, roof, etc.): shadow with soft edge
                let overlap = effective_h - current_h;
                let shadow_strength = clamp(overlap * 2.0, 0.0, 1.0);
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
fn wall_side_shade(wx: f32, wy: f32, bx: i32, by: i32, height: u32) -> vec3<f32> {
    let fx = fract(wx);
    let fy = fract(wy);
    var shade = vec3<f32>(0.0);

    if height == 0u { return shade; }

    let fh = f32(height);
    let edge_width = clamp(0.12 * fh, 0.04, 0.25);

    // Top edge (sun-facing: sun is upper-left)
    let top_neighbor = get_block(bx, by - 1);
    if block_height(top_neighbor) < height && fy < edge_width {
        let t = 1.0 - fy / edge_width;
        shade += vec3<f32>(0.15, 0.14, 0.12) * t;
    }
    // Left edge (sun-facing)
    let left_neighbor = get_block(bx - 1, by);
    if block_height(left_neighbor) < height && fx < edge_width {
        let t = 1.0 - fx / edge_width;
        shade += vec3<f32>(0.12, 0.11, 0.10) * t;
    }
    // Bottom edge (shadowed)
    let bottom_neighbor = get_block(bx, by + 1);
    if block_height(bottom_neighbor) < height && fy > (1.0 - edge_width) {
        let t = (fy - (1.0 - edge_width)) / edge_width;
        shade -= vec3<f32>(0.10, 0.10, 0.08) * t;
    }
    // Right edge (shadowed)
    let right_neighbor = get_block(bx + 1, by);
    if block_height(right_neighbor) < height && fx > (1.0 - edge_width) {
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

    // Wall neighbors: type 1 (stone) or type 4 (wall) or type 5 (glass)
    let h_wall = (left_t == 1u || left_t == 4u || left_t == 5u) ||
                 (right_t == 1u || right_t == 4u || right_t == 5u);
    let v_wall = (top_t == 1u || top_t == 4u || top_t == 5u) ||
                 (bot_t == 1u || bot_t == 4u || bot_t == 5u);

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

    // --- Temporal reprojection: reuse previous frame if possible ---
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

        // Time change invalidates lighting (sun position, shadows, flicker)
        let time_delta = abs(camera.time - camera.prev_time);
        let time_stable = time_delta < 0.0005; // only truly paused time allows reprojection

        // Only reproject when NOTHING changed: camera still, time paused, no fluid, no grid change
        let can_reproject = zoom_stable && in_prev_bounds && time_stable
            && camera_delta < 0.001 && !near_fluid
            && camera.force_refresh < 0.5;

        if can_reproject {
            // Use exact integer coords when camera hasn't moved (avoids sub-pixel drift)
            let prev_color = textureLoad(prev_output, vec2<i32>(i32(px), i32(py)), 0).rgb;
            textureStore(output, vec2<u32>(px, py), vec4(prev_color, 1.0));
            return;
        }
    }

    var block = get_block(bx, by);
    var btype = block_type(block);
    var bheight = block_height(block);
    var bflags = block_flags(block);
    var fheight = f32(bheight);

    // --- Oblique projection: show south face within the wall's own tile ---
    // The camera looks slightly from the south. If this block is tall and the
    // block to the south is shorter, the bottom strip of THIS tile shows the
    // south wall face. The wall stays entirely within its own block boundary.
    var is_wall_face = false;
    var wall_face_t = 0.0; // 0=top of face, 1=bottom of face

    let south_block = get_block(bx, by + 1);
    let south_h = block_height(south_block);
    let is_exterior_south = bheight > south_h && btype != 8u
        && !(is_door(block) && is_open(block));

    if is_exterior_south {
        let height_diff = f32(bheight - south_h);
        let face_height = min(height_diff * camera.oblique_strength, 0.35);
        let face_start = 1.0 - face_height; // face occupies bottom strip of tile
        if fy > face_start {
            is_wall_face = true;
            wall_face_t = (fy - face_start) / face_height; // 0=top of face, 1=bottom
        }
    }

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

        // Shadow on roof surface
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
        var face_color = block_base_color(btype, bflags);

        // Darken toward bottom of face (ambient occlusion at ground junction)
        face_color *= (0.60 + 0.40 * (1.0 - wall_face_t));

        // Subtle mortar/plank lines along the face
        let line = fract(fx * 4.0);
        let mortar = f32(line < 0.06) * 0.04;
        face_color -= vec3<f32>(mortar);

        // Glass face: show a window strip in the middle
        if btype == 5u {
            let glass_zone = wall_face_t > 0.2 && wall_face_t < 0.8;
            if glass_zone {
                face_color = vec3<f32>(0.4, 0.55, 0.7) * (0.8 + 0.2 * (1.0 - wall_face_t));
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
            color += face_glow * camera.indoor_glow_mul * (0.5 + night_boost);
        }

        color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
        textureStore(output, vec2<u32>(px, py), vec4<f32>(color, 1.0));
        return;
    }

    if btype == 5u {
        // Glass block: render with thin inset
        let glass_result = render_glass_block(world_x, world_y, fx, fy, bx, by);
        color = glass_result.xyz;
        is_glass_pixel = glass_result.w > 0.5;
    } else if btype == 6u {
        // Fireplace: animated emissive rendering
        color = render_fireplace(world_x, world_y, fx, fy, camera.time);
    } else if btype == 7u {
        // Electric light: ceiling fixture rendering
        color = render_electric_light(world_x, world_y, fx, fy, camera.time);
    } else if btype == 8u {
        // Tree: sprite-based rendering
        let tree_result = render_tree(world_x, world_y, fx, fy, bheight, bflags);
        color = tree_result.xyz;
        is_tree_pixel = tree_result.w > 0.01;
    } else if btype == 9u {
        // Bench
        color = render_bench(fx, fy, bflags);
    } else if btype == 10u {
        // Standing lamp (emissive)
        color = render_standing_lamp(fx, fy, camera.time);
    } else if btype == 11u {
        // Table lamp: bulb circle is emissive, bench surface is not
        let tl = render_table_lamp(fx, fy);
        color = tl.xyz;
        is_table_lamp_bulb = tl.w > 0.5;
    } else if btype == 12u {
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
    } else if btype == 13u {
        // Compost: brown-green organic heap with texture
        let noise = fract(sin(world_x * 13.7 + world_y * 7.3) * 43758.5);
        let heap = smoothstep(0.45, 0.2, length(vec2(fx - 0.5, fy - 0.5)));
        color = mix(vec3(0.30, 0.22, 0.12), vec3(0.25, 0.32, 0.10), noise * 0.5) * (0.7 + heap * 0.3);
        // Slight steam wisps
        let wisp = sin(world_x * 31.0 + camera.time * 2.0) * sin(world_y * 29.0 + camera.time * 1.7);
        color += vec3(0.05) * max(wisp, 0.0) * heap;
    } else {
        color = block_base_color(btype, bflags);
    }

    // Open door: treat as floor-level opening (overrides wall type)
    let door_is_open = is_door(block) && is_open(block);
    // Trees: transparent sprite pixels are ground-level; canopy keeps height for shadows
    let is_tree_ground = btype == 8u && !is_tree_pixel;
    let effective_height = select(bheight, 0u, door_is_open || is_tree_ground);
    let effective_fheight = f32(effective_height);

    // Height-based brightness (skip for trees — they have their own shading)
    if btype != 8u {
        color += vec3<f32>(effective_fheight * 0.03);
    }

    // Wall side faces (3D bevel) — skip for doors and trees
    if effective_height > 0u && btype != 8u {
        color += wall_side_shade(world_x, world_y, bx, by, effective_height);
    }

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
        // Outdoor pixel: trace shadow ray toward sun (per-pixel, stays here)
        let shadow_result = trace_shadow_ray(world_x, world_y, effective_fheight, sun_dir, sun_elev);
        shadow_tint = shadow_result.xyz;
        light_factor = shadow_result.w;

        // Outdoor lighting: directional window bleed only.
        // Outdoor point lights are handled by proximity glow (line-of-sight traced)
        // rather than the lightmap (which floods around obstacles unrealistically).
        if camera.enable_dir_bleed > 0.5 && effective_height == 0u {
            let bleed = compute_directional_bleed(world_x, world_y);
            light_color_out = bleed.xyz;
            light_intensity_out = bleed.w;
        }
    }

    if get_material(btype).is_emissive > 0.5 && (btype != 11u || is_table_lamp_bulb) {
        // Emissive block (fireplace/electric light): not affected by shadow/lighting.
        // Just clamp and output directly.
        color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
        textureStore(output, vec2<u32>(px, py), vec4<f32>(color, 1.0));
        return;
    }

    if btype == 5u && is_glass_pixel {
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
        let lit = color * (ambient + sun_color * light_factor * 0.85 * shadow_tint);
        // Point light is additive (not multiplied by base color) so it illuminates
        // even when ambient/sun is very low (e.g. at night)
        let pl_mul = select(camera.light_bleed_mul, camera.indoor_glow_mul, is_indoor);
        color = lit + light_color_out * light_intensity_out * pl_mul;
    }

    // Water effect
    if btype == 3u {
        let t = camera.time;
        let wave1 = sin(world_x * 11.0 + world_y * 7.0 + t * 2.0) * 0.03;
        let wave2 = sin(world_x * 5.0 - world_y * 13.0 + t * 1.3) * 0.02;
        let shimmer = wave1 + wave2 + 0.04;
        color += vec3<f32>(shimmer * 0.2, shimmer * 0.4, shimmer * 0.8);
        let caustic = abs(sin(world_x * 17.0 + t * 0.7) * sin(world_y * 19.0 + t * 0.9));
        color += vec3<f32>(0.0, 0.02, 0.06) * caustic * light_factor;
    }

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

    // --- Pleb rendering ---
    if camera.pleb_x > 0.5 {
        let pdx = world_x - camera.pleb_x;
        let pdy = world_y - camera.pleb_y;
        let pdist = length(vec2(pdx, pdy));

        // Torch: warm point light with wall occlusion
        if camera.pleb_torch > 0.5 && pdist < 6.0 {
            let vis = trace_glow_visibility(world_x, world_y, camera.pleb_x, camera.pleb_y, 1.0);
            if vis > 0.01 {
                let torch_atten = 1.0 / (1.0 + pdist * 0.4 + pdist * pdist * 0.08);
                let flicker = sin(camera.time * 8.3) * 0.15 + sin(camera.time * 13.1) * 0.1 + 0.85;
                color += vec3(1.0, 0.55, 0.15) * torch_atten * 0.5 * flicker * vis;
            }
        }

        // Headlight: directional cone with wall occlusion
        if camera.pleb_headlight > 0.5 && pdist > 0.5 && pdist < 10.0 {
            let vis = trace_glow_visibility(world_x, world_y, camera.pleb_x, camera.pleb_y, 1.5);
            if vis > 0.01 {
                let to_pixel = normalize(vec2(pdx, pdy));
                let light_dir = vec2(cos(camera.pleb_angle), sin(camera.pleb_angle));
                let cone = smoothstep(0.4, 0.85, dot(to_pixel, light_dir));
                let dist_atten = 1.0 / (1.0 + pdist * 0.2 + pdist * pdist * 0.04);
                color += vec3(0.85, 0.9, 1.0) * cone * dist_atten * 0.6 * vis;
            }
        }

        // Body rendering (close range)
        if pdist < 0.45 {
            // Selection ring (pulsing)
            if camera.pleb_selected > 0.5 {
                let ring_inner = 0.38;
                let ring_outer = 0.44;
                if pdist > ring_inner && pdist < ring_outer {
                    let pulse = sin(camera.time * 4.0) * 0.3 + 0.7;
                    color = mix(color, vec3(0.3, 0.9, 0.3), pulse);
                }
            }

            // Body: blue circle
            if pdist < 0.35 {
                let body_shade = 1.0 - pdist / 0.35 * 0.3;
                color = vec3(0.2, 0.45, 0.75) * body_shade;
            }

            // Head: skin-colored circle
            if pdist < 0.15 {
                color = vec3(0.85, 0.70, 0.55);
            }

            // Direction indicator: white dot in front
            let front_x = camera.pleb_x + cos(camera.pleb_angle) * 0.28;
            let front_y = camera.pleb_y + sin(camera.pleb_angle) * 0.28;
            let front_dist = length(vec2(world_x - front_x, world_y - front_y));
            if front_dist < 0.07 {
                color = vec3(0.95, 0.95, 1.0);
            }
        }
    }

    // Per-pixel proximity glow for floor tiles near light sources.
    // Applies to indoor floors AND outdoor ground (for outdoor campfires etc).
    // Wall tops are excluded to prevent light bleed onto the roof surface.
    // Apply proximity glow to floors, outdoor ground, and furniture (benches)
    let is_furniture = get_material(btype).is_furniture > 0.5 && !(btype == 11u && is_table_lamp_bulb);
    if camera.enable_prox_glow > 0.5 && (is_indoor || is_furniture || (!is_roofed_wall && effective_height == 0u)) {
        // Conditional glow: skip expensive 13x13 scan if lightmap shows no nearby light
        let lm_gate = sample_lightmap(world_x, world_y);
        if lm_gate.w > 0.02 {
            let prox_glow = compute_proximity_glow(world_x, world_y, camera.time);
            let night_boost = 1.0 - camera.sun_intensity * 0.7;
            color += prox_glow * camera.indoor_glow_mul * (0.5 + night_boost);
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
        let smoke_color = vec3(0.75, 0.73, 0.72);
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
        // Velocity: hue = direction, brightness = magnitude, with per-block arrows
        let sim_pos = vec2<i32>(vec2<f32>(world_x, world_y));
        let sim_clamped = clamp(sim_pos, vec2(0), vec2(i32(camera.grid_w) - 1, i32(camera.grid_h) - 1));
        let vel = textureLoad(fluid_vel_tex, sim_clamped, 0).xy;
        let mag = length(vel);
        let norm_mag = clamp(mag * 0.1, 0.0, 1.0);
        let angle = atan2(vel.y, vel.x) * 0.159 + 0.5;
        let vel_color = vec3(
            clamp(abs(angle * 6.0 - 3.0) - 1.0, 0.0, 1.0),
            clamp(2.0 - abs(angle * 6.0 - 2.0), 0.0, 1.0),
            clamp(2.0 - abs(angle * 6.0 - 4.0), 0.0, 1.0)
        ) * norm_mag;
        color = mix(color * 0.3, vel_color, clamp(norm_mag * 2.0, 0.0, 0.9));

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
        // Pressure: absolute pressure with ROYGBIV colormap (violet=low, red=high)
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

        // Absolute pressure mapped to 0..1 range
        let abs_p = clamp(abs(p) * 0.5, 0.0, 1.0);

        // Reverse ROYGBIV: violet(0) → blue → cyan → green → yellow → orange → red(1)
        var pcolor: vec3<f32>;
        if abs_p < 0.167 {
            let t = abs_p / 0.167;
            pcolor = mix(vec3(0.56, 0.0, 1.0), vec3(0.0, 0.0, 1.0), t);   // violet → blue
        } else if abs_p < 0.333 {
            let t = (abs_p - 0.167) / 0.167;
            pcolor = mix(vec3(0.0, 0.0, 1.0), vec3(0.0, 1.0, 1.0), t);   // blue → cyan
        } else if abs_p < 0.5 {
            let t = (abs_p - 0.333) / 0.167;
            pcolor = mix(vec3(0.0, 1.0, 1.0), vec3(0.0, 1.0, 0.0), t);   // cyan → green
        } else if abs_p < 0.667 {
            let t = (abs_p - 0.5) / 0.167;
            pcolor = mix(vec3(0.0, 1.0, 0.0), vec3(1.0, 1.0, 0.0), t);   // green → yellow
        } else if abs_p < 0.833 {
            let t = (abs_p - 0.667) / 0.167;
            pcolor = mix(vec3(1.0, 1.0, 0.0), vec3(1.0, 0.5, 0.0), t);   // yellow → orange
        } else {
            let t = (abs_p - 0.833) / 0.167;
            pcolor = mix(vec3(1.0, 0.5, 0.0), vec3(1.0, 0.0, 0.0), t);   // orange → red
        }

        color = mix(color * 0.25, pcolor, clamp(abs_p * 2.5 + 0.1, 0.0, 0.9));
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
        // Temperature: blue (cold) → white (ambient) → red (hot) → yellow (very hot)
        let temp = smoke.a;
        let temp_norm = clamp((temp + 20.0) / 520.0, 0.0, 1.0); // -20°C..500°C → 0..1
        var temp_color: vec3<f32>;
        if temp_norm < 0.05 {
            temp_color = vec3(0.0, 0.0, 0.8);  // very cold: blue
        } else if temp_norm < 0.1 {
            let t = (temp_norm - 0.05) / 0.05;
            temp_color = mix(vec3(0.0, 0.0, 0.8), vec3(0.7, 0.7, 0.9), t); // cold → cool
        } else if temp_norm < 0.15 {
            let t = (temp_norm - 0.1) / 0.05;
            temp_color = mix(vec3(0.7, 0.7, 0.9), vec3(0.9, 0.9, 0.9), t); // cool → white (ambient)
        } else if temp_norm < 0.4 {
            let t = (temp_norm - 0.15) / 0.25;
            temp_color = mix(vec3(0.9, 0.9, 0.9), vec3(1.0, 0.3, 0.0), t); // ambient → hot red
        } else {
            let t = (temp_norm - 0.4) / 0.6;
            temp_color = mix(vec3(1.0, 0.3, 0.0), vec3(1.0, 1.0, 0.3), t); // hot → very hot yellow
        }
        color = mix(color * 0.3, temp_color, 0.7);
    } else {
        // Heat Flow: velocity magnitude colored by temperature (convection patterns)
        let hf_vel_cell = vec2<i32>(i32(world_x), i32(world_y));
        let vel_raw = textureLoad(fluid_vel_tex, hf_vel_cell, 0).xy;
        let vel_mag = length(vel_raw);
        let temp_hf = smoke.a;
        let temp_delta_hf = temp_hf - 15.0; // delta from ~ambient

        // Background: dim terrain
        var hf_color = color * 0.2;

        if vel_mag > 0.5 {
            // Direction as hue, temperature as saturation, magnitude as brightness
            let dir_angle = atan2(vel_raw.y, vel_raw.x);
            let hue = (dir_angle / 6.283 + 0.5); // 0..1

            // Temperature coloring: cool flow = blue, warm flow = orange/red
            var flow_color: vec3<f32>;
            if temp_delta_hf > 5.0 {
                // Hot convection: orange → yellow
                let heat = clamp(temp_delta_hf / 200.0, 0.0, 1.0);
                flow_color = mix(vec3(1.0, 0.5, 0.1), vec3(1.0, 1.0, 0.3), heat);
            } else if temp_delta_hf < -3.0 {
                // Cold flow: blue → cyan
                let cold = clamp(-temp_delta_hf / 20.0, 0.0, 1.0);
                flow_color = mix(vec3(0.3, 0.5, 1.0), vec3(0.1, 0.8, 1.0), cold);
            } else {
                // Neutral flow: white/gray
                flow_color = vec3(0.7, 0.7, 0.7);
            }

            let brightness = clamp(vel_mag * 0.03, 0.0, 1.0);
            hf_color = mix(hf_color, flow_color, brightness);
        }

        color = hf_color;
    }

    // Final fog overlay: covers EVERYTHING (terrain + gas) in the border zone
    color = mix(vec3(0.12, 0.12, 0.15), color, border_fade);

    textureStore(output, vec2<u32>(px, py), vec4<f32>(color, 1.0));
}
