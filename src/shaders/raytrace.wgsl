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

// --- Pleb struct (must match Rust GpuPleb layout exactly) ---
struct GpuPleb {
    x: f32, y: f32, angle: f32, selected: f32,
    torch: f32, headlight: f32, carrying: f32, _pad1: f32,
    skin_r: f32, skin_g: f32, skin_b: f32, hair_style: f32,
    hair_r: f32, hair_g: f32, hair_b: f32, _pad2: f32,
    shirt_r: f32, shirt_g: f32, shirt_b: f32, _pad3: f32,
    pants_r: f32, pants_g: f32, pants_b: f32, _pad4: f32,
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
    return materials[min(bt, 47u)];
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
const SPRITE_SIZE: u32 = 16u;
const SPRITE_VARIANTS: u32 = 8u;

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
const SHADOW_STEP: f32 = 0.20;

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
        if sbt >= 15u && sbt <= 20u { continue; } // pipe components don't block light
        if sbt == 32u { continue; } // dug ground doesn't block light
        if sbt == 36u { continue; } // wire (height = connection mask, not visual)
        if sbt == 43u { continue; } // dimmer (height = level, not visual)
        if sbt == 45u { continue; } // breaker (height = threshold, not visual)

        // Doors: open = pass through, closed = block (regardless of height)
        if is_door(sb) {
            if is_open(sb) { continue; } else { return 0.0; }
        }

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

            // Electric lights: OFF unless powered above threshold
            if bt == 7u || bt == 10u || bt == 11u {
                let light_idx = u32(ny) * u32(camera.grid_w) + u32(nx);
                let lv = voltage[light_idx];
                if lv < 2.0 {
                    // Below threshold: completely off
                    intensity = 0.0;
                } else {
                    // Above threshold: scale from dim to full (2V=dim, 8V+=full)
                    let power_factor = clamp((lv - 2.0) / 6.0, 0.0, 1.0);
                    intensity *= power_factor;
                    // Flicker when marginal power (2-4V)
                    if lv < 4.0 {
                        let pf = sin(time * 15.0 + f32(light_idx) * 3.7) * 0.3 + 0.7;
                        intensity *= pf;
                    }
                }
            }

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

    // Full 360° continuous rotation
    let rot_hash = fract(sin(tree_id * 1.7 + 5.3) * 27183.6142);
    let angle = rot_hash * 6.2832; // full 360 degrees
    var ru = sprite_u - 0.5;
    var rv = sprite_v - 0.5;
    let cos_a = cos(angle);
    let sin_a = sin(angle);
    let rotated_u = ru * cos_a - rv * sin_a;
    let rotated_v = ru * sin_a + rv * cos_a;
    ru = rotated_u;
    rv = rotated_v;
    let sprite = sample_sprite(variant, ru + 0.5, rv + 0.5);

    if sprite.w < 0.01 {
        // Transparent: show ground
        return vec4<f32>(0.45, 0.35, 0.20, 0.0);
    }

    // Trunk vs canopy: trunk pixels are low height (< 0.4), canopy is high (> 0.5)
    let is_trunk = sprite.w < 0.4;
    let color = sprite.xyz * (0.85 + id_hash * 0.3);
    if is_trunk {
        // Trunk: darker, at ground level (low height doesn't cast shadows)
        return vec4<f32>(color * 0.7, 0.05); // very low height = ground level
    } else {
        // Canopy: full color and height
        return vec4<f32>(color, sprite.w);
    }
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

        // Pipe components (15-20), dug ground (32), crates (33), rocks (34) don't cast shadows
        let is_pipe_block = bt >= 15u && bt <= 20u;
        let is_dug_block = bt == 32u;
        let is_crate_block = bt == 33u; // height = item count, not visual
        let is_rock_block = bt == 34u;
        let is_wire_block = bt == 36u; // height = connection mask, not visual
        let is_dimmer_block = bt == 43u; // height = dimmer level, not visual
        let is_breaker_block = bt == 45u; // height = trip threshold, not visual

