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
// type: 0=air, 1=stone, 2=dirt, 3=water, 4=wall, 5=glass, 6=fireplace, 7=electric_light, 8=tree
// height: 0-255
// flags: bit0=is_door, bit1=has_roof, bit2=is_open
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

    // === House 1: Stone cottage (roofed, with windows) ===
    // Walls: x=10..29, y=10..25
    let h1_h = 3u8; // wall height
    let roof_flag = 2u8; // bit1 = has_roof
    // Top and bottom walls
    for x in 10..30 {
        set(&mut grid, x, 10, make_block(1, h1_h, 0));
        set(&mut grid, x, 25, make_block(1, h1_h, 0));
    }
    // Left and right walls
    for y in 10..26 {
        set(&mut grid, 10, y, make_block(1, h1_h, 0));
        set(&mut grid, 29, y, make_block(1, h1_h, 0));
    }
    // Windows (glass) in house 1 — top wall
    set(&mut grid, 14, 10, make_block(5, h1_h, 0)); // glass
    set(&mut grid, 15, 10, make_block(5, h1_h, 0));
    set(&mut grid, 24, 10, make_block(5, h1_h, 0));
    set(&mut grid, 25, 10, make_block(5, h1_h, 0));
    // Windows — bottom wall
    set(&mut grid, 14, 25, make_block(5, h1_h, 0));
    set(&mut grid, 15, 25, make_block(5, h1_h, 0));
    set(&mut grid, 24, 25, make_block(5, h1_h, 0));
    set(&mut grid, 25, 25, make_block(5, h1_h, 0));
    // Windows — side walls
    set(&mut grid, 10, 15, make_block(5, h1_h, 0));
    set(&mut grid, 10, 20, make_block(5, h1_h, 0));
    set(&mut grid, 29, 15, make_block(5, h1_h, 0));
    set(&mut grid, 29, 20, make_block(5, h1_h, 0));
    // Door
    set(&mut grid, 20, 10, make_block(4, 1, 1)); // door (low, flag=door)
    // Roof: fill interior with roofed floor
    for y in 11..25 {
        for x in 11..29 {
            set(&mut grid, x, y, make_block(2, 0, roof_flag)); // dirt floor + roof
        }
    }
    // Interior divider wall in house 1 (splits into two rooms)
    // Horizontal wall from x=11..28 at y=18, with a door at x=16
    for x in 11..29 {
        set(&mut grid, x, 18, make_block(1, h1_h, 0));
    }
    set(&mut grid, 16, 18, make_block(4, 1, 1)); // door in divider

    // Small alcove wall in north room (L-shaped room test)
    for y in 11..15 {
        set(&mut grid, 22, y, make_block(1, h1_h, 0));
    }

    // Fireplace in south room of house 1
    set(&mut grid, 19, 21, make_block(6, 1, roof_flag)); // fireplace (height 1, roofed)
    // Electric light in north room
    set(&mut grid, 15, 14, make_block(7, 0, roof_flag)); // electric light (height 0, roofed)

    // === House 2: Tall building (roofed, with windows) ===
    let h2_h = 5u8;
    for x in 35..55 {
        set(&mut grid, x, 30, make_block(1, h2_h, 0));
        set(&mut grid, x, 50, make_block(1, h2_h, 0));
    }
    for y in 30..51 {
        set(&mut grid, 35, y, make_block(1, h2_h, 0));
        set(&mut grid, 54, y, make_block(1, h2_h, 0));
    }
    // Windows — evenly spaced along each wall
    for &wx in &[38u32, 41, 44, 47, 50] {
        set(&mut grid, wx, 30, make_block(5, h2_h, 0));
        set(&mut grid, wx, 50, make_block(5, h2_h, 0));
    }
    for &wy in &[34u32, 38, 42, 46] {
        set(&mut grid, 35, wy, make_block(5, h2_h, 0));
        set(&mut grid, 54, wy, make_block(5, h2_h, 0));
    }
    // Door
    set(&mut grid, 45, 30, make_block(4, 1, 1));
    // Interior room divider wall
    for x in 36..54 {
        set(&mut grid, x, 40, make_block(1, h2_h, 0));
    }
    set(&mut grid, 44, 40, make_block(4, 1, 1)); // door in divider
    // Roof: fill interior
    for y in 31..50 {
        for x in 36..54 {
            let existing = grid[(y * w + x) as usize];
            if block_type_rs(existing) == 0 || block_type_rs(existing) == 2 {
                set(&mut grid, x, y, make_block(2, 0, roof_flag));
            }
        }
    }

    // === Small shed (low walls, glass roof/skylight feel) ===
    let h3_h = 2u8;
    for x in 45..52 {
        set(&mut grid, x, 8, make_block(1, h3_h, 0));
        set(&mut grid, x, 14, make_block(1, h3_h, 0));
    }
    for y in 8..15 {
        set(&mut grid, 45, y, make_block(1, h3_h, 0));
        set(&mut grid, 51, y, make_block(1, h3_h, 0));
    }
    // Glass windows on sides
    set(&mut grid, 48, 8, make_block(5, h3_h, 0));
    set(&mut grid, 48, 14, make_block(5, h3_h, 0));
    set(&mut grid, 45, 11, make_block(5, h3_h, 0));
    set(&mut grid, 51, 11, make_block(5, h3_h, 0));
    // Door
    set(&mut grid, 49, 14, make_block(4, 1, 1));
    // Roof
    for y in 9..14 {
        for x in 46..51 {
            set(&mut grid, x, y, make_block(2, 0, roof_flag));
        }
    }

    // Water pool (unchanged)
    for y in 40..48 {
        for x in 12..22 {
            set(&mut grid, x, y, make_block(3, 0, 0));
        }
    }

    // Some standalone glass walls (like a greenhouse fragment)
    for x in 5..9 {
        set(&mut grid, x, 55, make_block(5, 2, 0));
        set(&mut grid, x, 60, make_block(5, 2, 0));
    }
    for y in 55..61 {
        set(&mut grid, 5, y, make_block(5, 2, 0));
        set(&mut grid, 8, y, make_block(5, 2, 0));
    }
    // Greenhouse interior: roofed
    for y in 56..60 {
        for x in 6..8 {
            set(&mut grid, x, y, make_block(2, 0, roof_flag));
        }
    }

    // Scatter trees across the map using a simple hash
    // Avoid placing on existing structures
    for y in 0..GRID_H {
        for x in 0..GRID_W {
            let idx = (y * w + x) as usize;
            let existing = grid[idx];
            // Only place on bare dirt floor (type 2, height 0, no flags)
            if existing != make_block(2, 0, 0) {
                continue;
            }
            // Simple hash-based pseudo-random placement
            let h = ((x.wrapping_mul(374761393)) ^ (y.wrapping_mul(668265263)))
                .wrapping_add(1013904223);
            let r = (h >> 16) & 0xFFF; // 0..4095
            // ~3% chance of a tree
            if r < 120 {
                // Tree height varies 2-4
                let tree_h = 2 + ((h >> 8) & 0x3) as u8; // 2, 3, or 4
                grid[idx] = make_block(8, tree_h, 0);
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
                        let canopy_r = 0.43;
                        let trunk_r = 0.07;
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
                        let canopy_r = 0.38 - (cy + 0.1).abs() * 0.3; // tapers
                        let canopy_r = canopy_r.max(0.05);
                        if dist < trunk_r {
                            (75, 48, 22, 240u8)
                        } else if diamond < canopy_r + 0.1 && dist < 0.45 {
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
                        let canopy_r = 0.35;
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
                        let canopy_rx = 0.22;
                        let canopy_ry = 0.38;
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
    _pad1: f32,
}

// --- Application state ---
struct App {
    window: Option<Arc<Window>>,
    gfx: Option<GfxState>,
    egui_state: Option<EguiState>,
    camera: CameraUniform,
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
}

const LIGHTMAP_PROP_ITERATIONS: u32 = 16;
const LIGHTMAP_UPDATE_INTERVAL: u32 = 6; // recompute lightmap every N frames (~10fps at 60fps)
const RENDER_SCALE: f32 = 0.5; // render at half resolution, upscale via blit

struct GfxState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    #[allow(dead_code)]
    surface_format: wgpu::TextureFormat,
    // Lightmap: seed + iterative propagation (ping-pong, 64x64)
    lightmap_seed_pipeline: wgpu::ComputePipeline,
    lightmap_seed_bind_group: wgpu::BindGroup,
    lightmap_prop_pipeline: wgpu::ComputePipeline,
    lightmap_prop_bind_groups: [wgpu::BindGroup; 2], // [0]: read A write B, [1]: read B write A
    lightmap_textures: [wgpu::Texture; 2],
    // Raytrace pass (per-pixel, screen resolution)
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    render_bind_group: wgpu::BindGroup,
    output_texture: wgpu::Texture,
    camera_buffer: wgpu::Buffer,
    grid_buffer: wgpu::Buffer,
    sprite_buffer: wgpu::Buffer,
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
                center_x: 30.0, // centered on the houses area
                center_y: 30.0,
                zoom: 1.0, // will be set in init_gfx_async to fit map
                show_roofs: 0.0,
                screen_w: 800.0,
                screen_h: 600.0,
                grid_w: GRID_W as f32,
                grid_h: GRID_H as f32,
                time: 0.0,
                glass_light_mul: 0.12,
                indoor_glow_mul: 0.25,
                light_bleed_mul: 0.6,
                foliage_opacity: 0.55,
                foliage_variation: 0.3,
                oblique_strength: 0.12,
                _pad1: 0.0,
            },
            grid_data: Vec::new(),
            grid_dirty: false,
            mouse_pressed: false,
            mouse_dragged: false,
            last_mouse_x: 0.0,
            last_mouse_y: 0.0,
            dragging_light: None,
            start_time: Instant::now(),
            time_of_day: 0.0,
            time_paused: false,
            time_speed: 1.0,
            last_frame_time: Instant::now(),
            frame_count: 0,
            fps_accum: 0.0,
            fps_display: 0.0,
            lightmap_frame: 0,
        }
    }

    /// Convert screen pixel coordinates to world block coordinates
    fn screen_to_world(&self, sx: f64, sy: f64) -> (f32, f32) {
        // Scale mouse coords from window space to render space
        let rx = sx as f32 * RENDER_SCALE;
        let ry = sy as f32 * RENDER_SCALE;
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

    /// Try to toggle a door at the given world coordinates
    fn try_toggle_door(&mut self, wx: f32, wy: f32) {
        let bx = wx.floor() as i32;
        let by = wy.floor() as i32;
        if bx < 0 || by < 0 || bx >= GRID_W as i32 || by >= GRID_H as i32 {
            return;
        }
        let idx = (by as u32 * GRID_W + bx as u32) as usize;
        let block = self.grid_data[idx];
        if is_door_rs(block) {
            // Toggle bit2 (is_open)
            let flags = block_flags_rs(block);
            let new_flags = flags ^ 4; // toggle bit2
            let new_block = (block & 0xFF00FFFF) | ((new_flags as u32) << 16);
            self.grid_data[idx] = new_block;
            self.grid_dirty = true;
            let open = (new_flags & 4) != 0;
            log::info!("Door at ({}, {}): {}", bx, by, if open { "opened" } else { "closed" });
        }
    }

    async fn init_gfx_async(&mut self, window: Arc<Window>) {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let render_w = ((width as f32) * RENDER_SCALE).max(1.0) as u32;
        let render_h = ((height as f32) * RENDER_SCALE).max(1.0) as u32;
        self.camera.screen_w = render_w as f32;
        self.camera.screen_h = render_h as f32;
        // Zoom to show ~64 blocks (the houses area), not the full map
        let view_size = 64.0f32;
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
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

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

        // Output texture at render resolution (compute writes RGBA8, blit upscales to window)
        let output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("output-texture"),
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
        let output_view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());

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

        // --- Lightmap textures (two for ping-pong, 64x64) ---
        let lightmap_desc = wgpu::TextureDescriptor {
            label: Some("lightmap-texture-a"),
            size: wgpu::Extent3d {
                width: GRID_W,
                height: GRID_H,
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

        let lightmap_seed_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lightmap-seed-bg"),
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
                ],
            });

        // Raytrace shader samples the final lightmap result (texture A after even iterations)
        let lightmap_sample_view = lightmap_a.create_view(&wgpu::TextureViewDescriptor::default());

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute-bg"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&output_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: grid_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&lightmap_sample_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&lightmap_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: sprite_buffer.as_entire_binding(),
                },
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

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blit-bg"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&output_view),
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
            lightmap_seed_bind_group,
            lightmap_prop_pipeline,
            lightmap_prop_bind_groups: [prop_bg_0, prop_bg_1],
            lightmap_textures: [lightmap_a, lightmap_b],
            compute_pipeline,
            compute_bind_group,
            render_pipeline,
            render_bind_group,
            output_texture,
            camera_buffer,
            grid_buffer,
            sprite_buffer,
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

        let render_w = ((width as f32) * RENDER_SCALE).max(1.0) as u32;
        let render_h = ((height as f32) * RENDER_SCALE).max(1.0) as u32;
        self.camera.screen_w = render_w as f32;
        self.camera.screen_h = render_h as f32;

        // Recreate output texture at render resolution
        gfx.output_texture = gfx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("output-texture"),
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

        let output_view = gfx
            .output_texture
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
        gfx.compute_bind_group = gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute-bg"),
            layout: &compute_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&output_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gfx.camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: gfx.grid_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&lightmap_sample_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&lightmap_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: gfx.sprite_buffer.as_entire_binding(),
                },
            ],
        });

        let render_bgl = gfx.render_pipeline.get_bind_group_layout(0);
        let sampler = gfx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("blit-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        gfx.render_bind_group = gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blit-bg"),
            layout: &render_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&output_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
    }

    fn render(&mut self) {
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

        self.camera.time = self.time_of_day;

        let gfx = self.gfx.as_ref().unwrap();

        // Re-upload grid if dirty (door toggled etc.)
        if self.grid_dirty {
            gfx.queue.write_buffer(
                &gfx.grid_buffer,
                0,
                bytemuck::cast_slice(&self.grid_data),
            );
            self.grid_dirty = false;
        }

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
                ui.label(egui::RichText::new(format!("v22 | {:.0} fps", self.fps_display)).color(egui::Color32::from_rgba_premultiplied(200, 200, 200, 180)).size(14.0));
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
                let base_zoom = (self.camera.screen_w / GRID_W as f32).min(self.camera.screen_h / GRID_H as f32);
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

                ui.separator();

                let zoom_pct = zoom / base_zoom * 100.0;
                ui.label(format!("Zoom: {:.0}%", zoom_pct));
                ui.add(egui::Slider::new(&mut zoom, base_zoom * 0.5..=base_zoom * 4.0)
                    .text("Zoom")
                    .show_value(false)
                    .logarithmic(true));
                if ui.button("Reset zoom").clicked() {
                    zoom = base_zoom;
                }

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

        // Lightmap: only recompute every N frames (fire flicker at ~10fps is fine)
        // Always recompute when grid is dirty (door toggle, light moved)
        self.lightmap_frame += 1;
        let need_lightmap = self.grid_dirty || self.lightmap_frame >= LIGHTMAP_UPDATE_INTERVAL;
        if need_lightmap {
            self.lightmap_frame = 0;
            let lm_wg_x = (GRID_W + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
            let lm_wg_y = (GRID_H + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;

            // Seed pass
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("lightmap-seed"),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&gfx.lightmap_seed_pipeline);
                cpass.set_bind_group(0, &gfx.lightmap_seed_bind_group, &[]);
                cpass.dispatch_workgroups(lm_wg_x, lm_wg_y, 1);
            }

            // Propagation passes
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

        // Compute pass 2: raytrace (per-pixel, render resolution)
        let rt_w = self.camera.screen_w as u32;
        let rt_h = self.camera.screen_h as u32;
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("raytrace-pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&gfx.compute_pipeline);
            cpass.set_bind_group(0, &gfx.compute_bind_group, &[]);
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
            // Blit the raytraced scene
            rpass.set_pipeline(&gfx.render_pipeline);
            rpass.set_bind_group(0, &gfx.render_bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }

        // Render pass: egui overlay (separate encoder to avoid lifetime issues)
        {
            // Submit the main encoder FIRST (compute + blit) so the surface has content
            gfx.queue.submit(std::iter::once(encoder.finish()));

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
            .with_inner_size(PhysicalSize::new(1280u32, 1280u32));

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
                        PhysicalKey::Code(KeyCode::Escape) => event_loop.exit(),
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
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y as f64,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y / 50.0,
                };
        let base_zoom = (self.camera.screen_w / GRID_W as f32).min(self.camera.screen_h / GRID_H as f32);
                if scroll > 0.0 {
                    self.camera.zoom *= 1.1;
                } else if scroll < 0.0 {
                    self.camera.zoom /= 1.1;
                }
                self.camera.zoom = self.camera.zoom.clamp(base_zoom * 0.5, base_zoom * 4.0);
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
                            self.try_toggle_door(wx, wy);
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
            WindowEvent::CursorMoved { position, .. } => {
                if self.mouse_pressed {
                    let dx = position.x - self.last_mouse_x;
                    let dy = position.y - self.last_mouse_y;
                    // Only count as drag if moved more than 3 pixels
                    if dx.abs() > 3.0 || dy.abs() > 3.0 {
                        self.mouse_dragged = true;
                    }
                    if self.mouse_dragged {
                        self.camera.center_x -= dx as f32 * RENDER_SCALE / self.camera.zoom;
                        self.camera.center_y -= dy as f32 * RENDER_SCALE / self.camera.zoom;
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
