//! Simulation update — time, sun, plebs, physics, pipes, doors.
//! Extracted from render() to keep main.rs manageable.

use crate::*;
use crate::grid::*;
use crate::zones::*;

impl App {
    /// Update all simulation state. Returns frame delta time.
    pub(crate) fn update_simulation(&mut self) -> f32 {
        let mut events: Vec<(EventCategory, String)> = Vec::new();

        // Advance time + FPS tracking
        let now = Instant::now();
        let dt = now.elapsed_secs_since(&self.last_frame_time);
        self.last_frame_time = now;

        self.frame_count += 1;
        self.fps_accum += dt;
        if self.fps_accum >= 0.5 {
            self.fps_display = self.frame_count as f32 / self.fps_accum;
            self.frame_count = 0;
            self.fps_accum = 0.0;
        }

        // --- Tick active conditions + event triggers ---
        if !self.time_paused {
            let dt_game = dt * self.time_speed;
            let mut ended = Vec::new();
            for cond in self.conditions.iter_mut() {
                if cond.remaining > 0.0 {
                    cond.remaining -= dt_game;
                    if cond.remaining <= 0.0 { ended.push(cond.name.clone()); }
                }
            }
            for name in &ended {
                events.push((EventCategory::Weather, format!("{} has ended", name)));
                self.notify(NotifCategory::Positive, "\u{2705}", &format!("{} ended", name), "Conditions returning to normal.");
            }
            self.conditions.retain(|c| c.remaining > 0.0 || c.duration == 0.0);

            self.drought_check_timer -= dt_game;
            if self.drought_check_timer <= 0.0 {
                self.drought_check_timer = 60.0 + (self.time_of_day * 137.0) as f32 % 60.0;
                let seed = (self.time_of_day * 10000.0) as u32;
                let roll = seed.wrapping_mul(2654435761) & 0xFF;
                if roll < 25 && self.weather == WeatherState::Clear && !self.has_condition("Drought") {
                    let duration = 60.0 + (roll as f32) * 1.5;
                    self.add_condition("Drought", "\u{2600}", NotifCategory::Threat, duration);
                    self.notify(NotifCategory::Threat, "\u{2600}", "Drought",
                        format!("A drought has begun! Water drying up. ({:.0}s)", duration));
                    events.push((EventCategory::Weather, "Drought has begun!".to_string()));
                }
            }
        }
        let is_drought = self.has_condition("Drought");

        if !self.time_paused {
            self.time_of_day += dt * self.time_speed;
            // Wrap around
            while self.time_of_day >= DAY_DURATION {
                self.time_of_day -= DAY_DURATION;
            }
            while self.time_of_day < 0.0 {
                self.time_of_day += DAY_DURATION;
            }
        }

        // Save previous camera state for temporal reprojection
        // Set prev camera from LAST frame's values (not current — otherwise delta is always 0)
        self.camera.prev_center_x = self.prev_cam_x;
        self.camera.prev_center_y = self.prev_cam_y;
        self.camera.prev_zoom = self.prev_cam_zoom;
        self.camera.prev_time = self.prev_cam_time;

        self.camera.time = self.time_of_day;

        // Precompute sun on CPU (avoids trig per pixel in shader)
        {
            let t = (self.time_of_day / DAY_DURATION).rem_euclid(1.0);
            let dawn = 0.15f32;
            let dusk = 0.85f32;
            let day_t = ((t - dawn) / (dusk - dawn)).clamp(0.0, 1.0);
            let angle = day_t * std::f32::consts::PI;
            self.camera.sun_dir_x = -angle.cos();
            self.camera.sun_dir_y = -angle.sin() * 0.6 - 0.2;
            let noon = (day_t * std::f32::consts::PI).sin();
            let edge = smoothstep_f32(0.0, 0.15, day_t) * smoothstep_f32(1.0, 0.85, day_t);
            self.camera.sun_elevation = (1.0 + 3.0 * noon) * edge;
            let fade_in = smoothstep_f32(dawn - 0.05, dawn + 0.05, t);
            let fade_out = smoothstep_f32(dusk + 0.05, dusk - 0.05, t);
            let intensity = fade_in * fade_out;
            self.camera.sun_intensity = intensity;
            let dawn_col = [1.0f32, 0.55, 0.25];
            let noon_col = [1.0f32, 0.97, 0.90];
            let s = smoothstep_f32(0.0, 0.6, noon);
            self.camera.sun_color_r = (dawn_col[0] + (noon_col[0] - dawn_col[0]) * s) * intensity;
            self.camera.sun_color_g = (dawn_col[1] + (noon_col[1] - dawn_col[1]) * s) * intensity;
            self.camera.sun_color_b = (dawn_col[2] + (noon_col[2] - dawn_col[2]) * s) * intensity;
            let night_amb = [0.008f32, 0.008, 0.02];
            let day_amb = [0.10f32, 0.10, 0.13];
            self.camera.ambient_r = night_amb[0] + (day_amb[0] - night_amb[0]) * intensity;
            self.camera.ambient_g = night_amb[1] + (day_amb[1] - night_amb[1]) * intensity;
            self.camera.ambient_b = night_amb[2] + (day_amb[2] - night_amb[2]) * intensity;
        }

        // --- Weather tick ---
        if !self.time_paused {
            if let Some(new_weather) = tick_weather(&self.weather, &mut self.weather_timer, dt, self.time_speed) {
                let label = match &new_weather {
                    WeatherState::Clear => "Weather: Clear skies",
                    WeatherState::Cloudy => "Weather: Cloudy",
                    WeatherState::LightRain => "Weather: Light rain",
                    WeatherState::HeavyRain => "Weather: Heavy rain",
                };
                events.push((EventCategory::Weather, label.to_string()));
                self.weather = new_weather;
            }
            // --- Lightning during heavy rain ---
            self.lightning_flash = (self.lightning_flash - dt * 2.0).max(0.0); // slower decay for visible bolt
            if self.lightning_flash < 0.01 { self.lightning_strike = None; }
            if self.weather == WeatherState::HeavyRain {
                self.lightning_timer -= dt * self.time_speed;
                if self.lightning_timer <= 0.0 {
                    // Random strike location (outdoor, no roof)
                    let seed = (self.time_of_day * 10000.0) as u32;
                    let hash = |i: u32| -> u32 {
                        seed.wrapping_mul(2654435761).wrapping_add(i.wrapping_mul(1013904223))
                    };
                    let sx = (hash(0) % GRID_W) as i32;
                    let sy = (hash(1) % GRID_H) as i32;
                    let idx = (sy as u32 * GRID_W + sx as u32) as usize;
                    let block = self.grid_data[idx];
                    let has_roof = ((block >> 16) & 2) != 0;

                    if !has_roof {
                        // Lightning strike!
                        self.lightning_flash = 1.0;
                        self.lightning_strike = Some((sx as f32 + 0.5, sy as f32 + 0.5));
                        self.lightning_surge_done = false;

                        // Inject heat at strike point
                        self.fluid_params.splat_x = sx as f32 + 0.5;
                        self.fluid_params.splat_y = sy as f32 + 0.5;
                        self.fluid_params.splat_vx = 0.0;
                        self.fluid_params.splat_vy = 0.0;
                        self.fluid_params.splat_radius = 1.5;
                        self.fluid_params.splat_active = 1.0;

                        // Voltage surge: if strike hits a wire/conductor, inject massive voltage
                        let bt = block_type_rs(block);
                        let flags = block_flags_rs(block);
                        if is_conductor_rs(bt, flags) {
                            log::info!("Lightning hit power grid at ({}, {})! Voltage surge!", sx, sy);
                        }
                        // Voltage surge injection + breaker tripping happens in render pass
                        // via GPU voltage buffer writes + GPU-side breaker threshold check

                        events.push((EventCategory::Weather, format!("Lightning strike at ({}, {})", sx, sy)));
                    }

                    // Next strike in 5-15 game seconds
                    self.lightning_timer = 5.0 + (hash(2) & 0xFF) as f32 / 255.0 * 10.0;
                }
            } else {
                self.lightning_timer = 5.0; // reset when not heavy rain
                self.lightning_strike = None;
            }

            // --- Wind variation: slowly drift direction and magnitude ---
            self.wind_change_timer -= dt * self.time_speed;
            if self.wind_change_timer <= 0.0 {
                // Pick a new target: small random shift from current
                let seed = (self.time_of_day * 1000.0) as u32;
                let hash = |i: u32| -> f32 {
                    let h = seed.wrapping_mul(2654435761).wrapping_add(i.wrapping_mul(1013904223));
                    (h & 0xFFFF) as f32 / 65535.0
                };
                // Shift angle by ±45° (gentle drift)
                self.wind_target_angle += (hash(0) - 0.5) * std::f32::consts::FRAC_PI_2;
                // Vary magnitude ±30% around 8-12 range
                self.wind_target_mag = (self.wind_target_mag + (hash(1) - 0.5) * 6.0).clamp(3.0, 18.0);
                // Next change in 10-30 seconds game time
                self.wind_change_timer = 10.0 + hash(2) * 20.0;
            }
            // Smoothly interpolate current wind toward target
            let lerp_rate = 0.3 * dt * self.time_speed;
            let cur_angle = self.fluid_params.wind_y.atan2(self.fluid_params.wind_x);
            let cur_mag = (self.fluid_params.wind_x.powi(2) + self.fluid_params.wind_y.powi(2)).sqrt().max(0.1);
            // Angle interpolation (handle wrapping)
            let mut angle_diff = self.wind_target_angle - cur_angle;
            if angle_diff > std::f32::consts::PI { angle_diff -= std::f32::consts::TAU; }
            if angle_diff < -std::f32::consts::PI { angle_diff += std::f32::consts::TAU; }
            let new_angle = cur_angle + angle_diff * lerp_rate;
            let new_mag = cur_mag + (self.wind_target_mag - cur_mag) * lerp_rate;
            self.fluid_params.wind_x = new_angle.cos() * new_mag;
            self.fluid_params.wind_y = new_angle.sin() * new_mag;

            let mut rain = self.weather.rain_intensity();
            let mut sun_dim = self.weather.sun_dimming();
            // Drought: override weather effects
            if is_drought {
                rain = 0.0; // no rain during drought
                sun_dim = 1.0; // no cloud dimming
                // Temperature boost: +8°C equivalent (brighter sun)
                self.camera.sun_intensity *= 1.3;
            }
            // Dim sun during clouds/rain
            self.camera.sun_intensity *= sun_dim;
            self.camera.sun_color_r *= sun_dim;
            self.camera.sun_color_g *= sun_dim;
            self.camera.sun_color_b *= sun_dim;
            // Pass weather to shader and fluid sim
            self.camera.rain_intensity = rain;
            self.camera.cloud_cover = self.weather.cloud_cover();
            self.camera.wind_magnitude = (self.fluid_params.wind_x.powi(2) + self.fluid_params.wind_y.powi(2)).sqrt();
            self.camera.wind_angle = self.fluid_params.wind_y.atan2(self.fluid_params.wind_x);
            self.fluid_params.rain_intensity = rain;
            // Update wetness
            tick_wetness(
                &mut self.wetness_data, &self.grid_data,
                rain, self.camera.sun_intensity, dt, self.time_speed, GRID_W,
            );
            // Rain boosts CPU-side water table temporarily (so crops see moisture)
            if rain > 0.0 {
                let rain_boost = rain * 0.002 * dt * self.time_speed;
                for (i, wt) in self.water_table.iter_mut().enumerate() {
                    let b = self.grid_data[i];
                    let roof = (b >> 24) & 0xFF;
                    if roof == 0 { // outdoor only
                        *wt = (*wt + rain_boost).min(0.5);
                    }
                }
            }
            // Evaporation lowers water table back toward base
            if rain == 0.0 && self.camera.sun_intensity > 0.1 {
                let evap = 0.0001 * self.camera.sun_intensity * dt * self.time_speed;
                for wt in self.water_table.iter_mut() {
                    *wt = (*wt - evap).max(-3.0);
                }
            }
        }

        let prev_overlay = self.camera.fluid_overlay;
        self.camera.fluid_overlay = match self.fluid_overlay {
            FluidOverlay::None => 0.0,
            FluidOverlay::Gases => 1.0,
            FluidOverlay::Smoke => 2.0,
            FluidOverlay::Velocity => 3.0,
            FluidOverlay::Pressure => 4.0,
            FluidOverlay::O2 => 5.0,
            FluidOverlay::CO2 => 6.0,
            FluidOverlay::Temp => 7.0,
            FluidOverlay::Power => 9.0,
            FluidOverlay::PowerAmps => 10.0,
            FluidOverlay::PowerWatts => 11.0,
            FluidOverlay::Water => 12.0,
            FluidOverlay::WaterTable => 13.0,
        };
        // Pack velocity arrows flag as +0.25 on the overlay value
        if self.show_velocity_arrows && self.camera.fluid_overlay > 0.5 {
            self.camera.fluid_overlay += 0.25;
        }
        let prev_glow = self.camera.enable_prox_glow;
        let prev_bleed = self.camera.enable_dir_bleed;
        self.camera.enable_prox_glow = if self.enable_prox_glow { 1.0 } else { 0.0 };
        self.camera.enable_dir_bleed = if self.enable_dir_bleed { 1.0 } else { 0.0 };

        // Force refresh when grid changes or render settings toggle
        // Persist for several frames so lightmap has time to propagate changes
        let settings_changed = (self.camera.enable_prox_glow - prev_glow).abs() > 0.5
            || (self.camera.enable_dir_bleed - prev_bleed).abs() > 0.5
            || (self.camera.fluid_overlay - prev_overlay).abs() > 0.5;
        // Detect large time jumps (time-of-day buttons, slider scrubbing)
        let time_jumped = (self.camera.time - self.prev_cam_time).abs() > 1.0;
        if !self.enable_temporal {
            self.camera.force_refresh = 1.0; // always force refresh when temporal is disabled
        } else if self.grid_dirty || settings_changed || time_jumped {
            self.camera.force_refresh = 10.0;
            // Nudge prev camera to invalidate ALL reprojection checks
            // (some GPU drivers/WGSL compilers may not honor force_refresh alone)
            self.prev_cam_x += 100.0;
            self.prev_cam_y += 100.0;
        } else if self.camera.force_refresh > 0.5 {
            self.camera.force_refresh -= 1.0;
        }

        // --- Burst fire tick ---
        if self.burst_queue > 0 {
            self.burst_delay -= dt;
            if self.burst_delay <= 0.0 {
                if let Some(pleb) = self.selected_pleb.and_then(|i| self.plebs.get(i)) {
                    let dx = pleb.angle.cos();
                    let dy = pleb.angle.sin();
                    let sx = pleb.x + dx * 0.4;
                    let sy = pleb.y + dy * 0.4;
                    // Small random spread per shot
                    let spread = if self.burst_mode {
                        let seed = (self.burst_queue as f32 * 137.0 + self.time_of_day * 1000.0) as u32;
                        ((seed.wrapping_mul(2654435761) & 0xFFFF) as f32 / 65535.0 - 0.5) * 0.06
                    } else { 0.0 };
                    let bx = (pleb.angle + spread).cos();
                    let by = (pleb.angle + spread).sin();
                    self.physics_bodies.push(PhysicsBody::new_bullet(sx, sy, bx, by));
                    // Muzzle smoke
                    self.fluid_params.splat_x = sx;
                    self.fluid_params.splat_y = sy;
                    self.fluid_params.splat_vx = dx * 15.0;
                    self.fluid_params.splat_vy = dy * 15.0;
                    self.fluid_params.splat_radius = 0.3;
                    self.fluid_params.splat_active = 1.0;
                }
                self.burst_queue -= 1;
                self.burst_delay = 0.07; // ~70ms between burst shots (~14 rounds/sec)
            }
        }

        // --- Grenade charge ---
        if self.grenade_charging {
            self.grenade_charge = (self.grenade_charge + dt * 0.8).min(1.0); // ~1.25s to full charge
        }

        // --- Enemy random walk AI ---
        for pleb in self.plebs.iter_mut() {
            if !pleb.is_enemy { continue; }
            pleb.wander_timer -= dt * self.time_speed;
            if pleb.wander_timer <= 0.0 && pleb.path_idx >= pleb.path.len() {
                // Pick a random nearby walkable tile
                let seed = ((pleb.x * 137.0 + pleb.y * 311.0 + self.time_of_day * 1000.0) as u32)
                    .wrapping_mul(2654435761);
                let dx = ((seed & 0xFF) as f32 / 255.0 - 0.5) * 16.0;
                let dy = (((seed >> 8) & 0xFF) as f32 / 255.0 - 0.5) * 16.0;
                let target_x = (pleb.x + dx).clamp(1.0, GRID_W as f32 - 2.0) as i32;
                let target_y = (pleb.y + dy).clamp(1.0, GRID_H as f32 - 2.0) as i32;
                let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                let path = astar_path(&self.grid_data, start, (target_x, target_y));
                if !path.is_empty() {
                    pleb.path = path;
                    pleb.path_idx = 0;
                }
                // Next wander in 5-15 seconds
                pleb.wander_timer = 5.0 + ((seed >> 16) & 0xFF) as f32 / 255.0 * 10.0;
            }
        }

        // --- Pleb update ---
        // --- Cannon rotation (Q/E when cannon is selected) ---
        if let Some(cannon_idx) = self.block_sel.cannon {
            let rot_speed = 1.5f32; // radians per second
            if self.pressed_keys.contains(&KeyCode::KeyQ) {
                *self.cannon_angles.entry(cannon_idx).or_insert(0.0) -= rot_speed * dt;
            }
            if self.pressed_keys.contains(&KeyCode::KeyE) {
                *self.cannon_angles.entry(cannon_idx).or_insert(0.0) += rot_speed * dt;
            }
        }

        // --- Update all plebs ---
        let move_speed = 5.0f32;
        let sel = self.selected_pleb;

        for (i, pleb) in self.plebs.iter_mut().enumerate() {
            let is_selected = sel == Some(i);

            // WASD direct movement (only for selected pleb, blocked during crisis)
            if is_selected && !pleb.activity.is_crisis() {
                let mut dx = 0.0f32;
                let mut dy = 0.0f32;
                if self.pressed_keys.contains(&KeyCode::KeyW) { dy -= 1.0; }
                if self.pressed_keys.contains(&KeyCode::KeyS) { dy += 1.0; }
                if self.pressed_keys.contains(&KeyCode::KeyA) { dx -= 1.0; }
                if self.pressed_keys.contains(&KeyCode::KeyD) { dx += 1.0; }

                if dx != 0.0 || dy != 0.0 {
                    let len = (dx * dx + dy * dy).sqrt();
                    dx /= len; dy /= len;
                    pleb.angle = dy.atan2(dx);
                    let nx = pleb.x + dx * move_speed * dt;
                    let ny = pleb.y + dy * move_speed * dt;
                    if is_walkable_pos(&self.grid_data, nx, ny) {
                        pleb.x = nx;
                        pleb.y = ny;
                        let (cx, cy) = pleb_body_collision(&self.physics_bodies, pleb.x, pleb.y);
                        pleb.x = cx;
                        pleb.y = cy;
                    }
                    pleb.path.clear();
                    pleb.path_idx = 0;
                }

                // Q/E rotation
                if self.pressed_keys.contains(&KeyCode::KeyQ) { pleb.angle -= 2.0 * dt; }
                if self.pressed_keys.contains(&KeyCode::KeyE) { pleb.angle += 2.0 * dt; }
            }

            // Unstick: if pleb is on a non-walkable tile, nudge to nearest walkable
            if !is_walkable_pos(&self.grid_data, pleb.x, pleb.y) {
                let bx = pleb.x.floor() as i32;
                let by = pleb.y.floor() as i32;
                if let Some((wx, wy)) = adjacent_walkable(&self.grid_data, bx, by) {
                    pleb.x = wx as f32 + 0.5;
                    pleb.y = wy as f32 + 0.5;
                }
            }

            // A* path following (all plebs)
            if pleb.path_idx < pleb.path.len() {
                let (tx, ty) = pleb.path[pleb.path_idx];
                let target_x = tx as f32 + 0.5;
                let target_y = ty as f32 + 0.5;
                let ddx = target_x - pleb.x;
                let ddy = target_y - pleb.y;
                let dist = (ddx * ddx + ddy * ddy).sqrt();
                if dist < 0.2 {
                    pleb.path_idx += 1;
                } else {
                    let ndx = ddx / dist;
                    let ndy = ddy / dist;
                    pleb.angle = ndy.atan2(ndx);
                    let step_x = ndx * move_speed * dt;
                    let step_y = ndy * move_speed * dt;
                    let nx = pleb.x + step_x;
                    let ny = pleb.y + step_y;
                    if is_walkable_pos(&self.grid_data, nx, ny) {
                        pleb.x = nx;
                        pleb.y = ny;
                    } else if is_walkable_pos(&self.grid_data, nx, pleb.y) {
                        // Wall slide: try X only
                        pleb.x = nx;
                    } else if is_walkable_pos(&self.grid_data, pleb.x, ny) {
                        // Wall slide: try Y only
                        pleb.y = ny;
                    }
                }
            }
        }

        // --- Update pleb needs and auto-behaviors ---
        {
            let day_frac = self.time_of_day / DAY_DURATION;
            for (i, pleb) in self.plebs.iter_mut().enumerate() {
                let dx = pleb.x - pleb.prev_x;
                let dy = pleb.y - pleb.prev_y;
                let is_moving = (dx * dx + dy * dy) > 0.0001;
                let env = sample_environment(&self.grid_data, pleb.x, pleb.y, day_frac);
                let air = self.pleb_air_data.get(i);
                let is_sleeping = pleb.activity == PlebActivity::Sleeping;
                tick_needs(&mut pleb.needs, &env, dt, self.time_speed, is_moving, is_sleeping, air);

                let was_crisis = pleb.activity.is_crisis();
                tick_pleb_activity(pleb, &env, &self.grid_data, dt, self.time_speed, &mut self.ground_items);
                // Log new crisis
                if pleb.activity.is_crisis() && !was_crisis {
                    if let Some(reason) = pleb.activity.crisis_reason() {
                        events.push((EventCategory::Need, format!("{}: {}", pleb.name, reason)));
                    }
                }

                // Update walking state (handles both crisis and non-crisis walking)
                let inner = pleb.activity.inner().clone();
                if pleb.path_idx < pleb.path.len() && inner == PlebActivity::Idle {
                    if pleb.activity.is_crisis() {
                        let reason = pleb.activity.crisis_reason().unwrap_or("Crisis");
                        pleb.activity = PlebActivity::Crisis(Box::new(PlebActivity::Walking), reason);
                    } else {
                        pleb.activity = PlebActivity::Walking;
                    }
                } else if pleb.path_idx >= pleb.path.len() && inner == PlebActivity::Walking {
                    if pleb.activity.is_crisis() {
                        // Arrived at destination during crisis — check what to do
                        let reason = pleb.activity.crisis_reason().unwrap_or("Crisis");
                        if reason == "Starving!" {
                            // Arrived near bush — harvest or eat
                            if pleb.inventory.berries > 0 {
                                pleb.activity = PlebActivity::Crisis(Box::new(PlebActivity::Eating), reason);
                            } else if env.near_berry_bush {
                                if let Some((bx, by)) = env.nearest_berry_bush {
                                    pleb.harvest_target = Some((bx, by));
                                    pleb.activity = PlebActivity::Crisis(Box::new(PlebActivity::Harvesting(0.0)), reason);
                                }
                            } else {
                                pleb.activity = PlebActivity::Idle; // couldn't find food
                            }
                        } else if reason == "Exhausted!" {
                            if env.near_bed {
                                pleb.activity = PlebActivity::Crisis(Box::new(PlebActivity::Sleeping), reason);
                            } else {
                                pleb.activity = PlebActivity::Crisis(Box::new(PlebActivity::Sleeping), "Collapsed!");
                            }
                        } else if reason == "Overheating!" {
                            // Arrived at cool tile — stay idle, overheating check will re-trigger if still hot
                            pleb.activity = PlebActivity::Idle;
                        } else {
                            pleb.activity = PlebActivity::Idle;
                        }
                    } else {
                        pleb.activity = PlebActivity::Idle;
                    }
                }

                // Hauling state machine: pickup → walk to destination → deposit
                if pleb.activity == PlebActivity::Hauling {
                    let at_pickup = pleb.harvest_target.map(|(rx, ry)|
                        ((pleb.x - rx as f32 - 0.5).powi(2) + (pleb.y - ry as f32 - 0.5).powi(2)).sqrt() < 1.8
                    ).unwrap_or(false);
                    let at_delivery = pleb.haul_target.map(|(cx, cy)|
                        ((pleb.x - cx as f32 - 0.5).powi(2) + (pleb.y - cy as f32 - 0.5).powi(2)).sqrt() < 2.0
                    ).unwrap_or(false);
                    let path_done = pleb.path_idx >= pleb.path.len();

                    // Phase 1: not carrying anything → pick up item at harvest_target
                    if (path_done || at_pickup) && pleb.inventory.carrying.is_none() {
                        if let Some((rx, ry)) = pleb.harvest_target {
                            let ridx = (ry as u32 * GRID_W + rx as u32) as usize;
                            let is_rock = ridx < self.grid_data.len() && (self.grid_data[ridx] & 0xFF) == 34;
                            if is_rock {
                                let roof_bits = self.grid_data[ridx] & 0xFF000000;
                                let flag_bits = (self.grid_data[ridx] >> 16) & 2;
                                self.grid_data[ridx] = make_block(2, 0, flag_bits as u8) | roof_bits;
                                self.grid_dirty = true;
                                pleb.inventory.rocks += 1;
                                pleb.inventory.carrying = Some("Rock");
                                pleb.harvest_target = None;
                                events.push((EventCategory::Haul, format!("{} picked up a rock", pleb.name)));
                            } else if let Some(wi) = self.ground_items.iter().position(|item| {
                                item.x.floor() as i32 == rx && item.y.floor() as i32 == ry
                                    && matches!(item.kind, resources::ItemKind::Wood(_))
                            }) {
                                let wood_count = self.ground_items[wi].kind.count();
                                let take = wood_count.min(5);
                                if wood_count <= take { self.ground_items.remove(wi); }
                                else { self.ground_items[wi].kind = resources::ItemKind::Wood(wood_count - take); }
                                pleb.inventory.rocks = take;
                                pleb.inventory.carrying = Some("Wood");
                                pleb.harvest_target = None;
                                events.push((EventCategory::Haul, format!("{} picked up {} wood", pleb.name, take)));
                            } else {
                                // Item gone
                                pleb.harvest_target = None;
                                pleb.haul_target = None;
                                pleb.activity = PlebActivity::Idle;
                            }
                            // If we picked something up, walk to delivery target
                            if pleb.inventory.carrying.is_some() {
                                if let Some((cx, cy)) = pleb.haul_target {
                                    let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                                    let adj = adjacent_walkable(&self.grid_data, cx, cy).unwrap_or((cx, cy));
                                    let path = astar_path(&self.grid_data, start, adj);
                                    if !path.is_empty() { pleb.path = path; pleb.path_idx = 0; }
                                    else { pleb.activity = PlebActivity::Idle; }
                                } else { pleb.activity = PlebActivity::Idle; }
                            }
                        } else { pleb.activity = PlebActivity::Idle; }
                    }
                    // Phase 2: carrying item → deliver at haul_target
                    else if (path_done || at_delivery) && pleb.inventory.carrying.is_some() {
                        if let Some((cx, cy)) = pleb.haul_target {
                            if pleb.inventory.carrying == Some("Wood") {
                                if let Some(bp) = self.blueprints.get_mut(&(cx, cy)) {
                                    let deliver = pleb.inventory.rocks.min(bp.wood_needed - bp.wood_delivered);
                                    bp.wood_delivered += deliver;
                                    pleb.inventory.rocks -= deliver;
                                    if pleb.inventory.rocks == 0 { pleb.inventory.carrying = None; }
                                    events.push((EventCategory::Haul, format!("{} delivered {} wood ({}/{})",
                                        pleb.name, deliver, bp.wood_delivered, bp.wood_needed)));
                                }
                                self.active_work.remove(&(cx, cy));
                                pleb.haul_target = None;
                                pleb.activity = PlebActivity::Idle;
                            } else {
                                let cidx = cy as u32 * GRID_W + cx as u32;
                                let inv = self.crate_contents.entry(cidx).or_default();
                                if pleb.inventory.carrying == Some("Rock") {
                                    let can_store = inv.space().min(pleb.inventory.rocks);
                                    inv.rocks += can_store;
                                    pleb.inventory.rocks -= can_store;
                                    if pleb.inventory.rocks == 0 { pleb.inventory.carrying = None; }
                                }
                                if pleb.inventory.carrying.is_none() { pleb.haul_target = None; pleb.activity = PlebActivity::Idle; }
                                if let Some(inv) = self.crate_contents.get(&cidx) {
                                    let count = inv.total().min(CRATE_MAX_ITEMS) as u8;
                                    let ci = cidx as usize;
                                    if ci < self.grid_data.len() && (self.grid_data[ci] & 0xFF) == 33 {
                                        self.grid_data[ci] = (self.grid_data[ci] & 0xFFFF00FF) | ((count as u32) << 8);
                                        self.grid_dirty = true;
                                    }
                                }
                                events.push((EventCategory::Haul, format!("{} deposited items", pleb.name)));
                            }
                        }
                    }
                    else if path_done { pleb.activity = PlebActivity::Idle; }
                }

                // Crisis flee behavior: when holding breath or gasping, pathfind to fresh air
                if pleb.needs.breathing_state != BreathingState::Normal
                    && pleb.needs.breath_remaining < 15.0
                    && pleb.needs.flee_target.is_none()
                {
                    let bx = pleb.x.floor() as i32;
                    let by = pleb.y.floor() as i32;
                    if let Some(target) = find_breathable_tile(&self.grid_data, bx, by, 20) {
                        pleb.needs.flee_target = Some(target);
                        pleb.activity = PlebActivity::Walking;
                        let start = (bx, by);
                        let path = astar_path(&self.grid_data, start, target);
                        if !path.is_empty() {
                            pleb.path = path;
                            pleb.path_idx = 0;
                        }
                    }
                }
                if pleb.needs.breathing_state == BreathingState::Normal {
                    pleb.needs.flee_target = None;
                }

                pleb.prev_x = pleb.x;
                pleb.prev_y = pleb.y;
            }
        }

        // Auto-open doors near ANY pleb
        for pleb in &self.plebs {
            let pbx = pleb.x.floor() as i32;
            let pby = pleb.y.floor() as i32;
            for ddy in -1..=1 {
                for ddx in -1..=1 {
                    let door_x = pbx + ddx;
                    let door_y = pby + ddy;
                    if door_x >= 0 && door_y >= 0 && door_x < GRID_W as i32 && door_y < GRID_H as i32 {
                        let didx = (door_y as u32 * GRID_W + door_x as u32) as usize;
                        let db = self.grid_data[didx];
                        if is_door_rs(db) && (block_flags_rs(db) & 4) == 0 {
                            let dist = ((door_x as f32 + 0.5 - pleb.x).powi(2) + (door_y as f32 + 0.5 - pleb.y).powi(2)).sqrt();
                            if dist < 1.2 {
                                self.grid_data[didx] = (db & 0xFF00FFFF) | (((block_flags_rs(db) ^ 4) as u32) << 16);
                                self.grid_dirty = true;
                                self.auto_doors.push((door_x, door_y, self.time_of_day));
                            }
                        }
                    }
                }
            }
        }

        // Update camera uniform with selected pleb (for backward compat with single-pleb shader)
        // TODO: replace with pleb buffer for multi-pleb rendering
        if let Some(sel_idx) = self.selected_pleb {
            if let Some(pleb) = self.plebs.get(sel_idx) {
                self.camera.pleb_x = pleb.x;
                self.camera.pleb_y = pleb.y;
                self.camera.pleb_angle = pleb.angle;
                self.camera.pleb_selected = 1.0;
                self.camera.pleb_torch = if pleb.torch_on { 1.0 } else { 0.0 };
                self.camera.pleb_headlight = if pleb.headlight_on { 1.0 } else { 0.0 };
            }
        } else if let Some(pleb) = self.plebs.first() {
            // Show first pleb even if not selected (for lighting)
            self.camera.pleb_x = pleb.x;
            self.camera.pleb_y = pleb.y;
            self.camera.pleb_angle = pleb.angle;
            self.camera.pleb_selected = 0.0;
            self.camera.pleb_torch = if pleb.torch_on { 1.0 } else { 0.0 };
            self.camera.pleb_headlight = if pleb.headlight_on { 1.0 } else { 0.0 };
        } else {
            self.camera.pleb_x = 0.0;
            self.camera.pleb_y = 0.0;
            self.camera.pleb_torch = 0.0;
            self.camera.pleb_headlight = 0.0;
        }

        // Auto-close doors after 2 seconds
        {
            let current_time = self.time_of_day;
            // Check if any pleb is near auto-doors
            let pleb_positions: Vec<(f32, f32)> = self.plebs.iter().map(|p| (p.x, p.y)).collect();
            let mut doors_to_close = Vec::new();
            self.auto_doors.retain(|&(dx, dy, opened_time)| {
                let elapsed = (current_time - opened_time).abs();
                let should_close = elapsed > 2.0;
                let pleb_nearby = pleb_positions.iter().any(|&(px, py)| {
                    ((dx as f32 + 0.5 - px).powi(2) + (dy as f32 + 0.5 - py).powi(2)).sqrt() < 1.5
                });
                if should_close && !pleb_nearby {
                    doors_to_close.push((dx, dy));
                    false
                } else {
                    true
                }
            });
            for (dx, dy) in doors_to_close {
                let didx = (dy as u32 * GRID_W + dx as u32) as usize;
                let db = self.grid_data[didx];
                if is_door_rs(db) && (block_flags_rs(db) & 4) != 0 {
                    self.grid_data[didx] = (db & 0xFF00FFFF) | ((((db >> 16) & 0xFF) ^ 4) << 16);
                    self.grid_dirty = true;
                }
            }
        }

        // --- Physics tick ---
        {
            let sel_pleb = self.selected_pleb.and_then(|i| self.plebs.get(i));
            let pleb_data = sel_pleb.map(|p| {
                let pvx = if self.pressed_keys.contains(&KeyCode::KeyD) { 3.0 }
                    else if self.pressed_keys.contains(&KeyCode::KeyA) { -3.0 }
                    else { 0.0 };
                let pvy = if self.pressed_keys.contains(&KeyCode::KeyS) { 3.0 }
                    else if self.pressed_keys.contains(&KeyCode::KeyW) { -3.0 }
                    else { 0.0 };
                (p.x, p.y, pvx, pvy, p.angle)
            });
            // Collect pleb positions for bullet collision
            let pleb_positions: Vec<(f32, f32, usize)> = self.plebs.iter().enumerate()
                .map(|(i, p)| (p.x, p.y, i)).collect();
            let (impacts, bullet_hits) = tick_bodies(
                &mut self.physics_bodies,
                dt,
                &self.grid_data,
                self.fluid_params.wind_x,
                self.fluid_params.wind_y,
                pleb_data,
                &pleb_positions,
                self.selected_pleb,
                self.enable_ricochets,
            );

            // Apply bullet hits to plebs
            for hit in &bullet_hits {
                if let Some(pleb) = self.plebs.get_mut(hit.pleb_idx) {
                    events.push((EventCategory::Combat, format!("{} hit! ({:.0}% hp)", pleb.name, (pleb.needs.health - 0.2).max(0.0) * 100.0)));
                    pleb.needs.health -= 0.2;
                    self.fluid_params.splat_x = hit.x;
                    self.fluid_params.splat_y = hit.y;
                    self.fluid_params.splat_radius = 0.3;
                    self.fluid_params.splat_active = 1.0;
                }
            }

            // Handle projectile impacts — destroy blocks, inject smoke/toxic gas
            for impact in &impacts {
                if impact.destroy_block {
                    self.destroy_block_at(impact.block_x, impact.block_y);
                    log::info!("Cannonball destroyed block at ({}, {}) KE={:.0}",
                        impact.block_x, impact.block_y, impact.kinetic_energy);
                }
                if impact.is_grenade {
                    // Grenade: inject toxic cloud (high smoke + CO2) via direct dye write
                    // Stored in grenade_impacts for the render pass to write to dye texture
                    self.grenade_impacts.push((impact.x, impact.y));
                } else {
                    // Cannonball: inject smoke burst via splat
                    self.fluid_params.splat_x = impact.x;
                    self.fluid_params.splat_y = impact.y;
                    self.fluid_params.splat_vx = 0.0;
                    self.fluid_params.splat_vy = 0.0;
                    self.fluid_params.splat_radius = 2.0;
                    self.fluid_params.splat_active = 1.0;
                }
            }
        }

        // --- Crop growth ---
        if !self.time_paused {
            let grow_dt = dt * self.time_speed;
            let mut matured = Vec::new();
            for (&grid_idx, timer) in self.crop_timers.iter_mut() {
                let idx = grid_idx as usize;
                if idx >= self.grid_data.len() { continue; }
                let block = self.grid_data[idx];
                let bt = block & 0xFF;
                let stage = (block >> 8) & 0xFF;
                if bt != BT_CROP || stage >= CROP_MATURE { continue; }

                // --- Multi-factor growth model ---
                let day_frac = self.time_of_day / DAY_DURATION;
                let sun_t = ((day_frac - 0.15) / 0.7).clamp(0.0, 1.0);
                let sun_curve = (sun_t * std::f32::consts::PI).sin();
                let approx_temp = 5.0 + 20.0 * sun_curve;

                // Temperature: bell curve, optimal 15-28°C, zero outside 5-40°C
                let temp_factor = if approx_temp < CROP_TEMP_MIN || approx_temp > CROP_TEMP_MAX {
                    0.0
                } else if approx_temp >= CROP_OPTIMAL_LOW && approx_temp <= CROP_OPTIMAL_HIGH {
                    1.0
                } else if approx_temp < CROP_OPTIMAL_LOW {
                    (approx_temp - CROP_TEMP_MIN) / (CROP_OPTIMAL_LOW - CROP_TEMP_MIN)
                } else {
                    (CROP_TEMP_MAX - approx_temp) / (CROP_TEMP_MAX - CROP_OPTIMAL_HIGH)
                };

                // Sunlight: plants need light to photosynthesize
                let sun_factor = (self.camera.sun_intensity * 1.2).clamp(0.0, 1.0);

                // Water: combines water table depth + rain moisture
                let wt = if idx < self.water_table.len() { self.water_table[idx] } else { -3.0 };
                let wt_moisture = ((wt + 2.0) / 2.5).clamp(0.0, 1.0);
                let rain_moisture = (self.camera.rain_intensity * 0.5).min(0.3);
                // Approximate surface water: tiles with high water table or rain have it
                // (actual GPU water level not available per-tile on CPU)
                let surface_approx = if wt > -0.3 { 0.3 } else { rain_moisture };
                let water_avail = (wt_moisture + rain_moisture + surface_approx).clamp(0.0, 1.0);
                // Water response curve: ramps up to optimal, slight penalty if waterlogged
                let water_factor = if water_avail < 0.1 {
                    water_avail * 2.0 // very dry: severely limited
                } else if water_avail < 0.7 {
                    0.2 + water_avail * 1.14 // normal: linear ramp to ~1.0
                } else {
                    1.0 - (water_avail - 0.7) * 0.3 // waterlogged: slight penalty
                };

                // Per-tile randomness: 0.7-1.3 variation (deterministic from position)
                let hash = (grid_idx.wrapping_mul(2654435761).wrapping_add(stage * 1013904223)) & 0xFFFF;
                let random_factor = 0.7 + (hash as f32 / 65535.0) * 0.6;

                let growth_rate = temp_factor * sun_factor * water_factor * random_factor;
                *timer += grow_dt * growth_rate;
                if *timer >= CROP_GROW_TIME {
                    *timer = 0.0;
                    let new_stage = (stage + 1).min(CROP_MATURE);
                    let roof_h = block & 0xFF000000;
                    let flags_bits = (block >> 16) & 0xFF;
                    self.grid_data[idx] = make_block(BT_CROP as u8, new_stage as u8, flags_bits as u8) | roof_h;
                    self.grid_dirty = true;
                    if new_stage == CROP_MATURE {
                        matured.push(grid_idx);
                    }
                }
            }
            // Remove timers for matured crops
            for idx in matured { self.crop_timers.remove(&idx); }
        }

        // --- Work queue: assign idle friendly plebs to tasks ---
        {
            let mut tasks = generate_work_tasks(&self.zones, &self.grid_data, GRID_W, &self.active_work);
            // Add manual (player-ordered) tasks
            for task in self.manual_tasks.drain(..) {
                let pos = task.position();
                if !self.active_work.contains(&pos) {
                    tasks.push(task);
                }
            }
            for pleb in self.plebs.iter_mut() {
                if pleb.is_enemy { continue; }
                if pleb.activity != PlebActivity::Idle { continue; }
                if pleb.work_target.is_some() { continue; }

                // Find nearest task respecting priority
                let is_preferred = |t: &WorkTask| -> bool {
                    match self.work_priority {
                        WorkPriority::PlantFirst => matches!(t, WorkTask::Plant(_, _)),
                        WorkPriority::HarvestFirst => matches!(t, WorkTask::Harvest(_, _)),
                    }
                };
                let mut best_task: Option<(WorkTask, f32)> = None;
                let mut best_fallback: Option<(WorkTask, f32)> = None;
                for task in &tasks {
                    let (tx, ty) = task.position();
                    let dx = pleb.x - tx as f32 - 0.5;
                    let dy = pleb.y - ty as f32 - 0.5;
                    let dist = dx * dx + dy * dy;
                    if is_preferred(task) {
                        if best_task.is_none() || dist < best_task.as_ref().unwrap().1 {
                            best_task = Some((task.clone(), dist));
                        }
                    } else {
                        if best_fallback.is_none() || dist < best_fallback.as_ref().unwrap().1 {
                            best_fallback = Some((task.clone(), dist));
                        }
                    }
                }
                let best_task = best_task.or(best_fallback);

                if let Some((task, _)) = best_task {
                    let (tx, ty) = task.position();
                    let task_name = match &task {
                        WorkTask::Plant(_, _) => "plant",
                        WorkTask::Harvest(_, _) => "harvest",
                    };
                    events.push((EventCategory::Farm, format!("{} going to {} at ({},{})", pleb.name, task_name, tx, ty)));
                    self.active_work.insert((tx, ty));
                    pleb.work_target = Some((tx, ty));
                    // Path to the task
                    let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                    let path = astar_path(&self.grid_data, start, (tx, ty));
                    if !path.is_empty() {
                        pleb.path = path;
                        pleb.path_idx = 0;
                        pleb.activity = PlebActivity::Walking;
                    }
                }
            }

            // Handle Farming activity: pleb arrived at work target
            for pleb in self.plebs.iter_mut() {
                if pleb.is_enemy { continue; }

                // Check if pleb is doing Farming
                if let PlebActivity::Farming(progress) = &pleb.activity {
                    // Speed varies: trees take longer than crops/bushes
                    let speed = if let Some((tx, ty)) = pleb.work_target {
                        let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                        if tidx < self.grid_data.len() && (self.grid_data[tidx] & 0xFF) as u32 == BT_TREE { 0.15 } // ~7s for trees
                        else { 0.4 } // ~2.5s for crops/bushes
                    } else { 0.4 };
                    let new_progress = progress + dt * self.time_speed * speed;
                    if new_progress >= 1.0 {
                        // Complete the task
                        if let Some((tx, ty)) = pleb.work_target {
                            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                            if tidx < self.grid_data.len() {
                                let tblock = self.grid_data[tidx];
                                let tbt = tblock & 0xFF;
                                if tbt == BT_DIRT {
                                    let roof_h = tblock & 0xFF000000;
                                    let fflags = (tblock >> 16) & 0xFF;
                                    self.grid_data[tidx] = make_block(BT_CROP as u8, CROP_PLANTED as u8, fflags as u8) | roof_h;
                                    self.crop_timers.insert(tidx as u32, 0.0);
                                    self.grid_dirty = true;
                                    events.push((EventCategory::Farm, format!("{} planted a crop", pleb.name)));
                                } else if tbt == BT_CROP {
                                    let roof_h = tblock & 0xFF000000;
                                    let fflags = (tblock >> 16) & 0xFF;
                                    self.grid_data[tidx] = make_block(BT_DIRT as u8, 0, fflags as u8) | roof_h;
                                    self.crop_timers.remove(&(tidx as u32));
                                    self.grid_dirty = true;
                                    // Drop harvest on ground near pleb
                                    self.ground_items.push(resources::GroundItem {
                                        x: pleb.x, y: pleb.y,
                                        kind: resources::ItemKind::Berries(2),
                                    });
                                    events.push((EventCategory::Farm, format!("{} harvested a crop (2 berries dropped)", pleb.name)));
                                } else if tbt == BT_BERRY_BUSH {
                                    self.ground_items.push(resources::GroundItem {
                                        x: pleb.x, y: pleb.y,
                                        kind: resources::ItemKind::Berries(3),
                                    });
                                    events.push((EventCategory::Farm, format!("{} harvested berries (3 berries dropped)", pleb.name)));
                                } else if tbt == BT_TREE {
                                    // Chop down tree → remove all quadrants (2x2), drop 10 wood
                                    // Find the top-left corner from the quadrant flags
                                    let quadrant = (block_flags_rs(tblock) >> 3) & 3;
                                    let origin_x = tx - (quadrant & 1) as i32;
                                    let origin_y = ty - ((quadrant >> 1) & 1) as i32;
                                    // Clear all 4 tiles of the tree
                                    for dy in 0..2i32 {
                                        for dx in 0..2i32 {
                                            let nx = origin_x + dx;
                                            let ny = origin_y + dy;
                                            if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 { continue; }
                                            let nidx = (ny as u32 * GRID_W + nx as u32) as usize;
                                            if nidx < self.grid_data.len() && (self.grid_data[nidx] & 0xFF) as u32 == BT_TREE {
                                                let nroof = self.grid_data[nidx] & 0xFF000000;
                                                let nflags = (self.grid_data[nidx] >> 16) & 2;
                                                self.grid_data[nidx] = make_block(BT_DIRT as u8, 0, nflags as u8) | nroof;
                                            }
                                        }
                                    }
                                    self.grid_dirty = true;
                                    self.ground_items.push(resources::GroundItem {
                                        x: origin_x as f32 + 1.0, y: origin_y as f32 + 1.0,
                                        kind: resources::ItemKind::Wood(10),
                                    });
                                    events.push((EventCategory::Farm, format!("{} chopped a tree (10 wood)", pleb.name)));
                                }
                            }
                            self.active_work.remove(&(tx, ty));
                            pleb.work_target = None;
                        }
                        pleb.activity = PlebActivity::Idle;
                    } else {
                        pleb.activity = PlebActivity::Farming(new_progress);
                    }
                }

                // Arrived at work target: start farming (checks Walking OR Idle with work target)
                let has_work = pleb.work_target.is_some();
                let path_done = pleb.path_idx >= pleb.path.len();
                let is_walking_or_idle = pleb.activity == PlebActivity::Walking || pleb.activity == PlebActivity::Idle;
                if has_work && path_done && is_walking_or_idle {
                    if let Some((tx, ty)) = pleb.work_target {
                        let dist = ((pleb.x - tx as f32 - 0.5).powi(2) + (pleb.y - ty as f32 - 0.5).powi(2)).sqrt();
                        if dist < 1.5 {
                            pleb.activity = PlebActivity::Farming(0.0);
                        } else {
                            // Too far — release task and retry
                            self.active_work.remove(&(tx, ty));
                            pleb.work_target = None;
                            pleb.activity = PlebActivity::Idle;
                        }
                    }
                }
            }
        }

        // --- Construction: plebs build blueprints ---
        // 1. Handle Building activity progress
        for pleb in self.plebs.iter_mut() {
            if pleb.is_enemy { continue; }
            if let PlebActivity::Building(progress) = &pleb.activity {
                if let Some((tx, ty)) = pleb.work_target {
                    let new_progress = if let Some(bp) = self.blueprints.get(&(tx, ty)) {
                        progress + dt * self.time_speed / bp.build_time
                    } else {
                        1.0 // blueprint gone, finish immediately
                    };
                    if new_progress >= 1.0 {
                        // Construction complete — place the actual block
                        if let Some(bp) = self.blueprints.remove(&(tx, ty)) {
                            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                            if tidx < self.grid_data.len() {
                                self.grid_data[tidx] = bp.block_data;
                                self.grid_dirty = true;
                                events.push((EventCategory::Build, format!("{} built {}", pleb.name,
                                    block_defs::BlockRegistry::cached().name((bp.block_data & 0xFF) as u8))));
                            }
                        }
                        self.active_work.remove(&(tx, ty));
                        pleb.work_target = None;
                        pleb.activity = PlebActivity::Idle;
                    } else {
                        pleb.activity = PlebActivity::Building(new_progress);
                        // Also update blueprint progress for UI
                        if let Some(bp) = self.blueprints.get_mut(&(tx, ty)) {
                            bp.progress = new_progress;
                        }
                    }
                } else {
                    pleb.activity = PlebActivity::Idle;
                }
            }
        }

        // 2. Auto-assign idle plebs to blueprint tasks (haul resources or build)
        if !self.blueprints.is_empty() {
            let bp_positions: Vec<(i32, i32)> = self.blueprints.keys().copied().collect();
            for &(bx, by) in &bp_positions {
                if self.active_work.contains(&(bx, by)) { continue; }
                let bp = &self.blueprints[&(bx, by)];

                if bp.resources_met() {
                    // Resources delivered — assign pleb to build
                    let mut best: Option<(usize, f32)> = None;
                    for (i, pleb) in self.plebs.iter().enumerate() {
                        if pleb.is_enemy || pleb.work_target.is_some() { continue; }
                        if !matches!(pleb.activity, PlebActivity::Idle) { continue; }
                        let dist = ((pleb.x - bx as f32 - 0.5).powi(2) + (pleb.y - by as f32 - 0.5).powi(2)).sqrt();
                        if dist < 40.0 && (best.is_none() || dist < best.unwrap().1) {
                            best = Some((i, dist));
                        }
                    }
                    if let Some((pi, _)) = best {
                        let pleb = &mut self.plebs[pi];
                        let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                        let adj = adjacent_walkable(&self.grid_data, bx, by).unwrap_or((bx, by));
                        let path = astar_path(&self.grid_data, start, adj);
                        if !path.is_empty() {
                            pleb.path = path;
                            pleb.path_idx = 0;
                            pleb.activity = PlebActivity::Walking;
                            pleb.work_target = Some((bx, by));
                            self.active_work.insert((bx, by));
                        }
                    }
                } else {
                    // Needs resources — find nearest wood on ground and assign pleb to haul it
                    let wood_still_needed = bp.wood_needed - bp.wood_delivered;
                    if wood_still_needed > 0 {
                        // Find nearest wood ground item
                        let mut best_wood: Option<(usize, f32)> = None;
                        for (i, item) in self.ground_items.iter().enumerate() {
                            if let resources::ItemKind::Wood(_) = item.kind {
                                let d = ((item.x - bx as f32 - 0.5).powi(2) + (item.y - by as f32 - 0.5).powi(2)).sqrt();
                                if best_wood.is_none() || d < best_wood.unwrap().1 {
                                    best_wood = Some((i, d));
                                }
                            }
                        }
                        if let Some((wi, _)) = best_wood {
                            let wood_pos = (self.ground_items[wi].x.floor() as i32, self.ground_items[wi].y.floor() as i32);
                            // Find nearest idle pleb
                            let mut best_pleb: Option<(usize, f32)> = None;
                            for (i, pleb) in self.plebs.iter().enumerate() {
                                if pleb.is_enemy || pleb.work_target.is_some() { continue; }
                                if !matches!(pleb.activity, PlebActivity::Idle) { continue; }
                                let dist = ((pleb.x - wood_pos.0 as f32 - 0.5).powi(2) + (pleb.y - wood_pos.1 as f32 - 0.5).powi(2)).sqrt();
                                if dist < 50.0 && (best_pleb.is_none() || dist < best_pleb.unwrap().1) {
                                    best_pleb = Some((i, dist));
                                }
                            }
                            if let Some((pi, _)) = best_pleb {
                                let pleb = &mut self.plebs[pi];
                                let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                                let path = astar_path(&self.grid_data, start, wood_pos);
                                if !path.is_empty() {
                                    pleb.path = path;
                                    pleb.path_idx = 0;
                                    pleb.activity = PlebActivity::Hauling;
                                    pleb.harvest_target = Some(wood_pos); // pickup location
                                    pleb.haul_target = Some((bx, by));    // delivery = blueprint
                                    self.active_work.insert((bx, by));
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Plebs arriving at blueprints: start building (if resources met) or deliver wood
        for pleb in self.plebs.iter_mut() {
            if pleb.is_enemy { continue; }
            // Walking pleb arriving at blueprint → start building
            if pleb.activity == PlebActivity::Walking {
                if let Some((tx, ty)) = pleb.work_target {
                    if let Some(bp) = self.blueprints.get(&(tx, ty)) {
                        if bp.resources_met() {
                            let dist = ((pleb.x - tx as f32 - 0.5).powi(2) + (pleb.y - ty as f32 - 0.5).powi(2)).sqrt();
                            if dist < 1.5 {
                                pleb.activity = PlebActivity::Building(0.0);
                                pleb.path.clear();
                                pleb.path_idx = 0;
                            }
                        }
                    }
                }
            }
        }

        // Handle plebs arriving at ground item to eat:
        // Check proximity every frame (not just path_done) — pleb may walk close enough mid-path
        for pleb in self.plebs.iter_mut() {
            if pleb.is_enemy { continue; }
            let is_walking_or_idle = matches!(pleb.activity, PlebActivity::Walking | PlebActivity::Idle);
            let has_eat_target = pleb.harvest_target.is_some() && pleb.work_target.is_none() && pleb.haul_target.is_none();
            if is_walking_or_idle && has_eat_target {
                if let Some((tx, ty)) = pleb.harvest_target {
                    let dist = ((pleb.x - tx as f32 - 0.5).powi(2) + (pleb.y - ty as f32 - 0.5).powi(2)).sqrt();
                    if dist < 1.5 {
                        pleb.activity = PlebActivity::Eating;
                        pleb.path.clear();
                        pleb.path_idx = 0;
                    }
                }
            }
        }

        // --- Remove dead plebs ---
        for pleb in &self.plebs {
            if pleb.needs.health <= 0.0 {
                events.push((EventCategory::Combat, format!("{} has died!", pleb.name)));
            }
        }
        self.plebs.retain(|p| p.needs.health > 0.0);

        // Push all collected events to the game log
        for (cat, msg) in events {
            self.log_event(cat, msg);
        }

        dt
    }
}

/// Tick a single pleb's activity state machine and auto-behaviors.
/// Handles: sleep/harvest/eat progress, crisis triggers (starving/exhausted/overheating),
/// and non-crisis auto-behaviors (auto-eat, auto-sleep, auto-harvest).
fn tick_pleb_activity(
    pleb: &mut Pleb,
    env: &needs::EnvSample,
    grid: &[u32],
    dt: f32,
    time_speed: f32,
    ground_items: &mut Vec<resources::GroundItem>,
) {
    // --- Activity state machine (works on inner activity for crisis) ---
    let inner_act = pleb.activity.inner().clone();
    let was_crisis = pleb.activity.is_crisis();
    let crisis_reason = pleb.activity.crisis_reason();

    match &inner_act {
        PlebActivity::Sleeping => {
            if pleb.needs.rest > 0.95
                || pleb.needs.breathing_state != BreathingState::Normal
            {
                pleb.activity = PlebActivity::Idle;
            }
        }
        PlebActivity::Harvesting(progress) => {
            let new_progress = progress + dt * time_speed * 0.5;
            if new_progress >= 1.0 {
                // Drop berries on ground near pleb
                ground_items.push(resources::GroundItem {
                    x: pleb.x, y: pleb.y,
                    kind: resources::ItemKind::Berries(3),
                });
                pleb.harvest_target = None;
                log::info!("{} harvested 3 berries (dropped on ground)", pleb.name);
                if was_crisis {
                    pleb.activity = PlebActivity::Crisis(
                        Box::new(PlebActivity::Eating),
                        crisis_reason.unwrap_or("Starving"),
                    );
                } else {
                    pleb.activity = PlebActivity::Idle;
                }
            } else if was_crisis {
                pleb.activity = PlebActivity::Crisis(
                    Box::new(PlebActivity::Harvesting(new_progress)),
                    crisis_reason.unwrap_or("Starving"),
                );
            } else {
                pleb.activity = PlebActivity::Harvesting(new_progress);
            }
        }
        PlebActivity::Eating => {
            let mut ate = false;
            // Try eating from inventory first
            if pleb.inventory.berries > 0 {
                pleb.inventory.berries -= 1;
                pleb.needs.hunger = (pleb.needs.hunger + BERRY_HUNGER_RESTORE).min(1.0);
                log::info!("{} ate a berry from inventory (hunger: {:.0}%)",
                    pleb.name, pleb.needs.hunger * 100.0);
                ate = true;
            }
            // Try eating from ground item at harvest_target
            if !ate {
                if let Some((tx, ty)) = pleb.harvest_target {
                    if let Some(gi) = ground_items.iter_mut().position(|item| {
                        item.x.floor() as i32 == tx && item.y.floor() as i32 == ty
                            && matches!(item.kind, resources::ItemKind::Berries(_))
                    }) {
                        if let resources::ItemKind::Berries(n) = ground_items[gi].kind {
                            pleb.needs.hunger = (pleb.needs.hunger + BERRY_HUNGER_RESTORE).min(1.0);
                            if n <= 1 { ground_items.remove(gi); }
                            else { ground_items[gi].kind = resources::ItemKind::Berries(n - 1); }
                            log::info!("{} ate a berry from ground (hunger: {:.0}%)",
                                pleb.name, pleb.needs.hunger * 100.0);
                            ate = true;
                        }
                    }
                }
            }
            pleb.harvest_target = None;
            if was_crisis && pleb.needs.hunger < 0.3 && (pleb.inventory.berries > 0 || ate) {
                pleb.activity = PlebActivity::Crisis(
                    Box::new(PlebActivity::Eating),
                    crisis_reason.unwrap_or("Starving"),
                );
            } else {
                pleb.activity = PlebActivity::Idle;
            }
        }
        _ => {}
    }

    // --- Crisis auto-behaviors (override player control) ---
    let is_idle_or_walk = matches!(pleb.activity.inner(),
        PlebActivity::Idle | PlebActivity::Walking);

    if pleb.needs.hunger < 0.10 && is_idle_or_walk {
        // CRISIS: Starving
        if pleb.inventory.berries > 0 {
            pleb.activity = PlebActivity::Crisis(Box::new(PlebActivity::Eating), "Starving!");
        } else if let Some((bx, by)) = env.nearest_berry_bush {
            if env.near_berry_bush {
                pleb.harvest_target = Some((bx, by));
                pleb.activity = PlebActivity::Crisis(Box::new(PlebActivity::Harvesting(0.0)), "Starving!");
                pleb.path.clear();
                pleb.path_idx = 0;
            } else {
                send_pleb_to(pleb, grid, (bx, by),
                    PlebActivity::Crisis(Box::new(PlebActivity::Walking), "Starving!"));
            }
        }
    } else if pleb.needs.rest < 0.08 && is_idle_or_walk && !pleb.activity.is_crisis() {
        // CRISIS: Exhausted
        if env.near_bed {
            pleb.activity = PlebActivity::Crisis(Box::new(PlebActivity::Sleeping), "Exhausted!");
            pleb.path.clear();
            pleb.path_idx = 0;
        } else if let Some((bx, by)) = env.nearest_bed {
            send_pleb_to(pleb, grid, (bx, by),
                PlebActivity::Crisis(Box::new(PlebActivity::Walking), "Exhausted!"));
        } else {
            pleb.activity = PlebActivity::Crisis(Box::new(PlebActivity::Sleeping), "Collapsed!");
            pleb.path.clear();
            pleb.path_idx = 0;
        }
    }

    // CRISIS: Overheating — overrides ALL activities (even sleeping/harvesting)
    // Any pleb in dangerous heat drops everything and runs
    // Only triggers on idle/walking to prevent re-trigger loop when pleb arrives at cool tile
    let can_heat_flee = matches!(pleb.activity.inner(), PlebActivity::Idle | PlebActivity::Walking);
    if pleb.needs.air_temp > HEAT_CRISIS_TEMP && can_heat_flee && pleb.activity.crisis_reason() != Some("Overheating!") {
        let bx = pleb.x.floor() as i32;
        let by = pleb.y.floor() as i32;
        // Search from radius 3+ to avoid pathing to an adjacent tile that's equally hot
        if let Some(target) = find_cool_tile(grid, bx, by, 20) {
            send_pleb_to(pleb, grid, target,
                PlebActivity::Crisis(Box::new(PlebActivity::Walking), "Overheating!"));
        }
    } else if !pleb.activity.is_crisis() {
        // Non-crisis auto-behaviors
        if pleb.activity == PlebActivity::Idle || pleb.activity == PlebActivity::Walking {
            if pleb.needs.hunger < 0.25 && pleb.inventory.berries > 0 {
                pleb.activity = PlebActivity::Eating;
            } else if (pleb.needs.rest < 0.2 || (pleb.needs.rest < 0.4 && env.is_night))
                && !matches!(pleb.activity, PlebActivity::Sleeping)
            {
                if env.near_bed {
                    pleb.activity = PlebActivity::Sleeping;
                    pleb.path.clear();
                    pleb.path_idx = 0;
                } else if let Some((bx, by)) = env.nearest_bed {
                    send_pleb_to(pleb, grid, (bx, by), PlebActivity::Walking);
                }
            } else if pleb.needs.hunger < 0.4 && pleb.inventory.berries == 0 {
                if env.near_berry_bush && pleb.harvest_target.is_none() {
                    if let Some((bx, by)) = env.nearest_berry_bush {
                        pleb.harvest_target = Some((bx, by));
                        pleb.activity = PlebActivity::Harvesting(0.0);
                        pleb.path.clear();
                        pleb.path_idx = 0;
                    }
                } else if pleb.harvest_target.is_none() {
                    if let Some((bx, by)) = env.nearest_berry_bush {
                        send_pleb_to(pleb, grid, (bx, by), PlebActivity::Walking);
                    }
                }
            }
        }
    }
}

/// Helper: pathfind pleb to a target and set their activity. Returns true if path found.
fn send_pleb_to(pleb: &mut Pleb, grid: &[u32], target: (i32, i32), activity: PlebActivity) -> bool {
    let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
    let path = astar_path(grid, start, target);
    if !path.is_empty() {
        pleb.path = path;
        pleb.path_idx = 0;
        pleb.activity = activity;
        true
    } else {
        false
    }
}
