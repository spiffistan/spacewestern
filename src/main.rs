use bytemuck::Zeroable;
use std::sync::Arc;

mod materials;
mod grid;
mod sprites;
mod block_defs;

use materials::{GpuMaterial, build_material_table};
use block_defs::BlockRegistry;
use grid::{GRID_W, GRID_H, make_block, block_type_rs, block_flags_rs, is_door_rs, compute_roof_heights, generate_test_grid};
use sprites::generate_tree_sprites;

mod pleb;
use pleb::{Pleb, GpuPleb, is_walkable_pos, astar_path, adjacent_walkable, random_name, MAX_PLEBS, PlebActivity};

mod needs;
use needs::{sample_environment, tick_needs, mood_label, AirReadback, BreathingState, breathing_label, find_breathable_tile, find_cool_tile, find_nearest_crate, BERRY_HUNGER_RESTORE, HEAT_CRISIS_TEMP};

mod build;
mod camera;
mod fluid;

use build::{BuildTool, FluidOverlay};
use camera::CameraUniform;
use fluid::{FluidParams, FLUID_SIM_W, FLUID_SIM_H, FLUID_DYE_W, FLUID_DYE_H, FLUID_PRESSURE_ITERS, build_obstacle_field, smoothstep_f32, half_to_f32, f32_to_f16};

mod pipes;
mod ui;
mod gpu_init;
mod simulation;
use pipes::PipeNetwork;

mod physics;
use physics::{PhysicsBody, tick_bodies, pleb_body_collision, nearest_body};

mod weather;
use weather::{WeatherState, tick_weather, tick_wetness};

mod resources;
use resources::{CrateInventory, CRATE_MAX_ITEMS};

#[path = "time.rs"]
mod game_time;
use game_time::Instant;

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum SandboxTool {
    None,
    Lightning,
}

const WORKGROUP_SIZE: u32 = 8;
const DAY_DURATION: f32 = 60.0; // must match shader


/// GPU debug readback state (shift-hover info tool).
#[derive(Clone, Debug)]
struct DebugReadback {
    mode: bool,
    fluid_density: [f32; 4],
    block_temp: f32,
    block_temp_pending: bool,
    voltage: f32,
    voltage_pending: bool,
    fluid_pending: bool,
}

impl Default for DebugReadback {
    fn default() -> Self {
        Self { mode: false, fluid_density: [0.0; 4], block_temp: 15.0, block_temp_pending: false, voltage: 0.0, voltage_pending: false, fluid_pending: false }
    }
}

/// Which popup/slider is open for a selected block.
#[derive(Clone, Debug, Default)]
struct BlockSelection {
    pump: Option<u32>,
    pump_world: (f32, f32),
    fan: Option<u32>,
    fan_world: (f32, f32),
    dimmer: Option<u32>,
    dimmer_world: (f32, f32),
    cannon: Option<u32>,
    crate_idx: Option<u32>,
    crate_world: (f32, f32),
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
    pipe_network: PipeNetwork,
    fluid_speed: f32,             // fluid simulation speed multiplier
    enable_prox_glow: bool,       // per-pixel proximity glow (expensive)
    enable_dir_bleed: bool,       // directional light bleed (expensive)
    enable_temporal: bool,        // temporal reprojection (reuse previous frame)
    enable_ricochets: bool,       // bullets bounce off walls
    sandbox_mode: bool,           // enables sandbox build category + debug tools
    sandbox_tool: SandboxTool,    // current sandbox action
    show_pipe_overlay: bool,       // draw pipe gas contents as egui overlay
    pipe_width: f32,               // pipe conductance multiplier (1=narrow, 10=wide)
    drag_start: Option<(i32, i32)>, // grid coords where drag started (for shape building)
    block_sel: BlockSelection,      // which popup/slider is open
    build_category: Option<&'static str>, // selected build category, None = collapsed
    debug: DebugReadback,          // shift-hover readback state
    fluid_mouse_active: bool,  // middle mouse button held
    fluid_mouse_prev: Option<(f32, f32)>, // previous world position for velocity calc
    // Pleb (character)
    plebs: Vec<Pleb>,
    selected_pleb: Option<usize>,  // index into plebs vec
    placing_pleb: bool,
    next_pleb_id: usize,
    cannon_angles: std::collections::HashMap<u32, f32>, // grid_idx → angle (radians)
    show_pleb_help: bool,      // show controls modal
    show_inventory: bool,      // show pleb inventory window
    pressed_keys: std::collections::HashSet<KeyCode>,
    auto_doors: Vec<(i32, i32, f32)>,  // (x, y, time_opened) for auto-closing
    physics_bodies: Vec<PhysicsBody>,
    // Per-pleb air readback from fluid sim (updated one frame behind)
    pleb_air_data: Vec<AirReadback>,
    pleb_air_readback_pending: bool,
    // Context menu for pleb actions
    context_menu: Option<(f32, f32)>, // screen position for context menu popup
    // Storage crate inventories: grid_idx → stored items
    crate_contents: std::collections::HashMap<u32, CrateInventory>,
    // Rock context menu
    rock_context_menu: Option<(f32, f32, i32, i32)>,
    // Combat
    grenade_charging: bool,
    grenade_charge: f32,
    grenade_impacts: Vec<(f32, f32)>,
    burst_mode: bool,
    burst_queue: u8,       // remaining burst shots to fire (0 = none)
    burst_delay: f32,      // seconds until next burst shot
    // Weather system
    weather: WeatherState,
    weather_timer: f32,
    // Lightning
    lightning_timer: f32,           // seconds until next potential strike
    lightning_flash: f32,           // flash brightness (decays rapidly, 0-1)
    lightning_strike: Option<(f32, f32)>, // (x, y) of current strike for rendering
    lightning_surge_done: bool,         // prevents re-injecting voltage surge
    // Wind variation: slowly drifting target angle + magnitude
    wind_target_angle: f32,    // target angle in radians
    wind_target_mag: f32,      // target magnitude
    wind_change_timer: f32,    // seconds until next target shift
    wetness_data: Vec<f32>,  // 256x256 per-tile wetness (0.0-1.0)
    // Diagonal wall drag preview: (x, y, variant) per tile
    diag_preview: Vec<(i32, i32, u8)>,
}

const LIGHTMAP_SCALE: u32 = 2; // lightmap texels per grid cell (2x resolution)
const LIGHTMAP_W: u32 = GRID_W * LIGHTMAP_SCALE;
const LIGHTMAP_H: u32 = GRID_H * LIGHTMAP_SCALE;
const LIGHTMAP_PROP_ITERATIONS: u32 = 26; // more iterations for 2x res (covers ~13 tile radius)
const LIGHTMAP_UPDATE_INTERVAL: u32 = 2; // recompute every N frames (~30fps lightmap at 60fps)
#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_RENDER_SCALE: f32 = 0.5;
#[cfg(target_arch = "wasm32")]
const DEFAULT_RENDER_SCALE: f32 = 0.35;

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
    material_buffer: wgpu::Buffer,
    pleb_buffer: wgpu::Buffer,
    block_temp_buffer: wgpu::Buffer,
    thermal_pipeline: wgpu::ComputePipeline,
    thermal_bind_group: wgpu::BindGroup,
    // Power grid
    voltage_buffer: wgpu::Buffer,
    power_pipeline: wgpu::ComputePipeline,
    power_bind_group: wgpu::BindGroup,
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
    block_temp_readback_buffer: wgpu::Buffer,       // staging buffer for block temp readback
    // Pleb air readback — one texel per pleb, each at 256-byte aligned offset
    pleb_air_readback_buffer: wgpu::Buffer,
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
                force_refresh: 1.0,
                pleb_x: 0.0, pleb_y: 0.0, pleb_angle: 0.0, pleb_selected: 0.0, pleb_torch: 0.0, pleb_headlight: 0.0,
                prev_center_x: 0.0, prev_center_y: 0.0, prev_zoom: 0.0, prev_time: 0.0,
                rain_intensity: 0.0, cloud_cover: 0.0, wind_magnitude: 0.0, wind_angle: 0.0,
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
                wind_x: std::f32::consts::FRAC_PI_4.cos() * 10.0,
                wind_y: std::f32::consts::FRAC_PI_4.sin() * 10.0,
                smoke_rate: 0.3,
                fan_speed: 40.0,
                rain_intensity: 0.0,
            },
            fluid_overlay: FluidOverlay::None,
            pipe_network: PipeNetwork::new(),
            fluid_speed: 1.0,
            enable_prox_glow: true,
            enable_dir_bleed: true,
            enable_temporal: true,
            enable_ricochets: true,
            sandbox_mode: false,
            sandbox_tool: SandboxTool::None,
            drag_start: None,
            show_pipe_overlay: false,
            pipe_width: 5.0,
            block_sel: BlockSelection::default(),
            build_category: None,
            debug: DebugReadback::default(),
            fluid_dye_phase: 0,
            output_phase: 0,
            prev_cam_x: 0.0,
            prev_cam_y: 0.0,
            prev_cam_zoom: 0.0,
            prev_cam_time: 0.0,
            fluid_mouse_active: false,
            fluid_mouse_prev: None,
            plebs: {
                let jeff = Pleb::new(0, "Jeff".to_string(), 102.5, 100.5, 42);
                let mut enemies = vec![
                    ("Jaff", 115.0, 95.0, 101u32),
                    ("Juff", 125.0, 110.0, 202),
                    ("Jif",  95.0, 120.0, 303),
                    ("Bob",  135.0, 100.0, 404),
                ];
                let mut all = vec![jeff];
                for (i, (name, x, y, seed)) in enemies.iter().enumerate() {
                    let mut e = Pleb::new(i + 1, name.to_string(), *x, *y, *seed);
                    e.is_enemy = true;
                    e.wander_timer = 3.0 + (i as f32) * 2.0;
                    all.push(e);
                }
                all
            },
            selected_pleb: None,
            next_pleb_id: 1,
            placing_pleb: false,
            cannon_angles: std::collections::HashMap::new(),
            show_pleb_help: false,
            show_inventory: false,
            pressed_keys: std::collections::HashSet::new(),
            auto_doors: Vec::new(),
            physics_bodies: Vec::new(),
            pleb_air_data: Vec::new(),
            pleb_air_readback_pending: false,
            context_menu: None,
            crate_contents: std::collections::HashMap::new(),
            rock_context_menu: None,
            grenade_charging: false,
            grenade_charge: 0.0,
            grenade_impacts: Vec::new(),
            burst_mode: false,
            burst_queue: 0,
            burst_delay: 0.0,
            weather: WeatherState::Clear,
            weather_timer: 45.0,
            lightning_timer: 10.0,
            lightning_flash: 0.0,
            lightning_strike: None,
            lightning_surge_done: false,
            wind_target_angle: std::f32::consts::FRAC_PI_4, // ~NE
            wind_target_mag: 10.0,
            wind_change_timer: 15.0,
            wetness_data: vec![0.0; (GRID_W * GRID_H) as usize],
            diag_preview: Vec::new(),
        }
    }

    /// Convert world block coordinates to window screen pixels
    #[allow(dead_code)]
    fn world_to_screen(&self, wx: f32, wy: f32) -> (f32, f32) {
        let rx = (wx - self.camera.center_x) * self.camera.zoom + self.camera.screen_w * 0.5;
        let ry = (wy - self.camera.center_y) * self.camera.zoom + self.camera.screen_h * 0.5;
        (rx / self.render_scale, ry / self.render_scale)
    }

    /// Sync crate block height with item count for shader rendering.
    fn sync_crate_visual(&mut self, cidx: u32) {
        if let Some(inv) = self.crate_contents.get(&cidx) {
            let count = inv.total().min(CRATE_MAX_ITEMS) as u8;
            let idx = cidx as usize;
            if idx < self.grid_data.len() {
                let block = self.grid_data[idx];
                if (block & 0xFF) == 33 {
                    // Store item count in height byte (bits 8-15)
                    self.grid_data[idx] = (block & 0xFFFF00FF) | ((count as u32) << 8);
                    self.grid_dirty = true;
                }
            }
        }
    }

    /// Get the tiles a bench would occupy at (bx, by) with given rotation
    fn bed_tiles(&self, bx: i32, by: i32, rotation: u32) -> [(i32, i32); 2] {
        if rotation == 0 {
            [(bx, by), (bx + 1, by)]
        } else {
            [(bx, by), (bx, by + 1)]
        }
    }

    /// Get the 9 tiles a solar panel occupies at (bx, by) — 3×3 grid
    fn solar_tiles(&self, bx: i32, by: i32) -> [(i32, i32); 9] {
        [
            (bx, by),     (bx + 1, by),     (bx + 2, by),
            (bx, by + 1), (bx + 1, by + 1), (bx + 2, by + 1),
            (bx, by + 2), (bx + 1, by + 2), (bx + 2, by + 2),
        ]
    }

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

    /// Compute tiles for a hollow rectangle (walls) between two corners.
    fn hollow_rect_tiles(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
        let min_x = x0.min(x1);
        let max_x = x0.max(x1);
        let min_y = y0.min(y1);
        let max_y = y0.max(y1);
        let mut tiles = Vec::new();
        for x in min_x..=max_x {
            tiles.push((x, min_y));
            if max_y != min_y { tiles.push((x, max_y)); }
        }
        for y in (min_y + 1)..max_y {
            tiles.push((min_x, y));
            if max_x != min_x { tiles.push((max_x, y)); }
        }
        tiles
    }

    fn diagonal_wall_tiles(x0: i32, y0: i32, x1: i32, y1: i32, rotation: u32) -> Vec<(i32, i32, u8)> {
        compute_diagonal_wall_tiles(x0, y0, x1, y1, rotation)
    }
}

