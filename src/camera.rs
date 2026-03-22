//! Camera uniform — shared between Rust and all WGSL shaders.
//! Must match the Camera struct in every .wgsl file exactly.

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub center_x: f32,
    pub center_y: f32,
    pub zoom: f32,
    pub show_roofs: f32,
    pub screen_w: f32,
    pub screen_h: f32,
    pub grid_w: f32,
    pub grid_h: f32,
    pub time: f32,
    pub glass_light_mul: f32,
    pub indoor_glow_mul: f32,
    pub light_bleed_mul: f32,
    pub foliage_opacity: f32,
    pub foliage_variation: f32,
    pub oblique_strength: f32,
    pub lm_vp_min_x: f32,
    pub lm_vp_min_y: f32,
    pub lm_vp_max_x: f32,
    pub lm_vp_max_y: f32,
    pub lm_scale: f32,
    pub fluid_overlay: f32,
    pub sun_dir_x: f32,
    pub sun_dir_y: f32,
    pub sun_elevation: f32,
    pub sun_intensity: f32,
    pub sun_color_r: f32,
    pub sun_color_g: f32,
    pub sun_color_b: f32,
    pub ambient_r: f32,
    pub ambient_g: f32,
    pub ambient_b: f32,
    pub enable_prox_glow: f32,
    pub enable_dir_bleed: f32,
    pub force_refresh: f32,
    pub pleb_x: f32,
    pub pleb_y: f32,
    pub pleb_angle: f32,
    pub pleb_selected: f32,
    pub pleb_torch: f32,
    pub pleb_headlight: f32,
    pub prev_center_x: f32,
    pub prev_center_y: f32,
    pub prev_zoom: f32,
    pub prev_time: f32,
    pub rain_intensity: f32,
    pub cloud_cover: f32,
    pub wind_magnitude: f32,
    pub wind_angle: f32,   // wind direction in radians
    pub use_shadow_map: f32,    // 1.0 = sample shadow map, 0.0 = per-pixel ray trace
    pub shadow_map_scale: f32,  // shadow map texels per grid cell (for UV mapping)
    pub sound_speed: f32,       // wave equation propagation speed (c)
    pub sound_damping: f32,     // wave equation energy loss per step
    pub sound_coupling: f32,    // strength of sound→gas velocity coupling
    pub _pad4_a: f32,
    pub _pad4_b: f32,
    pub _pad4_c: f32,
}
