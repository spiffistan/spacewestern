//! Sub-tile mining system — 8×8 grid per rock tile with mineral veins.
//! Miners carve directionally, revealing geology in the cut face.

use crate::grid::*;
use std::collections::HashMap;

pub const MINING_RES: usize = 8; // 8×8 sub-cells per tile

// Material types found in rock
pub const MAT_HOST: u8 = 0; // host rock (stone blocks when mined)
pub const MAT_IRON: u8 = 1; // iron oxide vein
pub const MAT_COPPER: u8 = 2; // copper vein
pub const MAT_FLINT: u8 = 3; // flint nodule
pub const MAT_CRYSTAL: u8 = 4; // crystal pocket
pub const MAT_COAL: u8 = 5; // coal seam
pub const MAT_VOID: u8 = 6; // empty pocket (natural cavity)

// Host rock types (determines color, hardness, structural behavior)
pub const ROCK_CHALK: u8 = 0;
pub const ROCK_SANDSTONE: u8 = 1;
pub const ROCK_LIMESTONE: u8 = 2;
pub const ROCK_GRANITE: u8 = 3;
pub const ROCK_BASALT: u8 = 4;

#[derive(Clone, Copy, Debug)]
pub struct MiningCell {
    pub material: u8, // MAT_* constant
    pub hardness: u8, // 0 = mined out, 1-10 = intact
}

#[derive(Clone, Debug)]
pub struct MiningGrid {
    pub cells: [[MiningCell; MINING_RES]; MINING_RES],
    pub rock_type: u8, // ROCK_* constant
}

/// Base hardness per rock type (host rock cells).
pub fn rock_hardness(rock_type: u8) -> u8 {
    match rock_type {
        ROCK_CHALK => 2,
        ROCK_SANDSTONE => 4,
        ROCK_LIMESTONE => 5,
        ROCK_GRANITE => 8,
        ROCK_BASALT => 10,
        _ => 5,
    }
}

/// Base hardness for mineral materials.
fn mineral_hardness(mat: u8, host_hardness: u8) -> u8 {
    match mat {
        MAT_IRON => host_hardness.saturating_sub(1).max(2),
        MAT_COPPER => host_hardness.saturating_sub(1).max(2),
        MAT_FLINT => 6,   // flint is always moderately hard
        MAT_CRYSTAL => 3, // crystals are fragile
        MAT_COAL => 2,    // coal is soft
        MAT_VOID => 0,    // already empty
        _ => host_hardness,
    }
}

/// Deterministic noise for vein generation (continuous across tile boundaries).
pub fn vein_noise(wx: f32, wy: f32, seed: u32) -> f32 {
    let x = wx + seed as f32 * 0.7123;
    let y = wy + seed as f32 * 1.3217;
    // Two-octave sine hash noise
    let n1 = (x * 4.37 + y * 7.13).sin() * 43758.5453;
    let n2 = (x * 11.91 + y * 5.83).sin() * 27183.6;
    (n1.fract() + n2.fract()) * 0.5
}

