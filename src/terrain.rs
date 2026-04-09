//! Sub-tile terrain elevation system (1024x1024 heightmap over 256x256 grid).
//!
//! Provides continuous elevation for smooth digging, water flow, and rendering.
//! See docs/dn/DN-016-terrain-elevation-and-water.md for full design.

use crate::grid::{GRID_H, GRID_W};

// --- Resolution ---
pub const ELEV_W: u32 = GRID_W * 2; // sub-tile elevation resolution
pub const ELEV_H: u32 = GRID_H * 2;
pub const ELEV_SCALE: u32 = 2; // sub-cells per tile

// --- Digging constants ---
pub const DIG_BRUSH_RADIUS: f32 = 1.8; // sub-cells (~0.45 tiles)
pub const DIG_DEPTH_PER_STROKE: f32 = 0.04; // elevation units per swing (with shovel)
pub const DIG_SPEED_SHOVEL: f32 = 1.0;
pub const DIG_SPEED_PICK: f32 = 0.7;
pub const DIG_SPEED_HANDS: f32 = 0.25;
pub const DIG_SPEED_WET: f32 = 0.5;
pub const DIG_DEPTH_PENALTY: f32 = 0.1; // -10% speed per 0.5 depth
pub const DIG_SKILL_BONUS: f32 = 0.08; // +8% speed per skill level
pub const DIRT_PER_VOLUME: f32 = 6.0; // dirt items per elevation unit removed

// --- Dig zone cross-section profiles ---
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum CrossProfile {
    Flat,
    #[default]
    VShape,
    UShape,
}

