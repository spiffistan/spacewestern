//! Fluid simulation — parameters and obstacle field generation.

use crate::grid::*;

// Fluid simulation resolution. Textures always allocated at max (512x512).
// Runtime toggle switches effective dispatch resolution via FluidParams.sim_w/h.
pub const FLUID_SIM_MAX: u32 = 512; // texture allocation size
pub const FLUID_SIM_W: u32 = 256; // default dispatch resolution
pub const FLUID_SIM_H: u32 = 256;
pub const FLUID_DYE_W: u32 = 512;
pub const FLUID_DYE_H: u32 = 512;
#[cfg(not(target_arch = "wasm32"))]
pub const FLUID_PRESSURE_ITERS: u32 = 35;
#[cfg(target_arch = "wasm32")]
pub const FLUID_PRESSURE_ITERS: u32 = 20;

/// Fluid simulation uniform. Must match FluidParams in all fluid .wgsl files.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FluidParams {
    pub sim_w: f32,
    pub sim_h: f32,
    pub dye_w: f32,
    pub dye_h: f32,
    pub dt: f32,
    pub dissipation: f32,
    pub vorticity_strength: f32,
    pub pressure_iterations: f32,
    pub splat_x: f32,
    pub splat_y: f32,
    pub splat_vx: f32,
    pub splat_vy: f32,
    pub splat_radius: f32,
    pub splat_active: f32,
    pub time: f32,
    pub wind_x: f32,
    pub wind_y: f32,
    pub smoke_rate: f32,
    pub fan_speed: f32,
    pub rain_intensity: f32,
}

/// Build the obstacle field (256x256 u8) from the block grid.
/// 255 = solid obstacle, 0 = open.
pub fn build_obstacle_field(grid: &[u32], wall_data: &[u16]) -> Vec<u8> {
    grid.iter()
        .enumerate()
        .map(|(i, &b)| {
            // Wall_data walls are full obstacles (unless open door)
            if i < wall_data.len() && wall_data[i] != 0 {
                let wd = wall_data[i];
                let edges = wd & 0xF;
                let is_wd_door_open = (wd & WD_HAS_DOOR) != 0 && (wd & WD_DOOR_OPEN) != 0;
                // Full-thickness wall (all 4 edges) blocks gas completely
                if edges == 0xF && !is_wd_door_open {
                    return 255;
                }
                // Thin walls with edges: partial obstacle (treat as blocking for now)
                if edges != 0 && !is_wd_door_open {
                    return 255;
                }
            }
            let bt = b & 0xFF;
            let bh = (b >> 8) & 0xFF;
            let is_door = (b >> 16) & 1 != 0;
            let is_open = (b >> 16) & 4 != 0;
            let passable = bt_is!(
                bt,
                BT_TREE,
                BT_FIREPLACE,
                BT_CEILING_LIGHT,
                BT_FLOOR_LAMP,
                BT_TABLE_LAMP,
                BT_FAN,
                BT_COMPOST,
                BT_BERRY_BUSH,
                BT_CROP,
                BT_LIQUID_PIPE,
                BT_PIPE_BRIDGE,
                BT_LIQUID_INTAKE,
                BT_LIQUID_PUMP,
                BT_LIQUID_OUTPUT,
                BT_WIRE,
                BT_DIMMER,
                BT_BREAKER,
                BT_WIRE_BRIDGE,
                BT_RESTRICTOR
            ) || (bt >= BT_PIPE && bt <= BT_VALVE);
            let is_thin = bh > 0 && is_wall_block(bt) && thin_wall_is_walkable(b);
            if bh > 0 && !passable && !is_thin && !(is_door && is_open) {
                255
            } else {
                0
            }
        })
        .collect()
}

