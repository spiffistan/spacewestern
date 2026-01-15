use anyhow::Result;
use ash::vk;
use std::ffi::CStr;

pub struct ComputePipeline {
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_set: vk::DescriptorSet,
}

impl ComputePipeline {
    pub fn new(
        device: &ash::Device,
        storage_image_view: vk::ImageView,
    ) -> Result<Self> {
        // Create descriptor set layout
        let bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
        ];

        let layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);

        let descriptor_set_layout =
            unsafe { device.create_descriptor_set_layout(&layout_info, None)? };

        // Create pipeline layout
        let set_layouts = [descriptor_set_layout];
        let push_constant_ranges = [vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .offset(0)
            .size(std::mem::size_of::<PushConstants>() as u32)];

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(&push_constant_ranges);

        let pipeline_layout =
            unsafe { device.create_pipeline_layout(&pipeline_layout_info, None)? };

        // Load compute shader
        let shader_code = include_bytes!("../../shaders/spv/raytrace.comp.spv");
        let shader_code_u32: Vec<u32> = shader_code
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        let shader_module_info =
            vk::ShaderModuleCreateInfo::default().code(&shader_code_u32);

        let shader_module = unsafe { device.create_shader_module(&shader_module_info, None)? };

        let entry_name = unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") };
        let stage_info = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(shader_module)
            .name(entry_name);

        let compute_pipeline_info = vk::ComputePipelineCreateInfo::default()
            .stage(stage_info)
            .layout(pipeline_layout);

        let pipeline = unsafe {
            device
                .create_compute_pipelines(vk::PipelineCache::null(), &[compute_pipeline_info], None)
                .map_err(|e| e.1)?[0]
        };

        unsafe {
            device.destroy_shader_module(shader_module, None);
        }

        // Create descriptor pool
        let pool_sizes = [vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::STORAGE_IMAGE)
            .descriptor_count(1)];

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
        let image_info = [vk::DescriptorImageInfo::default()
            .image_view(storage_image_view)
            .image_layout(vk::ImageLayout::GENERAL)];

        let write = vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(&image_info);

        unsafe {
            device.update_descriptor_sets(&[write], &[]);
        }

        Ok(Self {
            pipeline,
            pipeline_layout,
            descriptor_set_layout,
            descriptor_pool,
            descriptor_set,
        })
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PushConstants {
    pub time: f32,
    pub width: u32,
    pub height: u32,
    pub zoom: f32,
    pub camera_x: f32,
    pub camera_z: f32,
    pub visible_layer: i32,  // -1 = show all, 0+ = show only up to this layer
    pub mouse_x: f32,        // Mouse X position in pixels
    pub mouse_y: f32,        // Mouse Y position in pixels
    pub is_dragging: u32,    // 1 if currently dragging a voxel
    pub drag_source_x: i32,  // Source voxel X (grid coords)
    pub drag_source_y: i32,  // Source voxel Y (grid coords)
    pub drag_source_z: i32,  // Source voxel Z (grid coords)
    // Number of active modifications
    pub num_removed: u32,
    pub num_placed: u32,
    // Padding to align ivec4 array to 16 bytes (offset 60 -> 64)
    pub _padding: u32,
    // Up to 8 removed voxels (x, y, z each)
    pub removed: [[i32; 4]; 8],  // [x, y, z, _padding] * 8
    // Up to 8 placed voxels (x, y, z, type each)
    pub placed: [[i32; 4]; 8],   // [x, y, z, type] * 8
}