/// Generate a mining grid for a tile at world position (tx, ty).
/// Deterministic: same position always produces the same grid.
pub fn generate_mining_grid(tx: i32, ty: i32, rock_type: u8) -> MiningGrid {
    let base_h = rock_hardness(rock_type);
    let mut grid = MiningGrid {
        cells: [[MiningCell {
            material: MAT_HOST,
            hardness: base_h,
        }; MINING_RES]; MINING_RES],
        rock_type,
    };

    let tile_seed = (tx as u32)
        .wrapping_mul(374761393)
        .wrapping_add((ty as u32).wrapping_mul(668265263));

    // Sample veins at sub-cell resolution
    for sy in 0..MINING_RES {
        for sx in 0..MINING_RES {
            // World-space position of this sub-cell center
            let wx = tx as f32 + (sx as f32 + 0.5) / MINING_RES as f32;
            let wy = ty as f32 + (sy as f32 + 0.5) / MINING_RES as f32;

            // Iron: wide shallow bands (seed 100)
            let iron_v = vein_noise(wx * 3.0, wy * 3.0, 100);
            if iron_v > 0.82 {
                grid.cells[sy][sx] = MiningCell {
                    material: MAT_IRON,
                    hardness: mineral_hardness(MAT_IRON, base_h),
                };
                continue;
            }

            // Copper: narrow deep lines (seed 200)
            let copper_v = vein_noise(wx * 6.0, wy * 6.0, 200);
            if copper_v > 0.90 {
                grid.cells[sy][sx] = MiningCell {
                    material: MAT_COPPER,
                    hardness: mineral_hardness(MAT_COPPER, base_h),
                };
                continue;
            }

            // Flint: nodules in chalk/limestone only (seed 300)
            if rock_type == ROCK_CHALK || rock_type == ROCK_LIMESTONE {
                let flint_v = vein_noise(wx * 5.0, wy * 5.0, 300);
                let flint_v2 = vein_noise(wx * 4.0 + 10.0, wy * 4.0 + 10.0, 350);
                if flint_v > 0.85 && flint_v2 > 0.7 {
                    grid.cells[sy][sx] = MiningCell {
                        material: MAT_FLINT,
                        hardness: mineral_hardness(MAT_FLINT, base_h),
                    };
                    continue;
                }
            }

            // Coal: thick seams in sedimentary rock (seed 400)
            if rock_type == ROCK_SANDSTONE || rock_type == ROCK_LIMESTONE {
                let coal_v = vein_noise(wx * 2.5, wy * 2.5, 400);
                if coal_v > 0.85 {
                    grid.cells[sy][sx] = MiningCell {
                        material: MAT_COAL,
                        hardness: mineral_hardness(MAT_COAL, base_h),
                    };
                    continue;
                }
            }

            // Crystal: rare pockets in any rock (seed 500)
            let crystal_v = vein_noise(wx * 8.0, wy * 8.0, 500);
            let crystal_v2 = vein_noise(wx * 7.0 + 5.0, wy * 7.0 + 5.0, 550);
            if crystal_v > 0.92 && crystal_v2 > 0.85 {
                grid.cells[sy][sx] = MiningCell {
                    material: MAT_CRYSTAL,
                    hardness: mineral_hardness(MAT_CRYSTAL, base_h),
                };
                continue;
            }

            // Void pockets: rare empty spaces (seed 600)
            let void_v = vein_noise(wx * 6.0, wy * 6.0, 600);
            if void_v > 0.95 {
                grid.cells[sy][sx] = MiningCell {
                    material: MAT_VOID,
                    hardness: 0,
                };
            }

            // Small hardness variation in host rock
            let h_var = (tile_seed.wrapping_add(sx as u32 * 17 + sy as u32 * 31)) % 3;
            if h_var == 0 && grid.cells[sy][sx].material == MAT_HOST {
                grid.cells[sy][sx].hardness = base_h.saturating_add(1).min(10);
            }
        }
    }

    grid
}

/// Determine rock type from terrain position (uses terrain noise zones).
/// This should match the geological zone layout from DN-024.
pub fn rock_type_at(tx: i32, ty: i32) -> u8 {
    let cx = GRID_W as f32 / 2.0;
    let cy = GRID_H as f32 / 2.0;
    let dx = tx as f32 - cx;
    let dy = ty as f32 - cy;
    // Simple quadrant-based geology for now
    let angle = dy.atan2(dx);
    // Add noise for organic boundaries
    let noise = ((tx as f32 * 0.05).sin() * (ty as f32 * 0.07).cos()) * 0.5;
    let sector = (angle + std::f32::consts::PI + noise) / std::f32::consts::TAU;
    match (sector * 5.0) as u32 % 5 {
        0 => ROCK_GRANITE,
        1 => ROCK_CHALK,
        2 => ROCK_SANDSTONE,
        3 => ROCK_LIMESTONE,
        4 => ROCK_BASALT,
        _ => ROCK_SANDSTONE,
    }
}

/// Pack mining grid into a flat u8 array for GPU upload.
/// Layout: 64 bytes per tile, one byte per sub-cell.
/// High nibble = material (0-15), low nibble = hardness (0=mined, 1-10=intact).
pub fn pack_mining_grid(grid: &MiningGrid) -> [u8; 64] {
    let mut packed = [0u8; 64];
    for sy in 0..MINING_RES {
        for sx in 0..MINING_RES {
            let cell = &grid.cells[sy][sx];
            packed[sy * MINING_RES + sx] = (cell.material << 4) | (cell.hardness & 0xF);
        }
    }
    packed
}

/// Mine a single sub-cell. Returns the material that was mined, or None if already empty.
pub fn mine_cell(grid: &mut MiningGrid, sx: usize, sy: usize) -> Option<u8> {
    if sx >= MINING_RES || sy >= MINING_RES {
        return None;
    }
    let cell = &mut grid.cells[sy][sx];
    if cell.hardness == 0 {
        return None; // already mined
    }
    let mat = cell.material;
    cell.hardness = 0;
    cell.material = MAT_VOID;
    Some(mat)
}

