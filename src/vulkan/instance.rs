use anyhow::{Context, Result};
use ash::{vk, Entry};
use std::ffi::CString;

const VALIDATION_LAYERS: [&str; 0] = [];

pub fn create_instance(entry: &Entry) -> Result<ash::Instance> {
    let app_name = CString::new("rt_game")?;
    let engine_name = CString::new("rt_engine")?;

    let app_info = vk::ApplicationInfo::default()
        .application_name(&app_name)
        .application_version(0)
        .engine_name(&engine_name)
        .engine_version(0)
        .api_version(vk::API_VERSION_1_3);

    let layer_cstrings: Vec<CString> = VALIDATION_LAYERS
        .iter()
        .map(|l| CString::new(*l).unwrap())
        .collect();

    let layer_ptrs: Vec<*const i8> =
        layer_cstrings.iter().map(|l| l.as_ptr()).collect();

    let mut extension_names: Vec<*const i8> = Vec::new();

    // Required for macOS/MoltenVK portability
    extension_names.push(vk::KHR_PORTABILITY_ENUMERATION_NAME.as_ptr());
    extension_names.push(vk::KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_NAME.as_ptr());

    // Required for surface creation
    extension_names.push(vk::KHR_SURFACE_NAME.as_ptr());
    extension_names.push(vk::EXT_METAL_SURFACE_NAME.as_ptr());

    let create_info = vk::InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_layer_names(&layer_ptrs)
        .enabled_extension_names(&extension_names)
        .flags(vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR);

    let instance = unsafe {
        entry
            .create_instance(&create_info, None)
            .context("Failed to create Vulkan instance")?
    };

    Ok(instance)
}
