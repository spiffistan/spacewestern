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
pub const BT_WALL_TORCH: u32 = 55;
pub const BT_WALL_LAMP: u32 = 56;
pub const BT_WORKBENCH: u32 = 57;
pub const BT_KILN: u32 = 58;
pub const BT_WELL: u32 = 59;
pub const BT_ROUGH_FLOOR: u32 = 60;
pub const BT_SAW_HORSE: u32 = 61;

/// Generate WGSL `const BT_*: u32 = N;` lines for all block type constants.
/// Prepend this to shader source so WGSL can use the same names as Rust.
pub fn wgsl_block_constants() -> String {
    let mut s = String::from("// --- Block type constants (generated from grid.rs) ---\n");
    let consts: &[(&str, u32)] = &[
        ("BT_AIR", BT_AIR),
        ("BT_STONE", BT_STONE),
        ("BT_DIRT", BT_DIRT),
        ("BT_WATER", BT_WATER),
        ("BT_WALL", BT_WALL),
        ("BT_GLASS", BT_GLASS),
        ("BT_FIREPLACE", BT_FIREPLACE),
        ("BT_CEILING_LIGHT", BT_CEILING_LIGHT),
        ("BT_TREE", BT_TREE),
        ("BT_BENCH", BT_BENCH),
        ("BT_FLOOR_LAMP", BT_FLOOR_LAMP),
        ("BT_TABLE_LAMP", BT_TABLE_LAMP),
        ("BT_FAN", BT_FAN),
        ("BT_COMPOST", BT_COMPOST),
        ("BT_INSULATED", BT_INSULATED),
        ("BT_PIPE", BT_PIPE),
        ("BT_PUMP", BT_PUMP),
        ("BT_TANK", BT_TANK),
        ("BT_VALVE", BT_VALVE),
        ("BT_OUTLET", BT_OUTLET),
        ("BT_INLET", BT_INLET),
        ("BT_WOOD_WALL", BT_WOOD_WALL),
        ("BT_STEEL_WALL", BT_STEEL_WALL),
        ("BT_SANDSTONE", BT_SANDSTONE),
        ("BT_GRANITE", BT_GRANITE),
        ("BT_LIMESTONE", BT_LIMESTONE),
        ("BT_WOOD_FLOOR", BT_WOOD_FLOOR),
        ("BT_STONE_FLOOR", BT_STONE_FLOOR),
        ("BT_CONCRETE_FLOOR", BT_CONCRETE_FLOOR),
        ("BT_CANNON", BT_CANNON),
        ("BT_BED", BT_BED),
        ("BT_BERRY_BUSH", BT_BERRY_BUSH),
        ("BT_DUG_GROUND", BT_DUG_GROUND),
        ("BT_CRATE", BT_CRATE),
        ("BT_ROCK", BT_ROCK),
        ("BT_MUD_WALL", BT_MUD_WALL),
        ("BT_WIRE", BT_WIRE),
        ("BT_SOLAR", BT_SOLAR),
        ("BT_BATTERY_S", BT_BATTERY_S),
        ("BT_BATTERY_M", BT_BATTERY_M),
        ("BT_BATTERY_L", BT_BATTERY_L),
        ("BT_WIND_TURBINE", BT_WIND_TURBINE),
        ("BT_SWITCH", BT_SWITCH),
        ("BT_DIMMER", BT_DIMMER),
        ("BT_DIAGONAL", BT_DIAGONAL),
        ("BT_BREAKER", BT_BREAKER),
        ("BT_RESTRICTOR", BT_RESTRICTOR),
        ("BT_CROP", BT_CROP),
        ("BT_FLOODLIGHT", BT_FLOODLIGHT),
        ("BT_LIQUID_PIPE", BT_LIQUID_PIPE),
        ("BT_PIPE_BRIDGE", BT_PIPE_BRIDGE),
        ("BT_WIRE_BRIDGE", BT_WIRE_BRIDGE),
        ("BT_LIQUID_INTAKE", BT_LIQUID_INTAKE),
        ("BT_LIQUID_PUMP", BT_LIQUID_PUMP),
        ("BT_LIQUID_OUTPUT", BT_LIQUID_OUTPUT),
        ("BT_WALL_TORCH", BT_WALL_TORCH),
        ("BT_WALL_LAMP", BT_WALL_LAMP),
        ("BT_WORKBENCH", BT_WORKBENCH),
        ("BT_KILN", BT_KILN),
        ("BT_WELL", BT_WELL),
        ("BT_ROUGH_FLOOR", BT_ROUGH_FLOOR),
        ("BT_SAW_HORSE", BT_SAW_HORSE),
    ];
    for &(name, val) in consts {
        s.push_str(&format!("const {}: u32 = {}u;\n", name, val));
    }
    s.push('\n');
    s
}

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

/// Get the block height byte. Wall edge bitmask masking is no longer needed
/// since walls are stored in wall_data (DN-008), not the block grid.
/// Legacy wall blocks in grid_data still have edge bits, but extraction
/// handles them; callers should use wall_data for wall edge info.
pub fn block_height_rs(b: u32) -> u8 {
    let h = ((b >> 8) & 0xFF) as u8;
    let bt = block_type_rs(b);
    // Legacy compat: mask wall type heights to lower 4 bits
    if is_wall_block(bt) { h & 0x0F } else { h }
}

/// Get the raw height byte (including edge bitmask for walls, connection mask for pipes).
pub fn block_height_raw(b: u32) -> u8 {
    ((b >> 8) & 0xFF) as u8
}

pub fn roof_height_rs(b: u32) -> u8 {
    ((b >> 24) & 0xFF) as u8
}

/// Extract roof flag (bit 1 of flags byte) and roof height (top byte), for use
/// when replacing a block but preserving its roof data.
pub fn extract_roof_data(block: u32) -> (u8, u32) {
    let roof_flag = block_flags_rs(block) & 2;
    let roof_h = block & 0xFF000000;
    (roof_flag, roof_h)
}

// =============================================================
// Wall data layer (u16 per tile) — independent of block grid.
// See DN-008 for full architecture.
// =============================================================
// bits 0-3:   edge bitmask (bit0=N, bit1=E, bit2=S, bit3=W)
// bits 4-5:   thickness (0=full/4, 1→3, 2→2, 3→1 sub-cell)
// bits 6-9:   wall material index (0-15)
// bit 10:     has_door
// bit 11:     door_is_open
// bit 12:     has_window
// bits 13-15: reserved

pub const WD_EDGE_N: u16 = 0x0001;
pub const WD_EDGE_E: u16 = 0x0002;
pub const WD_EDGE_S: u16 = 0x0004;
pub const WD_EDGE_W: u16 = 0x0008;
pub const WD_EDGE_MASK: u16 = 0x000F;
pub const WD_THICK_SHIFT: u32 = 4;
pub const WD_MAT_SHIFT: u32 = 6;
pub const WD_HAS_DOOR: u16 = 0x0400;
pub const WD_DOOR_OPEN: u16 = 0x0800;
pub const WD_HAS_WINDOW: u16 = 0x1000;

/// Wall material indices
pub const WMAT_STONE: u16 = 0;
pub const WMAT_GENERIC: u16 = 1;
pub const WMAT_GLASS: u16 = 2;
pub const WMAT_INSULATED: u16 = 3;
pub const WMAT_WOOD: u16 = 4;
pub const WMAT_STEEL: u16 = 5;
pub const WMAT_SANDSTONE: u16 = 6;
pub const WMAT_GRANITE: u16 = 7;
pub const WMAT_LIMESTONE: u16 = 8;
pub const WMAT_MUD: u16 = 9;

/// Map wall block type to wall material index
pub fn wall_block_to_material(bt: u32) -> u16 {
    match bt {
        BT_STONE => WMAT_STONE,
        BT_WALL => WMAT_GENERIC,
        BT_GLASS => WMAT_GLASS,
        BT_INSULATED => WMAT_INSULATED,
        BT_WOOD_WALL => WMAT_WOOD,
        BT_STEEL_WALL => WMAT_STEEL,
        BT_SANDSTONE => WMAT_SANDSTONE,
        BT_GRANITE => WMAT_GRANITE,
        BT_LIMESTONE => WMAT_LIMESTONE,
        BT_MUD_WALL => WMAT_MUD,
        _ => WMAT_GENERIC,
    }
}

/// Pack wall data into u16
pub fn pack_wall_data(edges: u16, thickness: u16, material: u16) -> u16 {
    (edges & WD_EDGE_MASK)
        | (((if thickness >= 4 { 0u16 } else { 4 - thickness }) & 3) << WD_THICK_SHIFT)
        | ((material & 0xF) << WD_MAT_SHIFT)
}

