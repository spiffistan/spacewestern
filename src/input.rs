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
            self.log_event(EventCategory::Weather, format!("Lightning strike at ({:.0}, {:.0})", wx, wy));
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
                        if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 { continue; }
                        let dist = ((dx * dx + dy * dy) as f32).sqrt();
                        if dist > radius as f32 { continue; }
                        let strength = 1.0 - dist / (radius as f32 + 0.5);
                        let water_val = (strength * 2.0) as f32;
                        let pixel = water_val.to_le_bytes();
                        // Write to both ping-pong textures
                        for ti in 0..2 {
                            gfx.queue.write_texture(
                                wgpu::TexelCopyTextureInfo {
                                    texture: &gfx.water_textures[ti],
                                    mip_level: 0,
                                    origin: wgpu::Origin3d { x: nx as u32, y: ny as u32, z: 0 },
                                    aspect: wgpu::TextureAspect::All,
                                },
                                &pixel,
                                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4), rows_per_image: Some(1) },
                                wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                            );
                        }
                        // Also raise CPU-side water table so crop growth sees it immediately
                        let widx = (ny as u32 * GRID_W + nx as u32) as usize;
                        if widx < self.water_table.len() {
                            self.water_table[widx] = self.water_table[widx].max(water_val * 0.5);
                        }
                    }
                }
                self.log_event(EventCategory::General, format!("Injected water at ({}, {})", cx, cy));
            }
            return;
        }
        if self.sandbox_tool == SandboxTool::Ignite {
            // Ignite: set flammable block on fire by raising its temperature
            let idx = (by as u32 * GRID_W + bx as u32) as usize;
            let block = self.grid_data[idx];
            let bt = block_type_rs(block);
            let reg = block_defs::BlockRegistry::cached();
            if let Some(def) = reg.get(bt) {
                if def.is_flammable {
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
                    self.log_event(EventCategory::Weather, format!("Fire! {} ignited at ({}, {})", def.name, bx, by));
                }
            }
            return;
        }
        if let SandboxTool::SoundPlace(idx) = self.sandbox_tool {
            if let Some(&(_name, db, freq, pattern, duration)) = SANDBOX_SOUNDS.get(idx) {
                self.sound_sources.push(SoundSource {
                    x: wx, y: wy,
                    amplitude: db_to_amplitude(db),
                    frequency: freq,
                    phase: 0.0,
                    pattern,
                    duration,
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
                self.physics_bodies.push(PhysicsBody::new_cannonball(spawn_x, spawn_y, dir_x, dir_y));
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
                if !self.cannon_angles.contains_key(&cannon_idx) {
                    let dir_bits = (flags >> 3) & 3;
                    let angle = match dir_bits {
                        0 => -std::f32::consts::FRAC_PI_2, // north
                        1 => 0.0,                           // east
                        2 => std::f32::consts::FRAC_PI_2,  // south
                        _ => std::f32::consts::PI,          // west
                    };
                    self.cannon_angles.insert(cannon_idx, angle);
                }
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
                p.headlight_on = true;
                self.plebs.push(p);
                self.selected_pleb = Some(self.plebs.len() - 1);
                self.placing_pleb = false;
                self.show_pleb_help = self.plebs.len() == 1; // show help on first ever
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
            self.block_sel.crate_idx = if self.block_sel.crate_idx == Some(cidx) { None } else { Some(cidx) };
            self.block_sel.crate_world = (bx as f32 + 0.5, by as f32 + 0.5);
            return;
        }

        // Pleb interaction (before build tools)
        if self.build_tool == BuildTool::None {
            // Check if clicking on any pleb
            let mut clicked_pleb = None;
            for (i, p) in self.plebs.iter().enumerate() {
                if ((wx - p.x).powi(2) + (wy - p.y).powi(2)).sqrt() < PLEB_CLICK_RADIUS {
                    clicked_pleb = Some(i);
                    break;
                }
            }

            if let Some(idx) = clicked_pleb {
                // Clicking a pleb selects it (deselects everything else)
                self.selected_pleb = Some(idx);
                let p = &self.plebs[idx];
                self.world_sel = WorldSelection::single_pleb(idx, p.x.floor() as i32, p.y.floor() as i32);
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
            let has_bp = self.blueprints.contains_key(&(bx, by));
            if !is_ground || has_bp {
                let (sel_x, sel_y, sel_w, sel_h, sel_bt) = if has_bp {
                    let bp_bt = (self.blueprints[&(bx, by)].block_data & 0xFF) as u32;
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
            if let Some(gi) = self.ground_items.iter().position(|item| {
                item.x.floor() as i32 == bx && item.y.floor() as i32 == by
            }) {
                let _gi_ref = &self.ground_items[gi]; // validates index
                self.world_sel = WorldSelection::single(bx, by, 1, 1, 0);
                self.context_menu = None;
                // If a pleb is selected, open context menu for hauling
                if self.selected_pleb.is_some() {
                    self.open_context_menu(self.last_mouse_x as f32, self.last_mouse_y as f32, wx, wy);
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
            return;
        }
    }

    /// Handle build tool placement at grid position.
    /// Get the bounding box (origin + size) for a block, accounting for multi-tile items.
    pub(crate) fn notify(&mut self, category: NotifCategory, icon: &'static str, title: impl Into<String>, desc: impl Into<String>) {
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

    pub(crate) fn add_condition(&mut self, name: impl Into<String>, icon: &'static str, category: NotifCategory, duration: f32) {
        let name = name.into();
        // Don't duplicate
        if self.conditions.iter().any(|c| c.name == name) { return; }
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
                    self.placing_pleb = false;
                    if self.debug.mode {
                        self.debug.mode = false;
                    } else if self.selected_pleb.is_some() {
                        self.selected_pleb = None;
                    } else if self.build_tool != BuildTool::None {
                        self.build_tool = BuildTool::None;
                    }
                }
                PhysicalKey::Code(KeyCode::KeyR) => {
                    if self.camera.show_roofs < 0.5 {
                        self.camera.show_roofs = 1.0;
                    } else {
                        self.camera.show_roofs = 0.0;
                    }
                    self.window.as_ref().unwrap().request_redraw();
                }
                PhysicalKey::Code(KeyCode::Space) => {
                    if self.selected_pleb.is_some() && self.burst_queue == 0 {
                        if self.burst_mode {
                            self.burst_queue = BURST_SHOT_COUNT;
                            self.burst_delay = 0.0; // fire first shot immediately
                        } else {
                            self.burst_queue = 1;
                            self.burst_delay = 0.0;
                        }
                    } else if self.selected_pleb.is_none() {
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
                        log::info!("Fire mode: {}", if self.burst_mode { "BURST" } else { "SINGLE" });
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
                        if matches!(self.build_tool, BuildTool::Place(12) | BuildTool::Place(16) | BuildTool::Place(20) | BuildTool::Place(19) | BuildTool::Place(44)) {
                            self.build_rotation = (self.build_rotation + 3) % 4;
                        } else {
                            self.build_rotation = (self.build_rotation + 1) % 2;
                        }
                    }
                }
                PhysicalKey::Code(KeyCode::KeyE) => {
                    if self.selected_pleb.is_none() {
                        if matches!(self.build_tool, BuildTool::Place(12) | BuildTool::Place(16) | BuildTool::Place(20) | BuildTool::Place(19) | BuildTool::Place(44)) {
                            self.build_rotation = (self.build_rotation + 1) % 4;
                        } else {
                            self.build_rotation = (self.build_rotation + 1) % 2;
                        }
                    }
                }
                PhysicalKey::Code(KeyCode::KeyT) => {
                    if let Some(idx) = self.selected_pleb {
                        if let Some(pleb) = self.plebs.get_mut(idx) {
                            pleb.torch_on = !pleb.torch_on;
                        }
                    }
                }
                PhysicalKey::Code(KeyCode::KeyG) => {
                    if let Some(idx) = self.selected_pleb {
                        if let Some(pleb) = self.plebs.get_mut(idx) {
                            pleb.headlight_on = !pleb.headlight_on;
                        }
                    }
                }
                PhysicalKey::Code(KeyCode::KeyF) => {
                    if let Some(pleb) = self.selected_pleb.and_then(|i| self.plebs.get(i)) {
                        let px = pleb.x;
                        let py = pleb.y;
                        let angle = pleb.angle;
                        if let Some(idx) = nearest_body(&self.physics_bodies, px, py, 1.2) {
                            let dx = angle.cos();
                            let dy = angle.sin();
                            self.physics_bodies[idx].throw(dx, dy, 18.0);
                        }
                    }
                }
                _ => {}
            }
        }
        // Key release: throw grenade on B release
        if !event.state.is_pressed() {
            if let PhysicalKey::Code(KeyCode::KeyB) = event.physical_key {
                if self.grenade_charging {
                    self.grenade_charging = false;
                    if let Some(pleb) = self.selected_pleb.and_then(|i| self.plebs.get(i)) {
                        let dx = pleb.angle.cos();
                        let dy = pleb.angle.sin();
                        let spawn_x = pleb.x + dx * 0.5;
                        let spawn_y = pleb.y + dy * 0.5;
                        let power = self.grenade_charge.clamp(0.0, 1.0);
                        self.physics_bodies.push(PhysicsBody::new_grenade(
                            spawn_x, spawn_y, dx, dy, power,
                        ));
                    }
                    self.grenade_charge = 0.0;
                }
            }
        }
    }

}
