//! Block grid — constants, packing, generation, and roof computation.

pub const GRID_W: u32 = 256;
pub const GRID_H: u32 = 256;

/// Pack a block into a u32: [type:8 | height:8 | flags:8 | roof_height:8]
pub fn make_block(block_type: u8, height: u8, flags: u8) -> u32 {
    (block_type as u32) | ((height as u32) << 8) | ((flags as u32) << 16)
}

pub fn block_type_rs(b: u32) -> u8 {
    (b & 0xFF) as u8
}

pub fn block_flags_rs(b: u32) -> u8 {
    ((b >> 16) & 0xFF) as u8
}

pub fn block_height_rs(b: u32) -> u8 {
    ((b >> 8) & 0xFF) as u8
}

pub fn roof_height_rs(b: u32) -> u8 {
    ((b >> 24) & 0xFF) as u8
}

// --- Grid index helpers ---

/// Convert grid (x, y) to flat index. Returns None if out of bounds.
#[inline]
pub fn grid_idx(x: i32, y: i32) -> Option<usize> {
    if x >= 0 && y >= 0 && x < GRID_W as i32 && y < GRID_H as i32 {
        Some((y as u32 * GRID_W + x as u32) as usize)
    } else {
        None
    }
}

/// Get block at (x, y). Returns 0 (air) for out-of-bounds.
#[inline]
pub fn get_block(grid: &[u32], x: i32, y: i32) -> u32 {
    grid_idx(x, y).map_or(0, |idx| grid[idx])
}

/// Set block at (x, y). No-op for out-of-bounds.
#[inline]
pub fn set_block(grid: &mut [u32], x: i32, y: i32, block: u32) {
    if let Some(idx) = grid_idx(x, y) {
        grid[idx] = block;
    }
}

// --- Game constants ---

pub const DAWN_FRAC: f32 = 0.15;   // day cycle fraction where dawn starts
pub const DUSK_FRAC: f32 = 0.85;   // day cycle fraction where dusk ends
pub const PLEB_MOVE_SPEED: f32 = 3.0;
pub const PLEB_RADIUS: f32 = 0.25;
pub const DOOR_AUTO_RANGE: f32 = 1.2;
pub const DOOR_AUTO_CLOSE_TIME: f32 = 2.0;
pub const CANNON_SPEED: f32 = 28.0;
pub const CANNON_RECOIL: f32 = 30.0;

pub fn is_door_rs(b: u32) -> bool {
    (block_flags_rs(b) & 1) != 0
}

/// Precompute roof heights and store in bits 24-31 of each block.
/// For every tile that's part of a roofed building, find the max wall height
/// in a large radius. This runs once at grid generation.
pub fn compute_roof_heights(grid: &mut Vec<u32>) {
    let w = GRID_W as i32;
    let h = GRID_H as i32;

    // Pass 1: identify which tiles are part of a roofed building
    let mut is_building = vec![false; grid.len()];
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            let block = grid[idx];
            let flags = block_flags_rs(block);

            if (flags & 2) != 0 {
                // Has roof flag
                is_building[idx] = true;
            } else if (block >> 8) & 0xFF > 0 || (flags & 1) != 0 {
                // Has height or is a door — check if adjacent to a roofed tile
                'outer: for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let nx = x + dx;
                        let ny = y + dy;
                        if nx >= 0 && ny >= 0 && nx < w && ny < h {
                            let nflags = (grid[(ny * w + nx) as usize] >> 16) & 0xFF;
                            if (nflags & 2) != 0 {
                                is_building[idx] = true;
                                break 'outer;
                            }
                        }
                    }
                }
            }
        }
    }

    // Pass 2: for each building tile, find the nearest enclosing wall in each
    // cardinal direction. This naturally finds the walls of THIS building without
    // bleeding into nearby buildings (which the old radius search did).
    let max_search = 20i32;
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            if !is_building[idx] {
                continue;
            }

            let mut max_h: u8 = 0;
            // Search in 4 cardinal directions for the nearest wall
            let dirs: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
            for &(ddx, ddy) in &dirs {
                for dist in 1..=max_search {
                    let nx = x + ddx * dist;
                    let ny = y + ddy * dist;
                    if nx < 0 || ny < 0 || nx >= w || ny >= h { break; }
                    let nidx = (ny * w + nx) as usize;
                    let nb = grid[nidx];
                    let nbh = ((nb >> 8) & 0xFF) as u8;
                    let nbt = (nb & 0xFF) as u8;
                    let nb_flags = ((nb >> 16) & 0xFF) as u8;
                    // Wall: has height, not roofed floor, not tree/fire/light
                    if nbh > 0 && (nb_flags & 2) == 0 && nbt != 8 && nbt != 6 && nbt != 7 {
                        max_h = max_h.max(nbh);
                        break; // found nearest wall in this direction
                    }
                    // If we hit a non-building tile, stop searching this direction
                    // (we've left the building footprint)
                    if !is_building[nidx] {
                        break;
                    }
                }
            }

            if max_h == 0 {
                max_h = 2; // fallback
            }

            // Store in bits 24-31
            grid[idx] = (grid[idx] & 0x00FFFFFF) | ((max_h as u32) << 24);
        }
    }
}

