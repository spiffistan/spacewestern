use anyhow::Result;
use ash::vk;

/// Simple sphere data for building acceleration structures
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Sphere {
    pub center: [f32; 3],
    pub radius: f32,
}

/// Wrapper around Vulkan acceleration structure resources
pub struct AccelerationStructure {
    pub handle: vk::AccelerationStructureKHR,
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub device_address: vk::DeviceAddress,
}

/// AABB (Axis-Aligned Bounding Box) for procedural geometry
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AabbPositions {
    pub min_x: f32,
    pub min_y: f32,
    pub min_z: f32,
    pub max_x: f32,
    pub max_y: f32,
    pub max_z: f32,
}

impl AabbPositions {
    pub fn from_sphere(sphere: &Sphere) -> Self {
        Self {
            min_x: sphere.center[0] - sphere.radius,
            min_y: sphere.center[1] - sphere.radius,
            min_z: sphere.center[2] - sphere.radius,
            max_x: sphere.center[0] + sphere.radius,
            max_y: sphere.center[1] + sphere.radius,
            max_z: sphere.center[2] + sphere.radius,
        }
    }
}

pub struct AccelerationStructureBuilder {
    accel_struct_loader: ash::khr::acceleration_structure::Device,
}

impl AccelerationStructureBuilder {
    pub fn new(instance: &ash::Instance, device: &ash::Device) -> Self {
        Self {
            accel_struct_loader: ash::khr::acceleration_structure::Device::new(instance, device),
        }
    }

    /// Create a bottom-level acceleration structure for procedural AABBs (spheres)
    pub fn build_blas_for_aabbs(
        &self,
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        aabbs: &[AabbPositions],
    ) -> Result<AccelerationStructure> {
        // Create AABB buffer
        let aabb_buffer_size = std::mem::size_of_val(aabbs) as vk::DeviceSize;
        let (aabb_buffer, aabb_memory) = create_buffer(
            instance,
            device,
            physical_device,
            aabb_buffer_size,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        // Upload AABB data
        unsafe {
            let ptr = device.map_memory(aabb_memory, 0, aabb_buffer_size, vk::MemoryMapFlags::empty())?;
            std::ptr::copy_nonoverlapping(aabbs.as_ptr() as *const u8, ptr as *mut u8, aabb_buffer_size as usize);
            device.unmap_memory(aabb_memory);
        }

        let aabb_device_address = unsafe {
            device.get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(aabb_buffer))
        };

        // Build geometry
        let aabb_data = vk::AccelerationStructureGeometryAabbsDataKHR::default()
            .data(vk::DeviceOrHostAddressConstKHR {
                device_address: aabb_device_address,
            })
            .stride(std::mem::size_of::<AabbPositions>() as vk::DeviceSize);

        let geometry = vk::AccelerationStructureGeometryKHR::default()
            .geometry_type(vk::GeometryTypeKHR::AABBS)
            .geometry(vk::AccelerationStructureGeometryDataKHR { aabbs: aabb_data })
            .flags(vk::GeometryFlagsKHR::OPAQUE);

        let geometries = [geometry];
        let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::default()
            .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .geometries(&geometries);

        let primitive_count = aabbs.len() as u32;
        let mut size_info = vk::AccelerationStructureBuildSizesInfoKHR::default();
        unsafe {
            self.accel_struct_loader.get_acceleration_structure_build_sizes(
                vk::AccelerationStructureBuildTypeKHR::DEVICE,
                &build_info,
                &[primitive_count],
                &mut size_info,
            );
        }

        // Create acceleration structure buffer
        let (as_buffer, as_memory) = create_buffer(
            instance,
            device,
            physical_device,
            size_info.acceleration_structure_size,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        // Create acceleration structure
        let as_create_info = vk::AccelerationStructureCreateInfoKHR::default()
            .buffer(as_buffer)
            .size(size_info.acceleration_structure_size)
            .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL);

        let accel_struct = unsafe {
            self.accel_struct_loader
                .create_acceleration_structure(&as_create_info, None)?
        };

        // Create scratch buffer
        let (scratch_buffer, scratch_memory) = create_buffer(
            instance,
            device,
            physical_device,
            size_info.build_scratch_size,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let scratch_address = unsafe {
            device.get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(scratch_buffer))
        };

        // Build the acceleration structure
        let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::default()
            .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .dst_acceleration_structure(accel_struct)
            .geometries(&geometries)
            .scratch_data(vk::DeviceOrHostAddressKHR {
                device_address: scratch_address,
            });

        let build_range = vk::AccelerationStructureBuildRangeInfoKHR::default()
            .primitive_count(primitive_count)
            .primitive_offset(0)
            .first_vertex(0)
            .transform_offset(0);

        let build_range_slice: &[vk::AccelerationStructureBuildRangeInfoKHR] = std::slice::from_ref(&build_range);

        // Record and submit build command
        let cmd_buffer = begin_single_time_commands(device, command_pool)?;
        unsafe {
            self.accel_struct_loader
                .cmd_build_acceleration_structures(cmd_buffer, &[build_info], &[build_range_slice]);
        }
        end_single_time_commands(device, command_pool, queue, cmd_buffer)?;

        // Get device address
        let as_device_address = unsafe {
            self.accel_struct_loader.get_acceleration_structure_device_address(
                &vk::AccelerationStructureDeviceAddressInfoKHR::default()
                    .acceleration_structure(accel_struct),
            )
        };

        // Cleanup scratch and aabb buffers
        unsafe {
            device.destroy_buffer(scratch_buffer, None);
            device.free_memory(scratch_memory, None);
            device.destroy_buffer(aabb_buffer, None);
            device.free_memory(aabb_memory, None);
        }

        Ok(AccelerationStructure {
            handle: accel_struct,
            buffer: as_buffer,
            memory: as_memory,
            device_address: as_device_address,
        })
    }