/// Read edge bitmask from wall_data
pub fn wd_edges(wd: u16) -> u16 {
    wd & WD_EDGE_MASK
}
/// Read thickness raw (0=full, 1→3, 2→2, 3→1)
pub fn wd_thickness_raw(wd: u16) -> u16 {
    (wd >> WD_THICK_SHIFT) & 3
}
/// Read thickness as sub-cell count (1-4)
pub fn wd_thickness(wd: u16) -> u16 {
    let raw = wd_thickness_raw(wd);
    if raw == 0 { 4 } else { 4 - raw }
}
/// Read material index
pub fn wd_material(wd: u16) -> u16 {
    (wd >> WD_MAT_SHIFT) & 0xF
}

/// Check if wall_data has a wall on the given edge (0=N, 1=E, 2=S, 3=W)
pub fn wd_has_edge(wd: u16, edge: u8) -> bool {
    if wd == 0 {
        return false;
    }
    let edges = wd_edges(wd);
    if edges == 0 && wd_thickness_raw(wd) == 0 {
        return true;
    } // full wall compat
    (edges & (1 << edge)) != 0
}

/// Check if crossing from (ax,ay) to (bx,by) is blocked by wall_data.
pub fn wd_edge_blocked(wall_data: &[u16], ax: i32, ay: i32, bx: i32, by: i32) -> bool {
    let dx = bx - ax;
    let dy = by - ay;
    let dir_a = if dy < 0 {
        0u8
    } else if dx > 0 {
        1
    } else if dy > 0 {
        2
    } else {
        3
    };
    let dir_b = (dir_a + 2) % 4;
    let gw = GRID_W as i32;
    let gh = GRID_H as i32;

    if ax >= 0 && ay >= 0 && ax < gw && ay < gh {
        let wd = wall_data[(ay as u32 * GRID_W + ax as u32) as usize];
        if wd != 0 {
            let is_open_door = (wd & WD_HAS_DOOR) != 0 && (wd & WD_DOOR_OPEN) != 0;
            if !is_open_door && wd_has_edge(wd, dir_a) {
                return true;
            }
        }
    }
    if bx >= 0 && by >= 0 && bx < gw && by < gh {
        let wd = wall_data[(by as u32 * GRID_W + bx as u32) as usize];
        if wd != 0 {
            let is_open_door = (wd & WD_HAS_DOOR) != 0 && (wd & WD_DOOR_OPEN) != 0;
            if !is_open_door && wd_has_edge(wd, dir_b) {
                return true;
            }
        }
    }
    false
}

/// Extract wall_data from the block grid (migration from legacy encoding).
/// Scans for wall block types and converts their edge/thickness/material into wall_data.
pub fn extract_wall_data_from_grid(grid: &[u32]) -> Vec<u16> {
    let size = (GRID_W * GRID_H) as usize;
    let mut wd = vec![0u16; size];
    for i in 0..size.min(grid.len()) {
        let block = grid[i];
        let bt = block_type_rs(block);
        if !is_wall_block(bt) {
            continue;
        }
        let h_raw = block_height_raw(block);
        let flags = block_flags_rs(block);
        let visual_h = block_height_rs(block);
        if visual_h == 0 {
            continue;
        } // no wall
        // Read edge mask from height byte upper bits
        let edge_mask = wall_edge_mask(h_raw);
        let edges = if edge_mask == 0 {
            // Full wall (legacy): all edges
            WD_EDGE_N | WD_EDGE_E | WD_EDGE_S | WD_EDGE_W
        } else {
            // Convert height-byte edge mask (bits 4-7) to wall_data edge mask (bits 0-3)
            ((edge_mask >> 4) as u16) & WD_EDGE_MASK
        };
        // Read thickness from flags byte
        let thick_raw = (flags >> 5) & 3;
        let thickness: u16 = if thick_raw == 0 {
            4
        } else {
            (4 - thick_raw) as u16
        };
        // Material from block type
        let material = wall_block_to_material(bt);
        // Door flags
        let is_door = (flags & 1) != 0;
        let is_open = (flags & 4) != 0;

        let mut w = pack_wall_data(edges, thickness, material);
        if is_door {
            w |= WD_HAS_DOOR;
        }
        if is_open {
            w |= WD_DOOR_OPEN;
        }
        if bt == BT_GLASS {
            w |= WD_HAS_WINDOW;
        }
        wd[i] = w;
    }
    wd
}

// --- Physical Doors ---

pub const MAX_DOORS: usize = 64;
pub const DOOR_MAX_ANGLE: f32 = 2.967; // ~170 degrees
pub const DOOR_OPEN_THRESHOLD: f32 = 0.524; // ~30 degrees — passable for pathfinding/gas

#[derive(Clone, Debug)]
pub struct Door {
    pub x: i32,
    pub y: i32,
    pub edge: u8,         // 0=N, 1=E, 2=S, 3=W
    pub angle: f32,       // 0.0=closed, DOOR_MAX_ANGLE=fully open
    pub angular_vel: f32, // rad/s
    pub hinge_side: u8,   // 0=left, 1=right (relative to facing inside)
    pub locked: bool,
    pub material: u8, // wall material index (color)
}

impl Door {
    pub fn new(x: i32, y: i32, edge: u8, hinge_side: u8, material: u8) -> Self {
        Self {
            x,
            y,
            edge,
            angle: 0.0,
            angular_vel: 0.0,
            hinge_side,
            locked: false,
            material,
        }
    }

    pub fn is_passable(&self) -> bool {
        self.angle > DOOR_OPEN_THRESHOLD
    }

    /// Pack for GPU upload (2 x u32).
    pub fn pack_gpu(&self) -> [u32; 2] {
        let w0 = (self.x as u32 & 0xFF)
            | ((self.y as u32 & 0xFF) << 8)
            | ((self.edge as u32 & 3) << 16)
            | ((self.hinge_side as u32 & 1) << 18)
            | (if self.locked { 1u32 } else { 0 } << 19)
            | ((self.material as u32 & 0xF) << 20);
        let w1 = self.angle.to_bits();
        [w0, w1]
    }
}

/// Scan wall_data for tiles with WD_HAS_DOOR and create Door structs.
/// Used after world generation to populate the doors list.
pub fn extract_doors_from_wall_data(wall_data: &[u16]) -> Vec<Door> {
    let mut doors = Vec::new();
    for i in 0..wall_data.len() {
        let wd = wall_data[i];
        if (wd & WD_HAS_DOOR) == 0 {
            continue;
        }
        let x = (i % GRID_W as usize) as i32;
        let y = (i / GRID_W as usize) as i32;
        let edges = wd & WD_EDGE_MASK;
        // Determine which edge the door is on (pick first set edge)
        let edge = if edges & WD_EDGE_N != 0 {
            0
        } else if edges & WD_EDGE_E != 0 {
            1
        } else if edges & WD_EDGE_S != 0 {
            2
        } else {
            3
        };
        let material = wd_material(wd) as u8;
        let is_open = (wd & WD_DOOR_OPEN) != 0;
        let mut door = Door::new(x, y, edge, 0, material);
        if is_open {
            door.angle = DOOR_OPEN_THRESHOLD + 0.1;
        }
        doors.push(door);
    }
    doors
}

// =============================================================
// Legacy thin wall encoding (in block height byte, still used
// during migration). Will be removed once wall_data is primary.
// =============================================================

/// Thin wall encoding (wall blocks only):
///
/// Height byte: bits 0-3 = wall height (0-15), bits 4-7 = edge bitmask
///   bit 4 = N edge, bit 5 = E edge, bit 6 = S edge, bit 7 = W edge
///   Edge bitmask 0 = full wall (all edges, backward compatible)
///
/// Flags byte: bits 5-6 = thickness (0=full/4, 1→3, 2→2, 3→1 sub-cell)
///   Thickness 0 (full) = entire tile is wall (backward compatible)
///
/// This encoding supports any combination of edges (T-junctions, crosses,
/// single edges, corners, opposite pairs) using 4 independent bits.

/// Edge bitmask constants for wall height byte
pub const WALL_EDGE_N: u8 = 0x10; // bit 4
pub const WALL_EDGE_E: u8 = 0x20; // bit 5
pub const WALL_EDGE_S: u8 = 0x40; // bit 6
pub const WALL_EDGE_W: u8 = 0x80; // bit 7

/// Convert edge index (0=N, 1=E, 2=S, 3=W) to bitmask
pub fn edge_to_mask(edge: u8) -> u8 {
    WALL_EDGE_N << (edge & 3)
}

/// Create wall block height byte: base height + edge bitmask
pub fn make_wall_height(base_height: u8, edge_mask: u8) -> u8 {
    (base_height & 0x0F) | (edge_mask & 0xF0)
}

/// Extract actual wall height (lower 4 bits of height byte)
pub fn wall_height(height_byte: u8) -> u8 {
    height_byte & 0x0F
}

/// Extract edge bitmask (upper 4 bits of height byte)
pub fn wall_edge_mask(height_byte: u8) -> u8 {
    height_byte & 0xF0
}

