#![allow(dead_code)] // Suppress warnings for data-driven fields and future-use utilities

use bytemuck::Zeroable;
use std::sync::Arc;

/// Check if a value matches any of the given block type constants.
/// Usage: `bt_is!(bt, BT_TREE, BT_FIREPLACE, BT_CEILING_LIGHT)`
/// Expands to: `bt == BT_TREE || bt == BT_FIREPLACE || bt == BT_CEILING_LIGHT`
macro_rules! bt_is {
    ($val:expr, $($bt:expr),+ $(,)?) => {
        $( $val == $bt )||+
    }
}

mod audio;
mod block_defs;
mod creature_defs;
mod creatures;
mod grid;
pub mod item_defs;
mod materials;
pub mod recipe_defs;
mod sprites;

use grid::*;
use item_defs::{ITEM_ROCK, ITEM_SCRAP_WOOD};
use materials::{GpuMaterial, build_material_table};
use sprites::generate_tree_sprites;

mod pleb;
use pleb::{
    GpuPleb, MAX_PLEBS, MentalBreakKind, Pleb, PlebActivity, PlebCommand, PlebShift,
    adjacent_walkable, astar_path_terrain_wd, is_walkable_pos, is_walkable_pos_wd, random_name,
};

mod needs;
use needs::{
    AirReadback, BERRY_HUNGER_RESTORE, BreathingState, HEAT_CRISIS_TEMP, WELL_DRINK_TIME,
    WELL_THIRST_RESTORE, breathing_label, find_breathable_tile, find_cool_tile, find_nearest_crate,
    find_nearest_well, mood_label, sample_environment, tick_needs,
};

mod build;
mod camera;
mod fluid;

use build::{BuildTool, FluidOverlay};
use camera::CameraUniform;
use fluid::{
    FLUID_DYE_H, FLUID_DYE_W, FLUID_PRESSURE_ITERS, FLUID_SIM_H, FLUID_SIM_MAX, FLUID_SIM_W,
    FluidParams, build_obstacle_field, f32_to_f16, half_to_f32, smoothstep_f32,
};

mod comms;
mod dust;
mod gpu_init;
mod pipes;
mod simulation;
mod ui;
use pipes::PipeNetwork;

mod physics;
use physics::{PROJ_BULLET, PhysicsBody, nearest_body, projectile_def, tick_bodies};

mod zones;
use zones::{Zone, ZoneKind};

mod weather;
use weather::{WeatherState, tick_weather, tick_wetness};

mod resources;
use resources::{CRATE_MAX_ITEMS, CrateInventory};

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

mod types;
use types::*;

mod placement;
#[cfg(test)]
pub(crate) use placement::compute_diagonal_wall_tiles;

mod fire;
mod fog;
mod input;

const WORKGROUP_SIZE: u32 = 8;
const DAY_DURATION: f32 = 60.0; // must match shader

// --- Gameplay tuning constants ---
const DOUBLE_CLICK_FRAMES: u32 = 30; // ~0.5s at 60fps
const PLEB_CLICK_RADIUS: f32 = 0.5; // world units to detect pleb click
const ZOOM_FACTOR: f32 = 1.1; // per scroll tick
const ZOOM_MIN_MULT: f32 = 0.2; // relative to base zoom
const ZOOM_MAX_MULT: f32 = 8.0; // relative to base zoom
const BURST_SHOT_COUNT: u8 = 3;
const LIGHTNING_SURGE_RADIUS: i32 = 12;
const LIGHTNING_SURGE_VOLTAGE: f32 = 200.0;
const LIGHTNING_BREAKER_RADIUS: i32 = 20;
const WATER_INJECT_RADIUS: i32 = 3;
const LIGHTMAP_MARGIN: f32 = 14.0; // tiles of margin >= max light radius
const MAX_SOUND_SOURCES: usize = 32;
const SOUND_SOURCE_STRIDE: usize = 8; // f32s per source in GPU buffer
const READBACK_ALIGNMENT: u64 = 256; // wgpu COPY_BYTES_PER_ROW_ALIGNMENT
const DRAG_THRESHOLD: f64 = 3.0; // pixels before drag is detected
const CAMERA_START_HOUR: f32 = 8.0; // game starts at 08:00
const DEFAULT_WINDOW_SIZE: (u32, u32) = (1440, 900);
const WINDOW_SCALE: f32 = 0.75; // fraction of monitor size

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
    time_of_day: f32,         // current time in seconds (0..DAY_DURATION)
    time_paused: bool,        // pause auto-advance
    show_pause_menu: bool,    // ESC game menu (quit, settings, etc.)
    time_speed: f32,          // playback speed multiplier
    last_frame_time: Instant, // for delta-time calculation
    last_click_frame: u32,
    last_click_pos: (i32, i32),
    // FPS tracking
    frame_count: u32,
    fps_accum: f32,
    fps_display: f32,
    // Lightmap update throttle (skip most frames)
    lightmap_frame: u32,
    // Build mode
    build_tool: BuildTool,
    build_rotation: u32,                  // 0=horizontal (E-W), 1=vertical (N-S)
    wall_thickness: u8,                   // 1-4 sub-cells (4=full, default)
    hover_world: (f32, f32),              // world coords under mouse cursor
    move_marker: Option<(f32, f32, f32)>, // (world_x, world_y, fade_timer)
    // Terrain sculpting
    terrain_tool: Option<TerrainTool>,
    terrain_brush_radius: u8, // 1-5
    // Fluid simulation
    fluid_params: FluidParams,
    fluid_dye_phase: usize, // 0 or 1: which dye texture is current (readable)
    output_phase: usize,    // 0 or 1: ping-pong for temporal reprojection
    prev_cam_x: f32,        // previous frame's camera center (for temporal reprojection)
    prev_cam_y: f32,
    prev_cam_zoom: f32,
    prev_cam_time: f32,
    fluid_overlay: FluidOverlay,
    pipe_network: PipeNetwork,   // gas pipe simulation
    liquid_network: PipeNetwork, // liquid pipe simulation
    fluid_speed: f32,            // fluid simulation speed multiplier
    enable_terrain_detail: bool, // procedural terrain variation (grass, pebbles, etc.)
    terrain_ao_strength: f32,    // terrain ambient occlusion strength (0-1)
    show_contours: bool,         // contour lines on/off
    contour_opacity: f32,        // contour line intensity (0 = off, 1 = full)
    contour_interval: f32,       // minor contour line spacing (elevation units)
    contour_major_mul: f32,      // major line every N minor intervals
    enable_prox_glow: bool,      // per-pixel proximity glow (expensive)
    enable_dir_bleed: bool,      // directional light bleed (expensive)
    enable_temporal: bool,       // temporal reprojection (reuse previous frame)
    enable_ricochets: bool,      // bullets bounce off walls
    hires_fluid: bool,           // 512x512 fluid sim (4x compute cost)
    fluid_pressure_iters: u32,   // Jacobi pressure solver iterations (quality vs perf)
    lightmap_interval: u32,      // recompute lightmap every N frames
    lightmap_iterations: u32,    // lightmap propagation iterations (radius)
    // Sound propagation
    sound_enabled: bool,
    sound_phase: usize, // 0 or 1 ping-pong
    sound_sources: Vec<SoundSource>,
    sound_speed: f32,             // wave propagation speed (c)
    sound_damping: f32,           // damping factor per step
    sound_coupling: f32,          // sound→gas velocity coupling strength
    sound_iters_per_frame: u32,   // iterations per frame (controls propagation speed)
    camera_pan_speed: f32,        // WASD pan speed (tiles/sec at zoom=1)
    dye_w: u32,                   // current dye texture width (tracks render resolution)
    dye_h: u32,                   // current dye texture height
    sandbox_mode: bool,           // enables sandbox build category + debug tools
    debug_creatures_always: bool, // spawn creatures regardless of time of day
    debug_bullet_slowmo: bool,    // enable bullet slow-mo
    debug_bullet_speed: f32,      // bullet time scale (0.01 = 100x slow, 1.0 = normal)
    debug_show_cover: bool,       // show cover positions as circles
    debug_show_flock: bool,       // show flock cohesion lines
    audio_output: Option<audio::AudioOutput>,
    sandbox_tool: SandboxTool,            // current sandbox action
    show_pipe_overlay: bool,              // draw gas pipe contents as egui overlay (ventilation)
    show_grid: bool,                      // tile grid lines overlay
    show_subgrid: bool,                   // 4x4 sub-grid around cursor
    wall_data: Vec<u16>,                  // wall edge layer (u16 per tile, see DN-008)
    show_liquid_overlay: bool,            // draw liquid pipe contents as egui overlay
    show_flow_overlay: bool, // draw flow arrows on pipes (pressure) and wires (current)
    show_velocity_arrows: bool, // draw fluid velocity vector field on overlays
    pipe_width: f32,         // pipe conductance multiplier (1=narrow, 10=wide)
    drag_start: Option<(i32, i32)>, // grid coords where drag started (for shape building)
    block_sel: BlockSelection, // which popup/slider is open
    build_category: Option<&'static str>, // selected build category, None = collapsed
    debug: DebugReadback,    // shift-hover readback state
    middle_mouse_pressed: bool, // middle mouse button held (fast pan)
    // Pleb (character)
    plebs: Vec<Pleb>,
    selected_pleb: Option<usize>, // index into plebs vec
    selected_group: Vec<usize>,   // multi-selected pleb indices for group commands
    placing_pleb: bool,
    placing_enemy: bool,
    next_pleb_id: usize,
    // Alien fauna
    creatures: Vec<creatures::Creature>,
    creature_spawn_timer: f32,
    next_pack_id: u16,
    blood_stains: Vec<(f32, f32, f32)>, // (x, y, fade_timer) — blood drops on ground
    cannon_angles: std::collections::HashMap<u32, f32>, // grid_idx → angle (radians)
    show_pleb_help: bool,               // show controls modal
    show_inventory: bool,               // show pleb inventory window
    inv_selected_slot: Option<usize>,   // selected inventory slot for swap/drop
    show_schedule: bool,                // show shift schedule window
    show_priorities: bool,              // show work priorities window
    pressed_keys: std::collections::HashSet<KeyCode>,
    doors: Vec<Door>,     // physical hinged doors
    doors_dirty: bool,    // door angles changed, re-upload door_buffer
    roof_paid: Vec<bool>, // per-tile: has fiber been delivered for this roof tile?
    roof_flash: Vec<f32>, // per-tile: seconds remaining to show roof after completion
    physics_bodies: Vec<PhysicsBody>,
    ground_items: Vec<resources::GroundItem>,
    blueprints: std::collections::HashMap<(i32, i32), Blueprint>,
    game_hints: Vec<GameHint>,
    active_hint_idx: Option<usize>, // hint being hovered → highlights active
    // Per-pleb air readback from fluid sim (updated one frame behind)
    pleb_air_data: Vec<AirReadback>,
    pleb_air_readback_pending: bool,
    // Context menu for pleb actions
    context_menu: Option<ContextMenu>,
    // World selection (Rimworld-style: click anything to inspect)
    world_sel: WorldSelection,
    // In-game event log
    game_log: std::collections::VecDeque<GameEvent>,
    // Event notifications (right panel) + active conditions
    notifications: Vec<GameNotification>,
    conditions: Vec<ActiveCondition>,
    next_notif_id: u32,
    drought_check_timer: f32,
    // Multi-select drag rectangle (screen coords)
    select_drag_start: Option<(f32, f32)>, // world coords where selection drag started
    // Storage crate inventories: grid_idx → stored items
    crate_contents: std::collections::HashMap<u32, CrateInventory>,
    craft_queues: std::collections::HashMap<u32, CraftQueue>,
    // Rock context menu
    // Combat
    grenade_charging: bool,
    grenade_charge: f32,
    grenade_impacts: Vec<(f32, f32)>,
    burst_mode: bool,
    burst_queue: u8,  // remaining burst shots to fire (0 = none)
    burst_delay: f32, // seconds until next burst shot
    // Weather system
    weather: WeatherState,
    weather_timer: f32,
    // Lightning
    lightning_timer: f32,                 // seconds until next potential strike
    lightning_flash: f32,                 // flash brightness (decays rapidly, 0-1)
    lightning_strike: Option<(f32, f32)>, // (x, y) of current strike for rendering
    lightning_surge_done: bool,           // prevents re-injecting voltage surge
    // Wind variation: slowly drifting target angle + magnitude
    wind_target_angle: f32, // target angle in radians
    wind_target_mag: f32,   // target magnitude
    wind_change_timer: f32, // seconds until next target shift
    wetness_data: Vec<f32>,
    // Zones & work queue
    zones: Vec<Zone>,
    active_work: std::collections::HashSet<(i32, i32)>,
    manual_tasks: Vec<zones::WorkTask>, // player-ordered tasks (harvest bush, etc.)
    work_priority: zones::WorkPriority,
    crop_timers: std::collections::HashMap<u32, f32>,
    water_phase: usize,
    water_frame: u32,
    water_table: Vec<f32>, // static water table height map (CPU copy for info overlay)
    elevation_data: Vec<f32>, // terrain elevation (0.0–6.0 tiles of height)
    terrain_data: Vec<u32>, // per-tile terrain type, vegetation, richness etc.
    terrain_dirty: bool,   // true when terrain_data needs re-upload to GPU
    terrain_params: grid::TerrainParams,
    game_state: GameState,
    // Character generation
    chargen_name: String,
    chargen_skin: [f32; 3],
    chargen_hair: [f32; 3],
    chargen_hair_style: u8,
    chargen_shirt: [f32; 3],
    chargen_pants: [f32; 3],
    chargen_backstory: Backstory,
    chargen_body_type: BodyType,
    chargen_gender: Gender,
    chargen_age: u8,
    chargen_trait: Option<PlebTrait>,
    chargen_preview_angle: f32, // rotating preview angle
    // Diagonal wall drag preview: (x, y, variant) per tile
    diag_preview: Vec<(i32, i32, u8)>,
    // Entryway position for hollow rect drag (shown differently in preview)
    drag_entryway: Option<(i32, i32)>,
    // Entry side override for hollow rect: 0=auto, 1=N, 2=E, 3=S, 4=W
    entry_side: u8,
    // Campfire subtile offset for preview: (x_off, y_off) in 0-2 range, or None for full tile
    campfire_subtile: Option<(u8, u8)>,
    menu_hover_id: Option<egui::Id>, // last hovered button ID for sound debounce
    // Per-tile voltage snapshot for labels (read back from GPU when power overlay active)
    voltage_data: Vec<f32>,
    voltage_readback_pending: bool,
    // Fog of war
    fog_enabled: bool,
    fog_visibility: Vec<u8>, // 256×256, per-tile: 0=not visible, 255=visible
    fog_explored: Vec<u8>,   // 256×256, per-tile: 0=shrouded, 255=explored
    fog_texture_data: Vec<u8>, // 256×256, composed for GPU upload
    fog_dirty: bool,
    fog_prev_tiles: Vec<(i32, i32)>, // per-pleb last known tile
    fog_vision_radius: i32,
    fog_start_explored: bool, // true = map starts pre-revealed
    // Fire system
    burn_progress: std::collections::HashMap<usize, f32>, // grid_idx → 0.0..1.0
    fire_intensity: f32, // sandbox ignite temperature multiplier (0.5 = smolder, 2.0 = inferno)
    // GPU dust simulation
    dust_injections: Vec<dust::DustInjection>,
    dust_storm_active: bool,
}