/// Find the next unmined sub-cell closest to the miner's facing edge.
/// `face`: 0=north (miner to south), 1=east (miner to west),
///         2=south (miner to north), 3=west (miner to east)
pub fn next_mine_target(grid: &MiningGrid, face: u8) -> Option<(usize, usize)> {
    match face {
        0 => {
            // Mining from north: scan from y=0 downward
            for sy in 0..MINING_RES {
                for sx in 0..MINING_RES {
                    if grid.cells[sy][sx].hardness > 0 {
                        return Some((sx, sy));
                    }
                }
            }
        }
        1 => {
            // Mining from east: scan from x=MINING_RES-1 leftward
            for sx in (0..MINING_RES).rev() {
                for sy in 0..MINING_RES {
                    if grid.cells[sy][sx].hardness > 0 {
                        return Some((sx, sy));
                    }
                }
            }
        }
        2 => {
            // Mining from south: scan from y=MINING_RES-1 upward
            for sy in (0..MINING_RES).rev() {
                for sx in 0..MINING_RES {
                    if grid.cells[sy][sx].hardness > 0 {
                        return Some((sx, sy));
                    }
                }
            }
        }
        3 => {
            // Mining from west: scan from x=0 rightward
            for sx in 0..MINING_RES {
                for sy in 0..MINING_RES {
                    if grid.cells[sy][sx].hardness > 0 {
                        return Some((sx, sy));
                    }
                }
            }
        }
        _ => {}
    }
    None
}

/// Count how many sub-cells have been mined out.
pub fn mined_count(grid: &MiningGrid) -> usize {
    grid.cells
        .iter()
        .flat_map(|row| row.iter())
        .filter(|c| c.hardness == 0)
        .count()
}

/// Check if the grid is fully mined.
pub fn is_fully_mined(grid: &MiningGrid) -> bool {
    mined_count(grid) >= MINING_RES * MINING_RES
}

/// Rock type display name.
pub fn rock_type_name(rt: u8) -> &'static str {
    match rt {
        ROCK_CHALK => "Chalk",
        ROCK_SANDSTONE => "Sandstone",
        ROCK_LIMESTONE => "Limestone",
        ROCK_GRANITE => "Granite",
        ROCK_BASALT => "Basalt",
        _ => "Rock",
    }
}

/// Material display name.
pub fn material_name(mat: u8) -> &'static str {
    match mat {
        MAT_HOST => "Stone",
        MAT_IRON => "Iron Ore",
        MAT_COPPER => "Copper Ore",
        MAT_FLINT => "Flint",
        MAT_CRYSTAL => "Crystal",
        MAT_COAL => "Coal",
        MAT_VOID => "Void",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_deterministic() {
        let g1 = generate_mining_grid(10, 20, ROCK_GRANITE);
        let g2 = generate_mining_grid(10, 20, ROCK_GRANITE);
        for sy in 0..MINING_RES {
            for sx in 0..MINING_RES {
                assert_eq!(g1.cells[sy][sx].material, g2.cells[sy][sx].material);
                assert_eq!(g1.cells[sy][sx].hardness, g2.cells[sy][sx].hardness);
            }
        }
    }

    #[test]
    fn test_mine_cell() {
        let mut g = generate_mining_grid(5, 5, ROCK_SANDSTONE);
        let mat = mine_cell(&mut g, 0, 0);
        assert!(mat.is_some());
        assert_eq!(g.cells[0][0].hardness, 0);
        // Mining again returns None
        assert!(mine_cell(&mut g, 0, 0).is_none());
    }

    #[test]
    fn test_next_mine_target_south_face() {
        let mut g = generate_mining_grid(5, 5, ROCK_CHALK);
        // Mine from south: first target should be bottom row
        let (_, sy) = next_mine_target(&g, 2).unwrap();
        assert_eq!(sy, MINING_RES - 1);
    }

    #[test]
    fn test_flint_in_chalk() {
        // Generate many grids and check that chalk has flint sometimes
        let mut found_flint = false;
        for tx in 0..50 {
            let g = generate_mining_grid(tx, 0, ROCK_CHALK);
            for row in &g.cells {
                for cell in row {
                    if cell.material == MAT_FLINT {
                        found_flint = true;
                    }
                }
            }
        }
        assert!(found_flint, "chalk should contain flint nodules");
    }

    #[test]
    fn test_no_flint_in_granite() {
        // Granite should never have flint
        for tx in 0..50 {
            let g = generate_mining_grid(tx, 0, ROCK_GRANITE);
            for row in &g.cells {
                for cell in row {
                    assert_ne!(cell.material, MAT_FLINT, "granite should not contain flint");
                }
            }
        }
    }

    #[test]
    fn test_pack_roundtrip() {
        let g = generate_mining_grid(7, 13, ROCK_LIMESTONE);
        let packed = pack_mining_grid(&g);
        for sy in 0..MINING_RES {
            for sx in 0..MINING_RES {
                let byte = packed[sy * MINING_RES + sx];
                let mat = byte >> 4;
                let hard = byte & 0xF;
                assert_eq!(mat, g.cells[sy][sx].material);
                assert_eq!(hard, g.cells[sy][sx].hardness.min(15));
            }
        }
    }

    #[test]
    fn test_fully_mined() {
        let mut g = generate_mining_grid(0, 0, ROCK_CHALK);
        assert!(!is_fully_mined(&g));
        for sy in 0..MINING_RES {
            for sx in 0..MINING_RES {
                mine_cell(&mut g, sx, sy);
            }
        }
        assert!(is_fully_mined(&g));
    }
}
