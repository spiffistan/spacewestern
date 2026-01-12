use anyhow::Result;
use ash::{vk, Entry};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub fn create_surface(
    entry: &Entry,
    instance: &ash::Instance,
    window: &winit::window::Window,
) -> Result<vk::SurfaceKHR> {
    let surface = unsafe {
        ash_window::create_surface(
            entry,
            instance,
            window.display_handle()?.as_raw(),
            window.window_handle()?.as_raw(),
            None,
        )?
    };
    Ok(surface)
}
