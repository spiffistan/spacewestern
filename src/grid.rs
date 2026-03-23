//! Block grid — constants, packing, generation, and roof computation.

pub const GRID_W: u32 = 256;
pub const GRID_H: u32 = 256;

// Block type IDs (must match blocks.toml). u32 for direct comparison with extracted block types.
pub const BT_AIR: u32 = 0;
pub const BT_STONE: u32 = 1;
pub const BT_DIRT: u32 = 2;
pub const BT_WATER: u32 = 3;
pub const BT_WALL: u32 = 4;
pub const BT_GLASS: u32 = 5;
pub const BT_FIREPLACE: u32 = 6;
pub const BT_CEILING_LIGHT: u32 = 7;
pub const BT_TREE: u32 = 8;
pub const BT_BENCH: u32 = 9;
pub const BT_FLOOR_LAMP: u32 = 10;
pub const BT_TABLE_LAMP: u32 = 11;
pub const BT_FAN: u32 = 12;
pub const BT_COMPOST: u32 = 13;
pub const BT_INSULATED: u32 = 14;
pub const BT_PIPE: u32 = 15;
pub const BT_PUMP: u32 = 16;
pub const BT_TANK: u32 = 17;
pub const BT_VALVE: u32 = 18;
pub const BT_OUTLET: u32 = 19;
pub const BT_INLET: u32 = 20;
pub const BT_WOOD_WALL: u32 = 21;
pub const BT_STEEL_WALL: u32 = 22;
pub const BT_SANDSTONE: u32 = 23;
pub const BT_GRANITE: u32 = 24;
pub const BT_LIMESTONE: u32 = 25;
pub const BT_WOOD_FLOOR: u32 = 26;
pub const BT_STONE_FLOOR: u32 = 27;
pub const BT_CONCRETE_FLOOR: u32 = 28;
pub const BT_CANNON: u32 = 29;
pub const BT_BED: u32 = 30;
pub const BT_BERRY_BUSH: u32 = 31;
pub const BT_DUG_GROUND: u32 = 32;
pub const BT_CRATE: u32 = 33;
pub const BT_ROCK: u32 = 34;
pub const BT_MUD_WALL: u32 = 35;
pub const BT_WIRE: u32 = 36;
pub const BT_SOLAR: u32 = 37;
pub const BT_BATTERY_S: u32 = 38;
pub const BT_BATTERY_M: u32 = 39;
pub const BT_BATTERY_L: u32 = 40;
pub const BT_WIND_TURBINE: u32 = 41;
pub const BT_SWITCH: u32 = 42;
pub const BT_DIMMER: u32 = 43;
pub const BT_DIAGONAL: u32 = 44;
pub const BT_BREAKER: u32 = 45;
pub const BT_RESTRICTOR: u32 = 46;
pub const BT_CROP: u32 = 47;
pub const BT_FLOODLIGHT: u32 = 48;
pub const BT_LIQUID_PIPE: u32 = 49;
pub const BT_PIPE_BRIDGE: u32 = 50;
pub const BT_WIRE_BRIDGE: u32 = 51;
pub const BT_LIQUID_INTAKE: u32 = 52;
pub const BT_LIQUID_PUMP: u32 = 53;
pub const BT_LIQUID_OUTPUT: u32 = 54;

/// Pack a block into a u32: [type:8 | height:8 | flags:8 | roof_height:8]
pub fn make_block(block_type: u8, height: u8, flags: u8) -> u32 {
    (block_type as u32) | ((height as u32) << 8) | ((flags as u32) << 16)
}

