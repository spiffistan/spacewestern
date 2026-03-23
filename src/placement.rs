//! Build placement, tile helpers, drag shapes, context menus, and block interactions.

use crate::*;

impl App {
    /// Sync crate block height with item count for shader rendering.
    pub(crate) fn sync_crate_visual(&mut self, cidx: u32) {
        if let Some(inv) = self.crate_contents.get(&cidx) {
            let count = inv.total().min(CRATE_MAX_ITEMS) as u8;
            let idx = cidx as usize;
            if idx < self.grid_data.len() {
                let block = self.grid_data[idx];
                if (block & 0xFF) == BT_CRATE {
                    // Store item count in height byte (bits 8-15)
                    self.grid_data[idx] = (block & 0xFFFF00FF) | ((count as u32) << 8);
                    self.grid_dirty = true;
                }
            }
        }
    }

    /// Get the tiles a bench would occupy at (bx, by) with given rotation
    pub(crate) fn bed_tiles(&self, bx: i32, by: i32, rotation: u32) -> [(i32, i32); 2] {
        if rotation == 0 {
            [(bx, by), (bx + 1, by)]
        } else {
            [(bx, by), (bx, by + 1)]
        }
    }

    /// Get the 9 tiles a solar panel occupies at (bx, by) — 3×3 grid
    pub(crate) fn solar_tiles(&self, bx: i32, by: i32) -> [(i32, i32); 9] {
        [
            (bx, by),     (bx + 1, by),     (bx + 2, by),
            (bx, by + 1), (bx + 1, by + 1), (bx + 2, by + 1),
            (bx, by + 2), (bx + 1, by + 2), (bx + 2, by + 2),
        ]
    }

    pub(crate) fn bench_tiles(&self, bx: i32, by: i32, rotation: u32) -> [(i32, i32); 3] {
        if rotation == 0 {
            // Horizontal: extends east
            [(bx, by), (bx + 1, by), (bx + 2, by)]
        } else {
            // Vertical: extends south
            [(bx, by), (bx, by + 1), (bx, by + 2)]
        }
    }

    /// Bridge tiles: 3-tile line in the rotation direction.
    pub(crate) fn bridge_tiles(&self, bx: i32, by: i32, rotation: u32) -> [(i32, i32); 3] {
        match rotation {
            0 => [(bx, by), (bx, by + 1), (bx, by + 2)],     // N: goes south
            1 => [(bx, by), (bx + 1, by), (bx + 2, by)],     // E: goes east
            2 => [(bx, by), (bx, by - 1), (bx, by - 2)],     // S: goes north
            _ => [(bx, by), (bx - 1, by), (bx - 2, by)],     // W: goes west
        }
    }

    /// For liquid intake: determine which of the 2 tiles is ground (seg 0) and which is water (seg 1).
    /// Returns (Some(ground_index), Some(water_index)) or (None, None) if invalid.
    pub(crate) fn intake_tile_assignment(&self, tiles: &[(i32, i32); 2]) -> (Option<usize>, Option<usize>) {
        let in_bounds = |x: i32, y: i32| x >= 0 && y >= 0 && x < GRID_W as i32 && y < GRID_H as i32;
        if !in_bounds(tiles[0].0, tiles[0].1) || !in_bounds(tiles[1].0, tiles[1].1) {
            return (None, None);
        }
        let bt = |i: usize| -> u32 {
            (self.grid_data[(tiles[i].1 as u32 * GRID_W + tiles[i].0 as u32) as usize] & 0xFF) as u32
        };
        let is_water = |b: u32| b == BT_WATER || b == BT_DUG_GROUND;
        let is_ground = |i: usize| self.can_place_at(tiles[i].0, tiles[i].1);
        // Try both orientations: tile 0=ground + tile 1=water, or tile 0=water + tile 1=ground
        if is_ground(0) && is_water(bt(1)) {
            (Some(0), Some(1))
        } else if is_water(bt(0)) && is_ground(1) {
            (Some(1), Some(0))
        } else {
            (None, None)
        }
    }

    /// Find wall neighbors of a floor tile. Returns directions (0=N,1=E,2=S,3=W) where walls exist.
    pub(crate) fn find_wall_neighbors(&self, x: i32, y: i32) -> Vec<u32> {
        let dirs: [(i32, i32, u32); 4] = [(0, -1, 0), (1, 0, 1), (0, 1, 2), (-1, 0, 3)];
        let reg = block_defs::BlockRegistry::cached();
        let mut result = Vec::new();
        for &(dx, dy, dir) in &dirs {
            let nx = x + dx;
            let ny = y + dy;
            if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 { continue; }
            let nidx = (ny as u32 * GRID_W + nx as u32) as usize;
            let nb = self.grid_data[nidx];
            let nbt = block_type_rs(nb);
            let nbh = (nb >> 8) & 0xFF;
            if nbh > 0 && reg.is_wall(nbt) {
                result.push(dir);
            }
        }
        result
    }

    /// Place a multi-tile block. Validates all tiles, then places with per-tile flags.
    /// `flags_fn(tile_index, roof_flag) -> combined_flags` computes the flags byte for each tile.
    /// Returns true if placement succeeded.
    pub(crate) fn place_multi_tiles(
        &mut self, tiles: &[(i32, i32)], block_id: u8, height: u8,
        flags_fn: impl Fn(usize, u8) -> u8,
    ) -> bool {
        let all_valid = tiles.iter().all(|&(tx, ty)| self.can_place_at(tx, ty));
        if !all_valid { return false; }
        for (i, &(tx, ty)) in tiles.iter().enumerate() {
            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
            let (roof_flag, roof_h) = extract_roof_data(self.grid_data[tidx]);
            self.grid_data[tidx] = make_block(block_id, height, flags_fn(i, roof_flag)) | roof_h;
        }
        self.grid_dirty = true;
        self.build_tool = BuildTool::None;
        true
    }

    /// Check if a tile is valid for wall-adjacent placement and return the facing direction.
    /// If multiple walls, uses build_rotation to disambiguate.
    /// Returns None if no adjacent wall or tile isn't floor-level.
    pub(crate) fn wall_adjacent_direction(&self, x: i32, y: i32) -> Option<u32> {
        if !self.can_place_at(x, y) { return None; }
        let walls = self.find_wall_neighbors(x, y);
        match walls.len() {
            0 => None,
            1 => Some(walls[0]),
            _ => {
                // Multiple walls: use build_rotation to pick
                if walls.contains(&self.build_rotation) {
                    Some(self.build_rotation)
                } else {
                    Some(walls[0])
                }
            }
        }
    }

    /// Check if a tile is valid for placement (ground level, in bounds)
    pub(crate) fn can_place_at(&self, x: i32, y: i32) -> bool {
        self.can_place_on(x, y, false)
    }

