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
};

@group(0) @binding(0) var output: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<uniform> camera: Camera;
@group(0) @binding(2) var<storage, read> grid: array<u32>;
@group(0) @binding(3) var lightmap_tex: texture_2d<f32>;
@group(0) @binding(4) var lightmap_sampler: sampler;

// --- Lightmap sampling ---
// Sample the pre-computed lightmap at a world position.
// Returns vec4(light_color_rgb, light_intensity) with bilinear interpolation.
fn sample_lightmap(wx: f32, wy: f32) -> vec4<f32> {
    let uv = vec2<f32>(wx / camera.grid_w, wy / camera.grid_h);
    return textureSampleLevel(lightmap_tex, lightmap_sampler, uv, 0.0);
}

// --- Block unpacking ---
// type: 0=air, 1=stone, 2=dirt, 3=water, 4=wall, 5=glass, 6=fireplace, 7=electric_light
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
// A block is "under a roof" if it has the roof flag, OR if it's a wall/glass/door
// block that has a roofed neighbor (i.e., it's part of a roofed building).
// Returns the roof height (0 = no roof).
fn get_roof_height(bx: i32, by: i32) -> f32 {
    let block = get_block(bx, by);

    // If the block itself is a roofed floor tile, find roof height from nearby walls
    if has_roof(block) {
        var rh: f32 = 0.0;
        for (var dy: i32 = -2; dy <= 2; dy++) {
            for (var dx: i32 = -2; dx <= 2; dx++) {
                let nb = get_block(bx + dx, by + dy);
                if block_height(nb) > 0u && !has_roof(nb) {
                    rh = max(rh, f32(block_height(nb)));
                }
            }
        }
        if rh < 1.0 { rh = 2.0; }
        return rh;
    }

    // If the block is a wall/glass/door with height, check if any neighbor is roofed
    // (meaning this block is part of a roofed building's envelope)
    // Exception: open doors are openings, not part of the roof envelope
    if block_height(block) > 0u && !(is_door(block) && is_open(block)) {
        let bh = f32(block_height(block));
        for (var dy: i32 = -1; dy <= 1; dy++) {
            for (var dx: i32 = -1; dx <= 1; dx++) {
                if dx == 0 && dy == 0 { continue; }
                let nb = get_block(bx + dx, by + dy);
                if has_roof(nb) {
                    // This wall/glass is part of a roofed building
                    return bh;
                }
            }
        }
    }

    return 0.0;
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
    let night_ambient = vec3<f32>(0.04, 0.04, 0.08);
    let day_ambient = vec3<f32>(0.14, 0.14, 0.18);
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
const INTERIOR_INDIRECT: f32 = 0.18;
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

    var sx = wx;
    var sy = wy;
    var tint = vec3<f32>(1.0);
    var light = 1.0;

    // Walk until we exit the building (hit a non-roofed tile) or run out of steps
    let max_steps = 64; // ~16 blocks at 0.25 step
    for (var i: i32 = 0; i < max_steps; i++) {
        sx += step_x;
        sy += step_y;

        let bx = i32(floor(sx));
        let by = i32(floor(sy));
        let block = get_block(bx, by);
        let bt = block_type(block);
        let bh = block_height(block);

        // Still on a roofed floor tile — ray is in interior airspace, continue
        if has_roof(block) && bh == 0u {
            continue;
        }

        // Fireplace or electric light: low interior block, ray passes over it
        if bt == 6u || bt == 7u {
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
                // Open door: ray passes through freely
                continue;
            }
            light *= 0.4; // closed door blocks most light
            continue;
        }

        // Hit any solid block (wall, stone, etc.) — ray is blocked
        if bh > 0u {
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
const GLOW_RADIUS: f32 = 8.0;
const FIRE_GLOW_INTENSITY: f32 = 0.70;
const ELIGHT_GLOW_INTENSITY: f32 = 0.85;

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
// Returns 1.0 if clear, 0.0 if blocked by a wall, partial for glass.
fn trace_glow_visibility(x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
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

        // Skip the source block itself (light sources have height)
        if sbt == 6u || sbt == 7u { continue; }

        if sbh == 0u { continue; } // open floor

        // Glass: partial transmission
        if sbt == 5u {
            vis *= 0.5;
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
    let search = i32(GLOW_RADIUS);
    let bx = i32(floor(wx));
    let by = i32(floor(wy));

    for (var dy: i32 = -search; dy <= search; dy++) {
        for (var dx: i32 = -search; dx <= search; dx++) {
            let nx = bx + dx;
            let ny = by + dy;
            let nb = get_block(nx, ny);
            let bt = block_type(nb);

            if bt != 6u && bt != 7u {
                continue;
            }

            let lcx = f32(nx) + 0.5;
            let lcy = f32(ny) + 0.5;
            let fdx = wx - lcx;
            let fdy = wy - lcy;
            let dist = sqrt(fdx * fdx + fdy * fdy);

            if dist > GLOW_RADIUS { continue; }

            // Check wall occlusion between this pixel and the light
            let vis = trace_glow_visibility(wx, wy, lcx, lcy);
            if vis < 0.01 { continue; }

            // Inverse-square-ish falloff with smooth range fade
            let atten = (1.0 / (1.0 + dist * 0.6 + dist * dist * 0.15))
                      * smoothstep(GLOW_RADIUS, GLOW_RADIUS * 0.4, dist);

            if bt == 6u {
                let phase = fire_hash(vec2<f32>(lcx, lcy)) * 6.28;
                let flicker = fire_flicker(time + phase);
                let intensity = FIRE_GLOW_INTENSITY * (0.7 + 0.3 * flicker);
                let heat = clamp(1.0 - dist / 3.0, 0.0, 1.0);
                let col = mix(FIRE_COLOR, FIRE_COLOR_HOT, heat * flicker);
                glow += col * intensity * atten * vis;
            } else {
                glow += ELIGHT_COLOR * ELIGHT_GLOW_INTENSITY * atten * vis;
            }
        }
    }

    return glow;
}

// Render fireplace block from top-down: stone hearth with animated fire
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
        default: { return vec3<f32>(1.0, 0.0, 1.0); }
    }
}

// Roof color with tile pattern
fn roof_color(wx: f32, wy: f32) -> vec3<f32> {
    let base = vec3<f32>(0.55, 0.35, 0.25); // terracotta
    let tile_x = fract(wx * 2.0);
    let tile_y = fract(wy * 2.0);
    let tile_edge = f32(tile_x < 0.06 || tile_y < 0.06) * 0.08;
    let row = floor(wy * 2.0);
    let offset = fract(row * 0.5) * 0.5;
    let shifted_x = fract(wx * 2.0 + offset);
    let shingle_edge = f32(shifted_x < 0.06) * 0.05;
    return base - vec3<f32>(tile_edge + shingle_edge);
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

    let block = get_block(bx, by);
    let btype = block_type(block);
    let bheight = block_height(block);
    let bflags = block_flags(block);
    let fheight = f32(bheight);

    // Compute sun parameters for this frame
    let sun_info = get_sun(camera.time);
    let sun_dir = vec2<f32>(sun_info.x, sun_info.y);
    let sun_elev = get_sun_elevation(camera.time);
    let sun_color = get_sun_color(camera.time);
    let ambient = get_ambient(camera.time);

    // --- Determine if this pixel is covered by a roof ---
    let roof_h = get_roof_height(bx, by);
    let is_roofed = roof_h > 0.5;

    // --- If roofed AND show_roofs is on, render the roof surface (hides everything below) ---
    if is_roofed && camera.show_roofs > 0.5 {
        let roof_col = roof_color(world_x, world_y);

        // Shadow on roof surface
        let roof_shadow = trace_shadow_ray(world_x, world_y, roof_h, sun_dir, sun_elev);
        let roof_light = roof_shadow.w;
        let roof_tint = roof_shadow.xyz;

        var color = roof_col * (ambient + sun_color * roof_light * 0.8 * roof_tint);

        // Edge where roof meets walls that stick above: darken slightly
        // Check if any neighbor is taller than the roof
        var near_tall = false;
        for (var dy: i32 = -1; dy <= 1; dy++) {
            for (var dx: i32 = -1; dx <= 1; dx++) {
                if dx == 0 && dy == 0 { continue; }
                let nb = get_block(bx + dx, by + dy);
                let nb_rh = get_roof_height(bx + dx, by + dy);
                // Neighbor is a non-roofed wall taller than our roof = edge
                if f32(block_height(nb)) >= roof_h && nb_rh < 0.5 {
                    near_tall = true;
                }
            }
        }

        // Eave shadow at roof edges (near outer walls)
        let dist_to_edge = min(min(fx, 1.0 - fx), min(fy, 1.0 - fy));
        var has_adjacent_exterior = false;
        let nb_t = get_block(bx, by - 1);
        let nb_b = get_block(bx, by + 1);
        let nb_l = get_block(bx - 1, by);
        let nb_r = get_block(bx + 1, by);
        let rh_t = get_roof_height(bx, by - 1);
        let rh_b = get_roof_height(bx, by + 1);
        let rh_l = get_roof_height(bx - 1, by);
        let rh_r = get_roof_height(bx + 1, by);

        // Directional eave shadow on edges facing away from sun
        // Sun dir points toward sun; edges facing away get darker
        let sun_n = normalize(sun_dir);
        if rh_b < 0.5 && fy > 0.7 && sun_n.y < 0.0 {
            let t = smoothstep(0.7, 1.0, fy) * 0.12 * (-sun_n.y);
            color *= (1.0 - t);
        }
        if rh_t < 0.5 && fy < 0.3 && sun_n.y > 0.0 {
            let t = smoothstep(0.3, 0.0, fy) * 0.12 * sun_n.y;
            color *= (1.0 - t);
        }
        if rh_r < 0.5 && fx > 0.7 && sun_n.x < 0.0 {
            let t = smoothstep(0.7, 1.0, fx) * 0.10 * (-sun_n.x);
            color *= (1.0 - t);
        }
        if rh_l < 0.5 && fx < 0.3 && sun_n.x > 0.0 {
            let t = smoothstep(0.3, 0.0, fx) * 0.10 * sun_n.x;
            color *= (1.0 - t);
        }
        // Bright edge on sun-facing sides
        if rh_t < 0.5 && fy < 0.3 && sun_n.y < 0.0 {
            let t = smoothstep(0.3, 0.0, fy) * 0.06 * (-sun_n.y);
            color += vec3<f32>(t);
        }
        if rh_b < 0.5 && fy > 0.7 && sun_n.y > 0.0 {
            let t = smoothstep(0.7, 1.0, fy) * 0.06 * sun_n.y;
            color += vec3<f32>(t);
        }
        if rh_l < 0.5 && fx < 0.3 && sun_n.x < 0.0 {
            let t = smoothstep(0.3, 0.0, fx) * 0.04 * (-sun_n.x);
            color += vec3<f32>(t);
        }
        if rh_r < 0.5 && fx > 0.7 && sun_n.x > 0.0 {
            let t = smoothstep(0.7, 1.0, fx) * 0.04 * sun_n.x;
            color += vec3<f32>(t);
        }

        color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
        textureStore(output, vec2<u32>(px, py), vec4<f32>(color, 1.0));
        return;
    }

    // --- Not roofed (or roofs transparent): render the actual block surface ---
    var color: vec3<f32>;
    var is_glass_pixel = false;

    // If under a roof but transparent mode, add subtle indoor tint
    let is_indoor = is_roofed && camera.show_roofs < 0.5;

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
    } else {
        color = block_base_color(btype, bflags);
    }

    // Open door: treat as floor-level opening (overrides wall type)
    let door_is_open = is_door(block) && is_open(block);
    let effective_height = select(bheight, 0u, door_is_open);
    let effective_fheight = f32(effective_height);

    // Height-based brightness
    color += vec3<f32>(effective_fheight * 0.03);

    // Wall side faces (3D bevel) — skip for open doors
    if effective_height > 0u {
        color += wall_side_shade(world_x, world_y, bx, by, effective_height);
    }

    // Grid lines (subtle, ground-level only)
    if effective_height == 0u {
        let grid_line_w = 0.03;
        let on_grid = f32(fx < grid_line_w || fx > (1.0 - grid_line_w) ||
                          fy < grid_line_w || fy > (1.0 - grid_line_w));
        color = mix(color, color * 0.75, on_grid * 0.4);
    }

    // Shadow / interior lighting
    var shadow_tint = vec3<f32>(1.0);
    var light_factor = 1.0;
    var light_color_out = vec3<f32>(0.0);
    var light_intensity_out = 0.0;

    if is_indoor {
        // Indoor pixel: skip shadow ray entirely. The roof blocks all direct sun.
        // Sample lightmap for point lights (pre-computed per block).
        let lm = sample_lightmap(world_x, world_y);
        light_color_out = lm.xyz;
        light_intensity_out = lm.w;

        // Interior sun lighting still needs per-pixel sunbeam tracing.
        // Window ambient is no longer in the lightmap; pass 0.0 — the
        // INTERIOR_INDIRECT base (0.18) already prevents pitch-black interiors,
        // and sunbeams through glass provide the directional fill.
        let sun_int = get_sun_intensity(camera.time);
        let interior = compute_interior_light(world_x, world_y, sun_int, sun_dir, 0.0);
        shadow_tint = interior.xyz;
        light_factor = interior.w;
    } else {
        // Outdoor pixel: trace shadow ray toward sun (per-pixel, stays here)
        let shadow_result = trace_shadow_ray(world_x, world_y, effective_fheight, sun_dir, sun_elev);
        shadow_tint = shadow_result.xyz;
        light_factor = shadow_result.w;

        // Light bleeding through windows/doors — now from lightmap
        let lm = sample_lightmap(world_x, world_y);
        light_color_out = lm.xyz;
        light_intensity_out = lm.w;
    }

    if btype == 6u || btype == 7u {
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
            let night_boost = 1.0 - get_sun_intensity(camera.time) * 0.7;
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

        // Per-pixel proximity glow: bright hot spot near light sources
        let prox_glow = compute_proximity_glow(world_x, world_y, camera.time);
        let night_boost = 1.0 - get_sun_intensity(camera.time) * 0.7;
        color += prox_glow * camera.indoor_glow_mul * (0.5 + night_boost);
    }

    // Outdoor light glow: warm light spilling through windows/doors onto ground
    if !is_indoor && light_intensity_out > 0.01 {
        // Stronger at night — warm pools of light on the ground outside windows
        let night_boost = 1.0 - get_sun_intensity(camera.time) * 0.8;
        color += light_color_out * light_intensity_out * camera.light_bleed_mul * night_boost;
    }

    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
    textureStore(output, vec2<u32>(px, py), vec4<f32>(color, 1.0));
}