/// Compute tiles for a continuous diagonal wall.
/// Returns (x, y, variant) for each tile — main diagonal tiles + fill tiles
/// that tessellate into a solid wall strip.
/// `rotation` determines which side of the wall is solid (0-3).
fn compute_diagonal_wall_tiles(x0: i32, y0: i32, x1: i32, y1: i32, rotation: u32) -> Vec<(i32, i32, u8)> {
        let dx = x1 - x0;
        let dy = y1 - y0;
        if dx == 0 && dy == 0 { return vec![(x0, y0, rotation as u8)]; }
        // Force true 45° diagonal: use the shorter axis as step count
        let steps = dx.abs().min(dy.abs()).max(1);
        let sx = dx.signum();
        let sy = dy.signum();
        // If either axis is zero, pick a default diagonal direction
        let sx = if sx == 0 { 1 } else { sx };
        let sy = if sy == 0 { 1 } else { sy };

        // Determine drag direction: \ (same sign) or / (opposite sign)
        let is_backslash = (sx > 0) == (sy > 0);

        // Auto-adapt rotation to match drag direction.
        // "below" side = variants 0,1; "above" side = variants 2,3
        let is_below = rotation < 2;
        let main_var: u8 = if is_backslash {
            if is_below { 1 } else { 3 } // \ variants
        } else {
            if is_below { 0 } else { 2 } // / variants
        };
        let fill_var: u8 = main_var ^ 2; // flip side: 0↔2, 1↔3

        let mut tiles = Vec::new();
        let mut x = x0;
        let mut y = y0;

        for i in 0..=steps {
            tiles.push((x, y, main_var));

            // Fill tile between this step and the previous (closes the gap).
            // The fill must go toward the solid side of the main variant.
            // Which of the two candidate positions (x-sx,y) or (x,y-sy) is correct
            // depends on both the variant and the drag direction.
            if i > 0 {
                let use_horizontal = match main_var {
                    0 | 3 => sx < 0, // right-facing: fill right when going left
                    _     => sx > 0, // left-facing: fill left when going right
                };
                let (fx, fy) = if use_horizontal {
                    (x - sx, y)
                } else {
                    (x, y - sy)
                };
                tiles.push((fx, fy, fill_var));
            }

            x += sx;
            y += sy;
        }
        tiles
}

impl App {
    /// Compute tiles for a line (pipes) snapped to dominant axis.
    fn line_tiles(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let mut tiles = Vec::new();
        if dx >= dy {
            // Horizontal line (ordered in drag direction)
            let step = if x1 >= x0 { 1 } else { -1 };
            let mut x = x0;
            loop {
                tiles.push((x, y0));
                if x == x1 { break; }
                x += step;
            }
        } else {
            // Vertical line (ordered in drag direction)
            let step = if y1 >= y0 { 1 } else { -1 };
            let mut y = y0;
            loop {
                tiles.push((x0, y));
                if y == y1 { break; }
                y += step;
            }
        }
        tiles
    }