    /// Check if a tile is valid for placement. If `on_furniture` is true,
    /// allows placement on benches (for table lamps).
    pub(crate) fn can_place_on(&self, x: i32, y: i32, on_furniture: bool) -> bool {
        if x < 0 || y < 0 || x >= GRID_W as i32 || y >= GRID_H as i32 {
            return false;
        }
        let idx = (y as u32 * GRID_W + x as u32) as usize;
        let block = self.grid_data[idx];
        let bt = block_type_rs(block);
        let bh = (block >> 8) & 0xFF;
        if on_furniture {
            return bt == BT_BENCH;
        }
        if !((bt == 0 || bt == 2) && bh == 0) {
            return false;
        }
        // Check terrain slope: too steep = can't build
        if !self.elevation_data.is_empty() && idx < self.elevation_data.len() {
            let elev = self.elevation_data[idx];
            // Check neighbor elevation differences
            for &(dx, dy) in &[(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let nx = x + dx;
                let ny = y + dy;
                if nx >= 0 && ny >= 0 && nx < GRID_W as i32 && ny < GRID_H as i32 {
                    let nidx = (ny as u32 * GRID_W + nx as u32) as usize;
                    if nidx < self.elevation_data.len() {
                        let diff = (self.elevation_data[nidx] - elev).abs();
                        if diff > 1.0 { return false; } // too steep
                    }
                }
            }
        }
        true
    }

    /// Convert screen pixel coordinates to world block coordinates
    pub(crate) fn screen_to_world(&self, sx: f64, sy: f64) -> (f32, f32) {
        // Scale mouse coords from window space to render space
        let rx = sx as f32 * self.render_scale;
        let ry = sy as f32 * self.render_scale;
        let wx = self.camera.center_x + (rx - self.camera.screen_w * 0.5) / self.camera.zoom;
        let wy = self.camera.center_y + (ry - self.camera.screen_h * 0.5) / self.camera.zoom;
        (wx, wy)
    }

    /// Try to pick up a light source at the given world coordinates (right-click)
    pub(crate) fn try_pick_light(&mut self, wx: f32, wy: f32) -> bool {
        let bx = wx.floor() as i32;
        let by = wy.floor() as i32;
        if bx < 0 || by < 0 || bx >= GRID_W as i32 || by >= GRID_H as i32 {
            return false;
        }
        let idx = (by as u32 * GRID_W + bx as u32) as usize;
        let block = self.grid_data[idx];
        let bt = block_type_rs(block);
        if bt == BT_FIREPLACE || bt == BT_CEILING_LIGHT {
            self.dragging_light = Some((bx as u32, by as u32));
            log::info!("Picked up light at ({}, {})", bx, by);
            return true;
        }
        false
    }

    /// Move a dragged light source to a new position
    pub(crate) fn move_light_to(&mut self, wx: f32, wy: f32) {
        let new_bx = wx.floor() as i32;
        let new_by = wy.floor() as i32;
        if new_bx < 0 || new_by < 0 || new_bx >= GRID_W as i32 || new_by >= GRID_H as i32 {
            return;
        }
        if let Some((old_x, old_y)) = self.dragging_light {
            let old_idx = (old_y * GRID_W + old_x) as usize;
            let new_idx = (new_by as u32 * GRID_W + new_bx as u32) as usize;

            // Only move if destination is a floor tile (type 2, height 0)
            let dest = self.grid_data[new_idx];
            let dest_bt = block_type_rs(dest);
            let dest_h = (dest >> 8) & 0xFF;
            if (dest_bt == 2 || dest_bt == 0) && dest_h == 0 && new_idx != old_idx {
                let light_block = self.grid_data[old_idx];
                let light_flags = block_flags_rs(light_block);
                let dest_flags = (dest >> 16) & 0xFF;

                // Replace old position with floor (preserve roof flag)
                self.grid_data[old_idx] = make_block(2, 0, (light_flags & 2) as u8);

                // Place light at new position (preserve destination roof flag)
                let new_block = (light_block & 0x0000FFFF) | ((dest_flags as u32) << 16);
                // Also preserve the precomputed roof height from destination
                let dest_roof_h = (dest >> 24) & 0xFF;
                self.grid_data[new_idx] = (new_block & 0x00FFFFFF) | (dest_roof_h << 24);

                self.dragging_light = Some((new_bx as u32, new_by as u32));
                self.grid_dirty = true;
            }
        }
    }

    /// Drop a dragged light source
    pub(crate) fn drop_light(&mut self) {
        if let Some((x, y)) = self.dragging_light.take() {
            log::info!("Placed light at ({}, {})", x, y);
            // Recompute roof heights since light moved
            compute_roof_heights(&mut self.grid_data);
            self.grid_dirty = true;
        }
    }

    /// Compute tiles for a hollow rectangle (walls) between two corners.
    pub(crate) fn hollow_rect_tiles(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
        let min_x = x0.min(x1);
        let max_x = x0.max(x1);
        let min_y = y0.min(y1);
        let max_y = y0.max(y1);
        let mut tiles = Vec::new();
        for x in min_x..=max_x {
            tiles.push((x, min_y));
            if max_y != min_y { tiles.push((x, max_y)); }
        }
        for y in (min_y + 1)..max_y {
            tiles.push((min_x, y));
            if max_x != min_x { tiles.push((max_x, y)); }
        }
        tiles
    }

    pub(crate) fn diagonal_wall_tiles(x0: i32, y0: i32, x1: i32, y1: i32, rotation: u32) -> Vec<(i32, i32, u8)> {
        compute_diagonal_wall_tiles(x0, y0, x1, y1, rotation)
    }

    /// Compute tiles for a line (pipes) snapped to dominant axis.
    pub(crate) fn line_tiles(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let mut tiles = Vec::new();
        if dx >= dy {
            // Horizontal line (ordered in drag direction)
            let step = if x1 >= x0 { 1 } else { -1 };
            let mut x = x0;
            loop {
                tiles.push((x, y0));
                if x == x1 { break; }
                x += step;
            }
        } else {
            // Vertical line (ordered in drag direction)
            let step = if y1 >= y0 { 1 } else { -1 };
            let mut y = y0;
            loop {
                tiles.push((x0, y));
                if y == y1 { break; }
                y += step;
            }
        }
        tiles
    }

    /// Compute tiles for a filled rectangle (destroy).
    pub(crate) fn filled_rect_tiles(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
        let min_x = x0.min(x1);
        let max_x = x0.max(x1);
        let min_y = y0.min(y1);
        let max_y = y0.max(y1);
        let mut tiles = Vec::new();
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                tiles.push((x, y));
            }
        }
        tiles
    }

    /// Check if a tile can support a roof (wall within 6 Manhattan distance).
    pub(crate) fn can_support_roof(grid: &[u32], x: i32, y: i32) -> bool {
        let max_dist = 6i32;
        for dy in -max_dist..=max_dist {
            for dx in -max_dist..=max_dist {
                if dx.abs() + dy.abs() > max_dist { continue; }
                let nx = x + dx;
                let ny = y + dy;
                if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 { continue; }
                let b = grid[(ny as u32 * GRID_W + nx as u32) as usize];
                let bt = b & 0xFF;
                let bh = (b >> 8) & 0xFF;
                if bh > 0 && bt_is!(bt, BT_STONE, BT_WALL, BT_GLASS, BT_INSULATED, BT_WOOD_WALL, BT_STEEL_WALL, BT_SANDSTONE, BT_GRANITE, BT_LIMESTONE) {
                    return true;
                }
            }
        }
        false
    }

    /// Apply the drag shape when mouse is released.
    pub(crate) fn apply_drag_shape(&mut self, sx: i32, sy: i32, ex: i32, ey: i32) {
        // Growing zone: paint zone on dirt tiles
        if self.build_tool == BuildTool::GrowingZone {
            let tiles = Self::filled_rect_tiles(sx, sy, ex, ey);
            for (tx, ty) in tiles {
                if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 { continue; }
                let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                let bt = self.grid_data[idx] & 0xFF;
                if bt == BT_DIRT {
                    if let Some(zone) = self.zones.iter_mut().find(|z| z.kind == ZoneKind::Growing) {
                        zone.tiles.insert((tx, ty));
                    } else {
                        let mut zone = Zone::new(ZoneKind::Growing);
                        zone.tiles.insert((tx, ty));
                        self.zones.push(zone);
                    }
                }
            }
            return;
        }

        if self.build_tool == BuildTool::StorageZone {
            let tiles = Self::filled_rect_tiles(sx, sy, ex, ey);
            for (tx, ty) in tiles {
                if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 { continue; }
                let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                let bh = (self.grid_data[idx] >> 8) & 0xFF;
                if bh == 0 { // only floor-level tiles
                    if let Some(zone) = self.zones.iter_mut().find(|z| z.kind == ZoneKind::Storage) {
                        zone.tiles.insert((tx, ty));
                    } else {
                        let mut zone = Zone::new(ZoneKind::Storage);
                        zone.tiles.insert((tx, ty));
                        self.zones.push(zone);
                    }
                }
            }
            return;
        }

        // Roof tool: special handling — sets flag, doesn't change block type
        if self.build_tool == BuildTool::Roof {
            let tiles = Self::filled_rect_tiles(sx, sy, ex, ey);
            for (tx, ty) in tiles {
                if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 { continue; }
                if Self::can_support_roof(&self.grid_data, tx, ty) {
                    let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                    let block = self.grid_data[idx];
                    let bh = (block >> 8) & 0xFF;
                    if bh == 0 { // only floor-level tiles
                        self.grid_data[idx] |= 2 << 16; // set roof flag (bit 1)
                        self.grid_dirty = true;
                    }
                }
            }
            compute_roof_heights(&mut self.grid_data);
            return;
        }

        if self.build_tool == BuildTool::RemoveFloor {
            let tiles = Self::filled_rect_tiles(sx, sy, ex, ey);
            for (tx, ty) in tiles {
                if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 { continue; }
                let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                let block = self.grid_data[idx];
                let bt = block & 0xFF;
                // Replace floor types (26/27/28) with dirt (2)
                if bt_is!(bt, BT_WOOD_FLOOR, BT_STONE_FLOOR, BT_CONCRETE_FLOOR) {
                    let roof_flag = (block >> 16) & 2;
                    let roof_h = block & 0xFF000000;
                    self.grid_data[idx] = make_block(2, 0, roof_flag as u8) | roof_h;
                    self.grid_dirty = true;
                }
            }
            return;
        }

        if self.build_tool == BuildTool::RemoveRoof {
            let tiles = Self::filled_rect_tiles(sx, sy, ex, ey);
            for (tx, ty) in tiles {
                if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 { continue; }
                let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                let block = self.grid_data[idx];
                let has_roof = (block >> 16) & 2 != 0;
                if has_roof {
                    self.grid_data[idx] &= !(2u32 << 16); // clear roof flag
                    self.grid_dirty = true;
                }
            }
            compute_roof_heights(&mut self.grid_data);
            return;
        }

        // Special case: diagonal wall drag places per-tile variants
        if self.build_tool == BuildTool::Place(44) {
            let diag_tiles = Self::diagonal_wall_tiles(sx, sy, ex, ey, self.build_rotation);
            for (tx, ty, variant) in diag_tiles {
                if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 { continue; }
                let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                let block = self.grid_data[idx];
                let bt = block_type_rs(block);
                let bh = (block >> 8) & 0xFF;
                if (bt == 0 || bt == 2) && bh == 0 {
                    let roof_flag = block_flags_rs(block) & 2;
                    let roof_h = block & 0xFF000000;
                    let flags = roof_flag | (variant << 3);
                    self.grid_data[idx] = make_block(44, 3, flags) | roof_h;
                    self.grid_dirty = true;
                }
            }
            if self.grid_dirty { compute_roof_heights(&mut self.grid_data); }
            return;
        }

        let reg = block_defs::BlockRegistry::cached();
        let (block_type_id, tiles) = match self.build_tool {
            BuildTool::Destroy => (0u32, Self::filled_rect_tiles(sx, sy, ex, ey)),
            BuildTool::Place(id) => {
                let shape = reg.get(id).and_then(|d| d.placement.as_ref()).and_then(|p| p.drag.as_ref());
                let t = match shape {
                    Some(block_defs::DragShape::Line) => Self::line_tiles(sx, sy, ex, ey),
                    Some(block_defs::DragShape::FilledRect) => Self::filled_rect_tiles(sx, sy, ex, ey),
                    Some(block_defs::DragShape::HollowRect) => Self::hollow_rect_tiles(sx, sy, ex, ey),
                    _ => return,
                };
                (id, t)
            }
            _ => return,
        };

        // Connection mask bits: bit4=N, bit5=E, bit6=S, bit7=W (stored in height byte)
        const CONN_N: u8 = 0x10;
        const CONN_E: u8 = 0x20;
        const CONN_S: u8 = 0x40;
        const CONN_W: u8 = 0x80;
        let bid = block_type_id;
        let is_line_type = bt_is!(bid, BT_PIPE, BT_WIRE, BT_RESTRICTOR, BT_LIQUID_PIPE);

        for (ti, &(tx, ty)) in tiles.iter().enumerate() {
            if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 { continue; }
            if self.build_tool == BuildTool::Destroy {
                self.destroy_block_at(tx, ty);
            } else {
                let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                let block = self.grid_data[idx];
                let bt = block_type_rs(block);
                let bh = (block >> 8) & 0xFF;
                let wire_anywhere = bid == BT_WIRE;
                let btu = bt as u32;
                let gas_pipe_compat = bt_is!(bid, BT_PIPE, BT_RESTRICTOR) && bt_is!(btu, BT_PIPE, BT_RESTRICTOR, BT_PIPE_BRIDGE);
                let liquid_pipe_compat = bid == BT_LIQUID_PIPE && bt_is!(btu, BT_LIQUID_PIPE, BT_PIPE_BRIDGE);
                let pipe_compat = gas_pipe_compat || liquid_pipe_compat;
                let same_type = btu == bid || pipe_compat; // allow pipe↔restrictor
                if ((btu == BT_AIR || btu == BT_DIRT) && bh == 0) || (wire_anywhere && btu != BT_WIRE) || (is_line_type && same_type) {
                    // Compute connection mask from neighbors in the line
                    let mut conn: u8 = 0;
                    if is_line_type && tiles.len() > 1 {
                        // Connect to predecessor/successor in the drag line
                        if ti > 0 {
                            let (px, py) = tiles[ti - 1];
                            if px < tx { conn |= CONN_W; }
                            if px > tx { conn |= CONN_E; }
                            if py < ty { conn |= CONN_N; }
                            if py > ty { conn |= CONN_S; }
                        }
                        if ti + 1 < tiles.len() {
                            let (nx, ny) = tiles[ti + 1];
                            if nx > tx { conn |= CONN_E; }
                            if nx < tx { conn |= CONN_W; }
                            if ny > ty { conn |= CONN_S; }
                            if ny < ty { conn |= CONN_N; }
                        }
                        // Also connect to existing adjacent same-type pipes/wires outside the drag
                        for &(ndx, ndy, mask) in &[(0i32,-1i32,CONN_N),(0,1,CONN_S),(1,0,CONN_E),(-1,0,CONN_W)] {
                            let anx = tx + ndx;
                            let any = ty + ndy;
                            if anx < 0 || any < 0 || anx >= GRID_W as i32 || any >= GRID_H as i32 { continue; }
                            let aidx = (any as u32 * GRID_W + anx as u32) as usize;
                            let abt = block_type_rs(self.grid_data[aidx]);
                            let abt = abt as u32;
                            let adj_gas_match = bt_is!(bid, BT_PIPE, BT_RESTRICTOR) && bt_is!(abt, BT_PIPE, BT_RESTRICTOR, BT_PIPE_BRIDGE);
                            let adj_liq_match = bid == BT_LIQUID_PIPE && bt_is!(abt, BT_LIQUID_PIPE, BT_PIPE_BRIDGE);
                            if abt == bid || adj_gas_match || adj_liq_match {
                                conn |= mask;
                            }
                        }
                    } else if is_line_type {
                        // Single tile: connect all directions (auto-detect)
                        conn = CONN_N | CONN_E | CONN_S | CONN_W;
                    }

                    if is_line_type && same_type {
                        if btu == bid {
                            // Same type: just merge new connections into existing mask
                            let existing_h = ((block >> 8) & 0xFF) as u8;
                            let merged = existing_h | conn;
                            self.grid_data[idx] = (block & 0xFFFF00FF) | ((merged as u32) << 8);
                        } else {
                            // Cross-type (pipe↔restrictor): replace block type, inherit connections
                            let existing_conn = ((block >> 8) & 0xF0) as u8;
                            let roof_flag = block_flags_rs(block) & 2;
                            let roof_h = block & 0xFF000000;
                            let base_h = reg.get(block_type_id).and_then(|d| d.placement.as_ref()).map(|p| p.place_height).unwrap_or(1);
                            let height = base_h | existing_conn | conn;
                            self.grid_data[idx] = make_block(block_type_id as u8, height, roof_flag) | roof_h;
                        }
                    } else if wire_anywhere && btu != BT_AIR && btu != BT_DIRT {
                        self.grid_data[idx] |= 0x80 << 16; // wire overlay flag
                    } else {
                        let roof_flag = block_flags_rs(block) & 2;
                        let roof_h = block & 0xFF000000;
                        let base_h = reg.get(block_type_id).and_then(|d| d.placement.as_ref()).map(|p| p.place_height).unwrap_or(3);
                        let height = if is_line_type { base_h | conn } else { base_h };
                        self.place_or_blueprint(tx, ty, make_block(block_type_id as u8, height, roof_flag) | roof_h);
                    }

                    // Update adjacent existing pipes/wires with reciprocal connection
                    if is_line_type {
                        let neighbors: [(i32, i32, u8, u8); 4] = [
                            (0, -1, CONN_N, CONN_S), // if we connect N, neighbor to N gets S
                            (0,  1, CONN_S, CONN_N),
                            (1,  0, CONN_E, CONN_W),
                            (-1, 0, CONN_W, CONN_E),
                        ];
                        let final_h = ((self.grid_data[idx] >> 8) & 0xFF) as u8;
                        let final_mask = final_h & 0xF0;
                        for &(ndx, ndy, our_bit, their_bit) in &neighbors {
                            if (final_mask & our_bit) == 0 { continue; }
                            let nnx = tx + ndx;
                            let nny = ty + ndy;
                            if nnx < 0 || nny < 0 || nnx >= GRID_W as i32 || nny >= GRID_H as i32 { continue; }
                            let nidx = (nny as u32 * GRID_W + nnx as u32) as usize;
                            let nb = self.grid_data[nidx];
                            let nbt = block_type_rs(nb);
                            // Update same-type (or pipe↔restrictor↔bridge) neighbors with connection mask
                            let nbt = nbt as u32;
                            let recip_match = nbt == bid
                                || (bt_is!(bid, BT_PIPE, BT_RESTRICTOR) && bt_is!(nbt, BT_PIPE, BT_RESTRICTOR, BT_PIPE_BRIDGE))
                                || (bid == BT_LIQUID_PIPE && bt_is!(nbt, BT_LIQUID_PIPE, BT_PIPE_BRIDGE));
                            if recip_match {
                                let nh = ((nb >> 8) & 0xFF) as u8;
                                if (nh & 0xF0) != 0 && (nh & their_bit) == 0 {
                                    let updated = nh | their_bit;
                                    self.grid_data[nidx] = (nb & 0xFFFF00FF) | ((updated as u32) << 8);
                                }
                            }
                        }
                    }

                    self.grid_dirty = true;
                }
            }
        }
        // Recompute roof heights after placing walls (needed for shadows)
        if self.grid_dirty {
            compute_roof_heights(&mut self.grid_data);
        }
    }

    /// Destroy a placed block at grid position, reverting to bare dirt.
    /// Cancel a blueprint at the given position, releasing any assigned plebs.
    pub(crate) fn cancel_blueprint(&mut self, bx: i32, by: i32) {
        if self.blueprints.remove(&(bx, by)).is_some() {
            self.active_work.remove(&(bx, by));
            // Release any pleb working on this blueprint
            for pleb in &mut self.plebs {
                if pleb.work_target == Some((bx, by)) || pleb.haul_target == Some((bx, by)) {
                    pleb.work_target = None;
                    pleb.haul_target = None;
                    pleb.harvest_target = None;
                    if matches!(pleb.activity, PlebActivity::Building(_) | PlebActivity::Hauling) {
                        pleb.activity = PlebActivity::Idle;
                        pleb.path.clear();
                    }
                }
            }
        }
    }

    pub(crate) fn destroy_block_at(&mut self, bx: i32, by: i32) {
        if bx < 0 || by < 0 || bx >= GRID_W as i32 || by >= GRID_H as i32 { return; }
        // Also cancel any blueprint at this position
        self.cancel_blueprint(bx, by);
        let idx = (by as u32 * GRID_W + bx as u32) as usize;
        let block = self.grid_data[idx];
        let bt = block_type_rs(block);

        // Air and bare dirt can't be destroyed
        if bt == 0 || (bt == 2 && block == make_block(2, 0, 0)) { return; }

        // Wall-mounted items (fan, inlet, outlet with height > 1): revert to stone wall
        // Revert to bare dirt — no roof, no height, no flags
        self.grid_data[idx] = make_block(2, 0, 0);
        self.grid_dirty = true;
        // Recompute roof heights since we may have removed a wall or roof tile
        compute_roof_heights(&mut self.grid_data);
    }

    /// Build and open a context menu for the given screen and world position.
    /// Place a block or create a blueprint (if not sandbox mode and block is structural).
    pub(crate) fn place_or_blueprint(&mut self, x: i32, y: i32, block_data: u32) {
        let bt = (block_data & 0xFF) as u32;
        // Structural blocks need construction in non-sandbox mode
        let needs_construction = !self.sandbox_mode && matches!(bt,
            BT_STONE | BT_WALL | BT_GLASS | BT_INSULATED |
            BT_WOOD_WALL | BT_STEEL_WALL | BT_SANDSTONE | BT_GRANITE |
            BT_LIMESTONE | BT_MUD_WALL | BT_DIAGONAL |
            BT_WOOD_FLOOR | BT_STONE_FLOOR | BT_CONCRETE_FLOOR |
            BT_FIREPLACE | BT_BENCH | BT_BED | BT_CRATE | BT_CANNON
        );
        if needs_construction {
            self.blueprints.insert((x, y), Blueprint::new(block_data));
        } else {
            let idx = (y as u32 * GRID_W + x as u32) as usize;
            self.grid_data[idx] = block_data;
            self.grid_dirty = true;
        }
    }

    pub(crate) fn open_context_menu(&mut self, screen_x: f32, screen_y: f32, wx: f32, wy: f32) {
        let bx = wx.floor() as i32;
        let by = wy.floor() as i32;
        let bt = if bx >= 0 && by >= 0 && bx < GRID_W as i32 && by < GRID_H as i32 {
            (self.grid_data[(by as u32 * GRID_W + bx as u32) as usize] & 0xFF) as u32
        } else { 0 };
        let sel_pleb = self.selected_pleb;
        let pleb_name = sel_pleb.and_then(|i| self.plebs.get(i)).map(|p| p.name.clone()).unwrap_or_default();

        let mut menu = ContextMenu::new(screen_x, screen_y, "");
        let mut has_actions = false;

        // Harvestable: berry bush or mature crop
        if sel_pleb.is_some() && (bt == BT_BERRY_BUSH || bt == BT_CROP) {
            let can_harvest = if bt == BT_CROP {
                let crop_h = (self.grid_data[(by as u32 * GRID_W + bx as u32) as usize] >> 8) & 0xFF;
                crop_h >= 3
            } else { true };
            if can_harvest {
                menu.title = if bt == BT_BERRY_BUSH { "Berry Bush".into() } else { "Crop".into() };
                menu.actions.push((format!("Harvest ({})", pleb_name), ContextAction::Harvest(bx, by)));
                has_actions = true;
            }
        }

        // Tree: chop down for wood
        if sel_pleb.is_some() && bt == BT_TREE {
            menu.actions.push((format!("Chop down ({})", pleb_name), ContextAction::Harvest(bx, by)));
            has_actions = true;
        }

        // Rock: haul to storage
        if bt == BT_ROCK {
            menu.title = "Rock".into();
            // Find nearest available pleb
            let mut best_pleb: Option<(usize, f32)> = None;
            for (i, p) in self.plebs.iter().enumerate() {
                if p.activity.is_crisis() || p.inventory.carrying.is_some() || p.is_enemy { continue; }
                let dist = ((p.x - bx as f32 - 0.5).powi(2) + (p.y - by as f32 - 0.5).powi(2)).sqrt();
                if best_pleb.is_none() || dist < best_pleb.unwrap().1 {
                    best_pleb = Some((i, dist));
                }
            }
            if let Some((pi, _)) = best_pleb {
                let pn = self.plebs[pi].name.clone();
                menu.actions.push((format!("Haul to storage ({})", pn), ContextAction::Haul(bx, by)));
                has_actions = true;
            }
        }

        // Ground items at this position: eat + haul actions
        if sel_pleb.is_some() {
            for (i, item) in self.ground_items.iter().enumerate() {
                let ix = item.x.floor() as i32;
                let iy = item.y.floor() as i32;
                if ix == bx && iy == by {
                    if menu.title.is_empty() { menu.title = format!("{}", item.kind.label()); }
                    if let resources::ItemKind::Berries(_n) = item.kind {
                        menu.actions.push((format!("Eat 1 berry ({})", pleb_name), ContextAction::Eat(i)));
                        has_actions = true;
                    }
                }
            }
        }

        // Move to (fallback when pleb selected but no specific action)
        if sel_pleb.is_some() && !has_actions {
            menu.title = "Move".into();
            menu.actions.push((format!("Move here ({})", pleb_name), ContextAction::MoveTo(wx, wy)));
            has_actions = true;
        }

        if has_actions {
            self.context_menu = Some(menu);
        }
    }

    pub(crate) fn get_block_bounds(&self, bx: i32, by: i32, bt: u32, flags: u8) -> (i32, i32, i32, i32) {
        let seg = (flags >> 3) & 3;
        let rot = (flags >> 5) & 3;
        if bt == BT_BENCH {
            let ox = if rot == 0 { bx - seg as i32 } else { bx };
            let oy = if rot == 0 { by } else { by - seg as i32 };
            if rot == 0 { (ox, oy, 3, 1) } else { (ox, oy, 1, 3) }
        } else if bt == BT_BED || bt == BT_BATTERY_M {
            let ox = if rot == 0 { bx - seg as i32 } else { bx };
            let oy = if rot == 0 { by } else { by - seg as i32 };
            if rot == 0 { (ox, oy, 2, 1) } else { (ox, oy, 1, 2) }
        } else if bt == BT_SOLAR {
            let row = (flags >> 5) & 3;
            let col = (flags >> 3) & 3;
            (bx - col as i32, by - row as i32, 3, 3)
        } else if bt == BT_BATTERY_L || bt == BT_WIND_TURBINE {
            let col = seg & 1;
            let row = (flags >> 5) & 1;
            (bx - col as i32, by - row as i32, 2, 2)
        } else {
            (bx, by, 1, 1)
        }
    }

    pub(crate) fn handle_build_placement(&mut self, wx: f32, wy: f32, bx: i32, by: i32, idx: usize, block: u32, bt: u32, flags: u8) {
        if self.build_tool == BuildTool::None { return; }
        {
            match self.build_tool {
                BuildTool::Place(37) => {
                    // Solar panel: 3×3 multi-tile placement
                    let tiles = self.solar_tiles(bx, by);
                    if self.place_multi_tiles(&tiles, 37, 0, |i, rf| {
                        rf | (((i % 3) as u8) << 3) | (((i / 3) as u8) << 5)
                    }) {
                        compute_roof_heights(&mut self.grid_data);
                        log::info!("Placed solar panel at ({}, {})", bx, by);
                    }
                }
                BuildTool::Place(41) => {
                    // Wind turbine: 2×2 placement with rotation stored in flags
                    let tiles = [(bx, by), (bx+1, by), (bx, by+1), (bx+1, by+1)];
                    let rot_bit = if self.build_rotation % 2 == 1 { 0x40u8 } else { 0u8 };
                    if self.place_multi_tiles(&tiles, 41, 2, |i, rf| {
                        rf | (((i % 2) as u8) << 3) | (((i / 2) as u8) << 5) | rot_bit
                    }) {
                        compute_roof_heights(&mut self.grid_data);
                    }
                }
                BuildTool::Place(39) => {
                    // Medium battery: 2-tile placement
                    let tiles = self.bed_tiles(bx, by, self.build_rotation);
                    let rot = self.build_rotation as u8;
                    self.place_multi_tiles(&tiles, 39, 1, |i, rf| {
                        rf | ((i as u8) << 3) | (rot << 5)
                    });
                }
                BuildTool::Place(40) => {
                    // Large battery: 2×2 placement
                    let tiles = [(bx, by), (bx+1, by), (bx, by+1), (bx+1, by+1)];
                    self.place_multi_tiles(&tiles, 40, 1, |i, rf| {
                        rf | (((i % 2) as u8) << 3) | (((i / 2) as u8) << 5)
                    });
                }
                BuildTool::Place(50) | BuildTool::Place(51) => {
                    // Pipe bridge (50) or Wire bridge (51): 3-tile placement
                    let bridge_id = match self.build_tool { BuildTool::Place(id) => id, _ => 50 };
                    let tiles = self.bridge_tiles(bx, by, self.build_rotation);
                    let all_valid = tiles.iter().all(|&(tx, ty)| {
                        if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 { return false; }
                        let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                        let tbt = block_type_rs(self.grid_data[tidx]);
                        self.can_place_at(tx, ty)
                            || (bridge_id == 50 && pipes::is_gas_pipe_component(tbt))
                            || (bridge_id == 50 && pipes::is_liquid_pipe_component(tbt))
                            || (bridge_id == 51 && (tbt == BT_WIRE || is_conductor_rs(tbt, block_flags_rs(self.grid_data[tidx]))))
                    });
                    if all_valid {
                        let rot = self.build_rotation as u8;
                        for (i, &(tx, ty)) in tiles.iter().enumerate() {
                            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                            let (roof_flag, roof_h) = extract_roof_data(self.grid_data[tidx]);
                            let seg_flags = roof_flag | ((i as u8) << 3) | (rot << 5);
                            self.grid_data[tidx] = make_block(bridge_id as u8, 1, seg_flags) | roof_h;
                        }
                        self.grid_dirty = true;
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::Place(52) => {
                    // Liquid Intake: 2-tile — one on ground, one on water/dug
                    let tiles = self.bed_tiles(bx, by, self.build_rotation);
                    let (ground_idx, water_idx) = self.intake_tile_assignment(&tiles);
                    if let (Some(gi), Some(wi)) = (ground_idx, water_idx) {
                        let rot = self.build_rotation as u8;
                        for &(seg, ti) in &[(0u8, gi), (1u8, wi)] {
                            let (tx, ty) = tiles[ti];
                            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                            let (rf, rh) = extract_roof_data(self.grid_data[tidx]);
                            self.grid_data[tidx] = make_block(52, 1, rf | (seg << 3) | (rot << 5)) | rh;
                        }
                        self.grid_dirty = true;
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::Place(9) => {
                    // Bench: 3-tile placement
                    let tiles = self.bench_tiles(bx, by, self.build_rotation);
                    let rot = self.build_rotation as u8;
                    if self.place_multi_tiles(&tiles, 9, 1, |i, rf| rf | ((i as u8) << 3) | (rot << 5)) {
                        log::info!("Placed bench at ({}, {})", bx, by);
                    }
                }
                BuildTool::Place(30) => {
                    // Bed: 2-tile placement
                    let tiles = self.bed_tiles(bx, by, self.build_rotation);
                    let rot = self.build_rotation as u8;
                    self.place_multi_tiles(&tiles, 30, 0, |i, rf| rf | ((i as u8) << 3) | (rot << 5));
                }
                BuildTool::Place(11) => {
                    // Table lamp: can only be placed on benches (type 9)
                    if bt == BT_BENCH {
                        let roof_h = block & 0xFF000000;
                        let roof_flag = flags & 2;
                        self.grid_data[idx] = make_block(11, 1, roof_flag) | roof_h;
                        self.grid_dirty = true;
                        log::info!("Placed table lamp at ({}, {})", bx, by);
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::Place(12) => {
                    // Fan: can be placed on a wall OR on the ground
                    let on_wall = is_wall_block(bt) && block_height_rs(block) > 0;
                    let on_ground = self.can_place_at(bx, by);
                    if on_wall || on_ground {
                        let (roof_flag, roof_h) = extract_roof_data(block);
                        let height = if on_wall { block_height_rs(block) } else { 1 };
                        let dir_flags = roof_flag | ((self.build_rotation as u8) << 3);
                        self.grid_data[idx] = make_block(12, height, dir_flags) | roof_h;
                        self.grid_dirty = true;
                        log::info!("Placed fan at ({}, {}) dir={}", bx, by, self.build_rotation);
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::Place(19) | BuildTool::Place(20) => {
                    // Outlet/Inlet: can place on ground OR on walls
                    let on_wall = is_wall_block(bt) && block_height_rs(block) > 0;
                    let on_ground = self.can_place_at(bx, by);
                    if on_ground || on_wall {
                        let height = if on_wall { block_height_rs(block) } else { 1 };
                        let (roof_flag, roof_h) = extract_roof_data(block);
                        let rot_flags = (self.build_rotation as u8) << 3;
                        let bt_new = if self.build_tool == BuildTool::Place(19) { 19 } else { 20 };
                        self.grid_data[idx] = make_block(bt_new, height, roof_flag | rot_flags) | roof_h;
                        self.grid_dirty = true;
                        log::info!("Placed {:?} at ({}, {})", self.build_tool, bx, by);
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::Place(id) => {
                    // Generic Place: use registry for place_height and flags
                    let reg = block_defs::BlockRegistry::cached();
                    let placement = reg.get(id).and_then(|d| d.placement.as_ref());
                    let place_height = placement.map(|p| p.place_height).unwrap_or(1);
                    let extra_flags = placement.map(|p| p.extra_flags).unwrap_or(0);
                    let stays_selected = placement.map(|p| p.stays_selected).unwrap_or(false);
                    let click_mode = placement.map(|p| p.click.clone()).unwrap_or(block_defs::ClickMode::Simple);

                    // Wall-adjacent placement: auto-detect wall direction
                    if click_mode == block_defs::ClickMode::WallAdjacent {
                        if let Some(dir) = self.wall_adjacent_direction(bx, by) {
                            let roof_flag = flags & 2;
                            let roof_h = block & 0xFF000000;
                            let dir_flags = roof_flag | ((dir as u8) << 3);
                            self.place_or_blueprint(bx, by, make_block(id as u8, place_height, dir_flags) | roof_h);
                            self.grid_dirty = true;
                            compute_roof_heights(&mut self.grid_data);
                            log::info!("Placed wall attachment {} at ({}, {}) facing dir={}", id, bx, by, dir);
                            self.build_tool = BuildTool::None;
                        }
                        return;
                    }

                    let bt = bt as u32;
                    let can_place = self.can_place_at(bx, by)
                        || (id == BT_PUMP && bt == BT_PIPE)
                        || (id == BT_LIQUID_PUMP && bt == BT_LIQUID_PIPE)
                        || (id == BT_LIQUID_OUTPUT && bt == BT_LIQUID_PIPE)
                        || (id == BT_WIRE && bt != BT_WIRE)
                        || (id == BT_PIPE && bt == BT_PIPE)
                        || (id == BT_RESTRICTOR && bt_is!(bt, BT_PIPE, BT_RESTRICTOR))
                        || (id == BT_LIQUID_PIPE && bt_is!(bt, BT_LIQUID_PIPE, BT_PIPE_BRIDGE))
                        || (bt_is!(id, BT_SWITCH, BT_DIMMER, BT_BREAKER) && bt_is!(bt, BT_WIRE, BT_AIR, BT_DIRT));
                    if can_place && click_mode != block_defs::ClickMode::None {
                        if id == BT_WIRE && bt != BT_AIR && bt != BT_DIRT {
                            // Wire on non-ground: add wire flag to existing block (bit 7)
                            self.grid_data[idx] |= 0x80 << 16; // set wire overlay flag
                        } else {
                            let roof_flag = flags & 2;
                            let rot_flags = (self.build_rotation as u8) << 3;
                            let mut combined_flags = roof_flag | rot_flags | extra_flags;
                            let mut final_height = place_height;
                            // Switch starts ON (flag bit 2)
                            if id == BT_SWITCH { combined_flags |= 4; }
                            // Dimmer starts at 100% (height = 10)
                            if id == BT_DIMMER { final_height = 10; }
                            // Fireplace starts at 50% intensity (height = 5)
                            if id == BT_FIREPLACE { final_height = 5; }
                            // Circuit breaker starts ON (flag bit 2), threshold in height = 15V
                            if id == BT_BREAKER { combined_flags |= 4; final_height = 15; }
                            // Restrictor starts at 50% opening (height lower nibble = 5)
                            if id == BT_RESTRICTOR { final_height = 5; }
                            // Single-click pipe/wire: auto-detect connections from adjacent matching blocks
                            if bt_is!(id, BT_PIPE, BT_WIRE, BT_RESTRICTOR, BT_LIQUID_PIPE) {
                                let mut conn: u8 = 0;
                                for &(ndx, ndy, mask) in &[(0i32,-1i32,0x10u8),(0,1,0x40),(1,0,0x20),(-1,0,0x80)] {
                                    let nx = bx + ndx;
                                    let ny = by + ndy;
                                    if nx >= 0 && ny >= 0 && nx < GRID_W as i32 && ny < GRID_H as i32 {
                                        let nidx = (ny as u32 * GRID_W + nx as u32) as usize;
                                        let nbt = block_type_rs(self.grid_data[nidx]);
                                        // Connect to same type, or pipes↔restrictors

                                        let is_pipe_match = (bt_is!(id, BT_PIPE, BT_RESTRICTOR) && bt_is!(nbt, BT_PIPE, BT_RESTRICTOR, BT_PIPE_BRIDGE))
                                            || (id == BT_LIQUID_PIPE && bt_is!(nbt, BT_LIQUID_PIPE, BT_PIPE_BRIDGE));
                                        if nbt == id || is_pipe_match {
                                            conn |= mask;
                                            // Also update neighbor's connection toward us
                                            // Skip if neighbor has mask=0 (auto-detect mode — already connects all directions)
                                            let opp_mask: u8 = match mask { 0x10 => 0x40, 0x40 => 0x10, 0x20 => 0x80, _ => 0x20 };
                                            let nb_h = ((self.grid_data[nidx] >> 8) & 0xFF) as u8;
                                            if (nb_h & 0xF0) != 0 {
                                                self.grid_data[nidx] = (self.grid_data[nidx] & 0xFFFF00FF) | (((nb_h | opp_mask) as u32) << 8);
                                            }
                                        }
                                    }
                                }
                                // If no neighbors found, connect all (standalone node)
                                if conn == 0 { conn = 0xF0; }
                                final_height |= conn;
                            }
                            let roof_h = block & 0xFF000000;
                            self.place_or_blueprint(bx, by, make_block(id as u8, final_height, combined_flags) | roof_h);
                        }
                        self.grid_dirty = true;
                        compute_roof_heights(&mut self.grid_data);
                        // Initialize cannon angle from build rotation
                        if id == BT_CANNON {
                            let angle = match self.build_rotation {
                                0 => -std::f32::consts::FRAC_PI_2, // north
                                1 => 0.0,                           // east
                                2 => std::f32::consts::FRAC_PI_2,  // south
                                _ => std::f32::consts::PI,          // west
                            };
                            self.cannon_angles.insert(idx as u32, angle);
                        }
                        log::info!("Placed {:?} at ({}, {})", self.build_tool, bx, by);
                        if !stays_selected {
                            self.build_tool = BuildTool::None;
                        }
                    }
                }
                BuildTool::Window => {
                    // Window (glass): replaces wall blocks
                    if bt_is!(bt, BT_STONE, BT_WALL, BT_INSULATED, BT_WOOD_WALL, BT_STEEL_WALL, BT_SANDSTONE, BT_GRANITE, BT_LIMESTONE) && (block >> 8) & 0xFF > 0 {
                        let height = ((block >> 8) & 0xFF) as u8;
                        let roof_flag = flags & 2;
                        let roof_h = block & 0xFF000000;
                        self.grid_data[idx] = make_block(5, height, roof_flag) | roof_h;
                        self.grid_dirty = true;
                        log::info!("Placed window at ({}, {})", bx, by);
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::Door => {
                    // Door: replaces wall blocks with door
                    if bt_is!(bt, BT_STONE, BT_GLASS, BT_INSULATED, BT_WOOD_WALL, BT_STEEL_WALL, BT_SANDSTONE, BT_GRANITE, BT_LIMESTONE) && (block >> 8) & 0xFF > 0 {
                        let roof_h = block & 0xFF000000;
                        // Door: height 1, flag bit0=is_door, starts closed (bit2=0)
                        self.grid_data[idx] = make_block(4, 1, 1) | roof_h;
                        self.grid_dirty = true;
                        log::info!("Placed door at ({}, {})", bx, by);
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::RemoveFloor => {
                    let block = self.grid_data[idx];
                    let bt_here = block_type_rs(block);
                    if bt_is!(bt_here, BT_WOOD_FLOOR, BT_STONE_FLOOR, BT_CONCRETE_FLOOR) {
                        let roof_flag = block_flags_rs(block) & 2;
                        let roof_h = block & 0xFF000000;
                        self.grid_data[idx] = make_block(2, 0, roof_flag) | roof_h;
                        self.grid_dirty = true;
                    }
                }
                BuildTool::RemoveRoof => {
                    let block = self.grid_data[idx];
                    if (block >> 16) & 2 != 0 {
                        self.grid_data[idx] &= !(2u32 << 16);
                        self.grid_dirty = true;
                        compute_roof_heights(&mut self.grid_data);
                    }
                }
                BuildTool::WoodBox => {
                    self.physics_bodies.push(PhysicsBody::new_wood_box(wx, wy));
                    // Don't deselect — can place multiple
                    return;
                }
                BuildTool::Dig => {
                    // Dig: 20% per click, max depth 5 (= 1 full block).
                    // Water appears at depth >= 1 (20%).
                    if bx >= 0 && by >= 0 && bx < GRID_W as i32 && by < GRID_H as i32 {
                        let bt_dig = block_type_rs(block);
                        let roof_h = block & 0xFF000000;
                        if bt_dig == BT_DIRT || bt_is!(bt_dig, BT_WOOD_FLOOR, BT_STONE_FLOOR, BT_CONCRETE_FLOOR) {
                            // Dirt or floor → dug ground depth 1 (20%)
                            self.grid_data[idx] = make_block(BT_DUG_GROUND as u8, 1, 0) | roof_h;
                            self.grid_dirty = true;
                        } else if bt_dig == BT_DUG_GROUND {
                            let depth = (block >> 8) & 0xFF;
                            if depth < 5 {
                                self.grid_data[idx] = make_block(BT_DUG_GROUND as u8, (depth + 1) as u8, 0) | roof_h;
                                self.grid_dirty = true;
                            }
                        }
                    }
                }
                BuildTool::GrowingZone => {
                    if bt == BT_DIRT {
                        if let Some(zone) = self.zones.iter_mut().find(|z| z.kind == ZoneKind::Growing) {
                            zone.tiles.insert((bx, by));
                        } else {
                            let mut zone = Zone::new(ZoneKind::Growing);
                            zone.tiles.insert((bx, by));
                            self.zones.push(zone);
                        }
                    }
                    return;
                }
                BuildTool::StorageZone => {
                    let bh = (block >> 8) & 0xFF;
                    if bh == 0 { // floor-level tiles only
                        if let Some(zone) = self.zones.iter_mut().find(|z| z.kind == ZoneKind::Storage) {
                            zone.tiles.insert((bx, by));
                        } else {
                            let mut zone = Zone::new(ZoneKind::Storage);
                            zone.tiles.insert((bx, by));
                            self.zones.push(zone);
                        }
                    }
                    return;
                }
                BuildTool::None | BuildTool::Destroy
                | BuildTool::Roof => {}
            }
            return;
        }
    }

    /// Handle click interactions with specific block types (toggles, popups).
    pub(crate) fn handle_block_click(&mut self, bx: i32, by: i32, idx: usize, block: u32, bt: u32, flags: u8) {
        // Toggle door
        if is_door_rs(block) {
            let new_flags = flags ^ 4; // toggle bit2 (is_open)
            let new_block = (block & 0xFF00FFFF) | ((new_flags as u32) << 16);
            self.grid_data[idx] = new_block;
            self.grid_dirty = true;
            let open = (new_flags & 4) != 0;

            // When opening a door: inject outward velocity burst (pressure release)
            // Detect which side is inside (roofed) and push air outward from there
            if open {
                let fx = bx as f32 + 0.5;
                let fy = by as f32 + 0.5;
                // Check neighbors for roofed tiles (inside) vs outdoor
                let mut push_dir = (0.0f32, 0.0f32);
                for &(dx, dy) in &[(0i32, -1i32), (0, 1), (-1, 0), (1, 0)] {
                    let nx = bx + dx;
                    let ny = by + dy;
                    if nx >= 0 && ny >= 0 && nx < GRID_W as i32 && ny < GRID_H as i32 {
                        let nb = self.grid_data[(ny as u32 * GRID_W + nx as u32) as usize];
                        let has_roof = ((nb >> 16) & 2) != 0;
                        if has_roof {
                            // This neighbor is inside — push AWAY from it
                            push_dir.0 -= dx as f32;
                            push_dir.1 -= dy as f32;
                        }
                    }
                }
                let mag = (push_dir.0 * push_dir.0 + push_dir.1 * push_dir.1).sqrt();
                if mag > 0.1 {
                    let norm_x = push_dir.0 / mag;
                    let norm_y = push_dir.1 / mag;
                    // Inject outward velocity slightly inside the room (behind the door)
                    self.fluid_params.splat_x = fx - norm_x * 1.5;
                    self.fluid_params.splat_y = fy - norm_y * 1.5;
                    self.fluid_params.splat_vx = norm_x * 60.0;
                    self.fluid_params.splat_vy = norm_y * 60.0;
                    self.fluid_params.splat_radius = 3.0;
                    self.fluid_params.splat_active = 1.0;
                }
            }

            log::info!("Door at ({}, {}): {}", bx, by, if open { "opened" } else { "closed" });
            return;
        }

        // Toggle valve open/closed
        if bt == BT_VALVE {
            let new_flags = flags ^ 4; // toggle bit2 (is_open)
            let new_block = (block & 0xFF00FFFF) | ((new_flags as u32) << 16);
            self.grid_data[idx] = new_block;
            self.grid_dirty = true;
            let open = (new_flags & 4) != 0;
            log::info!("Valve at ({}, {}): {}", bx, by, if open { "open" } else { "closed" });
            return;
        }

        // Toggle switch on/off
        if bt == BT_SWITCH && self.build_tool != BuildTool::Destroy {
            let new_flags = flags ^ 4; // toggle bit2
            let new_block = (block & 0xFF00FFFF) | ((new_flags as u32) << 16);
            self.grid_data[idx] = new_block;
            self.grid_dirty = true;
            let on = (new_flags & 4) != 0;
            log::info!("Switch at ({}, {}): {}", bx, by, if on { "ON" } else { "OFF" });
            return;
        }

        // Click dimmer, restrictor, or fireplace: show slider popup (shared UI)
        if (bt == BT_DIMMER || bt == BT_RESTRICTOR || bt == BT_FIREPLACE) && self.build_tool != BuildTool::Destroy {
            let didx = by as u32 * GRID_W + bx as u32;
            self.block_sel.dimmer = if self.block_sel.dimmer == Some(didx) { None } else { Some(didx) };
            self.block_sel.dimmer_world = (bx as f32 + 0.5, by as f32 + 0.5);
            return;
        }

        // Click circuit breaker: reset (turn back ON) if tripped
        if bt == BT_BREAKER && self.build_tool != BuildTool::Destroy {
            let is_on = (flags & 4) != 0;
            if !is_on {
                // Reset breaker (turn ON)
                let new_block = (block & 0xFF00FFFF) | (((flags | 4) as u32) << 16);
                self.grid_data[idx] = new_block;
                self.grid_dirty = true;
                log::info!("Circuit breaker at ({}, {}) RESET", bx, by);
            }
            return;
        }

        // Click fan: show speed popup (similar to pump)
        if bt == BT_FAN && self.build_tool != BuildTool::Destroy {
            let fidx = by as u32 * GRID_W + bx as u32;
            self.block_sel.fan = if self.block_sel.fan == Some(fidx) { None } else { Some(fidx) };
            self.block_sel.fan_world = (bx as f32 + 0.5, by as f32 + 0.5);
            return;
        }

        // Click pump: toggle pump speed popup
        if bt == BT_PUMP {
            let pidx = by as u32 * GRID_W + bx as u32;
            self.block_sel.pump = if self.block_sel.pump == Some(pidx) { None } else { Some(pidx) };
            self.block_sel.pump_world = (bx as f32 + 0.5, by as f32 + 0.5);
            return;
        }

        // Removal is handled by the Destroy tool, not by clicking
    }
}

/// Compute tiles for a continuous diagonal wall.
/// Returns (x, y, variant) for each tile — main diagonal tiles + fill tiles
/// that tessellate into a solid wall strip.
/// `rotation` determines which side of the wall is solid (0-3).
pub(crate) fn compute_diagonal_wall_tiles(x0: i32, y0: i32, x1: i32, y1: i32, rotation: u32) -> Vec<(i32, i32, u8)> {
        let dx = x1 - x0;
        let dy = y1 - y0;
        if dx == 0 && dy == 0 { return vec![(x0, y0, rotation as u8)]; }
        // Force true 45° diagonal: use the shorter axis as step count
        let steps = dx.abs().min(dy.abs()).max(1);
        let sx = dx.signum();
        let sy = dy.signum();
        // If either axis is zero, pick a default diagonal direction
        let sx = if sx == 0 { 1 } else { sx };
        let sy = if sy == 0 { 1 } else { sy };

        // Determine drag direction: \ (same sign) or / (opposite sign)
        let is_backslash = (sx > 0) == (sy > 0);

        // Auto-adapt rotation to match drag direction.
        // "below" side = variants 0,1; "above" side = variants 2,3
        let is_below = rotation < 2;
        let main_var: u8 = if is_backslash {
            if is_below { 1 } else { 3 } // \ variants
        } else {
            if is_below { 0 } else { 2 } // / variants
        };
        let fill_var: u8 = main_var ^ 2; // flip side: 0↔2, 1↔3

        let mut tiles = Vec::new();
        let mut x = x0;
        let mut y = y0;

        for i in 0..=steps {
            tiles.push((x, y, main_var));

            // Fill tile between this step and the previous (closes the gap).
            // The fill must go toward the solid side of the main variant.
            // Which of the two candidate positions (x-sx,y) or (x,y-sy) is correct
            // depends on both the variant and the drag direction.
            if i > 0 {
                let use_horizontal = match main_var {
                    0 | 3 => sx < 0, // right-facing: fill right when going left
                    _     => sx > 0, // left-facing: fill left when going right
                };
                let (fx, fy) = if use_horizontal {
                    (x - sx, y)
                } else {
                    (x, y - sy)
                };
                tiles.push((fx, fy, fill_var));
            }

            x += sx;
            y += sy;
        }
        tiles
}
