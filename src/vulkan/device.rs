use anyhow::Result;
use ash::vk;
use std::ffi::CStr;

/// Information about ray tracing support on the device
#[derive(Debug, Clone)]
pub struct RayTracingSupport {
    pub supported: bool,
    pub properties: Option<RayTracingProperties>,
}

#[derive(Debug, Clone)]
pub struct RayTracingProperties {
    pub shader_group_handle_size: u32,
    pub max_ray_recursion_depth: u32,
    pub shader_group_base_alignment: u32,
}

/// Check if the device supports hardware ray tracing
pub fn check_ray_tracing_support(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
) -> RayTracingSupport {
    let extensions = unsafe {
        instance
            .enumerate_device_extension_properties(physical_device)
            .unwrap_or_default()
    };

    let has_rt_pipeline = extensions.iter().any(|ext| {
        let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
        name == vk::KHR_RAY_TRACING_PIPELINE_NAME
    });

    let has_accel_struct = extensions.iter().any(|ext| {
        let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
        name == vk::KHR_ACCELERATION_STRUCTURE_NAME
    });

    let has_deferred_host = extensions.iter().any(|ext| {
        let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
        name == vk::KHR_DEFERRED_HOST_OPERATIONS_NAME
    });

    if has_rt_pipeline && has_accel_struct && has_deferred_host {
        // Get ray tracing properties
        let mut rt_pipeline_props = vk::PhysicalDeviceRayTracingPipelinePropertiesKHR::default();
        let mut props2 = vk::PhysicalDeviceProperties2::default().push_next(&mut rt_pipeline_props);

        unsafe {
            instance.get_physical_device_properties2(physical_device, &mut props2);
        }

        RayTracingSupport {
            supported: true,
            properties: Some(RayTracingProperties {
                shader_group_handle_size: rt_pipeline_props.shader_group_handle_size,
                max_ray_recursion_depth: rt_pipeline_props.max_ray_recursion_depth,
                shader_group_base_alignment: rt_pipeline_props.shader_group_base_alignment,
            }),
        }
    } else {
        RayTracingSupport {
            supported: false,
            properties: None,
        }
    }
}

pub fn pick_physical_device(
    instance: &ash::Instance,
    surface_loader: &ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
) -> Result<(vk::PhysicalDevice, u32)> {
    let devices = unsafe { instance.enumerate_physical_devices()? };
    for device in devices {
        let queues = unsafe { instance.get_physical_device_queue_family_properties(device) };
        for (i, q) in queues.iter().enumerate() {
            let supports_graphics = q.queue_flags.contains(vk::QueueFlags::GRAPHICS);
            let supports_surface = unsafe {
                surface_loader.get_physical_device_surface_support(device, i as u32, surface)?
            };
            if supports_graphics && supports_surface {
                return Ok((device, i as u32));
            }
        }
    }
    anyhow::bail!("No suitable device")
}

pub fn create_logical_device(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    queue_family_index: u32,
    enable_ray_tracing: bool,
) -> Result<(ash::Device, vk::Queue)> {
    let priorities = [1.0];
    let queue_info = vk::DeviceQueueCreateInfo::default()
        .queue_family_index(queue_family_index)
        .queue_priorities(&priorities);

    let queue_infos = [queue_info];

    // Build extension list based on platform and RT support
    let mut device_extensions: Vec<*const i8> = vec![vk::KHR_SWAPCHAIN_NAME.as_ptr()];

    // Check if portability subset is needed (macOS)
    let available_extensions = unsafe {
        instance
            .enumerate_device_extension_properties(physical_device)
            .unwrap_or_default()
    };

    let has_portability = available_extensions.iter().any(|ext| {
        let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
        name == vk::KHR_PORTABILITY_SUBSET_NAME
    });

    if has_portability {
        device_extensions.push(vk::KHR_PORTABILITY_SUBSET_NAME.as_ptr());
    }

    // Add ray tracing extensions if enabled
    if enable_ray_tracing {
        device_extensions.push(vk::KHR_RAY_TRACING_PIPELINE_NAME.as_ptr());
        device_extensions.push(vk::KHR_ACCELERATION_STRUCTURE_NAME.as_ptr());
        device_extensions.push(vk::KHR_DEFERRED_HOST_OPERATIONS_NAME.as_ptr());
        device_extensions.push(vk::KHR_BUFFER_DEVICE_ADDRESS_NAME.as_ptr());
        device_extensions.push(vk::KHR_SPIRV_1_4_NAME.as_ptr());
        device_extensions.push(vk::KHR_SHADER_FLOAT_CONTROLS_NAME.as_ptr());
    }

    // Enable required features for ray tracing
    let mut buffer_device_address_features =
        vk::PhysicalDeviceBufferDeviceAddressFeatures::default().buffer_device_address(true);

    let mut accel_struct_features =
        vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default().acceleration_structure(true);

    let mut rt_pipeline_features =
        vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::default().ray_tracing_pipeline(true);

    let mut device_create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_infos)
        .enabled_extension_names(&device_extensions);

    if enable_ray_tracing {
        device_create_info = device_create_info
            .push_next(&mut buffer_device_address_features)
            .push_next(&mut accel_struct_features)
            .push_next(&mut rt_pipeline_features);
    }

    let device = unsafe { instance.create_device(physical_device, &device_create_info, None)? };

    let queue = unsafe { device.get_device_queue(queue_family_index, 0) };
    Ok((device, queue))
}