    /// Create a top-level acceleration structure
    pub fn build_tlas(
        &self,
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        blas: &AccelerationStructure,
        instance_count: u32,
    ) -> Result<AccelerationStructure> {
        // Create instance buffer
        let transform = vk::TransformMatrixKHR {
            matrix: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
            ],
        };

        let as_instance = vk::AccelerationStructureInstanceKHR {
            transform,
            instance_custom_index_and_mask: vk::Packed24_8::new(0, 0xFF),
            instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                0,
                vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as u8,
            ),
            acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                device_handle: blas.device_address,
            },
        };

        let instance_buffer_size = std::mem::size_of::<vk::AccelerationStructureInstanceKHR>() as vk::DeviceSize;
        let (instance_buffer, instance_memory) = create_buffer(
            instance,
            device,
            physical_device,
            instance_buffer_size,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        unsafe {
            let ptr = device.map_memory(instance_memory, 0, instance_buffer_size, vk::MemoryMapFlags::empty())?;
            std::ptr::copy_nonoverlapping(
                &as_instance as *const _ as *const u8,
                ptr as *mut u8,
                instance_buffer_size as usize,
            );
            device.unmap_memory(instance_memory);
        }

        let instance_address = unsafe {
            device.get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(instance_buffer))
        };

        let instances_data = vk::AccelerationStructureGeometryInstancesDataKHR::default()
            .array_of_pointers(false)
            .data(vk::DeviceOrHostAddressConstKHR {
                device_address: instance_address,
            });

        let geometry = vk::AccelerationStructureGeometryKHR::default()
            .geometry_type(vk::GeometryTypeKHR::INSTANCES)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                instances: instances_data,
            });

        let geometries = [geometry];
        let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::default()
            .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .geometries(&geometries);

        let mut size_info = vk::AccelerationStructureBuildSizesInfoKHR::default();
        unsafe {
            self.accel_struct_loader.get_acceleration_structure_build_sizes(
                vk::AccelerationStructureBuildTypeKHR::DEVICE,
                &build_info,
                &[instance_count],
                &mut size_info,
            );
        }

        let (as_buffer, as_memory) = create_buffer(
            instance,
            device,
            physical_device,
            size_info.acceleration_structure_size,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let as_create_info = vk::AccelerationStructureCreateInfoKHR::default()
            .buffer(as_buffer)
            .size(size_info.acceleration_structure_size)
            .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL);

        let accel_struct = unsafe {
            self.accel_struct_loader
                .create_acceleration_structure(&as_create_info, None)?
        };

        let (scratch_buffer, scratch_memory) = create_buffer(
            instance,
            device,
            physical_device,
            size_info.build_scratch_size,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let scratch_address = unsafe {
            device.get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(scratch_buffer))
        };

        let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::default()
            .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .dst_acceleration_structure(accel_struct)
            .geometries(&geometries)
            .scratch_data(vk::DeviceOrHostAddressKHR {
                device_address: scratch_address,
            });

        let build_range = vk::AccelerationStructureBuildRangeInfoKHR::default()
            .primitive_count(instance_count)
            .primitive_offset(0)
            .first_vertex(0)
            .transform_offset(0);

        let build_range_slice: &[vk::AccelerationStructureBuildRangeInfoKHR] = std::slice::from_ref(&build_range);

        let cmd_buffer = begin_single_time_commands(device, command_pool)?;
        unsafe {
            self.accel_struct_loader
                .cmd_build_acceleration_structures(cmd_buffer, &[build_info], &[build_range_slice]);
        }
        end_single_time_commands(device, command_pool, queue, cmd_buffer)?;

        let as_device_address = unsafe {
            self.accel_struct_loader.get_acceleration_structure_device_address(
                &vk::AccelerationStructureDeviceAddressInfoKHR::default()
                    .acceleration_structure(accel_struct),
            )
        };

        unsafe {
            device.destroy_buffer(scratch_buffer, None);
            device.free_memory(scratch_memory, None);
            device.destroy_buffer(instance_buffer, None);
            device.free_memory(instance_memory, None);
        }

        Ok(AccelerationStructure {
            handle: accel_struct,
            buffer: as_buffer,
            memory: as_memory,
            device_address: as_device_address,
        })
    }

    pub fn destroy(&self, device: &ash::Device, accel_struct: &AccelerationStructure) {
        unsafe {
            self.accel_struct_loader
                .destroy_acceleration_structure(accel_struct.handle, None);
            device.destroy_buffer(accel_struct.buffer, None);
            device.free_memory(accel_struct.memory, None);
        }
    }
}

