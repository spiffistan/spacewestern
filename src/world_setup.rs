//! World setup — crash site generation, landing site selection, world preview.
//! Separated from ui.rs because this is game logic, not UI drawing.

use crate::*;

impl App {
    /// Regenerate elevation, water table, and terrain from current params.
    pub(crate) fn regenerate_world_preview(&mut self) {
        let seed = self.terrain_params.seed;
        self.grid_data = grid::generate_world(seed, self.terrain_params.tree_density);
        self.elevation_data =
            grid::generate_elevation_seeded(&self.grid_data, seed, self.terrain_params.hilliness);
        self.sub_elevation = crate::terrain::generate_elevation(&self.elevation_data);
        self.water_table = grid::generate_water_table_seeded(
            &self.grid_data,
            seed,
            self.terrain_params.water_table,
        );
        grid::adjust_water_table_for_elevation(&mut self.water_table, &self.elevation_data);
        self.water_equilibrium =
            grid::compute_equilibrium_water(&self.elevation_data, &self.water_table);
        grid::apply_wetland_buffer(
            &mut self.grid_data,
            &self.water_equilibrium,
            &self.water_table,
            &self.elevation_data,
            seed,
        );
        self.terrain_data = grid::generate_terrain_with_params(
            &self.elevation_data,
            &self.water_table,
            &self.terrain_params,
        );
        self.terrain_dirty = true;
        self.grid_dirty = true;
    }