/// Create thin wall flags (flags byte): thickness + roof/door flags
pub fn make_thin_wall_flags(roof_flag: u8, edge: u8, thickness: u8) -> (u8, u8) {
    let thick_bits = if thickness >= 4 { 0u8 } else { 4 - thickness };
    let flags = roof_flag | ((thick_bits & 3) << 5);
    let edge_mask = edge_to_mask(edge);
    (flags, edge_mask)
}

/// Create thin wall corner flags: two adjacent edges
pub fn make_thin_wall_corner_flags(roof_flag: u8, edge: u8, thickness: u8) -> (u8, u8) {
    let thick_bits = if thickness >= 4 { 0u8 } else { 4 - thickness };
    let flags = roof_flag | ((thick_bits & 3) << 5);
    let edge_mask = edge_to_mask(edge) | edge_to_mask((edge + 1) & 3);
    (flags, edge_mask)
}

/// Does a wall block have a wall on the given edge?
/// edge: 0=N, 1=E, 2=S, 3=W
/// height_byte: the full height byte of the block
/// flags: the flags byte (for thickness check)
pub fn has_wall_on_edge(height_byte: u8, flags: u8, edge: u8) -> bool {
    let thick_raw = (flags >> 5) & 3;
    if thick_raw == 0 {
        return true; // full wall, blocks all edges
    }
    let edge_mask = wall_edge_mask(height_byte);
    if edge_mask == 0 {
        return true; // no edges set = full wall (backward compat)
    }
    (edge_mask & edge_to_mask(edge)) != 0
}

/// Is movement between adjacent tiles blocked by a thin wall edge?
/// Checks both tiles: if either has a wall on the shared edge, crossing is blocked.
/// Open doors on the shared edge make it passable.
/// Direction: 0=N (from→north neighbor), 1=E, 2=S, 3=W.
/// Check edge blocking using both block grid (legacy) and wall_data layer.
/// Prefers wall_data when available, falls back to block grid.
pub fn edge_blocked_wd(
    grid: &[u32],
    wall_data: &[u16],
    ax: i32,
    ay: i32,
    bx: i32,
    by: i32,
) -> bool {
    // Check wall_data layer first (DN-008)
    if !wall_data.is_empty() && wd_edge_blocked(wall_data, ax, ay, bx, by) {
        return true;
    }
    // Fall back to block grid for blocks not in wall_data (legacy, diagonal walls, etc.)
    edge_blocked_grid(grid, ax, ay, bx, by)
}

/// Legacy: check edge blocking from block grid only.
pub fn edge_blocked(grid: &[u32], ax: i32, ay: i32, bx: i32, by: i32) -> bool {
    edge_blocked_grid(grid, ax, ay, bx, by)
}

fn edge_blocked_grid(grid: &[u32], ax: i32, ay: i32, bx: i32, by: i32) -> bool {
    let dx = bx - ax;
    let dy = by - ay;
    // Determine crossing direction from A's perspective
    let dir_from_a = if dy < 0 {
        0u8 // moving north
    } else if dx > 0 {
        1 // moving east
    } else if dy > 0 {
        2 // moving south
    } else {
        3 // moving west
    };
    let dir_from_b = (dir_from_a + 2) % 4; // opposite direction

    let gw = GRID_W as i32;
    let gh = GRID_H as i32;

    // Check tile A: does it have a wall on the exit edge?
    if ax >= 0 && ay >= 0 && ax < gw && ay < gh {
        let a_block = grid[(ay as u32 * GRID_W + ax as u32) as usize];
        let a_bt = block_type_rs(a_block);
        let a_flags = block_flags_rs(a_block);
        let a_height_raw = block_height_raw(a_block);
        let a_height = block_height_rs(a_block);
        if a_height > 0 && is_wall_block(a_bt) {
            // Open door: not blocked
            let a_is_door = (a_flags & 1) != 0;
            let a_is_open = (a_flags & 4) != 0;
            if !(a_is_door && a_is_open) && has_wall_on_edge(a_height_raw, a_flags, dir_from_a) {
                return true;
            }
        }
    }

    // Check tile B: does it have a wall on the entry edge?
    if bx >= 0 && by >= 0 && bx < gw && by < gh {
        let b_block = grid[(by as u32 * GRID_W + bx as u32) as usize];
        let b_bt = block_type_rs(b_block);
        let b_flags = block_flags_rs(b_block);
        let b_height_raw = block_height_raw(b_block);
        let b_height = block_height_rs(b_block);
        if b_height > 0 && is_wall_block(b_bt) {
            let b_is_door = (b_flags & 1) != 0;
            let b_is_open = (b_flags & 4) != 0;
            if !(b_is_door && b_is_open) && has_wall_on_edge(b_height_raw, b_flags, dir_from_b) {
                return true;
            }
        }
    }

    false
}

/// Is a thin wall tile walkable? (has open sub-cells that can be traversed)
pub fn thin_wall_is_walkable(block: u32) -> bool {
    let flags = block_flags_rs(block);
    let thick_raw = (flags >> 5) & 3;
    let height_raw = block_height_raw(block);
    let edge_mask = wall_edge_mask(height_raw);
    // Full wall (thick_raw=0 or edge_mask=0) is not walkable.
    // Thin wall with edges has open space for walking.
    thick_raw != 0 && edge_mask != 0
}

/// Is this block type part of the electrical power network?
/// Checks block type and wire overlay flag. Matches the GPU-side is_conductor() in power.wgsl.
pub fn is_conductor_rs(bt: u32, flags: u8) -> bool {
    bt_is!(
        bt,
        BT_WIRE,
        BT_SOLAR,
        BT_BATTERY_S,
        BT_BATTERY_M,
        BT_BATTERY_L,
        BT_WIND_TURBINE,
        BT_SWITCH,
        BT_DIMMER,
        BT_BREAKER,
        BT_FLOODLIGHT,
        BT_WIRE_BRIDGE,
        BT_WALL_LAMP,
        BT_CEILING_LIGHT,
        BT_FLOOR_LAMP,
        BT_TABLE_LAMP,
        BT_FAN,
        BT_PUMP
    ) || (flags & 0x80) != 0
}

/// Is this block type a ground/floor tile (walkable base, not a placed object)?
pub fn is_ground_block(bt: u32) -> bool {
    bt_is!(
        bt,
        BT_AIR,
        BT_DIRT,
        BT_WATER,
        BT_WOOD_FLOOR,
        BT_STONE_FLOOR,
        BT_CONCRETE_FLOOR,
        BT_ROUGH_FLOOR,
        BT_DUG_GROUND
    )
}

/// Is this block type a structural wall?
pub fn is_wall_block(bt: u32) -> bool {
    bt_is!(
        bt,
        BT_STONE,
        BT_WALL,
        BT_GLASS,
        BT_INSULATED,
        BT_WOOD_WALL,
        BT_STEEL_WALL,
        BT_SANDSTONE,
        BT_GRANITE,
        BT_LIMESTONE,
        BT_MUD_WALL,
        BT_DIAGONAL
    )
}

/// Is this block type a wire/power equipment (height byte = connection mask, not visual)?
pub fn is_wire_block(bt: u32) -> bool {
    bt_is!(
        bt,
        BT_WIRE,
        BT_DIMMER,
        BT_SWITCH,
        BT_BREAKER,
        BT_WIRE_BRIDGE
    )
}

/// Direction mask constants for pipe/wire connections.
/// Encoded in height byte upper nibble: N=0x10, E=0x20, S=0x40, W=0x80.
/// After >> 4: N=1, E=2, S=4, W=8.
pub const DIR_MASKS: [(i32, i32, u32); 4] = [(0, -1, 0x1), (0, 1, 0x4), (1, 0, 0x2), (-1, 0, 0x8)];

pub fn is_door_rs(b: u32) -> bool {
    (block_flags_rs(b) & 1) != 0
}

/// Precompute roof heights and store in bits 24-31 of each block.
/// For every tile that's part of a roofed building, find the max wall height
/// in a large radius. This runs once at grid generation.
pub fn compute_roof_heights(grid: &mut Vec<u32>) {
    compute_roof_heights_wd(grid, &[]);
}

