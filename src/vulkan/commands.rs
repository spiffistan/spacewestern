use anyhow::Result;
use ash::vk;

use super::Swapchain;

pub struct Commands {
    pool: vk::CommandPool,
    buffers: Vec<vk::CommandBuffer>,
}

impl Commands {
    pub fn new(device: &ash::Device, queue_family_index: u32, swapchain: &Swapchain) -> Result<Self> {
        let pool = create_command_pool(device, queue_family_index)?;
        let buffers = allocate_command_buffers(device, pool, swapchain.images().len() as u32)?;
        Ok(Self { pool, buffers })
    }

    pub fn pool(&self) -> vk::CommandPool {
        self.pool
    }

    pub fn buffers(&self) -> &[vk::CommandBuffer] {
        &self.buffers
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_command_pool(self.pool, None);
        }
    }
}

pub fn create_command_pool(
    device: &ash::Device,
    queue_family_index: u32,
) -> Result<vk::CommandPool> {
    let pool_info = vk::CommandPoolCreateInfo::default()
        .queue_family_index(queue_family_index)
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

    let command_pool = unsafe { device.create_command_pool(&pool_info, None)? };
    Ok(command_pool)
}

pub fn allocate_command_buffers(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    count: u32,
) -> Result<Vec<vk::CommandBuffer>> {
    let alloc_info = vk::CommandBufferAllocateInfo::default()
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(count);

    let command_buffers = unsafe { device.allocate_command_buffers(&alloc_info)? };
    Ok(command_buffers)
}

pub fn record_command_buffers(
    device: &ash::Device,
    command_buffers: &[vk::CommandBuffer],
    render_pass: &vk::RenderPass,
    framebuffers: &[vk::Framebuffer],
    extent: vk::Extent2D,
) -> Result<()> {
    for (i, &cmd_buf) in command_buffers.iter().enumerate() {
        let begin_info = vk::CommandBufferBeginInfo::default();
        unsafe { device.begin_command_buffer(cmd_buf, &begin_info)? };

        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        }];

        let render_pass_begin_info = vk::RenderPassBeginInfo::default()
            .render_pass(*render_pass)
            .framebuffer(framebuffers[i])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            })
            .clear_values(&clear_values);

        unsafe {
            device.cmd_begin_render_pass(
                cmd_buf,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );
            device.cmd_end_render_pass(cmd_buf);
            device.end_command_buffer(cmd_buf)?;
        }
    }
    Ok(())
}
