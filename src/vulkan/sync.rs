use anyhow::Result;
use ash::vk;

pub struct SyncObjects {
    pub image_available: vk::Semaphore,
    pub render_finished: vk::Semaphore,
    pub in_flight_fence: vk::Fence,
}

impl SyncObjects {
    pub fn new(device: &ash::Device) -> Result<Self> {
        Ok(Self {
            image_available: create_semaphore(device)?,
            render_finished: create_semaphore(device)?,
            in_flight_fence: create_fence(device)?,
        })
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_fence(self.in_flight_fence, None);
            device.destroy_semaphore(self.render_finished, None);
            device.destroy_semaphore(self.image_available, None);
        }
    }
}

pub fn create_semaphore(device: &ash::Device) -> Result<vk::Semaphore> {
    let semaphore_info = vk::SemaphoreCreateInfo::default();
    let semaphore = unsafe { device.create_semaphore(&semaphore_info, None)? };
    Ok(semaphore)
}

pub fn create_fence(device: &ash::Device) -> Result<vk::Fence> {
    let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
    let fence = unsafe { device.create_fence(&fence_info, None)? };
    Ok(fence)
}