/// Compute roof heights using both grid and wall_data.
pub fn compute_roof_heights_wd(grid: &mut Vec<u32>, wall_data: &[u16]) {
    let w = GRID_W as i32;
    let h = GRID_H as i32;

    // Pass 1: identify which tiles are part of a roofed building
    let mut is_building = vec![false; grid.len()];
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            let block = grid[idx];
            let flags = block_flags_rs(block);
            let has_wd = idx < wall_data.len() && wd_edges(wall_data[idx]) != 0;

            if (flags & 2) != 0 {
                // Has roof flag
                is_building[idx] = true;
            } else if (block >> 8) & 0xFF > 0 || (flags & 1) != 0 || has_wd {
                // Has height, is a door, or has wall_data — check if adjacent to a roofed tile
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
                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                        break;
                    }
                    let nidx = (ny * w + nx) as usize;
                    let nb = grid[nidx];
                    let nbh = ((nb >> 8) & 0xFF) as u8;
                    let nbt = (nb & 0xFF) as u8;
                    let nb_flags = ((nb >> 16) & 0xFF) as u8;
                    // Wall: has height, not roofed floor, not tree/fire/light/wire/dimmer/crate
                    // Wire(36), dimmer(43), varistor(47), restrictor(46) use height for level, not visual
                    // Crate(33) uses height for item count
                    let skip = bt_is!(
                        nbt as u32,
                        BT_TREE,
                        BT_FIREPLACE,
                        BT_CEILING_LIGHT,
                        BT_CRATE,
                        BT_WIRE,
                        BT_DIMMER,
                        BT_RESTRICTOR,
                        BT_LIQUID_PIPE,
                        BT_PIPE_BRIDGE,
                        BT_WIRE_BRIDGE,
                        BT_LIQUID_INTAKE,
                        BT_LIQUID_PUMP,
                        BT_LIQUID_OUTPUT
                    );
                    // Check wall_data layer: wall edges count as walls
                    let n_has_wd = nidx < wall_data.len() && wd_edges(wall_data[nidx]) != 0;
                    if n_has_wd && (nb_flags & 2) == 0 {
                        // Wall from wall_data: use material height (typically 3)
                        max_h = max_h.max(3);
                        break;
                    }
                    if nbh > 0 && (nb_flags & 2) == 0 && !skip {
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

/// Generate a natural world with trees, bushes, and rocks.
pub fn generate_world(seed: u32) -> Vec<u32> {
    let mut grid = vec![make_block(2, 0, 0); (GRID_W * GRID_H) as usize];
    let w = GRID_W;

    let set = |grid: &mut Vec<u32>, x: u32, y: u32, b: u32| {
        if x < GRID_W && y < GRID_H {
            grid[(y * w + x) as usize] = b;
        }
    };

    // Trees and bushes — clustered in small forests with outliers
    let noise = |x: f32, y: f32| -> f32 {
        let ix = x.floor() as i32;
        let iy = y.floor() as i32;
        let fx = x - x.floor();
        let fy = y - y.floor();
        let hash = |ix: i32, iy: i32| -> f32 {
            let h = ((ix.wrapping_mul(374761393) as u32) ^ (iy.wrapping_mul(668265263) as u32))
                .wrapping_add(1013904223)
                .wrapping_add(seed);
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
        if x >= GRID_W || y >= GRID_H {
            return false;
        }
        grid[(y * w + x) as usize] == make_block(2, 0, 0)
    };

    for y in 0..GRID_H {
        for x in 0..GRID_W {
            let idx = (y * w + x) as usize;
            if grid[idx] != make_block(2, 0, 0) {
                continue;
            }

            // Forest density from multi-octave noise (scale creates ~30-tile clusters)
            let scale = 0.07;
            let n1 = noise(x as f32 * scale, y as f32 * scale);
            let n2 = noise(
                x as f32 * scale * 2.3 + 100.0,
                y as f32 * scale * 2.3 + 200.0,
            ) * 0.5;
            let density = n1 + n2; // 0.0 - 1.5 range

            // Per-tile random hash
            let h = ((x.wrapping_mul(374761393)) ^ (y.wrapping_mul(668265263)))
                .wrapping_add(1013904223)
                .wrapping_add(seed);
            let r = (h >> 16) & 0xFFF; // 0..4095

            // Threshold based on density: dense areas have many trees
            let tree_threshold = if density > 0.9 {
                400
            }
            // dense forest
            else if density > 0.7 {
                150
            }
            // moderate forest
            else if density > 0.5 {
                40
            }
            // sparse
            else {
                8
            }; // rare outlier

            if r < tree_threshold {
                if r < tree_threshold / 5 {
                    // Large 2x2 tree
                    if is_bare(&grid, x + 1, y)
                        && is_bare(&grid, x, y + 1)
                        && is_bare(&grid, x + 1, y + 1)
                    {
                        let tree_h = 4 + ((h >> 8) & 0x1) as u8;
                        set(&mut grid, x, y, make_block(8, tree_h, 32 | 0));
                        set(&mut grid, x + 1, y, make_block(8, tree_h, 32 | 8));
                        set(&mut grid, x, y + 1, make_block(8, tree_h, 32 | 16));
                        set(&mut grid, x + 1, y + 1, make_block(8, tree_h, 32 | 24));
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
            let berry_threshold = if density > 0.6 && density < 1.0 {
                15
            } else if density > 0.4 {
                5
            } else {
                1
            };
            if grid[idx] == make_block(2, 0, 0) && berry_r < berry_threshold {
                grid[idx] = make_block(31, 1, 0);
            }

            // Rocks: scattered on bare ground, more common in sparse/open areas
            let rock_r = ((h >> 6) & 0xFFF) as u32;
            let rock_threshold = if density < 0.3 {
                12
            } else if density < 0.5 {
                6
            } else {
                2
            };
            if grid[idx] == make_block(2, 0, 0) && rock_r < rock_threshold {
                grid[idx] = make_block(34, 0, 0);
            }
        }
    }

    grid
}

/// Add sample buildings to an existing world grid for demo/testing.
/// Builds a base near map center: house with power, piping, lighting, crafting.
pub fn generate_sample_buildings(grid: &mut [u32]) {
    let mut wd = vec![0u16; (GRID_W * GRID_H) as usize];
    generate_sample_buildings_wd(grid, &mut wd);
}

/// Generate sample buildings, writing walls to wall_data instead of grid_data.
pub fn generate_sample_buildings_wd(grid: &mut [u32], wall_data: &mut [u16]) {
    let w = GRID_W;
    let roof = 2u8;

    let set = |grid: &mut [u32], x: i32, y: i32, b: u32| {
        if x >= 0 && y >= 0 && x < GRID_W as i32 && y < GRID_H as i32 {
            grid[(y as u32 * w + x as u32) as usize] = b;
        }
    };

    // Helper: place a wall edge in wall_data
    let set_wall = |wd: &mut [u16], x: i32, y: i32, edges: u16, thickness: u16, material: u16| {
        if x >= 0 && y >= 0 && x < GRID_W as i32 && y < GRID_H as i32 {
            let idx = (y as u32 * w + x as u32) as usize;
            let existing = wd[idx];
            let existing_edges = wd_edges(existing);
            let merged = existing_edges | edges;
            wd[idx] = pack_wall_data(merged, thickness, material);
            // Preserve door/window from existing
            wd[idx] |= existing & (WD_HAS_DOOR | WD_DOOR_OPEN | WD_HAS_WINDOW);
        }
    };

    // Helper: place a wall with door
    let set_wall_door =
        |wd: &mut [u16], x: i32, y: i32, edges: u16, thickness: u16, material: u16, open: bool| {
            if x >= 0 && y >= 0 && x < GRID_W as i32 && y < GRID_H as i32 {
                let idx = (y as u32 * w + x as u32) as usize;
                wd[idx] = pack_wall_data(edges, thickness, material) | WD_HAS_DOOR;
                if open {
                    wd[idx] |= WD_DOOR_OPEN;
                }
            }
        };

    // Helper: place a glass wall (window)
    let set_window = |wd: &mut [u16], x: i32, y: i32, edges: u16, thickness: u16| {
        if x >= 0 && y >= 0 && x < GRID_W as i32 && y < GRID_H as i32 {
            let idx = (y as u32 * w + x as u32) as usize;
            wd[idx] = pack_wall_data(edges, thickness, WMAT_GLASS) | WD_HAS_WINDOW;
        }
    };

    let cx = (GRID_W / 2) as i32;
    let cy = (GRID_H / 2) as i32;

    // === Main house (stone, 14x10 exterior) ===
    // Interior: wood floor + roof (covers entire footprint including wall tiles)
    for y in -5..=5 {
        for x in -7..=7 {
            set(
                grid,
                cx + x,
                cy + y,
                make_block(BT_WOOD_FLOOR as u8, 0, roof),
            );
        }
    }
    // Walls in wall_data: north, south, east, west edges of the perimeter
    for x in -7..=7 {
        set_wall(wall_data, cx + x, cy - 5, WD_EDGE_N, 4, WMAT_STONE); // north wall
        set_wall(wall_data, cx + x, cy + 5, WD_EDGE_S, 4, WMAT_STONE); // south wall
    }
    for y in -5..=5 {
        set_wall(wall_data, cx - 7, cy + y, WD_EDGE_W, 4, WMAT_STONE); // west wall
        set_wall(wall_data, cx + 7, cy + y, WD_EDGE_E, 4, WMAT_STONE); // east wall
    }
    // Corners get both edges
    set_wall(
        wall_data,
        cx - 7,
        cy - 5,
        WD_EDGE_N | WD_EDGE_W,
        4,
        WMAT_STONE,
    );
    set_wall(
        wall_data,
        cx + 7,
        cy - 5,
        WD_EDGE_N | WD_EDGE_E,
        4,
        WMAT_STONE,
    );
    set_wall(
        wall_data,
        cx - 7,
        cy + 5,
        WD_EDGE_S | WD_EDGE_W,
        4,
        WMAT_STONE,
    );
    set_wall(
        wall_data,
        cx + 7,
        cy + 5,
        WD_EDGE_S | WD_EDGE_E,
        4,
        WMAT_STONE,
    );
    // Front door (south)
    set_wall_door(wall_data, cx, cy + 5, WD_EDGE_S, 4, WMAT_GENERIC, false);
    // Windows (glass)
    for &wx in &[-4, -3, 3, 4] {
        set_window(wall_data, cx + wx, cy - 5, WD_EDGE_N, 4);
    }
    for &wx in &[-4, 4] {
        set_window(wall_data, cx + wx, cy + 5, WD_EDGE_S, 4);
    }
    // Dividing wall (two rooms) — N/S edges on interior tiles
    for y in -4..=1 {
        set_wall(wall_data, cx, cy + y, WD_EDGE_E | WD_EDGE_W, 4, WMAT_STONE);
    }
    set_wall_door(
        wall_data,
        cx,
        cy + 2,
        WD_EDGE_E | WD_EDGE_W,
        4,
        WMAT_GENERIC,
        false,
    ); // interior door

    // Furniture
    set(grid, cx - 4, cy, make_block(BT_FIREPLACE as u8, 5, roof));
    set(grid, cx - 5, cy - 3, make_block(BT_BED as u8, 0, roof));
    set(
        grid,
        cx - 4,
        cy - 3,
        make_block(BT_BED as u8, 0, roof | (1 << 3)),
    );
    set(grid, cx + 4, cy - 3, make_block(BT_BED as u8, 0, roof));
    set(
        grid,
        cx + 5,
        cy - 3,
        make_block(BT_BED as u8, 0, roof | (1 << 3)),
    );
    set(grid, cx + 3, cy + 2, make_block(BT_BENCH as u8, 1, roof));
    set(
        grid,
        cx + 4,
        cy + 2,
        make_block(BT_BENCH as u8, 1, roof | (1 << 3)),
    );
    set(
        grid,
        cx + 5,
        cy + 2,
        make_block(BT_BENCH as u8, 1, roof | (2 << 3)),
    );
    set(grid, cx - 2, cy + 3, make_block(BT_CRATE as u8, 0, roof));

    // === Power: solar → wire → battery → wire (through wall) → ceiling light ===
    for sy in 0..3i32 {
        for sx in 0..3i32 {
            let flags = ((sx as u8) << 3) | ((sy as u8) << 5);
            set(
                grid,
                cx + 10 + sx,
                cy - 2 + sy,
                make_block(BT_SOLAR as u8, 0, flags),
            );
        }
    }
    set(grid, cx + 9, cy, make_block(BT_BATTERY_S as u8, 1, 0));
    // Wire run: solar → battery → through wall → ceiling light
    for x in [8, 9] {
        set(grid, cx + x, cy - 1, make_block(BT_WIRE as u8, 0xF0, 0));
    }
    for x in [8, 9] {
        set(grid, cx + x, cy, make_block(BT_WIRE as u8, 0xF0, 0));
    }
    // Wire through wall (wall is in wall_data, wire in grid_data)
    set(grid, cx + 7, cy, make_block(BT_WIRE as u8, 0xF0, roof));
    // Interior wiring + light
    for x in 1..=6 {
        set(grid, cx + x, cy, make_block(BT_WIRE as u8, 0xF0, roof));
    }
    set(
        grid,
        cx + 3,
        cy - 2,
        make_block(BT_CEILING_LIGHT as u8, 0, roof),
    );
    // Wire branch to light
    set(grid, cx + 3, cy - 1, make_block(BT_WIRE as u8, 0xF0, roof));
    set(grid, cx + 3, cy, make_block(BT_WIRE as u8, 0xF0, roof));

    // === Pipe system: inlet (fireplace room) → pump → outlet (outside) ===
    set(grid, cx - 7, cy - 1, make_block(BT_INLET as u8, 1, 3 << 3)); // dir=west
    for x in -11..-7 {
        set(grid, cx + x, cy - 1, make_block(BT_PIPE as u8, 0xF0, 0));
    }
    set(grid, cx - 9, cy - 1, make_block(BT_PUMP as u8, 1, 3 << 3)); // dir=west
    set(
        grid,
        cx - 12,
        cy - 1,
        make_block(BT_OUTLET as u8, 1, 3 << 3),
    );

    // === Workshop area (south, outdoors) ===
    set(grid, cx - 3, cy + 8, make_block(BT_WORKBENCH as u8, 1, 0));
    set(grid, cx + 3, cy + 8, make_block(BT_KILN as u8, 2, 0));

    // === Well (on dug ground) ===
    set(grid, cx + 6, cy + 8, make_block(BT_WELL as u8, 1, 0));

    // === Outdoor lighting ===
    set(grid, cx - 3, cy + 6, make_block(BT_FLOOR_LAMP as u8, 1, 0));
    set(grid, cx + 3, cy + 6, make_block(BT_FLOOR_LAMP as u8, 1, 0));
    // Wall torches
    set(grid, cx - 2, cy - 6, make_block(BT_WALL_TORCH as u8, 0, 0)); // north
    set(grid, cx + 2, cy - 6, make_block(BT_WALL_TORCH as u8, 0, 0));

    // === Cannon (defense, east) ===
    set(
        grid,
        cx + 10,
        cy + 5,
        make_block(BT_CANNON as u8, 2, 1 << 3),
    ); // facing east
}

/// Generate the water table height map (256x256).
pub fn generate_water_table(grid: &[u32]) -> Vec<f32> {
    generate_water_table_seeded(grid, 0)
}

pub fn generate_water_table_seeded(grid: &[u32], seed: u32) -> Vec<f32> {
    let w = GRID_W;
    let h = GRID_H;
    let mut table = vec![-2.0f32; (w * h) as usize];

    let noise = |x: f32, y: f32| -> f32 {
        let ix = x.floor() as i32;
        let iy = y.floor() as i32;
        let fx = x - x.floor();
        let fy = y - y.floor();
        let hash = |ix: i32, iy: i32| -> f32 {
            let h = ((ix.wrapping_mul(374761393) as u32) ^ (iy.wrapping_mul(668265263) as u32))
                .wrapping_add(1013904223)
                .wrapping_add(seed.wrapping_mul(7919));
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

            // Boost near dug ground
            let block = grid[idx];
            let bt = block & 0xFF;
            let dug_boost = if bt == BT_DUG_GROUND { 1.0 } else { 0.0 };

            table[idx] = (depth + dug_boost).min(0.5);
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
        dawn_n,
        dusk_n,
        (0.0, -1.0),
        (0.0, 1.0),
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
                    if sxi < 0 || syi < 0 || sxi >= w || syi >= h {
                        break;
                    }
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
    let mut temp = vec![0.0f32; ao.len()]; // pre-allocate once, reuse across passes
    for _pass in 0..2 {
        // Horizontal
        temp.copy_from_slice(&ao);
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
pub fn generate_elevation(grid: &[u32]) -> Vec<f32> {
    generate_elevation_seeded(grid, 0)
}

pub fn generate_elevation_seeded(grid: &[u32], seed: u32) -> Vec<f32> {
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
                .wrapping_add(1013904223)
                .wrapping_add(seed.wrapping_mul(6271));
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
            let n1 = noise(fx * 0.025 + 300.0, fy * 0.025 + 700.0); // broad features
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

// Terrain type constants (stored in bits 0-3 of terrain_data)
pub const TERRAIN_GRASS: u32 = 0;
pub const TERRAIN_CHALKY: u32 = 1;
pub const TERRAIN_ROCKY: u32 = 2;
pub const TERRAIN_CLAY: u32 = 3;
pub const TERRAIN_GRAVEL: u32 = 4;
pub const TERRAIN_PEAT: u32 = 5;
pub const TERRAIN_MARSH: u32 = 6;
pub const TERRAIN_LOAM: u32 = 7;

/// Terrain data packing:
///   bits 0-3:   terrain type (0-15)
///   bits 4-8:   vegetation density (0-31)
///   bits 9-12:  grain/texture scale (0-15)
///   bits 13-14: surface roughness (0-3)
///   bits 15-19: soil richness (0-31)
///   bits 20-23: moisture retention (0-15)
///   bits 24-28: compaction (0-31, foot traffic wear)
pub fn pack_terrain(
    terrain_type: u32,
    vegetation: u32,
    grain: u32,
    roughness: u32,
    richness: u32,
    moisture_ret: u32,
) -> u32 {
    (terrain_type & 0xF)
        | ((vegetation & 0x1F) << 4)
        | ((grain & 0xF) << 9)
        | ((roughness & 0x3) << 13)
        | ((richness & 0x1F) << 15)
        | ((moisture_ret & 0xF) << 20)
    // bits 24-28: compaction, starts at 0
}

pub fn terrain_type(t: u32) -> u32 {
    t & 0xF
}
pub fn terrain_richness(t: u32) -> u32 {
    (t >> 15) & 0x1F
}
pub fn terrain_compaction(t: u32) -> u32 {
    (t >> 24) & 0x1F
}
pub fn terrain_roughness(t: u32) -> u32 {
    (t >> 13) & 0x3
}

/// Increment compaction on a terrain tile (clamped to 31).
pub fn terrain_add_compaction(t: &mut u32, amount: u32) {
    let cur = (*t >> 24) & 0x1F;
    let new = (cur + amount).min(31);
    *t = (*t & 0xE0FFFFFF) | (new << 24);
}

/// Decay compaction by 1 (natural decompaction over time).
pub fn terrain_decay_compaction(t: &mut u32) {
    let cur = (*t >> 24) & 0x1F;
    if cur > 0 {
        *t = (*t & 0xE0FFFFFF) | ((cur - 1) << 24);
    }
}

/// Parameters controlling terrain generation. Each weight (0.0-1.0) controls
/// how much of that terrain type appears. Higher = more area coverage.
#[derive(Clone, Debug)]
pub struct TerrainParams {
    pub grass: f32,
    pub loam: f32,
    pub clay: f32,
    pub chalky: f32,
    pub rocky: f32,
    pub gravel: f32,
    pub peat: f32,
    pub marsh: f32,
    pub pond_density: f32, // 0.0 = no ponds, 1.0 = many ponds
    pub seed: u32,
}

impl Default for TerrainParams {
    fn default() -> Self {
        Self {
            grass: 0.40,
            loam: 0.25,
            clay: 0.10,
            chalky: 0.02,
            rocky: 0.05,
            gravel: 0.05,
            peat: 0.03,
            marsh: 0.05,
            pond_density: 0.5,
            seed: 42,
        }
    }
}

/// Generate terrain data buffer from elevation, water table, and params.
pub fn generate_terrain(elevation: &[f32], water_table: &[f32]) -> Vec<u32> {
    generate_terrain_with_params(elevation, water_table, &TerrainParams::default())
}

/// Generate terrain data buffer with explicit parameters.
pub fn generate_terrain_with_params(
    elevation: &[f32],
    water_table: &[f32],
    params: &TerrainParams,
) -> Vec<u32> {
    let w = GRID_W;
    let h = GRID_H;
    let grid_size = (w * h) as usize;
    let mut terrain = vec![0u32; grid_size];
    let s = params.seed;

    let noise_seeded = |x: f32, y: f32, seed_extra: u32| -> f32 {
        let ix = x.floor() as i32;
        let iy = y.floor() as i32;
        let fx = x - x.floor();
        let fy = y - y.floor();
        let hash = |ix: i32, iy: i32| -> f32 {
            let h = ((ix.wrapping_mul(374761393) as u32) ^ (iy.wrapping_mul(668265263) as u32))
                .wrapping_add(1013904223)
                .wrapping_add(s)
                .wrapping_add(seed_extra);
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
    let noise = |x: f32, y: f32| -> f32 { noise_seeded(x, y, 0) };

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            let fx = x as f32;
            let fy = y as f32;

            let elev = if idx < elevation.len() {
                elevation[idx]
            } else {
                0.0
            };
            let wt = if idx < water_table.len() {
                water_table[idx]
            } else {
                -2.0
            };

            // --- Biome noise layers ---
            // Moisture gradient (noise-driven)
            let moisture_noise = noise(fx * 0.02 + 1000.0, fy * 0.02 + 1000.0);
            let moisture = moisture_noise + wt * 0.3; // wetter near high water table

            // Aridity (separate noise field)
            let arid_noise = noise(fx * 0.025 + 2000.0, fy * 0.025 + 2000.0);
            let aridity = arid_noise - wt * 0.2;

            // Rocky factor: increases with elevation
            let rock_noise = noise(fx * 0.04 + 3000.0, fy * 0.04 + 3000.0);
            let rockiness = (elev * 0.25 + rock_noise * 0.5).min(1.0);

            // Pond detection: concentric rings from noise peaks
            let pond_noise = noise_seeded(fx * 0.015 + 6000.0, fy * 0.015 + 6000.0, 100);
            let pond_detail = noise_seeded(fx * 0.05 + 7000.0, fy * 0.05 + 7000.0, 200);
            let pond_factor = (pond_noise * 0.7 + pond_detail * 0.3 + wt * 0.3).max(0.0);
            let pond_thresh = 1.0 - params.pond_density * 0.3; // higher density = lower threshold

            // --- Terrain type assignment (weighted scoring) ---
            // Each type gets a score: natural affinity * param weight + noise variation
            let terrain_type = if pond_factor > pond_thresh + 0.1 {
                TERRAIN_MARSH // pond center
            } else if pond_factor > pond_thresh {
                TERRAIN_MARSH // marsh ring
            } else if pond_factor > pond_thresh - 0.15 && params.clay > 0.01 {
                TERRAIN_CLAY // clay ring around ponds
            } else {
                // Score = weight * (noise_region + environmental_bias)
                // Each type uses its own noise field for coherent regions.
                // Environmental factors provide subtle bias (max ±0.15), noise dominates (0.0-1.0).
                let scores: [(u32, f32); 8] = [
                    (
                        TERRAIN_GRASS,
                        params.grass
                            * (noise_seeded(fx * 0.06, fy * 0.06, 10) + (1.0 - rockiness) * 0.1),
                    ),
                    (
                        TERRAIN_LOAM,
                        params.loam
                            * (noise_seeded(fx * 0.06, fy * 0.06, 20) + (1.0 - aridity) * 0.15),
                    ),
                    (
                        TERRAIN_CLAY,
                        params.clay
                            * (noise_seeded(fx * 0.06, fy * 0.06, 30) + (1.0 - aridity) * 0.1),
                    ),
                    (
                        TERRAIN_CHALKY,
                        params.chalky
                            * (noise_seeded(fx * 0.06, fy * 0.06, 40)
                                + aridity * 0.1
                                + rockiness * 0.05),
                    ),
                    (
                        TERRAIN_ROCKY,
                        params.rocky * (noise_seeded(fx * 0.06, fy * 0.06, 50) + rockiness * 0.15),
                    ),
                    (
                        TERRAIN_GRAVEL,
                        params.gravel * (noise_seeded(fx * 0.06, fy * 0.06, 60) + rockiness * 0.1),
                    ),
                    (
                        TERRAIN_PEAT,
                        params.peat * (noise_seeded(fx * 0.06, fy * 0.06, 70) + moisture * 0.15),
                    ),
                    (
                        TERRAIN_MARSH,
                        params.marsh
                            * (noise_seeded(fx * 0.06, fy * 0.06, 80) + (wt + 2.0).max(0.0) * 0.1),
                    ),
                ];
                let mut best_type = TERRAIN_GRASS;
                let mut best_score = -1.0f32;
                for &(tt, score) in &scores {
                    if score > best_score {
                        best_score = score;
                        best_type = tt;
                    }
                }
                best_type
            };

            // --- Vegetation density ---
            let veg_base = match terrain_type {
                TERRAIN_GRASS => 0.6 + (1.0 - aridity) * 0.4,
                TERRAIN_LOAM => 0.7 + (1.0 - aridity) * 0.3,
                TERRAIN_MARSH => 0.4 + moisture_noise * 0.3,
                TERRAIN_CLAY => 0.3 + (1.0 - aridity) * 0.3,
                TERRAIN_CHALKY => 0.15 + moisture_noise * 0.1,
                TERRAIN_ROCKY => 0.02,
                TERRAIN_GRAVEL => 0.1 + (1.0 - aridity) * 0.15,
                TERRAIN_PEAT => 0.25 + moisture_noise * 0.2,
                _ => 0.3,
            };
            let veg_noise = noise(fx * 0.15 + 4000.0, fy * 0.15 + 4000.0);
            let vegetation = ((veg_base + (veg_noise - 0.5) * 0.3) * 31.0).clamp(0.0, 31.0) as u32;

            // --- Grain/texture scale ---
            let grain = match terrain_type {
                TERRAIN_CHALKY => 6,  // chalky crumble
                TERRAIN_ROCKY => 12,  // coarse
                TERRAIN_GRAVEL => 10, // coarse-medium
                TERRAIN_GRASS => 4,   // fine
                TERRAIN_LOAM => 3,    // fine
                TERRAIN_MARSH => 5,   // medium
                TERRAIN_CLAY => 4,    // fine-medium
                TERRAIN_PEAT => 3,    // fine, soft
                _ => 5,
            };

            // --- Surface roughness ---
            let roughness = match terrain_type {
                TERRAIN_ROCKY => 3,
                TERRAIN_GRAVEL => 2,
                TERRAIN_CHALKY => 1,
                TERRAIN_PEAT => 0,
                _ => 1,
            };

            // --- Soil richness (farming potential) ---
            let richness_base = match terrain_type {
                TERRAIN_LOAM => 0.85,
                TERRAIN_GRASS => 0.6,
                TERRAIN_CLAY => 0.5,
                TERRAIN_MARSH => 0.7,
                TERRAIN_PEAT => 0.55,
                TERRAIN_CHALKY => 0.15,
                TERRAIN_GRAVEL => 0.15,
                TERRAIN_ROCKY => 0.05,
                _ => 0.4,
            };
            let rich_noise = noise(fx * 0.08 + 5000.0, fy * 0.08 + 5000.0);
            let richness =
                ((richness_base + (rich_noise - 0.5) * 0.3) * 31.0).clamp(0.0, 31.0) as u32;

            // --- Moisture retention ---
            let moist_ret = match terrain_type {
                TERRAIN_CLAY => 12,
                TERRAIN_LOAM => 10,
                TERRAIN_MARSH => 14,
                TERRAIN_GRASS => 8,
                TERRAIN_PEAT => 13,
                TERRAIN_CHALKY => 3,
                TERRAIN_GRAVEL => 3,
                TERRAIN_ROCKY => 1,
                _ => 6,
            };

            terrain[idx] = pack_terrain(
                terrain_type,
                vegetation,
                grain,
                roughness,
                richness,
                moist_ret,
            );
        }
    }

    terrain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_block_roundtrip() {
        // Pack and unpack should be lossless (using raw height for full byte)
        for bt in [0u8, 1, 5, 8, 13, 29, 255] {
            for h in [0u8, 1, 3, 5, 128, 255] {
                for f in [0u8, 1, 2, 4, 7, 63] {
                    let block = make_block(bt, h, f);
                    assert_eq!(
                        block_type_rs(block),
                        bt as u32,
                        "type mismatch for ({bt},{h},{f})"
                    );
                    assert_eq!(
                        block_height_raw(block),
                        h,
                        "height mismatch for ({bt},{h},{f})"
                    );
                    assert_eq!(
                        block_flags_rs(block),
                        f,
                        "flags mismatch for ({bt},{h},{f})"
                    );
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
        assert_eq!(
            roof_height_rs(interior),
            3,
            "interior should have roof height 3"
        );

        // Wall tiles should also have roof_height = 3
        let wall = grid[(10 * w + 11) as usize];
        assert_eq!(roof_height_rs(wall), 3, "wall should have roof height 3");

        // Outdoor tile should have roof_height = 0
        let outdoor = grid[(5 * w + 5) as usize];
        assert_eq!(
            roof_height_rs(outdoor),
            0,
            "outdoor should have roof height 0"
        );
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
        let empty_ground = existing_h == 0
            && (existing_bt == BT_AIR
                || existing_bt == BT_DIRT
                || existing_bt == BT_WOOD_FLOOR
                || existing_bt == BT_STONE_FLOOR
                || existing_bt == BT_CONCRETE_FLOOR);
        let pid = place_id as u32;
        empty_ground
            || (pid == BT_WIRE && existing_bt != BT_WIRE)
            || (pid == BT_PIPE && (existing_bt == BT_PIPE || existing_bt == BT_PIPE_BRIDGE))
            || (pid == BT_RESTRICTOR
                && (existing_bt == BT_PIPE
                    || existing_bt == BT_RESTRICTOR
                    || existing_bt == BT_PIPE_BRIDGE))
            || (pid == BT_LIQUID_PIPE
                && (existing_bt == BT_LIQUID_PIPE || existing_bt == BT_PIPE_BRIDGE))
            || (pid == BT_PUMP && existing_bt == BT_PIPE)
            || ((pid == BT_SWITCH || pid == BT_DIMMER || pid == BT_BREAKER)
                && (existing_bt == BT_WIRE || existing_bt == BT_AIR || existing_bt == BT_DIRT))
    }

    #[test]
    fn test_all_block_ids_valid() {
        // Every defined block type ID should be < NUM_MATERIALS
        let max_id = 54u32; // BT_LIQUID_OUTPUT
        for id in 0..=max_id {
            assert!(
                id < crate::materials::NUM_MATERIALS as u32,
                "Block ID {} exceeds NUM_MATERIALS ({})",
                id,
                crate::materials::NUM_MATERIALS
            );
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
        assert!(can_place_on_block(BT_WIRE as u8, BT_PIPE, 1)); // on pipes
        assert!(!can_place_on_block(BT_WIRE as u8, BT_WIRE, 0)); // NOT on existing wire
    }

    #[test]
    fn test_power_equipment_on_wire_or_ground() {
        for &id in &[BT_SWITCH, BT_DIMMER, BT_BREAKER] {
            assert!(
                can_place_on_block(id as u8, BT_WIRE, 0),
                "ID {} should place on wire",
                id
            );
            assert!(
                can_place_on_block(id as u8, BT_DIRT, 0),
                "ID {} should place on ground",
                id
            );
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
        let power_ids: &[u32] = &[
            36, 37, 38, 39, 40, 41, 42, 43, 45, 48, 51, 7, 10, 11, 12, 16,
        ];
        for &id in power_ids {
            assert!(
                is_conductor_rs(id, 0),
                "Block type {} should be a conductor",
                id
            );
        }
        // Wire overlay flag
        assert!(
            is_conductor_rs(1, 0x80),
            "Wall with wire overlay should be conductor"
        );
        // Non-conductors
        assert!(!is_conductor_rs(2, 0), "Dirt should not be conductor");
        assert!(!is_conductor_rs(15, 0), "Pipe should not be conductor");
    }

    #[test]
    fn test_bridge_connects_to_gas_pipes() {
        assert!(
            can_place_on_block(BT_PIPE as u8, BT_PIPE_BRIDGE, 1),
            "Gas pipe should be placeable on bridge"
        );
        assert!(
            can_place_on_block(BT_RESTRICTOR as u8, BT_PIPE_BRIDGE, 1),
            "Restrictor should be placeable on bridge"
        );
    }

    #[test]
    fn test_bridge_connects_to_liquid_pipes() {
        assert!(
            can_place_on_block(BT_LIQUID_PIPE as u8, BT_PIPE_BRIDGE, 1),
            "Liquid pipe should be placeable on bridge"
        );
    }

    /// Simulate intake tile assignment: given two block types, determine if placement is valid.
    /// Returns (ground_idx, water_idx) or None if invalid.
    fn intake_valid(bt0: u32, bh0: u8, bt1: u32, bh1: u8) -> Option<(usize, usize)> {
        let is_ground = |bt: u32, bh: u8| {
            bh == 0
                && (bt == BT_AIR
                    || bt == BT_DIRT
                    || bt == BT_WOOD_FLOOR
                    || bt == BT_STONE_FLOOR
                    || bt == BT_CONCRETE_FLOOR)
        };
        let is_water = |bt: u32| bt == BT_WATER || bt == BT_DUG_GROUND;
        if is_ground(bt0, bh0) && is_water(bt1) {
            Some((0, 1))
        } else if is_water(bt0) && is_ground(bt1, bh1) {
            Some((1, 0))
        } else {
            None
        }
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
        assert_eq!(
            r.unwrap(),
            (1, 0),
            "ground should be index 1, water index 0"
        );
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
        use crate::pipes::{is_gas_pipe_component, is_liquid_pipe_component};
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
        assert!(
            !is_gas_pipe_component(52),
            "Liquid intake NOT in gas network"
        );
    }

    #[test]
    fn test_liquid_pipes_walkable() {
        // All liquid pipe components should be recognized as walkable pipe blocks
        // (walkability is checked in pleb.rs using the same block type IDs)
        let liquid_types: &[u32] = &[
            BT_LIQUID_PIPE,
            BT_LIQUID_INTAKE,
            BT_LIQUID_PUMP,
            BT_LIQUID_OUTPUT,
        ];
        for &bt in liquid_types {
            // The pipe walkability check: bt matches AND height <= 1
            let is_any_pipe = (bt >= 15 && bt <= 20)
                || bt == BT_RESTRICTOR
                || bt == BT_LIQUID_PIPE
                || bt == BT_PIPE_BRIDGE
                || bt == BT_LIQUID_INTAKE
                || bt == BT_LIQUID_PUMP
                || bt == BT_LIQUID_OUTPUT;
            assert!(
                is_any_pipe,
                "Block type {} should be walkable as a pipe",
                bt
            );
        }
    }

    #[test]
    fn test_gas_pipe_types_walkable() {
        let gas_types: &[u32] = &[
            BT_PIPE,
            BT_PUMP,
            BT_TANK,
            BT_VALVE,
            BT_OUTLET,
            BT_INLET,
            BT_RESTRICTOR,
        ];
        for &bt in gas_types {
            let is_any_pipe = (bt >= 15 && bt <= 20)
                || bt == BT_RESTRICTOR
                || bt == BT_LIQUID_PIPE
                || bt == BT_PIPE_BRIDGE
                || bt == BT_LIQUID_INTAKE
                || bt == BT_LIQUID_PUMP
                || bt == BT_LIQUID_OUTPUT;
            assert!(
                is_any_pipe,
                "Block type {} should be walkable as a pipe",
                bt
            );
        }
    }

    #[test]
    fn test_num_materials_covers_all_blocks() {
        let highest = BT_LIQUID_OUTPUT; // 54
        assert!(
            crate::materials::NUM_MATERIALS > highest as usize,
            "NUM_MATERIALS ({}) must be > highest block ID ({})",
            crate::materials::NUM_MATERIALS,
            highest
        );
    }

    // --- Thin wall edge bitmask tests ---

    #[test]
    fn test_thin_wall_preserves_thickness() {
        // A 2-wide wall on north edge should stay 2-wide after creation
        //
        // Layout (4x4 sub-grid, thickness=2, N edge):
        // [2][2][2][2]   ← wall strip (2 sub-cells thick)
        // [2][2][2][2]
        // [ ][ ][ ][ ]   ← open space
        // [ ][ ][ ][ ]
        let (flags, edge_mask) = make_thin_wall_flags(0, 0, 2); // edge=N, thickness=2
        let block = make_block(BT_WALL as u8, make_wall_height(3, edge_mask), flags);

        assert_eq!(block_height_rs(block), 3, "visual height should be 3");
        assert_eq!(
            wall_edge_mask(block_height_raw(block)),
            WALL_EDGE_N as u8,
            "should have N edge only"
        );
        let thick_raw = (block_flags_rs(block) >> 5) & 3;
        let thickness = if thick_raw == 0 { 4 } else { 4 - thick_raw };
        assert_eq!(thickness, 2, "thickness should be 2");
    }

    #[test]
    fn test_thin_wall_corner_merge() {
        // Dragging a 2-wide N wall, then a 2-wide E wall onto the same tile
        // should create a corner (N+E) with thickness 2, NOT full-width.
        //
        // Layout (4x4 sub-grid, thickness=2, N+E corner):
        // [2][2][2][2]   ← N edge wall
        // [2][2][2][2]
        // [ ][ ][2][2]   ← E edge wall
        // [ ][ ][2][2]
        let (flags_n, mask_n) = make_thin_wall_flags(0, 0, 2); // N edge, thickness 2
        let (flags_e, mask_e) = make_thin_wall_flags(0, 1, 2); // E edge, thickness 2

        // Simulate merge: OR the masks
        let merged_mask = mask_n | mask_e;
        let height = make_wall_height(3, merged_mask);
        let block = make_block(BT_WALL as u8, height, flags_n); // use same thickness

        assert_eq!(block_height_rs(block), 3, "visual height should be 3");
        let raw = block_height_raw(block);
        assert!(has_wall_on_edge(raw, flags_n, 0), "should block N");
        assert!(has_wall_on_edge(raw, flags_n, 1), "should block E");
        assert!(!has_wall_on_edge(raw, flags_n, 2), "should NOT block S");
        assert!(!has_wall_on_edge(raw, flags_n, 3), "should NOT block W");

        let thick_raw = (flags_n >> 5) & 3;
        let thickness = if thick_raw == 0 { 4 } else { 4 - thick_raw };
        assert_eq!(thickness, 2, "thickness should remain 2 after merge");
    }

    #[test]
    fn test_thin_wall_t_junction() {
        // T-junction: N+E+S edges, all thickness 1
        //
        // Layout (4x4 sub-grid, thickness=1, N+E+S):
        // [1][1][1][1]   ← N edge
        // [ ][ ][ ][1]   ← E edge
        // [ ][ ][ ][1]
        // [1][1][1][1]   ← S edge
        let mask = WALL_EDGE_N | WALL_EDGE_E | WALL_EDGE_S;
        let thick_bits: u8 = 4 - 1; // thickness=1 → thick_bits=3
        let flags = (thick_bits & 3) << 5;
        let height = make_wall_height(3, mask);
        let block = make_block(BT_WALL as u8, height, flags);

        let raw = block_height_raw(block);
        assert!(has_wall_on_edge(raw, flags, 0), "T-junction should block N");
        assert!(has_wall_on_edge(raw, flags, 1), "T-junction should block E");
        assert!(has_wall_on_edge(raw, flags, 2), "T-junction should block S");
        assert!(
            !has_wall_on_edge(raw, flags, 3),
            "T-junction should NOT block W"
        );
        assert_eq!(block_height_rs(block), 3, "visual height should be 3");
        assert!(
            thin_wall_is_walkable(block),
            "T-junction with thickness 1 should be walkable"
        );
    }

    #[test]
    fn test_thin_wall_cross_junction() {
        // Cross: all 4 edges, thickness 1
        //
        // Layout (4x4 sub-grid, thickness=1, N+E+S+W):
        // [1][1][1][1]   ← N edge
        // [1][ ][ ][1]   ← W and E edges
        // [1][ ][ ][1]
        // [1][1][1][1]   ← S edge
        let mask = WALL_EDGE_N | WALL_EDGE_E | WALL_EDGE_S | WALL_EDGE_W;
        let thick_bits: u8 = 4 - 1;
        let flags = (thick_bits & 3) << 5;
        let height = make_wall_height(3, mask);
        let block = make_block(BT_WALL as u8, height, flags);

        let raw = block_height_raw(block);
        assert!(has_wall_on_edge(raw, flags, 0), "cross should block N");
        assert!(has_wall_on_edge(raw, flags, 1), "cross should block E");
        assert!(has_wall_on_edge(raw, flags, 2), "cross should block S");
        assert!(has_wall_on_edge(raw, flags, 3), "cross should block W");
        assert!(
            thin_wall_is_walkable(block),
            "cross with thickness 1 still has center open"
        );
    }

    #[test]
    fn test_edge_blocked_thin_wall_direction() {
        // 2-wide wall on N edge at (5,5). Moving north should be blocked,
        // moving east/south/west should NOT be blocked.
        //
        // Layout at (5,5):
        // [2][2][2][2]   ← N edge blocks northward movement
        // [2][2][2][2]
        // [ ][ ][ ][ ]   ← open: east/south/west pass through
        // [ ][ ][ ][ ]
        let mut grid = vec![make_block(BT_DIRT as u8, 0, 0); (GRID_W * GRID_H) as usize];
        let (flags, mask) = make_thin_wall_flags(0, 0, 2); // N edge, thickness 2
        grid[(5 * GRID_W + 5) as usize] =
            make_block(BT_WALL as u8, make_wall_height(3, mask), flags);

        assert!(edge_blocked(&grid, 5, 5, 5, 4), "N: should be blocked");
        assert!(!edge_blocked(&grid, 5, 5, 6, 5), "E: should NOT be blocked");
        assert!(!edge_blocked(&grid, 5, 5, 5, 6), "S: should NOT be blocked");
        assert!(!edge_blocked(&grid, 5, 5, 4, 5), "W: should NOT be blocked");
    }

    #[test]
    fn test_edge_blocked_t_junction() {
        // 1-wide T-junction (N+E+S) at (5,5).
        // North, east, south blocked. West open.
        //
        // Layout at (5,5):
        // [1][1][1][1]   ← N
        // [ ][ ][ ][1]   ← E
        // [ ][ ][ ][1]
        // [1][1][1][1]   ← S
        let mut grid = vec![make_block(BT_DIRT as u8, 0, 0); (GRID_W * GRID_H) as usize];
        let mask = WALL_EDGE_N | WALL_EDGE_E | WALL_EDGE_S;
        let thick_bits: u8 = 4 - 1;
        let flags = (thick_bits & 3) << 5;
        grid[(5 * GRID_W + 5) as usize] =
            make_block(BT_WALL as u8, make_wall_height(3, mask), flags);

        assert!(edge_blocked(&grid, 5, 5, 5, 4), "N: blocked by T-junction");
        assert!(edge_blocked(&grid, 5, 5, 6, 5), "E: blocked by T-junction");
        assert!(edge_blocked(&grid, 5, 5, 5, 6), "S: blocked by T-junction");
        assert!(!edge_blocked(&grid, 5, 5, 4, 5), "W: open in T-junction");
    }
}