/// Smoothstep interpolation.
pub fn smoothstep_f32(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Convert f32 to IEEE 754 half-precision float (u16).
pub fn f32_to_f16(v: f32) -> u16 {
    let bits = v.to_bits();
    let sign = (bits >> 31) & 1;
    let exp = ((bits >> 23) & 0xFF) as i32 - 127;
    let mant = bits & 0x7FFFFF;
    if exp > 15 {
        return ((sign << 15) | (0x1F << 10)) as u16;
    } // inf
    if exp < -14 {
        return (sign << 15) as u16;
    } // zero/denorm
    let h_exp = (exp + 15) as u32;
    let h_mant = mant >> 13;
    ((sign << 15) | (h_exp << 10) | h_mant) as u16
}

/// Convert IEEE 754 half-precision float to f32.
pub fn half_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) & 1) as u32;
    let exp = ((h >> 10) & 0x1F) as u32;
    let mant = (h & 0x3FF) as u32;
    if exp == 0 {
        if mant == 0 {
            return if sign == 1 { -0.0 } else { 0.0 };
        }
        let v = (mant as f32) / 1024.0 * 2.0f32.powi(-14);
        return if sign == 1 { -v } else { v };
    }
    if exp == 31 {
        return if mant == 0 { f32::INFINITY } else { f32::NAN };
    }
    let v = 2.0f32.powi(exp as i32 - 15) * (1.0 + mant as f32 / 1024.0);
    if sign == 1 { -v } else { v }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::make_block;

    #[test]
    fn test_obstacle_walls_block() {
        let grid = vec![make_block(1, 3, 0)]; // stone wall height 3
        let obs = build_obstacle_field(&grid, &[]);
        assert_eq!(obs[0], 255, "stone wall should be obstacle");
    }

    #[test]
    fn test_obstacle_open_ground() {
        let grid = vec![make_block(2, 0, 0)]; // dirt floor
        let obs = build_obstacle_field(&grid, &[]);
        assert_eq!(obs[0], 0, "dirt floor should not be obstacle");
    }

    #[test]
    fn test_obstacle_open_door() {
        let grid = vec![make_block(4, 1, 1 | 4)]; // door + open flags
        let obs = build_obstacle_field(&grid, &[]);
        assert_eq!(obs[0], 0, "open door should not be obstacle");
    }

    #[test]
    fn test_obstacle_closed_door() {
        let grid = vec![make_block(4, 1, 1)]; // door flag only (closed)
        let obs = build_obstacle_field(&grid, &[]);
        assert_eq!(obs[0], 255, "closed door should be obstacle");
    }

    #[test]
    fn test_obstacle_tree_not_blocking() {
        let grid = vec![make_block(8, 3, 0)]; // tree
        let obs = build_obstacle_field(&grid, &[]);
        assert_eq!(obs[0], 0, "tree should not block fluid");
    }

    #[test]
    fn test_obstacle_fire_not_blocking() {
        let grid = vec![make_block(6, 1, 0)]; // fireplace
        let obs = build_obstacle_field(&grid, &[]);
        assert_eq!(obs[0], 0, "fireplace should not block fluid");
    }

    #[test]
    fn test_obstacle_wall_data_blocks() {
        use crate::grid::pack_wall_data;
        let grid = vec![make_block(2, 0, 0)]; // dirt floor
        let wd = vec![pack_wall_data(0xF, 4, 0)]; // full wall in wall_data
        let obs = build_obstacle_field(&grid, &wd);
        assert_eq!(obs[0], 255, "wall_data wall should be obstacle");
    }

    #[test]
    fn test_obstacle_wall_data_open_door() {
        use crate::grid::{WD_DOOR_OPEN, WD_HAS_DOOR, pack_wall_data};
        let grid = vec![make_block(2, 0, 0)];
        let wd = vec![pack_wall_data(0xF, 4, 0) | WD_HAS_DOOR | WD_DOOR_OPEN];
        let obs = build_obstacle_field(&grid, &wd);
        assert_eq!(obs[0], 0, "open wall_data door should not block fluid");
    }

    #[test]
    fn test_half_float_roundtrip() {
        let f16 = f32_to_f16(1.0);
        let back = half_to_f32(f16);
        assert!(
            (back - 1.0).abs() < 0.001,
            "1.0 roundtrip failed: got {}",
            back
        );

        let f16 = f32_to_f16(0.5);
        let back = half_to_f32(f16);
        assert!(
            (back - 0.5).abs() < 0.001,
            "0.5 roundtrip failed: got {}",
            back
        );

        let f16 = f32_to_f16(0.0);
        let back = half_to_f32(f16);
        assert_eq!(back, 0.0, "0.0 roundtrip failed");
    }
}