const LIGHTMAP_SCALE: u32 = 2; // lightmap texels per grid cell (2x resolution)
const LIGHTMAP_W: u32 = GRID_W * LIGHTMAP_SCALE;
const LIGHTMAP_H: u32 = GRID_H * LIGHTMAP_SCALE;
#[cfg(not(target_arch = "wasm32"))]
const LIGHTMAP_PROP_ITERATIONS: u32 = 26; // covers ~13 tile radius
#[cfg(target_arch = "wasm32")]
const LIGHTMAP_PROP_ITERATIONS: u32 = 12; // reduced for browser perf
#[cfg(not(target_arch = "wasm32"))]
const LIGHTMAP_UPDATE_INTERVAL: u32 = 2; // recompute every N frames (~30fps lightmap at 60fps)
#[cfg(target_arch = "wasm32")]
const LIGHTMAP_UPDATE_INTERVAL: u32 = 4; // less frequent for browser perf
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
    sprite_buffer: wgpu::Buffer, // combined: trees + bushes
    material_buffer: wgpu::Buffer,
    pleb_buffer: wgpu::Buffer,
    block_temp_buffer: wgpu::Buffer,
    thermal_pipeline: wgpu::ComputePipeline,
    thermal_bind_group: wgpu::BindGroup,
    shadow_map_dummy: wgpu::Texture, // 1x1 dummy for binding 18 (shadow map removed)
    // Sound wave propagation
    sound_textures: [wgpu::Texture; 2], // Rg32Float ping-pong (R=pressure, G=velocity)
    sound_pipeline: wgpu::ComputePipeline,
    sound_bind_groups: [wgpu::BindGroup; 2], // ping-pong
    sound_source_buffer: wgpu::Buffer,
    elevation_buffer: wgpu::Buffer,
    terrain_buffer: wgpu::Buffer,
    wall_buffer: wgpu::Buffer, // u16 per tile: wall edges, thickness, material (DN-008)
    door_buffer: wgpu::Buffer, // physical door data for raytrace (binding 25)
    creature_buffer: wgpu::Buffer, // alien fauna data (binding 26)
    // Power grid
    voltage_buffer: wgpu::Buffer,
    power_pipeline: wgpu::ComputePipeline,
    power_bind_group: wgpu::BindGroup,
    // Ground water simulation
    water_textures: [wgpu::Texture; 2],
    water_table_buffer: wgpu::Buffer,
    water_readback_buffer: wgpu::Buffer, // single-texel readback for info overlay
    water_pipeline: wgpu::ComputePipeline,
    water_bind_groups: [wgpu::BindGroup; 2],
    // Fluid simulation GPU resources
    fluid_params_buffer: wgpu::Buffer,
    fluid_vel: [wgpu::Texture; 2],
    fluid_pres: [wgpu::Texture; 2],
    #[allow(dead_code)]
    fluid_div: wgpu::Texture,
    #[allow(dead_code)]
    fluid_curl: wgpu::Texture,
    fluid_dye: [wgpu::Texture; 2],
    fluid_obstacle: wgpu::Texture,
    #[allow(dead_code)]
    fluid_dummy_rg: wgpu::Texture,
    fluid_dummy_r: wgpu::Texture,   // 1x1 R32Float dummy (read)
    fluid_dummy_r_w: wgpu::Texture, // 1x1 R32Float dummy (write, separate to avoid read-write conflict)
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
    fluid_bg_pressure: [wgpu::BindGroup; 2],   // ping-pong
    fluid_bg_pressure_clear: wgpu::BindGroup,  // A→B clear
    fluid_bg_advect_dye: [wgpu::BindGroup; 2], // ping-pong dye
    // Debug readback
    debug_readback_buffer: wgpu::Buffer, // staging buffer for single texel readback
    block_temp_readback_buffer: wgpu::Buffer, // staging buffer for block temp readback
    voltage_readback_buffer: wgpu::Buffer, // full grid voltage readback for per-tile labels
    pipe_flow_buffer: wgpu::Buffer,      // per-tile flow direction (2 f32 per tile: flow_x, flow_y)
    // Pleb air readback — one texel per pleb, each at 256-byte aligned offset
    pleb_air_readback_buffer: wgpu::Buffer,
    // Fog of war
    fog_texture: wgpu::Texture,
    // Dust simulation
    dust_textures: [wgpu::Texture; 2], // R16Float 256x256 ping-pong
    dust_params_buffer: wgpu::Buffer,
    dust_pipeline: wgpu::ComputePipeline,
    dust_bind_group: wgpu::BindGroup,
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
                oblique_strength: 0.0,
                lm_vp_min_x: 0.0,
                lm_vp_min_y: 0.0,
                lm_vp_max_x: GRID_W as f32,
                lm_vp_max_y: GRID_H as f32,
                lm_scale: LIGHTMAP_SCALE as f32,
                fluid_overlay: 0.0,
                sun_dir_x: 0.0,
                sun_dir_y: 0.0,
                sun_elevation: 0.0,
                sun_intensity: 0.0,
                sun_color_r: 0.0,
                sun_color_g: 0.0,
                sun_color_b: 0.0,
                ambient_r: 0.0,
                ambient_g: 0.0,
                ambient_b: 0.0,
                enable_prox_glow: 1.0,
                enable_dir_bleed: 1.0,
                force_refresh: 1.0,
                pleb_x: 0.0,
                pleb_y: 0.0,
                pleb_angle: 0.0,
                pleb_selected: 0.0,
                pleb_torch: 0.0,
                pleb_headlight: 0.0,
                prev_center_x: 0.0,
                prev_center_y: 0.0,
                prev_zoom: 0.0,
                prev_time: 0.0,
                rain_intensity: 0.0,
                cloud_cover: 0.0,
                wind_magnitude: 0.0,
                wind_angle: 0.0,
                use_shadow_map: 0.0,
                shadow_map_scale: 0.0,
                sound_speed: 0.0,
                sound_damping: 0.0,
                sound_coupling: 0.0,
                enable_terrain_detail: 1.0,
                terrain_ao_strength: 2.5,
                fog_enabled: 0.0,
                hover_x: -1.0,
                hover_y: -1.0,
                shadow_intensity: 1.0,
                pleb_scale: 1.75,
                contour_opacity: 1.0,
                contour_interval: 1.0,
                contour_major_mul: 3.0,
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
            time_of_day: DAY_DURATION * (CAMERA_START_HOUR / 24.0),
            time_paused: false,
            show_pause_menu: false,
            time_speed: 0.25,
            last_frame_time: Instant::now(),
            last_click_frame: 0,
            last_click_pos: (-1, -1),
            frame_count: 0,
            fps_accum: 0.0,
            fps_display: 0.0,
            lightmap_frame: 0,
            build_tool: BuildTool::None,
            build_rotation: 0,
            wall_thickness: 1, // thin walls by default
            hover_world: (0.0, 0.0),
            move_marker: None,
            terrain_tool: None,
            terrain_brush_radius: 2,
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
            liquid_network: PipeNetwork::new(),
            fluid_speed: 1.0,
            enable_terrain_detail: true,
            terrain_ao_strength: 2.5,
            show_contours: false,
            contour_opacity: 1.0,
            contour_interval: 1.0,
            contour_major_mul: 3.0,
            enable_prox_glow: !cfg!(target_arch = "wasm32"),
            enable_dir_bleed: true,
            enable_temporal: true,
            enable_ricochets: false,
            hires_fluid: false,
            fluid_pressure_iters: FLUID_PRESSURE_ITERS,
            lightmap_interval: LIGHTMAP_UPDATE_INTERVAL,
            lightmap_iterations: LIGHTMAP_PROP_ITERATIONS,
            sound_enabled: true,
            sound_phase: 0,
            sound_sources: Vec::new(),
            sound_speed: 0.3,
            sound_damping: 0.005,
            sound_coupling: 0.15,
            sound_iters_per_frame: if cfg!(target_arch = "wasm32") { 0 } else { 4 },
            camera_pan_speed: 400.0,
            dye_w: FLUID_DYE_W,
            dye_h: FLUID_DYE_H,
            sandbox_mode: false,
            debug_creatures_always: false,
            debug_bullet_slowmo: false,
            debug_bullet_speed: 0.02,
            debug_show_cover: false,
            debug_show_flock: false,
            audio_output: None, // lazy init on first sound (browser requires user gesture)
            sandbox_tool: SandboxTool::None,
            drag_start: None,
            show_pipe_overlay: false,
            show_grid: false,
            wall_data: Vec::new(),
            show_subgrid: false,
            show_liquid_overlay: false,
            show_flow_overlay: false,
            show_velocity_arrows: false,
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
            middle_mouse_pressed: false,
            plebs: {
                // Start with one colonist at map center
                let cx = (GRID_W / 2) as f32 + 0.5;
                let cy = (GRID_H / 2) as f32 + 0.5;
                let mut jeff = Pleb::new(0, "Jeff".to_string(), cx, cy, 42);
                jeff.headlight_on = true;
                vec![jeff]
            },
            selected_pleb: Some(0),
            selected_group: Vec::new(),
            next_pleb_id: 1,
            placing_pleb: false,
            placing_enemy: false,
            creatures: Vec::new(),
            creature_spawn_timer: 0.0,
            next_pack_id: 0,
            blood_stains: Vec::new(),
            cannon_angles: std::collections::HashMap::new(),
            show_pleb_help: false,
            show_inventory: false,
            inv_selected_slot: None,
            show_schedule: false,
            show_priorities: false,
            pressed_keys: std::collections::HashSet::new(),
            doors: Vec::new(),
            doors_dirty: false,
            roof_paid: vec![false; (GRID_W * GRID_H) as usize],
            roof_flash: vec![0.0; (GRID_W * GRID_H) as usize],
            physics_bodies: Vec::new(),
            ground_items: {
                // Scatter starting resources near spawn for first tools
                let cx = GRID_W as f32 / 2.0;
                let cy = GRID_H as f32 / 2.0;
                vec![
                    resources::GroundItem::new(cx - 1.3, cy + 0.8, ITEM_SCRAP_WOOD, 3),
                    resources::GroundItem::new(cx + 1.5, cy - 0.5, ITEM_SCRAP_WOOD, 2),
                    resources::GroundItem::new(cx + 0.7, cy + 1.4, ITEM_ROCK, 3),
                    resources::GroundItem::new(cx - 0.9, cy - 1.2, ITEM_ROCK, 2),
                ]
            },
            blueprints: std::collections::HashMap::new(),
            game_hints: Vec::new(),
            active_hint_idx: None,
            pleb_air_data: Vec::new(),
            pleb_air_readback_pending: false,
            context_menu: None, // unified context menu
            world_sel: WorldSelection::none(),
            game_log: std::collections::VecDeque::new(),
            notifications: Vec::new(),
            conditions: Vec::new(),
            next_notif_id: 0,
            drought_check_timer: 30.0,
            select_drag_start: None,
            crate_contents: std::collections::HashMap::new(),
            craft_queues: std::collections::HashMap::new(),
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
            zones: Vec::new(),
            active_work: std::collections::HashSet::new(),
            manual_tasks: Vec::new(),
            work_priority: zones::WorkPriority::PlantFirst,
            crop_timers: std::collections::HashMap::new(),
            water_phase: 0,
            water_frame: 0,
            water_table: Vec::new(),
            elevation_data: Vec::new(), // populated after grid gen in init_gfx_async
            terrain_data: Vec::new(),   // populated after grid gen in init_gfx_async
            terrain_dirty: false,
            terrain_params: grid::TerrainParams::default(),
            game_state: GameState::MainMenu,
            chargen_name: "Jeff".to_string(),
            chargen_skin: [0.76, 0.60, 0.46],
            chargen_hair: [0.25, 0.15, 0.08],
            chargen_hair_style: 0,
            chargen_shirt: [0.35, 0.25, 0.20],
            chargen_pants: [0.30, 0.28, 0.22],
            chargen_backstory: Backstory::Sheriff,
            chargen_body_type: BodyType::Medium,
            chargen_gender: Gender::Male,
            chargen_age: 32,
            chargen_trait: None,
            chargen_preview_angle: 0.0,
            diag_preview: Vec::new(),
            drag_entryway: None,
            entry_side: 0,
            campfire_subtile: None,
            menu_hover_id: None,
            voltage_data: Vec::new(),
            voltage_readback_pending: false,
            fog_enabled: true,
            fog_visibility: vec![0u8; (GRID_W * GRID_H) as usize],
            fog_explored: vec![0u8; (GRID_W * GRID_H) as usize], // start unexplored (black)
            fog_texture_data: vec![0u8; (GRID_W * GRID_H) as usize],
            fog_dirty: true,
            fog_prev_tiles: Vec::new(),
            fog_vision_radius: 25,
            fog_start_explored: false, // true = map starts pre-revealed
            burn_progress: std::collections::HashMap::new(),
            fire_intensity: 1.0,
            dust_injections: Vec::new(),
            dust_storm_active: false,
        }
    }

    /// Inject a voltage surge into conductors near (cx, cy) and trip nearby breakers.
    fn lightning_surge(&mut self, cx: i32, cy: i32) {
        let gfx = match &self.gfx {
            Some(g) => g,
            None => return,
        };
        let mut surge_count = 0u32;
        for dy in -LIGHTNING_SURGE_RADIUS..=LIGHTNING_SURGE_RADIUS {
            for dx in -LIGHTNING_SURGE_RADIUS..=LIGHTNING_SURGE_RADIUS {
                let nx = cx + dx;
                let ny = cy + dy;
                if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 {
                    continue;
                }
                let dist_sq = (dx * dx + dy * dy) as f32;
                if dist_sq > (LIGHTNING_SURGE_RADIUS * LIGHTNING_SURGE_RADIUS) as f32 {
                    continue;
                }
                let nidx = (ny as u32 * GRID_W + nx as u32) as usize;
                let bt = block_type_rs(self.grid_data[nidx]);
                let flags = block_flags_rs(self.grid_data[nidx]);
                if is_conductor_rs(bt, flags) {
                    let dist = dist_sq.sqrt();
                    let surge = LIGHTNING_SURGE_VOLTAGE
                        * (1.0 - dist / LIGHTNING_SURGE_RADIUS as f32).max(0.0);
                    gfx.queue.write_buffer(
                        &gfx.voltage_buffer,
                        (nidx as u64) * 4,
                        bytemuck::bytes_of(&surge),
                    );
                    surge_count += 1;
                }
            }
        }
        log::warn!(
            "LIGHTNING SURGE: center=({},{}) hit {} conductors, max={}V",
            cx,
            cy,
            surge_count,
            LIGHTNING_SURGE_VOLTAGE
        );
        // Trip breakers in nearby area
        for dy in -LIGHTNING_BREAKER_RADIUS..=LIGHTNING_BREAKER_RADIUS {
            for dx in -LIGHTNING_BREAKER_RADIUS..=LIGHTNING_BREAKER_RADIUS {
                let bnx = cx + dx;
                let bny = cy + dy;
                if bnx < 0 || bny < 0 || bnx >= GRID_W as i32 || bny >= GRID_H as i32 {
                    continue;
                }
                let bnidx = (bny as u32 * GRID_W + bnx as u32) as usize;
                let cb = self.grid_data[bnidx];
                if (cb & 0xFF) == BT_BREAKER && ((cb >> 16) & 4) != 0 {
                    self.grid_data[bnidx] = cb & !(4u32 << 16);
                    self.grid_dirty = true;
                }
            }
        }
    }

    /// Convert world block coordinates to window screen pixels
    #[allow(dead_code)]
    fn world_to_screen(&self, wx: f32, wy: f32) -> (f32, f32) {
        let rx = (wx - self.camera.center_x) * self.camera.zoom + self.camera.screen_w * 0.5;
        let ry = (wy - self.camera.center_y) * self.camera.zoom + self.camera.screen_h * 0.5;
        (rx / self.render_scale, ry / self.render_scale)
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

        let output_view_a =
            gfx.output_textures[0].create_view(&wgpu::TextureViewDescriptor::default());
        let output_view_b =
            gfx.output_textures[1].create_view(&wgpu::TextureViewDescriptor::default());

        // Recreate dye textures at render resolution (square — world is square)
        let dye_size = render_w.max(render_h);
        self.dye_w = dye_size;
        self.dye_h = dye_size;
        let dye_desc = wgpu::TextureDescriptor {
            label: Some("fluid-dye-a"),
            size: wgpu::Extent3d {
                width: dye_size,
                height: dye_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        };
        gfx.fluid_dye[0] = gfx.device.create_texture(&dye_desc);
        gfx.fluid_dye[1] = gfx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fluid-dye-b"),
            ..dye_desc
        });

        // Rebuild bind groups with new texture view
        let lightmap_sample_view =
            gfx.lightmap_textures[0].create_view(&wgpu::TextureViewDescriptor::default());
        let lightmap_sampler = gfx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("lightmap-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let compute_bgl = gfx.compute_pipeline.get_bind_group_layout(0);
        let fog_sampler = gfx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("fog-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
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
        let water_view = gfx.water_textures[0].create_view(&wgpu::TextureViewDescriptor::default());
        let dust_b_view = gfx.dust_textures[1].create_view(&wgpu::TextureViewDescriptor::default());
        let shadow_map_view = gfx
            .shadow_map_dummy
            .create_view(&wgpu::TextureViewDescriptor::default());
        let sound_view = gfx.sound_textures[0].create_view(&wgpu::TextureViewDescriptor::default());
        gfx.compute_bind_groups = [
            gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("compute-bg-0"), // dye_A, write output_A, read prev output_B
                layout: &compute_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&output_view_a),
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
                    wgpu::BindGroupEntry {
                        binding: 11,
                        resource: gfx.material_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 12,
                        resource: gfx.pleb_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 13,
                        resource: gfx.block_temp_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 14,
                        resource: gfx.voltage_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 15,
                        resource: gfx.pipe_flow_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 16,
                        resource: wgpu::BindingResource::TextureView(&water_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 17,
                        resource: gfx.water_table_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 18,
                        resource: wgpu::BindingResource::TextureView(&shadow_map_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 19,
                        resource: wgpu::BindingResource::TextureView(&sound_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 20,
                        resource: gfx.elevation_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 21,
                        resource: wgpu::BindingResource::TextureView(
                            &gfx.fog_texture
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 22,
                        resource: wgpu::BindingResource::Sampler(&fog_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 23,
                        resource: gfx.terrain_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 24,
                        resource: gfx.wall_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 25,
                        resource: gfx.door_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 26,
                        resource: gfx.creature_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(&fv_dye_a),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: wgpu::BindingResource::Sampler(&fluid_dye_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 8,
                        resource: wgpu::BindingResource::TextureView(&fv_vel_a_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 9,
                        resource: wgpu::BindingResource::TextureView(&fv_pres_b_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 10,
                        resource: wgpu::BindingResource::TextureView(&output_view_b),
                    },
                    wgpu::BindGroupEntry {
                        binding: 27,
                        resource: wgpu::BindingResource::TextureView(&dust_b_view),
                    },
                ],
            }),
            gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("compute-bg-1"), // dye_A, write output_B, read prev output_A
                layout: &compute_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&output_view_b),
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
                    wgpu::BindGroupEntry {
                        binding: 11,
                        resource: gfx.material_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 12,
                        resource: gfx.pleb_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 13,
                        resource: gfx.block_temp_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 14,
                        resource: gfx.voltage_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 15,
                        resource: gfx.pipe_flow_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 16,
                        resource: wgpu::BindingResource::TextureView(&water_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 17,
                        resource: gfx.water_table_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 18,
                        resource: wgpu::BindingResource::TextureView(&shadow_map_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 19,
                        resource: wgpu::BindingResource::TextureView(&sound_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 20,
                        resource: gfx.elevation_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 21,
                        resource: wgpu::BindingResource::TextureView(
                            &gfx.fog_texture
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 22,
                        resource: wgpu::BindingResource::Sampler(&fog_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 23,
                        resource: gfx.terrain_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 24,
                        resource: gfx.wall_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 25,
                        resource: gfx.door_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 26,
                        resource: gfx.creature_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(&fv_dye_a),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: wgpu::BindingResource::Sampler(&fluid_dye_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 8,
                        resource: wgpu::BindingResource::TextureView(&fv_vel_a_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 9,
                        resource: wgpu::BindingResource::TextureView(&fv_pres_b_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 10,
                        resource: wgpu::BindingResource::TextureView(&output_view_a),
                    },
                    wgpu::BindGroupEntry {
                        binding: 27,
                        resource: wgpu::BindingResource::TextureView(&dust_b_view),
                    },
                ],
            }),
            gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("compute-bg-2"), // dye_B, write output_A, read prev output_B
                layout: &compute_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&output_view_a),
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
                    wgpu::BindGroupEntry {
                        binding: 11,
                        resource: gfx.material_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 12,
                        resource: gfx.pleb_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 13,
                        resource: gfx.block_temp_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 14,
                        resource: gfx.voltage_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 15,
                        resource: gfx.pipe_flow_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 16,
                        resource: wgpu::BindingResource::TextureView(&water_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 17,
                        resource: gfx.water_table_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 18,
                        resource: wgpu::BindingResource::TextureView(&shadow_map_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 19,
                        resource: wgpu::BindingResource::TextureView(&sound_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 20,
                        resource: gfx.elevation_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 21,
                        resource: wgpu::BindingResource::TextureView(
                            &gfx.fog_texture
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 22,
                        resource: wgpu::BindingResource::Sampler(&fog_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 23,
                        resource: gfx.terrain_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 24,
                        resource: gfx.wall_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 25,
                        resource: gfx.door_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 26,
                        resource: gfx.creature_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(&fv_dye_b),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: wgpu::BindingResource::Sampler(&fluid_dye_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 8,
                        resource: wgpu::BindingResource::TextureView(&fv_vel_a_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 9,
                        resource: wgpu::BindingResource::TextureView(&fv_pres_b_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 10,
                        resource: wgpu::BindingResource::TextureView(&output_view_b),
                    },
                    wgpu::BindGroupEntry {
                        binding: 27,
                        resource: wgpu::BindingResource::TextureView(&dust_b_view),
                    },
                ],
            }),
            gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("compute-bg-3"), // dye_B, write output_B, read prev output_A
                layout: &compute_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&output_view_b),
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
                    wgpu::BindGroupEntry {
                        binding: 11,
                        resource: gfx.material_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 12,
                        resource: gfx.pleb_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 13,
                        resource: gfx.block_temp_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 14,
                        resource: gfx.voltage_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 15,
                        resource: gfx.pipe_flow_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 16,
                        resource: wgpu::BindingResource::TextureView(&water_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 17,
                        resource: gfx.water_table_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 18,
                        resource: wgpu::BindingResource::TextureView(&shadow_map_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 19,
                        resource: wgpu::BindingResource::TextureView(&sound_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 20,
                        resource: gfx.elevation_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 21,
                        resource: wgpu::BindingResource::TextureView(
                            &gfx.fog_texture
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 22,
                        resource: wgpu::BindingResource::Sampler(&fog_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 23,
                        resource: gfx.terrain_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 24,
                        resource: gfx.wall_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 25,
                        resource: gfx.door_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 26,
                        resource: gfx.creature_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(&fv_dye_b),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: wgpu::BindingResource::Sampler(&fluid_dye_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 8,
                        resource: wgpu::BindingResource::TextureView(&fv_vel_a_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 9,
                        resource: wgpu::BindingResource::TextureView(&fv_pres_b_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 10,
                        resource: wgpu::BindingResource::TextureView(&output_view_a),
                    },
                    wgpu::BindGroupEntry {
                        binding: 27,
                        resource: wgpu::BindingResource::TextureView(&dust_b_view),
                    },
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

        // Recreate fluid dye bind groups (advect_dye + splat reference dye textures)
        let fluid_dye_bgl = gfx.fluid_p_advect_dye.get_bind_group_layout(0);
        let fv_vel_a_fluid = gfx.fluid_vel[0].create_view(&wgpu::TextureViewDescriptor::default());
        let fv_obstacle_view = gfx
            .fluid_obstacle
            .create_view(&wgpu::TextureViewDescriptor::default());
        gfx.fluid_bg_advect_dye = [
            gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("fluid-bg-advect-dye-0"),
                layout: &fluid_dye_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&fv_dye_a),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&fv_dye_b),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&fv_vel_a_fluid),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: gfx.fluid_params_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: gfx.grid_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(&fv_obstacle_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: gfx.block_temp_buffer.as_entire_binding(),
                    },
                ],
            }),
            gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("fluid-bg-advect-dye-1"),
                layout: &fluid_dye_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&fv_dye_b),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&fv_dye_a),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&fv_vel_a_fluid),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: gfx.fluid_params_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: gfx.grid_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(&fv_obstacle_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: gfx.block_temp_buffer.as_entire_binding(),
                    },
                ],
            }),
        ];
        // Splat uses dye_a at binding 7 — recreate with new dye texture
        let fluid_sim_bgl = gfx.fluid_p_splat.get_bind_group_layout(0);
        let fv_vel_b = gfx.fluid_vel[1].create_view(&wgpu::TextureViewDescriptor::default());
        let fv_dummy_r = gfx
            .fluid_dummy_r
            .create_view(&wgpu::TextureViewDescriptor::default());
        let fv_dummy_r_w = gfx
            .fluid_dummy_r_w
            .create_view(&wgpu::TextureViewDescriptor::default());
        gfx.fluid_bg_splat = gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fluid-bg-splat"),
            layout: &fluid_sim_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&fv_vel_b),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&fv_vel_a_fluid),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&fv_dummy_r),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&fv_dummy_r_w),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&fv_obstacle_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: gfx.fluid_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: gfx.grid_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(&fv_dye_a),
                },
            ],
        });
    }

    fn render(&mut self) {
        // Check if render scale changed — trigger resize to recreate output texture
        {
            let gfx = self.gfx.as_ref().unwrap();
            let expected_w = ((gfx.config.width as f32) * self.render_scale).max(1.0) as u32;
            let expected_h = ((gfx.config.height as f32) * self.render_scale).max(1.0) as u32;
            if expected_w != self.camera.screen_w as u32
                || expected_h != self.camera.screen_h as u32
            {
                let size = PhysicalSize::new(gfx.config.width, gfx.config.height);
                let _ = gfx;
                self.resize(size);
            }
        }

        let dt = self.update_simulation();

        // Play audio for fresh sound sources (lazy init: browser requires user gesture)
        if self.audio_output.is_none() {
            self.audio_output = audio::AudioOutput::new();
        }
        if let Some(ref audio) = self.audio_output {
            if self.sound_enabled {
                for src in &mut self.sound_sources {
                    if src.fresh {
                        audio.play(
                            src.x,
                            src.y,
                            src.amplitude,
                            src.frequency,
                            src.pattern,
                            src.duration,
                            self.camera.center_x,
                            self.camera.center_y,
                        );
                        src.fresh = false;
                    }
                }
            }
        }

        // --- Move marker fade ---
        if let Some((_, _, ref mut timer)) = self.move_marker {
            *timer -= dt;
            if *timer <= 0.0 {
                self.move_marker = None;
            }
        }

        // --- Terrain tool: clear if another build tool selected ---
        if self.build_tool != BuildTool::None {
            self.terrain_tool = None;
        }

        // --- Terrain tool application (while mouse held) ---
        if self.mouse_pressed
            && self.terrain_tool.is_some()
            && self.game_state == GameState::Playing
        {
            let tool = self.terrain_tool.unwrap();
            let (hx, hy) = self.hover_world;
            let cx = hx.floor() as i32;
            let cy = hy.floor() as i32;
            let r = self.terrain_brush_radius as i32;
            let sigma = r as f32 / 2.5;
            let shift_held = self.pressed_keys.contains(&KeyCode::ShiftLeft)
                || self.pressed_keys.contains(&KeyCode::ShiftRight);
            let strength = 2.0 * dt;

            // For flatten: compute weighted average first
            let flatten_target = if tool == TerrainTool::Flatten {
                let mut sum = 0.0f32;
                let mut wsum = 0.0f32;
                for dy in -r..=r {
                    for dx in -r..=r {
                        let w = (-(dx * dx + dy * dy) as f32 / (2.0 * sigma * sigma)).exp();
                        if w < 0.05 {
                            continue;
                        }
                        let tx = cx + dx;
                        let ty = cy + dy;
                        if tx >= 0 && ty >= 0 && tx < GRID_W as i32 && ty < GRID_H as i32 {
                            let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                            if idx < self.elevation_data.len() {
                                sum += self.elevation_data[idx] * w;
                                wsum += w;
                            }
                        }
                    }
                }
                if wsum > 0.0 { sum / wsum } else { 0.0 }
            } else {
                0.0
            };

            let mut changed = false;
            for dy in -r..=r {
                for dx in -r..=r {
                    let w = (-(dx * dx + dy * dy) as f32 / (2.0 * sigma * sigma)).exp();
                    if w < 0.05 {
                        continue;
                    }
                    let tx = cx + dx;
                    let ty = cy + dy;
                    if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 {
                        continue;
                    }
                    let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                    if idx >= self.elevation_data.len() {
                        continue;
                    }
                    // Skip tiles with buildings
                    let bt = block_type_rs(self.grid_data[idx]);
                    if bt != BT_DIRT && bt != BT_AIR && bt != BT_DUG_GROUND {
                        continue;
                    }

                    let delta = match tool {
                        TerrainTool::Raise => {
                            let dir = if shift_held { -1.0 } else { 1.0 };
                            w * strength * dir
                        }
                        TerrainTool::Lower => {
                            let dir = if shift_held { 1.0 } else { -1.0 };
                            w * strength * dir
                        }
                        TerrainTool::Flatten => {
                            (flatten_target - self.elevation_data[idx]) * w * 3.0 * dt
                        }
                        TerrainTool::Smooth => {
                            let cur = self.elevation_data[idx];
                            let mut avg = 0.0f32;
                            let mut count = 0.0f32;
                            for &(ndx, ndy) in &[(0i32, -1i32), (0, 1), (-1, 0), (1, 0)] {
                                let nx = tx + ndx;
                                let ny = ty + ndy;
                                if nx >= 0 && ny >= 0 && nx < GRID_W as i32 && ny < GRID_H as i32 {
                                    let ni = (ny as u32 * GRID_W + nx as u32) as usize;
                                    if ni < self.elevation_data.len() {
                                        avg += self.elevation_data[ni];
                                        count += 1.0;
                                    }
                                }
                            }
                            if count > 0.0 {
                                avg /= count;
                                (avg - cur) * w * 2.0 * dt
                            } else {
                                0.0
                            }
                        }
                    };
                    self.elevation_data[idx] = (self.elevation_data[idx] + delta).clamp(0.0, 8.0);
                    changed = true;
                }
            }

            if changed {
                // Re-upload elevation + AO to GPU
                if let Some(gfx) = &self.gfx {
                    let grid_size = (GRID_W * GRID_H) as usize;
                    let ao = grid::compute_terrain_ao(&self.elevation_data);
                    let mut elev_ao = vec![0.0f32; grid_size * 2];
                    for i in 0..grid_size {
                        elev_ao[i * 2] = self.elevation_data[i];
                        elev_ao[i * 2 + 1] = ao[i];
                    }
                    gfx.queue.write_buffer(
                        &gfx.elevation_buffer,
                        0,
                        bytemuck::cast_slice(&elev_ao),
                    );
                    // Adjust water table for new elevation
                    grid::adjust_water_table_for_elevation(
                        &mut self.water_table,
                        &self.elevation_data,
                    );
                    gfx.queue.write_buffer(
                        &gfx.water_table_buffer,
                        0,
                        bytemuck::cast_slice(&self.water_table),
                    );
                }
            }
        }

        // Generate hints every 30 frames
        if self.frame_count.is_multiple_of(30) {
            self.generate_hints();
        }

        // WASD camera pan (always active)
        {
            let shift = self.pressed_keys.contains(&KeyCode::ShiftLeft)
                || self.pressed_keys.contains(&KeyCode::ShiftRight);
            let pan_speed =
                self.camera_pan_speed / self.camera.zoom * if shift { 2.0 } else { 1.0 };
            let mut pan_x = 0.0f32;
            let mut pan_y = 0.0f32;
            if self.pressed_keys.contains(&KeyCode::KeyW)
                || self.pressed_keys.contains(&KeyCode::ArrowUp)
            {
                pan_y -= 1.0;
            }
            if self.pressed_keys.contains(&KeyCode::KeyS)
                || self.pressed_keys.contains(&KeyCode::ArrowDown)
            {
                pan_y += 1.0;
            }
            if self.pressed_keys.contains(&KeyCode::KeyA)
                || self.pressed_keys.contains(&KeyCode::ArrowLeft)
            {
                pan_x -= 1.0;
            }
            if self.pressed_keys.contains(&KeyCode::KeyD)
                || self.pressed_keys.contains(&KeyCode::ArrowRight)
            {
                pan_x += 1.0;
            }
            if pan_x != 0.0 || pan_y != 0.0 {
                let len = (pan_x * pan_x + pan_y * pan_y).sqrt();
                self.camera.center_x += pan_x / len * pan_speed * dt;
                self.camera.center_y += pan_y / len * pan_speed * dt;
                self.camera.center_x = self.camera.center_x.clamp(0.0, GRID_W as f32);
                self.camera.center_y = self.camera.center_y.clamp(0.0, GRID_H as f32);
            }
        }

        // Lightning voltage surge: handle natural lightning (heavy rain) via deferred injection
        // (must run before gfx borrow since it needs &mut self)
        if let Some((lx, ly)) = self.lightning_strike
            && !self.lightning_surge_done
        {
            self.lightning_surge_done = true;
            self.lightning_surge(lx.floor() as i32, ly.floor() as i32);
        }

        // --- Fire system tick (before gfx borrow so we can mutate self) ---
        let fire_temp_overrides = if !self.burn_progress.is_empty() {
            let (temps, destroyed) = fire::tick_fire(
                &self.grid_data,
                &self.wall_data,
                &mut self.burn_progress,
                dt,
                self.time_speed,
                self.frame_count,
                self.camera.rain_intensity,
                self.camera.wind_angle,
                self.camera.wind_magnitude,
                &self.wetness_data,
                self.fire_intensity,
            );
            for &idx in &destroyed {
                let bx = (idx % GRID_W as usize) as i32;
                let by = (idx / GRID_W as usize) as i32;
                let bt = block_type_rs(self.grid_data[idx]);
                if bt == BT_DIRT {
                    // Grass burned away — scorch the dirt (flags bit 3), don't destroy
                    // Bit 3 chosen to avoid conflict with bit 0 (door flag)
                    let flags = ((self.grid_data[idx] >> 16) & 0xFF) as u8;
                    let roof_h = self.grid_data[idx] & 0xFF000000;
                    self.grid_data[idx] = make_block(BT_DIRT as u8, 0, flags | 8) | roof_h;
                } else {
                    let replacement = fire::burn_replacement_pub(bt);
                    let roof_h = self.grid_data[idx] & 0xFF000000;
                    self.grid_data[idx] = make_block(replacement as u8, 0, 0)
                        | (if replacement == BT_AIR { 0 } else { roof_h });
                }
                self.grid_dirty = true;
                let evt = GameEventKind::FireConsumed(bx, by);
                self.log_event(evt.category(), evt.message());
            }
            if !destroyed.is_empty() {
                compute_roof_heights_wd(&mut self.grid_data, &self.wall_data);
            }
            temps
        } else {
            Vec::new()
        };

        let gfx = self.gfx.as_ref().unwrap();

        // Write fire temperature overrides to GPU
        for &(idx, temp) in &fire_temp_overrides {
            gfx.queue.write_buffer(
                &gfx.block_temp_buffer,
                (idx as u64) * 4,
                bytemuck::bytes_of(&temp),
            );
        }

        // Tick roof flash timers (runs every frame, not just grid_dirty)
        for f in self.roof_flash.iter_mut() {
            if *f > 0.0 {
                *f -= dt;
            }
        }

        // Re-upload grid if dirty (door toggled etc.)
        if self.grid_dirty {
            // Recompute roof heights (walls may have changed)
            compute_roof_heights_wd(&mut self.grid_data, &self.wall_data);
            // Roof fiber cost: suppress roof_h on unpaid tiles, create blueprints
            // Only create roof blueprints once ALL wall blueprints for that room are built.
            if !self.sandbox_mode {
                let grid_size = (GRID_W * GRID_H) as usize;
                let w = GRID_W as i32;
                let h = GRID_H as i32;
                let max_search = 20i32;
                for idx in 0..grid_size.min(self.grid_data.len()) {
                    let roof_h = (self.grid_data[idx] >> 24) & 0xFF;
                    if roof_h > 0 {
                        if self.roof_paid[idx] {
                            // Already paid, keep roof_h (shader handles visibility)
                        } else if !self.blueprints.contains_key(&(
                            (idx % GRID_W as usize) as i32,
                            (idx / GRID_W as usize) as i32,
                        )) {
                            // Check if all enclosing walls are built (no wall blueprints remain)
                            let bx = (idx % GRID_W as usize) as i32;
                            let by = (idx / GRID_W as usize) as i32;
                            let mut walls_complete = true;
                            let dirs: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
                            'dir_loop: for &(ddx, ddy) in &dirs {
                                for dist in 1..=max_search {
                                    let nx = bx + ddx * dist;
                                    let ny = by + ddy * dist;
                                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                                        break;
                                    }
                                    // Check if this tile has a wall blueprint (not yet built)
                                    if self
                                        .blueprints
                                        .get(&(nx, ny))
                                        .is_some_and(|bp| bp.is_wall())
                                    {
                                        walls_complete = false;
                                        break 'dir_loop;
                                    }
                                    let nidx = (ny as u32 * GRID_W + nx as u32) as usize;
                                    // Stop at actual walls (block or wall_data)
                                    let has_wd = nidx < self.wall_data.len()
                                        && wd_edges(self.wall_data[nidx]) != 0;
                                    let nb = self.grid_data[nidx];
                                    let nbh = ((nb >> 8) & 0xFF) as u8;
                                    let nb_flags = ((nb >> 16) & 0xFF) as u8;
                                    if has_wd && (nb_flags & 2) == 0 {
                                        break; // found built wall in this direction
                                    }
                                    if nbh > 0 && (nb_flags & 2) == 0 {
                                        break; // found built block wall
                                    }
                                }
                            }
                            if walls_complete {
                                self.blueprints.insert((bx, by), Blueprint::new_roof());
                            }
                            // Clear roof_h so shader sees no roof yet
                            self.grid_data[idx] &= 0x00FFFFFF;
                        } else {
                            // Blueprint exists but not yet built — keep roof_h suppressed
                            self.grid_data[idx] &= 0x00FFFFFF;
                        }
                    } else {
                        // No roof here — reset paid state if it was set (wall removed)
                        if self.roof_paid[idx] {
                            self.roof_paid[idx] = false;
                        }
                    }
                }
            }

            gfx.queue
                .write_buffer(&gfx.grid_buffer, 0, bytemuck::cast_slice(&self.grid_data));
            // Rebuild fluid obstacle field
            let obs_data = build_obstacle_field(&self.grid_data, &self.wall_data);
            gfx.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &gfx.fluid_obstacle,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &obs_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(GRID_W * 2),
                    rows_per_image: Some(GRID_H * 2),
                },
                wgpu::Extent3d {
                    width: GRID_W * 2,
                    height: GRID_H * 2,
                    depth_or_array_layers: 1,
                },
            );
            // Merge wall data: extract from grid (legacy walls) + keep existing wall_data edges
            let extracted = extract_wall_data_from_grid(&self.grid_data);
            for (cur, &ext) in self.wall_data.iter_mut().zip(extracted.iter()) {
                // OR the edges from both sources; keep thickness/material from whichever has edges
                let ext_edges = wd_edges(ext);
                let cur_edges = wd_edges(*cur);
                if ext_edges != 0 && cur_edges == 0 {
                    *cur = ext; // legacy wall, use extracted
                }
                // If cur_edges != 0, keep existing wall_data (placed via place_wall_edge)
                // If both have edges, keep wall_data (it's the authority)
            }
            gfx.queue
                .write_buffer(&gfx.wall_buffer, 0, bytemuck::cast_slice(&self.wall_data));
            self.grid_dirty = false;
            self.doors_dirty = true; // wall_data changed, refresh doors too
            self.pipe_network.rebuild(&self.grid_data);
            self.liquid_network
                .rebuild_with(&self.grid_data, pipes::is_liquid_pipe_component);
        }

        // Upload door angles (tiny buffer, independent of grid rebuild)
        if self.doors_dirty {
            let buf_len = MAX_DOORS * 2 + 1;
            let mut door_gpu = Vec::with_capacity(buf_len);
            door_gpu.push(self.doors.len() as u32);
            for door in &self.doors {
                let packed = door.pack_gpu();
                door_gpu.push(packed[0]);
                door_gpu.push(packed[1]);
            }
            door_gpu.resize(buf_len, 0);
            gfx.queue
                .write_buffer(&gfx.door_buffer, 0, bytemuck::cast_slice(&door_gpu));
            self.doors_dirty = false;
        }

        // Re-upload terrain data if dirty (compaction changed)
        if self.terrain_dirty {
            gfx.queue.write_buffer(
                &gfx.terrain_buffer,
                0,
                bytemuck::cast_slice(&self.terrain_data),
            );
            self.terrain_dirty = false;
        }

        // Upload fog texture when changed
        if self.fog_dirty {
            gfx.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &gfx.fog_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &self.fog_texture_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(GRID_W),
                    rows_per_image: Some(GRID_H),
                },
                wgpu::Extent3d {
                    width: GRID_W,
                    height: GRID_H,
                    depth_or_array_layers: 1,
                },
            );
            self.fog_dirty = false;
        }

        // Tick pipe network simulation — store outlet injections for post-shader application
        let pipe_injections = self.pipe_network.tick(dt, &self.grid_data, self.pipe_width);
        let liquid_injections = self
            .liquid_network
            .tick(dt, &self.grid_data, self.pipe_width);

        // Process liquid output: dump water onto ground surface + water table + wetness
        // Batch water texture writes: accumulate into staging buffer, write once
        let mut water_dirty_tiles: Vec<(u32, u32, f32)> = Vec::new();
        for &(lx, ly, _gas, pressure) in &liquid_injections {
            let cx = lx.floor() as i32;
            let cy = ly.floor() as i32;
            let spread = if pressure > 1.0 { 3 } else { 2 };
            for dy in -spread..=spread {
                for dx in -spread..=spread {
                    let nx = cx + dx;
                    let ny = cy + dy;
                    if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 {
                        continue;
                    }
                    let nidx = (ny as u32 * GRID_W + nx as u32) as usize;
                    let dist = ((dx * dx + dy * dy) as f32).sqrt();
                    let falloff = (1.0 - dist / (spread as f32 + 1.0)).max(0.0);
                    let amount = pressure.min(3.0) * falloff;
                    self.wetness_data[nidx] = (self.wetness_data[nidx] + amount * dt).min(1.0);
                    if nidx < self.water_table.len() {
                        self.water_table[nidx] =
                            (self.water_table[nidx] + amount * 0.3 * dt).min(0.5);
                    }
                    let water_level = (amount * 0.5 * dt).min(0.5);
                    water_dirty_tiles.push((nx as u32, ny as u32, water_level));
                }
            }
        }
        // Batch: find bounding rect and write one region
        if !water_dirty_tiles.is_empty() {
            let min_x = water_dirty_tiles.iter().map(|t| t.0).min().unwrap();
            let max_x = water_dirty_tiles.iter().map(|t| t.0).max().unwrap();
            let min_y = water_dirty_tiles.iter().map(|t| t.1).min().unwrap();
            let max_y = water_dirty_tiles.iter().map(|t| t.1).max().unwrap();
            let w = (max_x - min_x + 1) as usize;
            let h = (max_y - min_y + 1) as usize;
            let mut region = vec![0.0f32; w * h];
            for &(tx, ty, val) in &water_dirty_tiles {
                let rx = (tx - min_x) as usize;
                let ry = (ty - min_y) as usize;
                region[ry * w + rx] = region[ry * w + rx].max(val);
            }
            gfx.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &gfx.water_textures[self.water_phase],
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: min_x,
                        y: min_y,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                bytemuck::cast_slice(&region),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(w as u32 * 4),
                    rows_per_image: Some(h as u32),
                },
                wgpu::Extent3d {
                    width: w as u32,
                    height: h as u32,
                    depth_or_array_layers: 1,
                },
            );
        }

        // Write pipe flow directions to GPU buffer for shader animation (gas + liquid)
        // Batch into a single write_buffer call instead of per-cell writes
        {
            let grid_size = (GRID_W * GRID_H) as usize;
            let mut flow_buf = vec![[0.0f32; 2]; grid_size];
            for (&idx, cell) in self
                .pipe_network
                .cells
                .iter()
                .chain(self.liquid_network.cells.iter())
            {
                if (idx as usize) < grid_size {
                    flow_buf[idx as usize] = [cell.flow_x, cell.flow_y];
                }
            }
            gfx.queue
                .write_buffer(&gfx.pipe_flow_buffer, 0, bytemuck::cast_slice(&flow_buf));
        }

        // Upload fluid params (sim_w/h control effective resolution within max-size textures)
        let fluid_res = if self.hires_fluid {
            FLUID_SIM_MAX
        } else {
            FLUID_SIM_W
        };
        self.fluid_params.sim_w = fluid_res as f32;
        self.fluid_params.sim_h = fluid_res as f32;
        self.fluid_params.dye_w = self.dye_w as f32;
        self.fluid_params.dye_h = self.dye_h as f32;
        self.fluid_params.time = self.time_of_day;
        self.fluid_params.dt = (1.0 / 60.0) * self.fluid_speed;
        self.fluid_params.splat_active = 0.0; // splat disabled

        // Sound→Gas coupling: override splat with sound source velocity if no mouse active
        if self.sound_enabled
            && self.sound_coupling > 0.001
            && !self.sound_sources.is_empty()
            && let Some(src) = self.sound_sources.iter().max_by(|a, b| {
                a.amplitude
                    .abs()
                    .partial_cmp(&b.amplitude.abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        {
            let coupling = self.sound_coupling;
            let fx = src.x / GRID_W as f32 * fluid_res as f32;
            let fy = src.y / GRID_H as f32 * fluid_res as f32;
            let strength = src.amplitude * coupling * 60.0;

            // Use divergent splat: four cardinal splats creating outward expansion
            // The splat shader only supports one point, so we cycle through 4 offset
            // positions around the source, each pushing outward. Over 4 frames this
            // creates a symmetric expansion.
            let frame_dir = (self.frame_count % 4) as usize;
            let dirs: [(f32, f32); 4] = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)];
            let (dx, dy) = dirs[frame_dir];
            let offset = 1.5; // offset from source center

            let push = match src.pattern {
                0 => strength * 30.0, // impulse: strong constant push
                1 => {
                    // Bell: slow oscillation (use phase / frequency to get slow cycle)
                    let slow_phase = src.phase / src.frequency.max(1.0_f32);
                    slow_phase.sin() * strength * 15.0
                }
                _ => strength * 10.0,
            };

            if push.abs() > 0.1 {
                self.fluid_params.splat_x = fx + dx * offset;
                self.fluid_params.splat_y = fy + dy * offset;
                self.fluid_params.splat_vx = dx * push;
                self.fluid_params.splat_vy = dy * push;
                self.fluid_params.splat_radius = 4.0 + src.amplitude * 3.0;
                self.fluid_params.splat_active = 2.0; // velocity-only (no smoke)
            }
        }

        gfx.queue.write_buffer(
            &gfx.fluid_params_buffer,
            0,
            bytemuck::bytes_of(&self.fluid_params),
        );

        // Compute lightmap viewport bounds (grid coordinates with margin for light propagation)
        let half_w = self.camera.screen_w * 0.5 / self.camera.zoom;
        let half_h = self.camera.screen_h * 0.5 / self.camera.zoom;
        let lm_margin = LIGHTMAP_MARGIN;
        self.camera.lm_vp_min_x = (self.camera.center_x - half_w - lm_margin).max(0.0);
        self.camera.lm_vp_min_y = (self.camera.center_y - half_h - lm_margin).max(0.0);
        self.camera.lm_vp_max_x = (self.camera.center_x + half_w + lm_margin).min(GRID_W as f32);
        self.camera.lm_vp_max_y = (self.camera.center_y + half_h + lm_margin).min(GRID_H as f32);

        self.camera.use_shadow_map = 0.0; // shadow map removed, per-pixel ray trace only
        self.camera.enable_terrain_detail = if self.enable_terrain_detail { 1.0 } else { 0.0 };
        self.camera.terrain_ao_strength = self.terrain_ao_strength;
        self.camera.contour_opacity = if self.show_contours {
            self.contour_opacity
        } else {
            0.0
        };
        self.camera.contour_interval = self.contour_interval;
        self.camera.contour_major_mul = self.contour_major_mul;
        self.camera.fog_enabled = if self.fog_enabled { 1.0 } else { 0.0 };
        self.camera.hover_x = self.hover_world.0;
        self.camera.hover_y = self.hover_world.1;

        // Fog of war: update visibility when enabled
        if self.fog_enabled {
            let changed = fog::update_fog(
                &self.grid_data,
                &self.wall_data,
                &self.terrain_data,
                &self.plebs,
                self.camera.sun_intensity,
                self.fog_vision_radius,
                &mut self.fog_visibility,
                &mut self.fog_explored,
                &mut self.fog_texture_data,
                &mut self.fog_prev_tiles,
            );
            if changed {
                self.fog_dirty = true;
            }
        }

        // Update camera uniform
        gfx.queue
            .write_buffer(&gfx.camera_buffer, 0, bytemuck::bytes_of(&self.camera));

        // Upload pleb data to GPU buffer
        {
            let mut gpu_plebs = [GpuPleb::zeroed(); MAX_PLEBS];
            for (i, p) in self.plebs.iter().enumerate().take(MAX_PLEBS) {
                gpu_plebs[i] = p.to_gpu(self.selected_pleb == Some(i));
            }
            gfx.queue
                .write_buffer(&gfx.pleb_buffer, 0, bytemuck::cast_slice(&gpu_plebs));
        }
        // Upload creature data to GPU
        {
            let mut gpu_creatures = [creatures::GpuCreature::zeroed(); creatures::MAX_CREATURES];
            for (i, c) in self
                .creatures
                .iter()
                .enumerate()
                .take(creatures::MAX_CREATURES)
            {
                gpu_creatures[i] = c.to_gpu();
            }
            gfx.queue.write_buffer(
                &gfx.creature_buffer,
                0,
                bytemuck::cast_slice(&gpu_creatures),
            );
        }
        // --- egui frame setup (before bp_cam/blueprint computation) ---
        // gfx borrow ends here (re-borrowed later for GPU submission)
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
        let blueprint_tiles: Vec<((i32, i32), u8)> = if self.build_tool != BuildTool::None {
            let (hwx, hwy) = self.hover_world;
            let hbx = hwx.floor() as i32;
            let hby = hwy.floor() as i32;

            // Campfire subtile preview
            self.campfire_subtile = if matches!(self.build_tool, BuildTool::Place(id) if id == BT_CAMPFIRE)
            {
                let shift_held = self.pressed_keys.contains(&KeyCode::ShiftLeft)
                    || self.pressed_keys.contains(&KeyCode::ShiftRight);
                if shift_held {
                    let fx = (hwx.fract() + 1.0).fract();
                    let fy = (hwy.fract() + 1.0).fract();
                    Some((
                        ((fx * 4.0).floor() as u8).min(2),
                        ((fy * 4.0).floor() as u8).min(2),
                    ))
                } else {
                    Some((1, 1)) // centered
                }
            } else {
                None
            };
            // Diagonal wall drag: compute per-tile variants for triangle preview
            let diag_drag_tiles: Vec<(i32, i32, u8)> = if let Some((sx, sy)) = self
                .drag_start
                .filter(|_| self.build_tool == BuildTool::Place(44))
            {
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
                        let shape = reg
                            .get(id)
                            .and_then(|d| d.placement.as_ref())
                            .and_then(|p| p.drag.as_ref());
                        match shape {
                            Some(block_defs::DragShape::Line) => Self::line_tiles(sx, sy, hbx, hby),
                            Some(block_defs::DragShape::FilledRect) => {
                                Self::filled_rect_tiles(sx, sy, hbx, hby)
                            }
                            Some(block_defs::DragShape::HollowRect) => {
                                let pleb_pos = self
                                    .selected_pleb
                                    .and_then(|pi| self.plebs.get(pi).map(|p| (p.x, p.y)));
                                let (wall_tiles, entry) = Self::hollow_rect_tiles_with_entry_side(
                                    sx,
                                    sy,
                                    hbx,
                                    hby,
                                    pleb_pos,
                                    self.entry_side,
                                );
                                // Store entryway for preview rendering
                                self.drag_entryway = entry;
                                wall_tiles
                            }
                            Some(block_defs::DragShape::DiagonalLine) => {
                                Self::diagonal_wall_tiles(sx, sy, hbx, hby, self.build_rotation)
                                    .iter()
                                    .map(|&(x, y, _)| (x, y))
                                    .collect()
                            }
                            _ => vec![(hbx, hby)],
                        }
                    }
                    BuildTool::Destroy | BuildTool::GrowingZone | BuildTool::StorageZone => {
                        Self::filled_rect_tiles(sx, sy, hbx, hby)
                    }
                    BuildTool::Roof | BuildTool::RemoveRoof => {
                        Self::filled_rect_tiles(sx, sy, hbx, hby)
                    }
                    BuildTool::RemoveFloor => Self::filled_rect_tiles(sx, sy, hbx, hby),
                    _ => vec![(hbx, hby)],
                }
            } else {
                // No drag: single-tile or multi-tile preview
                match self.build_tool {
                    BuildTool::Place(9) => self.bench_tiles(hbx, hby, self.build_rotation).to_vec(),
                    BuildTool::Place(30) | BuildTool::Place(52) => {
                        self.bed_tiles(hbx, hby, self.build_rotation).to_vec()
                    }
                    BuildTool::Place(37) => self.solar_tiles(hbx, hby).to_vec(),
                    BuildTool::Place(39) => self.bed_tiles(hbx, hby, self.build_rotation).to_vec(),
                    BuildTool::Place(40) => vec![
                        (hbx, hby),
                        (hbx + 1, hby),
                        (hbx, hby + 1),
                        (hbx + 1, hby + 1),
                    ],
                    BuildTool::Place(41) => vec![
                        (hbx, hby),
                        (hbx + 1, hby),
                        (hbx, hby + 1),
                        (hbx + 1, hby + 1),
                    ],
                    BuildTool::Place(50) | BuildTool::Place(51) => {
                        self.bridge_tiles(hbx, hby, self.build_rotation).to_vec()
                    }
                    _ => vec![(hbx, hby)],
                }
            };
            let on_furniture = self.build_tool == BuildTool::Place(11);
            let is_physics = self.build_tool == BuildTool::WoodBox;
            let on_wall = matches!(
                self.build_tool,
                BuildTool::Place(12)
                    | BuildTool::Window
                    | BuildTool::Door
                    | BuildTool::Place(19)
                    | BuildTool::Place(20)
            );
            let mut preview_tiles: Vec<((i32, i32), u8)> = tiles
                .iter()
                .map(|&(tx, ty)| {
                    if is_physics {
                        // Physics bodies can be placed anywhere
                        ((tx, ty), 1u8)
                    } else if on_wall {
                        let valid =
                            if tx >= 0 && ty >= 0 && tx < GRID_W as i32 && ty < GRID_H as i32 {
                                let bidx = (ty as u32 * GRID_W + tx as u32) as usize;
                                let b = self.grid_data[bidx];
                                let bbt = b & 0xFF;
                                let bbh = (b >> 8) & 0xFF;
                                if self.build_tool == BuildTool::Window {
                                    bt_is!(
                                        bbt,
                                        BT_STONE,
                                        BT_WALL,
                                        BT_INSULATED,
                                        BT_WOOD_WALL,
                                        BT_STEEL_WALL,
                                        BT_SANDSTONE,
                                        BT_GRANITE,
                                        BT_LIMESTONE
                                    ) && bbh > 0
                                } else if self.build_tool == BuildTool::Door {
                                    bt_is!(
                                        bbt,
                                        BT_STONE,
                                        BT_GLASS,
                                        BT_INSULATED,
                                        BT_WOOD_WALL,
                                        BT_STEEL_WALL,
                                        BT_SANDSTONE,
                                        BT_GRANITE,
                                        BT_LIMESTONE
                                    ) && bbh > 0
                                } else if matches!(
                                    self.build_tool,
                                    BuildTool::Place(19)
                                        | BuildTool::Place(20)
                                        | BuildTool::Place(12)
                                ) {
                                    bt_is!(
                                        bbt,
                                        BT_STONE,
                                        BT_WALL,
                                        BT_GLASS,
                                        BT_INSULATED,
                                        BT_WOOD_WALL,
                                        BT_STEEL_WALL,
                                        BT_SANDSTONE,
                                        BT_GRANITE,
                                        BT_LIMESTONE
                                    ) && bbh > 0
                                } else {
                                    bt_is!(bbt, BT_STONE, BT_WALL) && bbh > 0
                                }
                            } else {
                                false
                            };
                        // Inlet/Outlet/Fan can also place on ground
                        if !valid
                            && matches!(
                                self.build_tool,
                                BuildTool::Place(19) | BuildTool::Place(20) | BuildTool::Place(12)
                            )
                        {
                            (
                                (tx, ty),
                                if self.can_place_on(tx, ty, false) {
                                    1u8
                                } else {
                                    0u8
                                },
                            )
                        } else {
                            ((tx, ty), if valid { 1u8 } else { 0u8 })
                        }
                    } else if self.build_tool == BuildTool::Place(36) {
                        // Wire can go anywhere
                        (
                            (tx, ty),
                            if tx >= 0 && ty >= 0 && tx < GRID_W as i32 && ty < GRID_H as i32 {
                                1u8
                            } else {
                                0u8
                            },
                        )
                    } else if self.build_tool == BuildTool::Place(52) {
                        // Liquid Intake: whole-unit validation — one ground + one water/dug
                        let intake_tiles = self.bed_tiles(hbx, hby, self.build_rotation);
                        let (gi, wi) = self.intake_tile_assignment(&intake_tiles);
                        (
                            (tx, ty),
                            if gi.is_some() && wi.is_some() {
                                1u8
                            } else {
                                0u8
                            },
                        )
                    } else if matches!(self.build_tool, BuildTool::Place(15) | BuildTool::Place(46))
                    {
                        // Gas Pipe/Restrictor: on empty ground OR existing gas pipe/restrictor
                        let ok = if tx >= 0 && ty >= 0 && tx < GRID_W as i32 && ty < GRID_H as i32 {
                            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                            let tbt = self.grid_data[tidx] & 0xFF;
                            self.can_place_on(tx, ty, false)
                                || bt_is!(tbt, BT_PIPE, BT_RESTRICTOR, BT_PIPE_BRIDGE)
                        } else {
                            false
                        };
                        ((tx, ty), if ok { 1u8 } else { 0u8 })
                    } else if self.build_tool == BuildTool::Place(49) {
                        // Liquid Pipe: on empty ground, existing liquid pipe, or bridge
                        let ok = if tx >= 0 && ty >= 0 && tx < GRID_W as i32 && ty < GRID_H as i32 {
                            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                            let tbt = self.grid_data[tidx] & 0xFF;
                            self.can_place_on(tx, ty, false)
                                || bt_is!(tbt, BT_LIQUID_PIPE, BT_PIPE_BRIDGE)
                        } else {
                            false
                        };
                        ((tx, ty), if ok { 1u8 } else { 0u8 })
                    } else if matches!(self.build_tool, BuildTool::Place(50) | BuildTool::Place(51))
                    {
                        // Bridges: on empty ground or existing pipes/wires
                        let ok = if tx >= 0 && ty >= 0 && tx < GRID_W as i32 && ty < GRID_H as i32 {
                            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                            let tbt = self.grid_data[tidx] & 0xFF;
                            self.can_place_on(tx, ty, false)
                                || (self.build_tool == BuildTool::Place(50)
                                    && (pipes::is_gas_pipe_component(tbt)
                                        || pipes::is_liquid_pipe_component(tbt)))
                                || (self.build_tool == BuildTool::Place(51) && tbt == BT_WIRE)
                        } else {
                            false
                        };
                        ((tx, ty), if ok { 1u8 } else { 0u8 })
                    } else if matches!(self.build_tool, BuildTool::Place(53) | BuildTool::Place(54))
                    {
                        // Liquid pump/output: on empty ground or on liquid pipe
                        let ok = if tx >= 0 && ty >= 0 && tx < GRID_W as i32 && ty < GRID_H as i32 {
                            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                            let tbt = self.grid_data[tidx] & 0xFF;
                            self.can_place_on(tx, ty, false) || tbt == BT_LIQUID_PIPE
                        } else {
                            false
                        };
                        ((tx, ty), if ok { 1u8 } else { 0u8 })
                    } else if matches!(
                        self.build_tool,
                        BuildTool::Place(42) | BuildTool::Place(43) | BuildTool::Place(45)
                    ) {
                        // Switch/Dimmer/Breaker: on empty ground or on wire
                        let ok = if tx >= 0 && ty >= 0 && tx < GRID_W as i32 && ty < GRID_H as i32 {
                            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                            let tbt = self.grid_data[tidx] & 0xFF;
                            self.can_place_on(tx, ty, false) || tbt == BT_WIRE
                        } else {
                            false
                        };
                        ((tx, ty), if ok { 1u8 } else { 0u8 })
                    } else {
                        // Wall-adjacent items: valid only if adjacent to a wall
                        let is_wall_adjacent = match self.build_tool {
                            BuildTool::Place(id) => {
                                let reg2 = block_defs::BlockRegistry::cached();
                                reg2.get(id)
                                    .and_then(|d| d.placement.as_ref())
                                    .map(|p| p.click == block_defs::ClickMode::WallAdjacent)
                                    .unwrap_or(false)
                            }
                            _ => false,
                        };
                        if is_wall_adjacent {
                            (
                                (tx, ty),
                                if self.wall_adjacent_direction(tx, ty).is_some() {
                                    1u8
                                } else {
                                    0u8
                                },
                            )
                        } else if matches!(self.build_tool, BuildTool::Place(id) if id == BT_WELL) {
                            // Well: must be on dug ground
                            let ok =
                                if tx >= 0 && ty >= 0 && tx < GRID_W as i32 && ty < GRID_H as i32 {
                                    let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                                    block_type_rs(self.grid_data[tidx]) == BT_DUG_GROUND
                                } else {
                                    false
                                };
                            ((tx, ty), if ok { 1u8 } else { 0u8 })
                        } else {
                            let base_ok = self.can_place_on(tx, ty, on_furniture);
                            // Thin wall: also allow placement on existing walls (for corner merge)
                            // and reject double walls (adjacent tile already has mirrored edge)
                            // Returns: 0=invalid, 1=valid/new, 2=upgrade/modify existing
                            let wall_state: u8 = if let BuildTool::Place(id) = self.build_tool {
                                if is_wall_block(id) && self.wall_thickness < 4 {
                                    let (min_x, max_x, min_y, max_y) =
                                        if let Some((sx2, sy2)) = self.drag_start {
                                            (sx2.min(hbx), sx2.max(hbx), sy2.min(hby), sy2.max(hby))
                                        } else {
                                            (tx, tx, ty, ty)
                                        };
                                    let (edge, _) = Self::thin_wall_edge_for_rect(
                                        tx,
                                        ty,
                                        min_x,
                                        max_x,
                                        min_y,
                                        max_y,
                                        self.build_rotation,
                                    );
                                    if self.is_double_wall(tx, ty, edge) {
                                        0 // invalid: double wall
                                    } else if !base_ok {
                                        // Check if existing wall tile can be upgraded (corner merge)
                                        let is_upgrade = if tx >= 0
                                            && ty >= 0
                                            && tx < GRID_W as i32
                                            && ty < GRID_H as i32
                                        {
                                            let i = (ty as u32 * GRID_W + tx as u32) as usize;
                                            let eb = self.grid_data[i];
                                            is_wall_block(block_type_rs(eb))
                                                && block_height_rs(eb) > 0
                                        } else {
                                            false
                                        };
                                        if is_upgrade { 2 } else { 0 } // orange for upgrade, red for invalid
                                    } else {
                                        1 // blue: new placement on empty ground
                                    }
                                } else if base_ok {
                                    1
                                } else {
                                    0
                                }
                            } else if base_ok {
                                1
                            } else {
                                0
                            };
                            ((tx, ty), wall_state)
                        }
                    }
                })
                .collect::<Vec<_>>();
            // Add entryway marker (state 3 = green gap)
            if let Some((ex, ey)) = self.drag_entryway {
                preview_tiles.push(((ex, ey), 3u8));
            }
            preview_tiles
        } else {
            vec![]
        };
        let bp_cam = (
            self.camera.center_x,
            self.camera.center_y,
            self.camera.zoom,
            self.camera.screen_w,
            self.camera.screen_h,
        );

        // Draw all UI (egui pass was started above, ctx is the cloned context)
        self.draw_ui(&ctx, bp_cam, blueprint_tiles, dt);

        // End egui pass and prepare for GPU rendering
        let egui_state = self.egui_state.as_mut().unwrap();
        let window = self.window.as_ref().unwrap();
        let egui_output = ctx.end_pass();
        egui_state
            .winit_state
            .handle_platform_output(window, egui_output.platform_output.clone());
        let paint_jobs = ctx.tessellate(egui_output.shapes, ctx.pixels_per_point());

        let gfx = self.gfx.as_ref().unwrap();
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [gfx.config.width, gfx.config.height],
            pixels_per_point: window.scale_factor() as f32,
        };

        // Upload egui textures
        for (id, image_delta) in &egui_output.textures_delta.set {
            egui_state
                .renderer
                .update_texture(&gfx.device, &gfx.queue, *id, image_delta);
        }
        let mut encoder = gfx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame-encoder"),
            });
        egui_state.renderer.update_buffers(
            &gfx.device,
            &gfx.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        let is_playing = self.game_state == GameState::Playing;
        let dye_w = self.dye_w;
        let dye_h = self.dye_h;

        if is_playing {
            // Lightmap: viewport-culled propagation at 2x resolution
            self.lightmap_frame += 1;
            let need_lightmap = self.lightmap_frame >= self.lightmap_interval
                || self.grid_dirty
                || self.camera.force_refresh > 0.5;
            if need_lightmap {
                self.lightmap_frame = 0;
                let lm_wg_x = LIGHTMAP_W.div_ceil(WORKGROUP_SIZE);
                let lm_wg_y = LIGHTMAP_H.div_ceil(WORKGROUP_SIZE);

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
                for i in 0..self.lightmap_iterations {
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
            let fluid_wg = fluid_res.div_ceil(WORKGROUP_SIZE);
            let dye_wg_x = self.dye_w.div_ceil(WORKGROUP_SIZE);
            let dye_wg_y = self.dye_h.div_ceil(WORKGROUP_SIZE);

            // 1. Curl
            {
                let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("fluid-curl"),
                    timestamp_writes: None,
                });
                p.set_pipeline(&gfx.fluid_p_curl);
                p.set_bind_group(0, &gfx.fluid_bg_curl, &[]);
                p.dispatch_workgroups(fluid_wg, fluid_wg, 1);
            }
            // 2. Vorticity
            {
                let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("fluid-vorticity"),
                    timestamp_writes: None,
                });
                p.set_pipeline(&gfx.fluid_p_vorticity);
                p.set_bind_group(0, &gfx.fluid_bg_vorticity, &[]);
                p.dispatch_workgroups(fluid_wg, fluid_wg, 1);
            }
            // 3. Splat
            {
                let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("fluid-splat"),
                    timestamp_writes: None,
                });
                p.set_pipeline(&gfx.fluid_p_splat);
                p.set_bind_group(0, &gfx.fluid_bg_splat, &[]);
                p.dispatch_workgroups(fluid_wg, fluid_wg, 1);
            }
            // 4. Divergence
            {
                let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("fluid-div"),
                    timestamp_writes: None,
                });
                p.set_pipeline(&gfx.fluid_p_divergence);
                p.set_bind_group(0, &gfx.fluid_bg_divergence, &[]);
                p.dispatch_workgroups(fluid_wg, fluid_wg, 1);
            }
            // 5. Pressure clear
            {
                let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("fluid-pres-clear"),
                    timestamp_writes: None,
                });
                p.set_pipeline(&gfx.fluid_p_pressure_clear);
                p.set_bind_group(0, &gfx.fluid_bg_pressure_clear, &[]);
                p.dispatch_workgroups(fluid_wg, fluid_wg, 1);
            }
            // 6. Pressure Jacobi (16 iterations, ping-pong)
            for i in 0..self.fluid_pressure_iters {
                let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("fluid-pressure"),
                    timestamp_writes: None,
                });
                p.set_pipeline(&gfx.fluid_p_pressure);
                p.set_bind_group(0, &gfx.fluid_bg_pressure[(i as usize) % 2], &[]);
                p.dispatch_workgroups(fluid_wg, fluid_wg, 1);
            }
            // 7. Gradient subtract
            {
                let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("fluid-gradient"),
                    timestamp_writes: None,
                });
                p.set_pipeline(&gfx.fluid_p_gradient);
                p.set_bind_group(0, &gfx.fluid_bg_gradient, &[]);
                p.dispatch_workgroups(fluid_wg, fluid_wg, 1);
            }
            // 8. Advect velocity
            {
                let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("fluid-advect-vel"),
                    timestamp_writes: None,
                });
                p.set_pipeline(&gfx.fluid_p_advect_vel);
                p.set_bind_group(0, &gfx.fluid_bg_advect_vel, &[]);
                p.dispatch_workgroups(fluid_wg, fluid_wg, 1);
            }
            // 9. Advect dye (512x512)
            {
                let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("fluid-advect-dye"),
                    timestamp_writes: None,
                });
                p.set_pipeline(&gfx.fluid_p_advect_dye);
                p.set_bind_group(0, &gfx.fluid_bg_advect_dye[self.fluid_dye_phase], &[]);
                p.dispatch_workgroups(dye_wg_x, dye_wg_y, 1);
            }
            // Flip dye phase for next frame
            self.fluid_dye_phase = 1 - self.fluid_dye_phase;

            // --- Dust simulation ---
            // Update DustParams uniform
            {
                let dust_dt = self.fluid_params.dt;
                let params = dust::DustParams {
                    grid_w: dust::DUST_SIM_W as f32,
                    grid_h: dust::DUST_SIM_H as f32,
                    dt: dust_dt,
                    decay_rate: dust::compute_decay_rate(60.0, dust_dt), // 60s half-life (very heavy)
                    diffusion: 0.001, // almost no spread (heavy particles settle in place)
                    wind_follow: 0.05, // 5% of air velocity — barely drifts
                    wind_x: self.fluid_params.wind_x,
                    wind_y: self.fluid_params.wind_y,
                    storm_active: if self.dust_storm_active { 1.0 } else { 0.0 },
                    storm_edge: dust::windward_edge(
                        self.fluid_params.wind_x,
                        self.fluid_params.wind_y,
                    ),
                    storm_density: 0.8,
                    _pad: 0.0,
                };
                gfx.queue
                    .write_buffer(&gfx.dust_params_buffer, 0, bytemuck::bytes_of(&params));
            }
            // CPU dust injections — write Gaussian blobs to BOTH dust textures
            for inj in self.dust_injections.drain(..) {
                let w = dust::DUST_SIM_W;
                let h = dust::DUST_SIM_H;
                let sx = inj.x / GRID_W as f32 * w as f32;
                let sy = inj.y / GRID_H as f32 * h as f32;
                let r = inj.radius / GRID_W as f32 * w as f32;
                let ri = r.ceil() as i32;
                let x0 = (sx as i32 - ri).max(0) as u32;
                let y0 = (sy as i32 - ri).max(0) as u32;
                let x1 = (sx as i32 + ri + 1).min(w as i32) as u32;
                let y1 = (sy as i32 + ri + 1).min(h as i32) as u32;
                let patch_w = x1 - x0;
                let patch_h = y1 - y0;
                if patch_w == 0 || patch_h == 0 {
                    continue;
                }
                let mut patch = vec![0.0f32; (patch_w * patch_h) as usize];
                for py in 0..patch_h {
                    for px in 0..patch_w {
                        let dx = (x0 + px) as f32 - sx;
                        let dy = (y0 + py) as f32 - sy;
                        let dist2 = dx * dx + dy * dy;
                        let sigma2 = r * r * 0.25;
                        let val = inj.density * (-dist2 / (2.0 * sigma2)).exp();
                        patch[(py * patch_w + px) as usize] = val;
                    }
                }
                let patch_bytes = bytemuck::cast_slice(&patch);
                for tex in &gfx.dust_textures {
                    gfx.queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: tex,
                            mip_level: 0,
                            origin: wgpu::Origin3d { x: x0, y: y0, z: 0 },
                            aspect: wgpu::TextureAspect::All,
                        },
                        patch_bytes,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(patch_w * 4), // 4 bytes per R32Float
                            rows_per_image: Some(patch_h),
                        },
                        wgpu::Extent3d {
                            width: patch_w,
                            height: patch_h,
                            depth_or_array_layers: 1,
                        },
                    );
                }
            }
            // Dust advection compute pass (reads A, writes B)
            {
                let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("dust"),
                    timestamp_writes: None,
                });
                p.set_pipeline(&gfx.dust_pipeline);
                p.set_bind_group(0, &gfx.dust_bind_group, &[]);
                p.dispatch_workgroups(
                    dust::DUST_SIM_W.div_ceil(8),
                    dust::DUST_SIM_H.div_ceil(8),
                    1,
                );
            }
            // Copy B → A so next frame reads the advected result
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &gfx.dust_textures[1],
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &gfx.dust_textures[0],
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: dust::DUST_SIM_W,
                    height: dust::DUST_SIM_H,
                    depth_or_array_layers: 1,
                },
            );

            // 10. Thermal exchange (256x256)
            {
                let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("thermal"),
                    timestamp_writes: None,
                });
                let tw = GRID_W.div_ceil(8);
                let th = GRID_H.div_ceil(8);
                p.set_pipeline(&gfx.thermal_pipeline);
                p.set_bind_group(0, &gfx.thermal_bind_group, &[]);
                p.dispatch_workgroups(tw, th, 1);
            }

            // 11. Power grid voltage relaxation (256x256)
            #[cfg(target_arch = "wasm32")]
            let power_iters = 4;
            #[cfg(not(target_arch = "wasm32"))]
            let power_iters = 8;
            {
                let tw = GRID_W.div_ceil(8);
                let th = GRID_H.div_ceil(8);
                for _ in 0..power_iters {
                    let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("power"),
                        timestamp_writes: None,
                    });
                    p.set_pipeline(&gfx.power_pipeline);
                    p.set_bind_group(0, &gfx.power_bind_group, &[]);
                    p.dispatch_workgroups(tw, th, 1);
                }
            }

            // 12. Ground water simulation (256x256, every 4 frames)
            self.water_frame += 1;
            if self.water_frame.is_multiple_of(4) && !self.time_paused {
                let tw = GRID_W.div_ceil(8);
                let th = GRID_H.div_ceil(8);
                let mut p = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("water"),
                    timestamp_writes: None,
                });
                p.set_pipeline(&gfx.water_pipeline);
                p.set_bind_group(0, &gfx.water_bind_groups[self.water_phase], &[]);
                p.dispatch_workgroups(tw, th, 1);
                drop(p);
                self.water_phase = 1 - self.water_phase;
            }

            // Debug: copy one dye texel at cursor position for readback
            let ctrl_for_debug = self.pressed_keys.contains(&KeyCode::ControlLeft)
                || self.pressed_keys.contains(&KeyCode::ControlRight);
            if self.debug.mode || ctrl_for_debug {
                let (wx, wy) = self.hover_world;
                let dye_x =
                    ((wx / GRID_W as f32) * dye_w as f32).clamp(0.0, (dye_w - 1) as f32) as u32;
                let dye_y =
                    ((wy / GRID_H as f32) * dye_h as f32).clamp(0.0, (dye_h - 1) as f32) as u32;
                // The current readable dye is the one we just wrote to (dye phase was already flipped)
                let dye_idx = self.fluid_dye_phase; // after flip, this points to the fresh output
                encoder.copy_texture_to_buffer(
                    wgpu::TexelCopyTextureInfo {
                        texture: &gfx.fluid_dye[dye_idx],
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: dye_x,
                            y: dye_y,
                            z: 0,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyBufferInfo {
                        buffer: &gfx.debug_readback_buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(READBACK_ALIGNMENT as u32),
                            rows_per_image: Some(1),
                        },
                    },
                    wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                );
                self.debug.fluid_pending = true;

                // Also copy water level at cursor
                let water_bx = wx.floor().clamp(0.0, (GRID_W - 1) as f32) as u32;
                let water_by = wy.floor().clamp(0.0, (GRID_H - 1) as f32) as u32;
                let water_read_idx = self.water_phase; // current readable water texture
                encoder.copy_texture_to_buffer(
                    wgpu::TexelCopyTextureInfo {
                        texture: &gfx.water_textures[water_read_idx],
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: water_bx,
                            y: water_by,
                            z: 0,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyBufferInfo {
                        buffer: &gfx.water_readback_buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(READBACK_ALIGNMENT as u32),
                            rows_per_image: Some(1),
                        },
                    },
                    wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                );
                self.debug.water_pending = true;

                // Also copy block temperature at cursor position from block_temp_buffer
                let btx = wx.floor() as i32;
                let bty = wy.floor() as i32;
                if btx >= 0 && bty >= 0 && btx < GRID_W as i32 && bty < GRID_H as i32 {
                    let bt_idx = (bty as u32 * GRID_W + btx as u32) as u64;
                    encoder.copy_buffer_to_buffer(
                        &gfx.block_temp_buffer,
                        bt_idx * 4,
                        &gfx.block_temp_readback_buffer,
                        0,
                        4,
                    );
                    encoder.copy_buffer_to_buffer(
                        &gfx.voltage_buffer,
                        bt_idx * 4,
                        &gfx.block_temp_readback_buffer,
                        4, // second f32
                        4,
                    );
                    self.debug.block_temp_pending = true;
                    self.debug.voltage_pending = true;
                }
            }

            // Copy full voltage buffer for per-tile labels (power overlay or flow overlay)
            if matches!(
                self.fluid_overlay,
                FluidOverlay::Power | FluidOverlay::PowerAmps | FluidOverlay::PowerWatts
            ) || self.show_flow_overlay
            {
                encoder.copy_buffer_to_buffer(
                    &gfx.voltage_buffer,
                    0,
                    &gfx.voltage_readback_buffer,
                    0,
                    (GRID_W * GRID_H * 4) as u64,
                );
                self.voltage_readback_pending = true;
            }

            // Copy dye texels at each pleb position for air readback
            if !self.plebs.is_empty() {
                let dye_idx = self.fluid_dye_phase;
                for (i, pleb) in self.plebs.iter().enumerate() {
                    let dye_x = ((pleb.x / GRID_W as f32) * dye_w as f32)
                        .clamp(0.0, (dye_w - 1) as f32) as u32;
                    let dye_y = ((pleb.y / GRID_H as f32) * dye_h as f32)
                        .clamp(0.0, (dye_h - 1) as f32) as u32;
                    encoder.copy_texture_to_buffer(
                        wgpu::TexelCopyTextureInfo {
                            texture: &gfx.fluid_dye[dye_idx],
                            mip_level: 0,
                            origin: wgpu::Origin3d {
                                x: dye_x,
                                y: dye_y,
                                z: 0,
                            },
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyBufferInfo {
                            buffer: &gfx.pleb_air_readback_buffer,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: (i as u64) * READBACK_ALIGNMENT,
                                bytes_per_row: Some(READBACK_ALIGNMENT as u32),
                                rows_per_image: Some(1),
                            },
                        },
                        wgpu::Extent3d {
                            width: 1,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                    );
                }
                self.pleb_air_readback_pending = true;
            }

            // Reset splat
            self.fluid_params.splat_active = 0.0;

            // Sound wave propagation (multiple iterations per frame)
            if self.sound_enabled {
                // Pack sound parameters into camera padding fields
                self.camera.sound_speed = self.sound_speed;
                self.camera.sound_damping = self.sound_damping;
                self.camera.sound_coupling = self.sound_coupling;
                // Re-upload camera with sound params
                gfx.queue
                    .write_buffer(&gfx.camera_buffer, 0, bytemuck::bytes_of(&self.camera));

                // Pack active sound sources into buffer
                let mut source_data = vec![0.0f32; 1 + MAX_SOUND_SOURCES * SOUND_SOURCE_STRIDE];
                let count = self.sound_sources.len().min(MAX_SOUND_SOURCES);
                source_data[0] = count as f32;
                for (i, src) in self
                    .sound_sources
                    .iter()
                    .enumerate()
                    .take(MAX_SOUND_SOURCES)
                {
                    let base = 1 + i * SOUND_SOURCE_STRIDE;
                    source_data[base] = src.x;
                    source_data[base + 1] = src.y;
                    source_data[base + 2] = src.amplitude;
                    source_data[base + 3] = src.frequency;
                    source_data[base + 4] = src.phase;
                    source_data[base + 5] = src.pattern as f32;
                    source_data[base + 6] = src.duration;
                }
                gfx.queue.write_buffer(
                    &gfx.sound_source_buffer,
                    0,
                    bytemuck::cast_slice(&source_data),
                );

                // Tick sources: advance phase, decrement duration, remove expired
                let dt_sound = 1.0 / 60.0;
                for src in &mut self.sound_sources {
                    src.phase += src.frequency * dt_sound * std::f32::consts::TAU;
                    src.duration -= dt_sound;
                }
                self.sound_sources.retain(|s| s.duration > 0.0);

                // Dispatch wave equation iterations (ensure even count so result lands in texture A)
                let iters = (self.sound_iters_per_frame / 2) * 2; // round down to even
                let iters = iters.max(2);
                let sw = (GRID_W * 2).div_ceil(WORKGROUP_SIZE);
                let sh = (GRID_H * 2).div_ceil(WORKGROUP_SIZE);
                for _ in 0..iters {
                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("sound-pass"),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(&gfx.sound_pipeline);
                    cpass.set_bind_group(0, &gfx.sound_bind_groups[self.sound_phase], &[]);
                    cpass.dispatch_workgroups(sw, sh, 1);
                    drop(cpass);
                    self.sound_phase = 1 - self.sound_phase;
                }
                // After even iterations, result is back in texture A (phase 0)
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
                cpass.set_bind_group(
                    0,
                    &gfx.compute_bind_groups[self.fluid_dye_phase * 2 + self.output_phase],
                    &[],
                );
                let wg_x = rt_w.div_ceil(WORKGROUP_SIZE);
                let wg_y = rt_h.div_ceil(WORKGROUP_SIZE);
                cpass.dispatch_workgroups(wg_x, wg_y, 1);
            }
        } // end if is_playing

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
            // Batch: collect all pipe temps into a staging vec, write once
            if !self.pipe_network.cells.is_empty() {
                let grid_size = (GRID_W * GRID_H) as usize;
                let mut temp_buf = vec![0.0f32; grid_size];
                for (&idx, cell) in &self.pipe_network.cells {
                    if (idx as usize) < grid_size {
                        temp_buf[idx as usize] = cell.gas[3];
                    }
                }
                gfx.queue
                    .write_buffer(&gfx.block_temp_buffer, 0, bytemuck::cast_slice(&temp_buf));
            }

            // Apply pipe outlet injections to dye texture (AFTER shader runs)
            // Write into cells ADJACENT to the outlet (in the outlet's facing direction)
            for &(ox, oy, gas, pressure) in &pipe_injections {
                if pressure < 0.05 {
                    continue;
                }
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
                if inject_x < 0
                    || inject_y < 0
                    || inject_x >= GRID_W as i32
                    || inject_y >= GRID_H as i32
                {
                    continue;
                }

                // Dye texture is render-resolution; map world coords to dye pixel coords
                let dye_x = ((inject_x as f32 / GRID_W as f32) * dye_w as f32) as i32;
                let dye_y = ((inject_y as f32 / GRID_H as f32) * dye_h as f32) as i32;
                let s = (pressure * 0.5).min(1.0);
                let pixel: [u16; 4] = [
                    f32_to_f16(gas[0] * s),
                    f32_to_f16(gas[1].max(0.3)),
                    f32_to_f16(gas[2] * s),
                    f32_to_f16(gas[3]),
                ];
                let bytes: &[u8] = bytemuck::cast_slice(&pixel);
                // Write to BOTH dye textures at the computed dye pixel
                let tx = dye_x.clamp(0, dye_w as i32 - 1) as u32;
                let ty = dye_y.clamp(0, dye_h as i32 - 1) as u32;
                for dye_idx in 0..2 {
                    gfx.queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &gfx.fluid_dye[dye_idx],
                            mip_level: 0,
                            origin: wgpu::Origin3d { x: tx, y: ty, z: 0 },
                            aspect: wgpu::TextureAspect::All,
                        },
                        bytes,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(8),
                            rows_per_image: Some(1),
                        },
                        wgpu::Extent3d {
                            width: 1,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                    );
                }
            }

            // Grenade toxic gas injection: continuous emission while fuse burns
            for &(gx, gy) in &self.grenade_impacts {
                let radius = 1i32; // small source — fluid sim spreads it
                for oy in -radius..=radius {
                    for ox in -radius..=radius {
                        let dist = ((ox * ox + oy * oy) as f32).sqrt();
                        if dist > radius as f32 + 0.5 {
                            continue;
                        }
                        let strength = 1.0 - dist / (radius as f32 + 1.0);
                        let wx = (gx as i32 + ox).clamp(0, GRID_W as i32 - 1);
                        let wy = (gy as i32 + oy).clamp(0, GRID_H as i32 - 1);
                        let dye_bx = ((wx as f32 / GRID_W as f32) * dye_w as f32) as i32;
                        let dye_by = ((wy as f32 / GRID_H as f32) * dye_h as f32) as i32;
                        let pixel: [u16; 4] = [
                            f32_to_f16(0.6 * strength),
                            f32_to_f16(0.2),
                            f32_to_f16(0.8 * strength),
                            f32_to_f16(15.0),
                        ];
                        let bytes: &[u8] = bytemuck::cast_slice(&pixel);
                        let tx = dye_bx.clamp(0, dye_w as i32 - 1) as u32;
                        let ty = dye_by.clamp(0, dye_h as i32 - 1) as u32;
                        for dye_idx in 0..2 {
                            gfx.queue.write_texture(
                                wgpu::TexelCopyTextureInfo {
                                    texture: &gfx.fluid_dye[dye_idx],
                                    mip_level: 0,
                                    origin: wgpu::Origin3d { x: tx, y: ty, z: 0 },
                                    aspect: wgpu::TextureAspect::All,
                                },
                                bytes,
                                wgpu::TexelCopyBufferLayout {
                                    offset: 0,
                                    bytes_per_row: Some(8),
                                    rows_per_image: Some(1),
                                },
                                wgpu::Extent3d {
                                    width: 1,
                                    height: 1,
                                    depth_or_array_layers: 1,
                                },
                            );
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
                    gfx.device
                        .poll(wgpu::PollType::Wait {
                            submission_index: None,
                            timeout: None,
                        })
                        .ok();
                    let data = buffer_slice.get_mapped_range();
                    let f16_data: &[u16] = bytemuck::cast_slice(&data);
                    for (i, &val) in f16_data.iter().enumerate().take(4) {
                        self.debug.fluid_density[i] = half_to_f32(val);
                    }
                    drop(data);
                    gfx.debug_readback_buffer.unmap();
                }
            }

            // Water level readback
            if self.debug.water_pending {
                self.debug.water_pending = false;
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let buffer_slice = gfx.water_readback_buffer.slice(..);
                    buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
                    gfx.device
                        .poll(wgpu::PollType::Wait {
                            submission_index: None,
                            timeout: None,
                        })
                        .ok();
                    let data = buffer_slice.get_mapped_range();
                    let f32_data: &[f32] = bytemuck::cast_slice(&data);
                    self.debug.water_level = f32_data[0];
                    drop(data);
                    gfx.water_readback_buffer.unmap();
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
                    gfx.device
                        .poll(wgpu::PollType::Wait {
                            submission_index: None,
                            timeout: None,
                        })
                        .ok();
                    let data = buffer_slice.get_mapped_range();
                    let values: &[f32] = bytemuck::cast_slice(&data);
                    self.debug.block_temp = values[0];
                    self.debug.voltage = values[1];
                    drop(data);
                    gfx.block_temp_readback_buffer.unmap();
                }
            }

            // Voltage grid readback for per-tile labels
            if self.voltage_readback_pending {
                self.voltage_readback_pending = false;
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let buf_size = (GRID_W * GRID_H * 4) as u64;
                    let buffer_slice = gfx.voltage_readback_buffer.slice(..buf_size);
                    buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
                    gfx.device
                        .poll(wgpu::PollType::Wait {
                            submission_index: None,
                            timeout: None,
                        })
                        .ok();
                    let data = buffer_slice.get_mapped_range();
                    let values: &[f32] = bytemuck::cast_slice(&data);
                    self.voltage_data.clear();
                    self.voltage_data.extend_from_slice(values);
                    drop(data);
                    gfx.voltage_readback_buffer.unmap();
                }
            }

            // Pleb air readback processing
            if self.pleb_air_readback_pending {
                self.pleb_air_readback_pending = false;
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let num_plebs = self.plebs.len();
                    if num_plebs > 0 {
                        let read_size = num_plebs as u64 * READBACK_ALIGNMENT;
                        let buffer_slice = gfx.pleb_air_readback_buffer.slice(..read_size);
                        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
                        gfx.device
                            .poll(wgpu::PollType::Wait {
                                submission_index: None,
                                timeout: None,
                            })
                            .ok();
                        let data = buffer_slice.get_mapped_range();
                        self.pleb_air_data.resize(num_plebs, AirReadback::default());
                        for i in 0..num_plebs {
                            let offset = i * READBACK_ALIGNMENT as usize;
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

            let mut egui_encoder =
                gfx.device
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
            let mut rpass: wgpu::RenderPass<'static> = unsafe { std::mem::transmute(rpass) };
            egui_state
                .renderer
                .render(&mut rpass, &paint_jobs, &screen_descriptor);
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
        let attrs = {
            // 50% larger than default, matching monitor aspect ratio
            let monitor = event_loop
                .primary_monitor()
                .or_else(|| event_loop.available_monitors().next());
            let (w, h) = if let Some(m) = monitor {
                let size = m.size();
                (
                    (size.width as f32 * WINDOW_SCALE) as u32,
                    (size.height as f32 * WINDOW_SCALE) as u32,
                )
            } else {
                DEFAULT_WINDOW_SIZE
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
        // Always track cursor position and handle panning/selection before egui
        if let WindowEvent::CursorMoved { position, .. } = &event {
            if self.mouse_pressed {
                let dx = position.x - self.last_mouse_x;
                let dy = position.y - self.last_mouse_y;
                if dx.abs() > DRAG_THRESHOLD || dy.abs() > DRAG_THRESHOLD {
                    self.mouse_dragged = true;
                }
                let shift_held = self.pressed_keys.contains(&KeyCode::ShiftLeft)
                    || self.pressed_keys.contains(&KeyCode::ShiftRight);
                // Shape-building tools: don't pan, just track drag
                if self.mouse_dragged && self.drag_start.is_some() {
                    // Preview is drawn in the egui section — just don't pan
                } else if self.mouse_dragged && shift_held && self.build_tool == BuildTool::None {
                    // Shift+drag = selection rectangle
                    if self.select_drag_start.is_none() {
                        let (wx, wy) = self.screen_to_world(self.last_mouse_x, self.last_mouse_y);
                        self.select_drag_start = Some((wx, wy));
                    }
                } else if self.mouse_dragged {
                    // Plain drag = pan
                    self.camera.center_x -= dx as f32 * self.render_scale / self.camera.zoom;
                    self.camera.center_y -= dy as f32 * self.render_scale / self.camera.zoom;
                    self.window.as_ref().unwrap().request_redraw();
                }
            }
            // Middle mouse drag = fast pan (3x speed, no drag threshold)
            if self.middle_mouse_pressed {
                let dx = position.x - self.last_mouse_x;
                let dy = position.y - self.last_mouse_y;
                let pan_mul = 3.0; // faster than left-click drag
                self.camera.center_x -= dx as f32 * self.render_scale / self.camera.zoom * pan_mul;
                self.camera.center_y -= dy as f32 * self.render_scale / self.camera.zoom * pan_mul;
                self.camera.center_x = self.camera.center_x.clamp(0.0, GRID_W as f32);
                self.camera.center_y = self.camera.center_y.clamp(0.0, GRID_H as f32);
                self.window.as_ref().unwrap().request_redraw();
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
            let response = egui_state
                .winit_state
                .on_window_event(self.window.as_ref().unwrap(), &event);
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
                if self.terrain_tool.is_some() {
                    // Scroll adjusts brush size when terrain tool is active
                    if scroll > 0.0 && self.terrain_brush_radius < 5 {
                        self.terrain_brush_radius += 1;
                    } else if scroll < 0.0 && self.terrain_brush_radius > 1 {
                        self.terrain_brush_radius -= 1;
                    }
                } else if scroll > 0.0 {
                    self.camera.zoom *= ZOOM_FACTOR;
                } else if scroll < 0.0 {
                    self.camera.zoom /= ZOOM_FACTOR;
                }
                self.camera.zoom = self
                    .camera
                    .zoom
                    .clamp(base_zoom * ZOOM_MIN_MULT, base_zoom * ZOOM_MAX_MULT);
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == winit::event::MouseButton::Left {
                    if state.is_pressed() {
                        self.mouse_pressed = true;
                        self.mouse_dragged = false;
                        // Start drag for shape-building tools
                        let is_shape_tool = match self.build_tool {
                            BuildTool::Destroy
                            | BuildTool::Roof
                            | BuildTool::RemoveFloor
                            | BuildTool::RemoveRoof
                            | BuildTool::GrowingZone
                            | BuildTool::StorageZone => true,
                            BuildTool::Place(id) => {
                                let reg = block_defs::BlockRegistry::cached();
                                reg.get(id)
                                    .and_then(|d| d.placement.as_ref())
                                    .and_then(|p| p.drag.as_ref())
                                    .map(|s| *s != block_defs::DragShape::None)
                                    .unwrap_or(false)
                            }
                            _ => false,
                        };
                        if is_shape_tool {
                            let (wx, wy) =
                                self.screen_to_world(self.last_mouse_x, self.last_mouse_y);
                            self.drag_start = Some((wx.floor() as i32, wy.floor() as i32));
                        }
                    } else {
                        // Mouse released
                        if let Some((sx, sy)) = self.drag_start.filter(|_| self.mouse_dragged) {
                            let (wx, wy) =
                                self.screen_to_world(self.last_mouse_x, self.last_mouse_y);
                            let (ex, ey) = (wx.floor() as i32, wy.floor() as i32);
                            self.apply_drag_shape(sx, sy, ex, ey);
                        } else if let Some((sx, sy)) = self.select_drag_start {
                            let (ex, ey) =
                                self.screen_to_world(self.last_mouse_x, self.last_mouse_y);
                            let min_x = sx.min(ex).floor() as i32;
                            let min_y = sy.min(ey).floor() as i32;
                            let max_x = sx.max(ex).ceil() as i32;
                            let max_y = sy.max(ey).ceil() as i32;
                            let mut items = Vec::new();
                            let mut seen = std::collections::HashSet::new();
                            for gy in min_y..max_y {
                                for gx in min_x..max_x {
                                    if gx < 0
                                        || gy < 0
                                        || gx >= GRID_W as i32
                                        || gy >= GRID_H as i32
                                    {
                                        continue;
                                    }
                                    let bidx = (gy as u32 * GRID_W + gx as u32) as usize;
                                    let b = self.grid_data[bidx];
                                    let bbt = block_type_rs(b);
                                    let bflags = block_flags_rs(b);
                                    let is_gnd = is_ground_block(bbt);
                                    // Include blueprints even on ground tiles
                                    let bp = self.blueprints.get(&(gx, gy));
                                    if is_gnd && bp.is_none() {
                                        continue;
                                    }
                                    let bt_for_sel = if let Some(bp) = bp {
                                        bp.block_data & 0xFF
                                    } else {
                                        bbt
                                    };
                                    let (ox, oy, ow, oh) = if bp.is_some() {
                                        (gx, gy, 1, 1)
                                    } else {
                                        self.get_block_bounds(gx, gy, bbt, bflags)
                                    };
                                    if seen.insert((ox, oy)) {
                                        items.push(SelectedItem {
                                            x: ox,
                                            y: oy,
                                            w: ow,
                                            h: oh,
                                            block_type: bt_for_sel,
                                            pleb_idx: None,
                                        });
                                    }
                                }
                            }
                            // Also check plebs in the selection area
                            let mut first_pleb = None;
                            for (pi, pleb) in self.plebs.iter().enumerate() {
                                let px = pleb.x.floor() as i32;
                                let py = pleb.y.floor() as i32;
                                if px >= min_x && px < max_x && py >= min_y && py < max_y {
                                    items.push(SelectedItem {
                                        x: px,
                                        y: py,
                                        w: 1,
                                        h: 1,
                                        block_type: SEL_PLEB,
                                        pleb_idx: Some(pi),
                                    });
                                    if first_pleb.is_none() {
                                        first_pleb = Some(pi);
                                    }
                                }
                            }
                            self.world_sel = WorldSelection { items };
                            // If exactly one pleb selected, make it the active pleb
                            self.selected_pleb = if self
                                .world_sel
                                .items
                                .iter()
                                .filter(|i| i.pleb_idx.is_some())
                                .count()
                                == 1
                            {
                                first_pleb
                            } else {
                                None
                            };
                        } else if !self.mouse_dragged {
                            let (wx, wy) =
                                self.screen_to_world(self.last_mouse_x, self.last_mouse_y);
                            // Ctrl+left-click on Mac = right-click equivalent
                            let ctrl_held = self.pressed_keys.contains(&KeyCode::ControlLeft)
                                || self.pressed_keys.contains(&KeyCode::ControlRight);
                            if ctrl_held && self.selected_pleb.is_some() {
                                self.open_context_menu(
                                    self.last_mouse_x as f32,
                                    self.last_mouse_y as f32,
                                    wx,
                                    wy,
                                );
                            } else {
                                self.handle_click(wx, wy);
                            }
                        }
                        self.mouse_pressed = false;
                        self.mouse_dragged = false;
                        self.drag_start = None;
                        self.drag_entryway = None;
                        self.entry_side = 0;
                        self.select_drag_start = None;
                    }
                }
                // Middle-click: fast pan
                if button == winit::event::MouseButton::Middle {
                    self.middle_mouse_pressed = state.is_pressed();
                }

                // Right-click: context menu for selected pleb, rock menu, or pick up lights
                if button == winit::event::MouseButton::Right {
                    if state.is_pressed() {
                        let (wx, wy) = self.screen_to_world(self.last_mouse_x, self.last_mouse_y);
                        if self.selected_pleb.is_some() {
                            self.open_context_menu(
                                self.last_mouse_x as f32,
                                self.last_mouse_y as f32,
                                wx,
                                wy,
                            );
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
        let tile_set: std::collections::HashMap<(i32, i32), u8> =
            tiles.iter().map(|&(x, y, v)| ((x, y), v)).collect();

        // For each tile, check that it connects to at least one neighbor's solid half
        for &(tx, ty, tv) in tiles {
            let neighbors = [(tx - 1, ty), (tx + 1, ty), (tx, ty - 1), (tx, ty + 1)];
            let mut connected = false;

            for (nx, ny) in neighbors {
                if let Some(&nv) = tile_set.get(&(nx, ny)) {
                    // Check if the shared edge has solid pixels on both sides.
                    // Sample 5 points along the shared edge.
                    let edge_solid_count = (0..5)
                        .filter(|&i| {
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
                        })
                        .count();

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
                    panic!(
                        "{}: tile ({},{}) variant {} has no connected neighbor",
                        label, tx, ty, tv
                    );
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
            (3, 3, "right-down"), // \ direction
            (-3, -3, "left-up"),  // \ direction
            (3, -3, "right-up"),  // / direction
            (-3, 3, "left-down"), // / direction
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
        assert_eq!(
            fwd_set, rev_set,
            "forward and reverse drags should cover same tiles"
        );
    }
}