    /// Compute tiles for a filled rectangle (destroy).
    fn filled_rect_tiles(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
        let min_x = x0.min(x1);
        let max_x = x0.max(x1);
        let min_y = y0.min(y1);
        let max_y = y0.max(y1);
        let mut tiles = Vec::new();
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                tiles.push((x, y));
            }
        }
        tiles
    }

    /// Check if a tile can support a roof (wall within 6 Manhattan distance).
    fn can_support_roof(grid: &[u32], x: i32, y: i32) -> bool {
        let max_dist = 6i32;
        for dy in -max_dist..=max_dist {
            for dx in -max_dist..=max_dist {
                if dx.abs() + dy.abs() > max_dist { continue; }
                let nx = x + dx;
                let ny = y + dy;
                if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 { continue; }
                let b = grid[(ny as u32 * GRID_W + nx as u32) as usize];
                let bt = b & 0xFF;
                let bh = (b >> 8) & 0xFF;
                if bh > 0 && matches!(bt, 1 | 4 | 5 | 14 | 21 | 22 | 23 | 24 | 25) {
                    return true;
                }
            }
        }
        false
    }

    /// Apply the drag shape when mouse is released.
    fn apply_drag_shape(&mut self, sx: i32, sy: i32, ex: i32, ey: i32) {
        // Roof tool: special handling — sets flag, doesn't change block type
        if self.build_tool == BuildTool::Roof {
            let tiles = Self::filled_rect_tiles(sx, sy, ex, ey);
            for (tx, ty) in tiles {
                if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 { continue; }
                if Self::can_support_roof(&self.grid_data, tx, ty) {
                    let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                    let block = self.grid_data[idx];
                    let bh = (block >> 8) & 0xFF;
                    if bh == 0 { // only floor-level tiles
                        self.grid_data[idx] |= 2 << 16; // set roof flag (bit 1)
                        self.grid_dirty = true;
                    }
                }
            }
            compute_roof_heights(&mut self.grid_data);
            return;
        }

        if self.build_tool == BuildTool::RemoveFloor {
            let tiles = Self::filled_rect_tiles(sx, sy, ex, ey);
            for (tx, ty) in tiles {
                if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 { continue; }
                let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                let block = self.grid_data[idx];
                let bt = block & 0xFF;
                // Replace floor types (26/27/28) with dirt (2)
                if matches!(bt, 26 | 27 | 28) {
                    let roof_flag = (block >> 16) & 2;
                    let roof_h = block & 0xFF000000;
                    self.grid_data[idx] = make_block(2, 0, roof_flag as u8) | roof_h;
                    self.grid_dirty = true;
                }
            }
            return;
        }

        if self.build_tool == BuildTool::RemoveRoof {
            let tiles = Self::filled_rect_tiles(sx, sy, ex, ey);
            for (tx, ty) in tiles {
                if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 { continue; }
                let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                let block = self.grid_data[idx];
                let has_roof = (block >> 16) & 2 != 0;
                if has_roof {
                    self.grid_data[idx] &= !(2u32 << 16); // clear roof flag
                    self.grid_dirty = true;
                }
            }
            compute_roof_heights(&mut self.grid_data);
            return;
        }

        // Special case: diagonal wall drag places per-tile variants
        if self.build_tool == BuildTool::Place(44) {
            let diag_tiles = Self::diagonal_wall_tiles(sx, sy, ex, ey, self.build_rotation);
            for (tx, ty, variant) in diag_tiles {
                if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 { continue; }
                let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                let block = self.grid_data[idx];
                let bt = block_type_rs(block);
                let bh = (block >> 8) & 0xFF;
                if (bt == 0 || bt == 2) && bh == 0 {
                    let roof_flag = block_flags_rs(block) & 2;
                    let roof_h = block & 0xFF000000;
                    let flags = roof_flag | (variant << 3);
                    self.grid_data[idx] = make_block(44, 3, flags) | roof_h;
                    self.grid_dirty = true;
                }
            }
            if self.grid_dirty { compute_roof_heights(&mut self.grid_data); }
            return;
        }

        let reg = block_defs::BlockRegistry::cached();
        let (block_type_id, tiles) = match self.build_tool {
            BuildTool::Destroy => (0u8, Self::filled_rect_tiles(sx, sy, ex, ey)),
            BuildTool::Place(id) => {
                let shape = reg.get(id).and_then(|d| d.placement.as_ref()).and_then(|p| p.drag.as_ref());
                let t = match shape {
                    Some(block_defs::DragShape::Line) => Self::line_tiles(sx, sy, ex, ey),
                    Some(block_defs::DragShape::FilledRect) => Self::filled_rect_tiles(sx, sy, ex, ey),
                    Some(block_defs::DragShape::HollowRect) => Self::hollow_rect_tiles(sx, sy, ex, ey),
                    _ => return,
                };
                (id, t)
            }
            _ => return,
        };

        // Connection mask bits: bit4=N, bit5=E, bit6=S, bit7=W (stored in height byte)
        const CONN_N: u8 = 0x10;
        const CONN_E: u8 = 0x20;
        const CONN_S: u8 = 0x40;
        const CONN_W: u8 = 0x80;
        let is_line_type = block_type_id == 15 || block_type_id == 36; // pipe or wire

        for (ti, &(tx, ty)) in tiles.iter().enumerate() {
            if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 { continue; }
            if self.build_tool == BuildTool::Destroy {
                self.destroy_block_at(tx, ty);
            } else {
                let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                let block = self.grid_data[idx];
                let bt = block_type_rs(block);
                let bh = (block >> 8) & 0xFF;
                let wire_anywhere = block_type_id == 36;
                let same_type = bt == block_type_id; // allow connecting to existing pipe/wire
                if ((bt == 0 || bt == 2) && bh == 0) || (wire_anywhere && bt != 36) || (is_line_type && same_type) {
                    // Compute connection mask from neighbors in the line
                    let mut conn: u8 = 0;
                    if is_line_type && tiles.len() > 1 {
                        // Connect to predecessor/successor in the drag line
                        if ti > 0 {
                            let (px, py) = tiles[ti - 1];
                            if px < tx { conn |= CONN_W; }
                            if px > tx { conn |= CONN_E; }
                            if py < ty { conn |= CONN_N; }
                            if py > ty { conn |= CONN_S; }
                        }
                        if ti + 1 < tiles.len() {
                            let (nx, ny) = tiles[ti + 1];
                            if nx > tx { conn |= CONN_E; }
                            if nx < tx { conn |= CONN_W; }
                            if ny > ty { conn |= CONN_S; }
                            if ny < ty { conn |= CONN_N; }
                        }
                        // To connect to existing wires, drag onto them (same_type merge handles it)
                    } else if is_line_type {
                        // Single tile: connect all directions (auto-detect)
                        conn = CONN_N | CONN_E | CONN_S | CONN_W;
                    }

                    if is_line_type && same_type {
                        // Existing pipe/wire: just merge new connections into existing mask
                        let existing_h = ((block >> 8) & 0xFF) as u8;
                        let merged = existing_h | conn;
                        self.grid_data[idx] = (block & 0xFFFF00FF) | ((merged as u32) << 8);
                    } else if wire_anywhere && bt != 0 && bt != 2 {
                        self.grid_data[idx] |= 0x80 << 16; // wire overlay flag
                    } else {
                        let roof_flag = block_flags_rs(block) & 2;
                        let roof_h = block & 0xFF000000;
                        let base_h = reg.get(block_type_id).and_then(|d| d.placement.as_ref()).map(|p| p.place_height).unwrap_or(3);
                        // Encode connection mask in upper nibble of height byte
                        let height = if is_line_type { base_h | conn } else { base_h };
                        self.grid_data[idx] = make_block(block_type_id, height, roof_flag) | roof_h;
                    }

                    self.grid_dirty = true;
                }
            }
        }
        // Recompute roof heights after placing walls (needed for shadows)
        if self.grid_dirty {
            compute_roof_heights(&mut self.grid_data);
        }
    }

    /// Destroy a placed block at grid position, reverting to bare dirt.
    fn destroy_block_at(&mut self, bx: i32, by: i32) {
        if bx < 0 || by < 0 || bx >= GRID_W as i32 || by >= GRID_H as i32 { return; }
        let idx = (by as u32 * GRID_W + bx as u32) as usize;
        let block = self.grid_data[idx];
        let bt = block_type_rs(block);

        // Air and bare dirt can't be destroyed
        if bt == 0 || (bt == 2 && block == make_block(2, 0, 0)) { return; }

        // Wall-mounted items (fan, inlet, outlet with height > 1): revert to stone wall
        // Revert to bare dirt — no roof, no height, no flags
        self.grid_data[idx] = make_block(2, 0, 0);
        self.grid_dirty = true;
        // Recompute roof heights since we may have removed a wall or roof tile
        compute_roof_heights(&mut self.grid_data);
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

        // Sandbox tools
        if self.sandbox_tool == SandboxTool::Lightning {
            // Trigger lightning at clicked location
            self.lightning_flash = 1.0;
            self.lightning_strike = Some((wx, wy));
            self.lightning_surge_done = false;
            // Heat/smoke at impact
            self.fluid_params.splat_x = wx;
            self.fluid_params.splat_y = wy;
            self.fluid_params.splat_vx = 0.0;
            self.fluid_params.splat_vy = 0.0;
            self.fluid_params.splat_radius = 1.5;
            self.fluid_params.splat_active = 1.0;
            log::info!("Sandbox: Lightning strike at ({:.0}, {:.0})", wx, wy);
            return;
        }

        // Destroy tool: single click destroys one block
        if self.build_tool == BuildTool::Destroy {
            // Check for physics bodies first
            self.physics_bodies.retain(|b| {
                let dist = ((wx - b.x).powi(2) + (wy - b.y).powi(2)).sqrt();
                dist > 0.5 // keep if far from click
            });
            self.destroy_block_at(bx, by);
            return;
        }

        // Click cannon: select for rotation, or fire if already selected
        if bt == 29 && self.build_tool == BuildTool::None {
            let cannon_idx = by as u32 * GRID_W + bx as u32;
            if self.block_sel.cannon == Some(cannon_idx) {
                // Already selected — fire!
                let angle = *self.cannon_angles.get(&cannon_idx).unwrap_or(&0.0);
                let dir_x = angle.cos();
                let dir_y = angle.sin();
                let spawn_x = bx as f32 + 0.5 + dir_x * 0.8;
                let spawn_y = by as f32 + 0.5 + dir_y * 0.8;
                self.physics_bodies.push(PhysicsBody::new_cannonball(spawn_x, spawn_y, dir_x, dir_y));
                // Muzzle smoke + recoil blast
                self.fluid_params.splat_x = bx as f32 + 0.5;
                self.fluid_params.splat_y = by as f32 + 0.5;
                self.fluid_params.splat_vx = -dir_x * 30.0;
                self.fluid_params.splat_vy = -dir_y * 30.0;
                self.fluid_params.splat_radius = 1.5;
                self.fluid_params.splat_active = 1.0;
                log::info!("Cannon fired at ({}, {})", bx, by);
            } else {
                // Select this cannon (deselect pleb)
                self.block_sel.cannon = Some(cannon_idx);
                self.selected_pleb = None;
                // Initialize angle from block direction bits if not yet set
                if !self.cannon_angles.contains_key(&cannon_idx) {
                    let dir_bits = (flags >> 3) & 3;
                    let angle = match dir_bits {
                        0 => -std::f32::consts::FRAC_PI_2, // north
                        1 => 0.0,                           // east
                        2 => std::f32::consts::FRAC_PI_2,  // south
                        _ => std::f32::consts::PI,          // west
                    };
                    self.cannon_angles.insert(cannon_idx, angle);
                }
                log::info!("Selected cannon at ({}, {})", bx, by);
            }
            return;
        } else if self.block_sel.cannon.is_some() && bt != 29 {
            // Clicked away from cannon — deselect
            self.block_sel.cannon = None;
        }

        // Placing pleb mode
        if self.placing_pleb {
            if is_walkable_pos(&self.grid_data, wx, wy) && self.plebs.len() < MAX_PLEBS {
                let id = self.next_pleb_id;
                self.next_pleb_id += 1;
                let name = random_name(id as u32);
                let mut p = Pleb::new(id, name, wx, wy, id as u32 * 7919 + 42);
                p.headlight_on = true;
                self.plebs.push(p);
                self.selected_pleb = Some(self.plebs.len() - 1);
                self.placing_pleb = false;
                self.show_pleb_help = self.plebs.len() == 1; // show help on first ever
            }
            return;
        }

        // Click on rock (no build tool): open rock context menu
        if bt == 34 && self.build_tool == BuildTool::None {
            self.rock_context_menu = Some((self.last_mouse_x as f32, self.last_mouse_y as f32, bx, by));
            return;
        }

        // Click on storage crate: toggle inspection popup
        if bt == 33 && self.build_tool != BuildTool::Destroy {
            let cidx = by as u32 * GRID_W + bx as u32;
            self.block_sel.crate_idx = if self.block_sel.crate_idx == Some(cidx) { None } else { Some(cidx) };
            self.block_sel.crate_world = (bx as f32 + 0.5, by as f32 + 0.5);
            return;
        }

        // Pleb interaction (before build tools)
        if self.build_tool == BuildTool::None {
            // Check if clicking on any pleb
            let mut clicked_pleb = None;
            for (i, p) in self.plebs.iter().enumerate() {
                if ((wx - p.x).powi(2) + (wy - p.y).powi(2)).sqrt() < 0.5 {
                    clicked_pleb = Some(i);
                    break;
                }
            }

            if let Some(idx) = clicked_pleb {
                self.selected_pleb = Some(idx);
                return;
            }

            // Click-to-move for selected pleb (blocked during crisis)
            if let Some(sel) = self.selected_pleb {
                if sel < self.plebs.len() && !self.plebs[sel].activity.is_crisis() {
                    let p = &self.plebs[sel];
                    let start_x = p.x.floor() as i32;
                    let start_y = p.y.floor() as i32;
                    let path = astar_path(&self.grid_data, (start_x, start_y), (bx, by));
                    if !path.is_empty() {
                        self.plebs[sel].path = path;
                        self.plebs[sel].path_idx = 1;
                    }
                    return;
                }
            }
        }

        // Build tool placement
        if self.build_tool != BuildTool::None {
            match self.build_tool {
                BuildTool::Place(37) => {
                    // Solar panel: 3×3 multi-tile placement
                    let tiles = self.solar_tiles(bx, by);
                    let all_valid = tiles.iter().all(|&(tx, ty)| self.can_place_at(tx, ty));
                    if all_valid {
                        for (i, &(tx, ty)) in tiles.iter().enumerate() {
                            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                            let tblock = self.grid_data[tidx];
                            let roof_flag = ((tblock >> 16) & 0xFF) as u8 & 2;
                            let roof_h = tblock & 0xFF000000;
                            // flags: bits 3-4 = column (0-2), bits 5-6 = row (0-2)
                            let col = (i % 3) as u8;
                            let row = (i / 3) as u8;
                            let seg_flags = roof_flag | (col << 3) | (row << 5);
                            self.grid_data[tidx] = make_block(37, 0, seg_flags) | roof_h;
                        }
                        self.grid_dirty = true;
                        compute_roof_heights(&mut self.grid_data);
                        log::info!("Placed solar panel at ({}, {})", bx, by);
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::Place(41) => {
                    // Wind turbine: 2×2 placement with rotation stored in flags
                    let tiles = [(bx, by), (bx+1, by), (bx, by+1), (bx+1, by+1)];
                    let all_valid = tiles.iter().all(|&(tx, ty)| self.can_place_at(tx, ty));
                    if all_valid {
                        for (i, &(tx, ty)) in tiles.iter().enumerate() {
                            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                            let tblock = self.grid_data[tidx];
                            let roof_flag = ((tblock >> 16) & 0xFF) as u8 & 2;
                            let roof_h = tblock & 0xFF000000;
                            let col = (i % 2) as u8;
                            let row = (i / 2) as u8;
                            // bits 3-4 = col, bits 5-6 = row, bit 7 = rotation (0=N-S, 1=E-W)
                            let rot_bit = if self.build_rotation % 2 == 1 { 0x40u8 } else { 0u8 };
                            let seg_flags = roof_flag | (col << 3) | (row << 5) | rot_bit;
                            self.grid_data[tidx] = make_block(41, 2, seg_flags) | roof_h;
                        }
                        self.grid_dirty = true;
                        compute_roof_heights(&mut self.grid_data);
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::Place(39) => {
                    // Medium battery: 2-tile placement (like bed)
                    let tiles = self.bed_tiles(bx, by, self.build_rotation);
                    let all_valid = tiles.iter().all(|&(tx, ty)| self.can_place_at(tx, ty));
                    if all_valid {
                        for (i, &(tx, ty)) in tiles.iter().enumerate() {
                            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                            let tblock = self.grid_data[tidx];
                            let roof_flag = ((tblock >> 16) & 0xFF) as u8 & 2;
                            let roof_h = tblock & 0xFF000000;
                            let seg_flags = roof_flag | ((i as u8) << 3) | ((self.build_rotation as u8) << 5);
                            self.grid_data[tidx] = make_block(39, 1, seg_flags) | roof_h;
                        }
                        self.grid_dirty = true;
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::Place(40) => {
                    // Large battery: 2×2 placement
                    let tiles = [(bx, by), (bx+1, by), (bx, by+1), (bx+1, by+1)];
                    let all_valid = tiles.iter().all(|&(tx, ty)| self.can_place_at(tx, ty));
                    if all_valid {
                        for (i, &(tx, ty)) in tiles.iter().enumerate() {
                            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                            let tblock = self.grid_data[tidx];
                            let roof_flag = ((tblock >> 16) & 0xFF) as u8 & 2;
                            let roof_h = tblock & 0xFF000000;
                            let col = (i % 2) as u8;
                            let row = (i / 2) as u8;
                            let seg_flags = roof_flag | (col << 3) | (row << 5);
                            self.grid_data[tidx] = make_block(40, 1, seg_flags) | roof_h;
                        }
                        self.grid_dirty = true;
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::Place(9) => {
                    // Bench: multi-tile placement
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
                BuildTool::Place(30) => {
                    // Bed: multi-tile placement
                    let tiles = self.bed_tiles(bx, by, self.build_rotation);
                    let all_valid = tiles.iter().all(|&(tx, ty)| self.can_place_at(tx, ty));
                    if all_valid {
                        for (i, &(tx, ty)) in tiles.iter().enumerate() {
                            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                            let tblock = self.grid_data[tidx];
                            let roof_flag = ((tblock >> 16) & 0xFF) as u8 & 2;
                            let roof_h = tblock & 0xFF000000;
                            // flags: bit3 = segment (0=head, 1=foot), bit5-6 = rotation
                            let seg_flags = roof_flag | ((i as u8) << 3) | ((self.build_rotation as u8) << 5);
                            self.grid_data[tidx] = make_block(30, 0, seg_flags) | roof_h;
                        }
                        self.grid_dirty = true;
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::Place(11) => {
                    // Table lamp: can only be placed on benches (type 9)
                    if bt == 9 {
                        let roof_h = block & 0xFF000000;
                        let roof_flag = flags & 2;
                        self.grid_data[idx] = make_block(11, 1, roof_flag) | roof_h;
                        self.grid_dirty = true;
                        log::info!("Placed table lamp at ({}, {})", bx, by);
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::Place(12) => {
                    // Fan: can be placed on a wall OR on the ground
                    let wall_types = [1i32, 4, 5, 14, 21, 22, 23, 24, 25];
                    let on_wall = wall_types.contains(&(bt as i32)) && (block >> 8) & 0xFF > 0;
                    let on_ground = self.can_place_at(bx, by);
                    if on_wall {
                        let wall_h = ((block >> 8) & 0xFF) as u8;
                        let roof_flag = flags & 2;
                        let roof_h = block & 0xFF000000;
                        let dir_flags = roof_flag | ((self.build_rotation as u8) << 3);
                        self.grid_data[idx] = make_block(12, wall_h, dir_flags) | roof_h;
                        self.grid_dirty = true;
                        log::info!("Placed fan at ({}, {}) dir={}", bx, by, self.build_rotation);
                        self.build_tool = BuildTool::None;
                    } else if on_ground {
                        let roof_flag = flags & 2;
                        let dir_flags = (self.build_rotation as u8) << 3;
                        let roof_h = block & 0xFF000000;
                        self.grid_data[idx] = make_block(12, 1, roof_flag | dir_flags) | roof_h;
                        self.grid_dirty = true;
                        log::info!("Placed fan on ground at ({}, {}) dir={}", bx, by, self.build_rotation);
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::Place(19) | BuildTool::Place(20) => {
                    // Outlet/Inlet: can place on ground OR on walls
                    let on_ground = self.can_place_at(bx, by);
                    let bt_at = block_type_rs(block);
                    let on_wall = matches!(bt_at, 1 | 4 | 5 | 14 | 21 | 22 | 23 | 24 | 25) && (block >> 8) & 0xFF > 0;
                    if on_ground || on_wall {
                        let height = if on_wall { ((block >> 8) & 0xFF) as u8 } else { 1 };
                        let roof_flag = flags & 2;
                        let rot_flags = (self.build_rotation as u8) << 3;
                        let bt_new = if self.build_tool == BuildTool::Place(19) { 19 } else { 20 };
                        let roof_h = block & 0xFF000000;
                        self.grid_data[idx] = make_block(bt_new, height, roof_flag | rot_flags) | roof_h;
                        self.grid_dirty = true;
                        log::info!("Placed {:?} at ({}, {})", self.build_tool, bx, by);
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::Place(id) => {
                    // Generic Place: use registry for place_height and flags
                    let reg = block_defs::BlockRegistry::cached();
                    let placement = reg.get(id).and_then(|d| d.placement.as_ref());
                    let place_height = placement.map(|p| p.place_height).unwrap_or(1);
                    let extra_flags = placement.map(|p| p.extra_flags).unwrap_or(0);
                    let stays_selected = placement.map(|p| p.stays_selected).unwrap_or(false);
                    let click_mode = placement.map(|p| p.click.clone()).unwrap_or(block_defs::ClickMode::Simple);

                    let can_place = self.can_place_at(bx, by)
                        || (id == 16 && bt == 15) // pump on pipe
                        || (id == 36 && bt != 36) // wire can go anywhere except on existing wire
                        || (id == 15 && bt == 15) // pipe on pipe = merge connections
                        || ((id == 42 || id == 43 || id == 45) && (bt == 36 || bt == 0 || bt == 2)); // switch/dimmer/breaker on wire or ground
                    if can_place && click_mode != block_defs::ClickMode::None {
                        if id == 36 && bt != 0 && bt != 2 {
                            // Wire on non-ground: add wire flag to existing block (bit 7)
                            self.grid_data[idx] |= 0x80 << 16; // set wire overlay flag
                        } else {
                            let roof_flag = flags & 2;
                            let rot_flags = (self.build_rotation as u8) << 3;
                            let mut combined_flags = roof_flag | rot_flags | extra_flags;
                            let mut final_height = place_height;
                            // Switch starts ON (flag bit 2)
                            if id == 42 { combined_flags |= 4; }
                            // Dimmer starts at 100% (height = 10)
                            if id == 43 { final_height = 10; }
                            // Circuit breaker starts ON (flag bit 2), threshold in height = 15V
                            if id == 45 { combined_flags |= 4; final_height = 15; }
                            // Single-click pipe/wire: auto-detect connections from adjacent matching blocks
                            if id == 15 || id == 36 {
                                let mut conn: u8 = 0;
                                for &(ndx, ndy, mask) in &[(0i32,-1i32,0x10u8),(0,1,0x40),(1,0,0x20),(-1,0,0x80)] {
                                    let nx = bx + ndx;
                                    let ny = by + ndy;
                                    if nx >= 0 && ny >= 0 && nx < GRID_W as i32 && ny < GRID_H as i32 {
                                        let nidx = (ny as u32 * GRID_W + nx as u32) as usize;
                                        let nbt = block_type_rs(self.grid_data[nidx]);
                                        if nbt == id {
                                            conn |= mask;
                                            // Also update neighbor's connection toward us
                                            let opp_mask: u8 = match mask { 0x10 => 0x40, 0x40 => 0x10, 0x20 => 0x80, _ => 0x20 };
                                            let nb_h = ((self.grid_data[nidx] >> 8) & 0xFF) as u8;
                                            self.grid_data[nidx] = (self.grid_data[nidx] & 0xFFFF00FF) | (((nb_h | opp_mask) as u32) << 8);
                                        }
                                    }
                                }
                                // If no neighbors found, connect all (standalone node)
                                if conn == 0 { conn = 0xF0; }
                                final_height |= conn;
                            }
                            let roof_h = block & 0xFF000000;
                            self.grid_data[idx] = make_block(id, final_height, combined_flags) | roof_h;
                        }
                        self.grid_dirty = true;
                        compute_roof_heights(&mut self.grid_data);
                        // Initialize cannon angle from build rotation
                        if id == 29 {
                            let angle = match self.build_rotation {
                                0 => -std::f32::consts::FRAC_PI_2, // north
                                1 => 0.0,                           // east
                                2 => std::f32::consts::FRAC_PI_2,  // south
                                _ => std::f32::consts::PI,          // west
                            };
                            self.cannon_angles.insert(idx as u32, angle);
                        }
                        log::info!("Placed {:?} at ({}, {})", self.build_tool, bx, by);
                        if !stays_selected {
                            self.build_tool = BuildTool::None;
                        }
                    }
                }
                BuildTool::Window => {
                    // Window (glass): replaces wall blocks
                    let wall_types = [1u32, 4, 14, 21, 22, 23, 24, 25];
                    if wall_types.contains(&(bt as u32)) && (block >> 8) & 0xFF > 0 {
                        let height = ((block >> 8) & 0xFF) as u8;
                        let roof_flag = flags & 2;
                        let roof_h = block & 0xFF000000;
                        self.grid_data[idx] = make_block(5, height, roof_flag) | roof_h;
                        self.grid_dirty = true;
                        log::info!("Placed window at ({}, {})", bx, by);
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::Door => {
                    // Door: replaces wall blocks with door
                    let wall_types = [1u32, 5, 14, 21, 22, 23, 24, 25];
                    if wall_types.contains(&(bt as u32)) && (block >> 8) & 0xFF > 0 {
                        let roof_h = block & 0xFF000000;
                        // Door: height 1, flag bit0=is_door, starts closed (bit2=0)
                        self.grid_data[idx] = make_block(4, 1, 1) | roof_h;
                        self.grid_dirty = true;
                        log::info!("Placed door at ({}, {})", bx, by);
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::RemoveFloor => {
                    let block = self.grid_data[idx];
                    let bt_here = block_type_rs(block);
                    if matches!(bt_here, 26 | 27 | 28) {
                        let roof_flag = block_flags_rs(block) & 2;
                        let roof_h = block & 0xFF000000;
                        self.grid_data[idx] = make_block(2, 0, roof_flag) | roof_h;
                        self.grid_dirty = true;
                    }
                }
                BuildTool::RemoveRoof => {
                    let block = self.grid_data[idx];
                    if (block >> 16) & 2 != 0 {
                        self.grid_data[idx] &= !(2u32 << 16);
                        self.grid_dirty = true;
                        compute_roof_heights(&mut self.grid_data);
                    }
                }
                BuildTool::WoodBox => {
                    self.physics_bodies.push(PhysicsBody::new_wood_box(wx, wy));
                    // Don't deselect — can place multiple
                    return;
                }
                BuildTool::Dig => {
                    // Dig: 20% per click, max depth 5 (= 1 full block).
                    // Water appears at depth >= 1 (20%).
                    if bx >= 0 && by >= 0 && bx < GRID_W as i32 && by < GRID_H as i32 {
                        let bt_dig = block_type_rs(block);
                        let roof_h = block & 0xFF000000;
                        if bt_dig == 2 || (bt_dig >= 26 && bt_dig <= 28) {
                            // Dirt or floor → dug ground depth 1 (20%)
                            self.grid_data[idx] = make_block(32, 1, 0) | roof_h;
                            self.grid_dirty = true;
                        } else if bt_dig == 32 {
                            let depth = (block >> 8) & 0xFF;
                            if depth < 5 {
                                self.grid_data[idx] = make_block(32, (depth + 1) as u8, 0) | roof_h;
                                self.grid_dirty = true;
                            }
                        }
                    }
                }
                BuildTool::None | BuildTool::Destroy
                | BuildTool::Roof => {}
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

            // When opening a door: inject outward velocity burst (pressure release)
            // Detect which side is inside (roofed) and push air outward from there
            if open {
                let fx = bx as f32 + 0.5;
                let fy = by as f32 + 0.5;
                // Check neighbors for roofed tiles (inside) vs outdoor
                let mut push_dir = (0.0f32, 0.0f32);
                for &(dx, dy) in &[(0i32, -1i32), (0, 1), (-1, 0), (1, 0)] {
                    let nx = bx + dx;
                    let ny = by + dy;
                    if nx >= 0 && ny >= 0 && nx < GRID_W as i32 && ny < GRID_H as i32 {
                        let nb = self.grid_data[(ny as u32 * GRID_W + nx as u32) as usize];
                        let has_roof = ((nb >> 16) & 2) != 0;
                        if has_roof {
                            // This neighbor is inside — push AWAY from it
                            push_dir.0 -= dx as f32;
                            push_dir.1 -= dy as f32;
                        }
                    }
                }
                let mag = (push_dir.0 * push_dir.0 + push_dir.1 * push_dir.1).sqrt();
                if mag > 0.1 {
                    let norm_x = push_dir.0 / mag;
                    let norm_y = push_dir.1 / mag;
                    // Inject outward velocity slightly inside the room (behind the door)
                    self.fluid_params.splat_x = fx - norm_x * 1.5;
                    self.fluid_params.splat_y = fy - norm_y * 1.5;
                    self.fluid_params.splat_vx = norm_x * 60.0;
                    self.fluid_params.splat_vy = norm_y * 60.0;
                    self.fluid_params.splat_radius = 3.0;
                    self.fluid_params.splat_active = 1.0;
                }
            }

            log::info!("Door at ({}, {}): {}", bx, by, if open { "opened" } else { "closed" });
            return;
        }

        // Toggle valve open/closed
        if bt == 18 {
            let new_flags = flags ^ 4; // toggle bit2 (is_open)
            let new_block = (block & 0xFF00FFFF) | ((new_flags as u32) << 16);
            self.grid_data[idx] = new_block;
            self.grid_dirty = true;
            let open = (new_flags & 4) != 0;
            log::info!("Valve at ({}, {}): {}", bx, by, if open { "open" } else { "closed" });
            return;
        }

        // Toggle switch on/off
        if bt == 42 && self.build_tool != BuildTool::Destroy {
            let new_flags = flags ^ 4; // toggle bit2
            let new_block = (block & 0xFF00FFFF) | ((new_flags as u32) << 16);
            self.grid_data[idx] = new_block;
            self.grid_dirty = true;
            let on = (new_flags & 4) != 0;
            log::info!("Switch at ({}, {}): {}", bx, by, if on { "ON" } else { "OFF" });
            return;
        }

        // Click dimmer: show slider popup
        if bt == 43 && self.build_tool != BuildTool::Destroy {
            let didx = by as u32 * GRID_W + bx as u32;
            self.block_sel.dimmer = if self.block_sel.dimmer == Some(didx) { None } else { Some(didx) };
            self.block_sel.dimmer_world = (bx as f32 + 0.5, by as f32 + 0.5);
            return;
        }

        // Click circuit breaker: reset (turn back ON) if tripped
        if bt == 45 && self.build_tool != BuildTool::Destroy {
            let is_on = (flags & 4) != 0;
            if !is_on {
                // Reset breaker (turn ON)
                let new_block = (block & 0xFF00FFFF) | (((flags | 4) as u32) << 16);
                self.grid_data[idx] = new_block;
                self.grid_dirty = true;
                log::info!("Circuit breaker at ({}, {}) RESET", bx, by);
            }
            return;
        }

        // Click fan: show speed popup (similar to pump)
        if bt == 12 && self.build_tool != BuildTool::Destroy {
            let fidx = by as u32 * GRID_W + bx as u32;
            self.block_sel.fan = if self.block_sel.fan == Some(fidx) { None } else { Some(fidx) };
            self.block_sel.fan_world = (bx as f32 + 0.5, by as f32 + 0.5);
            return;
        }

        // Click pump: toggle pump speed popup
        if bt == 16 {
            let pidx = by as u32 * GRID_W + bx as u32;
            self.block_sel.pump = if self.block_sel.pump == Some(pidx) { None } else { Some(pidx) };
            self.block_sel.pump_world = (bx as f32 + 0.5, by as f32 + 0.5);
            return;
        }

        // Removal is handled by the Destroy tool, not by clicking
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
                    wgpu::BindGroupEntry { binding: 11, resource: gfx.material_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 12, resource: gfx.pleb_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 13, resource: gfx.block_temp_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 14, resource: gfx.voltage_buffer.as_entire_binding() },
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
                    wgpu::BindGroupEntry { binding: 11, resource: gfx.material_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 12, resource: gfx.pleb_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 13, resource: gfx.block_temp_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 14, resource: gfx.voltage_buffer.as_entire_binding() },
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
                    wgpu::BindGroupEntry { binding: 11, resource: gfx.material_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 12, resource: gfx.pleb_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 13, resource: gfx.block_temp_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 14, resource: gfx.voltage_buffer.as_entire_binding() },
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
                    wgpu::BindGroupEntry { binding: 11, resource: gfx.material_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 12, resource: gfx.pleb_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 13, resource: gfx.block_temp_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 14, resource: gfx.voltage_buffer.as_entire_binding() },
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


        let dt = self.update_simulation();


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
            self.pipe_network.rebuild(&self.grid_data);
        }

        // Tick pipe network simulation — store outlet injections for post-shader application
        let pipe_injections = self.pipe_network.tick(dt, &self.grid_data, self.pipe_width);

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

        // Upload pleb data to GPU buffer
        {
            let mut gpu_plebs = [GpuPleb::zeroed(); MAX_PLEBS];
            for (i, p) in self.plebs.iter().enumerate().take(MAX_PLEBS) {
                gpu_plebs[i] = p.to_gpu(self.selected_pleb == Some(i));
            }
            gfx.queue.write_buffer(&gfx.pleb_buffer, 0, bytemuck::cast_slice(&gpu_plebs));
        }
        // --- egui frame setup (before bp_cam/blueprint computation) ---
        drop(gfx);
        let ctx = {
            let egui_state = self.egui_state.as_mut().unwrap();
            let window = self.window.as_ref().unwrap();
            let raw_input = egui_state.winit_state.take_egui_input(window);
            let ctx = egui_state.ctx.clone();
            ctx.begin_pass(raw_input);
            ctx
        };
        // NOTE: draw_ui call is placed after bp_cam/blueprint computation below

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
            // Diagonal wall drag: compute per-tile variants for triangle preview
            let diag_drag_tiles: Vec<(i32, i32, u8)> = if self.drag_start.is_some() && self.build_tool == BuildTool::Place(44) {
                let (sx, sy) = self.drag_start.unwrap();
                Self::diagonal_wall_tiles(sx, sy, hbx, hby, self.build_rotation)
            } else {
                Vec::new()
            };

            self.diag_preview = diag_drag_tiles.clone();
            let tiles: Vec<(i32, i32)> = if !diag_drag_tiles.is_empty() {
                diag_drag_tiles.iter().map(|&(x, y, _)| (x, y)).collect()
            } else if let Some((sx, sy)) = self.drag_start {
                // During drag: show the drag shape preview
                match self.build_tool {
                    BuildTool::Place(id) => {
                        let reg = block_defs::BlockRegistry::cached();
                        let shape = reg.get(id).and_then(|d| d.placement.as_ref()).and_then(|p| p.drag.as_ref());
                        match shape {
                            Some(block_defs::DragShape::Line) => Self::line_tiles(sx, sy, hbx, hby),
                            Some(block_defs::DragShape::FilledRect) => Self::filled_rect_tiles(sx, sy, hbx, hby),
                            Some(block_defs::DragShape::HollowRect) => Self::hollow_rect_tiles(sx, sy, hbx, hby),
                            Some(block_defs::DragShape::DiagonalLine) => Self::diagonal_wall_tiles(sx, sy, hbx, hby, self.build_rotation).iter().map(|&(x, y, _)| (x, y)).collect(),
                            _ => vec![(hbx, hby)],
                        }
                    }
                    BuildTool::Destroy => Self::filled_rect_tiles(sx, sy, hbx, hby),
                    BuildTool::Roof | BuildTool::RemoveRoof => Self::filled_rect_tiles(sx, sy, hbx, hby),
                    BuildTool::RemoveFloor => Self::filled_rect_tiles(sx, sy, hbx, hby),
                    _ => vec![(hbx, hby)],
                }
            } else {
                // No drag: single-tile or multi-tile preview
                match self.build_tool {
                    BuildTool::Place(9) => self.bench_tiles(hbx, hby, self.build_rotation).to_vec(),
                    BuildTool::Place(30) => self.bed_tiles(hbx, hby, self.build_rotation).to_vec(),
                    BuildTool::Place(37) => self.solar_tiles(hbx, hby).to_vec(),
                    BuildTool::Place(39) => self.bed_tiles(hbx, hby, self.build_rotation).to_vec(),
                    BuildTool::Place(40) => vec![(hbx, hby), (hbx+1, hby), (hbx, hby+1), (hbx+1, hby+1)],
                    BuildTool::Place(41) => vec![(hbx, hby), (hbx+1, hby), (hbx, hby+1), (hbx+1, hby+1)],
                    _ => vec![(hbx, hby)],
                }
            };
            let on_furniture = self.build_tool == BuildTool::Place(11);
            let is_physics = self.build_tool == BuildTool::WoodBox;
            let on_wall = matches!(self.build_tool, BuildTool::Place(12) | BuildTool::Window | BuildTool::Door | BuildTool::Place(19) | BuildTool::Place(20));
            tiles.iter().map(|&(tx, ty)| {
                if is_physics {
                    // Physics bodies can be placed anywhere
                    ((tx, ty), true)
                } else if on_wall {
                    let valid = if tx >= 0 && ty >= 0 && tx < GRID_W as i32 && ty < GRID_H as i32 {
                        let bidx = (ty as u32 * GRID_W + tx as u32) as usize;
                        let b = self.grid_data[bidx];
                        let bbt = b & 0xFF;
                        let bbh = (b >> 8) & 0xFF;
                        if self.build_tool == BuildTool::Window {
                            matches!(bbt, 1 | 4 | 14 | 21 | 22 | 23 | 24 | 25) && bbh > 0
                        } else if self.build_tool == BuildTool::Door {
                            matches!(bbt, 1 | 5 | 14 | 21 | 22 | 23 | 24 | 25) && bbh > 0
                        } else if matches!(self.build_tool, BuildTool::Place(19) | BuildTool::Place(20)) {
                            matches!(bbt, 1 | 4 | 5 | 14 | 21 | 22 | 23 | 24 | 25) && bbh > 0
                        } else if self.build_tool == BuildTool::Place(12) {
                            matches!(bbt, 1 | 4 | 5 | 14 | 21 | 22 | 23 | 24 | 25) && bbh > 0
                        } else {
                            (bbt == 1 || bbt == 4) && bbh > 0
                        }
                    } else { false };
                    // Inlet/Outlet/Fan can also place on ground
                    if !valid && matches!(self.build_tool, BuildTool::Place(19) | BuildTool::Place(20) | BuildTool::Place(12)) {
                        ((tx, ty), self.can_place_on(tx, ty, false))
                    } else {
                        ((tx, ty), valid)
                    }
                } else if self.build_tool == BuildTool::Place(36) {
                    // Wire can go anywhere
                    ((tx, ty), tx >= 0 && ty >= 0 && tx < GRID_W as i32 && ty < GRID_H as i32)
                } else if self.build_tool == BuildTool::Place(15) {
                    // Pipe: can go on empty ground OR existing pipe (merge connections)
                    let ok = if tx >= 0 && ty >= 0 && tx < GRID_W as i32 && ty < GRID_H as i32 {
                        let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                        let tbt = self.grid_data[tidx] & 0xFF;
                        self.can_place_on(tx, ty, false) || tbt == 15
                    } else { false };
                    ((tx, ty), ok)
                } else {
                    ((tx, ty), self.can_place_on(tx, ty, on_furniture))
                }
            }).collect()
        } else {
            vec![]
        };
        let bp_cam = (self.camera.center_x, self.camera.center_y, self.camera.zoom, self.camera.screen_w, self.camera.screen_h);

        // Draw all UI (egui pass was started above, ctx is the cloned context)
        self.draw_ui(&ctx, bp_cam, blueprint_tiles, dt);

        // End egui pass and prepare for GPU rendering
        let egui_state = self.egui_state.as_mut().unwrap();
        let window = self.window.as_ref().unwrap();
        let egui_output = ctx.end_pass();
        egui_state.winit_state.handle_platform_output(window, egui_output.platform_output.clone());
        let paint_jobs = ctx.tessellate(egui_output.shapes, ctx.pixels_per_point());

        let gfx = self.gfx.as_ref().unwrap();
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [gfx.config.width, gfx.config.height],
            pixels_per_point: window.scale_factor() as f32,
        };

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
        let need_lightmap = self.lightmap_frame >= LIGHTMAP_UPDATE_INTERVAL
            || self.grid_dirty
            || self.camera.force_refresh > 0.5;
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

        // 10. Thermal exchange (256x256)
        { let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("thermal"), timestamp_writes: None });
          let tw = (GRID_W + 7) / 8; let th = (GRID_H + 7) / 8;
          p.set_pipeline(&gfx.thermal_pipeline); p.set_bind_group(0, &gfx.thermal_bind_group, &[]); p.dispatch_workgroups(tw, th, 1); }

        // 11. Power grid voltage relaxation (256x256)
        #[cfg(target_arch = "wasm32")]
        let power_iters = 4;
        #[cfg(not(target_arch = "wasm32"))]
        let power_iters = 8;
        { let tw = (GRID_W + 7) / 8; let th = (GRID_H + 7) / 8;
          for _ in 0..power_iters {
            let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("power"), timestamp_writes: None });
            p.set_pipeline(&gfx.power_pipeline); p.set_bind_group(0, &gfx.power_bind_group, &[]); p.dispatch_workgroups(tw, th, 1);
          }
        }

        // Debug: copy one dye texel at cursor position for readback
        let shift_for_debug = self.pressed_keys.contains(&KeyCode::ShiftLeft)
            || self.pressed_keys.contains(&KeyCode::ShiftRight);
        if self.debug.mode || shift_for_debug {
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
            self.debug.fluid_pending = true;

            // Also copy block temperature at cursor position from block_temp_buffer
            let btx = wx.floor() as i32;
            let bty = wy.floor() as i32;
            if btx >= 0 && bty >= 0 && btx < GRID_W as i32 && bty < GRID_H as i32 {
                let bt_idx = (bty as u32 * GRID_W + btx as u32) as u64;
                encoder.copy_buffer_to_buffer(
                    &gfx.block_temp_buffer, bt_idx * 4,
                    &gfx.block_temp_readback_buffer, 0,
                    4,
                );
                encoder.copy_buffer_to_buffer(
                    &gfx.voltage_buffer, bt_idx * 4,
                    &gfx.block_temp_readback_buffer, 4, // second f32
                    4,
                );
                self.debug.block_temp_pending = true;
                self.debug.voltage_pending = true;
            }
        }

        // Copy dye texels at each pleb position for air readback
        if !self.plebs.is_empty() {
            let dye_idx = self.fluid_dye_phase;
            for (i, pleb) in self.plebs.iter().enumerate() {
                let dye_x = ((pleb.x / GRID_W as f32) * FLUID_DYE_W as f32)
                    .clamp(0.0, (FLUID_DYE_W - 1) as f32) as u32;
                let dye_y = ((pleb.y / GRID_H as f32) * FLUID_DYE_H as f32)
                    .clamp(0.0, (FLUID_DYE_H - 1) as f32) as u32;
                encoder.copy_texture_to_buffer(
                    wgpu::TexelCopyTextureInfo {
                        texture: &gfx.fluid_dye[dye_idx],
                        mip_level: 0,
                        origin: wgpu::Origin3d { x: dye_x, y: dye_y, z: 0 },
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyBufferInfo {
                        buffer: &gfx.pleb_air_readback_buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: (i * 256) as u64,
                            bytes_per_row: Some(256),
                            rows_per_image: Some(1),
                        },
                    },
                    wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                );
            }
            self.pleb_air_readback_pending = true;
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

            // Write pipe gas temperatures into block_temps buffer (AFTER thermal shader)
            // This makes pipe blocks show their internal gas temperature in the overlay
            // and allows heat exchange with surrounding air via the dye shader.
            for (&idx, cell) in &self.pipe_network.cells {
                let pipe_temp = cell.gas[3]; // temperature channel
                gfx.queue.write_buffer(
                    &gfx.block_temp_buffer,
                    (idx as u64) * 4,
                    bytemuck::bytes_of(&pipe_temp),
                );
            }

            // Lightning voltage surge: inject massive voltage at strike point (once per strike)
            if let Some((lx, ly)) = self.lightning_strike {
                if self.lightning_surge_done { } else {
                self.lightning_surge_done = true;
                let lix = lx.floor() as i32;
                let liy = ly.floor() as i32;
                if lix >= 0 && liy >= 0 && lix < GRID_W as i32 && liy < GRID_H as i32 {
                    let lidx = (liy as u32 * GRID_W + lix as u32) as usize;
                    let bt = block_type_rs(self.grid_data[lidx]);
                    let flags = block_flags_rs(self.grid_data[lidx]);
                    let is_conductor = bt == 36 || bt == 37 || bt == 38 || bt == 39
                        || bt == 40 || bt == 41 || bt == 42 || bt == 43
                        || (flags & 0x80) != 0;
                    if is_conductor {
                        // Inject 50V surge (way over the 12V max — will overload the network)
                        let surge: f32 = 50.0;
                        gfx.queue.write_buffer(
                            &gfx.voltage_buffer,
                            (lidx as u64) * 4,
                            bytemuck::bytes_of(&surge),
                        );
                    }
                }
                // Strike position stays alive until flash fades (for bolt rendering)
                // Voltage only injected once (this frame)
            }}  // close surge_done guard + lightning_strike

            // Apply pipe outlet injections to dye texture (AFTER shader runs)
            // Write into cells ADJACENT to the outlet (in the outlet's facing direction)
            for &(ox, oy, gas, pressure) in &pipe_injections {
                if pressure < 0.05 { continue; }
                let grid_idx = (oy as u32) * GRID_W + (ox as u32);
                let block = self.grid_data[grid_idx as usize];
                let dir_bits = (block >> 19) & 3; // bits 3-4 of flags = direction
                // Outlet direction: which way it faces (into the room)
                let (adx, ady): (i32, i32) = match dir_bits {
                    0 => (0, -1), // north
                    1 => (1, 0),  // east
                    2 => (0, 1),  // south
                    _ => (-1, 0), // west
                };
                let inject_x = ox as i32 + adx;
                let inject_y = oy as i32 + ady;
                if inject_x < 0 || inject_y < 0 || inject_x >= GRID_W as i32 || inject_y >= GRID_H as i32 { continue; }

                let dye_scale = (FLUID_DYE_W / FLUID_SIM_W) as i32;
                let dye_x = inject_x * dye_scale;
                let dye_y = inject_y * dye_scale;
                let s = (pressure * 0.5).min(1.0);
                let pixel: [u16; 4] = [
                    f32_to_f16(gas[0] * s),
                    f32_to_f16(gas[1].max(0.3)),
                    f32_to_f16(gas[2] * s),
                    f32_to_f16(gas[3]),
                ];
                let bytes: &[u8] = bytemuck::cast_slice(&pixel);
                // Write to BOTH dye textures so next frame's shader reads the injection
                // regardless of which texture is the ping-pong input
                for dy_off in 0..dye_scale {
                    for dx_off in 0..dye_scale {
                        let tx = (dye_x + dx_off).clamp(0, FLUID_DYE_W as i32 - 1) as u32;
                        let ty = (dye_y + dy_off).clamp(0, FLUID_DYE_H as i32 - 1) as u32;
                        for dye_idx in 0..2 {
                        gfx.queue.write_texture(
                            wgpu::TexelCopyTextureInfo {
                                texture: &gfx.fluid_dye[dye_idx],
                                mip_level: 0,
                                origin: wgpu::Origin3d { x: tx, y: ty, z: 0 },
                                aspect: wgpu::TextureAspect::All,
                            },
                            bytes,
                            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(8), rows_per_image: Some(1) },
                            wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                        );
                        } // for dye_idx
                    }
                }
            }

            // Grenade toxic gas injection: continuous emission while fuse burns
            for &(gx, gy) in &self.grenade_impacts {
                let dye_scale = (FLUID_DYE_W / FLUID_SIM_W) as i32;
                let radius = 1i32; // small source — fluid sim spreads it
                for oy in -radius..=radius {
                    for ox in -radius..=radius {
                        let dist = ((ox * ox + oy * oy) as f32).sqrt();
                        if dist > radius as f32 + 0.5 { continue; }
                        let strength = 1.0 - dist / (radius as f32 + 1.0);
                        let wx = (gx as i32 + ox).clamp(0, GRID_W as i32 - 1);
                        let wy = (gy as i32 + oy).clamp(0, GRID_H as i32 - 1);
                        let dye_bx = wx * dye_scale;
                        let dye_by = wy * dye_scale;
                        // Toxic: moderate smoke (R) + high CO2 (B), depleted O2 (G)
                        // Per-frame emission — builds up over 12 seconds
                        let pixel: [u16; 4] = [
                            f32_to_f16(0.6 * strength),   // smoke
                            f32_to_f16(0.2),               // very low O2
                            f32_to_f16(0.8 * strength),   // high CO2
                            f32_to_f16(15.0),              // slightly cool (chemical reaction)
                        ];
                        let bytes: &[u8] = bytemuck::cast_slice(&pixel);
                        for dy_off in 0..dye_scale {
                            for dx_off in 0..dye_scale {
                                let tx = (dye_bx + dx_off).clamp(0, FLUID_DYE_W as i32 - 1) as u32;
                                let ty = (dye_by + dy_off).clamp(0, FLUID_DYE_H as i32 - 1) as u32;
                                for dye_idx in 0..2 {
                                    gfx.queue.write_texture(
                                        wgpu::TexelCopyTextureInfo {
                                            texture: &gfx.fluid_dye[dye_idx],
                                            mip_level: 0,
                                            origin: wgpu::Origin3d { x: tx, y: ty, z: 0 },
                                            aspect: wgpu::TextureAspect::All,
                                        },
                                        bytes,
                                        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(8), rows_per_image: Some(1) },
                                        wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                                    );
                                }
                            }
                        }
                    }
                }
            }
            self.grenade_impacts.clear();

            // Debug: read back the dye texel
            // Debug readback processing
            if self.debug.fluid_pending {
                self.debug.fluid_pending = false;
                // Synchronous readback — native only (WASM can't block-wait on GPU)
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let buffer_slice = gfx.debug_readback_buffer.slice(..);
                    buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
                    gfx.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok();
                    let data = buffer_slice.get_mapped_range();
                    let f16_data: &[u16] = bytemuck::cast_slice(&data);
                    for i in 0..4 {
                        self.debug.fluid_density[i] = half_to_f32(f16_data[i]);
                    }
                    drop(data);
                    gfx.debug_readback_buffer.unmap();
                }
            }

            // Block temperature + voltage readback processing
            if self.debug.block_temp_pending {
                self.debug.block_temp_pending = false;
                self.debug.voltage_pending = false;
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let buffer_slice = gfx.block_temp_readback_buffer.slice(..8); // 2 f32s
                    buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
                    gfx.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok();
                    let data = buffer_slice.get_mapped_range();
                    let values: &[f32] = bytemuck::cast_slice(&data);
                    self.debug.block_temp = values[0];
                    self.debug.voltage = values[1];
                    drop(data);
                    gfx.block_temp_readback_buffer.unmap();
                }
            }

            // Pleb air readback processing
            if self.pleb_air_readback_pending {
                self.pleb_air_readback_pending = false;
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let num_plebs = self.plebs.len();
                    if num_plebs > 0 {
                        let read_size = (num_plebs * 256) as u64;
                        let buffer_slice = gfx.pleb_air_readback_buffer.slice(..read_size);
                        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
                        gfx.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok();
                        let data = buffer_slice.get_mapped_range();
                        self.pleb_air_data.resize(num_plebs, AirReadback::default());
                        for i in 0..num_plebs {
                            let offset = i * 256; // 256-byte aligned
                            let f16_data: &[u16] = bytemuck::cast_slice(&data[offset..offset + 8]);
                            self.pleb_air_data[i] = AirReadback {
                                smoke: half_to_f32(f16_data[0]),
                                o2: half_to_f32(f16_data[1]),
                                co2: half_to_f32(f16_data[2]),
                                temp: half_to_f32(f16_data[3]),
                            };
                        }
                        drop(data);
                        gfx.pleb_air_readback_buffer.unmap();
                    }
                }
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

    fn handle_keyboard(&mut self, event: &winit::event::KeyEvent) {
        if let PhysicalKey::Code(code) = event.physical_key {
            if event.state.is_pressed() {
                self.pressed_keys.insert(code);
            } else {
                self.pressed_keys.remove(&code);
            }
        }
        if event.state.is_pressed() {
            match event.physical_key {
                PhysicalKey::Code(KeyCode::Escape) => {
                    self.placing_pleb = false;
                    if self.debug.mode {
                        self.debug.mode = false;
                    } else if self.selected_pleb.is_some() {
                        self.selected_pleb = None;
                    } else if self.build_tool != BuildTool::None {
                        self.build_tool = BuildTool::None;
                    }
                }
                PhysicalKey::Code(KeyCode::KeyR) => {
                    if self.camera.show_roofs < 0.5 {
                        self.camera.show_roofs = 1.0;
                    } else {
                        self.camera.show_roofs = 0.0;
                    }
                    self.window.as_ref().unwrap().request_redraw();
                }
                PhysicalKey::Code(KeyCode::Space) => {
                    if self.selected_pleb.is_some() && self.burst_queue == 0 {
                        if self.burst_mode {
                            self.burst_queue = 3;
                            self.burst_delay = 0.0; // fire first shot immediately
                        } else {
                            self.burst_queue = 1;
                            self.burst_delay = 0.0;
                        }
                    } else if self.selected_pleb.is_none() {
                        self.time_paused = !self.time_paused;
                    }
                }
                PhysicalKey::Code(KeyCode::KeyB) => {
                    if self.selected_pleb.is_some() {
                        // Start charging grenade
                        self.grenade_charging = true;
                        self.grenade_charge = 0.0;
                    }
                }
                PhysicalKey::Code(KeyCode::KeyX) => {
                    if self.selected_pleb.is_some() {
                        self.burst_mode = !self.burst_mode;
                        log::info!("Fire mode: {}", if self.burst_mode { "BURST" } else { "SINGLE" });
                    }
                }
                PhysicalKey::Code(KeyCode::KeyQ) => {
                    if self.selected_pleb.is_none() {
                        if matches!(self.build_tool, BuildTool::Place(12) | BuildTool::Place(16) | BuildTool::Place(20) | BuildTool::Place(19) | BuildTool::Place(44)) {
                            self.build_rotation = (self.build_rotation + 3) % 4;
                        } else {
                            self.build_rotation = (self.build_rotation + 1) % 2;
                        }
                    }
                }
                PhysicalKey::Code(KeyCode::KeyE) => {
                    if self.selected_pleb.is_none() {
                        if matches!(self.build_tool, BuildTool::Place(12) | BuildTool::Place(16) | BuildTool::Place(20) | BuildTool::Place(19) | BuildTool::Place(44)) {
                            self.build_rotation = (self.build_rotation + 1) % 4;
                        } else {
                            self.build_rotation = (self.build_rotation + 1) % 2;
                        }
                    }
                }
                PhysicalKey::Code(KeyCode::KeyT) => {
                    if let Some(idx) = self.selected_pleb {
                        if let Some(pleb) = self.plebs.get_mut(idx) {
                            pleb.torch_on = !pleb.torch_on;
                        }
                    }
                }
                PhysicalKey::Code(KeyCode::KeyG) => {
                    if let Some(idx) = self.selected_pleb {
                        if let Some(pleb) = self.plebs.get_mut(idx) {
                            pleb.headlight_on = !pleb.headlight_on;
                        }
                    }
                }
                PhysicalKey::Code(KeyCode::KeyI) => {
                    if self.selected_pleb.is_some() {
                        self.show_inventory = !self.show_inventory;
                    }
                }
                PhysicalKey::Code(KeyCode::KeyF) => {
                    if let Some(pleb) = self.selected_pleb.and_then(|i| self.plebs.get(i)) {
                        let px = pleb.x;
                        let py = pleb.y;
                        let angle = pleb.angle;
                        if let Some(idx) = nearest_body(&self.physics_bodies, px, py, 1.2) {
                            let dx = angle.cos();
                            let dy = angle.sin();
                            self.physics_bodies[idx].throw(dx, dy, 18.0);
                        }
                    }
                }
                _ => {}
            }
        }
        // Key release: throw grenade on B release
        if !event.state.is_pressed() {
            if let PhysicalKey::Code(KeyCode::KeyB) = event.physical_key {
                if self.grenade_charging {
                    self.grenade_charging = false;
                    if let Some(pleb) = self.selected_pleb.and_then(|i| self.plebs.get(i)) {
                        let dx = pleb.angle.cos();
                        let dy = pleb.angle.sin();
                        let spawn_x = pleb.x + dx * 0.5;
                        let spawn_y = pleb.y + dy * 0.5;
                        let power = self.grenade_charge.clamp(0.0, 1.0);
                        self.physics_bodies.push(PhysicsBody::new_grenade(
                            spawn_x, spawn_y, dx, dy, power,
                        ));
                    }
                    self.grenade_charge = 0.0;
                }
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        #[cfg(not(target_arch = "wasm32"))]
        let attrs = {
            // 50% larger than default, matching monitor aspect ratio
            let monitor = event_loop.primary_monitor()
                .or_else(|| event_loop.available_monitors().next());
            let (w, h) = if let Some(m) = monitor {
                let size = m.size();
                ((size.width as f32 * 0.75) as u32, (size.height as f32 * 0.75) as u32)
            } else {
                (1440, 900)
            };
            Window::default_attributes()
                .with_title("Rayworld")
                .with_inner_size(PhysicalSize::new(w, h))
        };

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
                app.init_gfx_async(window_clone.clone()).await;
                // Force a resize + redraw after GPU init to show content immediately
                let size = window_clone.inner_size();
                app.resize(size);
                window_clone.request_redraw();
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
                // Shape-building tools: don't pan, just track drag
                if self.mouse_dragged && self.drag_start.is_some() {
                    // Preview is drawn in the egui section — just don't pan
                } else if self.mouse_dragged {
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
            WindowEvent::KeyboardInput { event, .. } => self.handle_keyboard(&event),
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y as f64,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y / 50.0,
                };
        let base_zoom = (self.camera.screen_w / 64.0).min(self.camera.screen_h / 64.0);
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
                        // Start drag for shape-building tools
                        let is_shape_tool = match self.build_tool {
                            BuildTool::Destroy | BuildTool::Roof | BuildTool::RemoveFloor | BuildTool::RemoveRoof => true,
                            BuildTool::Place(id) => {
                                let reg = block_defs::BlockRegistry::cached();
                                reg.get(id).and_then(|d| d.placement.as_ref()).and_then(|p| p.drag.as_ref())
                                    .map(|s| *s != block_defs::DragShape::None).unwrap_or(false)
                            }
                            _ => false,
                        };
                        if is_shape_tool {
                            let (wx, wy) = self.screen_to_world(self.last_mouse_x, self.last_mouse_y);
                            self.drag_start = Some((wx.floor() as i32, wy.floor() as i32));
                        }
                    } else {
                        // Mouse released
                        if self.mouse_dragged && self.drag_start.is_some() {
                            // Apply the drag shape
                            let (sx, sy) = self.drag_start.unwrap();
                            let (wx, wy) = self.screen_to_world(self.last_mouse_x, self.last_mouse_y);
                            let (ex, ey) = (wx.floor() as i32, wy.floor() as i32);
                            self.apply_drag_shape(sx, sy, ex, ey);
                        } else if !self.mouse_dragged {
                            let (wx, wy) = self.screen_to_world(self.last_mouse_x, self.last_mouse_y);
                            self.handle_click(wx, wy);
                        }
                        self.mouse_pressed = false;
                        self.mouse_dragged = false;
                        self.drag_start = None;
                    }
                }
                // Right-click: context menu for selected pleb, rock menu, or pick up lights
                if button == winit::event::MouseButton::Right {
                    if state.is_pressed() {
                        let (wx, wy) = self.screen_to_world(self.last_mouse_x, self.last_mouse_y);
                        // Check if right-clicking a rock
                        let rbx = wx.floor() as i32;
                        let rby = wy.floor() as i32;
                        let rblock_type = if rbx >= 0 && rby >= 0 && rbx < GRID_W as i32 && rby < GRID_H as i32 {
                            self.grid_data[(rby as u32 * GRID_W + rbx as u32) as usize] & 0xFF
                        } else { 0 };
                        if rblock_type == 34 {
                            // Right-click rock: open rock context menu
                            self.rock_context_menu = Some((self.last_mouse_x as f32, self.last_mouse_y as f32, rbx, rby));
                        } else if rblock_type == 33 {
                            // Right-click storage crate: deposit if carrying, else inspect
                            let cidx = rby as u32 * GRID_W + rbx as u32;
                            let mut deposited = false;
                            if let Some(sel_idx) = self.selected_pleb {
                                if let Some(pleb) = self.plebs.get_mut(sel_idx) {
                                    if pleb.inventory.carrying.is_some() {
                                        let dist = ((pleb.x - rbx as f32 - 0.5).powi(2) + (pleb.y - rby as f32 - 0.5).powi(2)).sqrt();
                                        if dist < 2.5 {
                                            // Close enough — deposit now
                                            let inv = self.crate_contents.entry(cidx).or_default();
                                            if pleb.inventory.carrying == Some("Rock") {
                                                let can_store = inv.space().min(pleb.inventory.rocks);
                                                inv.rocks += can_store;
                                                pleb.inventory.rocks -= can_store;
                                                if pleb.inventory.rocks == 0 { pleb.inventory.carrying = None; }
                                            }
                                            if pleb.inventory.carrying.is_none() {
                                                pleb.haul_target = None;
                                                pleb.activity = PlebActivity::Idle;
                                            }
                                            self.sync_crate_visual(cidx);
                                            deposited = true;
                                        } else {
                                            // Walk to crate and deposit
                                            let adj = adjacent_walkable(&self.grid_data, rbx, rby).unwrap_or((rbx, rby));
                                            let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                                            let path = astar_path(&self.grid_data, start, adj);
                                            if !path.is_empty() {
                                                pleb.path = path;
                                                pleb.path_idx = 0;
                                                pleb.activity = PlebActivity::Hauling;
                                                pleb.haul_target = Some((rbx, rby));
                                                pleb.harvest_target = None;
                                            }
                                            deposited = true;
                                        }
                                    }
                                }
                            }
                            if !deposited {
                                // Just inspect the crate
                                self.block_sel.crate_idx = if self.block_sel.crate_idx == Some(cidx) { None } else { Some(cidx) };
                                self.block_sel.crate_world = (rbx as f32 + 0.5, rby as f32 + 0.5);
                            }
                        } else if self.selected_pleb.is_some() {
                            self.context_menu = Some((self.last_mouse_x as f32, self.last_mouse_y as f32));
                        } else {
                            self.try_pick_light(wx, wy);
                        }
                    } else if self.dragging_light.is_some() {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Check if a pixel at (fx, fy) within a tile is on the wall half for a given variant.
    fn is_wall(fx: f32, fy: f32, variant: u8) -> bool {
        match variant {
            0 => fy > (1.0 - fx),
            1 => fy > fx,
            2 => fy < (1.0 - fx),
            3 => fy < fx,
            _ => false,
        }
    }

    /// Check that diagonal wall tiles form a continuous wall with no gaps.
    /// "No gaps" means: for every pair of adjacent tiles in the result,
    /// the solid halves share at least one edge pixel.
    fn assert_no_gaps(tiles: &[(i32, i32, u8)], label: &str) {
        // Build a set of (x, y, variant) for quick lookup
        let tile_set: std::collections::HashMap<(i32, i32), u8> = tiles.iter().map(|&(x, y, v)| ((x, y), v)).collect();

        // For each tile, check that it connects to at least one neighbor's solid half
        for &(tx, ty, tv) in tiles {
            let neighbors = [(tx - 1, ty), (tx + 1, ty), (tx, ty - 1), (tx, ty + 1)];
            let mut connected = false;

            for (nx, ny) in neighbors {
                if let Some(&nv) = tile_set.get(&(nx, ny)) {
                    // Check if the shared edge has solid pixels on both sides.
                    // Sample 5 points along the shared edge.
                    let edge_solid_count = (0..5).filter(|&i| {
                        let t = (i as f32 + 0.5) / 5.0;
                        let (tfx, tfy, nfx, nfy) = if nx == tx - 1 {
                            // neighbor is to the left: shared edge at fx=0 of current, fx=1 of neighbor
                            (0.0, t, 1.0, t)
                        } else if nx == tx + 1 {
                            (1.0, t, 0.0, t)
                        } else if ny == ty - 1 {
                            (t, 0.0, t, 1.0)
                        } else {
                            (t, 1.0, t, 0.0)
                        };
                        is_wall(tfx, tfy, tv) && is_wall(nfx, nfy, nv)
                    }).count();

                    if edge_solid_count >= 3 {
                        connected = true;
                        break;
                    }
                }
            }

            // Every tile except possibly the first/last should connect to another
            // (first and last are at the ends of the wall)
            if !connected && tiles.len() > 1 {
                // Check if it's an endpoint (first or last main tile on the diagonal)
                let is_endpoint = (tx, ty) == (tiles[0].0, tiles[0].1)
                    || (tx, ty) == (tiles.last().unwrap().0, tiles.last().unwrap().1);
                if !is_endpoint {
                    panic!("{}: tile ({},{}) variant {} has no connected neighbor", label, tx, ty, tv);
                }
            }
        }
    }

    /// Verify no duplicate tile positions (each grid cell should appear at most once).
    fn assert_no_duplicates(tiles: &[(i32, i32, u8)], label: &str) {
        let mut seen = std::collections::HashSet::new();
        for &(x, y, _) in tiles {
            if !seen.insert((x, y)) {
                panic!("{}: duplicate tile at ({},{})", label, x, y);
            }
        }
    }

    #[test]
    fn test_diagonal_wall_tiles_all_directions() {
        // Test all 8 combinations: 4 variants × 2 drag orientations (but variant
        // auto-adapts to drag direction, so we test all 4 rotations × 4 directions)
        let directions: [(i32, i32, &str); 4] = [
            (3, 3, "right-down"),    // \ direction
            (-3, -3, "left-up"),     // \ direction
            (3, -3, "right-up"),     // / direction
            (-3, 3, "left-down"),    // / direction
        ];

        for rotation in 0..4u32 {
            for &(dx, dy, dir_name) in &directions {
                let x0 = 10;
                let y0 = 10;
                let x1 = x0 + dx;
                let y1 = y0 + dy;
                let label = format!("rot={} dir={}", rotation, dir_name);

                let tiles = compute_diagonal_wall_tiles(x0, y0, x1, y1, rotation);

                // Should have main + fill tiles (2*steps - 1 for steps > 0)
                assert!(!tiles.is_empty(), "{}: no tiles generated", label);

                assert_no_duplicates(&tiles, &label);
                assert_no_gaps(&tiles, &label);
            }
        }
    }

    #[test]
    fn test_diagonal_wall_single_tile() {
        let tiles = compute_diagonal_wall_tiles(5, 5, 5, 5, 0);
        assert_eq!(tiles.len(), 1);
        assert_eq!(tiles[0], (5, 5, 0));
    }

    #[test]
    fn test_diagonal_wall_tiles_symmetry() {
        // Dragging from A to B should produce the same tile positions as B to A
        // (just in a different order), since the wall should look the same.
        let fwd = compute_diagonal_wall_tiles(5, 5, 8, 8, 1);
        let rev = compute_diagonal_wall_tiles(8, 8, 5, 5, 1);

        let mut fwd_set: Vec<(i32, i32)> = fwd.iter().map(|&(x, y, _)| (x, y)).collect();
        let mut rev_set: Vec<(i32, i32)> = rev.iter().map(|&(x, y, _)| (x, y)).collect();
        fwd_set.sort();
        rev_set.sort();
        assert_eq!(fwd_set, rev_set, "forward and reverse drags should cover same tiles");
    }
}

