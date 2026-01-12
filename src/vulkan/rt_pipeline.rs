use anyhow::Result;
use ash::vk;
use std::ffi::CStr;

use super::RayTracingProperties;

pub struct RayTracingPipeline {
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_set: vk::DescriptorSet,
    pub sbt_buffer: vk::Buffer,
    pub sbt_memory: vk::DeviceMemory,
    pub raygen_region: vk::StridedDeviceAddressRegionKHR,
    pub miss_region: vk::StridedDeviceAddressRegionKHR,
    pub hit_region: vk::StridedDeviceAddressRegionKHR,
    pub callable_region: vk::StridedDeviceAddressRegionKHR,
    rt_pipeline_loader: ash::khr::ray_tracing_pipeline::Device,
}

impl RayTracingPipeline {
    pub fn new(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        storage_image_view: vk::ImageView,
        tlas: vk::AccelerationStructureKHR,
        rt_props: &RayTracingProperties,
    ) -> Result<Self> {
        let rt_pipeline_loader = ash::khr::ray_tracing_pipeline::Device::new(instance, device);

        // Create descriptor set layout
        let bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR | vk::ShaderStageFlags::CLOSEST_HIT_KHR),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR),
        ];

        let layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
        let descriptor_set_layout = unsafe { device.create_descriptor_set_layout(&layout_info, None)? };

        // Create pipeline layout
        let set_layouts = [descriptor_set_layout];
        let push_constant_ranges = [vk::PushConstantRange::default()
            .stage_flags(
                vk::ShaderStageFlags::RAYGEN_KHR
                    | vk::ShaderStageFlags::CLOSEST_HIT_KHR
                    | vk::ShaderStageFlags::INTERSECTION_KHR,
            )
            .offset(0)
            .size(16)]; // time, width, height, padding

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(&push_constant_ranges);

        let pipeline_layout = unsafe { device.create_pipeline_layout(&pipeline_layout_info, None)? };

        // Load shaders
        let rgen_code = include_bytes!("../../shaders/spv/raygen.rgen.spv");
        let rmiss_code = include_bytes!("../../shaders/spv/miss.rmiss.spv");
        let rchit_code = include_bytes!("../../shaders/spv/closesthit.rchit.spv");
        let rint_code = include_bytes!("../../shaders/spv/intersection.rint.spv");

        let rgen_module = create_shader_module(device, rgen_code)?;
        let rmiss_module = create_shader_module(device, rmiss_code)?;
        let rchit_module = create_shader_module(device, rchit_code)?;
        let rint_module = create_shader_module(device, rint_code)?;

        let entry_name = unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") };

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::RAYGEN_KHR)
                .module(rgen_module)
                .name(entry_name),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::MISS_KHR)
                .module(rmiss_module)
                .name(entry_name),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::CLOSEST_HIT_KHR)
                .module(rchit_module)
                .name(entry_name),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::INTERSECTION_KHR)
                .module(rint_module)
                .name(entry_name),
        ];

        // Shader groups
        let shader_groups = [
            // Ray generation group
            vk::RayTracingShaderGroupCreateInfoKHR::default()
                .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(0)
                .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                .any_hit_shader(vk::SHADER_UNUSED_KHR)
                .intersection_shader(vk::SHADER_UNUSED_KHR),
            // Miss group
            vk::RayTracingShaderGroupCreateInfoKHR::default()
                .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(1)
                .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                .any_hit_shader(vk::SHADER_UNUSED_KHR)
                .intersection_shader(vk::SHADER_UNUSED_KHR),
            // Hit group (procedural with intersection shader)
            vk::RayTracingShaderGroupCreateInfoKHR::default()
                .ty(vk::RayTracingShaderGroupTypeKHR::PROCEDURAL_HIT_GROUP)
                .general_shader(vk::SHADER_UNUSED_KHR)
                .closest_hit_shader(2)
                .any_hit_shader(vk::SHADER_UNUSED_KHR)
                .intersection_shader(3),
        ];

        let pipeline_info = vk::RayTracingPipelineCreateInfoKHR::default()
            .stages(&shader_stages)
            .groups(&shader_groups)
            .max_pipeline_ray_recursion_depth(1)
            .layout(pipeline_layout);

        let pipeline = unsafe {
            rt_pipeline_loader
                .create_ray_tracing_pipelines(vk::DeferredOperationKHR::null(), vk::PipelineCache::null(), &[pipeline_info], None)
                .map_err(|e| e.1)?[0]
        };

        // Cleanup shader modules
        unsafe {
            device.destroy_shader_module(rgen_module, None);
            device.destroy_shader_module(rmiss_module, None);
            device.destroy_shader_module(rchit_module, None);
            device.destroy_shader_module(rint_module, None);
        }

        // Create shader binding table
        let handle_size = rt_props.shader_group_handle_size;
        let handle_alignment = rt_props.shader_group_base_alignment;
        let handle_size_aligned = align_up(handle_size, handle_alignment);

        let group_count = 3u32;
        let sbt_size = (handle_size_aligned * group_count) as vk::DeviceSize;

        // Get shader group handles
        let handles = unsafe {
            rt_pipeline_loader.get_ray_tracing_shader_group_handles(
                pipeline,
                0,
                group_count,
                (handle_size * group_count) as usize,
            )?
        };

        // Create SBT buffer
        let (sbt_buffer, sbt_memory) = create_sbt_buffer(
            instance,
            device,
            physical_device,
            sbt_size,
        )?;

        // Upload shader handles to SBT
        unsafe {
            let ptr = device.map_memory(sbt_memory, 0, sbt_size, vk::MemoryMapFlags::empty())?;
            let mut offset = 0usize;
            for i in 0..group_count as usize {
                let src_offset = i * handle_size as usize;
                std::ptr::copy_nonoverlapping(
                    handles[src_offset..src_offset + handle_size as usize].as_ptr(),
                    (ptr as *mut u8).add(offset),
                    handle_size as usize,
                );
                offset += handle_size_aligned as usize;
            }
            device.unmap_memory(sbt_memory);
        }

        let sbt_address = unsafe {
            device.get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(sbt_buffer))
        };

        let raygen_region = vk::StridedDeviceAddressRegionKHR {
            device_address: sbt_address,
            stride: handle_size_aligned as vk::DeviceSize,
            size: handle_size_aligned as vk::DeviceSize,
        };

        let miss_region = vk::StridedDeviceAddressRegionKHR {
            device_address: sbt_address + handle_size_aligned as vk::DeviceSize,
            stride: handle_size_aligned as vk::DeviceSize,
            size: handle_size_aligned as vk::DeviceSize,
        };

        let hit_region = vk::StridedDeviceAddressRegionKHR {
            device_address: sbt_address + (handle_size_aligned * 2) as vk::DeviceSize,
            stride: handle_size_aligned as vk::DeviceSize,
            size: handle_size_aligned as vk::DeviceSize,
        };

        let callable_region = vk::StridedDeviceAddressRegionKHR::default();

        // Create descriptor pool
        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                .descriptor_count(1),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(1),
        ];

        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(1)
            .pool_sizes(&pool_sizes);

        let descriptor_pool = unsafe { device.create_descriptor_pool(&pool_info, None)? };

        // Allocate descriptor set
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&set_layouts);

        let descriptor_set = unsafe { device.allocate_descriptor_sets(&alloc_info)?[0] };

        // Update descriptor set
        let accel_structs = [tlas];
        let mut accel_struct_write = vk::WriteDescriptorSetAccelerationStructureKHR::default()
            .acceleration_structures(&accel_structs);

        let as_write = vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
            .descriptor_count(1)
            .push_next(&mut accel_struct_write);

        let image_info = [vk::DescriptorImageInfo::default()
            .image_view(storage_image_view)
            .image_layout(vk::ImageLayout::GENERAL)];

        let image_write = vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set)
            .dst_binding(1)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(&image_info);

        unsafe {
            device.update_descriptor_sets(&[as_write, image_write], &[]);
        }

        Ok(Self {
            pipeline,
            pipeline_layout,
            descriptor_set_layout,
            descriptor_pool,
            descriptor_set,
            sbt_buffer,
            sbt_memory,
            raygen_region,
            miss_region,
            hit_region,
            callable_region,
            rt_pipeline_loader,
        })
    }

    pub fn trace_rays(&self, command_buffer: vk::CommandBuffer, width: u32, height: u32) {
        unsafe {
            self.rt_pipeline_loader.cmd_trace_rays(
                command_buffer,
                &self.raygen_region,
                &self.miss_region,
                &self.hit_region,
                &self.callable_region,
                width,
                height,
                1,
            );
        }
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_buffer(self.sbt_buffer, None);
            device.free_memory(self.sbt_memory, None);
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}

fn create_shader_module(device: &ash::Device, code: &[u8]) -> Result<vk::ShaderModule> {
    let code_u32: Vec<u32> = code
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();

    let create_info = vk::ShaderModuleCreateInfo::default().code(&code_u32);
    let module = unsafe { device.create_shader_module(&create_info, None)? };
    Ok(module)
}

fn create_sbt_buffer(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    size: vk::DeviceSize,
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let buffer_info = vk::BufferCreateInfo::default()
        .size(size)
        .usage(
            vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        )
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let buffer = unsafe { device.create_buffer(&buffer_info, None)? };
    let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
    let mem_properties = unsafe { instance.get_physical_device_memory_properties(physical_device) };

    let memory_type_index = find_memory_type(
        mem_requirements.memory_type_bits,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        &mem_properties,
    )?;

    let mut alloc_flags =
        vk::MemoryAllocateFlagsInfo::default().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);

    let alloc_info = vk::MemoryAllocateInfo::default()
        .allocation_size(mem_requirements.size)
        .memory_type_index(memory_type_index)
        .push_next(&mut alloc_flags);

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

fn align_up(value: u32, alignment: u32) -> u32 {
    (value + alignment - 1) & !(alignment - 1)
}
