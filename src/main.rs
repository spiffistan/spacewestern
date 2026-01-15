mod app;
mod input;
mod terrain;
mod vulkan;

use anyhow::{Context, Result};
use ash::{vk, Entry};
use std::time::Instant;
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::PhysicalKey,
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::WindowBuilder,
};

use input::InputState;
use terrain::{TerrainConfig, TerrainGenerator};

use vulkan::{
    check_ray_tracing_support, create_command_pool, create_fence, create_instance,
    create_logical_device, create_semaphore, pick_physical_device, AabbPositions,
    AccelerationStructure, AccelerationStructureBuilder, ComputePipeline, PushConstants,
    RayTracingPipeline, Sphere, StorageImage, Swapchain,
};

/// Renderer abstraction - either compute or hardware ray tracing
enum Renderer {
    Compute {
        pipeline: ComputePipeline,
    },
    HardwareRT {
        pipeline: RayTracingPipeline,
        as_builder: AccelerationStructureBuilder,
        blas: AccelerationStructure,
        tlas: AccelerationStructure,
    },
}

fn main() -> Result<()> {
    let event_loop = EventLoop::new().context("Failed to create event loop")?;
    let window = WindowBuilder::new()
        .with_title("RayWorld")
        .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
        .build(&event_loop)
        .context("Failed to create window")?;

    // --- Vulkan init ---
    let entry = unsafe { Entry::load()? };
    let instance = create_instance(&entry)?;

    println!("Vulkan instance created.");

    let surface = unsafe {
        ash_window::create_surface(
            &entry,
            &instance,
            window.display_handle()?.as_raw(),
            window.window_handle()?.as_raw(),
            None,
        )?
    };
    let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);

    let (physical_device, queue_family_index) =
        pick_physical_device(&instance, &surface_loader, surface)?;

    // Check for ray tracing support
    let rt_support = check_ray_tracing_support(&instance, physical_device);
    let use_hardware_rt = rt_support.supported;

    if use_hardware_rt {
        println!("Hardware ray tracing supported! Using RTX path.");
    } else {
        println!("Hardware ray tracing not supported. Using compute shader fallback.");
    }

    let (device, queue) =
        create_logical_device(&instance, physical_device, queue_family_index, use_hardware_rt)?;

    let swapchain = Swapchain::new(
        &entry,
        &instance,
        &device,
        physical_device,
        surface,
        queue_family_index,
    )?;

    let extent = swapchain.extent();

    // Create storage image for ray tracing output
    let storage_image = StorageImage::new(
        &instance,
        &device,
        physical_device,
        extent.width,
        extent.height,
    )?;

    let command_pool = create_command_pool(&device, queue_family_index)?;

    // Create renderer based on support
    let renderer = if use_hardware_rt {
        let rt_props = rt_support.properties.unwrap();
        let as_builder = AccelerationStructureBuilder::new(&instance, &device);

        // Create AABBs for voxel boxes (matching intersection shader)
        let aabbs = [
            // Main voxel cube
            AabbPositions {
                min_x: -0.5,
                min_y: -0.5,
                min_z: -3.5,
                max_x: 0.5,
                max_y: 0.5,
                max_z: -2.5,
            },
            // Ground plane as thin box
            AabbPositions {
                min_x: -100.0,
                min_y: -1.1,
                min_z: -100.0,
                max_x: 100.0,
                max_y: -1.0,
                max_z: 100.0,
            },
        ];

        let blas = as_builder.build_blas_for_aabbs(
            &instance,
            &device,
            physical_device,
            command_pool,
            queue,
            &aabbs,
        )?;

        let tlas = as_builder.build_tlas(
            &instance,
            &device,
            physical_device,
            command_pool,
            queue,
            &blas,
            1,
        )?;

        let pipeline = RayTracingPipeline::new(
            &instance,
            &device,
            physical_device,
            storage_image.view,
            tlas.handle,
            &rt_props,
        )?;

        Renderer::HardwareRT {
            pipeline,
            as_builder,
            blas,
            tlas,
        }
    } else {
        let pipeline = ComputePipeline::new(&device, storage_image.view)?;
        Renderer::Compute { pipeline }
    };

    // Allocate command buffer
    let alloc_info = vk::CommandBufferAllocateInfo::default()
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);
    let command_buffer = unsafe { device.allocate_command_buffers(&alloc_info)?[0] };

    let image_available = create_semaphore(&device)?;
    let render_finished = create_semaphore(&device)?;
    let fence = create_fence(&device)?;

    let start_time = Instant::now();

    // Input state
    let mut input = InputState::new();
    let mut last_frame_time = Instant::now();

    // Terrain generator for CPU-side raycasting
    let terrain = TerrainGenerator::new(TerrainConfig::default());

    // Track window size for ray calculations
    let mut window_width = extent.width as f32;
    let mut window_height = extent.height as f32;

    // Helper closure to create a ray from screen coordinates
    let create_ray_from_screen = |mouse_x: f32, mouse_y: f32, width: f32, height: f32, zoom: f32, camera_x: f32, camera_z: f32| -> ([f32; 3], [f32; 3]) {
        // Normalized coordinates (-1 to 1)
        let mut uv_x = (mouse_x + 0.5) / width * 2.0 - 1.0;
        let uv_y = (mouse_y + 0.5) / height * 2.0 - 1.0;
        uv_x *= width / height; // Aspect ratio correction

        // View size based on zoom
        let view_radius = 60.0 * zoom;
        let world_offset_x = uv_x * view_radius;
        let world_offset_y = uv_y * view_radius;

        // Camera setup (matching shader)
        let tilt_angle = 0.25_f32;
        let cam_forward = {
            let len = (1.0_f32 + tilt_angle * tilt_angle).sqrt();
            [0.0, -1.0 / len, tilt_angle / len]
        };
        let cam_right = [1.0_f32, 0.0, 0.0];
        // camUp = cross(camRight, camForward)
        let cam_up = [
            cam_right[1] * cam_forward[2] - cam_right[2] * cam_forward[1],
            cam_right[2] * cam_forward[0] - cam_right[0] * cam_forward[2],
            cam_right[0] * cam_forward[1] - cam_right[1] * cam_forward[0],
        ];
        // Normalize camUp
        let cam_up_len = (cam_up[0] * cam_up[0] + cam_up[1] * cam_up[1] + cam_up[2] * cam_up[2]).sqrt();
        let cam_up = [cam_up[0] / cam_up_len, cam_up[1] / cam_up_len, cam_up[2] / cam_up_len];

        let cam_height = 80.0 * zoom;

        // Orthographic ray origin
        let ray_origin = [
            camera_x + cam_right[0] * world_offset_x + cam_up[0] * world_offset_y,
            cam_height + cam_right[1] * world_offset_x + cam_up[1] * world_offset_y,
            camera_z + cam_right[2] * world_offset_x + cam_up[2] * world_offset_y,
        ];

        (ray_origin, cam_forward)
    };

    event_loop
        .run(move |event, elwt| {
            match event {
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::CloseRequested => {
                            unsafe {
                                device.device_wait_idle().ok();
                                device.destroy_fence(fence, None);
                                device.destroy_semaphore(image_available, None);
                                device.destroy_semaphore(render_finished, None);
                                device.destroy_command_pool(command_pool, None);

                                match &renderer {
                                    Renderer::Compute { pipeline } => {
                                        pipeline.destroy(&device);
                                    }
                                    Renderer::HardwareRT {
                                        pipeline,
                                        as_builder,
                                        blas,
                                        tlas,
                                    } => {
                                        pipeline.destroy(&device);
                                        as_builder.destroy(&device, tlas);
                                        as_builder.destroy(&device, blas);
                                    }
                                }

                                storage_image.destroy(&device);
                                swapchain.destroy(&device);
                                device.destroy_device(None);
                                surface_loader.destroy_surface(surface, None);
                                instance.destroy_instance(None);
                            }
                            elwt.exit();
                        }
                        WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    physical_key: PhysicalKey::Code(key_code),
                                    state,
                                    ..
                                },
                            ..
                        } => {
                            match state {
                                ElementState::Pressed => {
                                    input.key_pressed(key_code);
                                }
                                ElementState::Released => {
                                    input.key_released(key_code);
                                }
                            }
                        }
                        WindowEvent::MouseWheel { delta, .. } => {
                            let scroll_delta = match delta {
                                winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                                winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 50.0,
                            };
                            input.scroll(scroll_delta);
                        }
                        WindowEvent::MouseInput { state, button, .. } => {
                            let button_id = match button {
                                winit::event::MouseButton::Left => 0,
                                winit::event::MouseButton::Right => 1,
                                winit::event::MouseButton::Middle => 2,
                                _ => 3,
                            };
                            match state {
                                ElementState::Pressed => {
                                    input.mouse_pressed(button_id);

                                    // Left click - start voxel drag
                                    if button_id == 0 {
                                        let (ray_origin, ray_dir) = create_ray_from_screen(
                                            input.mouse_x,
                                            input.mouse_y,
                                            window_width,
                                            window_height,
                                            input.zoom,
                                            input.camera_x,
                                            input.camera_z,
                                        );

                                        if let Some((x, y, z, voxel_type)) = terrain.raycast(
                                            ray_origin,
                                            ray_dir,
                                            200.0,
                                            &input.removed_voxels,
                                            &input.placed_voxels,
                                        ) {
                                            println!("CLICK: Hit voxel at ({}, {}, {}), type={}", x, y, z, voxel_type);
                                            input.start_voxel_drag(x, y, z);
                                        } else {
                                            println!("CLICK: No voxel hit");
                                        }
                                    }
                                }
                                ElementState::Released => {
                                    // Left click release - complete voxel drag
                                    if button_id == 0 && input.dragged_voxel.active {
                                        let (ray_origin, ray_dir) = create_ray_from_screen(
                                            input.mouse_x,
                                            input.mouse_y,
                                            window_width,
                                            window_height,
                                            input.zoom,
                                            input.camera_x,
                                            input.camera_z,
                                        );

                                        // Find drop target (the voxel we're hovering over)
                                        if let Some((hit_x, _hit_y, hit_z, _)) = terrain.raycast(
                                            ray_origin,
                                            ray_dir,
                                            200.0,
                                            &input.removed_voxels,
                                            &input.placed_voxels,
                                        ) {
                                            // Get the source voxel type
                                            let source_pos = (
                                                input.dragged_voxel.source_x,
                                                input.dragged_voxel.source_y,
                                                input.dragged_voxel.source_z,
                                            );
                                            let source_type = if let Some(&vt) = input.placed_voxels.get(&source_pos) {
                                                vt
                                            } else {
                                                terrain.get_voxel(source_pos.0, source_pos.1, source_pos.2) as u8
                                            };

                                            // Place on the ground at target X,Z position
                                            let ground_y = terrain.get_height(hit_x as f32, hit_z as f32).ceil() as i32;
                                            input.complete_voxel_drag(hit_x, ground_y, hit_z, source_type);
                                        } else {
                                            input.cancel_voxel_drag();
                                        }
                                    }
                                    input.mouse_released(button_id);
                                }
                            }
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            input.mouse_moved(position.x as f32, position.y as f32);
                        }
                        WindowEvent::Resized(size) => {
                            window_width = size.width as f32;
                            window_height = size.height as f32;
                        }
                        _ => {}
                    }
                }

                Event::AboutToWait => unsafe {
                    // Calculate delta time
                    let now = Instant::now();
                    let delta_time = now.duration_since(last_frame_time).as_secs_f32();
                    last_frame_time = now;

                    // Update input state
                    input.update(delta_time);

                    device.wait_for_fences(&[fence], true, u64::MAX).ok();
                    device.reset_fences(&[fence]).ok();

                    let image_index = swapchain.acquire_next_image(image_available).unwrap();

                    // Reset and record command buffer
                    device
                        .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())
                        .unwrap();

                    let begin_info = vk::CommandBufferBeginInfo::default()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
                    device
                        .begin_command_buffer(command_buffer, &begin_info)
                        .unwrap();

                    // Transition storage image to GENERAL
                    storage_image.transition_layout(
                        &device,
                        command_buffer,
                        vk::ImageLayout::UNDEFINED,
                        vk::ImageLayout::GENERAL,
                    );

                    // Build removed and placed voxel arrays (up to 8 each)
                    let mut removed: [[i32; 4]; 8] = [[0; 4]; 8];
                    let mut num_removed = 0u32;
                    for (i, &(x, y, z)) in input.removed_voxels.iter().take(8).enumerate() {
                        removed[i] = [x, y, z, 0];
                        num_removed = (i + 1) as u32;
                    }

                    let mut placed: [[i32; 4]; 8] = [[0; 4]; 8];
                    let mut num_placed = 0u32;
                    for (i, (&(x, y, z), &vt)) in input.placed_voxels.iter().take(8).enumerate() {
                        placed[i] = [x, y, z, vt as i32];
                        num_placed = (i + 1) as u32;
                    }

                    let push_constants = PushConstants {
                        time: start_time.elapsed().as_secs_f32(),
                        width: extent.width,
                        height: extent.height,
                        zoom: input.zoom,
                        camera_x: input.camera_x,
                        camera_z: input.camera_z,
                        visible_layer: input.visible_layer,
                        mouse_x: input.mouse_x,
                        mouse_y: input.mouse_y,
                        is_dragging: if input.dragged_voxel.active { 1 } else { 0 },
                        drag_source_x: input.dragged_voxel.source_x,
                        drag_source_y: input.dragged_voxel.source_y,
                        drag_source_z: input.dragged_voxel.source_z,
                        num_removed,
                        num_placed,
                        _padding: 0,
                        removed,
                        placed,
                    };

                    // Debug: print when dragging or when there are modifications
                    static mut FRAME_COUNT: u64 = 0;
                    FRAME_COUNT += 1;
                    if push_constants.is_dragging == 1 || num_removed > 0 || num_placed > 0 {
                        if FRAME_COUNT % 60 == 0 {
                            println!("FRAME {}: is_dragging={}, drag_source=({},{},{}), num_removed={}, num_placed={}",
                                FRAME_COUNT, push_constants.is_dragging,
                                push_constants.drag_source_x, push_constants.drag_source_y, push_constants.drag_source_z,
                                num_removed, num_placed);
                        }
                    }

                    // Dispatch based on renderer type
                    match &renderer {
                        Renderer::Compute { pipeline } => {
                            device.cmd_bind_pipeline(
                                command_buffer,
                                vk::PipelineBindPoint::COMPUTE,
                                pipeline.pipeline,
                            );

                            device.cmd_bind_descriptor_sets(
                                command_buffer,
                                vk::PipelineBindPoint::COMPUTE,
                                pipeline.pipeline_layout,
                                0,
                                &[pipeline.descriptor_set],
                                &[],
                            );

                            device.cmd_push_constants(
                                command_buffer,
                                pipeline.pipeline_layout,
                                vk::ShaderStageFlags::COMPUTE,
                                0,
                                bytemuck::bytes_of(&push_constants),
                            );

                            let group_count_x = (extent.width + 15) / 16;
                            let group_count_y = (extent.height + 15) / 16;
                            device.cmd_dispatch(command_buffer, group_count_x, group_count_y, 1);
                        }
                        Renderer::HardwareRT { pipeline, .. } => {
                            device.cmd_bind_pipeline(
                                command_buffer,
                                vk::PipelineBindPoint::RAY_TRACING_KHR,
                                pipeline.pipeline,
                            );

                            device.cmd_bind_descriptor_sets(
                                command_buffer,
                                vk::PipelineBindPoint::RAY_TRACING_KHR,
                                pipeline.pipeline_layout,
                                0,
                                &[pipeline.descriptor_set],
                                &[],
                            );

                            device.cmd_push_constants(
                                command_buffer,
                                pipeline.pipeline_layout,
                                vk::ShaderStageFlags::RAYGEN_KHR
                                    | vk::ShaderStageFlags::CLOSEST_HIT_KHR
                                    | vk::ShaderStageFlags::INTERSECTION_KHR,
                                0,
                                bytemuck::bytes_of(&push_constants),
                            );

                            pipeline.trace_rays(command_buffer, extent.width, extent.height);
                        }
                    }

                    // Transition storage image to TRANSFER_SRC
                    storage_image.transition_layout(
                        &device,
                        command_buffer,
                        vk::ImageLayout::GENERAL,
                        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    );

                    // Transition swapchain image to TRANSFER_DST
                    let swapchain_barrier = vk::ImageMemoryBarrier::default()
                        .old_layout(vk::ImageLayout::UNDEFINED)
                        .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .image(swapchain.images()[image_index as usize])
                        .subresource_range(vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: 1,
                        })
                        .src_access_mask(vk::AccessFlags::empty())
                        .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);

                    device.cmd_pipeline_barrier(
                        command_buffer,
                        vk::PipelineStageFlags::TOP_OF_PIPE,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[swapchain_barrier],
                    );

                    // Copy storage image to swapchain image
                    let region = vk::ImageCopy::default()
                        .src_subresource(vk::ImageSubresourceLayers {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            mip_level: 0,
                            base_array_layer: 0,
                            layer_count: 1,
                        })
                        .dst_subresource(vk::ImageSubresourceLayers {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            mip_level: 0,
                            base_array_layer: 0,
                            layer_count: 1,
                        })
                        .extent(vk::Extent3D {
                            width: extent.width,
                            height: extent.height,
                            depth: 1,
                        });

                    device.cmd_copy_image(
                        command_buffer,
                        storage_image.image,
                        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                        swapchain.images()[image_index as usize],
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        &[region],
                    );

                    // Transition swapchain image to PRESENT_SRC
                    let present_barrier = vk::ImageMemoryBarrier::default()
                        .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .image(swapchain.images()[image_index as usize])
                        .subresource_range(vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: 1,
                        })
                        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                        .dst_access_mask(vk::AccessFlags::empty());

                    device.cmd_pipeline_barrier(
                        command_buffer,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[present_barrier],
                    );

                    device.end_command_buffer(command_buffer).unwrap();

                    // Submit - use appropriate wait stage based on renderer
                    let wait_semaphores = [image_available];
                    let wait_stages = match &renderer {
                        Renderer::Compute { .. } => [vk::PipelineStageFlags::COMPUTE_SHADER],
                        Renderer::HardwareRT { .. } => [vk::PipelineStageFlags::RAY_TRACING_SHADER_KHR],
                    };
                    let command_buffers_to_submit = [command_buffer];
                    let signal_semaphores = [render_finished];

                    let submit_info = vk::SubmitInfo::default()
                        .wait_semaphores(&wait_semaphores)
                        .wait_dst_stage_mask(&wait_stages)
                        .command_buffers(&command_buffers_to_submit)
                        .signal_semaphores(&signal_semaphores);

                    device.queue_submit(queue, &[submit_info], fence).unwrap();

                    swapchain
                        .present(queue, &signal_semaphores, image_index)
                        .unwrap();
                },
                _ => {}
            }
        })
        .context("Event loop error")?;

    Ok(())
}
