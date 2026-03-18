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

    // Pass 2: for each building tile, find max wall height in a large radius
    let search = 15i32; // handles buildings up to 30x30
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            if !is_building[idx] {
                continue;
            }

            let mut max_h: u8 = 0;
            for dy in -search..=search {
                for dx in -search..=search {
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                        continue;
                    }
                    let nidx = (ny * w + nx) as usize;
                    let nb = grid[nidx];
                    let nbh = ((nb >> 8) & 0xFF) as u8;
                    let nbt = (nb & 0xFF) as u8;
                    let nb_flags = ((nb >> 16) & 0xFF) as u8;
                    // Wall-type blocks: has height, not roofed floor, not tree/fire/light
                    if nbh > 0 && (nb_flags & 2) == 0 && nbt != 8 && nbt != 6 && nbt != 7 {
                        max_h = max_h.max(nbh);
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
    let h1_h = 3u8;
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

    // === House 2: Tall building ===
    let h2_h = 5u8;
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
    let h3_h = 2u8;
    for x in 45..52 { oset(&mut grid, x, 8, make_block(1, h3_h, 0)); oset(&mut grid, x, 14, make_block(1, h3_h, 0)); }
    for y in 8..15 { oset(&mut grid, 45, y, make_block(1, h3_h, 0)); oset(&mut grid, 51, y, make_block(1, h3_h, 0)); }
    for &(wx, wy) in &[(48u32, 8u32), (48, 14), (45, 11), (51, 11)] { oset(&mut grid, wx, wy, make_block(5, h3_h, 0)); }
    oset(&mut grid, 49, 14, make_block(4, 1, 1));
    for y in 9..14 { for x in 46..51 { oset(&mut grid, x, y, make_block(2, 0, roof_flag)); } }

    // Water pool
    for y in 40..48 { for x in 12..22 { oset(&mut grid, x, y, make_block(3, 0, 0)); } }

    // Greenhouse
    for x in 5..9 { oset(&mut grid, x, 55, make_block(5, 2, 0)); oset(&mut grid, x, 60, make_block(5, 2, 0)); }
    for y in 55..61 { oset(&mut grid, 5, y, make_block(5, 2, 0)); oset(&mut grid, 8, y, make_block(5, 2, 0)); }
    for y in 56..60 { for x in 6..8 { oset(&mut grid, x, y, make_block(2, 0, roof_flag)); } }

    // === Sealed insulated room (thermal test — no windows, no doors) ===
    // 8x8 interior, stone walls, fire inside
    let seal_h = 3u8;
    for x in 0..10 { oset(&mut grid, x, 30, make_block(1, seal_h, 0)); oset(&mut grid, x, 39, make_block(1, seal_h, 0)); }
    for y in 30..40 { oset(&mut grid, 0, y, make_block(1, seal_h, 0)); oset(&mut grid, 9, y, make_block(1, seal_h, 0)); }
    for y in 31..39 { for x in 1..9 { oset(&mut grid, x, y, make_block(2, 0, roof_flag)); } }
    oset(&mut grid, 4, 35, make_block(6, 1, roof_flag)); // fire in center
    // Door on south wall (can open to release heat)
    oset(&mut grid, 5, 39, make_block(4, 1, 1)); // closed door

    // Trees and bushes
    let is_bare = |grid: &Vec<u32>, x: u32, y: u32| -> bool {
        if x >= GRID_W || y >= GRID_H { return false; }
        grid[(y * w + x) as usize] == make_block(2, 0, 0)
    };
    for y in 0..GRID_H {
        for x in 0..GRID_W {
            let idx = (y * w + x) as usize;
            if grid[idx] != make_block(2, 0, 0) { continue; }
            let h = ((x.wrapping_mul(374761393)) ^ (y.wrapping_mul(668265263))).wrapping_add(1013904223);
            let r = (h >> 16) & 0xFFF;
            if r < 30 {
                if is_bare(&grid, x+1, y) && is_bare(&grid, x, y+1) && is_bare(&grid, x+1, y+1) {
                    let tree_h = 4 + ((h >> 8) & 0x1) as u8;
                    set(&mut grid, x, y, make_block(8, tree_h, 32 | 0));
                    set(&mut grid, x+1, y, make_block(8, tree_h, 32 | 8));
                    set(&mut grid, x, y+1, make_block(8, tree_h, 32 | 16));
                    set(&mut grid, x+1, y+1, make_block(8, tree_h, 32 | 24));
                }
            } else if r < 90 {
                grid[idx] = make_block(8, 2 + ((h >> 8) & 0x3) as u8, 0);
            } else if r < 140 {
                grid[idx] = make_block(8, 1 + ((h >> 8) & 0x1) as u8, 0);
            }
        }
    }

    grid
}
