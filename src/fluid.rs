//! Fluid simulation — parameters and obstacle field generation.

use crate::grid::GRID_W;

pub const FLUID_SIM_W: u32 = 256;
pub const FLUID_SIM_H: u32 = 256;
pub const FLUID_DYE_W: u32 = 512;
pub const FLUID_DYE_H: u32 = 512;
pub const FLUID_PRESSURE_ITERS: u32 = 35;

/// Fluid simulation uniform. Must match FluidParams in all fluid .wgsl files.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FluidParams {
    pub sim_w: f32, pub sim_h: f32, pub dye_w: f32, pub dye_h: f32,
    pub dt: f32, pub dissipation: f32, pub vorticity_strength: f32, pub pressure_iterations: f32,
    pub splat_x: f32, pub splat_y: f32, pub splat_vx: f32, pub splat_vy: f32,
    pub splat_radius: f32, pub splat_active: f32, pub time: f32, pub wind_x: f32,
    pub wind_y: f32, pub smoke_rate: f32, pub fan_speed: f32, pub _pad3: f32,
}

/// Build the obstacle field (256x256 u8) from the block grid.
/// 255 = solid obstacle, 0 = open.
pub fn build_obstacle_field(grid: &[u32]) -> Vec<u8> {
    grid.iter().map(|&b| {
        let bt = b & 0xFF;
        let bh = (b >> 8) & 0xFF;
        let is_door = (b >> 16) & 1 != 0;
        let is_open = (b >> 16) & 4 != 0;
        if bh > 0 && bt != 8 && bt != 6 && bt != 7 && bt != 10 && bt != 11 && bt != 12 && bt != 13 && !(is_door && is_open) { 255 } else { 0 }
    }).collect()
}

/// Smoothstep interpolation.
pub fn smoothstep_f32(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Convert IEEE 754 half-precision float to f32.
pub fn half_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) & 1) as u32;
    let exp = ((h >> 10) & 0x1F) as u32;
    let mant = (h & 0x3FF) as u32;
    if exp == 0 {
        if mant == 0 { return if sign == 1 { -0.0 } else { 0.0 }; }
        let v = (mant as f32) / 1024.0 * 2.0f32.powi(-14);
        return if sign == 1 { -v } else { v };
    }
    if exp == 31 { return if mant == 0 { f32::INFINITY } else { f32::NAN }; }
    let v = 2.0f32.powi(exp as i32 - 15) * (1.0 + mant as f32 / 1024.0);
    if sign == 1 { -v } else { v }
}
