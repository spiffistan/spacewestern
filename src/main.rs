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
const GRID_W: u32 = 64;
const GRID_H: u32 = 64;
const WORKGROUP_SIZE: u32 = 8;
const DAY_DURATION: f32 = 60.0; // must match shader

// --- Block representation on GPU ---
// Each block is a u32 packed as: [type:8 | height:8 | flags:8 | reserved:8]
// type: 0=air, 1=stone, 2=dirt, 3=water, 4=wall, 5=glass, 6=fireplace, 7=electric_light
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
    // Fireplace in center of house 1
    set(&mut grid, 19, 17, make_block(6, 1, roof_flag)); // fireplace (height 1, roofed)
    // Electric light near left-wall window (2 tiles east of window at x=10, y=15)
    set(&mut grid, 12, 15, make_block(7, 0, roof_flag)); // electric light (height 0, roofed)

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

    grid
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
    _pad2: f32,
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
    #[allow(dead_code)]
    start_time: Instant,
    // Time control
    time_of_day: f32,        // current time in seconds (0..DAY_DURATION)
    time_paused: bool,       // pause auto-advance
    time_speed: f32,         // playback speed multiplier
    last_frame_time: Instant, // for delta-time calculation
}

struct GfxState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    #[allow(dead_code)]
    surface_format: wgpu::TextureFormat,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    render_bind_group: wgpu::BindGroup,
    output_texture: wgpu::Texture,
    camera_buffer: wgpu::Buffer,
    grid_buffer: wgpu::Buffer,
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
                center_x: GRID_W as f32 / 2.0,
                center_y: GRID_H as f32 / 2.0,
                zoom: 1.0, // will be set in init_gfx_async to fit map
                show_roofs: 0.0,
                screen_w: 800.0,
                screen_h: 600.0,
                grid_w: GRID_W as f32,
                grid_h: GRID_H as f32,
                time: 0.0,
                _pad2: 0.0,
            },
            grid_data: Vec::new(),
            grid_dirty: false,
            mouse_pressed: false,
            mouse_dragged: false,
            last_mouse_x: 0.0,
            last_mouse_y: 0.0,
            start_time: Instant::now(),
            time_of_day: 0.0,
            time_paused: false,
            time_speed: 1.0,
            last_frame_time: Instant::now(),
        }
    }

    /// Convert screen pixel coordinates to world block coordinates
    fn screen_to_world(&self, sx: f64, sy: f64) -> (f32, f32) {
        let wx = self.camera.center_x + (sx as f32 - self.camera.screen_w * 0.5) / self.camera.zoom;
        let wy = self.camera.center_y + (sy as f32 - self.camera.screen_h * 0.5) / self.camera.zoom;
        (wx, wy)
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

        self.camera.screen_w = width as f32;
        self.camera.screen_h = height as f32;
        // Fit map to canvas: zoom = pixels per world unit
        self.camera.zoom = height as f32 / GRID_H as f32;
        log::info!("init_gfx: {}x{} (physical), zoom={}", width, height, self.camera.zoom);

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

        // Output texture (compute writes RGBA8, render samples it)
        let output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("output-texture"),
            size: wgpu::Extent3d {
                width,
                height,
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
        let grid_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("grid-buffer"),
            size: (self.grid_data.len() * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&grid_buffer, 0, bytemuck::cast_slice(&self.grid_data));

        // --- Compute pipeline ---
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
                ],
            });

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
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
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
            compute_pipeline,
            compute_bind_group,
            render_pipeline,
            render_bind_group,
            output_texture,
            camera_buffer,
            grid_buffer,
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

        self.camera.screen_w = width as f32;
        self.camera.screen_h = height as f32;

        // Recreate output texture at new size
        gfx.output_texture = gfx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("output-texture"),
            size: wgpu::Extent3d {
                width,
                height,
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
            ],
        });

        let render_bgl = gfx.render_pipeline.get_bind_group_layout(0);
        let sampler = gfx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("blit-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
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
        // Advance time
        let now = Instant::now();
        let dt = now.elapsed_secs_since(&self.last_frame_time);
        self.last_frame_time = now;

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
                ui.label(egui::RichText::new("v9").color(egui::Color32::from_rgba_premultiplied(200, 200, 200, 180)).size(14.0));
            });

        let mut time_val = self.time_of_day;
        let mut paused = self.time_paused;
        let mut speed = self.time_speed;
        let mut zoom = self.camera.zoom;
        let base_zoom = self.camera.screen_h / GRID_H as f32;
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
            });
        self.time_of_day = time_val;
        self.time_paused = paused;
        self.time_speed = speed;
        self.camera.zoom = zoom;

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

        // Compute pass: raytrace
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("raytrace-pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&gfx.compute_pipeline);
            cpass.set_bind_group(0, &gfx.compute_bind_group, &[]);
            let wg_x = (gfx.config.width + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
            let wg_y = (gfx.config.height + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
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

        let attrs = Window::default_attributes()
            .with_title("Spacewestern")
            .with_inner_size(PhysicalSize::new(2560u32, 1600u32));

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
            attrs.with_canvas(Some(canvas))
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
                let base_zoom = self.camera.screen_h / GRID_H as f32;
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
                        self.camera.center_x -= dx as f32 / self.camera.zoom;
                        self.camera.center_y -= dy as f32 / self.camera.zoom;
                        self.window.as_ref().unwrap().request_redraw();
                    }
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
