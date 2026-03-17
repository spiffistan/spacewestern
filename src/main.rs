use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

// --- Cross-platform time ---
#[cfg(not(target_arch = "wasm32"))]
mod time {
    #[derive(Clone, Copy)]
    pub struct Instant(std::time::Instant);

    impl Instant {
        pub fn now() -> Self {
            Instant(std::time::Instant::now())
        }
        pub fn elapsed_secs_since(&self, earlier: &Instant) -> f32 {
            (self.0 - earlier.0).as_secs_f32()
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod time {
    #[derive(Clone, Copy)]
    pub struct Instant(f64); // milliseconds from Performance.now()

    impl Instant {
        pub fn now() -> Self {
            let perf = web_sys::window()
                .expect("no window")
                .performance()
                .expect("no performance");
            Instant(perf.now())
        }
        pub fn elapsed_secs_since(&self, earlier: &Instant) -> f32 {
            ((self.0 - earlier.0) / 1000.0) as f32
        }
    }
}

use time::Instant;

// --- Constants ---
const GRID_W: u32 = 256;
const GRID_H: u32 = 256;
const WORKGROUP_SIZE: u32 = 8;
const DAY_DURATION: f32 = 60.0; // must match shader

// --- Block representation on GPU ---
// Each block is a u32 packed as: [type:8 | height:8 | flags:8 | reserved:8]
// type: 0=air, 1=stone, 2=dirt, 3=water, 4=wall, 5=glass, 6=fireplace, 7=electric_light, 8=tree, 9=bench, 10=standing_lamp, 11=table_lamp, 12=fan
// height: 0-255
// flags: bit0=is_door, bit1=has_roof, bit2=is_open
fn smoothstep_f32(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn half_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) & 1) as u32;
    let exp = ((h >> 10) & 0x1F) as u32;
    let mant = (h & 0x3FF) as u32;
    if exp == 0 {
        if mant == 0 { return if sign == 1 { -0.0 } else { 0.0 }; }
        // Denormalized
        let v = (mant as f32) / 1024.0 * 2.0f32.powi(-14);
        return if sign == 1 { -v } else { v };
    }
    if exp == 31 { return if mant == 0 { f32::INFINITY } else { f32::NAN }; }
    let v = 2.0f32.powi(exp as i32 - 15) * (1.0 + mant as f32 / 1024.0);
    if sign == 1 { -v } else { v }
}

fn make_block(block_type: u8, height: u8, flags: u8) -> u32 {
    (block_type as u32) | ((height as u32) << 8) | ((flags as u32) << 16)
}

fn generate_test_grid() -> Vec<u32> {
    let mut grid = vec![make_block(0, 0, 0); (GRID_W * GRID_H) as usize];
    let w = GRID_W;

    // Floor everywhere
    for y in 0..GRID_H {
        for x in 0..GRID_W {
            grid[(y * w + x) as usize] = make_block(2, 0, 0); // dirt floor
        }
    }

    // --- Helper closures ---
    let set = |grid: &mut Vec<u32>, x: u32, y: u32, b: u32| {
        if x < GRID_W && y < GRID_H {
            grid[(y * w + x) as usize] = b;
        }
    };

    // Center the building cluster in the map (offset from border)
    let ox = 90u32;
    let oy = 84u32;

    let oset = |grid: &mut Vec<u32>, x: u32, y: u32, b: u32| {
        if x + ox < GRID_W && y + oy < GRID_H {
            grid[((y + oy) * w + (x + ox)) as usize] = b;
        }
    };

    // === House 1: Stone cottage (roofed, with windows) ===
    // Walls: x=10..29, y=10..25
    let h1_h = 3u8; // wall height
    let roof_flag = 2u8; // bit1 = has_roof
    // Top and bottom walls
    for x in 10..30 {
        oset(&mut grid, x, 10, make_block(1, h1_h, 0));
        oset(&mut grid, x, 25, make_block(1, h1_h, 0));
    }
    // Left and right walls
    for y in 10..26 {
        oset(&mut grid, 10, y, make_block(1, h1_h, 0));
        oset(&mut grid, 29, y, make_block(1, h1_h, 0));
    }
    // Windows (glass) in house 1 — top wall
    oset(&mut grid, 14, 10, make_block(5, h1_h, 0)); // glass
    oset(&mut grid, 15, 10, make_block(5, h1_h, 0));
    oset(&mut grid, 24, 10, make_block(5, h1_h, 0));
    oset(&mut grid, 25, 10, make_block(5, h1_h, 0));
    // Windows — bottom wall
    oset(&mut grid, 14, 25, make_block(5, h1_h, 0));
    oset(&mut grid, 15, 25, make_block(5, h1_h, 0));
    oset(&mut grid, 24, 25, make_block(5, h1_h, 0));
    oset(&mut grid, 25, 25, make_block(5, h1_h, 0));
    // Windows — side walls
    oset(&mut grid, 10, 15, make_block(5, h1_h, 0));
    oset(&mut grid, 10, 20, make_block(5, h1_h, 0));
    oset(&mut grid, 29, 15, make_block(5, h1_h, 0));
    oset(&mut grid, 29, 20, make_block(5, h1_h, 0));
    // Door
    oset(&mut grid, 20, 10, make_block(4, 1, 1)); // door (low, flag=door)
    // Roof: fill interior with roofed floor
    for y in 11..25 {
        for x in 11..29 {
            oset(&mut grid, x, y, make_block(2, 0, roof_flag)); // dirt floor + roof
        }
    }
    // Interior divider wall in house 1 (splits into two rooms)
    // Horizontal wall from x=11..28 at y=18, with a door at x=16
    for x in 11..29 {
        oset(&mut grid, x, 18, make_block(1, h1_h, 0));
    }
    oset(&mut grid, 16, 18, make_block(4, 1, 1)); // door in divider

    // Small alcove wall in north room (L-shaped room test)
    for y in 11..15 {
        oset(&mut grid, 22, y, make_block(1, h1_h, 0));
    }

    // Fireplace in south room of house 1
    oset(&mut grid, 19, 21, make_block(6, 1, roof_flag)); // fireplace (height 1, roofed)
    // Outdoor fireplace (for comparison with indoor — O2 doesn't deplete outside)
    oset(&mut grid, 40, 15, make_block(6, 1, 0)); // no roof flag = outdoor
    // Electric light in north room
    oset(&mut grid, 15, 14, make_block(7, 0, roof_flag)); // electric light (height 0, roofed)

    // === House 2: Tall building (roofed, with windows) ===
    let h2_h = 5u8;
    for x in 35..55 {
        oset(&mut grid, x, 30, make_block(1, h2_h, 0));
        oset(&mut grid, x, 50, make_block(1, h2_h, 0));
    }
    for y in 30..51 {
        oset(&mut grid, 35, y, make_block(1, h2_h, 0));
        oset(&mut grid, 54, y, make_block(1, h2_h, 0));
    }
    // Windows — evenly spaced along each wall
    for &wx in &[38u32, 41, 44, 47, 50] {
        oset(&mut grid, wx, 30, make_block(5, h2_h, 0));
        oset(&mut grid, wx, 50, make_block(5, h2_h, 0));
    }
    for &wy in &[34u32, 38, 42, 46] {
        oset(&mut grid, 35, wy, make_block(5, h2_h, 0));
        oset(&mut grid, 54, wy, make_block(5, h2_h, 0));
    }
    // Door
    oset(&mut grid, 45, 30, make_block(4, 1, 1));
    // Interior room divider wall
    for x in 36..54 {
        oset(&mut grid, x, 40, make_block(1, h2_h, 0));
    }
    oset(&mut grid, 44, 40, make_block(4, 1, 1)); // door in divider
    // Roof: fill interior
    for y in 31..50 {
        for x in 36..54 {
            let existing = grid[((y + oy) * w + (x + ox)) as usize];
            if block_type_rs(existing) == 0 || block_type_rs(existing) == 2 {
                oset(&mut grid, x, y, make_block(2, 0, roof_flag));
            }
        }
    }

    // === Small shed (low walls, glass roof/skylight feel) ===
    let h3_h = 2u8;
    for x in 45..52 {
        oset(&mut grid, x, 8, make_block(1, h3_h, 0));
        oset(&mut grid, x, 14, make_block(1, h3_h, 0));
    }
    for y in 8..15 {
        oset(&mut grid, 45, y, make_block(1, h3_h, 0));
        oset(&mut grid, 51, y, make_block(1, h3_h, 0));
    }
    // Glass windows on sides
    oset(&mut grid, 48, 8, make_block(5, h3_h, 0));
    oset(&mut grid, 48, 14, make_block(5, h3_h, 0));
    oset(&mut grid, 45, 11, make_block(5, h3_h, 0));
    oset(&mut grid, 51, 11, make_block(5, h3_h, 0));
    // Door
    oset(&mut grid, 49, 14, make_block(4, 1, 1));
    // Roof
    for y in 9..14 {
        for x in 46..51 {
            oset(&mut grid, x, y, make_block(2, 0, roof_flag));
        }
    }

    // Water pool (unchanged)
    for y in 40..48 {
        for x in 12..22 {
            oset(&mut grid, x, y, make_block(3, 0, 0));
        }
    }

    // Some standalone glass walls (like a greenhouse fragment)
    for x in 5..9 {
        oset(&mut grid, x, 55, make_block(5, 2, 0));
        oset(&mut grid, x, 60, make_block(5, 2, 0));
    }
    for y in 55..61 {
        oset(&mut grid, 5, y, make_block(5, 2, 0));
        oset(&mut grid, 8, y, make_block(5, 2, 0));
    }
    // Greenhouse interior: roofed
    for y in 56..60 {
        for x in 6..8 {
            oset(&mut grid, x, y, make_block(2, 0, roof_flag));
        }
    }

    // Scatter trees and bushes across the map
    // Large trees (2x2), medium trees (1x1), bushes (1x1 short)
    // Flags bits 3-4 = quadrant (for 2x2 trees), bit 5 = is large (2x2)
    let is_bare = |grid: &Vec<u32>, x: u32, y: u32| -> bool {
        if x >= GRID_W || y >= GRID_H { return false; }
        grid[(y * w + x) as usize] == make_block(2, 0, 0)
    };
    for y in 0..GRID_H {
        for x in 0..GRID_W {
            let idx = (y * w + x) as usize;
            if grid[idx] != make_block(2, 0, 0) {
                continue;
            }
            let h = ((x.wrapping_mul(374761393)) ^ (y.wrapping_mul(668265263)))
                .wrapping_add(1013904223);
            let r = (h >> 16) & 0xFFF; // 0..4095

            if r < 30 {
                // Large tree (2x2) — check all 4 tiles are bare
                if is_bare(&grid, x+1, y) && is_bare(&grid, x, y+1) && is_bare(&grid, x+1, y+1) {
                    let tree_h = 4 + ((h >> 8) & 0x1) as u8; // height 4-5
                    // bit5 = large flag (32), bits 3-4 = quadrant
                    set(&mut grid, x,   y,   make_block(8, tree_h, 32 | 0));  // TL, quadrant 0
                    set(&mut grid, x+1, y,   make_block(8, tree_h, 32 | 8));  // TR, quadrant 1
                    set(&mut grid, x,   y+1, make_block(8, tree_h, 32 | 16)); // BL, quadrant 2
                    set(&mut grid, x+1, y+1, make_block(8, tree_h, 32 | 24)); // BR, quadrant 3
                }
            } else if r < 90 {
                // Medium tree (1x1)
                let tree_h = 2 + ((h >> 8) & 0x3) as u8; // height 2-5
                grid[idx] = make_block(8, tree_h, 0);
            } else if r < 140 {
                // Bush (1x1, short)
                let bush_h = 1 + ((h >> 8) & 0x1) as u8; // height 1-2
                grid[idx] = make_block(8, bush_h, 0);
            }
        }
    }

    grid
}

// --- Tree sprite generation ---
// Each sprite is SPRITE_SIZE x SPRITE_SIZE pixels, stored as packed u32 (RGBA8).
// R,G,B = color. A = height (0 = transparent/show ground, 1-255 = canopy/trunk height).
// The atlas is SPRITE_VARIANTS sprites laid out in a flat array.
const SPRITE_SIZE: u32 = 16;
const SPRITE_VARIANTS: u32 = 4;

fn generate_tree_sprites() -> Vec<u32> {
    let pixels_per = (SPRITE_SIZE * SPRITE_SIZE) as usize;
    let total = pixels_per * SPRITE_VARIANTS as usize;
    let mut data = vec![0u32; total];

    for variant in 0..SPRITE_VARIANTS {
        for y in 0..SPRITE_SIZE {
            for x in 0..SPRITE_SIZE {
                let cx = (x as f32 + 0.5) / SPRITE_SIZE as f32 - 0.5;
                let cy = (y as f32 + 0.5) / SPRITE_SIZE as f32 - 0.5;
                let dist = (cx * cx + cy * cy).sqrt();

                let (r, g, b, h) = match variant {
                    0 => {
                        // Round oak: large canopy
                        let canopy_r = 0.48;
                        let trunk_r = 0.08;
                        if dist < trunk_r {
                            (90, 58, 28, 220u8)
                        } else if dist < canopy_r {
                            let shade = 1.0 - dist / canopy_r;
                            let g = (55.0 + shade * 90.0) as u8;
                            let h = (140.0 + shade * 80.0) as u8;
                            (30 + (shade * 25.0) as u8, g, 18, h)
                        } else {
                            (0, 0, 0, 0u8)
                        }
                    }
                    1 => {
                        // Pine/conifer: pointed, diamond-ish shape
                        let abs_cx = cx.abs();
                        let abs_cy = cy.abs();
                        let diamond = abs_cx + abs_cy;
                        let trunk_r = 0.05;
                        let canopy_r = 0.42 - (cy + 0.1).abs() * 0.25;
                        let canopy_r = canopy_r.max(0.06);
                        if dist < trunk_r {
                            (75, 48, 22, 240u8)
                        } else if diamond < canopy_r + 0.12 && dist < 0.48 {
                            let shade = 1.0 - diamond / (canopy_r + 0.1);
                            let g = (40.0 + shade * 60.0) as u8;
                            let h = (160.0 + shade * 70.0) as u8;
                            (15 + (shade * 20.0) as u8, g, 22, h)
                        } else {
                            (0, 0, 0, 0u8)
                        }
                    }
                    2 => {
                        // Small bush: low, wide, lumpy
                        let canopy_r = 0.40;
                        let trunk_r = 0.05;
                        // Make it lumpy with a simple hash
                        let angle = cy.atan2(cx);
                        let lump = 1.0 + 0.12 * (angle * 3.0).sin() + 0.08 * (angle * 7.0).sin();
                        let effective_r = canopy_r * lump;
                        if dist < trunk_r {
                            (80, 52, 25, 120u8)
                        } else if dist < effective_r {
                            let shade = 1.0 - dist / effective_r;
                            let g = (65.0 + shade * 70.0) as u8;
                            let h = (80.0 + shade * 60.0) as u8;
                            (40 + (shade * 20.0) as u8, g, 25, h)
                        } else {
                            (0, 0, 0, 0u8)
                        }
                    }
                    _ => {
                        // Tall narrow tree: thin canopy
                        let canopy_rx = 0.26;
                        let canopy_ry = 0.44;
                        let trunk_r = 0.06;
                        let ellipse = (cx / canopy_rx).powi(2) + (cy / canopy_ry).powi(2);
                        if dist < trunk_r {
                            (85, 55, 25, 250u8)
                        } else if ellipse < 1.0 {
                            let shade = 1.0 - ellipse;
                            let g = (50.0 + shade * 80.0) as u8;
                            let h = (170.0 + shade * 70.0) as u8;
                            (25 + (shade * 20.0) as u8, g, 20, h)
                        } else {
                            (0, 0, 0, 0u8)
                        }
                    }
                };

                let packed = (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((h as u32) << 24);
                let idx = (variant * SPRITE_SIZE * SPRITE_SIZE + y * SPRITE_SIZE + x) as usize;
                data[idx] = packed;
            }
        }
    }

    data
}

// Rust-side helpers to read block fields from packed u32
fn block_type_rs(b: u32) -> u8 {
    (b & 0xFF) as u8
}

fn block_flags_rs(b: u32) -> u8 {
    ((b >> 16) & 0xFF) as u8
}

fn is_door_rs(b: u32) -> bool {
    (block_flags_rs(b) & 1) != 0
}

/// Precompute roof heights and store in bits 24-31 of each block.
/// For every tile that's part of a roofed building, find the max wall height
/// in a large radius. This runs once at grid generation.
fn compute_roof_heights(grid: &mut Vec<u32>) {
    let w = GRID_W as i32;
    let h = GRID_H as i32;

    // Pass 1: identify which tiles are part of a roofed building
    let mut is_building = vec![false; grid.len()];
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            let block = grid[idx];
            let flags = block_flags_rs(block);

            if (flags & 2) != 0 {
                // Has roof flag
                is_building[idx] = true;
            } else if (block >> 8) & 0xFF > 0 || (flags & 1) != 0 {
                // Has height or is a door — check if adjacent to a roofed tile
                'outer: for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let nx = x + dx;
                        let ny = y + dy;
                        if nx >= 0 && ny >= 0 && nx < w && ny < h {
                            let nflags = (grid[(ny * w + nx) as usize] >> 16) & 0xFF;
                            if (nflags & 2) != 0 {
                                is_building[idx] = true;
                                break 'outer;
                            }
                        }
                    }
                }
            }
        }
    }

    // Pass 2: for each building tile, find max wall height in a large radius
    let search = 15i32; // handles buildings up to 30x30
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            if !is_building[idx] {
                continue;
            }

            let mut max_h: u8 = 0;
            for dy in -search..=search {
                for dx in -search..=search {
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                        continue;
                    }
                    let nidx = (ny * w + nx) as usize;
                    let nb = grid[nidx];
                    let nbh = ((nb >> 8) & 0xFF) as u8;
                    let nbt = (nb & 0xFF) as u8;
                    let nb_flags = ((nb >> 16) & 0xFF) as u8;
                    // Wall-type blocks: has height, not roofed floor, not tree/fire/light
                    if nbh > 0 && (nb_flags & 2) == 0 && nbt != 8 && nbt != 6 && nbt != 7 {
                        max_h = max_h.max(nbh);
                    }
                }
            }

            if max_h == 0 {
                max_h = 2; // fallback
            }

            // Store in bits 24-31
            grid[idx] = (grid[idx] & 0x00FFFFFF) | ((max_h as u32) << 24);
        }
    }
}