pub fn generate_test_grid() -> Vec<u32> {
    let mut grid = vec![make_block(0, 0, 0); (GRID_W * GRID_H) as usize];
    let w = GRID_W;

    for y in 0..GRID_H {
        for x in 0..GRID_W {
            grid[(y * w + x) as usize] = make_block(2, 0, 0);
        }
    }

    let set = |grid: &mut Vec<u32>, x: u32, y: u32, b: u32| {
        if x < GRID_W && y < GRID_H { grid[(y * w + x) as usize] = b; }
    };

    let ox = 90u32;
    let oy = 84u32;
    let oset = |grid: &mut Vec<u32>, x: u32, y: u32, b: u32| {
        if x + ox < GRID_W && y + oy < GRID_H { grid[((y + oy) * w + (x + ox)) as usize] = b; }
    };

    // === House 1: Stone cottage ===
    let h1_h = 4u8;
    let roof_flag = 2u8;
    for x in 10..30 { oset(&mut grid, x, 10, make_block(1, h1_h, 0)); oset(&mut grid, x, 25, make_block(1, h1_h, 0)); }
    for y in 10..26 { oset(&mut grid, 10, y, make_block(1, h1_h, 0)); oset(&mut grid, 29, y, make_block(1, h1_h, 0)); }
    for &wx in &[14u32, 15, 24, 25] { oset(&mut grid, wx, 10, make_block(5, h1_h, 0)); oset(&mut grid, wx, 25, make_block(5, h1_h, 0)); }
    for &(wx, wy) in &[(10u32, 15u32), (10, 20), (29, 15), (29, 20)] { oset(&mut grid, wx, wy, make_block(5, h1_h, 0)); }
    oset(&mut grid, 20, 10, make_block(4, 1, 1));
    for y in 11..25 { for x in 11..29 { oset(&mut grid, x, y, make_block(2, 0, roof_flag)); } }
    for x in 11..29 { oset(&mut grid, x, 18, make_block(1, h1_h, 0)); }
    oset(&mut grid, 16, 18, make_block(4, 1, 1));
    for y in 11..15 { oset(&mut grid, 22, y, make_block(1, h1_h, 0)); }
    oset(&mut grid, 19, 21, make_block(6, 1, roof_flag));
    oset(&mut grid, 40, 15, make_block(6, 1, 0));
    oset(&mut grid, 15, 14, make_block(7, 0, roof_flag));
    // Default bed for Jeff (horizontal, in upper room)
    oset(&mut grid, 12, 13, make_block(30, 0, roof_flag));
    oset(&mut grid, 13, 13, make_block(30, 0, roof_flag));
    // Storage crate near bed
    oset(&mut grid, 12, 15, make_block(33, 0, roof_flag));

    // === House 2: Tall building ===
    let h2_h = 4u8;
    for x in 35..55 { oset(&mut grid, x, 30, make_block(1, h2_h, 0)); oset(&mut grid, x, 50, make_block(1, h2_h, 0)); }
    for y in 30..51 { oset(&mut grid, 35, y, make_block(1, h2_h, 0)); oset(&mut grid, 54, y, make_block(1, h2_h, 0)); }
    for &wx in &[38u32, 41, 44, 47, 50] { oset(&mut grid, wx, 30, make_block(5, h2_h, 0)); oset(&mut grid, wx, 50, make_block(5, h2_h, 0)); }
    for &wy in &[34u32, 38, 42, 46] { oset(&mut grid, 35, wy, make_block(5, h2_h, 0)); oset(&mut grid, 54, wy, make_block(5, h2_h, 0)); }
    oset(&mut grid, 45, 30, make_block(4, 1, 1));
    for x in 36..54 { oset(&mut grid, x, 40, make_block(1, h2_h, 0)); }
    oset(&mut grid, 44, 40, make_block(4, 1, 1));
    for y in 31..50 { for x in 36..54 {
        let existing = grid[((y + oy) * w + (x + ox)) as usize];
        if block_type_rs(existing) == 0 || block_type_rs(existing) == 2 { oset(&mut grid, x, y, make_block(2, 0, roof_flag)); }
    }}

    // === Small shed ===
    let h3_h = 4u8;
    for x in 45..52 { oset(&mut grid, x, 8, make_block(1, h3_h, 0)); oset(&mut grid, x, 14, make_block(1, h3_h, 0)); }
    for y in 8..15 { oset(&mut grid, 45, y, make_block(1, h3_h, 0)); oset(&mut grid, 51, y, make_block(1, h3_h, 0)); }
    for &(wx, wy) in &[(48u32, 8u32), (48, 14), (45, 11), (51, 11)] { oset(&mut grid, wx, wy, make_block(5, h3_h, 0)); }
    oset(&mut grid, 49, 14, make_block(4, 1, 1));
    for y in 9..14 { for x in 46..51 { oset(&mut grid, x, y, make_block(2, 0, roof_flag)); } }

    // Water pool: dug ground (depth 1-5, water fills at depth >= 1)
    for y in 40..48 {
        for x in 12..22 {
            let edge = y == 40 || y == 47 || x == 12 || x == 21;
            let depth: u8 = if edge { 2 } else { 5 }; // shallow edges, deep center
            oset(&mut grid, x, y, make_block(32, depth, 0));
        }
    }

    // Greenhouse
    for x in 5..9 { oset(&mut grid, x, 55, make_block(5, 2, 0)); oset(&mut grid, x, 60, make_block(5, 2, 0)); }
    for y in 55..61 { oset(&mut grid, 5, y, make_block(5, 2, 0)); oset(&mut grid, 8, y, make_block(5, 2, 0)); }
    for y in 56..60 { for x in 6..8 { oset(&mut grid, x, y, make_block(2, 0, roof_flag)); } }

    // === Sealed insulated room (thermal test — no windows, no doors) ===
    // 8x8 interior, insulated walls (type 14), fire inside
    let seal_h = 3u8;
    for x in 0..10 { oset(&mut grid, x, 30, make_block(14, seal_h, 0)); oset(&mut grid, x, 39, make_block(14, seal_h, 0)); }
    for y in 30..40 { oset(&mut grid, 0, y, make_block(14, seal_h, 0)); oset(&mut grid, 9, y, make_block(14, seal_h, 0)); }
    for y in 31..39 { for x in 1..9 { oset(&mut grid, x, y, make_block(2, 0, roof_flag)); } }
    oset(&mut grid, 4, 35, make_block(6, 1, roof_flag)); // fire in center
    // Door on south wall (can open to release heat)
    oset(&mut grid, 5, 39, make_block(4, 1, 1)); // closed door

    // === Example pipe network: sealed room → house 2 ===
    // Inlet on east wall of sealed room at (9, 35) — sucks smoke out
    oset(&mut grid, 9, 35, make_block(20, seal_h, 1 << 3)); // inlet, dir=east (bits 3-4 = 1)
    // Pipes running east from inlet
    for x in 10..20 { oset(&mut grid, x, 35, make_block(15, 1, 0)); } // horizontal pipe run
    // Pump at midpoint
    oset(&mut grid, 15, 35, make_block(16, 1, 1 << 3)); // pump, dir=east
    // Outlet into house 2's west wall at (35, 35) — pushes gas eastward into the building
    oset(&mut grid, 35, 35, make_block(19, h2_h, 1 << 3)); // outlet, dir=east (bits 3-4 = 1)
    // Connect pipe to house 2
    for x in 20..35 { oset(&mut grid, x, 35, make_block(15, 1, 0)); }

    // Trees and bushes — clustered in small forests with outliers
    // Simple 2D noise for clustering
    let noise = |x: f32, y: f32| -> f32 {
        let ix = x.floor() as i32;
        let iy = y.floor() as i32;
        let fx = x - x.floor();
        let fy = y - y.floor();
        let hash = |ix: i32, iy: i32| -> f32 {
            let h = ((ix.wrapping_mul(374761393) as u32) ^ (iy.wrapping_mul(668265263) as u32))
                .wrapping_add(1013904223);
            (h & 0xFFFF) as f32 / 65535.0
        };
        let a = hash(ix, iy);
        let b = hash(ix + 1, iy);
        let c = hash(ix, iy + 1);
        let d = hash(ix + 1, iy + 1);
        let sx = fx * fx * (3.0 - 2.0 * fx);
        let sy = fy * fy * (3.0 - 2.0 * fy);
        a + (b - a) * sx + (c - a) * sy + (a - b - c + d) * sx * sy
    };

    let is_bare = |grid: &Vec<u32>, x: u32, y: u32| -> bool {
        if x >= GRID_W || y >= GRID_H { return false; }
        grid[(y * w + x) as usize] == make_block(2, 0, 0)
    };

    for y in 0..GRID_H {
        for x in 0..GRID_W {
            let idx = (y * w + x) as usize;
            if grid[idx] != make_block(2, 0, 0) { continue; }

            // Forest density from multi-octave noise (scale creates ~30-tile clusters)
            let scale = 0.07;
            let n1 = noise(x as f32 * scale, y as f32 * scale);
            let n2 = noise(x as f32 * scale * 2.3 + 100.0, y as f32 * scale * 2.3 + 200.0) * 0.5;
            let density = n1 + n2; // 0.0 - 1.5 range

            // Per-tile random hash
            let h = ((x.wrapping_mul(374761393)) ^ (y.wrapping_mul(668265263))).wrapping_add(1013904223);
            let r = (h >> 16) & 0xFFF; // 0..4095

            // Threshold based on density: dense areas have many trees
            let tree_threshold = if density > 0.9 { 400 }  // dense forest
                else if density > 0.7 { 150 }  // moderate forest
                else if density > 0.5 { 40 }   // sparse
                else { 8 };                     // rare outlier

            if r < tree_threshold {
                if r < tree_threshold / 5 {
                    // Large 2x2 tree
                    if is_bare(&grid, x+1, y) && is_bare(&grid, x, y+1) && is_bare(&grid, x+1, y+1) {
                        let tree_h = 4 + ((h >> 8) & 0x1) as u8;
                        set(&mut grid, x, y, make_block(8, tree_h, 32 | 0));
                        set(&mut grid, x+1, y, make_block(8, tree_h, 32 | 8));
                        set(&mut grid, x, y+1, make_block(8, tree_h, 32 | 16));
                        set(&mut grid, x+1, y+1, make_block(8, tree_h, 32 | 24));
                    }
                } else if r < tree_threshold * 3 / 4 {
                    // Medium tree
                    let tree_h = 2 + ((h >> 8) & 0x3) as u8;
                    grid[idx] = make_block(8, tree_h, 0);
                } else {
                    // Small bush (tree type)
                    let bush_h = 1 + ((h >> 8) & 0x1) as u8;
                    grid[idx] = make_block(8, bush_h, 0);
                }
            }

            // Berry bushes: scattered in moderate-density areas, rarer than trees
            let berry_r = ((h >> 4) & 0xFFF) as u32;
            let berry_threshold = if density > 0.6 && density < 1.0 { 15 }
                else if density > 0.4 { 5 }
                else { 1 };
            if grid[idx] == make_block(2, 0, 0) && berry_r < berry_threshold {
                grid[idx] = make_block(31, 1, 0);
            }

            // Rocks: scattered on bare ground, more common in sparse/open areas
            let rock_r = ((h >> 6) & 0xFFF) as u32;
            let rock_threshold = if density < 0.3 { 12 }
                else if density < 0.5 { 6 }
                else { 2 };
            if grid[idx] == make_block(2, 0, 0) && rock_r < rock_threshold {
                grid[idx] = make_block(34, 0, 0);
            }
        }
    }

    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_block_roundtrip() {
        // Pack and unpack should be lossless
        for bt in [0u8, 1, 5, 8, 13, 29, 255] {
            for h in [0u8, 1, 3, 5, 128, 255] {
                for f in [0u8, 1, 2, 4, 7, 63] {
                    let block = make_block(bt, h, f);
                    assert_eq!(block_type_rs(block), bt, "type mismatch for ({bt},{h},{f})");
                    assert_eq!(block_height_rs(block), h, "height mismatch for ({bt},{h},{f})");
                    assert_eq!(block_flags_rs(block), f, "flags mismatch for ({bt},{h},{f})");
                }
            }
        }
    }

    #[test]
    fn test_block_flags() {
        // bit0 = door, bit1 = roof, bit2 = open
        let door_closed = make_block(4, 1, 1); // door flag
        assert!(is_door_rs(door_closed));
        assert_eq!(block_flags_rs(door_closed) & 4, 0); // not open

        let door_open = make_block(4, 1, 1 | 4); // door + open
        assert!(is_door_rs(door_open));
        assert_ne!(block_flags_rs(door_open) & 4, 0); // open

        let wall = make_block(1, 3, 0);
        assert!(!is_door_rs(wall));

        let roofed = make_block(2, 0, 2); // roof flag
        assert_eq!(block_flags_rs(roofed) & 2, 2);
    }

    #[test]
    fn test_roof_height_stored_in_upper_byte() {
        let mut block = make_block(2, 0, 2); // dirt floor with roof
        // Manually set roof height in bits 24-31
        block = (block & 0x00FFFFFF) | (5u32 << 24);
        assert_eq!(roof_height_rs(block), 5);
        // Shouldn't affect other fields
        assert_eq!(block_type_rs(block), 2);
        assert_eq!(block_height_rs(block), 0);
        assert_eq!(block_flags_rs(block), 2);
    }

    #[test]
    fn test_compute_roof_heights_simple_room() {
        // Create a tiny 8x8 grid with a 4x4 room
        // Override GRID_W/H for testing... actually compute_roof_heights uses the module constants.
        // So we need a full 256x256 grid. Create one with a small room.
        let mut grid = vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize];
        let w = GRID_W;

        // Build a 4x4 room at (10,10)-(13,13) with height-3 walls
        for x in 10..14 {
            grid[(10 * w + x) as usize] = make_block(1, 3, 0); // top wall
            grid[(13 * w + x) as usize] = make_block(1, 3, 0); // bottom wall
        }
        for y in 10..14 {
            grid[(y * w + 10) as usize] = make_block(1, 3, 0); // left wall
            grid[(y * w + 13) as usize] = make_block(1, 3, 0); // right wall
        }
        // Interior: roofed floor
        for y in 11..13 {
            for x in 11..13 {
                grid[(y * w + x) as usize] = make_block(2, 0, 2); // dirt + roof flag
            }
        }

        compute_roof_heights(&mut grid);

        // Interior tiles should have roof_height = 3 (from walls)
        let interior = grid[(11 * w + 11) as usize];
        assert_eq!(roof_height_rs(interior), 3, "interior should have roof height 3");

        // Wall tiles should also have roof_height = 3
        let wall = grid[(10 * w + 11) as usize];
        assert_eq!(roof_height_rs(wall), 3, "wall should have roof height 3");

        // Outdoor tile should have roof_height = 0
        let outdoor = grid[(5 * w + 5) as usize];
        assert_eq!(roof_height_rs(outdoor), 0, "outdoor should have roof height 0");
    }

    #[test]
    fn test_grid_idx_bounds() {
        assert!(grid_idx(0, 0).is_some());
        assert!(grid_idx(255, 255).is_some());
        assert!(grid_idx(256, 0).is_none());
        assert!(grid_idx(-1, 0).is_none());
        assert!(grid_idx(0, -1).is_none());
    }

    #[test]
    fn test_get_set_block() {
        let mut grid = vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize];
        set_block(&mut grid, 10, 10, make_block(1, 3, 0));
        assert_eq!(block_type_rs(get_block(&grid, 10, 10)), 1);
        assert_eq!(block_height_rs(get_block(&grid, 10, 10)), 3);
        // Out of bounds returns air (0)
        assert_eq!(get_block(&grid, -1, 0), 0);
        assert_eq!(get_block(&grid, 256, 0), 0);
    }
}