    /// Find a dry landing position near map center. Spirals outward until a spot is found
    /// where at least 80% of tiles within `buffer` radius are dry ground (no water, no stone).
    pub(crate) fn find_landing_site(&self, buffer: i32) -> (i32, i32) {
        use grid::*;
        let center_x = (GRID_W / 2) as i32;
        let center_y = (GRID_H / 2) as i32;

        let is_dry = |x: i32, y: i32| -> bool {
            if x < 0 || y < 0 || x >= GRID_W as i32 || y >= GRID_H as i32 {
                return false;
            }
            let idx = (y as u32 * GRID_W + x as u32) as usize;
            let bt = block_type_rs(self.grid_data[idx]);
            let has_water = idx < self.water_equilibrium.len() && self.water_equilibrium[idx] > 0.2;
            bt != BT_WATER && bt != BT_STONE && !has_water
        };

        let score = |cx: i32, cy: i32| -> f32 {
            let mut dry = 0u32;
            let mut total = 0u32;
            for dy in -buffer..=buffer {
                for dx in -buffer..=buffer {
                    if dx * dx + dy * dy <= buffer * buffer {
                        total += 1;
                        if is_dry(cx + dx, cy + dy) {
                            dry += 1;
                        }
                    }
                }
            }
            if total == 0 {
                0.0
            } else {
                dry as f32 / total as f32
            }
        };

        if score(center_x, center_y) >= 0.85 {
            return (center_x, center_y);
        }

        for radius in 1i32..100 {
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx.abs() != radius && dy.abs() != radius {
                        continue;
                    }
                    let x = center_x + dx;
                    let y = center_y + dy;
                    if score(x, y) >= 0.85 {
                        return (x, y);
                    }
                }
            }
        }
        (center_x, center_y)
    }

    /// Place the crash site: scattered salvage crates, campfire, reposition plebs.
    pub(crate) fn build_landing_pod(&mut self) {
        use grid::*;
        let (cx, cy) = self.find_landing_site(15);

        let mut rng_state: u32 = self.manifest.seed.wrapping_mul(2654435761).wrapping_add(42);
        let mut rng = || -> u32 {
            rng_state ^= rng_state << 13;
            rng_state ^= rng_state >> 17;
            rng_state ^= rng_state << 5;
            rng_state
        };

        // Clear a wider area around crash site (remove trees etc)
        let clear_radius = 12;
        let ground = make_block(BT_GROUND as u8, 0, 0);
        for dy in -clear_radius..=clear_radius {
            for dx in -clear_radius..=clear_radius {
                let dist_sq = dx * dx + dy * dy;
                if dist_sq <= clear_radius * clear_radius {
                    let x = cx + dx;
                    let y = cy + dy;
                    if x >= 0 && y >= 0 && x < GRID_W as i32 && y < GRID_H as i32 {
                        let idx = (y as u32 * GRID_W + x as u32) as usize;
                        if idx < self.grid_data.len() {
                            let bt = block_type_rs(self.grid_data[idx]);
                            if bt == BT_TREE
                                || bt == BT_DUSTWHISKER
                                || bt == BT_THORNBRAKE
                                || bt == BT_HOLLOW_REED
                                || bt == BT_SALTBRUSH
                                || bt == BT_DUSKBLOOM
                            {
                                self.grid_data[idx] = ground | (self.grid_data[idx] & 0xFF000000);
                            }
                        }
                    }
                }
            }
        }

        // Salvage crate positions: scattered in a ring around center
        let crate_count = 4 + (rng() % 3) as i32;
        let mut crate_positions = Vec::with_capacity(crate_count as usize);
        for _ in 0..crate_count {
            let angle = (rng() % 360) as f32 * std::f32::consts::PI / 180.0;
            let dist = 3.0 + (rng() % 70) as f32 / 10.0;
            let x = cx + (angle.cos() * dist) as i32;
            let y = cy + (angle.sin() * dist) as i32;
            if x >= 0 && y >= 0 && x < GRID_W as i32 && y < GRID_H as i32 {
                let idx = (y as u32 * GRID_W + x as u32) as usize;
                if idx < self.grid_data.len() {
                    let bt = block_type_rs(self.grid_data[idx]);
                    if bt == BT_AIR || bt == BT_GROUND {
                        let rotation = (rng() % 4) as u8;
                        let smoking = if rng() % 4 == 0 { 1u8 } else { 0u8 };
                        self.grid_data[idx] = make_block(BT_SALVAGE_CRATE as u8, rotation, smoking)
                            | (self.grid_data[idx] & 0xFF000000);
                        crate_positions.push((x, y));
                        if smoking == 1 {
                            self.smoking_crates.push((x as f32 + 0.5, y as f32 + 0.5));
                        }
                    }
                }
            }
        }

        // Salvage crate content tables
        let supply_tables: &[&[(u16, u16)]] = &[
            &[(item_defs::ITEM_BERRIES, 5), (item_defs::ITEM_FIBER, 3)],
            &[(item_defs::ITEM_ROCK, 4), (item_defs::ITEM_SCRAP_WOOD, 6)],
            &[(item_defs::ITEM_ROPE, 2), (item_defs::ITEM_FIBER, 4)],
            &[
                (item_defs::ITEM_BERRIES, 3),
                (item_defs::ITEM_SCRAP_WOOD, 4),
                (item_defs::ITEM_ROCK, 3),
            ],
            &[(item_defs::ITEM_KNIFE, 1), (item_defs::ITEM_FIBER, 2)],
            &[(item_defs::ITEM_BERRIES, 8), (item_defs::ITEM_SALT, 2)],
        ];

        for (i, &(x, y)) in crate_positions.iter().enumerate() {
            let crate_key = y as u32 * GRID_W + x as u32;
            let inv = self.crate_contents.entry(crate_key).or_default();
            let table = supply_tables[i % supply_tables.len()];
            for &(item_id, count) in table {
                inv.add(item_id, count);
            }
        }

        // Campfire at center (with fuel)
        {
            let fidx = (cy as u32 * GRID_W + cx as u32) as usize;
            if fidx < self.grid_data.len() {
                self.grid_data[fidx] =
                    make_block(BT_CAMPFIRE as u8, 5, 0) | (self.grid_data[fidx] & 0xFF000000);
            }
        }

        // Reposition plebs around the landing site
        let fcx = cx as f32 + 0.5;
        let fcy = cy as f32 + 2.5;
        for (i, pleb) in self.plebs.iter_mut().enumerate() {
            pleb.x = fcx + (i as f32 - 1.0);
            pleb.y = fcy;
        }

        // Center camera on landing site
        self.camera.center_x = fcx;
        self.camera.center_y = fcy;

        self.grid_dirty = true;
    }
}