fn create_buffer(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let buffer_info = vk::BufferCreateInfo::default()
        .size(size)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let buffer = unsafe { device.create_buffer(&buffer_info, None)? };
    let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
    let mem_properties = unsafe { instance.get_physical_device_memory_properties(physical_device) };

    let memory_type_index = find_memory_type(mem_requirements.memory_type_bits, properties, &mem_properties)?;

    let mut alloc_flags_info =
        vk::MemoryAllocateFlagsInfo::default().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);

    let alloc_info = vk::MemoryAllocateInfo::default()
        .allocation_size(mem_requirements.size)
        .memory_type_index(memory_type_index)
        .push_next(&mut alloc_flags_info);

    let memory = unsafe { device.allocate_memory(&alloc_info, None)? };
    unsafe { device.bind_buffer_memory(buffer, memory, 0)? };

    Ok((buffer, memory))
}

fn find_memory_type(
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
    mem_properties: &vk::PhysicalDeviceMemoryProperties,
) -> Result<u32> {
    for i in 0..mem_properties.memory_type_count {
        if (type_filter & (1 << i)) != 0
            && mem_properties.memory_types[i as usize]
                .property_flags
                .contains(properties)
        {
            return Ok(i);
        }
    }
    anyhow::bail!("Failed to find suitable memory type")
}

fn begin_single_time_commands(device: &ash::Device, command_pool: vk::CommandPool) -> Result<vk::CommandBuffer> {
    let alloc_info = vk::CommandBufferAllocateInfo::default()
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);

    let cmd_buffer = unsafe { device.allocate_command_buffers(&alloc_info)?[0] };

    let begin_info = vk::CommandBufferBeginInfo::default()
        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    unsafe { device.begin_command_buffer(cmd_buffer, &begin_info)? };

    Ok(cmd_buffer)
}

fn end_single_time_commands(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    cmd_buffer: vk::CommandBuffer,
) -> Result<()> {
    unsafe {
        device.end_command_buffer(cmd_buffer)?;

        let cmd_buffers = [cmd_buffer];
        let submit_info = vk::SubmitInfo::default().command_buffers(&cmd_buffers);

        device.queue_submit(queue, &[submit_info], vk::Fence::null())?;
        device.queue_wait_idle(queue)?;
        device.free_command_buffers(command_pool, &[cmd_buffer]);
    }

    Ok(())
}