// --- Camera uniform ---
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    center_x: f32,
    center_y: f32,
    zoom: f32,
    show_roofs: f32, // 0.0 = transparent (see interior), 1.0 = opaque roofs
    screen_w: f32,
    screen_h: f32,
    grid_w: f32,
    grid_h: f32,
    time: f32, // elapsed seconds, drives sun animation
    // Lightmap tuning parameters (adjustable via UI)
    glass_light_mul: f32,     // how much interior light shows through glass (default 0.12)
    indoor_glow_mul: f32,     // indoor floor glow strength from point lights (default 0.25)
    light_bleed_mul: f32,     // outdoor ground glow from interior lights (default 0.6)
    // Foliage shadow parameters
    foliage_opacity: f32,     // overall canopy shadow density (0=transparent, 1=opaque) (default 0.55)
    foliage_variation: f32,   // per-tree randomness in shadow density (default 0.3)
    oblique_strength: f32,    // wall face visibility per height unit (default 0.12)
    lm_vp_min_x: f32,        // lightmap viewport min x (grid coordinates)
    lm_vp_min_y: f32,        // lightmap viewport min y (grid coordinates)
    lm_vp_max_x: f32,        // lightmap viewport max x (grid coordinates)
    lm_vp_max_y: f32,        // lightmap viewport max y (grid coordinates)
    lm_scale: f32,           // lightmap texels per grid cell (e.g. 2.0 for 2x resolution)
    fluid_overlay: f32,      // 0=off, 1=smoke, 2=velocity, 3=pressure, 4=O2, 5=CO2
    sun_dir_x: f32,          // precomputed sun direction X
    sun_dir_y: f32,          // precomputed sun direction Y
    sun_elevation: f32,      // precomputed sun elevation
    sun_intensity: f32,      // precomputed sun intensity (0=night, 1=day)
    sun_color_r: f32,        // precomputed sun color R
    sun_color_g: f32,        // precomputed sun color G
    sun_color_b: f32,        // precomputed sun color B
    ambient_r: f32,          // precomputed ambient R
    ambient_g: f32,          // precomputed ambient G
    ambient_b: f32,          // precomputed ambient B
    enable_prox_glow: f32,   // 1.0 = on, 0.0 = off
    enable_dir_bleed: f32,   // 1.0 = on, 0.0 = off
    force_refresh: f32,      // 1.0 = skip reprojection this frame (grid changed)
    _pad3: f32,
    _pad4: f32,
    prev_center_x: f32,
    prev_center_y: f32,
    prev_zoom: f32,
    prev_time: f32,
}

// --- Fluid simulation uniform ---
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct FluidParams {
    sim_w: f32, sim_h: f32, dye_w: f32, dye_h: f32,
    dt: f32, dissipation: f32, vorticity_strength: f32, pressure_iterations: f32,
    splat_x: f32, splat_y: f32, splat_vx: f32, splat_vy: f32,
    splat_radius: f32, splat_active: f32, time: f32, wind_x: f32,
    wind_y: f32, smoke_rate: f32, fan_speed: f32, _pad3: f32,
}

const FLUID_SIM_W: u32 = 256;
const FLUID_SIM_H: u32 = 256;
const FLUID_DYE_W: u32 = 512;
const FLUID_DYE_H: u32 = 512;
const FLUID_PRESSURE_ITERS: u32 = 35; // odd: final in B, clear reads B→A, cycle consistent