        // Diagonal wall: only occlude if ray is on the wall half
        let is_diag_block = bt == 44u;
        var diag_open = false;
        if is_diag_block {
            let sfx = fract(sx);
            let sfy = fract(sy);
            let svar = (block_flags(block) >> 3u) & 3u;
            diag_open = !diag_is_wall(sfx, sfy, svar);
        }

        var effective_h = select(bh, 0.0, is_pipe_block || is_dug_block || is_crate_block || is_rock_block || is_wire_block || is_dimmer_block || is_breaker_block || diag_open);
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
    let mat = get_material(btype);
    let is_exterior_south = bheight > south_h && mat.shows_wall_face > 0.5
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

        // Glass face: window between sill and lintel with frame detail
        if btype == 5u {
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
        if btype == 14u {
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
        if btype == 35u {
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

    // Pre-pass: draw wire connections underneath power equipment
    // For battery, solar, wind, switch, dimmer — show wire entering from adjacent wire blocks
    let is_power_equip = btype == 37u || btype == 38u || btype == 39u || btype == 40u
        || btype == 41u || btype == 42u || btype == 43u;
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
    } else if btype >= 15u && btype <= 20u {
        // Piping system: auto-connected thin pipe rendering
        // Inlet/outlet on walls: use wall-like background. Ground pipes: use dirt.
        if (btype == 19u || btype == 20u) && bheight > 1u {
            color = block_base_color(1u, 0u); // stone wall background for wall-mounted
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
        var cn = n_n >= 15u && n_n <= 20u;
        var cs = n_s >= 15u && n_s <= 20u;
        var ce = n_e >= 15u && n_e <= 20u;
        var cw = n_w >= 15u && n_w <= 20u;
        if pipe_conn_mask != 0u && btype == 15u {
            cn = cn && (pipe_conn_mask & 1u) != 0u; // N
            ce = ce && (pipe_conn_mask & 2u) != 0u; // E
            cs = cs && (pipe_conn_mask & 4u) != 0u; // S
            cw = cw && (pipe_conn_mask & 8u) != 0u; // W
        }

        let cx = fx - 0.5;
        let cy = fy - 0.5;

        if btype == 16u {
            // --- Pump: square block connected to pipes ---
            let pump_r = 0.30;
            let pipe_r = 0.10;
            // Draw pipe stubs connecting to neighbors BEHIND the pump
            if cn && abs(cx) < pipe_r && cy < -pump_r { color = vec3(0.50, 0.52, 0.55); }
            if cs && abs(cx) < pipe_r && cy > pump_r { color = vec3(0.50, 0.52, 0.55); }
            if ce && abs(cy) < pipe_r && cx > pump_r { color = vec3(0.50, 0.52, 0.55); }
            if cw && abs(cy) < pipe_r && cx < -pump_r { color = vec3(0.50, 0.52, 0.55); }
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
        } else if btype == 17u {
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
                if cn && abs(cx) < 0.06 && cy < -tank_ry + 0.1 { color = vec3(0.50, 0.52, 0.55); }
                if cs && abs(cx) < 0.06 && cy > tank_ry - 0.1 { color = vec3(0.50, 0.52, 0.55); }
                if ce && abs(cy) < 0.06 && cx > tank_rx - 0.1 { color = vec3(0.50, 0.52, 0.55); }
                if cw && abs(cy) < 0.06 && cx < -tank_rx + 0.1 { color = vec3(0.50, 0.52, 0.55); }

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
            // --- Pipe / Pump / Valve / Outlet / Inlet: thin round pipe ---
            let pipe_r = 0.10; // pipe radius (thin)
            var on_pipe = false;
            var pipe_dist = 1.0; // distance from pipe center (for rounded shading)

            // Horizontal segment (E-W)
            if ce || cw {
                let x_min = select(-pipe_r, -0.5, cw);
                let x_max = select(pipe_r, 0.5, ce);
                if cx >= x_min && cx <= x_max && abs(cy) < pipe_r {
                    on_pipe = true;
                    pipe_dist = abs(cy) / pipe_r;
                }
            }
            // Vertical segment (N-S)
            if cn || cs {
                let y_min = select(-pipe_r, -0.5, cn);
                let y_max = select(pipe_r, 0.5, cs);
                if cy >= y_min && cy <= y_max && abs(cx) < pipe_r {
                    on_pipe = true;
                    pipe_dist = min(pipe_dist, abs(cx) / pipe_r);
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
                let pipe_base = vec3<f32>(0.40, 0.42, 0.46);
                let pipe_bright = vec3<f32>(0.62, 0.65, 0.70);
                color = mix(pipe_base, pipe_bright, shade);

                // Traveling circles along pipes — visible gray dots moving with flow
                let anim_speed = camera.time * 2.0;
                let dot_r = pipe_r * 0.6; // radius of each traveling dot
                // Horizontal pipe dots
                if (ce || cw) && abs(cy) < pipe_r {
                    // 3 dots spread across the tile, traveling east
                    for (var di: i32 = 0; di < 3; di++) {
                        let dot_x = fract(anim_speed + f32(di) * 0.333 + world_x * 0.1) - 0.5;
                        let d = length(vec2(cx - dot_x, cy));
                        if d < dot_r {
                            color = mix(color, vec3(0.28, 0.30, 0.33), 0.7);
                        }
                    }
                }
                // Vertical pipe dots
                if (cn || cs) && abs(cx) < pipe_r {
                    for (var di: i32 = 0; di < 3; di++) {
                        let dot_y = fract(anim_speed + f32(di) * 0.333 + world_y * 0.1) - 0.5;
                        let d = length(vec2(cx, cy - dot_y));
                        if d < dot_r {
                            color = mix(color, vec3(0.28, 0.30, 0.33), 0.7);
                        }
                    }
                }

                // Valve overlay
                if btype == 18u {
                    let valve_open = is_open(block);
                    let vc = select(vec3(0.65, 0.15, 0.15), vec3(0.15, 0.55, 0.15), valve_open);
                    let bar_along = select(abs(cy), abs(cx), ce || cw);
                    let bar_perp = select(abs(cx), abs(cy), ce || cw);
                    if bar_along < pipe_r * 1.8 && bar_perp < 0.04 {
                        color = vc;
                    }
                    if cdist < 0.04 { color = vc; }
                }
            }
            // Inlet/Outlet: rendered AFTER and ON TOP of everything (overlays wall sprite)
            if btype == 19u || btype == 20u {
                let is_outlet = btype == 19u;
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
    } else if btype == 30u {
        // Bed: 2-tile piece
        color = render_bed(fx, fy, bflags);
    } else if btype == 31u {
        // Berry bush: leafy mound with berries
        let bush_result = render_berry_bush(fx, fy, world_x, world_y, camera.time);
        color = bush_result.xyz;
        is_tree_pixel = bush_result.w > 0.01;
    } else if btype == 32u {
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
    } else if btype == 33u {
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
    } else if btype == 34u {
        // Rock: dark natural stone on dirt, irregular shape with outline
        let ground = vec3<f32>(0.42, 0.35, 0.22);
        // Irregular shape: offset center + multi-frequency noise distortion
        let rcx = fx - 0.48;
        let rcy = fy - 0.52;
        let n1 = fract(sin(world_x * 53.1 + world_y * 97.3) * 43758.5) * 0.06;
        let n2 = fract(sin(world_x * 127.7 - world_y * 43.1) * 23421.6) * 0.04;
        let n3 = fract(sin(world_x * 31.3 + world_y * 171.9) * 61283.1) * 0.03;
        let angle_warp = n1 + n2 - 0.05;
        let rock_dist = length(vec2<f32>(rcx * 1.4 + n3 - 0.015, rcy * 1.1 + angle_warp));
        let outline_r = 0.30;
        let fill_r = 0.26;
        if rock_dist < outline_r {
            if rock_dist < fill_r {
                // Rock interior: dark gray with variation
                let rvar = fract(sin(world_x * 127.1 + world_y * 311.7) * 43758.5) * 0.06 - 0.03;
                let rvar2 = fract(sin(world_x * 73.7 + world_y * 199.3) * 17654.3) * 0.04 - 0.02;
                color = vec3<f32>(0.32 + rvar, 0.30 + rvar + rvar2, 0.28 + rvar);
                // Edge darkening (AO)
                let edge_dark = smoothstep(fill_r, fill_r * 0.4, rock_dist);
                color *= 0.7 + edge_dark * 0.3;
                // Specular highlight (upper-left)
                let spec_spot = length(vec2<f32>(rcx + 0.08, rcy + 0.10));
                if spec_spot < 0.09 {
                    color += vec3<f32>(0.08, 0.07, 0.06) * (1.0 - spec_spot / 0.09);
                }
                // Subtle cracks
                let crack = abs(fract(rcx * 7.0 + rcy * 3.0 + n1 * 5.0) - 0.5);
                if crack < 0.04 && rock_dist < fill_r * 0.8 {
                    color *= 0.85;
                }
            } else {
                // Dark outline ring
                color = vec3<f32>(0.18, 0.17, 0.15);
            }
        } else {
            color = ground;
        }
    } else if btype == 36u {
        // Wire: copper conductor with directional segments
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
            let pwr_w = n_w == 36u || n_w == 37u || n_w == 38u || n_w == 39u || n_w == 40u || n_w == 41u || n_w == 42u || n_w == 43u || n_w == 45u || n_w == 7u || n_w == 12u || n_w == 10u;
            let pwr_e = n_e == 36u || n_e == 37u || n_e == 38u || n_e == 39u || n_e == 40u || n_e == 41u || n_e == 42u || n_e == 43u || n_e == 45u || n_e == 7u || n_e == 12u || n_e == 10u;
            let pwr_n = n_n == 36u || n_n == 37u || n_n == 38u || n_n == 39u || n_n == 40u || n_n == 41u || n_n == 42u || n_n == 43u || n_n == 45u || n_n == 7u || n_n == 12u || n_n == 10u;
            let pwr_s = n_s == 36u || n_s == 37u || n_s == 38u || n_s == 39u || n_s == 40u || n_s == 41u || n_s == 42u || n_s == 43u || n_s == 45u || n_s == 7u || n_s == 12u || n_s == 10u;
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
            color = wire_col;
        } else {
            color = ground;
        }
    } else if btype == 37u {
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
    } else if btype == 38u {
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
    } else if btype == 39u {
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
    } else if btype == 40u {
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
    } else if btype == 41u {
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
    } else if btype == 42u {
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
    } else if btype == 43u {
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
    } else if btype == 45u {
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
    } else if btype == 14u {
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
    } else if btype == 35u {
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
    } else if btype == 44u {
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
    } else {
        color = block_base_color(btype, bflags);
    }

    // Open door: treat as floor-level opening (overrides wall type)
    let door_is_open = is_door(block) && is_open(block);
    // Trees: transparent sprite pixels are ground-level; canopy keeps height for shadows
    let is_tree_ground = (btype == 8u || btype == 31u) && !is_tree_pixel;
    let is_pipe = btype >= 15u && btype <= 20u;
    let is_dug = btype == 32u; // dug ground: height = depth, not visual height
    let is_rock = btype == 34u;
    let is_crate = btype == 33u; // crate height = item count, not visual height
    let is_wire = btype == 36u; // wire height = connection mask, not visual
    let is_dimmer = btype == 43u; // dimmer height = level, not visual height
    let is_breaker = btype == 45u; // breaker height = threshold, not visual
    let is_diag_open = btype == 44u && !diag_is_wall(fx, fy, (bflags >> 3u) & 3u);
    let effective_height = select(bheight, 0u, door_is_open || is_tree_ground || is_pipe || is_dug || is_rock || is_crate || is_wire || is_dimmer || is_breaker || is_diag_open);
    let effective_fheight = f32(effective_height);

    // Height-based brightness (skip for trees — they have their own shading)
    if btype != 8u {
        color += vec3<f32>(effective_fheight * 0.03);
    }

    // Wet soil darkening: outdoor ground tiles darken in rain
    let is_ground_tile = btype == 2u || btype == 26u || btype == 27u || btype == 28u;
    if is_ground_tile && effective_height == 0u && camera.rain_intensity > 0.0 {
        let roof_h_wet = (block >> 24u) & 0xFFu;
        if roof_h_wet == 0u { // outdoor only
            let wet = camera.rain_intensity * 0.7;
            color *= 1.0 - wet * 0.3;
            // Slight blue tint from water
            color = mix(color, vec3(color.r * 0.7, color.g * 0.75, color.b * 0.9), wet * 0.15);
        }
    }

    // Wall side faces (3D bevel) — skip for doors and trees
    if effective_height > 0u && btype != 8u && btype != 44u {
        color += wall_side_shade(world_x, world_y, bx, by, effective_height);
    }

    // Wire overlay: draw wire on top of walls/blocks that have wire flag (bit 7)
    let has_wire_flag = (bflags & 0x80u) != 0u;
    if has_wire_flag && btype != 36u {
        // Same wire rendering logic as standalone wire, but overlaid
        let wn_w = block_type(get_block(bx - 1, by));
        let wn_e = block_type(get_block(bx + 1, by));
        let wn_n = block_type(get_block(bx, by - 1));
        let wn_s = block_type(get_block(bx, by + 1));
        let wf_w = (get_block(bx - 1, by) >> 16u) & 0x80u;
        let wf_e = (get_block(bx + 1, by) >> 16u) & 0x80u;
        let wf_n = (get_block(bx, by - 1) >> 16u) & 0x80u;
        let wf_s = (get_block(bx, by + 1) >> 16u) & 0x80u;
        let wc_w = wn_w == 36u || wn_w == 37u || wn_w == 38u || wf_w != 0u;
        let wc_e = wn_e == 36u || wn_e == 37u || wn_e == 38u || wf_e != 0u;
        let wc_n = wn_n == 36u || wn_n == 37u || wn_n == 38u || wf_n != 0u;
        let wc_s = wn_s == 36u || wn_s == 37u || wn_s == 38u || wf_s != 0u;
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
        // Outdoor pixel: single shadow ray with interleaved gradient noise jitter.
        // IGN has blue-noise-like spectral properties — distributes error evenly
        // across the image with no clumping or visible patterns. Much smoother
        // than white noise (fract(sin(...))), combined with temporal accumulation.
        // Reference: Jorge Jimenez, "Next Generation Post Processing in Call of Duty"
        let frame_offset = fract(camera.time * 5.3) * 5.0;
        let ign_x = f32(px) + frame_offset;
        let ign_y = f32(py) + frame_offset * 0.7;
        let ign1 = fract(52.9829189 * fract(0.06711056 * ign_x + 0.00583715 * ign_y));
        let ign2 = fract(52.9829189 * fract(0.00583715 * ign_x + 0.06711056 * ign_y));
        let shadow_result = trace_shadow_ray(
            world_x + (ign1 - 0.5) * 0.18,
            world_y + (ign2 - 0.5) * 0.18,
            effective_fheight, sun_dir, sun_elev);
        shadow_tint = shadow_result.xyz;
        light_factor = shadow_result.w;

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

    // Water effect (type 3 = water, type 32 depth>=1 = ground water at 20%+)
    let is_water = btype == 3u || (btype == 32u && bheight >= 1u);
    if is_water {
        let t = camera.time;
        // Multi-octave ripple normals
        let rip1 = sin(world_x * 11.0 + world_y * 7.0 + t * 2.0);
        let rip2 = sin(world_x * 5.3 - world_y * 13.0 + t * 1.3);
        let rip3 = sin(world_x * 23.0 + world_y * 3.0 + t * 3.1) * 0.5;
        let ripple = (rip1 + rip2 + rip3) * 0.02;

        // Depth-dependent base color (deeper = darker blue)
        let depth_factor = select(1.0, f32(bheight) / 5.0, btype == 32u);
        let shallow = vec3<f32>(0.15, 0.38, 0.55);
        let deep = vec3<f32>(0.06, 0.18, 0.40);
        var water_color = mix(shallow, deep, depth_factor * 0.7);

        // Animated caustic patterns (sun-modulated)
        let caust1 = abs(sin(world_x * 17.0 + t * 0.7) * sin(world_y * 19.0 + t * 0.9));
        let caust2 = abs(sin(world_x * 11.0 - t * 0.5) * sin(world_y * 13.0 + t * 1.1));
        let caustic = caust1 * caust2;
        water_color += vec3<f32>(0.02, 0.06, 0.10) * caustic * light_factor;

        // Sky reflection (simple fresnel approximation from above)
        let sky_color = vec3<f32>(0.5, 0.6, 0.8) * (camera.sun_intensity * 0.5 + 0.1);
        let reflect_amount = 0.15 + ripple * 2.0;
        water_color = mix(water_color, sky_color, clamp(reflect_amount, 0.0, 0.3));

        // Specular highlight from sun
        let spec = pow(max(rip1 * 0.5 + 0.5, 0.0), 16.0) * camera.sun_intensity * 0.3;
        water_color += vec3<f32>(spec);

        // Surface shimmer
        water_color += vec3<f32>(ripple * 0.3, ripple * 0.5, ripple * 0.8);

        // Edge darkening (shoreline)
        if btype == 3u {
            let edge_dist = min(min(fx, 1.0 - fx), min(fy, 1.0 - fy));
            // Check neighbors for non-water
            let n_n = block_type(get_block(bx, by - 1));
            let n_s = block_type(get_block(bx, by + 1));
            let n_e = block_type(get_block(bx + 1, by));
            let n_w = block_type(get_block(bx - 1, by));
            let shore_n = f32(n_n != 3u) * smoothstep(0.3, 0.0, fy);
            let shore_s = f32(n_s != 3u) * smoothstep(0.7, 1.0, fy);
            let shore_e = f32(n_e != 3u) * smoothstep(0.7, 1.0, fx);
            let shore_w = f32(n_w != 3u) * smoothstep(0.3, 0.0, fx);
            let shore = max(max(shore_n, shore_s), max(shore_e, shore_w));
            water_color = mix(water_color, vec3<f32>(0.30, 0.28, 0.22), shore * 0.5);
        }

        // Apply the same lighting as terrain (ambient + sun), so water darkens at night
        water_color = water_color * (ambient + sun_color * light_factor * 0.85);
        // Add point light contribution (torches, lamps illuminate water at night)
        let water_pl_mul = select(camera.light_bleed_mul, camera.indoor_glow_mul, is_indoor);
        water_color += light_color_out * light_intensity_out * water_pl_mul * 0.5;
        color = water_color;
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

    // --- Pleb rendering (all plebs from buffer) ---
    for (var pi: u32 = 0u; pi < MAX_PLEBS; pi++) {
        let p = plebs[pi];
        if p.x < 0.5 && p.y < 0.5 { continue; } // empty slot

        let pdx = world_x - p.x;
        let pdy = world_y - p.y;
        let pdist = length(vec2(pdx, pdy));

        // Skip pleb lights on wall tops / elevated surfaces — light is at ground level
        let is_elevated = effective_height > 0u;

        // Torch: warm point light with wall occlusion
        if p.torch > 0.5 && pdist < 6.0 && !is_elevated {
            let vis = trace_glow_visibility(world_x, world_y, p.x, p.y, 1.0);
            if vis > 0.01 {
                let torch_atten = 1.0 / (1.0 + pdist * 0.4 + pdist * pdist * 0.08);
                let flicker = sin(camera.time * 8.3 + f32(pi) * 2.0) * 0.15 + sin(camera.time * 13.1) * 0.1 + 0.85;
                color += vec3(1.0, 0.55, 0.15) * torch_atten * 0.5 * flicker * vis;
            }
        }

        // Headlight: directional cone with wall occlusion
        if p.headlight > 0.5 && pdist > 0.5 && pdist < 10.0 && !is_elevated {
            let vis = trace_glow_visibility(world_x, world_y, p.x, p.y, 1.5);
            if vis > 0.01 {
                let to_pixel = normalize(vec2(pdx, pdy));
                let light_dir = vec2(cos(p.angle), sin(p.angle));
                let cone = smoothstep(0.4, 0.85, dot(to_pixel, light_dir));
                let dist_atten = 1.0 / (1.0 + pdist * 0.2 + pdist * pdist * 0.04);
                color += vec3(0.85, 0.9, 1.0) * cone * dist_atten * 0.6 * vis;
            }
        }

        // Body rendering — matches portrait (shirt body, skin head, hair)
        if pdist < 0.45 {
            // Selection ring (pulsing green)
            if p.selected > 0.5 {
                let ring_inner = 0.38;
                let ring_outer = 0.44;
                if pdist > ring_inner && pdist < ring_outer {
                    let pulse = sin(camera.time * 4.0) * 0.3 + 0.7;
                    color = mix(color, vec3(0.3, 0.9, 0.3), pulse);
                }
            }

            // Pants (lower body) — outer ring
            if pdist < 0.35 && pdist > 0.20 {
                let shade = 1.0 - (pdist - 0.20) / 0.15 * 0.3;
                color = vec3(p.pants_r, p.pants_g, p.pants_b) * shade;
            }

            // Shirt (upper body) — middle area
            if pdist < 0.28 {
                let shade = 1.0 - pdist / 0.28 * 0.2;
                color = vec3(p.shirt_r, p.shirt_g, p.shirt_b) * shade;
            }

            // Head (skin) — offset slightly north (toward facing direction)
            let head_offset = vec2(cos(p.angle) * 0.08, sin(p.angle) * 0.08);
            let head_dist = length(vec2(pdx - head_offset.x, pdy - head_offset.y));
            if head_dist < 0.14 {
                let head_shade = 1.0 - head_dist / 0.14 * 0.15;
                color = vec3(p.skin_r, p.skin_g, p.skin_b) * head_shade;
            }

            // Hair (on top of head, offset further in facing direction)
            let hair_offset = vec2(cos(p.angle) * 0.14, sin(p.angle) * 0.14);
            let hair_dist = length(vec2(pdx - hair_offset.x, pdy - hair_offset.y));
            let hair_r = select(0.08, 0.12, p.hair_style > 1.5); // longer hair = bigger
            if hair_dist < hair_r {
                color = vec3(p.hair_r, p.hair_g, p.hair_b);
            }

            // Carried rock: small dark stone sprite offset above head
            if p.carrying > 0.5 {
                let carry_ox = cos(p.angle) * 0.05;
                let carry_oy = sin(p.angle) * 0.05 - 0.18; // offset above head
                let carry_dx = pdx - carry_ox;
                let carry_dy = pdy - carry_oy;
                let carry_dist = length(vec2(carry_dx * 1.3, carry_dy));
                if carry_dist < 0.10 {
                    let rv = fract(sin(world_x * 53.1 + world_y * 97.3) * 43758.5) * 0.04;
                    color = vec3(0.30 + rv, 0.28 + rv, 0.26 + rv);
                    if carry_dist > 0.07 {
                        color = vec3(0.16, 0.15, 0.13); // outline
                    }
                }
            }

            // Direction indicator: small bright dot at front edge
            let front_x = p.x + cos(p.angle) * 0.30;
            let front_y = p.y + sin(p.angle) * 0.30;
            let front_dist = length(vec2(world_x - front_x, world_y - front_y));
            if front_dist < 0.05 {
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
        let is_pipe_block_t = bt_for_temp >= 15u && bt_for_temp <= 20u;
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
        } else if temp_norm < 0.85 {
            // Very hot: deep red (60 to 100°C)
            let t = (temp_norm - 0.70) / 0.15;
            temp_color = mix(vec3(1.0, 0.45, 0.1), vec3(0.85, 0.1, 0.1), t);
        } else {
            // Extreme: red to bright white-yellow (100 to 500°C)
            let t = (temp_norm - 0.85) / 0.15;
            temp_color = mix(vec3(0.85, 0.1, 0.1), vec3(1.0, 1.0, 0.6), t);
        }
        color = mix(color * 0.3, temp_color, 0.7);
    } else if camera.fluid_overlay < 9.5 {
        // Power overlay: show voltage, highlight infrastructure, dim terrain
        let grid_idx_p = u32(by) * u32(camera.grid_w) + u32(bx);
        let v = voltage[grid_idx_p];
        let norm_v = clamp(v / 12.0, 0.0, 1.0);
        // Is this block part of the power grid?
        let is_pwr_infra = btype == 36u || btype == 37u || btype == 38u || btype == 39u
            || btype == 40u || btype == 41u || btype == 42u || btype == 43u || btype == 45u
            || btype == 7u || btype == 10u || btype == 11u || btype == 12u || btype == 16u
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
        let is_amp_infra = btype == 36u || btype == 37u || btype == 38u || btype == 39u
            || btype == 40u || btype == 41u || btype == 42u || btype == 43u
            || btype == 7u || btype == 10u || btype == 11u || btype == 12u || btype == 16u
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
        // Watts overlay: generation (green) vs consumption (red)
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

            // Per-block arrow showing flow direction
            let hf_fx = fract(world_x) - 0.5;
            let hf_fy = fract(world_y) - 0.5;
            let hf_dir = vel_raw / vel_mag;
            let hf_along = hf_fx * hf_dir.x + hf_fy * hf_dir.y;
            let hf_perp = abs(-hf_fx * hf_dir.y + hf_fy * hf_dir.x);
            let hf_arrow_len = clamp(vel_mag * 0.02, 0.1, 0.4);
            let hf_on_shaft = hf_along > -0.05 && hf_along < hf_arrow_len && hf_perp < 0.06;
            let hf_head_t = (hf_along - hf_arrow_len + 0.12) / 0.12;
            let hf_on_head = hf_head_t > 0.0 && hf_head_t < 1.0 && hf_perp < 0.15 * (1.0 - hf_head_t);
            if hf_on_shaft || hf_on_head {
                hf_color = mix(hf_color, flow_color * 1.5, 0.8);
            }
        }

        color = hf_color;
    }

    // Rain overlay: animated streaks
    if camera.rain_intensity > 0.01 {
        let rain_speed = 25.0;
        let streak_seed = world_x * 3.7 + world_y * 0.3;
        let streak_x = fract(streak_seed + camera.time * 0.02);
        let streak_y = fract(world_y * 2.0 + camera.time * rain_speed * 0.08);
        let streak = smoothstep(0.92, 1.0, streak_y) * smoothstep(0.03, 0.0, abs(streak_x - 0.5));
        // Add a second layer of smaller streaks
        let streak2_x = fract(world_x * 7.1 + world_y * 0.7 + camera.time * 0.03 + 0.5);
        let streak2_y = fract(world_y * 3.0 + camera.time * rain_speed * 0.12 + 0.3);
        let streak2 = smoothstep(0.94, 1.0, streak2_y) * smoothstep(0.02, 0.0, abs(streak2_x - 0.5));
        let rain_alpha = (streak + streak2 * 0.6) * camera.rain_intensity * 0.35;
        let rain_color = vec3(0.65, 0.70, 0.80);
        color = mix(color, rain_color, clamp(rain_alpha, 0.0, 0.4));
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

    textureStore(output, vec2<u32>(px, py), vec4<f32>(color, 1.0));
}