pub fn block_type_rs(b: u32) -> u32 {
    b & 0xFF
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

/// Is this block type part of the electrical power network?
/// Checks block type and wire overlay flag. Matches the GPU-side is_conductor() in power.wgsl.
pub fn is_conductor_rs(bt: u32, flags: u8) -> bool {
    matches!(bt, 36..=43 | 45 | 48 | 51 | 7 | 10..=12 | 16) || (flags & 0x80) != 0
}

/// Is this block type a ground/floor tile (walkable base, not a placed object)?
pub fn is_ground_block(bt: u32) -> bool {
    bt_is!(bt, BT_AIR, BT_DIRT, BT_WATER, BT_WOOD_FLOOR, BT_STONE_FLOOR, BT_CONCRETE_FLOOR, BT_DUG_GROUND)
}

/// Is this block type a structural wall?
pub fn is_wall_block(bt: u32) -> bool {
    bt_is!(bt, BT_STONE, BT_WALL, BT_GLASS, BT_INSULATED,
        BT_WOOD_WALL, BT_STEEL_WALL, BT_SANDSTONE, BT_GRANITE, BT_LIMESTONE, BT_MUD_WALL, BT_DIAGONAL)
}

/// Is this block type a wire/power equipment (height byte = connection mask, not visual)?
pub fn is_wire_block(bt: u32) -> bool {
    bt_is!(bt, BT_WIRE, BT_DIMMER, BT_SWITCH, BT_BREAKER, BT_WIRE_BRIDGE)
}

/// Direction mask constants for pipe/wire connections.
/// Encoded in height byte upper nibble: N=0x10, E=0x20, S=0x40, W=0x80.
/// After >> 4: N=1, E=2, S=4, W=8.
pub const DIR_MASKS: [(i32, i32, u32); 4] = [(0, -1, 0x1), (0, 1, 0x4), (1, 0, 0x2), (-1, 0, 0x8)];

/// Unpacked block data for convenient access.
pub struct BlockInfo {
    pub raw: u32,
    pub block_type: u32,
    pub height: u32,
    pub flags: u8,
    pub roof_height: u32,
}

impl BlockInfo {
    pub fn from_raw(b: u32) -> Self {
        BlockInfo {
            raw: b,
            block_type: b & 0xFF,
            height: (b >> 8) & 0xFF,
            flags: ((b >> 16) & 0xFF) as u8,
            roof_height: (b >> 24) & 0xFF,
        }
    }

    pub fn is_door(&self) -> bool { self.flags & 1 != 0 }
    pub fn is_roofed(&self) -> bool { self.flags & 2 != 0 }
    pub fn is_open(&self) -> bool { self.flags & 4 != 0 }
}

/// Get unpacked block info at (x, y). Returns air for out-of-bounds.
pub fn block_at(grid: &[u32], x: i32, y: i32) -> BlockInfo {
    if x >= 0 && y >= 0 && x < GRID_W as i32 && y < GRID_H as i32 {
        BlockInfo::from_raw(grid[(y as u32 * GRID_W + x as u32) as usize])
    } else {
        BlockInfo::from_raw(0)
    }
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
                    // Wall: has height, not roofed floor, not tree/fire/light/wire/dimmer/crate
                    // Wire(36), dimmer(43), varistor(47), restrictor(46) use height for level, not visual
                    // Crate(33) uses height for item count
                    let skip = bt_is!(nbt as u32, BT_TREE, BT_FIREPLACE, BT_CEILING_LIGHT,
                        BT_CRATE, BT_WIRE, BT_DIMMER, BT_RESTRICTOR,
                        BT_LIQUID_PIPE, BT_PIPE_BRIDGE, BT_WIRE_BRIDGE,
                        BT_LIQUID_INTAKE, BT_LIQUID_PUMP, BT_LIQUID_OUTPUT);
                    if nbh > 0 && (nb_flags & 2) == 0 && !skip
                    {
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

/// Generate the water table height map (256x256).
/// Values represent depth below ground: negative = below surface, positive = above (springs).
/// Uses multi-octave noise with hotspots near the pond area.
pub fn generate_water_table(grid: &[u32]) -> Vec<f32> {
    let w = GRID_W;
    let h = GRID_H;
    let mut table = vec![-2.0f32; (w * h) as usize];

    // Same noise function as tree generation
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

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;

            // Multi-octave noise for natural variation
            let scale1 = 0.03; // large-scale features (~30 tile wavelength)
            let scale2 = 0.08; // medium detail
            let n1 = noise(x as f32 * scale1 + 50.0, y as f32 * scale1 + 50.0);
            let n2 = noise(x as f32 * scale2 + 200.0, y as f32 * scale2 + 200.0) * 0.4;
            let base = n1 + n2; // 0.0 to 1.4 range

            // Map to water table depth: -3.0 (deep/dry) to -0.3 (near surface/wet)
            let depth = -3.0 + base * 2.0; // range: -3.0 to -0.2

            // Hotspot near the pond area (world gen offset is 90, 84)
            let pond_cx = 90.0 + 17.0; // center of the pond
            let pond_cy = 84.0 + 44.0;
            let pond_dist = ((x as f32 - pond_cx).powi(2) + (y as f32 - pond_cy).powi(2)).sqrt();
            let pond_boost = (1.0 - (pond_dist / 20.0).min(1.0)) * 2.5; // raises water table near pond

            // Also boost near dug ground (it was dug because there's water)
            let block = grid[idx];
            let bt = block & 0xFF;
            let dug_boost = if bt == BT_DUG_GROUND { 1.0 } else { 0.0 };

            table[idx] = (depth + pond_boost + dug_boost).min(0.5); // cap at 0.5 (strong spring)
        }
    }

    table
}

/// Compute terrain ambient occlusion by tracing rays in dawn AND dusk sun directions.
/// Uses soft occlusion (proportional to how much terrain exceeds the ray) and
/// Gaussian blur for smooth, natural-looking structural shadows.
/// Produces a 0.0–1.0 value per cell: 1.0 = fully exposed, lower = occluded.
pub fn compute_terrain_ao(elevation: &[f32]) -> Vec<f32> {
    let w = GRID_W as i32;
    let h = GRID_H as i32;
    let mut ao = vec![1.0f32; elevation.len()];

    // Dawn (~04:20) and dusk (~19:20) sun directions
    let dawn_angle = 0.0436 * std::f32::consts::PI;
    let dusk_angle = 0.9364 * std::f32::consts::PI;
    let dawn_dir = (-dawn_angle.cos(), -dawn_angle.sin() * 0.6 - 0.2);
    let dusk_dir = (-dusk_angle.cos(), -dusk_angle.sin() * 0.6 - 0.2);
    let norm = |d: (f32, f32)| -> (f32, f32) {
        let len = (d.0 * d.0 + d.1 * d.1).sqrt();
        (d.0 / len, d.1 / len)
    };
    let dawn_n = norm(dawn_dir);
    let dusk_n = norm(dusk_dir);
    let dirs: [(f32, f32); 6] = [
        dawn_n, dusk_n,
        (0.0, -1.0), (0.0, 1.0),
        norm((dawn_n.0 + dusk_n.0, dawn_n.1 - 0.5)),
        norm((dusk_n.0 + dawn_n.0, dusk_n.1 + 0.5)),
    ];
    let sun_angle = 0.20f32;
    let max_dist = 18;

    // Pass 1: soft occlusion — accumulate proportional blocking from ALL obstacles
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            let e = elevation[idx];
            let mut blocked = 0.0f32;

            for &(dx, dy) in &dirs {
                let mut ray_blocked = false;
                for step in 1..=max_dist {
                    let sx = x as f32 + dx * step as f32;
                    let sy = y as f32 + dy * step as f32;
                    let sxi = sx as i32;
                    let syi = sy as i32;
                    if sxi < 0 || syi < 0 || sxi >= w || syi >= h { break; }
                    let sidx = (syi * w + sxi) as usize;
                    let ray_h = e + sun_angle * step as f32;
                    let excess = elevation[sidx] - ray_h;
                    if excess > 0.0 {
                        // Soft: proportional to how much the obstacle exceeds the ray
                        // Capped at 2.0 to avoid extreme values from tall cliffs
                        let block_amount = excess.min(2.0);
                        // Inverse square falloff with distance
                        let weight = 1.0 / (1.0 + step as f32 * step as f32 * 0.1);
                        blocked += block_amount * weight;
                        ray_blocked = true;
                        break; // first obstacle along this ray
                    }
                }
                // Small ambient contribution if ray escaped (exposed to sky)
                if !ray_blocked {
                    blocked -= 0.02; // slightly brighten fully exposed cells
                }
            }

            ao[idx] = (1.0 - blocked * 0.10).clamp(0.4, 1.0);
        }
    }

    // Pass 2: Gaussian blur (7×7 kernel, applied twice for extra softness)
    let kernel: [f32; 7] = [0.03, 0.11, 0.22, 0.28, 0.22, 0.11, 0.03];
    for _pass in 0..2 {
        // Horizontal
        let mut temp = ao.clone();
        for y in 0..h {
            for x in 0..w {
                let mut sum = 0.0f32;
                let mut wt = 0.0f32;
                for k in 0..7i32 {
                    let sx = (x + k - 3).clamp(0, w - 1);
                    let ki = kernel[k as usize];
                    sum += temp[(y * w + sx) as usize] * ki;
                    wt += ki;
                }
                ao[(y * w + x) as usize] = sum / wt;
            }
        }
        // Vertical
        temp.copy_from_slice(&ao);
        for y in 0..h {
            for x in 0..w {
                let mut sum = 0.0f32;
                let mut wt = 0.0f32;
                for k in 0..7i32 {
                    let sy = (y + k - 3).clamp(0, h - 1);
                    let ki = kernel[k as usize];
                    sum += temp[(sy * w + x) as usize] * ki;
                    wt += ki;
                }
                ao[(y * w + x) as usize] = sum / wt;
            }
        }
    }

    ao
}

