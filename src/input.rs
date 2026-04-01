//! Input handling: click, keyboard, notify/log helpers.

use crate::*;

impl App {
    /// Handle left-click: build tool placement, door toggle, or light toggle
    pub(crate) fn handle_click(&mut self, wx: f32, wy: f32) {
        let bx = wx.floor() as i32;
        let by = wy.floor() as i32;
        if bx < 0 || by < 0 || bx >= GRID_W as i32 || by >= GRID_H as i32 {
            return;
        }
        let idx = (by as u32 * GRID_W + bx as u32) as usize;
        let block = self.grid_data[idx];
        let bt = block_type_rs(block);
        let flags = block_flags_rs(block);

        // Sandbox tools
        if self.sandbox_tool == SandboxTool::Lightning {
            // Trigger lightning at clicked location
            self.lightning_flash = 1.0;
            self.lightning_strike = Some((wx, wy));
            self.lightning_surge_done = true; // surge injected immediately below
            // Heat/smoke at impact
            self.fluid_params.splat_x = wx;
            self.fluid_params.splat_y = wy;
            self.fluid_params.splat_vx = 0.0;
            self.fluid_params.splat_vy = 0.0;
            self.fluid_params.splat_radius = 1.5;
            self.fluid_params.splat_active = 1.0;
            self.lightning_surge(wx.floor() as i32, wy.floor() as i32);
            self.log_event(
                EventCategory::Weather,
                format!("Lightning strike at ({:.0}, {:.0})", wx, wy),
            );
            return;
        }
        if self.sandbox_tool == SandboxTool::InjectWater {
            // Inject water into the water texture at clicked position
            if let Some(gfx) = &self.gfx {
                let cx = bx;
                let cy = by;
                let radius = WATER_INJECT_RADIUS;
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let nx = cx + dx;
                        let ny = cy + dy;
                        if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 {
                            continue;
                        }
                        let dist = ((dx * dx + dy * dy) as f32).sqrt();
                        if dist > radius as f32 {
                            continue;
                        }
                        let strength = 1.0 - dist / (radius as f32 + 0.5);
                        let water_val = strength * 2.0;
                        let pixel = water_val.to_le_bytes();
                        // Write to both ping-pong textures
                        for ti in 0..2 {
                            gfx.queue.write_texture(
                                wgpu::TexelCopyTextureInfo {
                                    texture: &gfx.water_textures[ti],
                                    mip_level: 0,
                                    origin: wgpu::Origin3d {
                                        x: nx as u32,
                                        y: ny as u32,
                                        z: 0,
                                    },
                                    aspect: wgpu::TextureAspect::All,
                                },
                                &pixel,
                                wgpu::TexelCopyBufferLayout {
                                    offset: 0,
                                    bytes_per_row: Some(4),
                                    rows_per_image: Some(1),
                                },
                                wgpu::Extent3d {
                                    width: 1,
                                    height: 1,
                                    depth_or_array_layers: 1,
                                },
                            );
                        }
                        // Also raise CPU-side water table so crop growth sees it immediately
                        let widx = (ny as u32 * GRID_W + nx as u32) as usize;
                        if widx < self.water_table.len() {
                            self.water_table[widx] = self.water_table[widx].max(water_val * 0.5);
                        }
                    }
                }
                self.log_event(
                    EventCategory::General,
                    format!("Injected water at ({}, {})", cx, cy),
                );
            }
            return;
        }
        if self.sandbox_tool == SandboxTool::Ignite {
            // Ignite: set flammable block on fire by raising its temperature
            let idx = (by as u32 * GRID_W + bx as u32) as usize;
            let block = self.grid_data[idx];
            let bt = block_type_rs(block);
            let reg = block_defs::BlockRegistry::cached();
            if let Some(def) = reg.get(bt)
                && def.is_flammable
            {
                let ignite_temp = def.ignition_temp + 150.0 * self.fire_intensity;
                // Write high temperature to GPU block_temps buffer
                if let Some(gfx) = &self.gfx {
                    gfx.queue.write_buffer(
                        &gfx.block_temp_buffer,
                        (idx as u64) * 4,
                        bytemuck::bytes_of(&ignite_temp),
                    );
                }
                self.burn_progress.insert(idx, 0.0);
                self.log_event(
                    EventCategory::Weather,
                    format!("Fire! {} ignited at ({}, {})", def.name, bx, by),
                );
            }
            return;
        }
        if let SandboxTool::SoundPlace(idx) = self.sandbox_tool {
            if let Some(&(_name, db, freq, pattern, duration)) = SANDBOX_SOUNDS.get(idx) {
                self.sound_sources.push(SoundSource {
                    x: wx,
                    y: wy,
                    amplitude: db_to_amplitude(db),
                    frequency: freq,
                    phase: 0.0,
                    pattern,
                    duration,
                    fresh: true,
                });
            }
            return;
        }

        // Destroy tool: single click destroys one block
        if self.build_tool == BuildTool::Destroy {
            // Check for physics bodies first
            self.physics_bodies.retain(|b| {
                let dist = ((wx - b.x).powi(2) + (wy - b.y).powi(2)).sqrt();
                dist > 0.5 // keep if far from click
            });
            self.destroy_block_at(bx, by);
            return;
        }

        // Click cannon: select for rotation, or fire if already selected
        if bt == BT_CANNON && self.build_tool == BuildTool::None {
            let cannon_idx = by as u32 * GRID_W + bx as u32;
            if self.block_sel.cannon == Some(cannon_idx) {
                // Already selected — fire!
                let angle = *self.cannon_angles.get(&cannon_idx).unwrap_or(&0.0);
                let dir_x = angle.cos();
                let dir_y = angle.sin();
                let spawn_x = bx as f32 + 0.5 + dir_x * 0.8;
                let spawn_y = by as f32 + 0.5 + dir_y * 0.8;
                self.physics_bodies
                    .push(PhysicsBody::new_cannonball(spawn_x, spawn_y, dir_x, dir_y));
                // Muzzle smoke + recoil blast
                self.fluid_params.splat_x = bx as f32 + 0.5;
                self.fluid_params.splat_y = by as f32 + 0.5;
                self.fluid_params.splat_vx = -dir_x * 30.0;
                self.fluid_params.splat_vy = -dir_y * 30.0;
                self.fluid_params.splat_radius = 1.5;
                self.fluid_params.splat_active = 1.0;
                log::info!("Cannon fired at ({}, {})", bx, by);
            } else {
                // Select this cannon (deselect pleb)
                self.block_sel.cannon = Some(cannon_idx);
                self.selected_pleb = None;
                // Initialize angle from block direction bits if not yet set
                self.cannon_angles.entry(cannon_idx).or_insert_with(|| {
                    let dir_bits = (flags >> 3) & 3;
                    match dir_bits {
                        0 => -std::f32::consts::FRAC_PI_2, // north
                        1 => 0.0,                          // east
                        2 => std::f32::consts::FRAC_PI_2,  // south
                        _ => std::f32::consts::PI,         // west
                    }
                });
                log::info!("Selected cannon at ({}, {})", bx, by);
            }
            return;
        } else if self.block_sel.cannon.is_some() && bt != BT_CANNON {
            // Clicked away from cannon — deselect
            self.block_sel.cannon = None;
        }

        // Placing pleb mode
        if self.placing_pleb {
            if is_walkable_pos(&self.grid_data, wx, wy) && self.plebs.len() < MAX_PLEBS {
                let id = self.next_pleb_id;
                self.next_pleb_id += 1;
                let name = random_name(id as u32);
                let mut p = Pleb::new(id, name, wx, wy, id as u32 * 7919 + 42);
                p.headlight_mode = 2; // normal beam
                self.plebs.push(p);
                self.selected_pleb = Some(self.plebs.len() - 1);
                self.placing_pleb = false;
                self.show_pleb_help = self.plebs.len() == 1; // show help on first ever
            }
            return;
        }

        // Placing enemy mode
        if self.placing_enemy {
            if is_walkable_pos(&self.grid_data, wx, wy) && self.plebs.len() < MAX_PLEBS {
                let id = self.next_pleb_id;
                self.next_pleb_id += 1;
                let name = format!("Redskull #{}", id);
                let mut p = Pleb::new(id, name, wx, wy, id as u32 * 3571 + 99);
                p.is_enemy = true;
                // Redskull appearance: red tones
                p.appearance.shirt_r = 0.55;
                p.appearance.shirt_g = 0.12;
                p.appearance.shirt_b = 0.10;
                p.appearance.pants_r = 0.25;
                p.appearance.pants_g = 0.18;
                p.appearance.pants_b = 0.15;
                // Randomize enemy weapon: ~60% melee, ~40% ranged
                let wpn_roll = (id as u32).wrapping_mul(2654435761) % 10;
                if wpn_roll < 6 {
                    p.inventory.add(item_defs::ITEM_STONE_AXE, 1);
                    p.equipped_weapon = Some(item_defs::ITEM_STONE_AXE);
                    p.prefer_ranged = false;
                } else {
                    p.inventory.add(item_defs::ITEM_PISTOL, 1);
                    p.equipped_weapon = Some(item_defs::ITEM_PISTOL);
                    p.prefer_ranged = true;
                }
                // Assign to enemy group 255 (all placed enemies share a group)
                p.group_id = Some(255);
                self.plebs.push(p);
                self.placing_enemy = false;
            }
            return;
        }

        // Click on rock (no build tool): open context menu
        if bt == BT_ROCK && self.build_tool == BuildTool::None {
            self.open_context_menu(self.last_mouse_x as f32, self.last_mouse_y as f32, wx, wy);
            return;
        }

        // Click on storage crate: toggle inspection popup
        if bt == BT_CRATE && self.build_tool != BuildTool::Destroy {
            let cidx = by as u32 * GRID_W + bx as u32;
            self.block_sel.crate_idx = if self.block_sel.crate_idx == Some(cidx) {
                None
            } else {
                Some(cidx)
            };
            self.block_sel.crate_world = (bx as f32 + 0.5, by as f32 + 0.5);
            return;
        }

        // Pleb interaction (before build tools)
        if self.build_tool == BuildTool::None {
            // Check if clicking on any pleb
            let clicked_pleb = self
                .plebs
                .iter()
                .position(|p| ((wx - p.x).powi(2) + (wy - p.y).powi(2)).sqrt() < PLEB_CLICK_RADIUS);

            if let Some(idx) = clicked_pleb {
                // Double-click pleb: select all friendly plebs in viewport
                let is_pleb_double = self.frame_count - self.last_click_frame < DOUBLE_CLICK_FRAMES
                    && self.last_click_pos == (bx, by);
                if is_pleb_double && !self.plebs[idx].is_enemy {
                    // Select all non-enemy plebs visible in viewport
                    let half_w = self.camera.screen_w * 0.5 / self.camera.zoom;
                    let half_h = self.camera.screen_h * 0.5 / self.camera.zoom;
                    let vp_min_x = self.camera.center_x - half_w;
                    let vp_max_x = self.camera.center_x + half_w;
                    let vp_min_y = self.camera.center_y - half_h;
                    let vp_max_y = self.camera.center_y + half_h;
                    self.selected_group.clear();
                    for (pi, p) in self.plebs.iter().enumerate() {
                        if !p.is_dead
                            && !p.is_enemy
                            && p.x >= vp_min_x
                            && p.x <= vp_max_x
                            && p.y >= vp_min_y
                            && p.y <= vp_max_y
                        {
                            self.selected_group.push(pi);
                        }
                    }
                    self.selected_pleb = Some(idx);
                    self.last_click_frame = self.frame_count;
                    self.last_click_pos = (bx, by);
                    let p = &self.plebs[idx];
                    self.world_sel =
                        WorldSelection::single_pleb(idx, p.x.floor() as i32, p.y.floor() as i32);
                    self.context_menu = None;
                    return;
                }
                self.last_click_frame = self.frame_count;
                self.last_click_pos = (bx, by);

                let shift = self.pressed_keys.contains(&KeyCode::ShiftLeft)
                    || self.pressed_keys.contains(&KeyCode::ShiftRight);
                let ctrl = self.pressed_keys.contains(&KeyCode::ControlLeft)
                    || self.pressed_keys.contains(&KeyCode::ControlRight);
                if (shift || ctrl) && !self.plebs[idx].is_enemy {
                    // Shift/Ctrl+click: add/remove from multi-selection
                    if let Some(pos) = self.selected_group.iter().position(|&i| i == idx) {
                        self.selected_group.remove(pos);
                    } else {
                        self.selected_group.push(idx);
                    }
                    self.selected_pleb = Some(idx);
                } else {
                    // Normal click: select single pleb, clear group
                    self.selected_pleb = Some(idx);
                    self.selected_group.clear();
                }
                let p = &self.plebs[idx];
                self.world_sel =
                    WorldSelection::single_pleb(idx, p.x.floor() as i32, p.y.floor() as i32);
                self.block_sel = BlockSelection::default();
                self.context_menu = None;
                return;
            }

            // Double-click detection: interact on double-click, select on single
            let is_double = self.frame_count - self.last_click_frame < DOUBLE_CLICK_FRAMES
                && self.last_click_pos == (bx, by);
            self.last_click_frame = self.frame_count;
            self.last_click_pos = (bx, by);

            if is_double {
                // Double-click: interact with the block (toggle, slider, etc.)
                self.handle_block_click(bx, by, idx, block, bt, flags);
                return;
            }

            // Single-click on non-ground: select block (deselects pleb)
            let is_ground = is_ground_block(bt);
            let bp = self.blueprints.get(&(bx, by));
            if !is_ground || bp.is_some() {
                let (sel_x, sel_y, sel_w, sel_h, sel_bt) = if let Some(bp) = bp {
                    let bp_bt = bp.block_data & 0xFF;
                    (bx, by, 1, 1, bp_bt)
                } else {
                    let (sx, sy, sw, sh) = self.get_block_bounds(bx, by, bt, flags);
                    (sx, sy, sw, sh, bt)
                };
                self.world_sel = WorldSelection::single(sel_x, sel_y, sel_w, sel_h, sel_bt);
                self.selected_pleb = None;
                self.block_sel = BlockSelection::default();
                self.context_menu = None;
                return;
            }

            // Click on ground item: select it
            if let Some(gi) = self
                .ground_items
                .iter()
                .position(|item| item.x.floor() as i32 == bx && item.y.floor() as i32 == by)
            {
                let _gi_ref = &self.ground_items[gi]; // validates index
                self.world_sel = WorldSelection::single(bx, by, 1, 1, 0);
                self.context_menu = None;
                // If a pleb is selected, open context menu for hauling
                if self.selected_pleb.is_some() {
                    self.open_context_menu(
                        self.last_mouse_x as f32,
                        self.last_mouse_y as f32,
                        wx,
                        wy,
                    );
                }
                return;
            }

            // Click on empty ground: deselect everything
            self.world_sel = WorldSelection::none();
            self.selected_pleb = None;
            self.block_sel = BlockSelection::default();
            self.context_menu = None;
        }

        // Build tool placement (delegated to keep handle_click manageable)
        if self.build_tool != BuildTool::None {
            self.handle_build_placement(wx, wy, bx, by, idx, block, bt, flags);
        }
    }

    /// Handle build tool placement at grid position.
    /// Get the bounding box (origin + size) for a block, accounting for multi-tile items.
    /// Spawn sample enemies for the demo map.
    pub(crate) fn spawn_sample_enemies(&mut self) {
        let cx = (GRID_W / 2) as f32 + 0.5;
        let cy = (GRID_H / 2) as f32 + 0.5;

        let spawn_enemy =
            |plebs: &mut Vec<Pleb>, next_id: &mut usize, name: &str, x: f32, y: f32, seed: u32| {
                let mut e = Pleb::new(*next_id, name.to_string(), x, y, seed);
                e.is_enemy = true;
                e.wander_timer = 3.0;
                // Redskull: distinctive red skin, dark clothes
                e.appearance.skin_r = 0.8;
                e.appearance.skin_g = 0.15;
                e.appearance.skin_b = 0.1;
                e.appearance.shirt_r = 0.2;
                e.appearance.shirt_g = 0.1;
                e.appearance.shirt_b = 0.1;
                e.appearance.pants_r = 0.15;
                e.appearance.pants_g = 0.1;
                e.appearance.pants_b = 0.1;
                e.appearance.hair_r = 0.1;
                e.appearance.hair_g = 0.05;
                e.appearance.hair_b = 0.05;
                let wpn_roll = seed.wrapping_mul(2654435761) % 10;
                if wpn_roll < 6 {
                    e.inventory.add(item_defs::ITEM_STONE_AXE, 1);
                    e.equipped_weapon = Some(item_defs::ITEM_STONE_AXE);
                    e.prefer_ranged = false;
                } else {
                    e.inventory.add(item_defs::ITEM_PISTOL, 1);
                    e.equipped_weapon = Some(item_defs::ITEM_PISTOL);
                    e.prefer_ranged = true;
                }
                e.group_id = Some(255); // enemy group
                *next_id += 1;
                plebs.push(e);
            };

        // Spawn Redskulls around the base perimeter
        spawn_enemy(
            &mut self.plebs,
            &mut self.next_pleb_id,
            "Redskull",
            cx + 25.0,
            cy - 15.0,
            666,
        );
        spawn_enemy(
            &mut self.plebs,
            &mut self.next_pleb_id,
            "Redskull",
            cx - 20.0,
            cy + 18.0,
            667,
        );
        spawn_enemy(
            &mut self.plebs,
            &mut self.next_pleb_id,
            "Redskull",
            cx + 30.0,
            cy + 10.0,
            668,
        );
    }

    pub(crate) fn notify(
        &mut self,
        category: NotifCategory,
        icon: &'static str,
        title: impl Into<String>,
        desc: impl Into<String>,
    ) {
        self.next_notif_id += 1;
        self.notifications.push(GameNotification {
            id: self.next_notif_id,
            title: title.into(),
            description: desc.into(),
            category,
            icon,
            time_created: self.time_of_day,
            dismissed: false,
        });
    }

    pub(crate) fn add_condition(
        &mut self,
        name: impl Into<String>,
        icon: &'static str,
        category: NotifCategory,
        duration: f32,
    ) {
        let name = name.into();
        // Don't duplicate
        if self.conditions.iter().any(|c| c.name == name) {
            return;
        }
        self.next_notif_id += 1;
        self.conditions.push(ActiveCondition {
            id: self.next_notif_id,
            name,
            icon,
            category,
            remaining: duration,
            duration,
        });
    }

    pub(crate) fn has_condition(&self, name: &str) -> bool {
        self.conditions.iter().any(|c| c.name == name)
    }

    pub(crate) fn log_event(&mut self, category: EventCategory, message: impl Into<String>) {
        self.game_log.push_back(GameEvent {
            time: self.time_of_day,
            message: message.into(),
            category,
        });
        while self.game_log.len() > MAX_LOG_ENTRIES {
            self.game_log.pop_front();
        }
    }

    pub(crate) fn handle_keyboard(&mut self, event: &winit::event::KeyEvent) {
        if let PhysicalKey::Code(code) = event.physical_key {
            if event.state.is_pressed() {
                self.pressed_keys.insert(code);
            } else {
                self.pressed_keys.remove(&code);
            }
        }
        if event.state.is_pressed() {
            match event.physical_key {
                PhysicalKey::Code(KeyCode::Escape) => {
                    // Close whatever is open, in priority order
                    // When nothing is open: toggle pause menu
                    if self.attack_mode {
                        self.attack_mode = false;
                    } else if self.show_pause_menu {
                        self.show_pause_menu = false;
                        self.time_paused = false;
                    } else if self.context_menu.is_some() {
                        self.context_menu = None;
                    } else if self.show_inventory {
                        self.show_inventory = false;
                    } else if self.show_schedule {
                        self.show_schedule = false;
                    } else if self.show_priorities {
                        self.show_priorities = false;
                    } else if self.placing_pleb || self.placing_enemy {
                        self.placing_pleb = false;
                        self.placing_enemy = false;
                    } else if self.terrain_tool.is_some() {
                        self.terrain_tool = None;
                    } else if self.build_tool != BuildTool::None {
                        self.build_tool = BuildTool::None;
                    } else if self.build_category.is_some() {
                        self.build_category = None;
                        self.sandbox_tool = SandboxTool::None;
                    } else if self.selected_pleb.is_some() || !self.selected_group.is_empty() {
                        self.selected_pleb = None;
                        self.selected_group.clear();
                        self.world_sel = WorldSelection::none();
                    } else {
                        // Nothing open — show pause menu
                        self.show_pause_menu = true;
                        self.time_paused = true;
                    }
                }
                PhysicalKey::Code(KeyCode::KeyR) => {
                    let shift = self.pressed_keys.contains(&KeyCode::ShiftLeft)
                        || self.pressed_keys.contains(&KeyCode::ShiftRight);
                    if shift {
                        // Shift+R: toggle roof visibility
                        if self.camera.show_roofs < 0.5 {
                            self.camera.show_roofs = 1.0;
                        } else {
                            self.camera.show_roofs = 0.0;
                        }
                        if let Some(w) = self.window.as_ref() {
                            w.request_redraw();
                        }
                    } else if self.selected_pleb.is_some() {
                        // R: toggle melee/ranged preference
                        let indices: Vec<usize> = if !self.selected_group.is_empty() {
                            self.selected_group.clone()
                        } else if let Some(idx) = self.selected_pleb {
                            vec![idx]
                        } else {
                            vec![]
                        };
                        let new_state = indices
                            .first()
                            .and_then(|&i| self.plebs.get(i))
                            .map(|p| !p.prefer_ranged)
                            .unwrap_or(false);
                        for &i in &indices {
                            if let Some(p) = self.plebs.get_mut(i) {
                                if !p.is_enemy {
                                    p.prefer_ranged = new_state;
                                    p.update_equipped_weapon();
                                }
                            }
                        }
                    } else if self.wall_thickness > 1 {
                        // R: decrease wall thickness
                        self.wall_thickness -= 1;
                    }
                }
                PhysicalKey::Code(KeyCode::KeyD) => {
                    // D: toggle draft for all selected plebs (group or single)
                    let draft_indices: Vec<usize> = if !self.selected_group.is_empty() {
                        self.selected_group.clone()
                    } else if let Some(idx) = self.selected_pleb {
                        vec![idx]
                    } else {
                        vec![]
                    };
                    // Determine toggle direction from first pleb
                    let new_drafted = draft_indices
                        .first()
                        .and_then(|&i| self.plebs.get(i))
                        .map(|p| !p.drafted)
                        .unwrap_or(false);
                    for &idx in &draft_indices {
                        if let Some(pleb) = self.plebs.get_mut(idx) {
                            if pleb.is_enemy {
                                continue;
                            }
                            pleb.drafted = new_drafted;
                            pleb.update_equipped_weapon();
                            if !pleb.drafted {
                                // Returning to autonomous: clear manual targets
                                pleb.work_target = None;
                                pleb.haul_target = None;
                                pleb.harvest_target = None;
                            }
                        }
                    }
                }
                PhysicalKey::Code(KeyCode::Space) => {
                    // Space always toggles pause
                    if self.show_pause_menu {
                        self.show_pause_menu = false;
                        self.time_paused = false;
                    } else {
                        self.time_paused = !self.time_paused;
                    }
                }
                PhysicalKey::Code(KeyCode::KeyB) => {
                    if self.selected_pleb.is_some() {
                        // Start charging grenade
                        self.grenade_charging = true;
                        self.grenade_charge = 0.0;
                    }
                }
                PhysicalKey::Code(KeyCode::KeyX) => {
                    if self.selected_pleb.is_some() {
                        self.burst_mode = !self.burst_mode;
                        log::info!(
                            "Fire mode: {}",
                            if self.burst_mode { "BURST" } else { "SINGLE" }
                        );
                    }
                }
                PhysicalKey::Code(KeyCode::KeyI) => {
                    if self.selected_pleb.is_some() {
                        self.show_inventory = !self.show_inventory;
                        self.inv_selected_slot = None;
                    }
                }
                PhysicalKey::Code(KeyCode::KeyQ) => {
                    if self.selected_pleb.is_none() {
                        // During HollowRect drag: rotate entry side
                        let is_hollow_drag = self.drag_start.is_some()
                            && matches!(self.build_tool, BuildTool::Place(id) if {
                                let reg = block_defs::BlockRegistry::cached();
                                reg.get(id)
                                    .and_then(|d| d.placement.as_ref())
                                    .and_then(|p| p.drag.as_ref())
                                    == Some(&block_defs::DragShape::HollowRect)
                            });
                        if is_hollow_drag {
                            // Cycle: 0(auto) → 4(W) → 3(S) → 2(E) → 1(N) → 0
                            self.entry_side = if self.entry_side == 0 {
                                4
                            } else {
                                self.entry_side - 1
                            };
                        } else {
                            let four_way = matches!(
                                self.build_tool,
                                BuildTool::Place(12)
                                    | BuildTool::Place(16)
                                    | BuildTool::Place(20)
                                    | BuildTool::Place(19)
                                    | BuildTool::Place(44)
                            ) || (self.wall_thickness < 4
                                && matches!(self.build_tool, BuildTool::Place(id) if is_wall_block(id)));
                            if four_way {
                                self.build_rotation = (self.build_rotation + 3) % 4;
                            } else {
                                self.build_rotation = (self.build_rotation + 1) % 2;
                            }
                        }
                    }
                }
                PhysicalKey::Code(KeyCode::KeyE) => {
                    if self.selected_pleb.is_none() {
                        // During HollowRect drag: rotate entry side
                        let is_hollow_drag = self.drag_start.is_some()
                            && matches!(self.build_tool, BuildTool::Place(id) if {
                                let reg = block_defs::BlockRegistry::cached();
                                reg.get(id)
                                    .and_then(|d| d.placement.as_ref())
                                    .and_then(|p| p.drag.as_ref())
                                    == Some(&block_defs::DragShape::HollowRect)
                            });
                        if is_hollow_drag {
                            // Cycle: 0(auto) → 1(N) → 2(E) → 3(S) → 4(W) → 0
                            self.entry_side = (self.entry_side + 1) % 5;
                        } else {
                            let four_way = matches!(
                                self.build_tool,
                                BuildTool::Place(12)
                                    | BuildTool::Place(16)
                                    | BuildTool::Place(20)
                                    | BuildTool::Place(19)
                                    | BuildTool::Place(44)
                            ) || (self.wall_thickness < 4
                                && matches!(self.build_tool, BuildTool::Place(id) if is_wall_block(id)));
                            if four_way {
                                self.build_rotation = (self.build_rotation + 1) % 4;
                            } else {
                                self.build_rotation = (self.build_rotation + 1) % 2;
                            }
                        }
                    }
                }
                PhysicalKey::Code(KeyCode::KeyT) => {
                    if let Some(idx) = self.selected_pleb
                        && let Some(pleb) = self.plebs.get_mut(idx)
                    {
                        pleb.torch_on = !pleb.torch_on;
                    }
                }
                PhysicalKey::Code(KeyCode::KeyG) => {
                    let ctrl = self.pressed_keys.contains(&KeyCode::ControlLeft)
                        || self.pressed_keys.contains(&KeyCode::ControlRight);
                    let shift = self.pressed_keys.contains(&KeyCode::ShiftLeft)
                        || self.pressed_keys.contains(&KeyCode::ShiftRight);
                    if ctrl && shift {
                        // Ctrl+Shift+G: dissolve group of selected pleb
                        if let Some(idx) = self.selected_pleb {
                            if let Some(gid) = self.plebs[idx].group_id {
                                comms::dissolve_group(&mut self.plebs, gid);
                                self.selected_group.clear();
                            }
                        }
                    } else if ctrl && self.selected_group.len() >= 2 {
                        // Ctrl+G: form group from multi-selection
                        let indices = self.selected_group.clone();
                        let gid = comms::form_group(&mut self.plebs, &indices);
                        // Draft all group members
                        for &i in &indices {
                            if let Some(p) = self.plebs.get_mut(i) {
                                if !p.drafted {
                                    p.drafted = true;
                                    p.update_equipped_weapon();
                                }
                            }
                        }
                        // Show bubble on all members
                        for &i in &indices {
                            if let Some(p) = self.plebs.get_mut(i) {
                                p.set_bubble(pleb::BubbleKind::Text(format!("Group {}", gid)), 2.0);
                            }
                        }
                    } else if let Some(idx) = self.selected_pleb {
                        // G: toggle headlight on/off (no ctrl)
                        if let Some(pleb) = self.plebs.get_mut(idx) {
                            if pleb.headlight_mode > 0 {
                                pleb.headlight_mode = 0;
                            } else {
                                pleb.headlight_mode = 2; // default to normal
                            }
                        }
                    } else if shift {
                        self.show_subgrid = !self.show_subgrid;
                    } else {
                        self.show_grid = !self.show_grid;
                    }
                }
                PhysicalKey::Code(KeyCode::KeyC) => {
                    // C: toggle crouch for selected plebs
                    let indices: Vec<usize> = if !self.selected_group.is_empty() {
                        self.selected_group.clone()
                    } else if let Some(idx) = self.selected_pleb {
                        vec![idx]
                    } else {
                        vec![]
                    };
                    let new_state = indices
                        .first()
                        .and_then(|&i| self.plebs.get(i))
                        .map(|p| !p.crouching)
                        .unwrap_or(false);
                    for &i in &indices {
                        if let Some(p) = self.plebs.get_mut(i) {
                            if !p.is_enemy && !p.is_dead {
                                p.crouching = new_state;
                                p.peek_timer = 0.0;
                            }
                        }
                    }
                }
                PhysicalKey::Code(KeyCode::KeyF) => {
                    if let Some(pleb) = self.selected_pleb.and_then(|i| self.plebs.get(i)) {
                        // Pleb selected: throw nearest physics body
                        let px = pleb.x;
                        let py = pleb.y;
                        let angle = pleb.angle;
                        if let Some(idx) = nearest_body(&self.physics_bodies, px, py, 1.2) {
                            let dx = angle.cos();
                            let dy = angle.sin();
                            self.physics_bodies[idx].throw(dx, dy, 18.0);
                        }
                    } else if self.wall_thickness < 4 {
                        // F: increase wall thickness
                        self.wall_thickness += 1;
                    }
                }
                PhysicalKey::Code(KeyCode::Tab) => {
                    // Tab: cycle through friendly plebs
                    let friendlies: Vec<usize> = self
                        .plebs
                        .iter()
                        .enumerate()
                        .filter(|(_, p)| !p.is_enemy && !p.is_dead)
                        .map(|(i, _)| i)
                        .collect();
                    if !friendlies.is_empty() {
                        let cur = self.selected_pleb.unwrap_or(usize::MAX);
                        let next = friendlies
                            .iter()
                            .find(|&&i| i > cur)
                            .or(friendlies.first())
                            .copied()
                            .unwrap_or(0);
                        self.selected_pleb = Some(next);
                        self.selected_group.clear();
                    }
                }
                PhysicalKey::Code(KeyCode::KeyS) => {
                    // S: hold position (cancel path for selected plebs)
                    let indices: Vec<usize> = if !self.selected_group.is_empty() {
                        self.selected_group.clone()
                    } else if let Some(idx) = self.selected_pleb {
                        vec![idx]
                    } else {
                        vec![]
                    };
                    for &i in &indices {
                        if let Some(pleb) = self.plebs.get_mut(i) {
                            if !pleb.is_enemy && !pleb.is_dead {
                                pleb.path.clear();
                                pleb.path_idx = 0;
                                if pleb.activity == PlebActivity::Walking {
                                    pleb.activity = PlebActivity::Idle;
                                }
                            }
                        }
                    }
                }
                PhysicalKey::Code(KeyCode::KeyA) => {
                    // A: toggle attack mode (when pleb selected)
                    if self.selected_pleb.is_some() {
                        self.attack_mode = !self.attack_mode;
                    }
                }
                PhysicalKey::Code(
                    code @ (KeyCode::Digit1
                    | KeyCode::Digit2
                    | KeyCode::Digit3
                    | KeyCode::Digit4
                    | KeyCode::Digit5
                    | KeyCode::Digit6
                    | KeyCode::Digit7
                    | KeyCode::Digit8
                    | KeyCode::Digit9),
                ) => {
                    let ctrl = self.pressed_keys.contains(&KeyCode::ControlLeft)
                        || self.pressed_keys.contains(&KeyCode::ControlRight);
                    let group_num = match code {
                        KeyCode::Digit1 => 1u8,
                        KeyCode::Digit2 => 2,
                        KeyCode::Digit3 => 3,
                        KeyCode::Digit4 => 4,
                        KeyCode::Digit5 => 5,
                        KeyCode::Digit6 => 6,
                        KeyCode::Digit7 => 7,
                        KeyCode::Digit8 => 8,
                        KeyCode::Digit9 => 9,
                        _ => 0,
                    };
                    if ctrl {
                        // Ctrl+N: assign selected plebs to numbered group N
                        let indices: Vec<usize> = if !self.selected_group.is_empty() {
                            self.selected_group.clone()
                        } else if let Some(idx) = self.selected_pleb {
                            vec![idx]
                        } else {
                            vec![]
                        };
                        if !indices.is_empty() {
                            // Clear old members of this group
                            for p in self.plebs.iter_mut() {
                                if p.group_id == Some(group_num) {
                                    p.group_id = None;
                                }
                            }
                            // Assign new members
                            for &i in &indices {
                                if let Some(p) = self.plebs.get_mut(i) {
                                    p.group_id = Some(group_num);
                                }
                            }
                        }
                    } else {
                        // N: select all plebs in numbered group N
                        let group: Vec<usize> = self
                            .plebs
                            .iter()
                            .enumerate()
                            .filter(|(_, p)| p.group_id == Some(group_num) && !p.is_dead)
                            .map(|(i, _)| i)
                            .collect();
                        if !group.is_empty() {
                            self.selected_pleb = Some(group[0]);
                            self.selected_group = group;
                        }
                    }
                }
                _ => {}
            }
            // Logical key matching for +/- (varies by keyboard layout)
            if let winit::keyboard::Key::Character(ch) = &event.logical_key {
                match ch.as_str() {
                    "-" => {
                        if self.wall_thickness > 1 {
                            self.wall_thickness -= 1;
                        }
                    }
                    "+" | "=" => {
                        if self.wall_thickness < 4 {
                            self.wall_thickness += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
        // Key release: throw grenade on B release
        if !event.state.is_pressed()
            && let PhysicalKey::Code(KeyCode::KeyB) = event.physical_key
            && self.grenade_charging
        {
            self.grenade_charging = false;
            if let Some(pleb) = self.selected_pleb.and_then(|i| self.plebs.get(i)) {
                let dx = pleb.angle.cos();
                let dy = pleb.angle.sin();
                let spawn_x = pleb.x + dx * 0.5;
                let spawn_y = pleb.y + dy * 0.5;
                let power = self.grenade_charge.clamp(0.0, 1.0);
                self.physics_bodies
                    .push(PhysicsBody::new_grenade(spawn_x, spawn_y, dx, dy, power));
            }
            self.grenade_charge = 0.0;
        }
    }
}
