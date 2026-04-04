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
                if block_type_rs(block) == BT_CRATE {
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
            (bx, by),
            (bx + 1, by),
            (bx + 2, by),
            (bx, by + 1),
            (bx + 1, by + 1),
            (bx + 2, by + 1),
            (bx, by + 2),
            (bx + 1, by + 2),
            (bx + 2, by + 2),
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
            0 => [(bx, by), (bx, by + 1), (bx, by + 2)], // N: goes south
            1 => [(bx, by), (bx + 1, by), (bx + 2, by)], // E: goes east
            2 => [(bx, by), (bx, by - 1), (bx, by - 2)], // S: goes north
            _ => [(bx, by), (bx - 1, by), (bx - 2, by)], // W: goes west
        }
    }

    /// For liquid intake: determine which of the 2 tiles is ground (seg 0) and which is water (seg 1).
    /// Returns (Some(ground_index), Some(water_index)) or (None, None) if invalid.
    pub(crate) fn intake_tile_assignment(
        &self,
        tiles: &[(i32, i32); 2],
    ) -> (Option<usize>, Option<usize>) {
        let in_bounds = |x: i32, y: i32| x >= 0 && y >= 0 && x < GRID_W as i32 && y < GRID_H as i32;
        if !in_bounds(tiles[0].0, tiles[0].1) || !in_bounds(tiles[1].0, tiles[1].1) {
            return (None, None);
        }
        let bt = |i: usize| -> u32 {
            self.grid_data[(tiles[i].1 as u32 * GRID_W + tiles[i].0 as u32) as usize] & 0xFF
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
        let mut result = Vec::with_capacity(4);
        for &(dx, dy, dir) in &dirs {
            let nx = x + dx;
            let ny = y + dy;
            if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 {
                continue;
            }
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
        &mut self,
        tiles: &[(i32, i32)],
        block_id: u8,
        height: u8,
        flags_fn: impl Fn(usize, u8) -> u8,
    ) -> bool {
        let all_valid = tiles.iter().all(|&(tx, ty)| self.can_place_at(tx, ty));
        if !all_valid {
            return false;
        }
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
        if !self.can_place_at(x, y) {
            return None;
        }
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
        let bh = block_height_rs(block) as u32;
        if on_furniture {
            return bt == BT_BENCH;
        }
        // Allow placement on ground tiles (air/dirt with no height)
        // Also allow on tiles that only have wall_data edges (thin walls don't block furniture)
        let is_ground = (bt == 0 || bt == 2) && bh == 0;
        let is_thin_wall_only = is_wall_block(bt) && thin_wall_is_walkable(block);
        let has_wd_only = idx < self.wall_data.len()
            && wd_edges(self.wall_data[idx]) != 0
            && (bt == 0 || bt == 2)
            && bh == 0;
        if !(is_ground || is_thin_wall_only || has_wd_only) {
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
                        if diff > 1.0 {
                            return false;
                        } // too steep
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
        if bt == BT_FIREPLACE || bt == BT_CAMPFIRE || bt == BT_CEILING_LIGHT {
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
                self.grid_data[old_idx] = make_block(2, 0, light_flags & 2);

                // Place light at new position (preserve destination roof flag)
                let new_block = (light_block & 0x0000FFFF) | (dest_flags << 16);
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
            compute_roof_heights_wd(&mut self.grid_data, &self.wall_data);
            self.grid_dirty = true;
        }
    }

    /// Place a wall directly into wall_data (DN-008 Phase 2).
    /// Does NOT write to grid_data — the tile keeps its current block type.
    pub(crate) fn place_wall_edge(
        &mut self,
        tx: i32,
        ty: i32,
        edges: u16,
        thickness: u16,
        material: u16,
    ) {
        self.place_wall_edge_h(tx, ty, edges, thickness, material, 0);
    }

    /// Place a wall edge with explicit height (0 = full/3, 1-7 = explicit).
    pub(crate) fn place_wall_edge_h(
        &mut self,
        tx: i32,
        ty: i32,
        edges: u16,
        thickness: u16,
        material: u16,
        height: u16,
    ) {
        if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 {
            return;
        }
        let idx = (ty as u32 * GRID_W + tx as u32) as usize;
        if idx >= self.wall_data.len() {
            return;
        }
        let existing = self.wall_data[idx];
        let existing_edges = wd_edges(existing);
        let merged_edges = existing_edges | edges;
        self.wall_data[idx] =
            pack_wall_data(merged_edges, thickness, material) | ((height & 7) << WD_HEIGHT_SHIFT);
        // Preserve door/window flags from existing
        self.wall_data[idx] |= existing & (WD_HAS_DOOR | WD_DOOR_OPEN | WD_HAS_WINDOW);
        self.grid_dirty = true;
    }

    /// Check if placing a thin wall with the given edge would create a double wall
    /// (the adjacent tile already has a wall on the mirrored edge).
    pub(crate) fn is_double_wall(&self, tx: i32, ty: i32, edge: u8) -> bool {
        let (nx, ny) = match edge {
            0 => (tx, ty - 1),
            1 => (tx + 1, ty),
            2 => (tx, ty + 1),
            _ => (tx - 1, ty),
        };
        if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 {
            return false;
        }
        let nidx = (ny as u32 * GRID_W + nx as u32) as usize;
        let mirror_edge = (edge + 2) % 4;
        // Check wall_data layer first
        if nidx < self.wall_data.len() && wd_has_edge(self.wall_data[nidx], mirror_edge) {
            return true;
        }
        // Fall back to block grid for legacy walls
        let nb = self.grid_data[nidx];
        let nbt = block_type_rs(nb);
        let nflags = block_flags_rs(nb);
        let nh = block_height_rs(nb);
        let nh_raw = block_height_raw(nb);
        if nh == 0 || !is_wall_block(nbt) {
            return false;
        }
        has_wall_on_edge(nh_raw, nflags, mirror_edge)
    }

    /// Check if tile has wall edges (in wall_data or block grid).
    pub(crate) fn tile_has_walls(&self, tx: i32, ty: i32) -> bool {
        if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 {
            return false;
        }
        let idx = (ty as u32 * GRID_W + tx as u32) as usize;
        // Check wall_data layer
        if idx < self.wall_data.len() && wd_edges(self.wall_data[idx]) != 0 {
            return true;
        }
        // Check block grid (legacy)
        let block = self.grid_data[idx];
        is_wall_block(block_type_rs(block)) && block_height_rs(block) > 0
    }

    /// Compute thin wall flags for a tile in a hollow rect drag.
    /// Returns (edge, is_corner) where edge is 0=N,1=E,2=S,3=W.
    pub(crate) fn thin_wall_edge_for_rect(
        tx: i32,
        ty: i32,
        min_x: i32,
        max_x: i32,
        min_y: i32,
        max_y: i32,
        rotation: u32,
    ) -> (u8, bool) {
        let is_line_h = min_y == max_y && min_x != max_x; // horizontal line
        let is_line_v = min_x == max_x && min_y != max_y; // vertical line
        let is_single = min_x == max_x && min_y == max_y;

        if is_single {
            return (rotation as u8, false);
        }

        // Lines: keep the same edge as the single-tile preview (rotation).
        // This ensures the wall doesn't flip when you start dragging.
        if is_line_h || is_line_v {
            return (rotation as u8 & 3, false);
        }

        // Rectangle: detect which edge of the rect this tile sits on
        let on_top = ty == min_y;
        let on_bot = ty == max_y;
        let on_left = tx == min_x;
        let on_right = tx == max_x;

        let is_corner = (on_top || on_bot) && (on_left || on_right);
        if is_corner {
            // Primary edge + next clockwise = L shape
            let edge = if on_top && on_right {
                0u8 // N → N+E
            } else if on_bot && on_right {
                1 // E → E+S
            } else if on_bot && on_left {
                2 // S → S+W
            } else {
                3 // W → W+N
            };
            (edge, true)
        } else {
            let edge = if on_top {
                0u8
            } else if on_right {
                1
            } else if on_bot {
                2
            } else {
                3
            };
            (edge, false)
        }
    }

    /// Compute tiles for a hollow rectangle (walls) between two corners.
    pub(crate) fn hollow_rect_tiles(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
        Self::hollow_rect_tiles_with_entry(x0, y0, x1, y1, None).0
    }

    /// Compute tiles for a hollow rectangle with an optional entryway.
    /// `entry_side`: 0=auto (use pleb_pos or default south), 1=N, 2=E, 3=S, 4=W.
    /// If `pleb_pos` is given and entry_side is 0, the entryway is placed on the side closest to the pleb.
    /// Returns (tiles, entryway_position).
    pub(crate) fn hollow_rect_tiles_with_entry(
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        pleb_pos: Option<(f32, f32)>,
    ) -> (Vec<(i32, i32)>, Option<(i32, i32)>) {
        Self::hollow_rect_tiles_with_entry_side(x0, y0, x1, y1, pleb_pos, 0)
    }

    /// Compute tiles for a hollow rectangle with an explicit entry side.
    /// `entry_side`: 0=auto (use pleb_pos or default south), 1=N, 2=E, 3=S, 4=W.
    pub(crate) fn hollow_rect_tiles_with_entry_side(
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        pleb_pos: Option<(f32, f32)>,
        entry_side: u8,
    ) -> (Vec<(i32, i32)>, Option<(i32, i32)>) {
        let min_x = x0.min(x1);
        let max_x = x0.max(x1);
        let min_y = y0.min(y1);
        let max_y = y0.max(y1);

        // Room must be at least 3x3 for an entryway to make sense
        let w = max_x - min_x;
        let h = max_y - min_y;
        let entry = if w >= 2 && h >= 2 {
            let mid_x = (min_x + max_x) / 2;
            let mid_y = (min_y + max_y) / 2;
            match entry_side {
                1 => Some((mid_x, min_y)), // North
                2 => Some((max_x, mid_y)), // East
                3 => Some((mid_x, max_y)), // South
                4 => Some((min_x, mid_y)), // West
                _ => {
                    // Auto: use pleb position or default south
                    if let Some((px, py)) = pleb_pos {
                        let d_north = (py - min_y as f32).abs();
                        let d_south = (py - max_y as f32).abs();
                        let d_west = (px - min_x as f32).abs();
                        let d_east = (px - max_x as f32).abs();
                        let min_d = d_north.min(d_south).min(d_west).min(d_east);

                        if min_d == d_north {
                            Some((mid_x, min_y))
                        } else if min_d == d_south {
                            Some((mid_x, max_y))
                        } else if min_d == d_west {
                            Some((min_x, mid_y))
                        } else {
                            Some((max_x, mid_y))
                        }
                    } else {
                        Some((mid_x, max_y)) // default: south
                    }
                }
            }
        } else {
            None // too small for entryway
        };

        let perimeter = 2 * ((max_x - min_x + 1) + (max_y - min_y + 1).saturating_sub(2)) as usize;
        let mut tiles = Vec::with_capacity(perimeter);
        for x in min_x..=max_x {
            if entry != Some((x, min_y)) {
                tiles.push((x, min_y));
            }
            if max_y != min_y && entry != Some((x, max_y)) {
                tiles.push((x, max_y));
            }
        }
        for y in (min_y + 1)..max_y {
            if entry != Some((min_x, y)) {
                tiles.push((min_x, y));
            }
            if max_x != min_x && entry != Some((max_x, y)) {
                tiles.push((max_x, y));
            }
        }
        (tiles, entry)
    }

    pub(crate) fn diagonal_wall_tiles(
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        rotation: u32,
    ) -> Vec<(i32, i32, u8)> {
        compute_diagonal_wall_tiles(x0, y0, x1, y1, rotation)
    }

    /// Compute tiles for a line (pipes) snapped to dominant axis.
    pub(crate) fn line_tiles(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let len = dx.max(dy) as usize + 1;
        let mut tiles = Vec::with_capacity(len);
        if dx >= dy {
            // Horizontal line (ordered in drag direction)
            let step = if x1 >= x0 { 1 } else { -1 };
            let mut x = x0;
            loop {
                tiles.push((x, y0));
                if x == x1 {
                    break;
                }
                x += step;
            }
        } else {
            // Vertical line (ordered in drag direction)
            let step = if y1 >= y0 { 1 } else { -1 };
            let mut y = y0;
            loop {
                tiles.push((x0, y));
                if y == y1 {
                    break;
                }
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
        let w = (max_x - min_x + 1) as usize;
        let h = (max_y - min_y + 1) as usize;
        let mut tiles = Vec::with_capacity(w * h);
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                tiles.push((x, y));
            }
        }
        tiles
    }

    /// Check if a tile can support a roof (wall within 6 Manhattan distance).
    pub(crate) fn can_support_roof_wd(grid: &[u32], wall_data: &[u16], x: i32, y: i32) -> bool {
        let max_dist = 6i32;
        for dy in -max_dist..=max_dist {
            for dx in -max_dist..=max_dist {
                if dx.abs() + dy.abs() > max_dist {
                    continue;
                }
                let nx = x + dx;
                let ny = y + dy;
                if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 {
                    continue;
                }
                let idx = (ny as u32 * GRID_W + nx as u32) as usize;
                // Check wall_data layer (thin walls)
                if idx < wall_data.len() {
                    let wd = wall_data[idx];
                    let edges = wd & 0xF;
                    if edges != 0 {
                        return true;
                    }
                }
                // Check block grid walls
                let b = grid[idx];
                let bt = b & 0xFF;
                let bh = (b >> 8) & 0xFF;
                if bh > 0
                    && bt_is!(
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
                        BT_MUD_WALL
                    )
                {
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
                if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 {
                    continue;
                }
                let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                let bt = self.grid_data[idx] & 0xFF;
                if bt == BT_GROUND {
                    if let Some(zone) = self.zones.iter_mut().find(|z| z.kind == ZoneKind::Growing)
                    {
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
                if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 {
                    continue;
                }
                let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                let bh = (self.grid_data[idx] >> 8) & 0xFF;
                if bh == 0 {
                    // only floor-level tiles
                    if let Some(zone) = self.zones.iter_mut().find(|z| z.kind == ZoneKind::Storage)
                    {
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

        // Dig zone: paint dig area
        if self.build_tool == BuildTool::DigZone {
            let tiles = Self::filled_rect_tiles(sx, sy, ex, ey);
            for (tx, ty) in tiles {
                if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 {
                    continue;
                }
                let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                let bt = block_type_rs(self.grid_data[idx]);
                if bt == BT_GROUND || bt == BT_DUG_GROUND {
                    let base_elev = crate::terrain::sample_elevation(
                        &self.sub_elevation,
                        tx as f32 + 0.5,
                        ty as f32 + 0.5,
                    );
                    if let Some(dz) = self.dig_zones.first_mut() {
                        dz.tiles.insert((tx, ty));
                        dz.base_elevations.entry((tx, ty)).or_insert(base_elev);
                        dz.target_depth = self.dig_depth;
                    } else {
                        let mut dz = zones::DigZone {
                            tiles: std::collections::HashSet::new(),
                            target_depth: self.dig_depth,
                            profile: crate::terrain::CrossProfile::VShape,
                            width: 0.0,
                            base_elevations: std::collections::HashMap::new(),
                        };
                        dz.tiles.insert((tx, ty));
                        dz.base_elevations.insert((tx, ty), base_elev);
                        self.dig_zones.push(dz);
                    }
                    if let Some(zone) = self.zones.iter_mut().find(|z| z.kind == ZoneKind::Dig) {
                        zone.tiles.insert((tx, ty));
                    } else {
                        let mut zone = Zone::new(ZoneKind::Dig);
                        zone.tiles.insert((tx, ty));
                        self.zones.push(zone);
                    }
                }
            }
            return;
        }

        // Berm zone: paint berm area
        if self.build_tool == BuildTool::BermZone {
            let tiles = Self::filled_rect_tiles(sx, sy, ex, ey);
            for (tx, ty) in tiles {
                if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 {
                    continue;
                }
                let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                let bt = block_type_rs(self.grid_data[idx]);
                if bt == BT_GROUND || bt == BT_DUG_GROUND {
                    let base_elev = crate::terrain::sample_elevation(
                        &self.sub_elevation,
                        tx as f32 + 0.5,
                        ty as f32 + 0.5,
                    );
                    if let Some(bz) = self.berm_zones.first_mut() {
                        bz.tiles.insert((tx, ty));
                    } else {
                        let bz = zones::BermZone {
                            tiles: std::collections::HashSet::from([(tx, ty)]),
                            target_height: base_elev + 0.5,
                        };
                        self.berm_zones.push(bz);
                    }
                    if let Some(zone) = self.zones.iter_mut().find(|z| z.kind == ZoneKind::Berm) {
                        zone.tiles.insert((tx, ty));
                    } else {
                        let mut zone = Zone::new(ZoneKind::Berm);
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
                if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 {
                    continue;
                }
                if Self::can_support_roof_wd(&self.grid_data, &self.wall_data, tx, ty) {
                    let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                    let block = self.grid_data[idx];
                    let bh = block_height_rs(block) as u32;
                    if bh == 0 {
                        // only floor-level tiles
                        self.grid_data[idx] |= 2 << 16; // set roof flag (bit 1)
                        self.grid_dirty = true;
                    }
                }
            }
            compute_roof_heights_wd(&mut self.grid_data, &self.wall_data);
            return;
        }

        if self.build_tool == BuildTool::RemoveFloor {
            let tiles = Self::filled_rect_tiles(sx, sy, ex, ey);
            for (tx, ty) in tiles {
                if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 {
                    continue;
                }
                let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                let block = self.grid_data[idx];
                let bt = block & 0xFF;
                // Replace floor types (26/27/28) with dirt (2)
                if bt_is!(
                    bt,
                    BT_WOOD_FLOOR,
                    BT_STONE_FLOOR,
                    BT_CONCRETE_FLOOR,
                    BT_ROUGH_FLOOR
                ) {
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
                if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 {
                    continue;
                }
                let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                let block = self.grid_data[idx];
                let has_roof = (block >> 16) & 2 != 0;
                if has_roof {
                    self.grid_data[idx] &= !(2u32 << 16); // clear roof flag
                    self.grid_dirty = true;
                }
            }
            compute_roof_heights_wd(&mut self.grid_data, &self.wall_data);
            return;
        }

        // Special case: diagonal wall drag places per-tile variants
        if self.build_tool == BuildTool::Place(44) {
            let diag_tiles = Self::diagonal_wall_tiles(sx, sy, ex, ey, self.build_rotation);
            for (tx, ty, variant) in diag_tiles {
                if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 {
                    continue;
                }
                let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                let block = self.grid_data[idx];
                let bt = block_type_rs(block);
                let bh = block_height_rs(block) as u32;
                if (bt == 0 || bt == 2) && bh == 0 {
                    let roof_flag = block_flags_rs(block) & 2;
                    let roof_h = block & 0xFF000000;
                    let flags = roof_flag | (variant << 3);
                    self.grid_data[idx] = make_block(44, 3, flags) | roof_h;
                    self.grid_dirty = true;
                }
            }
            if self.grid_dirty {
                compute_roof_heights_wd(&mut self.grid_data, &self.wall_data);
            }
            return;
        }

        let reg = block_defs::BlockRegistry::cached();
        let (block_type_id, tiles) = match self.build_tool {
            BuildTool::Destroy => (0u32, Self::filled_rect_tiles(sx, sy, ex, ey)),
            BuildTool::Place(id) => {
                let shape = reg
                    .get(id)
                    .and_then(|d| d.placement.as_ref())
                    .and_then(|p| p.drag.as_ref());
                let t = match shape {
                    Some(block_defs::DragShape::Line) => Self::line_tiles(sx, sy, ex, ey),
                    Some(block_defs::DragShape::FilledRect) => {
                        Self::filled_rect_tiles(sx, sy, ex, ey)
                    }
                    Some(block_defs::DragShape::HollowRect) => {
                        let pleb_pos = self
                            .selected_pleb
                            .and_then(|pi| self.plebs.get(pi).map(|p| (p.x, p.y)));
                        Self::hollow_rect_tiles_with_entry_side(
                            sx,
                            sy,
                            ex,
                            ey,
                            pleb_pos,
                            self.entry_side,
                        )
                        .0
                    }
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
            if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 {
                continue;
            }
            if self.build_tool == BuildTool::Destroy {
                self.destroy_block_at(tx, ty);
            } else {
                let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                let block = self.grid_data[idx];
                let bt = block_type_rs(block);
                let bh = block_height_rs(block) as u32;
                let wire_anywhere = bid == BT_WIRE;
                let gas_pipe_compat = bt_is!(bid, BT_PIPE, BT_RESTRICTOR)
                    && bt_is!(bt, BT_PIPE, BT_RESTRICTOR, BT_PIPE_BRIDGE);
                let liquid_pipe_compat =
                    bid == BT_LIQUID_PIPE && bt_is!(bt, BT_LIQUID_PIPE, BT_PIPE_BRIDGE);
                let pipe_compat = gas_pipe_compat || liquid_pipe_compat;
                let same_type = bt == bid || pipe_compat; // allow pipe↔restrictor
                // Thin wall merge: allow placing on existing wall tiles for corner upgrades
                let has_wd_walls = idx < self.wall_data.len() && wd_edges(self.wall_data[idx]) != 0;
                let thin_wall_merge = is_wall_block(bid)
                    && self.wall_thickness < 4
                    && (has_wd_walls || (is_wall_block(bt) && bh > 0));

                if ((bt == BT_AIR || bt == BT_GROUND) && bh == 0)
                    || (wire_anywhere && bt != BT_WIRE)
                    || (is_line_type && same_type)
                    || thin_wall_merge
                {
                    // Compute connection mask from neighbors in the line
                    let mut conn: u8 = 0;
                    if is_line_type && tiles.len() > 1 {
                        // Connect to predecessor/successor in the drag line
                        if ti > 0 {
                            let (px, py) = tiles[ti - 1];
                            if px < tx {
                                conn |= CONN_W;
                            }
                            if px > tx {
                                conn |= CONN_E;
                            }
                            if py < ty {
                                conn |= CONN_N;
                            }
                            if py > ty {
                                conn |= CONN_S;
                            }
                        }
                        if ti + 1 < tiles.len() {
                            let (nx, ny) = tiles[ti + 1];
                            if nx > tx {
                                conn |= CONN_E;
                            }
                            if nx < tx {
                                conn |= CONN_W;
                            }
                            if ny > ty {
                                conn |= CONN_S;
                            }
                            if ny < ty {
                                conn |= CONN_N;
                            }
                        }
                        // Also connect to existing adjacent same-type pipes/wires outside the drag
                        for &(ndx, ndy, mask) in &[
                            (0i32, -1i32, CONN_N),
                            (0, 1, CONN_S),
                            (1, 0, CONN_E),
                            (-1, 0, CONN_W),
                        ] {
                            let anx = tx + ndx;
                            let any = ty + ndy;
                            if anx < 0 || any < 0 || anx >= GRID_W as i32 || any >= GRID_H as i32 {
                                continue;
                            }
                            let aidx = (any as u32 * GRID_W + anx as u32) as usize;
                            let abt = block_type_rs(self.grid_data[aidx]);
                            let adj_gas_match = bt_is!(bid, BT_PIPE, BT_RESTRICTOR)
                                && bt_is!(abt, BT_PIPE, BT_RESTRICTOR, BT_PIPE_BRIDGE);
                            let adj_liq_match = bid == BT_LIQUID_PIPE
                                && bt_is!(abt, BT_LIQUID_PIPE, BT_PIPE_BRIDGE);
                            if abt == bid || adj_gas_match || adj_liq_match {
                                conn |= mask;
                            }
                        }
                    } else if is_line_type {
                        // Single tile: connect all directions (auto-detect)
                        conn = CONN_N | CONN_E | CONN_S | CONN_W;
                    }

                    if is_line_type && same_type {
                        if bt == bid {
                            // Same type: just merge new connections into existing mask
                            let existing_h = (block_height_rs(block) as u32) as u8;
                            let merged = existing_h | conn;
                            self.grid_data[idx] = (block & 0xFFFF00FF) | ((merged as u32) << 8);
                        } else {
                            // Cross-type (pipe↔restrictor): replace block type, inherit connections
                            let existing_conn = ((block >> 8) & 0xF0) as u8;
                            let roof_flag = block_flags_rs(block) & 2;
                            let roof_h = block & 0xFF000000;
                            let base_h = reg
                                .get(block_type_id)
                                .and_then(|d| d.placement.as_ref())
                                .map(|p| p.place_height)
                                .unwrap_or(1);
                            let height = base_h | existing_conn | conn;
                            self.grid_data[idx] =
                                make_block(block_type_id as u8, height, roof_flag) | roof_h;
                        }
                    } else if wire_anywhere && bt != BT_AIR && bt != BT_GROUND {
                        self.grid_data[idx] |= 0x80 << 16; // wire overlay flag
                    } else {
                        let roof_flag = block_flags_rs(block) & 2;
                        let roof_h = block & 0xFF000000;
                        let base_h = reg
                            .get(block_type_id)
                            .and_then(|d| d.placement.as_ref())
                            .map(|p| p.place_height)
                            .unwrap_or(3);
                        let height = if is_line_type { base_h | conn } else { base_h };

                        // Thin wall: compute wall edge from rect position or rotation
                        let is_wall_type = is_wall_block(block_type_id);
                        if is_wall_type && self.wall_thickness < 4 {
                            let (min_x, max_x) = (sx.min(ex), sx.max(ex));
                            let (min_y, max_y) = (sy.min(ey), sy.max(ey));
                            let (edge, is_corner) = Self::thin_wall_edge_for_rect(
                                tx,
                                ty,
                                min_x,
                                max_x,
                                min_y,
                                max_y,
                                self.build_rotation,
                            );

                            // Rule 1: skip if adjacent tile already has wall on mirrored edge
                            if self.is_double_wall(tx, ty, edge) {
                                continue;
                            }

                            let wd_mat = wall_block_to_material(block_type_id);
                            let wd_thick = self.wall_thickness as u16;
                            let wd_h = height as u16; // wall height from blocks.toml place_height

                            // Rule 2: auto-merge edges if tile already has a wall
                            if self.tile_has_walls(tx, ty) {
                                let new_edge_bit = 1u16 << edge;
                                let edges = if is_corner {
                                    new_edge_bit | (1u16 << ((edge + 1) & 3))
                                } else {
                                    new_edge_bit
                                };
                                if self.sandbox_mode {
                                    self.place_wall_edge_h(tx, ty, edges, wd_thick, wd_mat, wd_h);
                                } else {
                                    let bp = Blueprint::new_wall_h(
                                        block_type_id,
                                        edges,
                                        wd_thick,
                                        wd_mat,
                                        wd_h,
                                    );
                                    self.blueprints.insert((tx, ty), bp);
                                }
                                continue;
                            }

                            // New thin wall
                            let new_edge_bit = 1u16 << edge;
                            let edges = if is_corner {
                                new_edge_bit | (1u16 << ((edge + 1) & 3))
                            } else {
                                new_edge_bit
                            };
                            if self.sandbox_mode {
                                self.place_wall_edge_h(tx, ty, edges, wd_thick, wd_mat, wd_h);
                            } else {
                                let bp = Blueprint::new_wall_h(
                                    block_type_id,
                                    edges,
                                    wd_thick,
                                    wd_mat,
                                    wd_h,
                                );
                                self.blueprints.insert((tx, ty), bp);
                            }
                            continue;
                        } else if is_wall_block(block_type_id) {
                            // Full-thickness walls
                            let wd_mat = wall_block_to_material(block_type_id);
                            if self.sandbox_mode {
                                self.place_wall_edge_h(
                                    tx,
                                    ty,
                                    WD_EDGE_MASK,
                                    4,
                                    wd_mat,
                                    height as u16,
                                );
                            } else {
                                let bp = Blueprint::new_wall_h(
                                    block_type_id,
                                    WD_EDGE_MASK,
                                    4,
                                    wd_mat,
                                    height as u16,
                                );
                                self.blueprints.insert((tx, ty), bp);
                            }
                        } else {
                            self.place_or_blueprint(
                                tx,
                                ty,
                                make_block(block_type_id as u8, height, roof_flag) | roof_h,
                            );
                        }
                    }

                    // Update adjacent existing pipes/wires with reciprocal connection
                    if is_line_type {
                        let neighbors: [(i32, i32, u8, u8); 4] = [
                            (0, -1, CONN_N, CONN_S), // if we connect N, neighbor to N gets S
                            (0, 1, CONN_S, CONN_N),
                            (1, 0, CONN_E, CONN_W),
                            (-1, 0, CONN_W, CONN_E),
                        ];
                        let final_h = ((self.grid_data[idx] >> 8) & 0xFF) as u8;
                        let final_mask = final_h & 0xF0;
                        for &(ndx, ndy, our_bit, their_bit) in &neighbors {
                            if (final_mask & our_bit) == 0 {
                                continue;
                            }
                            let nnx = tx + ndx;
                            let nny = ty + ndy;
                            if nnx < 0 || nny < 0 || nnx >= GRID_W as i32 || nny >= GRID_H as i32 {
                                continue;
                            }
                            let nidx = (nny as u32 * GRID_W + nnx as u32) as usize;
                            let nb = self.grid_data[nidx];
                            let nbt = block_type_rs(nb);
                            let recip_match = nbt == bid
                                || (bt_is!(bid, BT_PIPE, BT_RESTRICTOR)
                                    && bt_is!(nbt, BT_PIPE, BT_RESTRICTOR, BT_PIPE_BRIDGE))
                                || (bid == BT_LIQUID_PIPE
                                    && bt_is!(nbt, BT_LIQUID_PIPE, BT_PIPE_BRIDGE));
                            if recip_match {
                                let nh = ((nb >> 8) & 0xFF) as u8;
                                if (nh & 0xF0) != 0 && (nh & their_bit) == 0 {
                                    let updated = nh | their_bit;
                                    self.grid_data[nidx] =
                                        (nb & 0xFFFF00FF) | ((updated as u32) << 8);
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
            compute_roof_heights_wd(&mut self.grid_data, &self.wall_data);
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
                    if matches!(
                        pleb.activity,
                        PlebActivity::Building(_) | PlebActivity::Hauling
                    ) {
                        pleb.activity = PlebActivity::Idle;
                        pleb.path.clear();
                    }
                }
            }
        }
    }

    pub(crate) fn destroy_block_at(&mut self, bx: i32, by: i32) {
        if bx < 0 || by < 0 || bx >= GRID_W as i32 || by >= GRID_H as i32 {
            return;
        }
        // Also cancel any blueprint at this position
        self.cancel_blueprint(bx, by);
        let idx = (by as u32 * GRID_W + bx as u32) as usize;
        let block = self.grid_data[idx];
        let bt = block_type_rs(block);

        // Air and bare dirt can't be destroyed (unless they have wall_data)
        let has_wd = idx < self.wall_data.len() && self.wall_data[idx] != 0;
        if !has_wd && (bt == 0 || (bt == 2 && block == make_block(2, 0, 0))) {
            return;
        }

        // Wall-mounted items (fan, inlet, outlet with height > 1): revert to stone wall
        // Revert to bare dirt — no roof, no height, no flags
        self.grid_data[idx] = make_block(2, 0, 0);
        // Also clear wall_data at this tile
        if idx < self.wall_data.len() {
            self.wall_data[idx] = 0;
        }
        self.grid_dirty = true;
        // Recompute roof heights since we may have removed a wall or roof tile
        compute_roof_heights_wd(&mut self.grid_data, &self.wall_data);
    }

    /// Build and open a context menu for the given screen and world position.
    /// Place a block or create a blueprint (if not sandbox mode and block is structural).
    pub(crate) fn place_or_blueprint(&mut self, x: i32, y: i32, block_data: u32) {
        let bt = block_data & 0xFF;
        // Non-buildable block types that always place instantly (terrain, vegetation)
        let always_instant = matches!(
            bt,
            BT_AIR
                | BT_GROUND
                | BT_WATER
                | BT_TREE
                | BT_BERRY_BUSH
                | BT_CROP
                | BT_DUG_GROUND
                | BT_ROCK
        );
        if !self.sandbox_mode && !always_instant {
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
            self.grid_data[(by as u32 * GRID_W + bx as u32) as usize] & 0xFF
        } else {
            0
        };
        let sel_pleb = self.selected_pleb;
        let pleb_name = sel_pleb
            .and_then(|i| self.plebs.get(i))
            .map(|p| p.name.clone())
            .unwrap_or_default();

        let mut menu = ContextMenu::new(screen_x, screen_y, "");
        let mut has_actions = false;

        // Harvestable: berry bush or mature crop
        if sel_pleb.is_some() && (bt == BT_BERRY_BUSH || bt == BT_CROP) {
            let block_h = (self.grid_data[(by as u32 * GRID_W + bx as u32) as usize] >> 8) & 0xFF;
            let can_harvest = if bt == BT_CROP {
                block_h >= 3 // mature crop
            } else {
                block_h > 0 // berry bush with berries remaining
            };
            if can_harvest {
                menu.title = if bt == BT_BERRY_BUSH {
                    "Berry Bush".into()
                } else {
                    "Crop".into()
                };
                menu.actions.push((
                    format!("Harvest ({})", pleb_name),
                    ContextAction::Harvest(bx, by),
                    true,
                ));
                has_actions = true;
            }
        }

        // Tree: gather branches (no axe) or chop down (needs axe on belt)
        if sel_pleb.is_some() && bt == BT_TREE {
            menu.actions.push((
                format!("Gather branches ({})", pleb_name),
                ContextAction::GatherBranches(bx, by),
                true,
            ));
            let has_axe = sel_pleb
                .and_then(|pi| self.plebs.get(pi))
                .is_some_and(|p| p.has_tool("axe"));
            let chop_label = if has_axe {
                format!("\u{1fa93} Chop down ({})", pleb_name)
            } else {
                "\u{1fa93} Chop down (needs axe)".to_string()
            };
            menu.actions
                .push((chop_label, ContextAction::Harvest(bx, by), has_axe));
            has_actions = true;
        }

        // Rock: mine (needs pick) or pick up small rocks
        if bt == BT_ROCK && sel_pleb.is_some() {
            menu.title = "Rock".into();
            let has_pick = sel_pleb
                .and_then(|pi| self.plebs.get(pi))
                .is_some_and(|p| p.has_tool("pick"));
            let mine_label = if has_pick {
                format!("\u{26cf} Mine rock ({})", pleb_name)
            } else {
                "\u{26cf} Mine rock (needs pick)".to_string()
            };
            menu.actions
                .push((mine_label, ContextAction::Haul(bx, by), has_pick));
            has_actions = true;
        }

        // Enemy pleb at this position: fire at target
        if sel_pleb.is_some() {
            for (ei, enemy) in self.plebs.iter().enumerate() {
                if !enemy.is_enemy || enemy.is_dead {
                    continue;
                }
                let edist = ((wx - enemy.x).abs()).max((wy - enemy.y).abs());
                if edist < 0.8 {
                    menu.title = enemy.name.clone();
                    let has_ranged = sel_pleb
                        .and_then(|pi| self.plebs.get(pi))
                        .map(|p| {
                            p.inventory.stacks.iter().any(|s| {
                                item_defs::ItemRegistry::cached()
                                    .get(s.item_id)
                                    .map_or(false, |d| d.is_ranged_weapon())
                            })
                        })
                        .unwrap_or(false);
                    menu.actions.push((
                        format!("\u{1f52b} Fire at {} ({})", enemy.name, pleb_name),
                        ContextAction::FireAt(ei),
                        has_ranged,
                    ));
                    has_actions = true;
                    break;
                }
            }
        }

        // Creature at this position: hunt (alive) or butcher (dead)
        if sel_pleb.is_some() {
            for (ci, creature) in self.creatures.iter().enumerate() {
                let cdist = ((wx - creature.x).abs()).max((wy - creature.y).abs());
                if cdist < 0.8 {
                    let cname = creature_defs::CreatureRegistry::cached()
                        .name(creature.species_id)
                        .to_string();
                    menu.title = cname.clone();
                    if creature.is_dead {
                        // Dead creature: offer butcher
                        let def =
                            creature_defs::CreatureRegistry::cached().get(creature.species_id);
                        if def.is_some_and(|d| d.drops_item > 0) {
                            let has_knife = sel_pleb
                                .and_then(|pi| self.plebs.get(pi))
                                .is_some_and(|p| p.has_tool("knife"));
                            let label = if has_knife {
                                format!("\u{1f52a} Butcher {} ({})", cname, pleb_name)
                            } else {
                                format!("\u{1f52a} Butcher (needs knife)")
                            };
                            menu.actions.push((
                                label,
                                ContextAction::Butcher(
                                    creature.x.floor() as i32,
                                    creature.y.floor() as i32,
                                ),
                                has_knife,
                            ));
                            has_actions = true;
                        }
                    } else {
                        menu.actions.push((
                            format!("\u{1f3af} Hunt {} ({})", cname, pleb_name),
                            ContextAction::Hunt(ci),
                            true,
                        ));
                        has_actions = true;
                    }
                    break;
                }
            }
        }

        // Ground items at this position: eat + haul actions
        if sel_pleb.is_some() {
            for (i, item) in self.ground_items.iter().enumerate() {
                let ix = item.x.floor() as i32;
                let iy = item.y.floor() as i32;
                if ix == bx && iy == by {
                    if menu.title.is_empty() {
                        menu.title = item.stack.label();
                    }
                    if item.stack.item_id == item_defs::ITEM_BERRIES {
                        menu.actions.push((
                            format!("Eat 1 berry ({})", pleb_name),
                            ContextAction::Eat(i),
                            true,
                        ));
                    }
                    // Equip: weapons, tools, or belt
                    {
                        let item_reg = item_defs::ItemRegistry::cached();
                        if let Some(def) = item_reg.get(item.stack.item_id) {
                            if def.is_belt_item() {
                                menu.actions.push((
                                    format!("\u{1f9e4} Equip {} ({})", def.name, pleb_name),
                                    ContextAction::Equip(i),
                                    true,
                                ));
                            } else if def.is_belt {
                                let already_has = sel_pleb
                                    .and_then(|pi| self.plebs.get(pi))
                                    .is_some_and(|p| p.equipment.belt_capacity > 0);
                                let label = if already_has {
                                    format!("\u{1f9e4} Replace belt ({})", pleb_name)
                                } else {
                                    format!("\u{1f9e4} Equip {} ({})", def.name, pleb_name)
                                };
                                menu.actions.push((label, ContextAction::Equip(i), true));
                            }
                        }
                    }
                    // Haul any ground item to nearest crate
                    menu.actions.push((
                        format!("Haul {} ({})", item.stack.label(), pleb_name),
                        ContextAction::Haul(bx, by),
                        true,
                    ));
                    has_actions = true;
                }
            }
        }

        // Fish: right-click near water with a fishing line
        if sel_pleb.is_some() {
            let widx = (by as u32 * GRID_W + bx as u32) as usize;
            let has_water = widx < self.water_depth_cpu.len() && self.water_depth_cpu[widx] > 0.3;
            if has_water {
                let has_line = sel_pleb.and_then(|pi| self.plebs.get(pi)).is_some_and(|p| {
                    p.has_tool("fishing") || p.inventory.count_of(item_defs::ITEM_FISHING_LINE) > 0
                });
                let fish_label = if has_line {
                    format!("\u{1f3a3} Fish here ({})", pleb_name)
                } else {
                    "\u{1f3a3} Fish here (needs line)".to_string()
                };
                menu.title = "Water".into();
                menu.actions
                    .push((fish_label, ContextAction::Fish(bx, by), has_line));
                has_actions = true;
            }
        }

        // Hand-craft recipes — only when right-clicking ON or near the pleb
        if let Some(pi) = sel_pleb
            && let Some(pleb) = self.plebs.get(pi)
        {
            let near_pleb = (wx - pleb.x).abs() < 1.5 && (wy - pleb.y).abs() < 1.5;
            if near_pleb {
                let idle = matches!(pleb.activity, PlebActivity::Idle | PlebActivity::Walking);
                let recipe_reg = recipe_defs::RecipeRegistry::cached();
                let item_reg = item_defs::ItemRegistry::cached();
                for recipe in recipe_reg.for_station("hand") {
                    let has_mats = idle
                        && recipe
                            .inputs
                            .iter()
                            .all(|ing| pleb.inventory.count_of(ing.item) >= ing.count as u32);
                    let ing_text: Vec<String> = recipe
                        .inputs
                        .iter()
                        .map(|ing| format!("{} {}", ing.count, item_reg.name(ing.item)))
                        .collect();
                    let label = format!("Craft {} ({})", recipe.name, ing_text.join(", "));
                    menu.actions
                        .push((label, ContextAction::HandCraft(recipe.id), has_mats));
                    has_actions = true;
                }
            }
        }

        // Move to (always available when pleb selected)
        if sel_pleb.is_some() {
            menu.actions.push((
                format!("Move here ({})", pleb_name),
                ContextAction::MoveTo(wx, wy),
                true,
            ));
            // (Grenade throwing moved to action bar targeting mode)
            has_actions = true;
        }

        if has_actions {
            // If the only action is "Move here", execute it directly
            if menu.actions.len() == 1 {
                if let Some((_, ContextAction::MoveTo(mx, my), true)) = menu.actions.first() {
                    let (mx, my) = (*mx, *my);
                    // Collect move indices: group or single pleb
                    let move_indices: Vec<usize> = if !self.selected_group.is_empty() {
                        self.selected_group.clone()
                    } else if let Some(idx) = sel_pleb {
                        vec![idx]
                    } else {
                        vec![]
                    };
                    let offsets = crate::comms::spread_offsets(
                        move_indices.len(),
                        self.flock_spacing.min_spacing(),
                    );
                    for (k, &pi) in move_indices.iter().enumerate() {
                        if let Some(pleb) = self.plebs.get_mut(pi) {
                            let (ox, oy) = offsets[k];
                            let gx = (mx + ox).floor() as i32;
                            let gy = (my + oy).floor() as i32;
                            let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                            let path = pleb::astar_path_terrain_water_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                &self.water_depth_cpu,
                                start,
                                (gx, gy),
                            );
                            if !path.is_empty() {
                                pleb.path = path;
                                pleb.path_idx = 1;
                                pleb.activity = PlebActivity::Walking;
                                pleb.work_target = None;
                                pleb.haul_target = None;
                                pleb.harvest_target = None;
                            }
                        }
                    }
                    self.move_marker = Some((mx.floor() + 0.5, my.floor() + 0.5, 2.0));
                    return;
                }
            }
            self.context_menu = Some(menu);
        }
    }

    pub(crate) fn get_block_bounds(
        &self,
        bx: i32,
        by: i32,
        bt: u32,
        flags: u8,
    ) -> (i32, i32, i32, i32) {
        let seg = (flags >> 3) & 3;
        let rot = (flags >> 5) & 3;
        if bt == BT_BENCH {
            let ox = if rot == 0 { bx - seg as i32 } else { bx };
            let oy = if rot == 0 { by } else { by - seg as i32 };
            if rot == 0 {
                (ox, oy, 3, 1)
            } else {
                (ox, oy, 1, 3)
            }
        } else if bt == BT_BED || bt == BT_BATTERY_M {
            let ox = if rot == 0 { bx - seg as i32 } else { bx };
            let oy = if rot == 0 { by } else { by - seg as i32 };
            if rot == 0 {
                (ox, oy, 2, 1)
            } else {
                (ox, oy, 1, 2)
            }
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

    pub(crate) fn handle_build_placement(
        &mut self,
        wx: f32,
        wy: f32,
        bx: i32,
        by: i32,
        idx: usize,
        block: u32,
        bt: u32,
        flags: u8,
    ) {
        if self.build_tool == BuildTool::None {
            return;
        }
        {
            match self.build_tool {
                BuildTool::Place(37) => {
                    // Solar panel: 3×3 multi-tile placement
                    let tiles = self.solar_tiles(bx, by);
                    if self.place_multi_tiles(&tiles, 37, 0, |i, rf| {
                        rf | (((i % 3) as u8) << 3) | (((i / 3) as u8) << 5)
                    }) {
                        compute_roof_heights_wd(&mut self.grid_data, &self.wall_data);
                        log::info!("Placed solar panel at ({}, {})", bx, by);
                    }
                }
                BuildTool::Place(41) => {
                    // Wind turbine: 2×2 placement with rotation stored in flags
                    let tiles = [(bx, by), (bx + 1, by), (bx, by + 1), (bx + 1, by + 1)];
                    let rot_bit = if self.build_rotation % 2 == 1 {
                        0x40u8
                    } else {
                        0u8
                    };
                    if self.place_multi_tiles(&tiles, 41, 2, |i, rf| {
                        rf | (((i % 2) as u8) << 3) | (((i / 2) as u8) << 5) | rot_bit
                    }) {
                        compute_roof_heights_wd(&mut self.grid_data, &self.wall_data);
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
                    let tiles = [(bx, by), (bx + 1, by), (bx, by + 1), (bx + 1, by + 1)];
                    self.place_multi_tiles(&tiles, 40, 1, |i, rf| {
                        rf | (((i % 2) as u8) << 3) | (((i / 2) as u8) << 5)
                    });
                }
                BuildTool::Place(50) | BuildTool::Place(51) => {
                    // Pipe bridge (50) or Wire bridge (51): 3-tile placement
                    let bridge_id = match self.build_tool {
                        BuildTool::Place(id) => id,
                        _ => 50,
                    };
                    let tiles = self.bridge_tiles(bx, by, self.build_rotation);
                    let all_valid = tiles.iter().all(|&(tx, ty)| {
                        if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 {
                            return false;
                        }
                        let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                        let tbt = block_type_rs(self.grid_data[tidx]);
                        self.can_place_at(tx, ty)
                            || (bridge_id == 50 && pipes::is_gas_pipe_component(tbt))
                            || (bridge_id == 50 && pipes::is_liquid_pipe_component(tbt))
                            || (bridge_id == 51
                                && (tbt == BT_WIRE
                                    || is_conductor_rs(tbt, block_flags_rs(self.grid_data[tidx]))))
                    });
                    if all_valid {
                        let rot = self.build_rotation as u8;
                        for (i, &(tx, ty)) in tiles.iter().enumerate() {
                            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                            let (roof_flag, roof_h) = extract_roof_data(self.grid_data[tidx]);
                            let seg_flags = roof_flag | ((i as u8) << 3) | (rot << 5);
                            self.grid_data[tidx] =
                                make_block(bridge_id as u8, 1, seg_flags) | roof_h;
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
                            self.grid_data[tidx] =
                                make_block(52, 1, rf | (seg << 3) | (rot << 5)) | rh;
                        }
                        self.grid_dirty = true;
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::Place(9) => {
                    // Bench: 3-tile placement
                    let tiles = self.bench_tiles(bx, by, self.build_rotation);
                    let rot = self.build_rotation as u8;
                    if self
                        .place_multi_tiles(&tiles, 9, 1, |i, rf| rf | ((i as u8) << 3) | (rot << 5))
                    {
                        log::info!("Placed bench at ({}, {})", bx, by);
                    }
                }
                BuildTool::Place(30) => {
                    // Bed: 2-tile placement
                    let tiles = self.bed_tiles(bx, by, self.build_rotation);
                    let rot = self.build_rotation as u8;
                    self.place_multi_tiles(&tiles, 30, 0, |i, rf| {
                        rf | ((i as u8) << 3) | (rot << 5)
                    });
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
                        let bt_new = if self.build_tool == BuildTool::Place(19) {
                            19
                        } else {
                            20
                        };
                        self.grid_data[idx] =
                            make_block(bt_new, height, roof_flag | rot_flags) | roof_h;
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
                    let click_mode = placement.map_or(block_defs::ClickMode::Simple, |p| p.click);

                    // Wall-adjacent placement: auto-detect wall direction
                    if click_mode == block_defs::ClickMode::WallAdjacent {
                        if let Some(dir) = self.wall_adjacent_direction(bx, by) {
                            let roof_flag = flags & 2;
                            let roof_h = block & 0xFF000000;
                            let dir_flags = roof_flag | ((dir as u8) << 3);
                            self.place_or_blueprint(
                                bx,
                                by,
                                make_block(id as u8, place_height, dir_flags) | roof_h,
                            );
                            self.grid_dirty = true;
                            compute_roof_heights_wd(&mut self.grid_data, &self.wall_data);
                            log::info!(
                                "Placed wall attachment {} at ({}, {}) facing dir={}",
                                id,
                                bx,
                                by,
                                dir
                            );
                            self.build_tool = BuildTool::None;
                        }
                        return;
                    }

                    // Well: must be placed on dug ground with water table
                    if id == BT_WELL {
                        if bt == BT_DUG_GROUND {
                            let (roof_flag, roof_h) = extract_roof_data(block);
                            self.place_or_blueprint(
                                bx,
                                by,
                                make_block(id as u8, place_height, roof_flag) | roof_h,
                            );
                            self.grid_dirty = true;
                            compute_roof_heights_wd(&mut self.grid_data, &self.wall_data);
                            log::info!("Placed well at ({}, {})", bx, by);
                            self.build_tool = BuildTool::None;
                        }
                        return;
                    }

                    let can_place = self.can_place_at(bx, by)
                        || (id == BT_PUMP && bt == BT_PIPE)
                        || (id == BT_LIQUID_PUMP && bt == BT_LIQUID_PIPE)
                        || (id == BT_LIQUID_OUTPUT && bt == BT_LIQUID_PIPE)
                        || (id == BT_WIRE && bt != BT_WIRE)
                        || (id == BT_PIPE && bt == BT_PIPE)
                        || (id == BT_RESTRICTOR && bt_is!(bt, BT_PIPE, BT_RESTRICTOR))
                        || (id == BT_LIQUID_PIPE && bt_is!(bt, BT_LIQUID_PIPE, BT_PIPE_BRIDGE))
                        || (bt_is!(id, BT_SWITCH, BT_DIMMER, BT_BREAKER)
                            && bt_is!(bt, BT_WIRE, BT_AIR, BT_GROUND));
                    if can_place && click_mode != block_defs::ClickMode::None {
                        if id == BT_WIRE && bt != BT_AIR && bt != BT_GROUND {
                            // Wire on non-ground: add wire flag to existing block (bit 7)
                            self.grid_data[idx] |= 0x80 << 16; // set wire overlay flag
                        } else {
                            let roof_flag = flags & 2;
                            let rot_flags = (self.build_rotation as u8) << 3;
                            let mut combined_flags = roof_flag | rot_flags | extra_flags;
                            let mut final_height = place_height;
                            // Switch starts ON (flag bit 2)
                            if id == BT_SWITCH {
                                combined_flags |= 4;
                            }
                            // Dimmer starts at 100% (height = 10)
                            if id == BT_DIMMER {
                                final_height = 10;
                            }
                            // Fire blocks: default intensity
                            if id == BT_FIREPLACE {
                                final_height = 5;
                            } else if id == BT_CAMPFIRE {
                                final_height = 3;
                                // Subtile placement: encode offset in flags bits 3-6
                                // x_off in bits 3-4 (0-2), y_off in bits 5-6 (0-2)
                                let shift_held = self
                                    .pressed_keys
                                    .contains(&winit::keyboard::KeyCode::ShiftLeft)
                                    || self
                                        .pressed_keys
                                        .contains(&winit::keyboard::KeyCode::ShiftRight);
                                let (sub_x, sub_y) = if shift_held {
                                    // Map fractional position to subtile: 0..1 → 0..4 subtiles,
                                    // then clamp offset for 2x2 block (max offset = 2)
                                    let fx = (wx.fract() + 1.0).fract(); // ensure positive
                                    let fy = (wy.fract() + 1.0).fract();
                                    let sx = ((fx * 4.0).floor() as u8).min(2);
                                    let sy = ((fy * 4.0).floor() as u8).min(2);
                                    (sx, sy)
                                } else {
                                    (1, 1) // default: centered
                                };
                                combined_flags |= (sub_x & 3) << 3;
                                combined_flags |= (sub_y & 3) << 5;
                            }
                            // Circuit breaker starts ON (flag bit 2), threshold in height = 15V
                            if id == BT_BREAKER {
                                combined_flags |= 4;
                                final_height = 15;
                            }
                            // Restrictor starts at 50% opening (height lower nibble = 5)
                            if id == BT_RESTRICTOR {
                                final_height = 5;
                            }
                            // Single-click pipe/wire: auto-detect connections from adjacent matching blocks
                            if bt_is!(id, BT_PIPE, BT_WIRE, BT_RESTRICTOR, BT_LIQUID_PIPE) {
                                let mut conn: u8 = 0;
                                for &(ndx, ndy, mask) in &[
                                    (0i32, -1i32, 0x10u8),
                                    (0, 1, 0x40),
                                    (1, 0, 0x20),
                                    (-1, 0, 0x80),
                                ] {
                                    let nx = bx + ndx;
                                    let ny = by + ndy;
                                    if nx >= 0
                                        && ny >= 0
                                        && nx < GRID_W as i32
                                        && ny < GRID_H as i32
                                    {
                                        let nidx = (ny as u32 * GRID_W + nx as u32) as usize;
                                        let nbt = block_type_rs(self.grid_data[nidx]);
                                        // Connect to same type, or pipes↔restrictors

                                        let is_pipe_match = (bt_is!(id, BT_PIPE, BT_RESTRICTOR)
                                            && bt_is!(nbt, BT_PIPE, BT_RESTRICTOR, BT_PIPE_BRIDGE))
                                            || (id == BT_LIQUID_PIPE
                                                && bt_is!(nbt, BT_LIQUID_PIPE, BT_PIPE_BRIDGE));
                                        if nbt == id || is_pipe_match {
                                            conn |= mask;
                                            // Also update neighbor's connection toward us
                                            // Skip if neighbor has mask=0 (auto-detect mode — already connects all directions)
                                            let opp_mask: u8 = match mask {
                                                0x10 => 0x40,
                                                0x40 => 0x10,
                                                0x20 => 0x80,
                                                _ => 0x20,
                                            };
                                            let nb_h = ((self.grid_data[nidx] >> 8) & 0xFF) as u8;
                                            if (nb_h & 0xF0) != 0 {
                                                self.grid_data[nidx] = (self.grid_data[nidx]
                                                    & 0xFFFF00FF)
                                                    | (((nb_h | opp_mask) as u32) << 8);
                                            }
                                        }
                                    }
                                }
                                // If no neighbors found, connect all (standalone node)
                                if conn == 0 {
                                    conn = 0xF0;
                                }
                                final_height |= conn;
                            }
                            let roof_h = block & 0xFF000000;
                            self.place_or_blueprint(
                                bx,
                                by,
                                make_block(id as u8, final_height, combined_flags) | roof_h,
                            );
                        }
                        self.grid_dirty = true;
                        compute_roof_heights_wd(&mut self.grid_data, &self.wall_data);
                        // Initialize cannon angle from build rotation
                        if id == BT_CANNON {
                            let angle = match self.build_rotation {
                                0 => -std::f32::consts::FRAC_PI_2, // north
                                1 => 0.0,                          // east
                                2 => std::f32::consts::FRAC_PI_2,  // south
                                _ => std::f32::consts::PI,         // west
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
                    if bt_is!(
                        bt,
                        BT_STONE,
                        BT_WALL,
                        BT_INSULATED,
                        BT_WOOD_WALL,
                        BT_STEEL_WALL,
                        BT_SANDSTONE,
                        BT_GRANITE,
                        BT_LIMESTONE
                    ) && block_height_rs(block) as u32 > 0
                    {
                        let height = (block_height_rs(block) as u32) as u8;
                        let roof_flag = flags & 2;
                        let roof_h = block & 0xFF000000;
                        self.grid_data[idx] = make_block(5, height, roof_flag) | roof_h;
                        self.grid_dirty = true;
                        log::info!("Placed window at ({}, {})", bx, by);
                        self.build_tool = BuildTool::None;
                    }
                }
                BuildTool::Door => {
                    // Door: place on a wall_data edge tile
                    let wd = if idx < self.wall_data.len() {
                        self.wall_data[idx]
                    } else {
                        0
                    };
                    let has_wall_edges = wd_edges(wd) != 0;
                    // Also accept legacy wall blocks
                    let legacy_wall = is_wall_block(bt) && block_height_rs(block) as u32 > 0;
                    if has_wall_edges || legacy_wall {
                        // Already has a door here? Skip
                        if (wd & WD_HAS_DOOR) != 0
                            || self.doors.iter().any(|d| d.x == bx && d.y == by)
                        {
                            // Already has door
                        } else if self.doors.len() >= MAX_DOORS {
                            log::warn!("Max doors ({}) reached", MAX_DOORS);
                        } else {
                            let edge = wd_first_edge(wd);
                            let material = wd_material(wd) as u8;
                            // Hinge side: default 0 (left), R key flips
                            let hinge_side = (self.build_rotation & 1) as u8;
                            let door = Door::new(bx, by, edge, hinge_side, material);
                            self.doors.push(door);
                            // Set WD_HAS_DOOR in wall_data
                            if idx < self.wall_data.len() {
                                self.wall_data[idx] |= WD_HAS_DOOR;
                            }
                            self.grid_dirty = true;
                            log::info!(
                                "Placed door at ({}, {}) edge={} hinge={}",
                                bx,
                                by,
                                edge,
                                hinge_side
                            );
                            self.build_tool = BuildTool::None;
                        }
                    }
                }
                BuildTool::RemoveFloor => {
                    let block = self.grid_data[idx];
                    let bt_here = block_type_rs(block);
                    if bt_is!(
                        bt_here,
                        BT_WOOD_FLOOR,
                        BT_STONE_FLOOR,
                        BT_CONCRETE_FLOOR,
                        BT_ROUGH_FLOOR
                    ) {
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
                        compute_roof_heights_wd(&mut self.grid_data, &self.wall_data);
                    }
                }
                BuildTool::WoodBox => {
                    self.physics_bodies.push(PhysicsBody::new_wood_box(wx, wy));
                    // Don't deselect — can place multiple
                }
                BuildTool::Dig => {
                    // Legacy sandbox dig — now uses elevation system instead of block types
                    if bx >= 0 && by >= 0 && bx < GRID_W as i32 && by < GRID_H as i32 {
                        let bt_dig = block_type_rs(block);
                        if bt_dig == BT_GROUND || bt_dig == BT_DUG_GROUND {
                            // Lower sub-tile elevation directly (sandbox instant dig)
                            let wx = bx as f32 + 0.5;
                            let wy = by as f32 + 0.5;
                            let current =
                                crate::terrain::sample_elevation(&self.sub_elevation, wx, wy);
                            crate::terrain::apply_dig_stroke(
                                &mut self.sub_elevation,
                                wx,
                                wy,
                                0.15,                 // deeper per click than pleb strokes
                                |_, _| current - 2.0, // allow digging up to 2.0 deep
                            );
                            self.sub_elevation_dirty = true;
                            // Mark terrain as disturbed
                            if idx < self.terrain_data.len() {
                                self.terrain_data[idx] |= 0x3 << 13;
                                self.terrain_data[idx] &= !0x1F000000;
                                self.terrain_dirty = true;
                            }
                            // Clay terrain yields clay items when dug
                            if idx < self.terrain_data.len()
                                && terrain_type(self.terrain_data[idx]) == TERRAIN_CLAY
                            {
                                self.ground_items.push(resources::GroundItem::new(
                                    bx as f32 + 0.5,
                                    by as f32 + 0.5,
                                    item_defs::ITEM_CLAY,
                                    2,
                                ));
                            }
                        }
                    }
                }
                BuildTool::GrowingZone => {
                    if bt == BT_GROUND {
                        if let Some(zone) =
                            self.zones.iter_mut().find(|z| z.kind == ZoneKind::Growing)
                        {
                            zone.tiles.insert((bx, by));
                        } else {
                            let mut zone = Zone::new(ZoneKind::Growing);
                            zone.tiles.insert((bx, by));
                            self.zones.push(zone);
                        }
                    }
                }
                BuildTool::StorageZone => {
                    let bh = block_height_rs(block) as u32;
                    if bh == 0 {
                        // floor-level tiles only
                        if let Some(zone) =
                            self.zones.iter_mut().find(|z| z.kind == ZoneKind::Storage)
                        {
                            zone.tiles.insert((bx, by));
                        } else {
                            let mut zone = Zone::new(ZoneKind::Storage);
                            zone.tiles.insert((bx, by));
                            self.zones.push(zone);
                        }
                    }
                }
                BuildTool::DigZone => {
                    // Add to dig zone (any diggable terrain)
                    let bt_dig = block_type_rs(block);
                    if bt_dig == BT_GROUND || bt_dig == BT_DUG_GROUND {
                        let base_elev = crate::terrain::sample_elevation(
                            &self.sub_elevation,
                            bx as f32 + 0.5,
                            by as f32 + 0.5,
                        );
                        if let Some(dz) = self.dig_zones.first_mut() {
                            dz.tiles.insert((bx, by));
                            dz.base_elevations.entry((bx, by)).or_insert(base_elev);
                            dz.target_depth = self.dig_depth; // update depth from UI
                        } else {
                            let mut dz = zones::DigZone {
                                tiles: std::collections::HashSet::new(),
                                target_depth: self.dig_depth,
                                profile: crate::terrain::CrossProfile::VShape,
                                width: 0.0,
                                base_elevations: std::collections::HashMap::new(),
                            };
                            dz.tiles.insert((bx, by));
                            dz.base_elevations.insert((bx, by), base_elev);
                            self.dig_zones.push(dz);
                        }
                        // Also register as a regular zone for overlay rendering
                        if let Some(zone) = self.zones.iter_mut().find(|z| z.kind == ZoneKind::Dig)
                        {
                            zone.tiles.insert((bx, by));
                        } else {
                            let mut zone = Zone::new(ZoneKind::Dig);
                            zone.tiles.insert((bx, by));
                            self.zones.push(zone);
                        }
                    }
                }
                BuildTool::BermZone => {
                    // Add to berm zone (any ground tile)
                    let bt_berm = block_type_rs(block);
                    if bt_berm == BT_GROUND || bt_berm == BT_DUG_GROUND {
                        let base_elev = crate::terrain::sample_elevation(
                            &self.sub_elevation,
                            bx as f32 + 0.5,
                            by as f32 + 0.5,
                        );
                        if let Some(bz) = self.berm_zones.first_mut() {
                            bz.tiles.insert((bx, by));
                        } else {
                            let mut bz = zones::BermZone {
                                tiles: std::collections::HashSet::new(),
                                target_height: base_elev + 0.5, // raise 0.5 above current
                            };
                            bz.tiles.insert((bx, by));
                            self.berm_zones.push(bz);
                        }
                        if let Some(zone) = self.zones.iter_mut().find(|z| z.kind == ZoneKind::Berm)
                        {
                            zone.tiles.insert((bx, by));
                        } else {
                            let mut zone = Zone::new(ZoneKind::Berm);
                            zone.tiles.insert((bx, by));
                            self.zones.push(zone);
                        }
                    }
                }
                BuildTool::None | BuildTool::Destroy | BuildTool::Roof | BuildTool::WaterFill => {}
            }
        }
    }

    /// Handle click interactions with specific block types (toggles, popups).
    pub(crate) fn handle_block_click(
        &mut self,
        bx: i32,
        by: i32,
        idx: usize,
        block: u32,
        bt: u32,
        flags: u8,
    ) {
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

            log::info!(
                "Door at ({}, {}): {}",
                bx,
                by,
                if open { "opened" } else { "closed" }
            );
            return;
        }

        // Toggle valve open/closed
        if bt == BT_VALVE {
            let new_flags = flags ^ 4; // toggle bit2 (is_open)
            let new_block = (block & 0xFF00FFFF) | ((new_flags as u32) << 16);
            self.grid_data[idx] = new_block;
            self.grid_dirty = true;
            let open = (new_flags & 4) != 0;
            log::info!(
                "Valve at ({}, {}): {}",
                bx,
                by,
                if open { "open" } else { "closed" }
            );
            return;
        }

        // Toggle switch on/off
        if bt == BT_SWITCH && self.build_tool != BuildTool::Destroy {
            let new_flags = flags ^ 4; // toggle bit2
            let new_block = (block & 0xFF00FFFF) | ((new_flags as u32) << 16);
            self.grid_data[idx] = new_block;
            self.grid_dirty = true;
            let on = (new_flags & 4) != 0;
            log::info!(
                "Switch at ({}, {}): {}",
                bx,
                by,
                if on { "ON" } else { "OFF" }
            );
            return;
        }

        // Click dimmer, restrictor, or fireplace: show slider popup (shared UI)
        if (bt == BT_DIMMER || bt == BT_RESTRICTOR || bt == BT_FIREPLACE || bt == BT_CAMPFIRE)
            && self.build_tool != BuildTool::Destroy
        {
            let didx = by as u32 * GRID_W + bx as u32;
            self.block_sel.dimmer = if self.block_sel.dimmer == Some(didx) {
                None
            } else {
                Some(didx)
            };
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
            self.block_sel.fan = if self.block_sel.fan == Some(fidx) {
                None
            } else {
                Some(fidx)
            };
            self.block_sel.fan_world = (bx as f32 + 0.5, by as f32 + 0.5);
            return;
        }

        // Click pump: toggle pump speed popup
        if bt == BT_PUMP {
            let pidx = by as u32 * GRID_W + bx as u32;
            self.block_sel.pump = if self.block_sel.pump == Some(pidx) {
                None
            } else {
                Some(pidx)
            };
            self.block_sel.pump_world = (bx as f32 + 0.5, by as f32 + 0.5);
            return;
        }

        // Click crafting station (workbench or kiln): show recipe popup
        if (bt == BT_WORKBENCH || bt == BT_KILN || bt == BT_SAW_HORSE)
            && self.build_tool != BuildTool::Destroy
        {
            let widx = by as u32 * GRID_W + bx as u32;
            self.block_sel.workbench = if self.block_sel.workbench == Some(widx) {
                None
            } else {
                Some(widx)
            };
            self.block_sel.workbench_world = (bx as f32 + 0.5, by as f32 + 0.5);
        }

        // Removal is handled by the Destroy tool, not by clicking
    }
}

/// Compute tiles for a continuous diagonal wall.
/// Returns (x, y, variant) for each tile — main diagonal tiles + fill tiles
/// that tessellate into a solid wall strip.
/// `rotation` determines which side of the wall is solid (0-3).
pub(crate) fn compute_diagonal_wall_tiles(
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    rotation: u32,
) -> Vec<(i32, i32, u8)> {
    let dx = x1 - x0;
    let dy = y1 - y0;
    if dx == 0 && dy == 0 {
        return vec![(x0, y0, rotation as u8)];
    }
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
    } else if is_below {
        0 // / variants
    } else {
        2
    };
    let fill_var: u8 = main_var ^ 2; // flip side: 0↔2, 1↔3

    let mut tiles = Vec::with_capacity((steps as usize + 1) * 2);
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
                _ => sx > 0,     // left-facing: fill left when going right
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
