//! GPU initialization — wgpu setup, pipeline creation, bind groups.
//! Extracted from main.rs to keep it manageable.

use crate::*;
use crate::grid::generate_water_table;

impl App {
    pub(crate) async fn init_gfx_async(&mut self, window: Arc<Window>) {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let render_w = ((width as f32) * self.render_scale).max(1.0) as u32;
        let render_h = ((height as f32) * self.render_scale).max(1.0) as u32;
        self.camera.screen_w = render_w as f32;
        self.camera.screen_h = render_h as f32;
        // Zoom to show ~64 blocks (the houses area), not the full map
        let view_size = 64.0f32; // default zoom (wider view)
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
                    required_limits: {
                        let mut limits = wgpu::Limits::downlevel_defaults()
                            .using_resolution(adapter.limits());
                        limits.max_storage_buffers_per_shader_stage = 8;
                        limits
                    },
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
        self.pipe_network.rebuild(&self.grid_data);
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

        // Material table buffer
        let material_data = build_material_table();
        let material_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("material-buffer"),
            size: (material_data.len() * std::mem::size_of::<GpuMaterial>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&material_buffer, 0, bytemuck::cast_slice(&material_data));

        // Pleb storage buffer (up to MAX_PLEBS, updated each frame)
        let pleb_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pleb-buffer"),
            size: (MAX_PLEBS * std::mem::size_of::<GpuPleb>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Block temperature buffer (256x256 f32, initialized to 15°C ambient)
        let voltage_data = vec![0.0f32; (GRID_W * GRID_H) as usize];
        let voltage_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("voltage-buffer"),
            size: (voltage_data.len() * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        queue.write_buffer(&voltage_buffer, 0, bytemuck::cast_slice(&voltage_data));

        // Pipe flow direction buffer (2 f32 per tile: flow_x, flow_y)
        let pipe_flow_data = vec![0.0f32; (GRID_W * GRID_H * 2) as usize];
        let pipe_flow_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pipe-flow-buffer"),
            size: (pipe_flow_data.len() * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let block_temp_data = vec![15.0f32; (GRID_W * GRID_H) as usize];
        let block_temp_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("block-temp-buffer"),
            size: (block_temp_data.len() * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        queue.write_buffer(&block_temp_buffer, 0, bytemuck::cast_slice(&block_temp_data));

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
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/lightmap.wgsl").into()),
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
                // Material table
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
                // Voltage buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
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
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&lm_view_a) },
                wgpu::BindGroupEntry { binding: 1, resource: camera_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: grid_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: material_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: voltage_buffer.as_entire_binding() },
            ],
        });
        let lightmap_seed_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lightmap-seed-bg-b"),
            layout: &seed_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&lm_view_b) },
                wgpu::BindGroupEntry { binding: 1, resource: camera_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: grid_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: material_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: voltage_buffer.as_entire_binding() },
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
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/lightmap_propagate.wgsl").into()),
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
                // binding 4: material table
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
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
                wgpu::BindGroupEntry { binding: 4, resource: material_buffer.as_entire_binding() },
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
                wgpu::BindGroupEntry { binding: 4, resource: material_buffer.as_entire_binding() },
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

        let fluid_vel_a = make_fluid_tex("fluid-vel-a", FLUID_SIM_MAX, FLUID_SIM_MAX, wgpu::TextureFormat::Rg32Float);
        let fluid_vel_b = make_fluid_tex("fluid-vel-b", FLUID_SIM_MAX, FLUID_SIM_MAX, wgpu::TextureFormat::Rg32Float);
        let fluid_pres_a = make_fluid_tex("fluid-pres-a", FLUID_SIM_MAX, FLUID_SIM_MAX, wgpu::TextureFormat::R32Float);
        let fluid_pres_b = make_fluid_tex("fluid-pres-b", FLUID_SIM_MAX, FLUID_SIM_MAX, wgpu::TextureFormat::R32Float);
        let fluid_div = make_fluid_tex("fluid-div", FLUID_SIM_MAX, FLUID_SIM_MAX, wgpu::TextureFormat::R32Float);
        let fluid_curl_tex = make_fluid_tex("fluid-curl", FLUID_SIM_MAX, FLUID_SIM_MAX, wgpu::TextureFormat::R32Float);
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
            size: wgpu::Extent3d { width: GRID_W, height: GRID_H, depth_or_array_layers: 1 },
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
                wgpu::BindGroupLayoutEntry { binding: 7, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
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
                wgpu::BindGroupLayoutEntry { binding: 6, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        // --- Fluid shader modules ---
        let fluid_sim_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fluid-sim"), source: wgpu::ShaderSource::Wgsl(include_str!("shaders/fluid.wgsl").into()),
        });
        let fluid_pressure_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fluid-pressure"), source: wgpu::ShaderSource::Wgsl(include_str!("shaders/fluid_pressure.wgsl").into()),
        });
        let fluid_dye_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fluid-dye"), source: wgpu::ShaderSource::Wgsl(include_str!("shaders/fluid_dye.wgsl").into()),
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
                wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::TextureView(&fv_dye_a) },
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
                wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::TextureView(&fv_dye_a) },
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
                wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::TextureView(&fv_dye_a) },
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
                wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::TextureView(&fv_dye_a) },
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
                wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::TextureView(&fv_dye_a) },
            ],
        });
        // advect_vel: reads vel_B → writes vel_A (uses dye for buoyancy)
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
                wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::TextureView(&fv_dye_a) },
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
                wgpu::BindGroupEntry { binding: 6, resource: block_temp_buffer.as_entire_binding() },
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
                wgpu::BindGroupEntry { binding: 6, resource: block_temp_buffer.as_entire_binding() },
            ],
        });

        // --- Raytrace compute pipeline (now also reads the lightmap) ---
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("raytrace-compute"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/raytrace.wgsl").into()),
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
                    // Material table (storage buffer, read-only)
                    wgpu::BindGroupLayoutEntry {
                        binding: 11,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Pleb data buffer (storage buffer, read-only)
                    wgpu::BindGroupLayoutEntry {
                        binding: 12,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Block temperature buffer (storage buffer, read-only)
                    wgpu::BindGroupLayoutEntry {
                        binding: 13,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Voltage buffer (storage buffer, read-only)
                    wgpu::BindGroupLayoutEntry {
                        binding: 14,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Pipe flow buffer (storage buffer, read-only)
                    wgpu::BindGroupLayoutEntry {
                        binding: 15,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Water level texture (read-only)
                    wgpu::BindGroupLayoutEntry {
                        binding: 16,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Water table buffer (read-only storage)
                    wgpu::BindGroupLayoutEntry {
                        binding: 17,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Shadow map texture (pre-computed per grid cell, sampled with bilinear)
                    wgpu::BindGroupLayoutEntry {
                        binding: 18,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Sound pressure texture (wave equation output)
                    wgpu::BindGroupLayoutEntry {
                        binding: 19,
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

        // Water texture view for raytrace shader (created early for bind group)
        let water_tex_desc_early = wgpu::TextureDescriptor {
            label: Some("water-level-a"),
            size: wgpu::Extent3d { width: GRID_W, height: GRID_H, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        };
        let water_a = device.create_texture(&water_tex_desc_early);
        let water_b = device.create_texture(&wgpu::TextureDescriptor { label: Some("water-level-b"), ..water_tex_desc_early });
        let water_zeros = vec![0u8; (GRID_W * GRID_H * 4) as usize];
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &water_a, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &water_zeros, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(GRID_W * 4), rows_per_image: Some(GRID_H) },
            wgpu::Extent3d { width: GRID_W, height: GRID_H, depth_or_array_layers: 1 },
        );
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &water_b, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &water_zeros, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(GRID_W * 4), rows_per_image: Some(GRID_H) },
            wgpu::Extent3d { width: GRID_W, height: GRID_H, depth_or_array_layers: 1 },
        );
        let fv_water_a = water_a.create_view(&wgpu::TextureViewDescriptor::default());
        let fv_water_b = water_b.create_view(&wgpu::TextureViewDescriptor::default());

        // Water table: static height map generated from terrain
        let water_table_data = generate_water_table(&self.grid_data);
        self.water_table = water_table_data.clone();
        let water_table_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("water-table"),
            size: (GRID_W * GRID_H * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&water_table_buffer, 0, bytemuck::cast_slice(&self.water_table));

        let water_readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("water-readback"),
            size: 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Fluid dye sampler (bilinear for smooth smoke overlay)
        let fluid_dye_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("fluid-dye-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Shadow map texture (8× grid resolution max for sub-tile shadow detail)
        const SHADOW_MAP_MAX_SCALE: u32 = 8;
        let shadow_map_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow-map"),
            size: wgpu::Extent3d { width: GRID_W * SHADOW_MAP_MAX_SCALE, height: GRID_H * SHADOW_MAP_MAX_SCALE, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_map_sample_view = shadow_map_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Sound textures (created early for raytrace bind group)
        let sound_tex_a_early = make_fluid_tex("sound-a", GRID_W, GRID_H, wgpu::TextureFormat::Rg32Float);
        let sound_tex_b_early = make_fluid_tex("sound-b", GRID_W, GRID_H, wgpu::TextureFormat::Rg32Float);
        let sound_sample_view = sound_tex_a_early.create_view(&wgpu::TextureViewDescriptor::default());

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
                wgpu::BindGroupEntry { binding: 11, resource: material_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 12, resource: pleb_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 13, resource: block_temp_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 14, resource: voltage_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 15, resource: pipe_flow_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 16, resource: wgpu::BindingResource::TextureView(&fv_water_a) },
                wgpu::BindGroupEntry { binding: 17, resource: water_table_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 18, resource: wgpu::BindingResource::TextureView(&shadow_map_sample_view) },
                wgpu::BindGroupEntry { binding: 19, resource: wgpu::BindingResource::TextureView(&sound_sample_view) },
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
                wgpu::BindGroupEntry { binding: 11, resource: material_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 12, resource: pleb_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 13, resource: block_temp_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 14, resource: voltage_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 15, resource: pipe_flow_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 16, resource: wgpu::BindingResource::TextureView(&fv_water_a) },
                wgpu::BindGroupEntry { binding: 17, resource: water_table_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 18, resource: wgpu::BindingResource::TextureView(&shadow_map_sample_view) },
                wgpu::BindGroupEntry { binding: 19, resource: wgpu::BindingResource::TextureView(&sound_sample_view) },
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
                wgpu::BindGroupEntry { binding: 11, resource: material_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 12, resource: pleb_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 13, resource: block_temp_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 14, resource: voltage_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 15, resource: pipe_flow_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 16, resource: wgpu::BindingResource::TextureView(&fv_water_a) },
                wgpu::BindGroupEntry { binding: 17, resource: water_table_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 18, resource: wgpu::BindingResource::TextureView(&shadow_map_sample_view) },
                wgpu::BindGroupEntry { binding: 19, resource: wgpu::BindingResource::TextureView(&sound_sample_view) },
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
                wgpu::BindGroupEntry { binding: 11, resource: material_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 12, resource: pleb_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 13, resource: block_temp_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 14, resource: voltage_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 15, resource: pipe_flow_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 16, resource: wgpu::BindingResource::TextureView(&fv_water_a) },
                wgpu::BindGroupEntry { binding: 17, resource: water_table_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 18, resource: wgpu::BindingResource::TextureView(&shadow_map_sample_view) },
                wgpu::BindGroupEntry { binding: 19, resource: wgpu::BindingResource::TextureView(&sound_sample_view) },
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
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/blit.wgsl").into()),
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
        let block_temp_readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("block-temp-readback"),
            size: 256, // only need 4 bytes but alignment
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Full voltage readback buffer for per-tile labels (only used when power overlay active)
        let voltage_readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("voltage-readback"),
            size: (GRID_W * GRID_H * 4) as u64, // 256x256 f32
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Per-pleb air readback: 16 plebs × 256 bytes each (alignment)
        let pleb_air_readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pleb-air-readback"),
            size: 16 * 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        self.egui_state = Some(EguiState {
            ctx: egui_ctx,
            winit_state: egui_winit_state,
            renderer: egui_renderer,
        });

        // --- Thermal exchange pipeline ---
        let thermal_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("thermal-compute"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/thermal.wgsl").into()),
        });
        let thermal_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("thermal-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            ],
        });
        // Use dye texture A for temperature readback (current frame's dye)
        let thermal_bind_group_val = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("thermal-bg"),
            layout: &thermal_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: block_temp_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: camera_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: grid_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: material_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&fv_dye_a) },
            ],
        });
        let thermal_pipeline_val = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("thermal-pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("thermal-pl"),
                bind_group_layouts: &[&thermal_bgl],
                push_constant_ranges: &[],
            })),
            module: &thermal_shader,
            entry_point: Some("main_thermal"),
            compilation_options: Default::default(),
            cache: None,
        });

        // --- Shadow map pre-pass pipeline ---
        let shadow_map_write_view = shadow_map_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow-map-compute"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shadow_map.wgsl").into()),
        });
        let shadow_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow-map-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format: wgpu::TextureFormat::Rgba8Unorm, view_dimension: wgpu::TextureViewDimension::D2 }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });
        let shadow_map_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow-map-bg"),
            layout: &shadow_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&shadow_map_write_view) },
                wgpu::BindGroupEntry { binding: 1, resource: camera_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: grid_buffer.as_entire_binding() },
            ],
        });
        let shadow_map_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("shadow-map-pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("shadow-map-pl"),
                bind_group_layouts: &[&shadow_bgl],
                push_constant_ranges: &[],
            })),
            module: &shadow_shader,
            entry_point: Some("main_shadow"),
            compilation_options: Default::default(),
            cache: None,
        });

        // --- Power grid pipeline ---
        let power_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("power-compute"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/power.wgsl").into()),
        });
        let power_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("power-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });
        let power_bind_group_val = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("power-bg"),
            layout: &power_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: voltage_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: camera_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: grid_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: block_temp_buffer.as_entire_binding() },
            ],
        });
        let power_pipeline_val = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("power-pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("power-pl"),
                bind_group_layouts: &[&power_bgl],
                push_constant_ranges: &[],
            })),
            module: &power_shader,
            entry_point: Some("main_power"),
            compilation_options: Default::default(),
            cache: None,
        });

        // --- Ground water simulation pipeline (textures already created above) ---
        let water_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("water-compute"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/water.wgsl").into()),
        });
        let water_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("water-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format: wgpu::TextureFormat::R32Float, view_dimension: wgpu::TextureViewDimension::D2 }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });
        let water_bg_ab = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("water-bg-ab"), layout: &water_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&fv_water_a) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&fv_water_b) },
                wgpu::BindGroupEntry { binding: 2, resource: grid_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: camera_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: water_table_buffer.as_entire_binding() },
            ],
        });
        let water_bg_ba = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("water-bg-ba"), layout: &water_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&fv_water_b) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&fv_water_a) },
                wgpu::BindGroupEntry { binding: 2, resource: grid_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: camera_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: water_table_buffer.as_entire_binding() },
            ],
        });
        let water_pipeline_val = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("water-pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("water-pl"), bind_group_layouts: &[&water_bgl], push_constant_ranges: &[],
            })),
            module: &water_shader,
            entry_point: Some("main_water"),
            compilation_options: Default::default(),
            cache: None,
        });

        // --- Sound wave propagation pipeline ---
        let sound_view_a = sound_tex_a_early.create_view(&wgpu::TextureViewDescriptor::default());
        let sound_view_b = sound_tex_b_early.create_view(&wgpu::TextureViewDescriptor::default());
        let sound_source_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sound-sources"),
            size: (1 + 16 * 8) * 4, // count + up to 16 sources × 8 f32
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sound_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sound-compute"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sound.wgsl").into()),
        });
        let sound_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sound-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format: wgpu::TextureFormat::Rg32Float, view_dimension: wgpu::TextureViewDimension::D2 }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });
        // Ping-pong: bg[0] reads A writes B, bg[1] reads B writes A
        let sound_bg_0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sound-bg-0"), layout: &sound_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&sound_view_a) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&sound_view_b) },
                wgpu::BindGroupEntry { binding: 2, resource: grid_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: camera_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: sound_source_buffer.as_entire_binding() },
            ],
        });
        let sound_bg_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sound-bg-1"), layout: &sound_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&sound_view_b) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&sound_view_a) },
                wgpu::BindGroupEntry { binding: 2, resource: grid_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: camera_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: sound_source_buffer.as_entire_binding() },
            ],
        });
        let sound_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("sound-pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("sound-pl"), bind_group_layouts: &[&sound_bgl], push_constant_ranges: &[],
            })),
            module: &sound_shader,
            entry_point: Some("main_sound"),
            compilation_options: Default::default(),
            cache: None,
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
            material_buffer,
            pleb_buffer,
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
            block_temp_readback_buffer,
            voltage_readback_buffer,
            pleb_air_readback_buffer,
            block_temp_buffer,
            thermal_pipeline: thermal_pipeline_val,
            thermal_bind_group: thermal_bind_group_val,
            shadow_map_texture,
            shadow_map_pipeline,
            shadow_map_bind_group,
            sound_textures: [sound_tex_a_early, sound_tex_b_early],
            sound_pipeline,
            sound_bind_groups: [sound_bg_0, sound_bg_1],
            sound_source_buffer,
            voltage_buffer,
            pipe_flow_buffer,
            power_pipeline: power_pipeline_val,
            power_bind_group: power_bind_group_val,
            water_textures: [water_a, water_b],
            water_table_buffer,
            water_readback_buffer,
            water_pipeline: water_pipeline_val,
            water_bind_groups: [water_bg_ab, water_bg_ba],
        });

        self.window = Some(window);
    }
}
