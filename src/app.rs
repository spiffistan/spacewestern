use anyhow::Result;
use ash::{vk, Entry};

use crate::vulkan::{
    create_instance, create_logical_device, create_surface, pick_physical_device,
    Commands, Swapchain, SyncObjects,
};

pub struct VulkanApp {
    entry: Entry,
    instance: ash::Instance,
    surface: vk::SurfaceKHR,
    surface_loader: ash::khr::surface::Instance,
    device: ash::Device,
    queue: vk::Queue,
    swapchain: Swapchain,
    commands: Commands,
    sync: SyncObjects,
}

impl VulkanApp {
    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let entry = unsafe { Entry::load()? };
        let instance = create_instance(&entry)?;
        let surface = create_surface(&entry, &instance, window)?;
        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);

        let (physical, family) = pick_physical_device(&instance, &surface_loader, surface)?;
        let (device, queue) = create_logical_device(&instance, physical, family, false)?;

        let swapchain = Swapchain::new(
            &entry,
            &instance,
            &device,
            physical,
            surface,
            family,
        )?;

        let commands = Commands::new(&device, family, &swapchain)?;
        let sync = SyncObjects::new(&device)?;

        Ok(Self {
            entry,
            instance,
            surface,
            surface_loader,
            device,
            queue,
            swapchain,
            commands,
            sync,
        })
    }

    pub fn draw_frame(&mut self) {
        unsafe {
            self.device.wait_for_fences(&[self.sync.in_flight_fence], true, u64::MAX).ok();
            self.device.reset_fences(&[self.sync.in_flight_fence]).ok();

            let image_index = self.swapchain.acquire_next_image(self.sync.image_available).unwrap();

            let wait_semaphores = [self.sync.image_available];
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let command_buffers = [self.commands.buffers()[image_index as usize]];
            let signal_semaphores = [self.sync.render_finished];

            let submit_info = vk::SubmitInfo::default()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&command_buffers)
                .signal_semaphores(&signal_semaphores);

            self.device
                .queue_submit(self.queue, &[submit_info], self.sync.in_flight_fence)
                .unwrap();

            self.swapchain.present(self.queue, &signal_semaphores, image_index).unwrap();
        }
    }

    pub fn shutdown(&mut self) {
        unsafe {
            self.device.device_wait_idle().ok();
        }
    }
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().ok();
            self.sync.destroy(&self.device);
            self.commands.destroy(&self.device);
            self.swapchain.destroy(&self.device);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}