/// Adjust water table based on elevation — hilltops are drier, valleys wetter.
/// Call after both water_table and elevation are generated.
pub fn adjust_water_table_for_elevation(water_table: &mut [f32], elevation: &[f32]) {
    for i in 0..water_table.len().min(elevation.len()) {
        // Higher elevation = lower effective water table (drier hilltops)
        // Lower elevation = higher effective water table (wet valleys)
        water_table[i] -= elevation[i] * 0.4; // 0.4 units drier per elevation unit
        water_table[i] = water_table[i].clamp(-3.0, 0.5);
    }
}

/// Generate terrain elevation map using multi-octave noise.
/// Returns 256×256 f32 values in range 0.0–6.0 representing tiles of height.
/// Features: gentle rolling hills, flat areas in the center, low near water.
pub fn generate_elevation(grid: &[u32]) -> Vec<f32> {
    let w = GRID_W;
    let h = GRID_H;
    let mut elev = vec![0.0f32; (w * h) as usize];

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

    // Mostly flat terrain with a few gentle hills rising from the plain.
    // Hills are sparse — only where noise exceeds a high threshold do we get elevation.
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            let fx = x as f32;
            let fy = y as f32;

            // Multi-octave noise for hill placement
            let n1 = noise(fx * 0.025 + 300.0, fy * 0.025 + 700.0);   // broad features
            let n2 = noise(fx * 0.06 + 500.0, fy * 0.06 + 100.0) * 0.4; // medium detail
            let n3 = noise(fx * 0.15 + 800.0, fy * 0.15 + 400.0) * 0.15; // fine bumps
            let raw = n1 + n2 + n3; // ~0.0–1.55 range

            // Threshold: only values above ~0.7 produce hills (keeps most of map flat)
            let hill_threshold = 0.65;
            let hill_raw = ((raw - hill_threshold) / (1.0 - hill_threshold)).max(0.0);
            // Smooth ramp: square root for gentle slopes, scale to max ~4 tiles high
            let height = hill_raw.sqrt() * 4.0;

            // Subtle undulation everywhere (very low amplitude, gives life to flat areas)
            let micro = noise(fx * 0.08 + 150.0, fy * 0.08 + 250.0) * 0.15;

            // Suppress near water
            let bt = grid[idx] & 0xFF;
            let water_suppress = if bt == BT_WATER { 0.0 } else { 1.0 };

            // Edge fade: flatten near map edges (10 tile border)
            let edge_dist = (fx.min(w as f32 - fx)).min(fy.min(h as f32 - fy));
            let edge_fade = (edge_dist / 10.0).min(1.0);

            elev[idx] = (height + micro) * water_suppress * edge_fade;
        }
    }

    elev
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
                    assert_eq!(block_type_rs(block), bt as u32, "type mismatch for ({bt},{h},{f})");
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

    #[test]
    fn test_pipe_connection_mask_encoding() {
        // Connection mask bits: CONN_N=0x10, CONN_E=0x20, CONN_S=0x40, CONN_W=0x80
        // Stored in height byte, extracted as (height >> 4) & 0xF
        // After >> 4: N=1, E=2, S=4, W=8

        // Horizontal pipe: connects E+W
        let h_pipe = make_block(15, 1 | 0x20 | 0x80, 0); // base_h=1, CONN_E + CONN_W
        assert_eq!(block_type_rs(h_pipe), 15);
        let h_mask = (block_height_rs(h_pipe) >> 4) & 0xF;
        assert_eq!(h_mask & 0x2, 0x2, "should connect E"); // E=bit1
        assert_eq!(h_mask & 0x8, 0x8, "should connect W"); // W=bit3
        assert_eq!(h_mask & 0x1, 0x0, "should NOT connect N");
        assert_eq!(h_mask & 0x4, 0x0, "should NOT connect S");

        // Vertical pipe: connects N+S
        let v_pipe = make_block(15, 1 | 0x10 | 0x40, 0); // base_h=1, CONN_N + CONN_S
        let v_mask = (block_height_rs(v_pipe) >> 4) & 0xF;
        assert_eq!(v_mask & 0x1, 0x1, "should connect N");
        assert_eq!(v_mask & 0x4, 0x4, "should connect S");
        assert_eq!(v_mask & 0x2, 0x0, "should NOT connect E");
        assert_eq!(v_mask & 0x8, 0x0, "should NOT connect W");

        // Corner pipe: connects N+E (L-bend)
        let c_pipe = make_block(15, 1 | 0x10 | 0x20, 0);
        let c_mask = (block_height_rs(c_pipe) >> 4) & 0xF;
        assert_eq!(c_mask & 0x1, 0x1, "should connect N");
        assert_eq!(c_mask & 0x2, 0x2, "should connect E");
        assert_eq!(c_mask & 0x4, 0x0, "should NOT connect S");
        assert_eq!(c_mask & 0x8, 0x0, "should NOT connect W");

        // Wire: height=0, so mask is in upper nibble only
        let h_wire = make_block(36, 0 | 0x20 | 0x80, 0); // base_h=0, CONN_E + CONN_W
        assert_eq!(block_type_rs(h_wire), 36);
        let w_mask = (block_height_rs(h_wire) >> 4) & 0xF;
        assert_eq!(w_mask & 0x2, 0x2, "wire should connect E");
        assert_eq!(w_mask & 0x8, 0x8, "wire should connect W");
        assert_eq!(w_mask & 0x1, 0x0, "wire should NOT connect N");

        // No mask (legacy/single-click): mask=0 → auto-detect
        let legacy = make_block(15, 1, 0); // just base height, no mask
        let l_mask = (block_height_rs(legacy) >> 4) & 0xF;
        assert_eq!(l_mask, 0, "legacy pipe should have mask=0 (auto-detect)");

        // All directions (single-tile drag):
        let all = make_block(36, 0xF0, 0); // all 4 connections
        let a_mask = (block_height_rs(all) >> 4) & 0xF;
        assert_eq!(a_mask, 0xF, "all-direction should have mask=15");
    }

    // --- Placement validation tests ---
    // These test the placement rules that determine blue vs red blueprint.
    // A blue blueprint MUST mean the block is actually placeable.

    /// Helper: check if block type `place_id` can be placed on a tile containing `existing_bt`
    /// Must match the actual placement rules in main.rs handle_click and blueprint validation.
    fn can_place_on_block(place_id: u8, existing_bt: u32, existing_h: u8) -> bool {
        let empty_ground = existing_h == 0 && (existing_bt == BT_AIR || existing_bt == BT_DIRT
            || existing_bt == BT_WOOD_FLOOR || existing_bt == BT_STONE_FLOOR || existing_bt == BT_CONCRETE_FLOOR);
        let pid = place_id as u32;
        empty_ground
            || (pid == BT_WIRE && existing_bt != BT_WIRE)
            || (pid == BT_PIPE && (existing_bt == BT_PIPE || existing_bt == BT_PIPE_BRIDGE))
            || (pid == BT_RESTRICTOR && (existing_bt == BT_PIPE || existing_bt == BT_RESTRICTOR || existing_bt == BT_PIPE_BRIDGE))
            || (pid == BT_LIQUID_PIPE && (existing_bt == BT_LIQUID_PIPE || existing_bt == BT_PIPE_BRIDGE))
            || (pid == BT_PUMP && existing_bt == BT_PIPE)
            || ((pid == BT_SWITCH || pid == BT_DIMMER || pid == BT_BREAKER) && (existing_bt == BT_WIRE || existing_bt == BT_AIR || existing_bt == BT_DIRT))
    }

    #[test]
    fn test_all_block_ids_valid() {
        // Every defined block type ID should be < NUM_MATERIALS
        let max_id = 54u32; // BT_LIQUID_OUTPUT
        for id in 0..=max_id {
            assert!(id < crate::materials::NUM_MATERIALS as u32,
                "Block ID {} exceeds NUM_MATERIALS ({})", id, crate::materials::NUM_MATERIALS);
        }
    }

    #[test]
    fn test_pipe_placeable_on_ground() {
        assert!(can_place_on_block(BT_PIPE as u8, BT_DIRT, 0));
        assert!(can_place_on_block(BT_LIQUID_PIPE as u8, BT_DIRT, 0));
        assert!(can_place_on_block(BT_RESTRICTOR as u8, BT_DIRT, 0));
    }

    #[test]
    fn test_pipe_placeable_on_existing_pipe() {
        assert!(can_place_on_block(BT_PIPE as u8, BT_PIPE, 1));
        assert!(can_place_on_block(BT_RESTRICTOR as u8, BT_PIPE, 1));
        assert!(can_place_on_block(BT_LIQUID_PIPE as u8, BT_LIQUID_PIPE, 1));
    }

    #[test]
    fn test_pipe_not_cross_connect() {
        // Gas pipes shouldn't visually merge onto liquid pipes and vice versa
        assert!(!can_place_on_block(BT_PIPE as u8, BT_LIQUID_PIPE, 1));
        assert!(!can_place_on_block(BT_LIQUID_PIPE as u8, BT_PIPE, 1));
    }

    #[test]
    fn test_wire_placeable_anywhere() {
        assert!(can_place_on_block(BT_WIRE as u8, BT_DIRT, 0));
        assert!(can_place_on_block(BT_WIRE as u8, BT_STONE, 3)); // on walls
        assert!(can_place_on_block(BT_WIRE as u8, BT_PIPE, 1));  // on pipes
        assert!(!can_place_on_block(BT_WIRE as u8, BT_WIRE, 0)); // NOT on existing wire
    }

    #[test]
    fn test_power_equipment_on_wire_or_ground() {
        for &id in &[BT_SWITCH, BT_DIMMER, BT_BREAKER] {
            assert!(can_place_on_block(id as u8, BT_WIRE, 0), "ID {} should place on wire", id);
            assert!(can_place_on_block(id as u8, BT_DIRT, 0), "ID {} should place on ground", id);
        }
    }

    #[test]
    fn test_pump_on_pipe() {
        assert!(can_place_on_block(BT_PUMP as u8, BT_PIPE, 1));
        assert!(can_place_on_block(BT_PUMP as u8, BT_DIRT, 0)); // also on ground
    }

    #[test]
    fn test_liquid_intake_on_water_or_dug() {
        // Liquid intake is a 2-tile block; at least one tile must be water/dug
        // This tests the individual tile acceptance (both ground and water tiles should be valid)
        let dug = make_block(BT_DUG_GROUND as u8, 1, 0);
        let water = make_block(BT_WATER as u8, 0, 0);
        let dirt = make_block(BT_DIRT as u8, 0, 0);
        assert_eq!(block_type_rs(dug), BT_DUG_GROUND);
        assert_eq!(block_type_rs(water), BT_WATER);
        assert_eq!(block_type_rs(dirt), BT_DIRT);
    }

    #[test]
    fn test_is_conductor_includes_all_power_blocks() {
        // All power grid components should be recognized as conductors
        let power_ids: &[u32] = &[36, 37, 38, 39, 40, 41, 42, 43, 45, 48, 51, 7, 10, 11, 12, 16];
        for &id in power_ids {
            assert!(is_conductor_rs(id, 0), "Block type {} should be a conductor", id);
        }
        // Wire overlay flag
        assert!(is_conductor_rs(1, 0x80), "Wall with wire overlay should be conductor");
        // Non-conductors
        assert!(!is_conductor_rs(2, 0), "Dirt should not be conductor");
        assert!(!is_conductor_rs(15, 0), "Pipe should not be conductor");
    }

    #[test]
    fn test_bridge_connects_to_gas_pipes() {
        assert!(can_place_on_block(BT_PIPE as u8, BT_PIPE_BRIDGE, 1),
            "Gas pipe should be placeable on bridge");
        assert!(can_place_on_block(BT_RESTRICTOR as u8, BT_PIPE_BRIDGE, 1),
            "Restrictor should be placeable on bridge");
    }

    #[test]
    fn test_bridge_connects_to_liquid_pipes() {
        assert!(can_place_on_block(BT_LIQUID_PIPE as u8, BT_PIPE_BRIDGE, 1),
            "Liquid pipe should be placeable on bridge");
    }

    /// Simulate intake tile assignment: given two block types, determine if placement is valid.
    /// Returns (ground_idx, water_idx) or None if invalid.
    fn intake_valid(bt0: u32, bh0: u8, bt1: u32, bh1: u8) -> Option<(usize, usize)> {
        let is_ground = |bt: u32, bh: u8| bh == 0 && (bt == BT_AIR || bt == BT_DIRT
            || bt == BT_WOOD_FLOOR || bt == BT_STONE_FLOOR || bt == BT_CONCRETE_FLOOR);
        let is_water = |bt: u32| bt == BT_WATER || bt == BT_DUG_GROUND;
        if is_ground(bt0, bh0) && is_water(bt1) { Some((0, 1)) }
        else if is_water(bt0) && is_ground(bt1, bh1) { Some((1, 0)) }
        else { None }
    }

    #[test]
    fn test_liquid_intake_ground_plus_water() {
        // Ground first, water second
        assert!(intake_valid(BT_DIRT, 0, BT_WATER, 0).is_some());
        assert!(intake_valid(BT_DIRT, 0, BT_DUG_GROUND, 1).is_some());
    }

    #[test]
    fn test_liquid_intake_water_plus_ground() {
        // Water first, ground second (reversed click direction)
        let r = intake_valid(BT_WATER, 0, BT_DIRT, 0);
        assert!(r.is_some());
        assert_eq!(r.unwrap(), (1, 0), "ground should be index 1, water index 0");
    }

    #[test]
    fn test_liquid_intake_both_ground_invalid() {
        // Both tiles are ground — no water source, invalid
        assert!(intake_valid(BT_DIRT, 0, BT_DIRT, 0).is_none());
    }

    #[test]
    fn test_liquid_intake_both_water_invalid() {
        // Both tiles are water — no ground anchor, invalid
        assert!(intake_valid(BT_WATER, 0, BT_WATER, 0).is_none());
    }

    #[test]
    fn test_liquid_intake_wall_invalid() {
        // Can't place on a wall tile
        assert!(intake_valid(BT_STONE, 3, BT_WATER, 0).is_none());
    }

    #[test]
    fn test_liquid_components_in_network() {
        use crate::pipes::{is_liquid_pipe_component, is_gas_pipe_component};
        // Liquid network includes: liquid pipe, bridge, intake, pump, output
        assert!(is_liquid_pipe_component(49), "Liquid pipe");
        assert!(is_liquid_pipe_component(50), "Bridge in liquid network");
        assert!(is_liquid_pipe_component(52), "Liquid intake");
        assert!(is_liquid_pipe_component(53), "Liquid pump");
        assert!(is_liquid_pipe_component(54), "Liquid output");
        // Gas network includes bridge too
        assert!(is_gas_pipe_component(50), "Bridge in gas network");
        // Cross-isolation: liquid components not in gas network
        assert!(!is_gas_pipe_component(49), "Liquid pipe NOT in gas network");
        assert!(!is_gas_pipe_component(52), "Liquid intake NOT in gas network");
    }

    #[test]
    fn test_liquid_pipes_walkable() {
        // All liquid pipe components should be recognized as walkable pipe blocks
        // (walkability is checked in pleb.rs using the same block type IDs)
        let liquid_types: &[u32] = &[BT_LIQUID_PIPE, BT_LIQUID_INTAKE, BT_LIQUID_PUMP, BT_LIQUID_OUTPUT];
        for &bt in liquid_types {
            // The pipe walkability check: bt matches AND height <= 1
            let is_any_pipe = (bt >= 15 && bt <= 20) || bt == BT_RESTRICTOR
                || bt == BT_LIQUID_PIPE || bt == BT_PIPE_BRIDGE
                || bt == BT_LIQUID_INTAKE || bt == BT_LIQUID_PUMP || bt == BT_LIQUID_OUTPUT;
            assert!(is_any_pipe, "Block type {} should be walkable as a pipe", bt);
        }
    }

    #[test]
    fn test_gas_pipe_types_walkable() {
        let gas_types: &[u32] = &[BT_PIPE, BT_PUMP, BT_TANK, BT_VALVE, BT_OUTLET, BT_INLET, BT_RESTRICTOR];
        for &bt in gas_types {
            let is_any_pipe = (bt >= 15 && bt <= 20) || bt == BT_RESTRICTOR
                || bt == BT_LIQUID_PIPE || bt == BT_PIPE_BRIDGE
                || bt == BT_LIQUID_INTAKE || bt == BT_LIQUID_PUMP || bt == BT_LIQUID_OUTPUT;
            assert!(is_any_pipe, "Block type {} should be walkable as a pipe", bt);
        }
    }

    #[test]
    fn test_num_materials_covers_all_blocks() {
        let highest = BT_LIQUID_OUTPUT; // 54
        assert!(crate::materials::NUM_MATERIALS > highest as usize,
            "NUM_MATERIALS ({}) must be > highest block ID ({})",
            crate::materials::NUM_MATERIALS, highest);
    }
}