fn build_obstacle_field(grid: &[u32]) -> Vec<u8> {
    grid.iter().map(|&b| {
        let bt = b & 0xFF;
        let bh = (b >> 8) & 0xFF;
        let is_door = (b >> 16) & 1 != 0;
        let is_open = (b >> 16) & 4 != 0;
        // Walls and glass block fluid. Trees, open doors, and light sources don't.
        if bh > 0 && bt != 8 && bt != 6 && bt != 7 && bt != 10 && bt != 11 && bt != 12 && !(is_door && is_open) { 255 } else { 0 }
    }).collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum BuildTool {
    None,
    Fireplace,
    ElectricLight,
    Bench,
    StandingLamp,
    TableLamp,
    Fan,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum FluidOverlay {
    None,
    Smoke,     // show dye density as colored overlay
    Velocity,  // show velocity magnitude as heatmap
    Pressure,  // show pressure field
    O2,        // show O2 levels (blue=high, red=depleted)
    CO2,       // show CO2 levels (dark=none, yellow-green=high)
}

// --- Application state ---
struct App {
    window: Option<Arc<Window>>,
    gfx: Option<GfxState>,
    egui_state: Option<EguiState>,
    camera: CameraUniform,
    render_scale: f32,
    grid_data: Vec<u32>,
    grid_dirty: bool, // true when grid needs re-upload to GPU
    mouse_pressed: bool,
    mouse_dragged: bool, // true if mouse moved while pressed (pan, not click)
    last_mouse_x: f64,
    last_mouse_y: f64,
    // Right-click drag to move light sources
    dragging_light: Option<(u32, u32)>, // grid position of light being dragged
    #[allow(dead_code)]
    start_time: Instant,
    // Time control
    time_of_day: f32,        // current time in seconds (0..DAY_DURATION)
    time_paused: bool,       // pause auto-advance
    time_speed: f32,         // playback speed multiplier
    last_frame_time: Instant, // for delta-time calculation
    // FPS tracking
    frame_count: u32,
    fps_accum: f32,
    fps_display: f32,
    // Lightmap update throttle (skip most frames)
    lightmap_frame: u32,
    // Build mode
    build_tool: BuildTool,
    build_rotation: u32,       // 0=horizontal (E-W), 1=vertical (N-S)
    hover_world: (f32, f32),   // world coords under mouse cursor
    // Fluid simulation
    fluid_params: FluidParams,
    fluid_dye_phase: usize,    // 0 or 1: which dye texture is current (readable)
    output_phase: usize,       // 0 or 1: ping-pong for temporal reprojection
    prev_cam_x: f32,           // previous frame's camera center (for temporal reprojection)
    prev_cam_y: f32,
    prev_cam_zoom: f32,
    prev_cam_time: f32,
    fluid_overlay: FluidOverlay,
    fluid_speed: f32,             // fluid simulation speed multiplier
    debug_mode: bool,             // show debug tooltip at cursor
    enable_prox_glow: bool,       // per-pixel proximity glow (expensive)
    enable_dir_bleed: bool,       // directional light bleed (expensive)
    debug_fluid_density: [f32; 4], // last readback: RGBA from dye texture at cursor
    debug_fluid_readback_pending: bool,
    fluid_mouse_active: bool,  // middle mouse button held
    fluid_mouse_prev: Option<(f32, f32)>, // previous world position for velocity calc
}

const LIGHTMAP_SCALE: u32 = 2; // lightmap texels per grid cell (2x resolution)
const LIGHTMAP_W: u32 = GRID_W * LIGHTMAP_SCALE;
const LIGHTMAP_H: u32 = GRID_H * LIGHTMAP_SCALE;
const LIGHTMAP_PROP_ITERATIONS: u32 = 26; // more iterations for 2x res (covers ~13 tile radius)
const LIGHTMAP_UPDATE_INTERVAL: u32 = 2; // recompute every N frames (~30fps lightmap at 60fps)
const DEFAULT_RENDER_SCALE: f32 = 0.5;

struct GfxState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    #[allow(dead_code)]
    surface_format: wgpu::TextureFormat,
    // Lightmap: seed + iterative propagation (ping-pong, 512x512 at 2x scale)
    lightmap_seed_pipeline: wgpu::ComputePipeline,
    lightmap_seed_bind_groups: [wgpu::BindGroup; 2], // [0]: write A, [1]: write B
    lightmap_prop_pipeline: wgpu::ComputePipeline,
    lightmap_prop_bind_groups: [wgpu::BindGroup; 2], // [0]: read A write B, [1]: read B write A
    lightmap_textures: [wgpu::Texture; 2],
    // Raytrace pass (per-pixel, screen resolution)
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_groups: [wgpu::BindGroup; 4],
    render_pipeline: wgpu::RenderPipeline,
    render_bind_groups: [wgpu::BindGroup; 2],
    output_textures: [wgpu::Texture; 2],
    camera_buffer: wgpu::Buffer,
    grid_buffer: wgpu::Buffer,
    sprite_buffer: wgpu::Buffer,
    // Fluid simulation GPU resources
    fluid_params_buffer: wgpu::Buffer,
    fluid_vel: [wgpu::Texture; 2],
    fluid_pres: [wgpu::Texture; 2],
    fluid_div: wgpu::Texture,
    fluid_curl: wgpu::Texture,
    fluid_dye: [wgpu::Texture; 2],
    fluid_obstacle: wgpu::Texture,
    fluid_dummy_rg: wgpu::Texture,  // 1x1 Rg32Float dummy for unused bindings
    fluid_dummy_r: wgpu::Texture,   // 1x1 R32Float dummy (read)
    fluid_dummy_r_w: wgpu::Texture,  // 1x1 R32Float dummy (write, separate to avoid read-write conflict)
    // Fluid pipelines
    fluid_p_curl: wgpu::ComputePipeline,
    fluid_p_vorticity: wgpu::ComputePipeline,
    fluid_p_divergence: wgpu::ComputePipeline,
    fluid_p_gradient: wgpu::ComputePipeline,
    fluid_p_advect_vel: wgpu::ComputePipeline,
    fluid_p_splat: wgpu::ComputePipeline,
    fluid_p_pressure: wgpu::ComputePipeline,
    fluid_p_pressure_clear: wgpu::ComputePipeline,
    fluid_p_advect_dye: wgpu::ComputePipeline,
    // Fluid bind groups (fixed phase assignments per frame)
    fluid_bg_curl: wgpu::BindGroup,
    fluid_bg_vorticity: wgpu::BindGroup,
    fluid_bg_splat: wgpu::BindGroup,
    fluid_bg_divergence: wgpu::BindGroup,
    fluid_bg_gradient: wgpu::BindGroup,
    fluid_bg_advect_vel: wgpu::BindGroup,
    fluid_bg_pressure: [wgpu::BindGroup; 2],       // ping-pong
    fluid_bg_pressure_clear: wgpu::BindGroup,       // A→B clear
    fluid_bg_advect_dye: [wgpu::BindGroup; 2],     // ping-pong dye
    // Debug readback
    debug_readback_buffer: wgpu::Buffer,            // staging buffer for single texel readback
}

struct EguiState {
    ctx: egui::Context,
    winit_state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            gfx: None,
            egui_state: None,
            camera: CameraUniform {
                center_x: 128.0, // centered on the map
                center_y: 128.0,
                zoom: 1.0, // will be set in init_gfx_async to fit map
                show_roofs: 0.0,
                screen_w: 800.0,
                screen_h: 600.0,
                grid_w: GRID_W as f32,
                grid_h: GRID_H as f32,
                time: 0.0,
                glass_light_mul: 0.12,
                indoor_glow_mul: 0.25,
                light_bleed_mul: 0.10,
                foliage_opacity: 0.55,
                foliage_variation: 0.3,
                oblique_strength: 0.12,
                lm_vp_min_x: 0.0,
                lm_vp_min_y: 0.0,
                lm_vp_max_x: GRID_W as f32,
                lm_vp_max_y: GRID_H as f32,
                lm_scale: LIGHTMAP_SCALE as f32,
                fluid_overlay: 0.0,
                sun_dir_x: 0.0, sun_dir_y: 0.0, sun_elevation: 0.0,
                sun_intensity: 0.0,
                sun_color_r: 0.0, sun_color_g: 0.0, sun_color_b: 0.0,
                ambient_r: 0.0, ambient_g: 0.0, ambient_b: 0.0,
                enable_prox_glow: 1.0,
                enable_dir_bleed: 1.0,
                force_refresh: 1.0, _pad3: 0.0, _pad4: 0.0,
                prev_center_x: 0.0, prev_center_y: 0.0, prev_zoom: 0.0, prev_time: 0.0,
            },
            render_scale: DEFAULT_RENDER_SCALE,
            grid_data: Vec::new(),
            grid_dirty: false,
            mouse_pressed: false,
            mouse_dragged: false,
            last_mouse_x: 0.0,
            last_mouse_y: 0.0,
            dragging_light: None,
            start_time: Instant::now(),
            time_of_day: DAY_DURATION * (8.0 / 24.0), // start at 08:00
            time_paused: false,
            time_speed: 1.0,
            last_frame_time: Instant::now(),
            frame_count: 0,
            fps_accum: 0.0,
            fps_display: 0.0,
            lightmap_frame: 0,
            build_tool: BuildTool::None,
            build_rotation: 0,
            hover_world: (0.0, 0.0),
            fluid_params: FluidParams {
                sim_w: FLUID_SIM_W as f32,
                sim_h: FLUID_SIM_H as f32,
                dye_w: FLUID_DYE_W as f32,
                dye_h: FLUID_DYE_H as f32,
                dt: 1.0 / 60.0,
                dissipation: 0.999,
                vorticity_strength: 35.0,
                pressure_iterations: FLUID_PRESSURE_ITERS as f32,
                splat_x: 0.0,
                splat_y: 0.0,
                splat_vx: 0.0,
                splat_vy: 0.0,
                splat_radius: 5.0,
                splat_active: 0.0,
                time: 0.0,
                wind_x: 10.0,
                wind_y: 10.0,
                smoke_rate: 0.3,
                fan_speed: 40.0,
                _pad3: 0.0,
            },
            fluid_overlay: FluidOverlay::None,
            fluid_speed: 1.0,
            debug_mode: false,
            enable_prox_glow: true,
            enable_dir_bleed: true,
            debug_fluid_density: [0.0; 4],
            debug_fluid_readback_pending: false,
            fluid_dye_phase: 0,
            output_phase: 0,
            prev_cam_x: 0.0,
            prev_cam_y: 0.0,
            prev_cam_zoom: 0.0,
            prev_cam_time: 0.0,
            fluid_mouse_active: false,
            fluid_mouse_prev: None,
        }
    }

    /// Convert world block coordinates to window screen pixels
    #[allow(dead_code)]
    fn world_to_screen(&self, wx: f32, wy: f32) -> (f32, f32) {
        let rx = (wx - self.camera.center_x) * self.camera.zoom + self.camera.screen_w * 0.5;
        let ry = (wy - self.camera.center_y) * self.camera.zoom + self.camera.screen_h * 0.5;
        (rx / self.render_scale, ry / self.render_scale)
    }

    /// Get the tiles a bench would occupy at (bx, by) with given rotation
    fn bench_tiles(&self, bx: i32, by: i32, rotation: u32) -> [(i32, i32); 3] {
        if rotation == 0 {
            // Horizontal: extends east
            [(bx, by), (bx + 1, by), (bx + 2, by)]
        } else {
            // Vertical: extends south
            [(bx, by), (bx, by + 1), (bx, by + 2)]
        }
    }

    /// Check if a tile is valid for placement (ground level, in bounds)
    fn can_place_at(&self, x: i32, y: i32) -> bool {
        self.can_place_on(x, y, false)
    }

    /// Check if a tile is valid for placement. If `on_furniture` is true,
    /// allows placement on benches (for table lamps).
    fn can_place_on(&self, x: i32, y: i32, on_furniture: bool) -> bool {
        if x < 0 || y < 0 || x >= GRID_W as i32 || y >= GRID_H as i32 {
            return false;
        }
        let idx = (y as u32 * GRID_W + x as u32) as usize;
        let block = self.grid_data[idx];
        let bt = block_type_rs(block);
        let bh = (block >> 8) & 0xFF;
        if on_furniture {
            bt == 9 // bench
        } else {
            (bt == 0 || bt == 2) && bh == 0
        }
    }

    /// Convert screen pixel coordinates to world block coordinates
    fn screen_to_world(&self, sx: f64, sy: f64) -> (f32, f32) {
        // Scale mouse coords from window space to render space
        let rx = sx as f32 * self.render_scale;
        let ry = sy as f32 * self.render_scale;
        let wx = self.camera.center_x + (rx - self.camera.screen_w * 0.5) / self.camera.zoom;
        let wy = self.camera.center_y + (ry - self.camera.screen_h * 0.5) / self.camera.zoom;
        (wx, wy)
    }

    /// Try to pick up a light source at the given world coordinates (right-click)
    fn try_pick_light(&mut self, wx: f32, wy: f32) -> bool {
        let bx = wx.floor() as i32;
        let by = wy.floor() as i32;
        if bx < 0 || by < 0 || bx >= GRID_W as i32 || by >= GRID_H as i32 {
            return false;
        }
        let idx = (by as u32 * GRID_W + bx as u32) as usize;
        let block = self.grid_data[idx];
        let bt = block_type_rs(block);
        if bt == 6 || bt == 7 {
            self.dragging_light = Some((bx as u32, by as u32));
            log::info!("Picked up light at ({}, {})", bx, by);
            return true;
        }
        false
    }

    /// Move a dragged light source to a new position
    fn move_light_to(&mut self, wx: f32, wy: f32) {
        let new_bx = wx.floor() as i32;
        let new_by = wy.floor() as i32;
        if new_bx < 0 || new_by < 0 || new_bx >= GRID_W as i32 || new_by >= GRID_H as i32 {
            return;
        }
        if let Some((old_x, old_y)) = self.dragging_light {
            let old_idx = (old_y * GRID_W + old_x) as usize;
            let new_idx = (new_by as u32 * GRID_W + new_bx as u32) as usize;

            // Only move if destination is a floor tile (type 2, height 0)
            let dest = self.grid_data[new_idx];
            let dest_bt = block_type_rs(dest);
            let dest_h = (dest >> 8) & 0xFF;
            if (dest_bt == 2 || dest_bt == 0) && dest_h == 0 && new_idx != old_idx {
                let light_block = self.grid_data[old_idx];
                let light_flags = block_flags_rs(light_block);
                let dest_flags = (dest >> 16) & 0xFF;

                // Replace old position with floor (preserve roof flag)
                self.grid_data[old_idx] = make_block(2, 0, (light_flags & 2) as u8);

                // Place light at new position (preserve destination roof flag)
                let new_block = (light_block & 0x0000FFFF) | ((dest_flags as u32) << 16);
                // Also preserve the precomputed roof height from destination
                let dest_roof_h = (dest >> 24) & 0xFF;
                self.grid_data[new_idx] = (new_block & 0x00FFFFFF) | (dest_roof_h << 24);

                self.dragging_light = Some((new_bx as u32, new_by as u32));
                self.grid_dirty = true;
            }
        }
    }

    /// Drop a dragged light source
    fn drop_light(&mut self) {
        if let Some((x, y)) = self.dragging_light.take() {
            log::info!("Placed light at ({}, {})", x, y);
            // Recompute roof heights since light moved
            compute_roof_heights(&mut self.grid_data);
            self.grid_dirty = true;
        }
    }

    /// Handle left-click: build tool placement, door toggle, or light toggle
    fn handle_click(&mut self, wx: f32, wy: f32) {
        let bx = wx.floor() as i32;
        let by = wy.floor() as i32;
        if bx < 0 || by < 0 || bx >= GRID_W as i32 || by >= GRID_H as i32 {
            return;
        }
        let idx = (by as u32 * GRID_W + bx as u32) as usize;
        let block = self.grid_data[idx];
        let bt = block_type_rs(block);
        let flags = block_flags_rs(block);

        // Build tool placement
        if self.build_tool != BuildTool::None {
            match self.build_tool {
                BuildTool::Bench => {
                    let tiles = self.bench_tiles(bx, by, self.build_rotation);
                    let all_valid = tiles.iter().all(|&(tx, ty)| self.can_place_at(tx, ty));
                    if all_valid {
                        for (i, &(tx, ty)) in tiles.iter().enumerate() {
                            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                            let tblock = self.grid_data[tidx];
                            let roof_flag = ((tblock >> 16) & 0xFF) as u8 & 2;
                            let roof_h = tblock & 0xFF000000;
                            // flags: bit3-4 = segment (0,1,2), bit5-6 = rotation
                            let seg_flags = roof_flag | ((i as u8) << 3) | ((self.build_rotation as u8) << 5);
                            self.grid_data[tidx] = make_block(9, 1, seg_flags) | roof_h;
                        }
                        self.grid_dirty = true;
                        log::info!("Placed bench at ({}, {})", bx, by);
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::Fireplace | BuildTool::ElectricLight | BuildTool::StandingLamp => {
                    if self.can_place_at(bx, by) {
                        let roof_flag = flags & 2;
                        let new_block = match self.build_tool {
                            BuildTool::Fireplace => make_block(6, 1, roof_flag),
                            BuildTool::ElectricLight => make_block(7, 0, roof_flag),
                            BuildTool::StandingLamp => make_block(10, 2, roof_flag), // height 2: above bench, below ceiling
                            _ => unreachable!(),
                        };
                        let roof_h = block & 0xFF000000;
                        self.grid_data[idx] = new_block | roof_h;
                        self.grid_dirty = true;
                        log::info!("Placed {:?} at ({}, {})", self.build_tool, bx, by);
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::TableLamp => {
                    // Table lamp can only be placed on benches (type 9)
                    if bt == 9 {
                        // Replace the bench tile with a table lamp, keeping bench flags
                        let roof_h = block & 0xFF000000;
                        let roof_flag = flags & 2;
                        self.grid_data[idx] = make_block(11, 1, roof_flag) | roof_h;
                        self.grid_dirty = true;
                        log::info!("Placed table lamp at ({}, {})", bx, by);
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::Fan => {
                    // Fan: must be placed on a wall (type 1 or 4 with height > 0)
                    if (bt == 1 || bt == 4) && (block >> 8) & 0xFF > 0 {
                        let wall_h = ((block >> 8) & 0xFF) as u8;
                        let roof_flag = flags & 2;
                        let roof_h = block & 0xFF000000;
                        let dir_flags = roof_flag | ((self.build_rotation as u8) << 3);
                        self.grid_data[idx] = make_block(12, wall_h, dir_flags) | roof_h;
                        self.grid_dirty = true;
                        log::info!("Placed fan at ({}, {}) dir={}", bx, by, self.build_rotation);
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::None => {}
            }
            return;
        }

        // Toggle door
        if is_door_rs(block) {
            let new_flags = flags ^ 4; // toggle bit2 (is_open)
            let new_block = (block & 0xFF00FFFF) | ((new_flags as u32) << 16);
            self.grid_data[idx] = new_block;
            self.grid_dirty = true;
            let open = (new_flags & 4) != 0;
            log::info!("Door at ({}, {}): {}", bx, by, if open { "opened" } else { "closed" });
            return;
        }

        // Remove any light source by clicking on it (replace with dirt floor)
        if bt == 6 || bt == 7 || bt == 10 || bt == 11 {
            let roof_flag = flags & 2;
            let roof_h = block & 0xFF000000;
            self.grid_data[idx] = make_block(2, 0, roof_flag) | roof_h;
            self.grid_dirty = true;
            let name = match bt { 6 => "Fireplace", 7 => "Electric light", 10 => "Floor lamp", 11 => "Table lamp", _ => "Light" };
            log::info!("Removed {} at ({}, {})", name, bx, by);
        }

        // Remove fan: revert to stone wall
        if bt == 12 {
            let roof_flag = flags & 2;
            let roof_h = block & 0xFF000000;
            let height = ((block >> 8) & 0xFF) as u8;
            self.grid_data[idx] = make_block(1, height, roof_flag) | roof_h;
            self.grid_dirty = true;
            log::info!("Removed fan at ({}, {})", bx, by);
        }
    }

    async fn init_gfx_async(&mut self, window: Arc<Window>) {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let render_w = ((width as f32) * self.render_scale).max(1.0) as u32;
        let render_h = ((height as f32) * self.render_scale).max(1.0) as u32;
        self.camera.screen_w = render_w as f32;
        self.camera.screen_h = render_h as f32;
        // Zoom to show ~64 blocks (the houses area), not the full map
        let view_size = 32.0f32; // default zoom
        let fit_w = render_w as f32 / view_size;
        let fit_h = render_h as f32 / view_size;
        self.camera.zoom = fit_w.min(fit_h);
        log::info!("init_gfx: {}x{} (window), {}x{} (render), zoom={}", width, height, render_w, render_h, self.camera.zoom);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::all(),
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find GPU adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("rayworld-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults()
                        .using_resolution(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::default(),
                    ..Default::default()
                },
            )
            .await
            .expect("Failed to create device");

        let surface_caps = surface.get_capabilities(&adapter);
        // Always use a non-sRGB (linear) surface format.
        // Gamma correction is applied in the raytrace shader for consistency
        // between native and web (WebGPU often lacks sRGB surfaces).
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| !f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        log::info!("Surface format: {:?}", surface_format);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Output textures at render resolution (ping-pong for temporal reprojection)
        let output_texture_a = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("output-texture-a"),
            size: wgpu::Extent3d {
                width: render_w,
                height: render_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let output_texture_b = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("output-texture-b"),
            size: wgpu::Extent3d {
                width: render_w,
                height: render_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let output_view_a = output_texture_a.create_view(&wgpu::TextureViewDescriptor::default());
        let output_view_b = output_texture_b.create_view(&wgpu::TextureViewDescriptor::default());

        // Camera uniform buffer
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera-uniform"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&camera_buffer, 0, bytemuck::bytes_of(&self.camera));

        // Grid storage buffer
        self.grid_data = generate_test_grid();
        compute_roof_heights(&mut self.grid_data);
        let grid_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("grid-buffer"),
            size: (self.grid_data.len() * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&grid_buffer, 0, bytemuck::cast_slice(&self.grid_data));

        // Tree sprite buffer
        let sprite_data = generate_tree_sprites();
        let sprite_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite-buffer"),
            size: (sprite_data.len() * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&sprite_buffer, 0, bytemuck::cast_slice(&sprite_data));

        // --- Lightmap textures (two for ping-pong, at LIGHTMAP_SCALE × grid resolution) ---
        let lightmap_desc = wgpu::TextureDescriptor {
            label: Some("lightmap-texture-a"),
            size: wgpu::Extent3d {
                width: LIGHTMAP_W,
                height: LIGHTMAP_H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let lightmap_a = device.create_texture(&lightmap_desc);
        let lightmap_b = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("lightmap-texture-b"),
            ..lightmap_desc
        });
        let lm_view_a = lightmap_a.create_view(&wgpu::TextureViewDescriptor::default());
        let lm_view_b = lightmap_b.create_view(&wgpu::TextureViewDescriptor::default());

        // Lightmap sampler (bilinear for smooth gradients — used by raytrace shader)
        let lightmap_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("lightmap-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // --- Lightmap seed pipeline (writes to texture A) ---
        let lightmap_seed_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lightmap-seed"),
            source: wgpu::ShaderSource::Wgsl(include_str!("lightmap.wgsl").into()),
        });

        let seed_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lightmap-seed-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let lightmap_seed_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lightmap-seed-bg-a"),
            layout: &seed_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&lm_view_a),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: grid_buffer.as_entire_binding(),
                },
            ],
        });
        let lightmap_seed_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lightmap-seed-bg-b"),
            layout: &seed_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&lm_view_b),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: grid_buffer.as_entire_binding(),
                },
            ],
        });

        let lightmap_seed_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("lightmap-seed-pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("lightmap-seed-pl"),
                bind_group_layouts: &[&seed_bgl],
                push_constant_ranges: &[],
            })),
            module: &lightmap_seed_shader,
            entry_point: Some("main_lightmap_seed"),
            compilation_options: Default::default(),
            cache: None,
        });

        // --- Lightmap propagation pipeline (reads texture_2d, writes storage) ---
        let lightmap_prop_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lightmap-propagate"),
            source: wgpu::ShaderSource::Wgsl(include_str!("lightmap_propagate.wgsl").into()),
        });

        let prop_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lightmap-prop-bgl"),
            entries: &[
                // binding 0: source lightmap (read via textureLoad)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // binding 1: destination lightmap (write)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                // binding 2: camera uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 3: grid buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Bind group [0]: read A, write B
        let prop_bg_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lightmap-prop-bg-0"),
            layout: &prop_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&lm_view_a) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&lm_view_b) },
                wgpu::BindGroupEntry { binding: 2, resource: camera_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: grid_buffer.as_entire_binding() },
            ],
        });

        // Bind group [1]: read B, write A
        let prop_bg_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lightmap-prop-bg-1"),
            layout: &prop_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&lm_view_b) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&lm_view_a) },
                wgpu::BindGroupEntry { binding: 2, resource: camera_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: grid_buffer.as_entire_binding() },
            ],
        });

        let lightmap_prop_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("lightmap-prop-pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("lightmap-prop-pl"),
                bind_group_layouts: &[&prop_bgl],
                push_constant_ranges: &[],
            })),
            module: &lightmap_prop_shader,
            entry_point: Some("main_lightmap_propagate"),
            compilation_options: Default::default(),
            cache: None,
        });

        // --- Fluid simulation GPU resources ---
        let make_fluid_tex = |label: &str, w: u32, h: u32, format: wgpu::TextureFormat| -> wgpu::Texture {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                mip_level_count: 1, sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            })
        };

        let fluid_vel_a = make_fluid_tex("fluid-vel-a", FLUID_SIM_W, FLUID_SIM_H, wgpu::TextureFormat::Rg32Float);
        let fluid_vel_b = make_fluid_tex("fluid-vel-b", FLUID_SIM_W, FLUID_SIM_H, wgpu::TextureFormat::Rg32Float);
        let fluid_pres_a = make_fluid_tex("fluid-pres-a", FLUID_SIM_W, FLUID_SIM_H, wgpu::TextureFormat::R32Float);
        let fluid_pres_b = make_fluid_tex("fluid-pres-b", FLUID_SIM_W, FLUID_SIM_H, wgpu::TextureFormat::R32Float);
        let fluid_div = make_fluid_tex("fluid-div", FLUID_SIM_W, FLUID_SIM_H, wgpu::TextureFormat::R32Float);
        let fluid_curl_tex = make_fluid_tex("fluid-curl", FLUID_SIM_W, FLUID_SIM_H, wgpu::TextureFormat::R32Float);
        let fluid_dye_a = make_fluid_tex("fluid-dye-a", FLUID_DYE_W, FLUID_DYE_H, wgpu::TextureFormat::Rgba16Float);
        let fluid_dye_b = make_fluid_tex("fluid-dye-b", FLUID_DYE_W, FLUID_DYE_H, wgpu::TextureFormat::Rgba16Float);

        // Initialize dye textures with O2 = 1.0 (channel G = f16(1.0) = 0x3C00)
        {
            let texels = (FLUID_DYE_W * FLUID_DYE_H) as usize;
            let mut init_data = vec![0u8; texels * 8]; // 8 bytes per RGBA16Float texel
            for i in 0..texels {
                // G channel = f16(1.0) = 0x3C00, little-endian at byte offset 2
                init_data[i * 8 + 2] = 0x00;
                init_data[i * 8 + 3] = 0x3C;
            }
            for tex in [&fluid_dye_a, &fluid_dye_b] {
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: tex,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &init_data,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(FLUID_DYE_W * 8),
                        rows_per_image: Some(FLUID_DYE_H),
                    },
                    wgpu::Extent3d {
                        width: FLUID_DYE_W,
                        height: FLUID_DYE_H,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }

        let fluid_obstacle_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fluid-obstacle"),
            size: wgpu::Extent3d { width: FLUID_SIM_W, height: FLUID_SIM_H, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let fluid_dummy_rg = make_fluid_tex("fluid-dummy-rg", 1, 1, wgpu::TextureFormat::Rg32Float);
        let fluid_dummy_r = make_fluid_tex("fluid-dummy-r", 1, 1, wgpu::TextureFormat::R32Float);
        let fluid_dummy_r_w = make_fluid_tex("fluid-dummy-r-w", 1, 1, wgpu::TextureFormat::R32Float);

        // Texture views
        let fv_vel_a = fluid_vel_a.create_view(&wgpu::TextureViewDescriptor::default());
        let fv_vel_b = fluid_vel_b.create_view(&wgpu::TextureViewDescriptor::default());
        let fv_pres_a = fluid_pres_a.create_view(&wgpu::TextureViewDescriptor::default());
        let fv_pres_b = fluid_pres_b.create_view(&wgpu::TextureViewDescriptor::default());
        let fv_div = fluid_div.create_view(&wgpu::TextureViewDescriptor::default());
        let fv_curl = fluid_curl_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let fv_dye_a = fluid_dye_a.create_view(&wgpu::TextureViewDescriptor::default());
        let fv_dye_b = fluid_dye_b.create_view(&wgpu::TextureViewDescriptor::default());
        let fv_obstacle = fluid_obstacle_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let fv_dummy_rg = fluid_dummy_rg.create_view(&wgpu::TextureViewDescriptor::default());
        let fv_dummy_r = fluid_dummy_r.create_view(&wgpu::TextureViewDescriptor::default());
        let fv_dummy_r_w = fluid_dummy_r_w.create_view(&wgpu::TextureViewDescriptor::default());

        // Upload initial obstacle field
        let obstacle_data = build_obstacle_field(&self.grid_data);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &fluid_obstacle_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &obstacle_data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(FLUID_SIM_W), rows_per_image: Some(FLUID_SIM_H) },
            wgpu::Extent3d { width: FLUID_SIM_W, height: FLUID_SIM_H, depth_or_array_layers: 1 },
        );

        // Fluid params buffer
        let fluid_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fluid-params"),
            size: std::mem::size_of::<FluidParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // --- Fluid bind group layouts ---
        let fluid_sim_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fluid-sim-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format: wgpu::TextureFormat::Rg32Float, view_dimension: wgpu::TextureViewDimension::D2 }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format: wgpu::TextureFormat::R32Float, view_dimension: wgpu::TextureViewDimension::D2 }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 5, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 6, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        let fluid_pressure_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fluid-pressure-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format: wgpu::TextureFormat::R32Float, view_dimension: wgpu::TextureViewDimension::D2 }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        let fluid_dye_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fluid-dye-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format: wgpu::TextureFormat::Rgba16Float, view_dimension: wgpu::TextureViewDimension::D2 }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 5, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            ],
        });

        // --- Fluid shader modules ---
        let fluid_sim_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fluid-sim"), source: wgpu::ShaderSource::Wgsl(include_str!("fluid.wgsl").into()),
        });
        let fluid_pressure_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fluid-pressure"), source: wgpu::ShaderSource::Wgsl(include_str!("fluid_pressure.wgsl").into()),
        });
        let fluid_dye_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fluid-dye"), source: wgpu::ShaderSource::Wgsl(include_str!("fluid_dye.wgsl").into()),
        });

        // --- Fluid pipelines ---
        let make_fluid_sim_pipeline = |label: &str, entry: &str| -> wgpu::ComputePipeline {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(label), bind_group_layouts: &[&fluid_sim_bgl], push_constant_ranges: &[],
                })),
                module: &fluid_sim_shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let fluid_p_curl = make_fluid_sim_pipeline("fluid-curl", "main_curl");
        let fluid_p_vorticity = make_fluid_sim_pipeline("fluid-vorticity", "main_vorticity");
        let fluid_p_divergence = make_fluid_sim_pipeline("fluid-divergence", "main_divergence");
        let fluid_p_gradient = make_fluid_sim_pipeline("fluid-gradient", "main_gradient_subtract");
        let fluid_p_advect_vel = make_fluid_sim_pipeline("fluid-advect-vel", "main_advect_velocity");
        let fluid_p_splat = make_fluid_sim_pipeline("fluid-splat", "main_splat");

        let fluid_p_pressure = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("fluid-pressure"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("fluid-pressure-pl"), bind_group_layouts: &[&fluid_pressure_bgl], push_constant_ranges: &[],
            })),
            module: &fluid_pressure_shader,
            entry_point: Some("main_pressure"),
            compilation_options: Default::default(),
            cache: None,
        });
        let fluid_p_pressure_clear = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("fluid-pressure-clear"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("fluid-pressure-clear-pl"), bind_group_layouts: &[&fluid_pressure_bgl], push_constant_ranges: &[],
            })),
            module: &fluid_pressure_shader,
            entry_point: Some("main_pressure_clear"),
            compilation_options: Default::default(),
            cache: None,
        });
        let fluid_p_advect_dye = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("fluid-advect-dye"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("fluid-advect-dye-pl"), bind_group_layouts: &[&fluid_dye_bgl], push_constant_ranges: &[],
            })),
            module: &fluid_dye_shader,
            entry_point: Some("main_advect_dye"),
            compilation_options: Default::default(),
            cache: None,
        });

        // --- Fluid bind groups (fixed phase assignments) ---
        // curl: reads vel_A → writes curl
        let fluid_bg_curl = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fluid-bg-curl"), layout: &fluid_sim_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&fv_vel_a) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&fv_dummy_rg) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&fv_dummy_r) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&fv_curl) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&fv_obstacle) },
                wgpu::BindGroupEntry { binding: 5, resource: fluid_params_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 6, resource: grid_buffer.as_entire_binding() },
            ],
        });
        // vorticity: reads vel_A, curl → writes vel_B
        let fluid_bg_vorticity = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fluid-bg-vorticity"), layout: &fluid_sim_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&fv_vel_a) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&fv_vel_b) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&fv_curl) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&fv_dummy_r) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&fv_obstacle) },
                wgpu::BindGroupEntry { binding: 5, resource: fluid_params_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 6, resource: grid_buffer.as_entire_binding() },
            ],
        });
        // splat: reads vel_B → writes vel_A
        let fluid_bg_splat = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fluid-bg-splat"), layout: &fluid_sim_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&fv_vel_b) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&fv_vel_a) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&fv_dummy_r) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&fv_dummy_r_w) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&fv_obstacle) },
                wgpu::BindGroupEntry { binding: 5, resource: fluid_params_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 6, resource: grid_buffer.as_entire_binding() },
            ],
        });
        // divergence: reads vel_A → writes div
        let fluid_bg_divergence = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fluid-bg-divergence"), layout: &fluid_sim_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&fv_vel_a) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&fv_dummy_rg) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&fv_dummy_r) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&fv_div) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&fv_obstacle) },
                wgpu::BindGroupEntry { binding: 5, resource: fluid_params_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 6, resource: grid_buffer.as_entire_binding() },
            ],
        });
        // gradient: reads vel_A, pres_A → writes vel_B
        let fluid_bg_gradient = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fluid-bg-gradient"), layout: &fluid_sim_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&fv_vel_a) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&fv_vel_b) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&fv_pres_b) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&fv_dummy_r) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&fv_obstacle) },
                wgpu::BindGroupEntry { binding: 5, resource: fluid_params_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 6, resource: grid_buffer.as_entire_binding() },
            ],
        });
        // advect_vel: reads vel_B → writes vel_A
        let fluid_bg_advect_vel = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fluid-bg-advect-vel"), layout: &fluid_sim_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&fv_vel_b) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&fv_vel_a) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&fv_dummy_r) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&fv_dummy_r_w) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&fv_obstacle) },
                wgpu::BindGroupEntry { binding: 5, resource: fluid_params_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 6, resource: grid_buffer.as_entire_binding() },
            ],
        });

        // Pressure bind groups
        // pressure_clear: A→B (same config as pressure[0])
        let fluid_bg_pressure_clear = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fluid-bg-pressure-clear"), layout: &fluid_pressure_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&fv_pres_b) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&fv_pres_a) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&fv_div) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&fv_obstacle) },
                wgpu::BindGroupEntry { binding: 4, resource: fluid_params_buffer.as_entire_binding() },
            ],
        });
        // pressure[0]: A→B
        let fluid_bg_pressure_ab = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fluid-bg-pressure-ab"), layout: &fluid_pressure_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&fv_pres_a) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&fv_pres_b) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&fv_div) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&fv_obstacle) },
                wgpu::BindGroupEntry { binding: 4, resource: fluid_params_buffer.as_entire_binding() },
            ],
        });
        let fluid_bg_pressure_ba = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fluid-bg-pressure-ba"), layout: &fluid_pressure_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&fv_pres_b) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&fv_pres_a) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&fv_div) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&fv_obstacle) },
                wgpu::BindGroupEntry { binding: 4, resource: fluid_params_buffer.as_entire_binding() },
            ],
        });

        // Dye advection bind groups
        let fluid_bg_advect_dye_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fluid-bg-advect-dye-0"), layout: &fluid_dye_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&fv_dye_a) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&fv_dye_b) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&fv_vel_a) },
                wgpu::BindGroupEntry { binding: 3, resource: fluid_params_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: grid_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(&fv_obstacle) },
            ],
        });
        let fluid_bg_advect_dye_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fluid-bg-advect-dye-1"), layout: &fluid_dye_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&fv_dye_b) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&fv_dye_a) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&fv_vel_a) },
                wgpu::BindGroupEntry { binding: 3, resource: fluid_params_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: grid_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(&fv_obstacle) },
            ],
        });

        // --- Raytrace compute pipeline (now also reads the lightmap) ---
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("raytrace-compute"),
            source: wgpu::ShaderSource::Wgsl(include_str!("raytrace.wgsl").into()),
        });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("compute-bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Lightmap texture (sampled, bilinear)
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // Tree sprite atlas (storage buffer, read-only)
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Fluid dye texture (sampled, bilinear)
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 7,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 8,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 9,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Previous frame output for temporal reprojection
                    wgpu::BindGroupLayoutEntry {
                        binding: 10,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });

        // Raytrace shader samples the final lightmap result (texture A after even iterations)
        let lightmap_sample_view = lightmap_a.create_view(&wgpu::TextureViewDescriptor::default());

        // Fluid dye sampler (bilinear for smooth smoke overlay)
        let fluid_dye_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("fluid-dye-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // 4 compute bind groups: [dye_phase * 2 + output_phase]
        // output_phase 0: write A, read prev B; output_phase 1: write B, read prev A
        let compute_bind_group_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute-bg-0"), // dye_A, write output_A, read prev output_B
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&output_view_a) },
                wgpu::BindGroupEntry { binding: 1, resource: camera_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: grid_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&lightmap_sample_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&lightmap_sampler) },
                wgpu::BindGroupEntry { binding: 5, resource: sprite_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&fv_dye_a) },
                wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::Sampler(&fluid_dye_sampler) },
                wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::TextureView(&fv_vel_a) },
                wgpu::BindGroupEntry { binding: 9, resource: wgpu::BindingResource::TextureView(&fv_pres_b) },
                wgpu::BindGroupEntry { binding: 10, resource: wgpu::BindingResource::TextureView(&output_view_b) },
            ],
        });
        let compute_bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute-bg-1"), // dye_A, write output_B, read prev output_A
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&output_view_b) },
                wgpu::BindGroupEntry { binding: 1, resource: camera_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: grid_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&lightmap_sample_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&lightmap_sampler) },
                wgpu::BindGroupEntry { binding: 5, resource: sprite_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&fv_dye_a) },
                wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::Sampler(&fluid_dye_sampler) },
                wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::TextureView(&fv_vel_a) },
                wgpu::BindGroupEntry { binding: 9, resource: wgpu::BindingResource::TextureView(&fv_pres_b) },
                wgpu::BindGroupEntry { binding: 10, resource: wgpu::BindingResource::TextureView(&output_view_a) },
            ],
        });
        let compute_bind_group_2 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute-bg-2"), // dye_B, write output_A, read prev output_B
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&output_view_a) },
                wgpu::BindGroupEntry { binding: 1, resource: camera_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: grid_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&lightmap_sample_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&lightmap_sampler) },
                wgpu::BindGroupEntry { binding: 5, resource: sprite_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&fv_dye_b) },
                wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::Sampler(&fluid_dye_sampler) },
                wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::TextureView(&fv_vel_a) },
                wgpu::BindGroupEntry { binding: 9, resource: wgpu::BindingResource::TextureView(&fv_pres_b) },
                wgpu::BindGroupEntry { binding: 10, resource: wgpu::BindingResource::TextureView(&output_view_b) },
            ],
        });
        let compute_bind_group_3 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute-bg-3"), // dye_B, write output_B, read prev output_A
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&output_view_b) },
                wgpu::BindGroupEntry { binding: 1, resource: camera_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: grid_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&lightmap_sample_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&lightmap_sampler) },
                wgpu::BindGroupEntry { binding: 5, resource: sprite_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&fv_dye_b) },
                wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::Sampler(&fluid_dye_sampler) },
                wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::TextureView(&fv_vel_a) },
                wgpu::BindGroupEntry { binding: 9, resource: wgpu::BindingResource::TextureView(&fv_pres_b) },
                wgpu::BindGroupEntry { binding: 10, resource: wgpu::BindingResource::TextureView(&output_view_a) },
            ],
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compute-pl"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        let compute_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("raytrace-pipeline"),
                layout: Some(&compute_pipeline_layout),
                module: &compute_shader,
                entry_point: Some("main_raytrace"),
                compilation_options: Default::default(),
                cache: None,
            });

        // --- Render (blit) pipeline ---
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blit-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("blit.wgsl").into()),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("blit-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("blit-bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let render_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blit-bg-a"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&output_view_a),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let render_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blit-bg-b"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&output_view_b),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("blit-pl"),
                bind_group_layouts: &[&render_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("blit-pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &blit_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &blit_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        // --- egui setup ---
        let egui_ctx = egui::Context::default();
        let egui_winit_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            surface_format,
            egui_wgpu::RendererOptions::default(),
        );

        let debug_readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("debug-readback"),
            size: 256, // one row with COPY_BYTES_PER_ROW_ALIGNMENT padding
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        self.egui_state = Some(EguiState {
            ctx: egui_ctx,
            winit_state: egui_winit_state,
            renderer: egui_renderer,
        });

        self.gfx = Some(GfxState {
            surface,
            device,
            queue,
            config,
            surface_format,
            lightmap_seed_pipeline,
            lightmap_seed_bind_groups: [lightmap_seed_bind_group_a, lightmap_seed_bind_group_b],
            lightmap_prop_pipeline,
            lightmap_prop_bind_groups: [prop_bg_0, prop_bg_1],
            lightmap_textures: [lightmap_a, lightmap_b],
            compute_pipeline,
            compute_bind_groups: [compute_bind_group_0, compute_bind_group_1, compute_bind_group_2, compute_bind_group_3],
            render_pipeline,
            render_bind_groups: [render_bind_group_a, render_bind_group_b],
            output_textures: [output_texture_a, output_texture_b],
            camera_buffer,
            grid_buffer,
            sprite_buffer,
            // Fluid simulation GPU resources
            fluid_params_buffer,
            fluid_vel: [fluid_vel_a, fluid_vel_b],
            fluid_pres: [fluid_pres_a, fluid_pres_b],
            fluid_div,
            fluid_curl: fluid_curl_tex,
            fluid_dye: [fluid_dye_a, fluid_dye_b],
            fluid_obstacle: fluid_obstacle_tex,
            fluid_dummy_rg,
            fluid_dummy_r,
            fluid_dummy_r_w,
            fluid_p_curl,
            fluid_p_vorticity,
            fluid_p_divergence,
            fluid_p_gradient,
            fluid_p_advect_vel,
            fluid_p_splat,
            fluid_p_pressure,
            fluid_p_pressure_clear,
            fluid_p_advect_dye,
            fluid_bg_curl,
            fluid_bg_vorticity,
            fluid_bg_splat,
            fluid_bg_divergence,
            fluid_bg_gradient,
            fluid_bg_advect_vel,
            fluid_bg_pressure: [fluid_bg_pressure_ab, fluid_bg_pressure_ba],
            fluid_bg_pressure_clear,
            fluid_bg_advect_dye: [fluid_bg_advect_dye_0, fluid_bg_advect_dye_1],
            debug_readback_buffer,
        });

        self.window = Some(window);
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        let width = new_size.width.max(1);
        let height = new_size.height.max(1);

        let gfx = self.gfx.as_mut().unwrap();

        gfx.config.width = width;
        gfx.config.height = height;
        gfx.surface.configure(&gfx.device, &gfx.config);

        let render_w = ((width as f32) * self.render_scale).max(1.0) as u32;
        let render_h = ((height as f32) * self.render_scale).max(1.0) as u32;

        // Scale zoom to maintain the same view when window resizes
        let old_min = self.camera.screen_w.min(self.camera.screen_h);
        let new_min = (render_w as f32).min(render_h as f32);
        if old_min > 0.0 {
            self.camera.zoom *= new_min / old_min;
        }

        self.camera.screen_w = render_w as f32;
        self.camera.screen_h = render_h as f32;

        // Recreate both output textures at render resolution (ping-pong)
        let output_desc = wgpu::TextureDescriptor {
            label: Some("output-texture-a"),
            size: wgpu::Extent3d {
                width: render_w,
                height: render_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        gfx.output_textures[0] = gfx.device.create_texture(&output_desc);
        gfx.output_textures[1] = gfx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("output-texture-b"),
            ..output_desc
        });

        let output_view_a = gfx
            .output_textures[0]
            .create_view(&wgpu::TextureViewDescriptor::default());
        let output_view_b = gfx
            .output_textures[1]
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Rebuild bind groups with new texture view
        let lightmap_sample_view = gfx
            .lightmap_textures[0]
            .create_view(&wgpu::TextureViewDescriptor::default());
        let lightmap_sampler = gfx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("lightmap-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let compute_bgl = gfx.compute_pipeline.get_bind_group_layout(0);
        let fluid_dye_sampler = gfx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("fluid-dye-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let fv_dye_a = gfx.fluid_dye[0].create_view(&wgpu::TextureViewDescriptor::default());
        let fv_dye_b = gfx.fluid_dye[1].create_view(&wgpu::TextureViewDescriptor::default());
        let fv_vel_a_view = gfx.fluid_vel[0].create_view(&wgpu::TextureViewDescriptor::default());
        let fv_pres_b_view = gfx.fluid_pres[1].create_view(&wgpu::TextureViewDescriptor::default());
        gfx.compute_bind_groups = [
            gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("compute-bg-0"), // dye_A, write output_A, read prev output_B
                layout: &compute_bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&output_view_a) },
                    wgpu::BindGroupEntry { binding: 1, resource: gfx.camera_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: gfx.grid_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&lightmap_sample_view) },
                    wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&lightmap_sampler) },
                    wgpu::BindGroupEntry { binding: 5, resource: gfx.sprite_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&fv_dye_a) },
                    wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::Sampler(&fluid_dye_sampler) },
                    wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::TextureView(&fv_vel_a_view) },
                    wgpu::BindGroupEntry { binding: 9, resource: wgpu::BindingResource::TextureView(&fv_pres_b_view) },
                    wgpu::BindGroupEntry { binding: 10, resource: wgpu::BindingResource::TextureView(&output_view_b) },
                ],
            }),
            gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("compute-bg-1"), // dye_A, write output_B, read prev output_A
                layout: &compute_bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&output_view_b) },
                    wgpu::BindGroupEntry { binding: 1, resource: gfx.camera_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: gfx.grid_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&lightmap_sample_view) },
                    wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&lightmap_sampler) },
                    wgpu::BindGroupEntry { binding: 5, resource: gfx.sprite_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&fv_dye_a) },
                    wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::Sampler(&fluid_dye_sampler) },
                    wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::TextureView(&fv_vel_a_view) },
                    wgpu::BindGroupEntry { binding: 9, resource: wgpu::BindingResource::TextureView(&fv_pres_b_view) },
                    wgpu::BindGroupEntry { binding: 10, resource: wgpu::BindingResource::TextureView(&output_view_a) },
                ],
            }),
            gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("compute-bg-2"), // dye_B, write output_A, read prev output_B
                layout: &compute_bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&output_view_a) },
                    wgpu::BindGroupEntry { binding: 1, resource: gfx.camera_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: gfx.grid_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&lightmap_sample_view) },
                    wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&lightmap_sampler) },
                    wgpu::BindGroupEntry { binding: 5, resource: gfx.sprite_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&fv_dye_b) },
                    wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::Sampler(&fluid_dye_sampler) },
                    wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::TextureView(&fv_vel_a_view) },
                    wgpu::BindGroupEntry { binding: 9, resource: wgpu::BindingResource::TextureView(&fv_pres_b_view) },
                    wgpu::BindGroupEntry { binding: 10, resource: wgpu::BindingResource::TextureView(&output_view_b) },
                ],
            }),
            gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("compute-bg-3"), // dye_B, write output_B, read prev output_A
                layout: &compute_bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&output_view_b) },
                    wgpu::BindGroupEntry { binding: 1, resource: gfx.camera_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: gfx.grid_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&lightmap_sample_view) },
                    wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&lightmap_sampler) },
                    wgpu::BindGroupEntry { binding: 5, resource: gfx.sprite_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&fv_dye_b) },
                    wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::Sampler(&fluid_dye_sampler) },
                    wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::TextureView(&fv_vel_a_view) },
                    wgpu::BindGroupEntry { binding: 9, resource: wgpu::BindingResource::TextureView(&fv_pres_b_view) },
                    wgpu::BindGroupEntry { binding: 10, resource: wgpu::BindingResource::TextureView(&output_view_a) },
                ],
            }),
        ];

        let render_bgl = gfx.render_pipeline.get_bind_group_layout(0);
        let sampler = gfx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("blit-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        gfx.render_bind_groups = [
            gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("blit-bg-a"),
                layout: &render_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&output_view_a),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            }),
            gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("blit-bg-b"),
                layout: &render_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&output_view_b),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            }),
        ];
    }

    fn render(&mut self) {
        // Check if render scale changed — trigger resize to recreate output texture
        {
            let gfx = self.gfx.as_ref().unwrap();
            let expected_w = ((gfx.config.width as f32) * self.render_scale).max(1.0) as u32;
            let expected_h = ((gfx.config.height as f32) * self.render_scale).max(1.0) as u32;
            if expected_w != self.camera.screen_w as u32 || expected_h != self.camera.screen_h as u32 {
                let size = PhysicalSize::new(gfx.config.width, gfx.config.height);
                let _ = gfx;
                self.resize(size);
            }
        }

        // Advance time + FPS tracking
        let now = Instant::now();
        let dt = now.elapsed_secs_since(&self.last_frame_time);
        self.last_frame_time = now;

        self.frame_count += 1;
        self.fps_accum += dt;
        if self.fps_accum >= 0.5 {
            self.fps_display = self.frame_count as f32 / self.fps_accum;
            self.frame_count = 0;
            self.fps_accum = 0.0;
        }

        if !self.time_paused {
            self.time_of_day += dt * self.time_speed;
            // Wrap around
            while self.time_of_day >= DAY_DURATION {
                self.time_of_day -= DAY_DURATION;
            }
            while self.time_of_day < 0.0 {
                self.time_of_day += DAY_DURATION;
            }
        }

        // Save previous camera state for temporal reprojection
        // Set prev camera from LAST frame's values (not current — otherwise delta is always 0)
        self.camera.prev_center_x = self.prev_cam_x;
        self.camera.prev_center_y = self.prev_cam_y;
        self.camera.prev_zoom = self.prev_cam_zoom;
        self.camera.prev_time = self.prev_cam_time;

        self.camera.time = self.time_of_day;

        // Precompute sun on CPU (avoids trig per pixel in shader)
        {
            let t = (self.time_of_day / DAY_DURATION).rem_euclid(1.0);
            let dawn = 0.15f32;
            let dusk = 0.85f32;
            let day_t = ((t - dawn) / (dusk - dawn)).clamp(0.0, 1.0);
            let angle = day_t * std::f32::consts::PI;
            self.camera.sun_dir_x = -angle.cos();
            self.camera.sun_dir_y = -angle.sin() * 0.6 - 0.2;
            let noon = (day_t * std::f32::consts::PI).sin();
            let edge = smoothstep_f32(0.0, 0.15, day_t) * smoothstep_f32(1.0, 0.85, day_t);
            self.camera.sun_elevation = (1.0 + 3.0 * noon) * edge;
            let fade_in = smoothstep_f32(dawn - 0.05, dawn + 0.05, t);
            let fade_out = smoothstep_f32(dusk + 0.05, dusk - 0.05, t);
            let intensity = fade_in * fade_out;
            self.camera.sun_intensity = intensity;
            let dawn_col = [1.0f32, 0.55, 0.25];
            let noon_col = [1.0f32, 0.97, 0.90];
            let s = smoothstep_f32(0.0, 0.6, noon);
            self.camera.sun_color_r = (dawn_col[0] + (noon_col[0] - dawn_col[0]) * s) * intensity;
            self.camera.sun_color_g = (dawn_col[1] + (noon_col[1] - dawn_col[1]) * s) * intensity;
            self.camera.sun_color_b = (dawn_col[2] + (noon_col[2] - dawn_col[2]) * s) * intensity;
            let night_amb = [0.008f32, 0.008, 0.02];
            let day_amb = [0.10f32, 0.10, 0.13];
            self.camera.ambient_r = night_amb[0] + (day_amb[0] - night_amb[0]) * intensity;
            self.camera.ambient_g = night_amb[1] + (day_amb[1] - night_amb[1]) * intensity;
            self.camera.ambient_b = night_amb[2] + (day_amb[2] - night_amb[2]) * intensity;
        }

        self.camera.fluid_overlay = match self.fluid_overlay {
            FluidOverlay::None => 0.0,
            FluidOverlay::Smoke => 1.0,
            FluidOverlay::Velocity => 2.0,
            FluidOverlay::Pressure => 3.0,
            FluidOverlay::O2 => 4.0,
            FluidOverlay::CO2 => 5.0,
        };
        let prev_glow = self.camera.enable_prox_glow;
        let prev_bleed = self.camera.enable_dir_bleed;
        self.camera.enable_prox_glow = if self.enable_prox_glow { 1.0 } else { 0.0 };
        self.camera.enable_dir_bleed = if self.enable_dir_bleed { 1.0 } else { 0.0 };

        // Force refresh when grid changes or render settings toggle
        // Persist for several frames so lightmap has time to propagate changes
        let settings_changed = (self.camera.enable_prox_glow - prev_glow).abs() > 0.5
            || (self.camera.enable_dir_bleed - prev_bleed).abs() > 0.5;
        if self.grid_dirty || settings_changed {
            self.camera.force_refresh = 5.0; // refresh for 5 frames
        } else if self.camera.force_refresh > 0.5 {
            self.camera.force_refresh -= 1.0;
        }

        let gfx = self.gfx.as_ref().unwrap();

        // Re-upload grid if dirty (door toggled etc.)
        if self.grid_dirty {
            gfx.queue.write_buffer(
                &gfx.grid_buffer,
                0,
                bytemuck::cast_slice(&self.grid_data),
            );
            // Rebuild fluid obstacle field
            let obs_data = build_obstacle_field(&self.grid_data);
            gfx.queue.write_texture(
                wgpu::TexelCopyTextureInfo { texture: &gfx.fluid_obstacle, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                &obs_data,
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(FLUID_SIM_W), rows_per_image: Some(FLUID_SIM_H) },
                wgpu::Extent3d { width: FLUID_SIM_W, height: FLUID_SIM_H, depth_or_array_layers: 1 },
            );
            self.grid_dirty = false;
        }

        // Upload fluid params
        self.fluid_params.time = self.time_of_day;
        self.fluid_params.dt = (1.0 / 60.0) * self.fluid_speed;
        self.fluid_params.splat_active = if self.fluid_mouse_active { 1.0 } else { 0.0 };
        gfx.queue.write_buffer(&gfx.fluid_params_buffer, 0, bytemuck::bytes_of(&self.fluid_params));

        // Compute lightmap viewport bounds (grid coordinates with margin for light propagation)
        let half_w = self.camera.screen_w * 0.5 / self.camera.zoom;
        let half_h = self.camera.screen_h * 0.5 / self.camera.zoom;
        let lm_margin = 14.0; // tiles of margin (>= max light radius)
        self.camera.lm_vp_min_x = (self.camera.center_x - half_w - lm_margin).max(0.0);
        self.camera.lm_vp_min_y = (self.camera.center_y - half_h - lm_margin).max(0.0);
        self.camera.lm_vp_max_x = (self.camera.center_x + half_w + lm_margin).min(GRID_W as f32);
        self.camera.lm_vp_max_y = (self.camera.center_y + half_h + lm_margin).min(GRID_H as f32);

        // Update camera uniform
        gfx.queue
            .write_buffer(&gfx.camera_buffer, 0, bytemuck::bytes_of(&self.camera));

        let frame = match gfx.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                let size = self.window.as_ref().unwrap().inner_size();
                self.resize(size);
                return;
            }
            Err(e) => {
                log::error!("Surface error: {:?}", e);
                return;
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Pre-compute blueprint preview data (before egui borrows self)
        let blueprint_tiles: Vec<((i32, i32), bool)> = if self.build_tool != BuildTool::None {
            let (hwx, hwy) = self.hover_world;
            let hbx = hwx.floor() as i32;
            let hby = hwy.floor() as i32;
            let tiles: Vec<(i32, i32)> = match self.build_tool {
                BuildTool::Bench => self.bench_tiles(hbx, hby, self.build_rotation).to_vec(),
                _ => vec![(hbx, hby)],
            };
            let on_furniture = self.build_tool == BuildTool::TableLamp;
            let on_wall = self.build_tool == BuildTool::Fan;
            tiles.iter().map(|&(tx, ty)| {
                if on_wall {
                    let valid = if tx >= 0 && ty >= 0 && tx < GRID_W as i32 && ty < GRID_H as i32 {
                        let bidx = (ty as u32 * GRID_W + tx as u32) as usize;
                        let b = self.grid_data[bidx];
                        let bbt = b & 0xFF;
                        let bbh = (b >> 8) & 0xFF;
                        (bbt == 1 || bbt == 4) && bbh > 0
                    } else { false };
                    ((tx, ty), valid)
                } else {
                    ((tx, ty), self.can_place_on(tx, ty, on_furniture))
                }
            }).collect()
        } else {
            vec![]
        };
        let bp_cam = (self.camera.center_x, self.camera.center_y, self.camera.zoom, self.camera.screen_w, self.camera.screen_h);
        let bp_ppp = self.window.as_ref().map(|w| w.scale_factor() as f32).unwrap_or(1.0);

        // --- egui frame ---
        let egui_state = self.egui_state.as_mut().unwrap();
        let window = self.window.as_ref().unwrap();
        let raw_input = egui_state.winit_state.take_egui_input(window);
        egui_state.ctx.begin_pass(raw_input);

        // Draw UI
        // Version label in top-right corner
        egui::Area::new(egui::Id::new("version_label"))
            .anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0])
            .show(&egui_state.ctx, |ui| {
                ui.label(egui::RichText::new(format!("v38 | {:.0} fps", self.fps_display)).color(egui::Color32::from_rgba_premultiplied(200, 200, 200, 180)).size(14.0));
            });

        let mut time_val = self.time_of_day;
        let mut paused = self.time_paused;
        let mut speed = self.time_speed;
        let mut zoom = self.camera.zoom;
        let mut glass_light = self.camera.glass_light_mul;
        let mut indoor_glow = self.camera.indoor_glow_mul;
        let mut bleed = self.camera.light_bleed_mul;
        let mut foliage_opacity = self.camera.foliage_opacity;
        let mut foliage_variation = self.camera.foliage_variation;
        let mut oblique = self.camera.oblique_strength;
                let base_zoom = (self.camera.screen_w / 32.0).min(self.camera.screen_h / 32.0);
        egui::Window::new("Time Control")
            .default_pos([10.0, 10.0])
            .default_width(300.0)
            .resizable(false)
            .show(&egui_state.ctx, |ui| {
                // Time of day as hours:minutes for display
                let day_frac = time_val / DAY_DURATION;
                let hours = (day_frac * 24.0) as u32;
                let minutes = ((day_frac * 24.0 - hours as f32) * 60.0) as u32;

                // Determine phase
                let phase = if day_frac < 0.15 {
                    "Night"
                } else if day_frac < 0.25 {
                    "Dawn"
                } else if day_frac < 0.75 {
                    "Day"
                } else if day_frac < 0.85 {
                    "Dusk"
                } else {
                    "Night"
                };

                ui.label(format!("{:02}:{:02} - {}", hours, minutes, phase));
                ui.add(egui::Slider::new(&mut time_val, 0.0..=DAY_DURATION)
                    .text("Time")
                    .show_value(false));
                ui.horizontal(|ui| {
                    if ui.button(if paused { "Play" } else { "Pause" }).clicked() {
                        paused = !paused;
                    }
                    ui.add(egui::Slider::new(&mut speed, 0.1..=5.0)
                        .text("Speed")
                        .logarithmic(true));
                });
                ui.horizontal(|ui| {
                    if ui.button("Night").clicked()  { time_val = DAY_DURATION * 0.0; paused = true; }
                    if ui.button("Dawn").clicked()   { time_val = DAY_DURATION * 0.18; paused = true; }
                    if ui.button("Day").clicked()    { time_val = DAY_DURATION * 0.5; paused = true; }
                    if ui.button("Dusk").clicked()   { time_val = DAY_DURATION * 0.82; paused = true; }
                });

                ui.separator();

                let zoom_pct = zoom / base_zoom * 100.0;
                ui.label(format!("Zoom: {:.0}%", zoom_pct));
                ui.add(egui::Slider::new(&mut zoom, base_zoom * 0.05..=base_zoom * 8.0)
                    .text("Zoom")
                    .show_value(false)
                    .logarithmic(true));
                if ui.button("Reset zoom").clicked() {
                    zoom = base_zoom;
                }
                let mut rs = self.render_scale;
                ui.add(egui::Slider::new(&mut rs, 0.15..=1.0)
                    .text("Render quality")
                    .step_by(0.05));
                self.render_scale = rs;

                ui.separator();
                ui.label("Lighting");
                ui.add(egui::Slider::new(&mut glass_light, 0.0..=0.5)
                    .text("Window glow")
                    .step_by(0.01));
                ui.add(egui::Slider::new(&mut indoor_glow, 0.0..=1.0)
                    .text("Indoor glow")
                    .step_by(0.01));
                ui.add(egui::Slider::new(&mut bleed, 0.0..=2.0)
                    .text("Light bleed")
                    .step_by(0.01));

                ui.separator();
                ui.label("Foliage Shadows");
                ui.add(egui::Slider::new(&mut foliage_opacity, 0.0..=1.0)
                    .text("Canopy density")
                    .step_by(0.01));
                ui.add(egui::Slider::new(&mut foliage_variation, 0.0..=1.0)
                    .text("Tree variation")
                    .step_by(0.01));

                ui.separator();
                ui.label("Fluid Sim");
                let mut fluid_spd = self.fluid_speed;
                ui.add(egui::Slider::new(&mut fluid_spd, 0.0..=5.0)
                    .text("Fluid speed")
                    .step_by(0.1));
                self.fluid_speed = fluid_spd;
                ui.horizontal(|ui| {
                    ui.label("Wind:");
                    let mut wx = self.fluid_params.wind_x;
                    let mut wy = self.fluid_params.wind_y;
                    ui.add(egui::Slider::new(&mut wx, -20.0..=20.0).text("X").step_by(0.5));
                    ui.add(egui::Slider::new(&mut wy, -20.0..=20.0).text("Y").step_by(0.5));
                    self.fluid_params.wind_x = wx;
                    self.fluid_params.wind_y = wy;
                });
                let mut sr = self.fluid_params.smoke_rate;
                ui.add(egui::Slider::new(&mut sr, 0.0..=1.0)
                    .text("Smoke rate")
                    .step_by(0.05));
                self.fluid_params.smoke_rate = sr;
                let mut fs = self.fluid_params.fan_speed;
                ui.add(egui::Slider::new(&mut fs, 0.0..=50.0)
                    .text("Fan speed")
                    .step_by(1.0));
                self.fluid_params.fan_speed = fs;

                ui.separator();
                ui.label("Camera");
                ui.add(egui::Slider::new(&mut oblique, 0.0..=0.3)
                    .text("Wall face tilt")
                    .step_by(0.005));
            });
        self.time_of_day = time_val;
        self.time_paused = paused;
        self.time_speed = speed;
        self.camera.zoom = zoom;
        self.camera.glass_light_mul = glass_light;
        self.camera.indoor_glow_mul = indoor_glow;
        self.camera.light_bleed_mul = bleed;
        self.camera.foliage_opacity = foliage_opacity;
        self.camera.foliage_variation = foliage_variation;
        self.camera.oblique_strength = oblique;

        // Build toolbar — floating bottom-center bar
        egui::Area::new(egui::Id::new("build_bar"))
            .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -20.0])
            .show(&egui_state.ctx, |ui| {
                egui::Frame::window(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Build:");
                        let tool = &mut self.build_tool;
                        if ui.selectable_label(*tool == BuildTool::Fireplace, "Fireplace").clicked() {
                            *tool = if *tool == BuildTool::Fireplace { BuildTool::None } else { BuildTool::Fireplace };
                        }
                        if ui.selectable_label(*tool == BuildTool::ElectricLight, "Electric Light").clicked() {
                            *tool = if *tool == BuildTool::ElectricLight { BuildTool::None } else { BuildTool::ElectricLight };
                        }
                        if ui.selectable_label(*tool == BuildTool::Bench, "Bench").clicked() {
                            *tool = if *tool == BuildTool::Bench { BuildTool::None } else { BuildTool::Bench };
                        }
                        if ui.selectable_label(*tool == BuildTool::StandingLamp, "Floor Lamp").clicked() {
                            *tool = if *tool == BuildTool::StandingLamp { BuildTool::None } else { BuildTool::StandingLamp };
                        }
                        if ui.selectable_label(*tool == BuildTool::TableLamp, "Table Lamp").clicked() {
                            *tool = if *tool == BuildTool::TableLamp { BuildTool::None } else { BuildTool::TableLamp };
                        }
                        if ui.selectable_label(*tool == BuildTool::Fan, "Fan").clicked() {
                            *tool = if *tool == BuildTool::Fan { BuildTool::None } else { BuildTool::Fan };
                        }
                        if *tool != BuildTool::None {
                            ui.separator();
                            let hint = match *tool {
                                BuildTool::Bench => {
                                    let rot_label = if self.build_rotation == 0 { "H" } else { "V" };
                                    format!("Click to place | Q/E rotate [{}]", rot_label)
                                }
                                BuildTool::TableLamp => "Click on a bench to place".to_string(),
                                BuildTool::Fan => {
                                    let dir = match self.build_rotation { 0 => "N", 1 => "E", 2 => "S", _ => "W" };
                                    format!("Click wall to place | Q/E rotate [{}]", dir)
                                }
                                _ => "Click to place".to_string(),
                            };
                            ui.label(hint);
                        }
                    });
                });
            });

        // --- Overlay toggle bar (bottom-right) ---
        egui::Area::new(egui::Id::new("overlay_bar"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -20.0])
            .show(&egui_state.ctx, |ui| {
                egui::Frame::window(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Overlay:");
                        let ov = &mut self.fluid_overlay;
                        if ui.selectable_label(*ov == FluidOverlay::None, "Off").clicked() {
                            *ov = FluidOverlay::None;
                        }
                        if ui.selectable_label(*ov == FluidOverlay::Smoke, "Smoke").clicked() {
                            *ov = if *ov == FluidOverlay::Smoke { FluidOverlay::None } else { FluidOverlay::Smoke };
                        }
                        if ui.selectable_label(*ov == FluidOverlay::Velocity, "Velocity").clicked() {
                            *ov = if *ov == FluidOverlay::Velocity { FluidOverlay::None } else { FluidOverlay::Velocity };
                        }
                        if ui.selectable_label(*ov == FluidOverlay::Pressure, "Pressure").clicked() {
                            *ov = if *ov == FluidOverlay::Pressure { FluidOverlay::None } else { FluidOverlay::Pressure };
                        }
                        if ui.selectable_label(*ov == FluidOverlay::O2, "O2").clicked() {
                            *ov = if *ov == FluidOverlay::O2 { FluidOverlay::None } else { FluidOverlay::O2 };
                        }
                        if ui.selectable_label(*ov == FluidOverlay::CO2, "CO2").clicked() {
                            *ov = if *ov == FluidOverlay::CO2 { FluidOverlay::None } else { FluidOverlay::CO2 };
                        }
                        ui.separator();
                        let mut debug = self.debug_mode;
                        if ui.selectable_label(debug, "Debug").clicked() {
                            debug = !debug;
                        }
                        self.debug_mode = debug;
                        ui.separator();
                        if ui.selectable_label(self.enable_prox_glow, "Glow").clicked() {
                            self.enable_prox_glow = !self.enable_prox_glow;
                        }
                        if ui.selectable_label(self.enable_dir_bleed, "Bleed").clicked() {
                            self.enable_dir_bleed = !self.enable_dir_bleed;
                        }
                    });
                });
            });

        // Wind direction compass (bottom-right, above overlay bar)
        {
            let wx = self.fluid_params.wind_x;
            let wy = self.fluid_params.wind_y;
            let wind_mag = (wx * wx + wy * wy).sqrt();
            egui::Area::new(egui::Id::new("wind_compass"))
                .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -60.0])
                .interactable(false)
                .show(&egui_state.ctx, |ui| {
                    let size = 40.0;
                    let (resp, painter) = ui.allocate_painter(egui::Vec2::splat(size), egui::Sense::hover());
                    let center = resp.rect.center();
                    // Circle background
                    painter.circle_filled(center, size * 0.45, egui::Color32::from_rgba_unmultiplied(30, 30, 40, 180));
                    painter.circle_stroke(center, size * 0.45, egui::Stroke::new(1.0, egui::Color32::from_gray(100)));
                    if wind_mag > 0.1 {
                        let dir_x = wx / wind_mag;
                        let dir_y = wy / wind_mag;
                        let arrow_len = size * 0.35 * (wind_mag / 20.0).min(1.0).max(0.3);
                        let tip = center + egui::Vec2::new(dir_x * arrow_len, dir_y * arrow_len);
                        let tail = center - egui::Vec2::new(dir_x * arrow_len * 0.3, dir_y * arrow_len * 0.3);
                        // Arrow shaft
                        painter.line_segment([tail, tip], egui::Stroke::new(2.0, egui::Color32::from_rgb(200, 220, 255)));
                        // Arrowhead
                        let perp = egui::Vec2::new(-dir_y, dir_x) * arrow_len * 0.3;
                        let head_base = center + egui::Vec2::new(dir_x * arrow_len * 0.5, dir_y * arrow_len * 0.5);
                        painter.add(egui::Shape::convex_polygon(
                            vec![tip, head_base + perp, head_base - perp],
                            egui::Color32::from_rgb(200, 220, 255),
                            egui::Stroke::NONE,
                        ));
                    } else {
                        painter.text(center, egui::Align2::CENTER_CENTER, "·", egui::FontId::proportional(14.0), egui::Color32::from_gray(150));
                    }
                });
        }

        // Debug tooltip at cursor position
        if self.debug_mode {
            let (wx, wy) = self.hover_world;
            let bx = wx.floor() as i32;
            let by = wy.floor() as i32;

            let mut block_info = String::from("OOB");
            if bx >= 0 && by >= 0 && bx < GRID_W as i32 && by < GRID_H as i32 {
                let idx = (by as u32 * GRID_W + bx as u32) as usize;
                let block = self.grid_data[idx];
                let bt = block & 0xFF;
                let bh = (block >> 8) & 0xFF;
                let flags = (block >> 16) & 0xFF;
                let type_name = match bt {
                    0 => "air", 1 => "stone", 2 => "dirt", 3 => "water",
                    4 => "wall", 5 => "glass", 6 => "fire", 7 => "e-light",
                    8 => "tree", 9 => "bench", 10 => "floor-lamp", 11 => "table-lamp",
                    _ => "?",
                };
                let roof = if flags & 2 != 0 { " R" } else { "" };
                let door = if flags & 1 != 0 { if flags & 4 != 0 { " D:open" } else { " D:shut" } } else { "" };
                block_info = format!("{}(h{}){}{}", type_name, bh, roof, door);
            }

            let [smoke_r, o2, co2, _unused] = self.debug_fluid_density;
            let tip = format!(
                "({:.1}, {:.1})\n{}\nSmoke: {:.3}\nO2: {:.3}\nCO2: {:.3}",
                wx, wy, block_info, smoke_r, o2, co2
            );

            // Position tooltip near cursor
            let cursor_screen = egui_state.ctx.input(|i| {
                i.pointer.hover_pos().unwrap_or(egui::Pos2::ZERO)
            });
            egui::Area::new(egui::Id::new("debug_tooltip"))
                .fixed_pos(cursor_screen + egui::Vec2::new(15.0, 15.0))
                .interactable(false)
                .show(&egui_state.ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.label(egui::RichText::new(tip).monospace().size(12.0));
                    });
                });
        }

        // Blueprint preview — draw ghost overlay for placement
        if !blueprint_tiles.is_empty() {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;

            let painter = egui_state.ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("blueprint"),
            ));

            for &((tx, ty), valid) in &blueprint_tiles {
                let color = if valid {
                    egui::Color32::from_rgba_unmultiplied(80, 180, 255, 80)
                } else {
                    egui::Color32::from_rgba_unmultiplied(255, 60, 60, 80)
                };

                let wx0 = tx as f32;
                let wy0 = ty as f32;
                // World → physical pixels → logical points (egui coords)
                let sx0 = ((wx0 - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy0 = ((wy0 - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                let sx1 = ((wx0 + 1.0 - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy1 = ((wy0 + 1.0 - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;

                painter.rect_filled(
                    egui::Rect::from_min_max(egui::pos2(sx0, sy0), egui::pos2(sx1, sy1)),
                    0.0,
                    color,
                );

                // Fan direction arrow
                if self.build_tool == BuildTool::Fan {
                    let center = egui::pos2((sx0 + sx1) / 2.0, (sy0 + sy1) / 2.0);
                    let tile_size = (sx1 - sx0).max(1.0);
                    let (adx, ady) = match self.build_rotation {
                        0 => (0.0f32, -1.0f32),
                        1 => (1.0, 0.0),
                        2 => (0.0, 1.0),
                        _ => (-1.0, 0.0),
                    };
                    let arrow_len = tile_size * 0.8;
                    let tip = center + egui::Vec2::new(adx * arrow_len * 0.5, ady * arrow_len * 0.5);
                    let tail = center - egui::Vec2::new(adx * arrow_len * 0.5, ady * arrow_len * 0.5);
                    painter.line_segment([tail, tip], egui::Stroke::new(2.0, egui::Color32::WHITE));
                    let perp = egui::Vec2::new(-ady, adx) * arrow_len * 0.2;
                    let head_base = center + egui::Vec2::new(adx * arrow_len * 0.2, ady * arrow_len * 0.2);
                    painter.add(egui::Shape::convex_polygon(
                        vec![tip, head_base + perp, head_base - perp],
                        egui::Color32::WHITE, egui::Stroke::NONE,
                    ));
                }
            }
        }

        let egui_output = egui_state.ctx.end_pass();
        egui_state.winit_state.handle_platform_output(window, egui_output.platform_output.clone());

        let paint_jobs = egui_state.ctx.tessellate(egui_output.shapes, egui_state.ctx.pixels_per_point());
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [gfx.config.width, gfx.config.height],
            pixels_per_point: window.scale_factor() as f32,
        };

        let egui_state = self.egui_state.as_mut().unwrap();
        let gfx = self.gfx.as_ref().unwrap();

        // Upload egui textures
        for (id, image_delta) in &egui_output.textures_delta.set {
            egui_state.renderer.update_texture(&gfx.device, &gfx.queue, *id, image_delta);
        }
        let mut encoder = gfx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame-encoder"),
            });
        egui_state.renderer.update_buffers(&gfx.device, &gfx.queue, &mut encoder, &paint_jobs, &screen_descriptor);

        // Lightmap: viewport-culled propagation at 2x resolution
        self.lightmap_frame += 1;
        let need_lightmap = self.lightmap_frame >= LIGHTMAP_UPDATE_INTERVAL;
        if need_lightmap {
            self.lightmap_frame = 0;
            let lm_wg_x = (LIGHTMAP_W + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
            let lm_wg_y = (LIGHTMAP_H + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;

            // Seed pass: write to both textures (ensures clean state for ping-pong)
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("lightmap-seed-a"),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&gfx.lightmap_seed_pipeline);
                cpass.set_bind_group(0, &gfx.lightmap_seed_bind_groups[0], &[]);
                cpass.dispatch_workgroups(lm_wg_x, lm_wg_y, 1);
            }
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("lightmap-seed-b"),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&gfx.lightmap_seed_pipeline);
                cpass.set_bind_group(0, &gfx.lightmap_seed_bind_groups[1], &[]);
                cpass.dispatch_workgroups(lm_wg_x, lm_wg_y, 1);
            }

            // Propagation passes (viewport-culled in shader)
            for i in 0..LIGHTMAP_PROP_ITERATIONS {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("lightmap-propagate"),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&gfx.lightmap_prop_pipeline);
                cpass.set_bind_group(0, &gfx.lightmap_prop_bind_groups[(i as usize) % 2], &[]);
                cpass.dispatch_workgroups(lm_wg_x, lm_wg_y, 1);
            }
        }

        // --- Fluid simulation (every frame) ---
        let fluid_wg = (FLUID_SIM_W + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        let dye_wg = (FLUID_DYE_W + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;

        // 1. Curl
        { let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("fluid-curl"), timestamp_writes: None });
          p.set_pipeline(&gfx.fluid_p_curl); p.set_bind_group(0, &gfx.fluid_bg_curl, &[]); p.dispatch_workgroups(fluid_wg, fluid_wg, 1); }
        // 2. Vorticity
        { let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("fluid-vorticity"), timestamp_writes: None });
          p.set_pipeline(&gfx.fluid_p_vorticity); p.set_bind_group(0, &gfx.fluid_bg_vorticity, &[]); p.dispatch_workgroups(fluid_wg, fluid_wg, 1); }
        // 3. Splat
        { let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("fluid-splat"), timestamp_writes: None });
          p.set_pipeline(&gfx.fluid_p_splat); p.set_bind_group(0, &gfx.fluid_bg_splat, &[]); p.dispatch_workgroups(fluid_wg, fluid_wg, 1); }
        // 4. Divergence
        { let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("fluid-div"), timestamp_writes: None });
          p.set_pipeline(&gfx.fluid_p_divergence); p.set_bind_group(0, &gfx.fluid_bg_divergence, &[]); p.dispatch_workgroups(fluid_wg, fluid_wg, 1); }
        // 5. Pressure clear
        { let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("fluid-pres-clear"), timestamp_writes: None });
          p.set_pipeline(&gfx.fluid_p_pressure_clear); p.set_bind_group(0, &gfx.fluid_bg_pressure_clear, &[]); p.dispatch_workgroups(fluid_wg, fluid_wg, 1); }
        // 6. Pressure Jacobi (16 iterations, ping-pong)
        for i in 0..FLUID_PRESSURE_ITERS {
            let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("fluid-pressure"), timestamp_writes: None });
            p.set_pipeline(&gfx.fluid_p_pressure);
            p.set_bind_group(0, &gfx.fluid_bg_pressure[(i as usize) % 2], &[]);
            p.dispatch_workgroups(fluid_wg, fluid_wg, 1);
        }
        // 7. Gradient subtract
        { let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("fluid-gradient"), timestamp_writes: None });
          p.set_pipeline(&gfx.fluid_p_gradient); p.set_bind_group(0, &gfx.fluid_bg_gradient, &[]); p.dispatch_workgroups(fluid_wg, fluid_wg, 1); }
        // 8. Advect velocity
        { let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("fluid-advect-vel"), timestamp_writes: None });
          p.set_pipeline(&gfx.fluid_p_advect_vel); p.set_bind_group(0, &gfx.fluid_bg_advect_vel, &[]); p.dispatch_workgroups(fluid_wg, fluid_wg, 1); }
        // 9. Advect dye (512x512)
        { let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("fluid-advect-dye"), timestamp_writes: None });
          p.set_pipeline(&gfx.fluid_p_advect_dye); p.set_bind_group(0, &gfx.fluid_bg_advect_dye[self.fluid_dye_phase], &[]); p.dispatch_workgroups(dye_wg, dye_wg, 1); }
        // Flip dye phase for next frame
        self.fluid_dye_phase = 1 - self.fluid_dye_phase;

        // Debug: copy one dye texel at cursor position for readback
        if self.debug_mode {
            let (wx, wy) = self.hover_world;
            let dye_x = ((wx / GRID_W as f32) * FLUID_DYE_W as f32).clamp(0.0, (FLUID_DYE_W - 1) as f32) as u32;
            let dye_y = ((wy / GRID_H as f32) * FLUID_DYE_H as f32).clamp(0.0, (FLUID_DYE_H - 1) as f32) as u32;
            // The current readable dye is the one we just wrote to (dye phase was already flipped)
            let dye_idx = self.fluid_dye_phase; // after flip, this points to the fresh output
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &gfx.fluid_dye[dye_idx],
                    mip_level: 0,
                    origin: wgpu::Origin3d { x: dye_x, y: dye_y, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &gfx.debug_readback_buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256), // must be multiple of COPY_BYTES_PER_ROW_ALIGNMENT
                        rows_per_image: Some(1),
                    },
                },
                wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            );
            self.debug_fluid_readback_pending = true;
        }

        // Reset splat
        self.fluid_params.splat_active = 0.0;

        // Compute pass 2: raytrace (per-pixel, render resolution)
        let rt_w = self.camera.screen_w as u32;
        let rt_h = self.camera.screen_h as u32;
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("raytrace-pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&gfx.compute_pipeline);
            cpass.set_bind_group(0, &gfx.compute_bind_groups[self.fluid_dye_phase * 2 + self.output_phase], &[]);
            let wg_x = (rt_w + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
            let wg_y = (rt_h + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
            cpass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        // Render pass: blit output texture to screen
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blit-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // Blit the raytraced scene (read from current output phase)
            rpass.set_pipeline(&gfx.render_pipeline);
            rpass.set_bind_group(0, &gfx.render_bind_groups[self.output_phase], &[]);
            rpass.draw(0..3, 0..1);
        }

        // Flip output phase for next frame (ping-pong)
        self.output_phase = 1 - self.output_phase;

        // Store current camera for next frame's temporal reprojection
        self.prev_cam_x = self.camera.center_x;
        self.prev_cam_y = self.camera.center_y;
        self.prev_cam_zoom = self.camera.zoom;
        self.prev_cam_time = self.camera.time;

        // Render pass: egui overlay (separate encoder to avoid lifetime issues)
        {
            // Submit the main encoder FIRST (compute + blit) so the surface has content
            gfx.queue.submit(std::iter::once(encoder.finish()));

            // Debug: read back the dye texel
            if self.debug_fluid_readback_pending {
                self.debug_fluid_readback_pending = false;
                let buffer_slice = gfx.debug_readback_buffer.slice(..);
                buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
                gfx.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok();
                {
                    let data = buffer_slice.get_mapped_range();
                    let f16_data: &[u16] = bytemuck::cast_slice(&data);
                    // Convert f16 to f32 manually (half-float format)
                    for i in 0..4 {
                        self.debug_fluid_density[i] = half_to_f32(f16_data[i]);
                    }
                }
                gfx.debug_readback_buffer.unmap();
            }

            let mut egui_encoder = gfx
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("egui-encoder"),
                });
            let rpass = egui_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // preserve the blit output
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // SAFETY: egui_encoder lives until after rpass is dropped (submit consumes both)
            let mut rpass: wgpu::RenderPass<'static> = unsafe {
                std::mem::transmute(rpass)
            };
            egui_state.renderer.render(&mut rpass, &paint_jobs, &screen_descriptor);
            drop(rpass);
            gfx.queue.submit(std::iter::once(egui_encoder.finish()));
        }

        frame.present();

        // Free egui textures
        for id in &egui_output.textures_delta.free {
            egui_state.renderer.free_texture(id);
        }

        self.window.as_ref().unwrap().request_redraw();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        #[cfg(not(target_arch = "wasm32"))]
        let attrs = Window::default_attributes()
            .with_title("Spacewestern")
            .with_inner_size(PhysicalSize::new(1920u32, 1920u32));

        #[cfg(target_arch = "wasm32")]
        let attrs = {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            let canvas = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("rayworld-canvas")
                .unwrap()
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .unwrap();
            // Don't set inner_size — use the canvas dimensions from HTML
            Window::default_attributes()
                .with_title("Spacewestern")
                .with_canvas(Some(canvas))
        };

        let window = Arc::new(event_loop.create_window(attrs).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            pollster::block_on(self.init_gfx_async(window));
            self.last_frame_time = Instant::now();
        }

        #[cfg(target_arch = "wasm32")]
        {
            let window_clone = window.clone();
            // We need to move `self` data into the async block.
            // Store the window now, then spawn the async GPU init.
            self.window = Some(window.clone());

            // We can't move `self` into a Future, so use a raw pointer trick:
            // store a pointer to self and use it in the spawned future.
            let app_ptr = self as *mut App;
            wasm_bindgen_futures::spawn_local(async move {
                let app = unsafe { &mut *app_ptr };
                app.init_gfx_async(window_clone).await;
            });
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        // Always track cursor position and handle panning before egui
        if let WindowEvent::CursorMoved { position, .. } = &event {
            if self.mouse_pressed {
                let dx = position.x - self.last_mouse_x;
                let dy = position.y - self.last_mouse_y;
                if dx.abs() > 3.0 || dy.abs() > 3.0 {
                    self.mouse_dragged = true;
                }
                if self.mouse_dragged {
                    self.camera.center_x -= dx as f32 * self.render_scale / self.camera.zoom;
                    self.camera.center_y -= dy as f32 * self.render_scale / self.camera.zoom;
                    self.window.as_ref().unwrap().request_redraw();
                }
            }
            // Move dragged light source
            if self.dragging_light.is_some() {
                let (wx, wy) = self.screen_to_world(position.x, position.y);
                self.move_light_to(wx, wy);
            }
            self.last_mouse_x = position.x;
            self.last_mouse_y = position.y;
            self.hover_world = self.screen_to_world(position.x, position.y);
        }

        // Let egui process the event first
        if let Some(egui_state) = self.egui_state.as_mut() {
            let response = egui_state.winit_state.on_window_event(self.window.as_ref().unwrap(), &event);
            if response.consumed {
                return; // egui consumed this event (e.g., clicking on the UI)
            }
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state.is_pressed() {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Escape) => {
                            if self.build_tool != BuildTool::None {
                                self.build_tool = BuildTool::None;
                            } else {
                                event_loop.exit();
                            }
                        }
                        PhysicalKey::Code(KeyCode::KeyR) => {
                            if self.camera.show_roofs < 0.5 {
                                self.camera.show_roofs = 1.0;
                                log::info!("Roofs: opaque");
                            } else {
                                self.camera.show_roofs = 0.0;
                                log::info!("Roofs: transparent (see interior)");
                            }
                            self.window.as_ref().unwrap().request_redraw();
                        }
                        PhysicalKey::Code(KeyCode::Space) => {
                            self.time_paused = !self.time_paused;
                            log::info!("Time: {}", if self.time_paused { "paused" } else { "playing" });
                        }
                        PhysicalKey::Code(KeyCode::KeyQ) => {
                            if self.build_tool == BuildTool::Fan {
                                self.build_rotation = (self.build_rotation + 3) % 4;
                            } else {
                                self.build_rotation = (self.build_rotation + 1) % 2;
                            }
                        }
                        PhysicalKey::Code(KeyCode::KeyE) => {
                            if self.build_tool == BuildTool::Fan {
                                self.build_rotation = (self.build_rotation + 1) % 4;
                            } else {
                                self.build_rotation = (self.build_rotation + 1) % 2;
                            }
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y as f64,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y / 50.0,
                };
        let base_zoom = (self.camera.screen_w / 32.0).min(self.camera.screen_h / 32.0);
                if scroll > 0.0 {
                    self.camera.zoom *= 1.1;
                } else if scroll < 0.0 {
                    self.camera.zoom /= 1.1;
                }
                self.camera.zoom = self.camera.zoom.clamp(base_zoom * 0.05, base_zoom * 8.0);
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == winit::event::MouseButton::Left {
                    if state.is_pressed() {
                        self.mouse_pressed = true;
                        self.mouse_dragged = false;
                    } else {
                        // Mouse released — if we didn't drag, treat as a click
                        if !self.mouse_dragged {
                            let (wx, wy) = self.screen_to_world(self.last_mouse_x, self.last_mouse_y);
                            self.handle_click(wx, wy);
                        }
                        self.mouse_pressed = false;
                        self.mouse_dragged = false;
                    }
                }
                // Right-click: pick up / drop light sources
                if button == winit::event::MouseButton::Right {
                    if state.is_pressed() {
                        let (wx, wy) = self.screen_to_world(self.last_mouse_x, self.last_mouse_y);
                        self.try_pick_light(wx, wy);
                    } else {
                        self.drop_light();
                    }
                }
            }
            WindowEvent::CursorMoved { .. } => {
                // Handled in the pre-egui block above
            }
            WindowEvent::Resized(new_size) => {
                if self.gfx.is_some() {
                    self.resize(new_size);
                }
            }
            WindowEvent::RedrawRequested => {
                if self.gfx.is_some() {
                    self.render();
                }
            }
            _ => {}
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}

#[cfg(target_arch = "wasm32")]
fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Info).expect("Failed to init console_log");
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}
