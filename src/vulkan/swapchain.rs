use anyhow::Result;
use ash::vk;

pub struct Swapchain {
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    format: vk::Format,
    extent: vk::Extent2D,
    swapchain_loader: ash::khr::swapchain::Device,
}

impl Swapchain {
    pub fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        queue_family_index: u32,
    ) -> Result<Self> {
        let surface_loader = ash::khr::surface::Instance::new(entry, instance);
        let swapchain_loader = ash::khr::swapchain::Device::new(instance, device);

        let surface_caps = unsafe {
            surface_loader.get_physical_device_surface_capabilities(physical_device, surface)?
        };
        let surface_formats = unsafe {
            surface_loader.get_physical_device_surface_formats(physical_device, surface)?
        };

        let format = surface_formats
            .iter()
            .find(|f| {
                f.format == vk::Format::B8G8R8A8_SRGB
                    && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .unwrap_or(&surface_formats[0]);

        let extent = surface_caps.current_extent;

        let image_count = surface_caps.min_image_count + 1;
        let image_count = if surface_caps.max_image_count > 0 {
            image_count.min(surface_caps.max_image_count)
        } else {
            image_count
        };

        let queue_family_indices = [queue_family_index];
        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&queue_family_indices)
            .pre_transform(surface_caps.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO)
            .clipped(true);

        let swapchain = unsafe { swapchain_loader.create_swapchain(&create_info, None)? };
        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };

        let image_views: Vec<vk::ImageView> = images
            .iter()
            .map(|&image| {
                let view_info = vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format.format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::IDENTITY,
                        g: vk::ComponentSwizzle::IDENTITY,
                        b: vk::ComponentSwizzle::IDENTITY,
                        a: vk::ComponentSwizzle::IDENTITY,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });
                unsafe { device.create_image_view(&view_info, None).unwrap() }
            })
            .collect();

        Ok(Self {
            swapchain,
            images,
            image_views,
            format: format.format,
            extent,
            swapchain_loader,
        })
    }

    pub fn handle(&self) -> vk::SwapchainKHR {
        self.swapchain
    }

    pub fn images(&self) -> &[vk::Image] {
        &self.images
    }

    pub fn image_views(&self) -> &[vk::ImageView] {
        &self.image_views
    }

    pub fn format(&self) -> vk::Format {
        self.format
    }

    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    pub fn acquire_next_image(&self, semaphore: vk::Semaphore) -> Result<u32> {
        let (index, _) = unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                semaphore,
                vk::Fence::null(),
            )?
        };
        Ok(index)
    }

    pub fn present(&self, queue: vk::Queue, wait_semaphores: &[vk::Semaphore], image_index: u32) -> Result<()> {
        let swapchains = [self.swapchain];
        let image_indices = [image_index];

        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        unsafe { self.swapchain_loader.queue_present(queue, &present_info)? };
        Ok(())
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            for &image_view in &self.image_views {
                device.destroy_image_view(image_view, None);
            }
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        }
    }
}

