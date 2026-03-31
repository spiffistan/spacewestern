//! GPU dust simulation — separate from fluid sim.
//! Terrain-colored, wind-reactive, wall-blocked, slow-decaying.
//! See docs/dn/DN-013-communication-flocking.md for context.

pub const DUST_SIM_W: u32 = 512;
pub const DUST_SIM_H: u32 = 512;

/// GPU-side dust simulation parameters. Must match dust.wgsl exactly.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DustParams {
    pub grid_w: f32,
    pub grid_h: f32,
    pub dt: f32,
    pub decay_rate: f32,  // per-frame multiplier (~0.9992 for 15s half-life)
    pub diffusion: f32,   // neighbor blend rate (0.02 = slow)
    pub wind_follow: f32, // fraction of air velocity dust follows (0.8)
    pub wind_x: f32,
    pub wind_y: f32,
    pub storm_active: f32,  // 1.0 during dust storm, 0.0 otherwise
    pub storm_edge: f32,    // windward edge: 0=N, 1=E, 2=S, 3=W
    pub storm_density: f32, // injection strength per frame
    pub _pad: f32,
}

/// CPU-side dust injection request (written to GPU textures before compute pass).
#[derive(Clone, Debug)]
pub struct DustInjection {
    pub x: f32, // world position
    pub y: f32,
    pub radius: f32,  // in tiles
    pub density: f32, // injection strength
}

/// Compute the per-frame decay multiplier for a given half-life.
pub fn compute_decay_rate(half_life_seconds: f32, dt: f32) -> f32 {
    if dt <= 0.0 || half_life_seconds <= 0.0 {
        return 1.0;
    }
    let frames_per_halflife = half_life_seconds / dt;
    0.5f32.powf(1.0 / frames_per_halflife)
}

/// Determine windward edge from wind direction (0=N, 1=E, 2=S, 3=W).
pub fn windward_edge(wind_x: f32, wind_y: f32) -> f32 {
    if wind_x.abs() > wind_y.abs() {
        if wind_x > 0.0 { 3.0 } else { 1.0 } // wind blows east → enters from west
    } else {
        if wind_y > 0.0 { 0.0 } else { 2.0 } // wind blows south → enters from north
    }
}