impl CrossProfile {
    /// Depth multiplier at normalized position across the zone width (0=edge, 0.5=center, 1=edge).
    pub fn depth_factor(self, t: f32) -> f32 {
        let centered = (t * 2.0 - 1.0).abs(); // 0 at center, 1 at edges
        match self {
            CrossProfile::Flat => 1.0,
            CrossProfile::VShape => 1.0 - centered * centered,
            CrossProfile::UShape => smoothstep(1.0, 0.35, centered),
        }
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Generate 1024x1024 elevation from the 256x256 grid-resolution elevation array.
/// Uses bicubic-style interpolation with fractal noise for natural sub-tile variation.
pub fn generate_elevation(elevation_256: &[f32]) -> Vec<f32> {
    let mut elev = vec![0.0f32; (ELEV_W * ELEV_H) as usize];

    for sy in 0..ELEV_H {
        for sx in 0..ELEV_W {
            // Map sub-cell to tile-space (0.5 offset to sample at sub-cell center)
            let tx = (sx as f32 + 0.5) / ELEV_SCALE as f32;
            let ty = (sy as f32 + 0.5) / ELEV_SCALE as f32;

            // Bilinear interpolation from 256x256
            let gx0 = (tx.floor() as i32).clamp(0, GRID_W as i32 - 1) as u32;
            let gy0 = (ty.floor() as i32).clamp(0, GRID_H as i32 - 1) as u32;
            let gx1 = (gx0 + 1).min(GRID_W - 1);
            let gy1 = (gy0 + 1).min(GRID_H - 1);
            let fx = tx - tx.floor();
            let fy = ty - ty.floor();

            let e00 = elevation_256[(gy0 * GRID_W + gx0) as usize];
            let e10 = elevation_256[(gy0 * GRID_W + gx1) as usize];
            let e01 = elevation_256[(gy1 * GRID_W + gx0) as usize];
            let e11 = elevation_256[(gy1 * GRID_W + gx1) as usize];

            let base = e00 * (1.0 - fx) * (1.0 - fy)
                + e10 * fx * (1.0 - fy)
                + e01 * (1.0 - fx) * fy
                + e11 * fx * fy;

            // Fractal noise for natural micro-variation
            let noise = hash_noise(sx, sy) * 0.015;

            elev[(sy * ELEV_W + sx) as usize] = base + noise;
        }
    }

    elev
}

/// Simple deterministic noise for sub-tile elevation variation.
fn hash_noise(x: u32, y: u32) -> f32 {
    let seed = x.wrapping_mul(73856093) ^ y.wrapping_mul(19349663);
    let h = seed.wrapping_mul(2654435761);
    (h & 0xFFFF) as f32 / 65535.0 * 2.0 - 1.0 // -1 to 1
}

/// Apply a single dig brush stroke at a world position.
/// Returns the amount of dirt produced (for resource generation).
pub fn apply_dig_stroke(
    elevation: &mut [f32],
    world_x: f32,
    world_y: f32,
    depth_per_stroke: f32,
    target_depth_at: impl Fn(u32, u32) -> f32, // target elevation at each sub-cell
) -> f32 {
    let cx = (world_x * ELEV_SCALE as f32) as i32;
    let cy = (world_y * ELEV_SCALE as f32) as i32;
    let r = DIG_BRUSH_RADIUS.ceil() as i32;
    let mut dirt = 0.0f32;

    for dy in -r..=r {
        for dx in -r..=r {
            let sx = cx + dx;
            let sy = cy + dy;
            if sx < 0 || sy < 0 || sx >= ELEV_W as i32 || sy >= ELEV_H as i32 {
                continue;
            }
            let dist = ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt();
            if dist > DIG_BRUSH_RADIUS {
                continue;
            }

            // Smooth falloff from center
            let falloff = smoothstep(DIG_BRUSH_RADIUS, 0.0, dist);
            let idx = (sy as u32 * ELEV_W + sx as u32) as usize;
            let target = target_depth_at(sx as u32, sy as u32);
            let remaining = (elevation[idx] - target).max(0.0);
            let dig = (depth_per_stroke * falloff).min(remaining);

            if dig > 0.0001 {
                elevation[idx] -= dig;
                dirt += dig;
            }
        }
    }

    dirt
}

// --- Fill/Berm constants ---
pub const FILL_BRUSH_RADIUS: f32 = 1.6; // slightly tighter than dig brush
pub const FILL_HEIGHT_PER_STROKE: f32 = 0.03; // elevation units per dump

/// Apply a single fill brush stroke at a world position (raises terrain).
/// Returns the amount of dirt consumed.
pub fn apply_fill_stroke(
    elevation: &mut [f32],
    world_x: f32,
    world_y: f32,
    height_per_stroke: f32,
    max_elevation_at: impl Fn(u32, u32) -> f32, // target max elevation at each sub-cell
) -> f32 {
    let cx = (world_x * ELEV_SCALE as f32) as i32;
    let cy = (world_y * ELEV_SCALE as f32) as i32;
    let r = FILL_BRUSH_RADIUS.ceil() as i32;
    let mut dirt_used = 0.0f32;

    for dy in -r..=r {
        for dx in -r..=r {
            let sx = cx + dx;
            let sy = cy + dy;
            if sx < 0 || sy < 0 || sx >= ELEV_W as i32 || sy >= ELEV_H as i32 {
                continue;
            }
            let dist = ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt();
            if dist > FILL_BRUSH_RADIUS {
                continue;
            }

            let falloff = smoothstep(FILL_BRUSH_RADIUS, 0.0, dist);
            let idx = (sy as u32 * ELEV_W + sx as u32) as usize;
            let max_h = max_elevation_at(sx as u32, sy as u32);
            let remaining = (max_h - elevation[idx]).max(0.0);
            let fill = (height_per_stroke * falloff).min(remaining);

            if fill > 0.0001 {
                elevation[idx] += fill;
                dirt_used += fill;
            }
        }
    }

    dirt_used
}

/// Sample elevation at a world position with bilinear interpolation (CPU-side).
pub fn sample_elevation(elevation: &[f32], world_x: f32, world_y: f32) -> f32 {
    let sx = world_x * ELEV_SCALE as f32 - 0.5;
    let sy = world_y * ELEV_SCALE as f32 - 0.5;
    let x0 = (sx.floor() as i32).clamp(0, ELEV_W as i32 - 1) as u32;
    let y0 = (sy.floor() as i32).clamp(0, ELEV_H as i32 - 1) as u32;
    let x1 = (x0 + 1).min(ELEV_W - 1);
    let y1 = (y0 + 1).min(ELEV_H - 1);
    let fx = sx - sx.floor();
    let fy = sy - sy.floor();

    let e00 = elevation[(y0 * ELEV_W + x0) as usize];
    let e10 = elevation[(y0 * ELEV_W + x1) as usize];
    let e01 = elevation[(y1 * ELEV_W + x0) as usize];
    let e11 = elevation[(y1 * ELEV_W + x1) as usize];

    e00 * (1.0 - fx) * (1.0 - fy) + e10 * fx * (1.0 - fy) + e01 * (1.0 - fx) * fy + e11 * fx * fy
}

/// Get the bounding sub-cell rectangle affected by a dig brush at world position.
/// Returns (min_sx, min_sy, max_sx, max_sy) in sub-cell coords, clamped to heightmap bounds.
pub fn dig_brush_bounds(world_x: f32, world_y: f32) -> (u32, u32, u32, u32) {
    let cx = (world_x * ELEV_SCALE as f32) as i32;
    let cy = (world_y * ELEV_SCALE as f32) as i32;
    let r = DIG_BRUSH_RADIUS.ceil() as i32;
    (
        (cx - r).max(0) as u32,
        (cy - r).max(0) as u32,
        ((cx + r) as u32).min(ELEV_W - 1),
        ((cy + r) as u32).min(ELEV_H - 1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_profile_v_shape() {
        // Center should be deepest
        assert!((CrossProfile::VShape.depth_factor(0.5) - 1.0).abs() < 0.01);
        // Edges should be zero
        assert!(CrossProfile::VShape.depth_factor(0.0) < 0.01);
        assert!(CrossProfile::VShape.depth_factor(1.0) < 0.01);
        // Quarter should be ~0.75
        let q = CrossProfile::VShape.depth_factor(0.25);
        assert!(q > 0.6 && q < 0.85, "quarter was {}", q);
    }

    #[test]
    fn cross_profile_u_shape() {
        // Center should be full depth
        assert!((CrossProfile::UShape.depth_factor(0.5) - 1.0).abs() < 0.01);
        // Edges taper
        assert!(CrossProfile::UShape.depth_factor(0.0) < 0.1);
        // But the flat bottom is wide — 30% in should be near full
        let inner = CrossProfile::UShape.depth_factor(0.35);
        assert!(inner > 0.8, "inner was {}", inner);
    }

    #[test]
    fn dig_stroke_produces_dirt() {
        let mut elev = vec![1.0f32; (ELEV_W * ELEV_H) as usize];
        let dirt = apply_dig_stroke(&mut elev, 128.0, 128.0, 0.1, |_, _| 0.0);
        assert!(dirt > 0.0, "should produce dirt");
        // Center should be lower than edges
        let center_idx = (128 * ELEV_SCALE * ELEV_W + 128 * ELEV_SCALE) as usize;
        assert!(elev[center_idx] < 1.0, "center should be dug");
    }

    #[test]
    fn dig_respects_target_depth() {
        let mut elev = vec![1.0f32; (ELEV_W * ELEV_H) as usize];
        // Target depth = 0.95, so max dig = 0.05
        for _ in 0..100 {
            apply_dig_stroke(&mut elev, 128.0, 128.0, 0.1, |_, _| 0.95);
        }
        let center_idx = (128 * ELEV_SCALE * ELEV_W + 128 * ELEV_SCALE) as usize;
        assert!(
            elev[center_idx] >= 0.94,
            "should not dig below target: {}",
            elev[center_idx]
        );
    }
}
