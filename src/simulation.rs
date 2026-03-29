//! Simulation update — time, sun, plebs, physics, pipes, doors.
//! Extracted from render() to keep main.rs manageable.

use crate::item_defs::*;
use crate::recipe_defs;
use crate::zones::*;
use crate::*;

impl App {
    /// Update all simulation state. Returns frame delta time.
    pub(crate) fn update_simulation(&mut self) -> f32 {
        let mut events: Vec<GameEventKind> = Vec::new();

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
                    if cond.remaining <= 0.0 {
                        ended.push(cond.name.clone());
                    }
                }
            }
            for name in &ended {
                events.push(GameEventKind::DroughtEnded(name.to_string()));
                self.notify(
                    NotifCategory::Positive,
                    "\u{2705}",
                    &format!("{} ended", name),
                    "Conditions returning to normal.",
                );
            }
            self.conditions
                .retain(|c| c.remaining > 0.0 || c.duration == 0.0);

            self.drought_check_timer -= dt_game;
            if self.drought_check_timer <= 0.0 {
                self.drought_check_timer = 60.0 + (self.time_of_day * 137.0) as f32 % 60.0;
                let seed = (self.time_of_day * 10000.0) as u32;
                let roll = seed.wrapping_mul(2654435761) & 0xFF;
                if roll < 25
                    && self.weather == WeatherState::Clear
                    && !self.has_condition("Drought")
                {
                    let duration = 60.0 + (roll as f32) * 1.5;
                    self.add_condition("Drought", "\u{2600}", NotifCategory::Threat, duration);
                    self.notify(
                        NotifCategory::Threat,
                        "\u{2600}",
                        "Drought",
                        format!("A drought has begun! Water drying up. ({:.0}s)", duration),
                    );
                    events.push(GameEventKind::DroughtStarted);
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
            // Shadow intensity: stronger at dawn/dusk (low sun = directional),
            // softer at noon (overhead = scattered). noon: 0→1→0 over the day.
            const SHADOW_DAWN_DUSK: f32 = 0.9;
            const SHADOW_NOON_REDUCTION: f32 = 0.4;
            self.camera.shadow_intensity = SHADOW_DAWN_DUSK - SHADOW_NOON_REDUCTION * noon;
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
            if let Some(new_weather) =
                tick_weather(&self.weather, &mut self.weather_timer, dt, self.time_speed)
            {
                let label = match &new_weather {
                    WeatherState::Clear => "Weather: Clear skies",
                    WeatherState::Cloudy => "Weather: Cloudy",
                    WeatherState::LightRain => "Weather: Light rain",
                    WeatherState::HeavyRain => "Weather: Heavy rain",
                };
                events.push(GameEventKind::WeatherChanged(label));
                self.weather = new_weather;
            }
            // --- Lightning during heavy rain ---
            self.lightning_flash = (self.lightning_flash - dt * 2.0).max(0.0); // slower decay for visible bolt
            if self.lightning_flash < 0.01 {
                self.lightning_strike = None;
            }
            if self.weather == WeatherState::HeavyRain {
                self.lightning_timer -= dt * self.time_speed;
                if self.lightning_timer <= 0.0 {
                    // Random strike location (outdoor, no roof)
                    let seed = (self.time_of_day * 10000.0) as u32;
                    let hash = |i: u32| -> u32 {
                        seed.wrapping_mul(2654435761)
                            .wrapping_add(i.wrapping_mul(1013904223))
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

                        // Thunder (~120 dB)
                        if self.sound_enabled {
                            self.sound_sources.push(SoundSource {
                                x: sx as f32 + 0.5,
                                y: sy as f32 + 0.5,
                                amplitude: db_to_amplitude(120.0),
                                frequency: 0.0,
                                phase: 0.0,
                                pattern: 0,
                                duration: 0.2,
                            });
                        }

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
                            log::info!(
                                "Lightning hit power grid at ({}, {})! Voltage surge!",
                                sx,
                                sy
                            );
                        }
                        // Voltage surge injection + breaker tripping happens in render pass
                        // via GPU voltage buffer writes + GPU-side breaker threshold check

                        events.push(GameEventKind::Lightning(sx, sy));
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
                    let h = seed
                        .wrapping_mul(2654435761)
                        .wrapping_add(i.wrapping_mul(1013904223));
                    (h & 0xFFFF) as f32 / 65535.0
                };
                // Shift angle by ±45° (gentle drift)
                self.wind_target_angle += (hash(0) - 0.5) * std::f32::consts::FRAC_PI_2;
                // Vary magnitude ±30% around 8-12 range
                self.wind_target_mag =
                    (self.wind_target_mag + (hash(1) - 0.5) * 6.0).clamp(3.0, 18.0);
                // Next change in 10-30 seconds game time
                self.wind_change_timer = 10.0 + hash(2) * 20.0;
            }
            // Smoothly interpolate current wind toward target
            let lerp_rate = 0.3 * dt * self.time_speed;
            let cur_angle = self.fluid_params.wind_y.atan2(self.fluid_params.wind_x);
            let cur_mag = (self.fluid_params.wind_x.powi(2) + self.fluid_params.wind_y.powi(2))
                .sqrt()
                .max(0.1);
            // Angle interpolation (handle wrapping)
            let mut angle_diff = self.wind_target_angle - cur_angle;
            if angle_diff > std::f32::consts::PI {
                angle_diff -= std::f32::consts::TAU;
            }
            if angle_diff < -std::f32::consts::PI {
                angle_diff += std::f32::consts::TAU;
            }
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
            self.camera.wind_magnitude =
                (self.fluid_params.wind_x.powi(2) + self.fluid_params.wind_y.powi(2)).sqrt();
            self.camera.wind_angle = self.fluid_params.wind_y.atan2(self.fluid_params.wind_x);
            self.fluid_params.rain_intensity = rain;
            // Update wetness
            tick_wetness(
                &mut self.wetness_data,
                &self.grid_data,
                rain,
                self.camera.sun_intensity,
                dt,
                self.time_speed,
                GRID_W,
            );
            // Rain boosts CPU-side water table temporarily (so crops see moisture)
            if rain > 0.0 {
                let rain_boost = rain * 0.002 * dt * self.time_speed;
                for (i, wt) in self.water_table.iter_mut().enumerate() {
                    let b = self.grid_data[i];
                    let roof = (b >> 24) & 0xFF;
                    if roof == 0 {
                        // outdoor only
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
            FluidOverlay::Sound => 14.0,
            FluidOverlay::Terrain => 15.0,
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
                        let seed =
                            (self.burst_queue as f32 * 137.0 + self.time_of_day * 1000.0) as u32;
                        ((seed.wrapping_mul(2654435761) & 0xFFFF) as f32 / 65535.0 - 0.5) * 0.06
                    } else {
                        0.0
                    };
                    let bx = (pleb.angle + spread).cos();
                    let by = (pleb.angle + spread).sin();
                    self.physics_bodies
                        .push(PhysicsBody::new_bullet(sx, sy, bx, by));
                    // Gunshot sound (~100 dB)
                    if self.sound_enabled {
                        self.sound_sources.push(SoundSource {
                            x: sx,
                            y: sy,
                            amplitude: db_to_amplitude(100.0),
                            frequency: 0.0,
                            phase: 0.0,
                            pattern: 0,
                            duration: 0.05,
                        });
                    }
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
            if !pleb.is_enemy {
                continue;
            }
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
                let path = astar_path_terrain_wd(
                    &self.grid_data,
                    &self.wall_data,
                    &self.terrain_data,
                    start,
                    (target_x, target_y),
                );
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
        let move_speed = 20.0f32;
        let sel = self.selected_pleb;

        for (i, pleb) in self.plebs.iter_mut().enumerate() {
            if pleb.is_dead {
                continue;
            } // corpses don't act
            let is_selected = sel == Some(i);

            // Q/E rotation for selected pleb
            if is_selected && !pleb.activity.is_crisis() {
                // Q/E rotation
                if self.pressed_keys.contains(&KeyCode::KeyQ) {
                    pleb.angle -= 2.0 * dt;
                }
                if self.pressed_keys.contains(&KeyCode::KeyE) {
                    pleb.angle += 2.0 * dt;
                }
            }

            // Unstick: if pleb is on a non-walkable tile, nudge to nearest walkable
            if !is_walkable_pos_wd(&self.grid_data, &self.wall_data, pleb.x, pleb.y) {
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
                    // Speed modifier from terrain compaction + roughness
                    let tile_x = pleb.x.floor() as i32;
                    let tile_y = pleb.y.floor() as i32;
                    let speed_mul = if tile_x >= 0
                        && tile_y >= 0
                        && tile_x < GRID_W as i32
                        && tile_y < GRID_H as i32
                    {
                        let tidx = (tile_y as u32 * GRID_W + tile_x as u32) as usize;
                        if tidx < self.terrain_data.len() {
                            let compact = terrain_compaction(self.terrain_data[tidx]) as f32;
                            let rough = terrain_roughness(self.terrain_data[tidx]) as f32;
                            // Compaction: 0→1.0x, 31→1.25x speed boost
                            // Roughness: 0→1.0x, 3→0.85x speed penalty
                            (1.0 + compact / 31.0 * 0.25) * (1.0 - rough / 3.0 * 0.15)
                        } else {
                            1.0
                        }
                    } else {
                        1.0
                    };

                    let ndx = ddx / dist;
                    let ndy = ddy / dist;
                    pleb.angle = ndy.atan2(ndx);
                    let effective_speed = move_speed * speed_mul * self.time_speed;
                    let step_x = ndx * effective_speed * dt;
                    let step_y = ndy * effective_speed * dt;
                    let nx = pleb.x + step_x;
                    let ny = pleb.y + step_y;
                    // Check walkability AND wall edge crossings
                    let old_tx = pleb.x.floor() as i32;
                    let old_ty = pleb.y.floor() as i32;
                    let can_move = |mx: f32, my: f32| -> bool {
                        if !is_walkable_pos_wd(&self.grid_data, &self.wall_data, mx, my) {
                            return false;
                        }
                        // Check if movement crosses a tile boundary with a wall edge
                        let new_tx = mx.floor() as i32;
                        let new_ty = my.floor() as i32;
                        if new_tx != old_tx || new_ty != old_ty {
                            if edge_blocked_wd(
                                &self.grid_data,
                                &self.wall_data,
                                old_tx,
                                old_ty,
                                new_tx,
                                new_ty,
                            ) {
                                return false;
                            }
                        }
                        true
                    };
                    if can_move(nx, ny) {
                        pleb.x = nx;
                        pleb.y = ny;
                    } else if can_move(nx, pleb.y) {
                        pleb.x = nx;
                    } else if can_move(pleb.x, ny) {
                        pleb.y = ny;
                    }

                    // Increment compaction on the tile being walked on
                    if tile_x >= 0
                        && tile_y >= 0
                        && tile_x < GRID_W as i32
                        && tile_y < GRID_H as i32
                    {
                        let tidx = (tile_y as u32 * GRID_W + tile_x as u32) as usize;
                        if tidx < self.terrain_data.len() {
                            let before = terrain_compaction(self.terrain_data[tidx]);
                            terrain_add_compaction(&mut self.terrain_data[tidx], 1);
                            if terrain_compaction(self.terrain_data[tidx]) != before {
                                self.terrain_dirty = true;
                            }
                        }
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
                tick_needs(
                    &mut pleb.needs,
                    &env,
                    dt,
                    self.time_speed,
                    is_moving,
                    is_sleeping,
                    air,
                );

                let was_crisis = pleb.activity.is_crisis();
                tick_pleb_activity(
                    pleb,
                    &env,
                    &self.grid_data,
                    &self.wall_data,
                    &self.terrain_data,
                    dt,
                    self.time_speed,
                    &mut self.ground_items,
                    self.time_of_day,
                );
                // Log new crisis
                if pleb.activity.is_crisis() && !was_crisis {
                    if let Some(reason) = pleb.activity.crisis_reason() {
                        events.push(GameEventKind::CrisisStarted {
                            pleb: pleb.name.clone(),
                            reason,
                        });
                    }
                }

                // Update walking state (handles both crisis and non-crisis walking)
                let inner = pleb.activity.inner().clone();
                if pleb.path_idx < pleb.path.len() && inner == PlebActivity::Idle {
                    if pleb.activity.is_crisis() {
                        let reason = pleb.activity.crisis_reason().unwrap_or("Crisis");
                        pleb.activity =
                            PlebActivity::Crisis(Box::new(PlebActivity::Walking), reason);
                    } else {
                        pleb.activity = PlebActivity::Walking;
                    }
                } else if pleb.path_idx >= pleb.path.len() && inner == PlebActivity::Walking {
                    if pleb.activity.is_crisis() {
                        // Arrived at destination during crisis — check what to do
                        let reason = pleb.activity.crisis_reason().unwrap_or("Crisis");
                        if reason == "Starving!" {
                            // Arrived near bush — harvest or eat
                            if pleb.inventory.count_of(ITEM_BERRIES) > 0 {
                                pleb.activity =
                                    PlebActivity::Crisis(Box::new(PlebActivity::Eating), reason);
                            } else if env.near_berry_bush {
                                if let Some((bx, by)) = env.nearest_berry_bush {
                                    pleb.harvest_target = Some((bx, by));
                                    pleb.activity = PlebActivity::Crisis(
                                        Box::new(PlebActivity::Harvesting(0.0)),
                                        reason,
                                    );
                                }
                            } else {
                                pleb.activity = PlebActivity::Idle; // couldn't find food
                            }
                        } else if reason == "Exhausted!" {
                            if env.near_bed {
                                pleb.activity =
                                    PlebActivity::Crisis(Box::new(PlebActivity::Sleeping), reason);
                            } else {
                                pleb.activity = PlebActivity::Crisis(
                                    Box::new(PlebActivity::Sleeping),
                                    "Collapsed!",
                                );
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
                    let at_pickup = pleb
                        .harvest_target
                        .map(|(rx, ry)| {
                            ((pleb.x - rx as f32 - 0.5).powi(2)
                                + (pleb.y - ry as f32 - 0.5).powi(2))
                            .sqrt()
                                < 1.8
                        })
                        .unwrap_or(false);
                    let at_delivery = pleb
                        .haul_target
                        .map(|(cx, cy)| {
                            ((pleb.x - cx as f32 - 0.5).powi(2)
                                + (pleb.y - cy as f32 - 0.5).powi(2))
                            .sqrt()
                                < 2.0
                        })
                        .unwrap_or(false);
                    let path_done = pleb.path_idx >= pleb.path.len();

                    // Phase 1: at pickup location → pick up item
                    // at_pickup required (not just path_done) so carrying plebs
                    // redirected to fetch more don't skip pickup
                    if at_pickup {
                        if let Some((rx, ry)) = pleb.harvest_target {
                            let ridx = (ry as u32 * GRID_W + rx as u32) as usize;
                            let is_rock =
                                ridx < self.grid_data.len() && (self.grid_data[ridx] & 0xFF) == 34;
                            if is_rock {
                                let roof_bits = self.grid_data[ridx] & 0xFF000000;
                                let flag_bits = (self.grid_data[ridx] >> 16) & 2;
                                self.grid_data[ridx] =
                                    make_block(2, 0, flag_bits as u8) | roof_bits;
                                self.grid_dirty = true;
                                // Stone Pick: quarry 4 rock, bare hands: 1 rock
                                let has_pick = pleb.inventory.count_of(ITEM_STONE_PICK) > 0;
                                let rock_yield: u16 = if has_pick { 4 } else { 1 };
                                pleb.inventory.add(ITEM_ROCK, rock_yield);
                                pleb.harvest_target = None;
                                events.push(GameEventKind::PickedUp {
                                    pleb: pleb.name.clone(),
                                    count: rock_yield,
                                    item: "rock".into(),
                                });
                            } else if let Some(wi) = {
                                // Prefer the specific item type the blueprint needs
                                let prefer_id: Option<u16> =
                                    pleb.haul_target.and_then(|(cx, cy)| {
                                        self.blueprints.get(&(cx, cy)).and_then(|bp| {
                                            if bp.is_roof() {
                                                Some(ITEM_FIBER)
                                            } else if bp.is_campfire() {
                                                Some(ITEM_SCRAP_WOOD)
                                            } else {
                                                None
                                            }
                                        })
                                    });
                                // Try preferred item first, fall back to any item
                                let preferred = prefer_id.and_then(|pid| {
                                    self.ground_items.iter().position(|item| {
                                        item.stack.item_id == pid
                                            && item.x.floor() as i32 == rx
                                            && item.y.floor() as i32 == ry
                                    })
                                });
                                preferred.or_else(|| {
                                    self.ground_items.iter().position(|item| {
                                        item.x.floor() as i32 == rx && item.y.floor() as i32 == ry
                                    })
                                })
                            } {
                                // Pick up ground item
                                let item_id = self.ground_items[wi].stack.item_id;
                                let count = self.ground_items[wi].stack.count;
                                let max_take = if item_id == ITEM_WOOD { 5u16 } else { 10 };
                                let take = count.min(max_take);
                                if count <= take {
                                    self.ground_items.remove(wi);
                                } else {
                                    self.ground_items[wi].stack.count -= take;
                                }
                                pleb.inventory.add(item_id, take);
                                // Batch pickup: also grab nearby matching items within 2 tiles
                                let mut extra = 0u16;
                                let mut gi2 = 0;
                                while gi2 < self.ground_items.len() {
                                    let gi = &self.ground_items[gi2];
                                    if gi.stack.item_id == item_id {
                                        let d = (gi.x - pleb.x).powi(2) + (gi.y - pleb.y).powi(2);
                                        if d < 4.0 {
                                            // within 2 tiles
                                            let t = gi.stack.count;
                                            extra += t;
                                            pleb.inventory.add(item_id, t);
                                            self.ground_items.remove(gi2);
                                            continue;
                                        }
                                    }
                                    gi2 += 1;
                                }
                                let total = take + extra;
                                let name = ItemRegistry::cached().name(item_id);
                                events.push(GameEventKind::PickedUp {
                                    pleb: pleb.name.clone(),
                                    count: total,
                                    item: name.to_string(),
                                });
                                pleb.harvest_target = None;
                            } else {
                                // Item gone
                                pleb.harvest_target = None;
                                pleb.haul_target = None;
                                pleb.activity = PlebActivity::Idle;
                            }
                            // If we picked something up, walk to delivery target
                            if pleb.inventory.is_carrying() {
                                if let Some((cx, cy)) = pleb.haul_target {
                                    let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                                    let adj = adjacent_walkable(&self.grid_data, cx, cy)
                                        .unwrap_or((cx, cy));
                                    let path = astar_path_terrain_wd(
                                        &self.grid_data,
                                        &self.wall_data,
                                        &self.terrain_data,
                                        start,
                                        adj,
                                    );
                                    if !path.is_empty() {
                                        pleb.path = path;
                                        pleb.path_idx = 0;
                                    } else {
                                        pleb.activity = PlebActivity::Idle;
                                    }
                                } else {
                                    pleb.activity = PlebActivity::Idle;
                                }
                            }
                        } else {
                            pleb.activity = PlebActivity::Idle;
                        }
                    }
                    // Phase 2: carrying item, at delivery location, path done → deliver
                    else if at_delivery
                        && pleb.inventory.is_carrying()
                        && pleb.path_idx >= pleb.path.len()
                    {
                        if let Some((cx, cy)) = pleb.haul_target {
                            let is_blueprint = self.blueprints.contains_key(&(cx, cy));
                            let bp_is_roof = self
                                .blueprints
                                .get(&(cx, cy))
                                .map_or(false, |bp| bp.is_roof());
                            let is_crate = {
                                let ci = (cy as u32 * GRID_W + cx as u32) as usize;
                                ci < self.grid_data.len()
                                    && block_type_rs(self.grid_data[ci]) == BT_CRATE
                            };
                            if is_blueprint {
                                // Deliver materials to blueprint
                                if let Some(bp) = self.blueprints.get_mut(&(cx, cy)) {
                                    if bp.wood_delivered < bp.wood_needed {
                                        let have = pleb.inventory.wood() as u32;
                                        let deliver = have.min(bp.wood_needed - bp.wood_delivered);
                                        bp.wood_delivered += deliver;
                                        // Consume logs first, then wood
                                        let mut remaining = deliver as u16;
                                        let log_take =
                                            remaining.min(pleb.inventory.count_of(ITEM_LOG) as u16);
                                        if log_take > 0 {
                                            pleb.inventory.remove(ITEM_LOG, log_take);
                                            remaining -= log_take;
                                        }
                                        if remaining > 0 {
                                            pleb.inventory.remove(ITEM_WOOD, remaining);
                                        }
                                        if deliver > 0 {
                                            events.push(GameEventKind::Delivered {
                                                pleb: pleb.name.clone(),
                                                material: "wood",
                                                amount: deliver,
                                            });
                                        }
                                    }
                                    if bp.clay_delivered < bp.clay_needed {
                                        let have = pleb.inventory.count_of(ITEM_CLAY) as u32;
                                        let deliver = have.min(bp.clay_needed - bp.clay_delivered);
                                        bp.clay_delivered += deliver;
                                        pleb.inventory.remove(ITEM_CLAY, deliver as u16);
                                        if deliver > 0 {
                                            events.push(GameEventKind::Delivered {
                                                pleb: pleb.name.clone(),
                                                material: "clay",
                                                amount: deliver,
                                            });
                                        }
                                    }
                                    if bp.plank_delivered < bp.plank_needed {
                                        let have = pleb.inventory.count_of(ITEM_PLANK) as u32;
                                        let deliver =
                                            have.min(bp.plank_needed - bp.plank_delivered);
                                        bp.plank_delivered += deliver;
                                        pleb.inventory.remove(ITEM_PLANK, deliver as u16);
                                        if deliver > 0 {
                                            events.push(GameEventKind::Delivered {
                                                pleb: pleb.name.clone(),
                                                material: "planks",
                                                amount: deliver,
                                            });
                                        }
                                    }
                                    if bp.rock_delivered < bp.rock_needed {
                                        let have = pleb.inventory.count_of(ITEM_ROCK) as u32;
                                        let deliver = have.min(bp.rock_needed - bp.rock_delivered);
                                        bp.rock_delivered += deliver;
                                        pleb.inventory.remove(ITEM_ROCK, deliver as u16);
                                        if deliver > 0 {
                                            events.push(GameEventKind::Delivered {
                                                pleb: pleb.name.clone(),
                                                material: "rock",
                                                amount: deliver,
                                            });
                                        }
                                    }
                                    if bp.rope_delivered < bp.rope_needed {
                                        let have = pleb.inventory.count_of(ITEM_ROPE) as u32;
                                        let deliver = have.min(bp.rope_needed - bp.rope_delivered);
                                        bp.rope_delivered += deliver;
                                        pleb.inventory.remove(ITEM_ROPE, deliver as u16);
                                        if deliver > 0 {
                                            events.push(GameEventKind::Delivered {
                                                pleb: pleb.name.clone(),
                                                material: "rope",
                                                amount: deliver,
                                            });
                                        }
                                    }
                                }
                                self.active_work.remove(&(cx, cy));
                                pleb.haul_target = None;
                                // If blueprint now has all resources, start building immediately
                                // (prevents work zone from stealing the pleb)
                                let start_building =
                                    self.blueprints.get(&(cx, cy)).map_or(false, |bp| {
                                        if bp.is_roof() {
                                            pleb.inventory.count_of(ITEM_FIBER) >= 1
                                        } else if bp.is_campfire() {
                                            pleb.inventory.count_of(ITEM_SCRAP_WOOD) >= 3
                                        } else {
                                            bp.resources_met()
                                        }
                                    });
                                let bp_is_campfire = self
                                    .blueprints
                                    .get(&(cx, cy))
                                    .map_or(false, |bp| bp.is_campfire());
                                if start_building {
                                    if bp_is_roof {
                                        pleb.inventory.remove(ITEM_FIBER, 1);
                                    } else if bp_is_campfire {
                                        pleb.inventory.remove(ITEM_SCRAP_WOOD, 3);
                                    }
                                    pleb.activity = PlebActivity::Building(0.0);
                                    pleb.work_target = Some((cx, cy));
                                    self.active_work.insert((cx, cy));
                                } else {
                                    // Not enough special materials — fetch more immediately
                                    let fetch_id = if bp_is_campfire {
                                        ITEM_SCRAP_WOOD
                                    } else {
                                        ITEM_FIBER
                                    };
                                    // Find nearest matching ground item
                                    let mut nearest: Option<(i32, i32, f32)> = None;
                                    for gi in self.ground_items.iter() {
                                        if gi.stack.item_id == fetch_id {
                                            let d = (gi.x - cx as f32 - 0.5).powi(2)
                                                + (gi.y - cy as f32 - 0.5).powi(2);
                                            if nearest.map_or(true, |(_, _, bd)| d < bd) {
                                                nearest = Some((
                                                    gi.x.floor() as i32,
                                                    gi.y.floor() as i32,
                                                    d,
                                                ));
                                            }
                                        }
                                    }
                                    if let Some((gx, gy, _)) = nearest {
                                        let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                                        let path = astar_path_terrain_wd(
                                            &self.grid_data,
                                            &self.wall_data,
                                            &self.terrain_data,
                                            start,
                                            (gx, gy),
                                        );
                                        if !path.is_empty() {
                                            pleb.path = path;
                                            pleb.path_idx = 0;
                                            pleb.activity = PlebActivity::Hauling;
                                            pleb.harvest_target = Some((gx, gy));
                                            pleb.haul_target = Some((cx, cy));
                                        } else {
                                            pleb.activity = PlebActivity::Idle;
                                            pleb.work_target = None;
                                        }
                                    } else {
                                        pleb.activity = PlebActivity::Idle;
                                        pleb.work_target = None;
                                    }
                                }
                            } else if is_crate {
                                // Deposit all carried items in crate (preserves liquid on containers)
                                let cidx = cy as u32 * GRID_W + cx as u32;
                                let inv = self.crate_contents.entry(cidx).or_default();
                                let carried: Vec<ItemStack> =
                                    pleb.inventory.stacks.drain(..).collect();
                                for stack in carried {
                                    if !inv.add_stack(stack.clone()) {
                                        // Crate full for this item — put back in pleb inventory
                                        pleb.inventory.add_stack(stack);
                                    }
                                }
                                // Sync crate visual
                                if let Some(inv) = self.crate_contents.get(&cidx) {
                                    let count = inv.total().min(CRATE_MAX_ITEMS) as u8;
                                    let ci = cidx as usize;
                                    if ci < self.grid_data.len()
                                        && (self.grid_data[ci] & 0xFF) == BT_CRATE
                                    {
                                        self.grid_data[ci] = (self.grid_data[ci] & 0xFFFF00FF)
                                            | ((count as u32) << 8);
                                        self.grid_dirty = true;
                                    }
                                }
                                if !pleb.inventory.is_carrying() {
                                    // All deposited successfully
                                    pleb.haul_target = None;
                                    pleb.activity = PlebActivity::Idle;
                                    events.push(GameEventKind::Deposited(pleb.name.clone()));
                                } else {
                                    // Crate full, still carrying — try another crate or drop at storage zone
                                    let px = pleb.x.floor() as i32;
                                    let py = pleb.y.floor() as i32;
                                    let alt_crate = find_nearest_crate(&self.grid_data, px, py)
                                        .filter(|&(ax, ay)| ax != cx || ay != cy); // skip the full one
                                    if let Some((ax, ay)) = alt_crate {
                                        // Redirect to another crate
                                        pleb.haul_target = Some((ax, ay));
                                        let start = (px, py);
                                        let adj = adjacent_walkable(&self.grid_data, ax, ay)
                                            .unwrap_or((ax, ay));
                                        let path = astar_path_terrain_wd(
                                            &self.grid_data,
                                            &self.wall_data,
                                            &self.terrain_data,
                                            start,
                                            adj,
                                        );
                                        if !path.is_empty() {
                                            pleb.path = path;
                                            pleb.path_idx = 0;
                                        } else {
                                            pleb.haul_target = None;
                                            pleb.activity = PlebActivity::Idle;
                                        }
                                    } else {
                                        // No other crate — drop remaining items on ground
                                        for stack in pleb.inventory.stacks.drain(..) {
                                            self.ground_items.push(resources::GroundItem {
                                                x: cx as f32 + 0.5,
                                                y: cy as f32 + 0.5,
                                                stack,
                                            });
                                        }
                                        pleb.haul_target = None;
                                        pleb.activity = PlebActivity::Idle;
                                        events.push(GameEventKind::Dropped(pleb.name.clone()));
                                    }
                                }
                            } else {
                                // Drop at storage zone tile
                                for stack in pleb.inventory.stacks.drain(..) {
                                    self.ground_items.push(resources::GroundItem {
                                        x: cx as f32 + 0.5,
                                        y: cy as f32 + 0.5,
                                        stack,
                                    });
                                }
                                pleb.haul_target = None;
                                pleb.activity = PlebActivity::Idle;
                                events.push(GameEventKind::Stored(pleb.name.clone()));
                            }
                        }
                    } else if pleb.path_idx >= pleb.path.len() && !at_pickup && !at_delivery {
                        // Path ended but not at target — repath to the right destination
                        let target = if pleb.inventory.is_carrying() {
                            pleb.haul_target
                        } else {
                            pleb.harvest_target
                        };
                        if let Some((tx2, ty2)) = target {
                            let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                            let path = astar_path_terrain_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                start,
                                (tx2, ty2),
                            );
                            if !path.is_empty() {
                                pleb.path = path;
                                pleb.path_idx = 0;
                            } else {
                                pleb.activity = PlebActivity::Idle;
                            }
                        } else {
                            pleb.activity = PlebActivity::Idle;
                        }
                    }
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
                        let path = astar_path_terrain_wd(
                            &self.grid_data,
                            &self.wall_data,
                            &self.terrain_data,
                            start,
                            target,
                        );
                        if !path.is_empty() {
                            pleb.path = path;
                            pleb.path_idx = 0;
                        }
                    }
                }
                if pleb.needs.breathing_state == BreathingState::Normal {
                    pleb.needs.flee_target = None;
                }

                // Apply knockback velocity (from explosions)
                if pleb.knockback_vx.abs() + pleb.knockback_vy.abs() > 0.01 {
                    let kx = pleb.x + pleb.knockback_vx * dt;
                    let ky = pleb.y + pleb.knockback_vy * dt;
                    if is_walkable_pos_wd(&self.grid_data, &self.wall_data, kx, ky) {
                        pleb.x = kx;
                        pleb.y = ky;
                    }
                    pleb.knockback_vx *= (1.0 - 5.0 * dt).max(0.0);
                    pleb.knockback_vy *= (1.0 - 5.0 * dt).max(0.0);
                }

                // Stagger tick
                if let PlebActivity::Staggering(remaining) = pleb.activity {
                    if remaining - dt <= 0.0 {
                        pleb.activity = PlebActivity::Idle;
                    } else {
                        pleb.activity = PlebActivity::Staggering(remaining - dt);
                    }
                }

                // Command queue: if pleb just became idle and has queued commands, execute next
                if pleb.activity == PlebActivity::Idle && !pleb.command_queue.is_empty() {
                    let cmd = pleb.command_queue.remove(0);
                    let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                    match cmd {
                        PlebCommand::MoveTo(wx, wy) => {
                            let goal = (wx.floor() as i32, wy.floor() as i32);
                            let path = astar_path_terrain_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                start,
                                goal,
                            );
                            if !path.is_empty() {
                                pleb.path = path;
                                pleb.path_idx = 1;
                                pleb.activity = PlebActivity::Walking;
                                pleb.work_target = None;
                                pleb.harvest_target = None;
                                pleb.haul_target = None;
                            }
                        }
                        PlebCommand::Harvest(hx, hy) => {
                            let adj =
                                adjacent_walkable(&self.grid_data, hx, hy).unwrap_or((hx, hy));
                            let path = astar_path_terrain_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                start,
                                adj,
                            );
                            if !path.is_empty() {
                                pleb.path = path;
                                pleb.path_idx = 0;
                                pleb.activity = PlebActivity::Walking;
                                pleb.work_target = Some((hx, hy));
                                pleb.harvest_target = None;
                                pleb.haul_target = None;
                            }
                        }
                        PlebCommand::Haul(hx, hy) => {
                            if let Some((cx, cy)) = find_nearest_crate(&self.grid_data, hx, hy) {
                                let path = astar_path_terrain_wd(
                                    &self.grid_data,
                                    &self.wall_data,
                                    &self.terrain_data,
                                    start,
                                    (hx, hy),
                                );
                                if !path.is_empty() {
                                    pleb.path = path;
                                    pleb.path_idx = 0;
                                    pleb.activity = PlebActivity::Hauling;
                                    pleb.haul_target = Some((cx, cy));
                                    pleb.harvest_target = Some((hx, hy));
                                }
                            }
                        }
                        PlebCommand::Eat(ix, iy) => {
                            let dist = ((pleb.x - ix as f32 - 0.5).powi(2)
                                + (pleb.y - iy as f32 - 0.5).powi(2))
                            .sqrt();
                            if dist < 1.5 {
                                pleb.harvest_target = Some((ix, iy));
                                pleb.activity = PlebActivity::Eating;
                                pleb.work_target = None;
                                pleb.haul_target = None;
                                pleb.path.clear();
                            } else {
                                let path = astar_path_terrain_wd(
                                    &self.grid_data,
                                    &self.wall_data,
                                    &self.terrain_data,
                                    start,
                                    (ix, iy),
                                );
                                if !path.is_empty() {
                                    pleb.path = path;
                                    pleb.path_idx = 0;
                                    pleb.activity = PlebActivity::Walking;
                                    pleb.harvest_target = Some((ix, iy));
                                    pleb.work_target = None;
                                    pleb.haul_target = None;
                                }
                            }
                        }
                        PlebCommand::DigClay(dx, dy) => {
                            let path = astar_path_terrain_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                start,
                                (dx, dy),
                            );
                            if !path.is_empty() {
                                pleb.path = path;
                                pleb.path_idx = 0;
                                pleb.activity = PlebActivity::Walking;
                                pleb.work_target = Some((dx, dy));
                                pleb.harvest_target = None;
                                pleb.haul_target = None;
                            }
                        }
                        PlebCommand::HandCraft(recipe_id) => {
                            let recipe_reg = recipe_defs::RecipeRegistry::cached();
                            if let Some(recipe) = recipe_reg.get(recipe_id) {
                                let can = recipe.inputs.iter().all(|ing| {
                                    pleb.inventory.count_of(ing.item) >= ing.count as u32
                                });
                                if can {
                                    for ing in &recipe.inputs {
                                        pleb.inventory.remove(ing.item, ing.count);
                                    }
                                    pleb.activity = PlebActivity::Crafting(recipe_id, 0.0);
                                    pleb.path.clear();
                                    pleb.path_idx = 0;
                                }
                            }
                        }
                        PlebCommand::GatherBranches(gx, gy) => {
                            let adj =
                                adjacent_walkable(&self.grid_data, gx, gy).unwrap_or((gx, gy));
                            let path = astar_path_terrain_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                start,
                                adj,
                            );
                            if !path.is_empty() {
                                pleb.path = path;
                                pleb.path_idx = 0;
                                pleb.activity = PlebActivity::Walking;
                                pleb.work_target = Some((gx, gy));
                                pleb.harvest_target = Some((gx, gy));
                                pleb.haul_target = None;
                            }
                        }
                    }
                }

                pleb.prev_x = pleb.x;
                pleb.prev_y = pleb.y;
            }
        }

        // --- Physical door tick ---
        {
            const WIND_TORQUE_SCALE: f32 = 0.008;
            const DOOR_DAMPING: f32 = 3.0;
            const LATCH_SPRING: f32 = 20.0;
            const DOOR_PROXIMITY_SQ: f32 = 1.44; // 1.2^2
            const DOOR_PUSH_IMPULSE: f32 = 3.0;
            const DOOR_BOUNCE_RESTITUTION: f32 = 0.3;
            const DOOR_SLAM_DB_BASE: f32 = 50.0;
            const DOOR_SLAM_DB_FACTOR: f32 = 5.0;
            const DOOR_SLAM_DB_MAX_BONUS: f32 = 30.0;
            const DOOR_SLAM_VEL_THRESHOLD: f32 = 2.0;

            let wind_mag = self.camera.wind_magnitude;
            let wind_angle = self.camera.wind_angle;

            // Pleb positions for door interaction
            let pleb_positions: Vec<(f32, f32)> = self.plebs.iter().map(|p| (p.x, p.y)).collect();

            for door in &mut self.doors {
                // Pleb proximity: push door open
                let door_cx = door.x as f32 + 0.5;
                let door_cy = door.y as f32 + 0.5;
                let pleb_nearby = pleb_positions.iter().any(|&(px, py)| {
                    (door_cx - px).powi(2) + (door_cy - py).powi(2) < DOOR_PROXIMITY_SQ
                });
                if pleb_nearby && !door.is_passable() && !door.locked {
                    door.angular_vel += DOOR_PUSH_IMPULSE * dt;
                }

                if door.locked {
                    // Locked doors don't move (but can still be pushed by plebs above)
                    door.angular_vel = 0.0;
                } else {
                    // Wind torque: depends on angle between wind and door normal
                    let edge_normal = match door.edge {
                        0 => std::f32::consts::FRAC_PI_2,  // N: normal points south (+y)
                        1 => 0.0,                          // E: normal points west (-x)
                        2 => -std::f32::consts::FRAC_PI_2, // S: normal points north (-y)
                        _ => std::f32::consts::PI,         // W: normal points east (+x)
                    };
                    let hinge_sign = if door.hinge_side == 0 { 1.0f32 } else { -1.0 };
                    let effective_normal = edge_normal + door.angle * hinge_sign;
                    let angle_diff = wind_angle - effective_normal;
                    let torque = wind_mag * angle_diff.sin() * door.angle.cos() * WIND_TORQUE_SCALE;

                    // Damping (hinge friction)
                    let damping = -door.angular_vel * DOOR_DAMPING;

                    // Latch spring: snaps shut when nearly closed and slow
                    let latch = if door.angle < 0.05 && door.angular_vel.abs() < 0.3 {
                        -door.angle * LATCH_SPRING
                    } else {
                        0.0
                    };

                    // Integrate
                    let accel = torque + damping + latch;
                    door.angular_vel += accel * dt;
                }

                door.angle += door.angular_vel * dt;

                // Clamp to [0, max_angle]
                if door.angle <= 0.0 {
                    let slam_vel = door.angular_vel.abs();
                    door.angle = 0.0;
                    door.angular_vel = (-door.angular_vel * DOOR_BOUNCE_RESTITUTION).max(0.0);
                    if slam_vel > DOOR_SLAM_VEL_THRESHOLD && self.sound_enabled {
                        let db = DOOR_SLAM_DB_BASE
                            + (slam_vel * DOOR_SLAM_DB_FACTOR).min(DOOR_SLAM_DB_MAX_BONUS);
                        self.sound_sources.push(SoundSource {
                            x: door_cx,
                            y: door_cy,
                            amplitude: db_to_amplitude(db),
                            frequency: 0.0,
                            phase: 0.0,
                            pattern: 0,
                            duration: 0.05,
                        });
                    }
                }
                if door.angle >= DOOR_MAX_ANGLE {
                    door.angle = DOOR_MAX_ANGLE;
                    door.angular_vel = 0.0; // wall stop
                }

                // Sync WD_DOOR_OPEN bit — only set grid_dirty if it actually changed
                let didx = (door.y as u32 * GRID_W + door.x as u32) as usize;
                if didx < self.wall_data.len() {
                    let was_open = (self.wall_data[didx] & WD_DOOR_OPEN) != 0;
                    let now_open = door.is_passable();
                    if was_open != now_open {
                        if now_open {
                            self.wall_data[didx] |= WD_DOOR_OPEN;
                        } else {
                            self.wall_data[didx] &= !WD_DOOR_OPEN;
                        }
                        self.grid_dirty = true;
                    }
                }
            }
            // Always upload door angles for rendering (tiny buffer, no grid rebuild needed)
            self.doors_dirty = !self.doors.is_empty();
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

        // (Auto-close replaced by physical door tick above — doors swing shut via damping/latch)

        // --- Physics tick ---
        {
            let sel_pleb = self.selected_pleb.and_then(|i| self.plebs.get(i));
            let pleb_data = sel_pleb.map(|p| (p.x, p.y, 0.0f32, 0.0f32, p.angle));
            // Collect pleb positions for bullet collision
            let pleb_positions: Vec<(f32, f32, usize)> = self
                .plebs
                .iter()
                .enumerate()
                .map(|(i, p)| (p.x, p.y, i))
                .collect();
            // Extract sound source data for physics body force coupling
            let sound_data: Vec<(f32, f32, f32)> = self
                .sound_sources
                .iter()
                .map(|s| (s.x, s.y, s.amplitude))
                .collect();
            let (impacts, bullet_hits, explosion_events) = tick_bodies(
                &mut self.physics_bodies,
                dt,
                &self.grid_data,
                &self.wall_data,
                self.fluid_params.wind_x,
                self.fluid_params.wind_y,
                pleb_data,
                &pleb_positions,
                self.selected_pleb,
                self.enable_ricochets,
                &sound_data,
            );

            // Apply bullet hits to plebs (data-driven damage)
            for hit in &bullet_hits {
                if let Some(pleb) = self.plebs.get_mut(hit.pleb_idx) {
                    // Look up damage from the projectile that hit (scan bodies for the source)
                    // For now use a fixed lookup since bullets are the only hitscan type
                    let dmg = projectile_def(PROJ_BULLET).hit_damage;
                    events.push(GameEventKind::PlebHit {
                        pleb: pleb.name.clone(),
                        hp_pct: (pleb.needs.health - dmg).max(0.0) * 100.0,
                    });
                    pleb.needs.health -= dmg;
                    self.fluid_params.splat_x = hit.x;
                    self.fluid_params.splat_y = hit.y;
                    self.fluid_params.splat_radius = 0.3;
                    self.fluid_params.splat_active = 1.0;
                }
            }

            // Handle projectile impacts — data-driven sound, smoke, gas emission
            for impact in &impacts {
                let def = projectile_def(impact.projectile_id);

                if impact.destroy_block {
                    self.destroy_block_at(impact.block_x, impact.block_y);
                    log::info!(
                        "Projectile destroyed block at ({}, {}) KE={:.0}",
                        impact.block_x,
                        impact.block_y,
                        impact.kinetic_energy
                    );
                }

                // Impact sound
                if def.impact.sound_db > 0.0 && self.sound_enabled {
                    self.sound_sources.push(SoundSource {
                        x: impact.x,
                        y: impact.y,
                        amplitude: db_to_amplitude(def.impact.sound_db),
                        frequency: 0.0,
                        phase: 0.0,
                        pattern: 0,
                        duration: def.impact.sound_duration,
                    });
                }

                // Impact smoke splat
                if def.impact.smoke_radius > 0.0 {
                    self.fluid_params.splat_x = impact.x;
                    self.fluid_params.splat_y = impact.y;
                    self.fluid_params.splat_vx = 0.0;
                    self.fluid_params.splat_vy = 0.0;
                    self.fluid_params.splat_radius = def.impact.smoke_radius;
                    self.fluid_params.splat_active = 1.0;
                }

                // Fuse gas emission (written to dye texture in render pass)
                if def.fuse.is_some() {
                    self.grenade_impacts.push((impact.x, impact.y));
                }
            }

            // Process explosion events — blast force, knockback, sound, fluid burst
            for expl in &explosion_events {
                let radius = expl.def.radius;
                let force = expl.def.force;

                // Push physics bodies outward
                for body in &mut self.physics_bodies {
                    let dx = body.x - expl.x;
                    let dy = body.y - expl.y;
                    let dist = (dx * dx + dy * dy).sqrt().max(0.3);
                    if dist > radius {
                        continue;
                    }
                    let falloff = 1.0 / (dist * dist);
                    let impulse = force * falloff / body.mass;
                    let nx = dx / dist;
                    let ny = dy / dist;
                    body.vx += nx * impulse;
                    body.vy += ny * impulse;
                    body.vz += impulse * 0.3; // upward kick
                    body.spin_x += ny * impulse * 0.2;
                    body.spin_y -= nx * impulse * 0.2;
                }

                // Knock back plebs
                for pleb in &mut self.plebs {
                    let dx = pleb.x - expl.x;
                    let dy = pleb.y - expl.y;
                    let dist = (dx * dx + dy * dy).sqrt().max(0.5);
                    if dist > radius {
                        continue;
                    }
                    let falloff = 1.0 / (dist * dist);
                    let impulse = force * falloff * 0.5;
                    let nx = dx / dist;
                    let ny = dy / dist;
                    pleb.knockback_vx += nx * impulse;
                    pleb.knockback_vy += ny * impulse;
                    // Stagger if close enough
                    if dist < radius * 0.5 && !pleb.activity.is_crisis() {
                        pleb.activity = PlebActivity::Staggering(0.6);
                        pleb.path.clear();
                    }
                    // Explosion damage (falls off with distance)
                    if expl.def.damage > 0.0 {
                        let dmg = expl.def.damage / (dist * dist).max(1.0);
                        pleb.needs.health = (pleb.needs.health - dmg).max(0.0);
                    }
                }

                // Explosion sound
                if expl.def.sound_db > 0.0 && self.sound_enabled {
                    self.sound_sources.push(SoundSource {
                        x: expl.x,
                        y: expl.y,
                        amplitude: db_to_amplitude(expl.def.sound_db),
                        frequency: 0.0,
                        phase: 0.0,
                        pattern: 0,
                        duration: expl.def.sound_duration,
                    });
                }

                // Fluid burst (expanding pressure wave)
                self.fluid_params.splat_x = expl.x;
                self.fluid_params.splat_y = expl.y;
                self.fluid_params.splat_vx = 0.0;
                self.fluid_params.splat_vy = 0.0;
                self.fluid_params.splat_radius = 4.0;
                self.fluid_params.splat_active = 1.0;

                events.push(GameEventKind::Explosion(expl.x, expl.y));

                // Blow open nearby doors
                for door in &mut self.doors {
                    if door.locked {
                        continue;
                    }
                    let dx = door.x as f32 + 0.5 - expl.x;
                    let dy = door.y as f32 + 0.5 - expl.y;
                    let dist = (dx * dx + dy * dy).sqrt().max(0.5);
                    if dist > radius {
                        continue;
                    }
                    let impulse = force * (1.0 - dist / radius) / dist.max(0.5);
                    // Push toward open (positive angular velocity)
                    door.angular_vel += impulse * 0.5;
                }
            }
        }

        // --- Crop growth ---
        if !self.time_paused {
            let grow_dt = dt * self.time_speed;
            let mut matured = Vec::new();
            for (&grid_idx, timer) in self.crop_timers.iter_mut() {
                let idx = grid_idx as usize;
                if idx >= self.grid_data.len() {
                    continue;
                }
                let block = self.grid_data[idx];
                let bt = block & 0xFF;
                let stage = block_height_rs(block) as u32;
                if bt != BT_CROP || stage >= CROP_MATURE {
                    continue;
                }

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
                let wt = if idx < self.water_table.len() {
                    self.water_table[idx]
                } else {
                    -3.0
                };
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

                // Soil richness from terrain data (0-31 → 0.3-1.3 multiplier)
                let richness_factor = if idx < self.terrain_data.len() {
                    let richness = terrain_richness(self.terrain_data[idx]);
                    0.3 + (richness as f32 / 31.0) * 1.0
                } else {
                    1.0
                };

                // Per-tile randomness: 0.7-1.3 variation (deterministic from position)
                let hash = (grid_idx
                    .wrapping_mul(2654435761)
                    .wrapping_add(stage * 1013904223))
                    & 0xFFFF;
                let random_factor = 0.7 + (hash as f32 / 65535.0) * 0.6;

                let growth_rate =
                    temp_factor * sun_factor * water_factor * random_factor * richness_factor;
                *timer += grow_dt * growth_rate;
                if *timer >= CROP_GROW_TIME {
                    *timer = 0.0;
                    let new_stage = (stage + 1).min(CROP_MATURE);
                    let roof_h = block & 0xFF000000;
                    let flags_bits = block_flags_rs(block) as u32;
                    self.grid_data[idx] =
                        make_block(BT_CROP as u8, new_stage as u8, flags_bits as u8) | roof_h;
                    self.grid_dirty = true;
                    if new_stage == CROP_MATURE {
                        matured.push(grid_idx);
                    }
                }
            }
            // Remove timers for matured crops
            for idx in matured {
                self.crop_timers.remove(&idx);
            }
        }

        // --- Terrain compaction decay (natural path fading) ---
        // Decay a batch of random tiles each frame so unused paths slowly fade
        if !self.time_paused && self.frame_count % 30 == 0 {
            let grid_size = self.terrain_data.len();
            if grid_size > 0 {
                // Decay 64 random tiles per tick (covers full map in ~3000 frames)
                // Rain accelerates decay (2x-4x based on intensity)
                let rain_bonus = if self.camera.rain_intensity > 0.3 {
                    2u32
                } else {
                    0
                };
                let samples = 64 + rain_bonus * 64; // more tiles when raining
                for k in 0..samples {
                    let hash = self
                        .frame_count
                        .wrapping_mul(2654435761)
                        .wrapping_add(k * 1013904223);
                    let idx = (hash as usize) % grid_size;
                    terrain_decay_compaction(&mut self.terrain_data[idx]);
                    // Slowly heal dig holes (1 in 8 chance per sample when raining, 1 in 30 otherwise)
                    if terrain_dig_holes(self.terrain_data[idx]) > 0 {
                        let heal_chance = if self.camera.rain_intensity > 0.3 {
                            8
                        } else {
                            30
                        };
                        if (hash >> 8) % heal_chance == 0 {
                            terrain_remove_dig_hole(&mut self.terrain_data[idx]);
                        }
                    }
                }
                self.terrain_dirty = true;
            }
        }

        // --- Work queue: assign idle friendly plebs to tasks by priority ---
        {
            let mut farm_tasks =
                generate_work_tasks(&self.zones, &self.grid_data, GRID_W, &self.active_work);
            for task in self.manual_tasks.drain(..) {
                let pos = task.position();
                if !self.active_work.contains(&pos) {
                    farm_tasks.push(task);
                }
            }

            // Collect workbenches/kilns with pending craft orders
            let craft_stations: Vec<(i32, i32, u32)> = self
                .craft_queues
                .iter()
                .filter(|(_, q)| q.pending())
                .filter_map(|(&gidx, _)| {
                    let x = (gidx % GRID_W) as i32;
                    let y = (gidx / GRID_W) as i32;
                    if !self.active_work.contains(&(x, y)) {
                        Some((x, y, gidx))
                    } else {
                        None
                    }
                })
                .collect();

            // Collect ground items that could be hauled (with a nearby crate)
            // Skip items already in a storage zone — they're considered "stored"
            let storage_tiles: std::collections::HashSet<(i32, i32)> = self
                .zones
                .iter()
                .filter(|z| z.kind == ZoneKind::Storage)
                .flat_map(|z| z.tiles.iter().copied())
                .collect();
            let haul_candidates: Vec<(i32, i32)> = self
                .ground_items
                .iter()
                .map(|item| (item.x.floor() as i32, item.y.floor() as i32))
                .filter(|&(ix, iy)| !self.active_work.contains(&(ix, iy)))
                .filter(|&(ix, iy)| !storage_tiles.contains(&(ix, iy)))
                .filter(|&(ix, iy)| find_nearest_crate(&self.grid_data, ix, iy).is_some())
                .collect();

            for pleb in self.plebs.iter_mut() {
                if pleb.is_enemy || pleb.is_dead {
                    continue;
                }
                if pleb.activity != PlebActivity::Idle {
                    continue;
                }
                if pleb.work_target.is_some() || pleb.haul_target.is_some() {
                    continue;
                }

                // Try work types in priority order (1 first, then 2, then 3)
                let mut assigned = false;
                for priority_level in 1..=3u8 {
                    if assigned {
                        break;
                    }
                    // Collect which work types this pleb has at this priority level
                    for wt in 0..zones::WORK_TYPE_COUNT {
                        if pleb.work_priorities[wt] != priority_level {
                            continue;
                        }
                        match wt {
                            zones::WORK_FARM => {
                                // Find nearest farm task (store index to avoid clone)
                                let mut best: Option<(usize, f32)> = None;
                                for (i, task) in farm_tasks.iter().enumerate() {
                                    let (tx, ty) = task.position();
                                    let dist = (pleb.x - tx as f32 - 0.5).powi(2)
                                        + (pleb.y - ty as f32 - 0.5).powi(2);
                                    if best.map_or(true, |(_, bd)| dist < bd) {
                                        best = Some((i, dist));
                                    }
                                }
                                if let Some((task_idx, _)) = best {
                                    let task = &farm_tasks[task_idx];
                                    let (tx, ty) = task.position();
                                    let task_name = match task {
                                        WorkTask::Plant(_, _) => "plant",
                                        WorkTask::Harvest(_, _) => "harvest",
                                    };
                                    events.push(GameEventKind::TaskAssigned {
                                        pleb: pleb.name.clone(),
                                        task: task_name,
                                        x: tx,
                                        y: ty,
                                    });
                                    self.active_work.insert((tx, ty));
                                    pleb.work_target = Some((tx, ty));
                                    let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                                    let path = astar_path_terrain_wd(
                                        &self.grid_data,
                                        &self.wall_data,
                                        &self.terrain_data,
                                        start,
                                        (tx, ty),
                                    );
                                    if !path.is_empty() {
                                        pleb.path = path;
                                        pleb.path_idx = 0;
                                        pleb.activity = PlebActivity::Walking;
                                    }
                                    assigned = true;
                                }
                            }
                            zones::WORK_HAUL => {
                                // Find nearest ground item to haul
                                let mut best: Option<((i32, i32), f32)> = None;
                                for &(ix, iy) in &haul_candidates {
                                    let dist = (pleb.x - ix as f32 - 0.5).powi(2)
                                        + (pleb.y - iy as f32 - 0.5).powi(2);
                                    if best.map_or(true, |(_, bd)| dist < bd) {
                                        best = Some(((ix, iy), dist));
                                    }
                                }
                                if let Some(((ix, iy), _)) = best {
                                    if let Some((cx, cy)) =
                                        find_nearest_crate(&self.grid_data, ix, iy)
                                    {
                                        let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                                        let path = astar_path_terrain_wd(
                                            &self.grid_data,
                                            &self.wall_data,
                                            &self.terrain_data,
                                            start,
                                            (ix, iy),
                                        );
                                        if !path.is_empty() {
                                            pleb.path = path;
                                            pleb.path_idx = 0;
                                            pleb.activity = PlebActivity::Hauling;
                                            pleb.harvest_target = Some((ix, iy));
                                            pleb.haul_target = Some((cx, cy));
                                            self.active_work.insert((ix, iy));
                                            events.push(GameEventKind::AutoHauling(
                                                pleb.name.clone(),
                                            ));
                                            assigned = true;
                                        }
                                    }
                                }
                            }
                            zones::WORK_CRAFT => {
                                // Find nearest workbench/kiln with pending orders AND available ingredients
                                let recipe_reg = recipe_defs::RecipeRegistry::cached();
                                let mut best: Option<((i32, i32, u32), f32)> = None;
                                for &(sx, sy, gidx) in &craft_stations {
                                    // Check if the next order's ingredients are available
                                    let craftable = self
                                        .craft_queues
                                        .get(&gidx)
                                        .and_then(|q| q.next_order())
                                        .and_then(|order| recipe_reg.get(order.recipe_id))
                                        .map(|recipe| {
                                            recipe.inputs.iter().all(|ing| {
                                                let in_inv =
                                                    pleb.inventory.count_of(ing.item) as u16;
                                                let in_crates: u16 = self
                                                    .crate_contents
                                                    .values()
                                                    .map(|c| c.count_of(ing.item) as u16)
                                                    .sum();
                                                let on_ground: u16 = self
                                                    .ground_items
                                                    .iter()
                                                    .filter(|gi| gi.stack.item_id == ing.item)
                                                    .map(|gi| gi.stack.count)
                                                    .sum();
                                                in_inv + in_crates + on_ground >= ing.count
                                            })
                                        })
                                        .unwrap_or(false);
                                    if !craftable {
                                        continue;
                                    }
                                    let dist = (pleb.x - sx as f32 - 0.5).powi(2)
                                        + (pleb.y - sy as f32 - 0.5).powi(2);
                                    if best.map_or(true, |(_, bd)| dist < bd) {
                                        best = Some(((sx, sy, gidx), dist));
                                    }
                                }
                                if let Some(((sx, sy, _gidx), _)) = best {
                                    let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                                    let adj = adjacent_walkable(&self.grid_data, sx, sy)
                                        .unwrap_or((sx, sy));
                                    let path = astar_path_terrain_wd(
                                        &self.grid_data,
                                        &self.wall_data,
                                        &self.terrain_data,
                                        start,
                                        adj,
                                    );
                                    if !path.is_empty() {
                                        pleb.path = path;
                                        pleb.path_idx = 0;
                                        pleb.activity = PlebActivity::Walking;
                                        pleb.work_target = Some((sx, sy));
                                        self.active_work.insert((sx, sy));
                                        events.push(GameEventKind::GoingToCraft(pleb.name.clone()));
                                        assigned = true;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Handle Farming activity: pleb arrived at work target
            for pleb in self.plebs.iter_mut() {
                if pleb.is_enemy {
                    continue;
                }

                // Check if pleb is doing Farming
                if let PlebActivity::Farming(progress) = &pleb.activity {
                    // Speed varies: trees take longer than crops/bushes
                    // Stone axe: 2x tree chopping speed
                    let has_axe = pleb.inventory.count_of(ITEM_STONE_AXE) > 0;
                    // Trees require Stone Axe — cancel if pleb doesn't have one
                    let is_tree_target = pleb.work_target.map_or(false, |(tx, ty)| {
                        let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                        tidx < self.grid_data.len()
                            && (self.grid_data[tidx] & 0xFF) as u32 == BT_TREE
                    });
                    if is_tree_target && !has_axe {
                        pleb.activity = PlebActivity::Idle;
                        pleb.work_target = None;
                        continue;
                    }

                    let speed = if is_tree_target {
                        0.50 // ~2s with axe (required)
                    } else if let Some(_) = pleb.work_target {
                        0.4 // ~2.5s for crops/bushes
                    } else {
                        0.4
                    };
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
                                    let fflags = block_flags_rs(tblock) as u32;
                                    self.grid_data[tidx] =
                                        make_block(BT_CROP as u8, CROP_PLANTED as u8, fflags as u8)
                                            | roof_h;
                                    self.crop_timers.insert(tidx as u32, 0.0);
                                    self.grid_dirty = true;
                                    events.push(GameEventKind::Planted(pleb.name.clone()));
                                } else if tbt == BT_CROP {
                                    let roof_h = tblock & 0xFF000000;
                                    let fflags = block_flags_rs(tblock) as u32;
                                    self.grid_data[tidx] =
                                        make_block(BT_DIRT as u8, 0, fflags as u8) | roof_h;
                                    self.crop_timers.remove(&(tidx as u32));
                                    self.grid_dirty = true;
                                    // Drop harvest on ground near pleb
                                    self.ground_items.push(resources::GroundItem {
                                        x: pleb.x,
                                        y: pleb.y,
                                        stack: ItemStack::new(ITEM_BERRIES, 2),
                                    });
                                    self.ground_items.push(resources::GroundItem {
                                        x: pleb.x + 0.2,
                                        y: pleb.y + 0.2,
                                        stack: ItemStack::new(ITEM_FIBER, 2),
                                    });
                                    events.push(GameEventKind::Harvested {
                                        pleb: pleb.name.clone(),
                                        what: "a crop (berries + fiber)",
                                    });
                                } else if tbt == BT_BERRY_BUSH {
                                    self.ground_items.push(resources::GroundItem {
                                        x: pleb.x,
                                        y: pleb.y,
                                        stack: ItemStack::new(ITEM_BERRIES, 3),
                                    });
                                    events.push(GameEventKind::Harvested {
                                        pleb: pleb.name.clone(),
                                        what: "berries",
                                    });
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
                                            if nx < 0
                                                || ny < 0
                                                || nx >= GRID_W as i32
                                                || ny >= GRID_H as i32
                                            {
                                                continue;
                                            }
                                            let nidx = (ny as u32 * GRID_W + nx as u32) as usize;
                                            if nidx < self.grid_data.len()
                                                && (self.grid_data[nidx] & 0xFF) as u32 == BT_TREE
                                            {
                                                let nroof = self.grid_data[nidx] & 0xFF000000;
                                                let nflags = (self.grid_data[nidx] >> 16) & 2;
                                                self.grid_data[nidx] =
                                                    make_block(BT_DIRT as u8, 0, nflags as u8)
                                                        | nroof;
                                            }
                                        }
                                    }
                                    self.grid_dirty = true;
                                    // Tree drops: 1 heavy log + scattered sticks + fiber
                                    let cx = origin_x as f32 + 1.0;
                                    let cy = origin_y as f32 + 1.0;
                                    // 1 log (big, heavy)
                                    self.ground_items
                                        .push(resources::GroundItem::new(cx, cy, ITEM_LOG, 1));
                                    // Individual sticks scattered around
                                    for i in 0..3u32 {
                                        let angle = i as f32 * 2.1 + 0.5;
                                        let dist = 0.3 + (i as f32) * 0.15;
                                        self.ground_items.push(resources::GroundItem::new(
                                            cx + angle.cos() * dist,
                                            cy + angle.sin() * dist,
                                            ITEM_SCRAP_WOOD,
                                            1,
                                        ));
                                    }
                                    // Fiber
                                    self.ground_items.push(resources::GroundItem::new(
                                        cx - 0.4,
                                        cy + 0.3,
                                        ITEM_FIBER,
                                        2,
                                    ));
                                    events.push(GameEventKind::Harvested {
                                        pleb: pleb.name.clone(),
                                        what: "a tree (log + sticks + fiber)",
                                    });
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

                // Arrived at work target: start farming or crafting
                let has_work = pleb.work_target.is_some();
                let path_done = pleb.path_idx >= pleb.path.len();
                let is_walking_or_idle =
                    pleb.activity == PlebActivity::Walking || pleb.activity == PlebActivity::Idle;
                if has_work && path_done && is_walking_or_idle {
                    if let Some((tx, ty)) = pleb.work_target {
                        let dist = ((pleb.x - tx as f32 - 0.5).powi(2)
                            + (pleb.y - ty as f32 - 0.5).powi(2))
                        .sqrt();
                        if dist < 1.5 {
                            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                            let tbt = if tidx < self.grid_data.len() {
                                block_type_rs(self.grid_data[tidx])
                            } else {
                                0
                            };

                            // Branch gathering: arrived at tree with harvest_target but no axe
                            if tbt == BT_TREE
                                && pleb.harvest_target.is_some()
                                && pleb.inventory.count_of(ITEM_STONE_AXE) == 0
                            {
                                pleb.activity = PlebActivity::Harvesting(0.0);
                                pleb.work_target = None;
                                pleb.path.clear();
                                pleb.path_idx = 0;
                            } else if tbt == BT_WORKBENCH || tbt == BT_KILN || tbt == BT_SAW_HORSE {
                                // Try to start crafting from queue
                                let gidx = ty as u32 * GRID_W + tx as u32;
                                let started = if let Some(queue) = self.craft_queues.get(&gidx) {
                                    if let Some(order) = queue.next_order() {
                                        let recipe_reg = recipe_defs::RecipeRegistry::cached();
                                        if let Some(recipe) = recipe_reg.get(order.recipe_id) {
                                            // Check ingredients from inventory + crates + ground
                                            let mut have_all = true;
                                            for ing in &recipe.inputs {
                                                let in_inv =
                                                    pleb.inventory.count_of(ing.item) as u16;
                                                let in_crates: u16 = self
                                                    .crate_contents
                                                    .values()
                                                    .map(|c| c.count_of(ing.item) as u16)
                                                    .sum();
                                                let on_ground: u16 = self
                                                    .ground_items
                                                    .iter()
                                                    .filter(|gi| gi.stack.item_id == ing.item)
                                                    .map(|gi| gi.stack.count)
                                                    .sum();
                                                if in_inv + in_crates + on_ground < ing.count {
                                                    have_all = false;
                                                    break;
                                                }
                                            }
                                            if have_all {
                                                Some(order.recipe_id)
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };

                                if let Some(recipe_id) = started {
                                    let recipe_reg = recipe_defs::RecipeRegistry::cached();
                                    let recipe = recipe_reg.get(recipe_id).unwrap();
                                    // Consume ingredients from pleb inventory + crates + ground
                                    for ing in &recipe.inputs {
                                        let mut need = ing.count;
                                        let from_inv = pleb.inventory.remove(ing.item, need);
                                        need -= from_inv;
                                        if need > 0 {
                                            for (_, cinv) in self.crate_contents.iter_mut() {
                                                if need == 0 {
                                                    break;
                                                }
                                                let taken = cinv.remove(ing.item, need);
                                                need -= taken;
                                            }
                                        }
                                        // Take remaining from ground items
                                        if need > 0 {
                                            let mut i = 0;
                                            while i < self.ground_items.len() && need > 0 {
                                                if self.ground_items[i].stack.item_id == ing.item {
                                                    let take =
                                                        self.ground_items[i].stack.count.min(need);
                                                    self.ground_items[i].stack.count -= take;
                                                    need -= take;
                                                    if self.ground_items[i].stack.count == 0 {
                                                        self.ground_items.remove(i);
                                                        continue;
                                                    }
                                                }
                                                i += 1;
                                            }
                                        }
                                    }
                                    pleb.activity = PlebActivity::Crafting(recipe_id, 0.0);
                                    events.push(GameEventKind::Crafting {
                                        pleb: pleb.name.clone(),
                                        recipe: recipe.name.clone(),
                                    });
                                } else {
                                    // Can't craft — missing ingredients, release
                                    self.active_work.remove(&(tx, ty));
                                    pleb.work_target = None;
                                    pleb.activity = PlebActivity::Idle;
                                }
                            } else if tbt == BT_WELL {
                                // Start drinking at well
                                pleb.activity = PlebActivity::Drinking(0.0);
                            } else if tbt == BT_DIRT
                                && tidx < self.terrain_data.len()
                                && terrain_dig_holes(self.terrain_data[tidx]) < 7
                            {
                                // Dig earth: add a hole (no elevation change, tile stays BT_DIRT)
                                let is_clay_terrain =
                                    terrain_type(self.terrain_data[tidx]) == TERRAIN_CLAY;
                                terrain_add_dig_hole(&mut self.terrain_data[tidx]);
                                self.terrain_dirty = true;
                                let has_shovel = pleb.inventory.count_of(ITEM_WOODEN_SHOVEL) > 0;
                                let base_yield: u16 = if is_clay_terrain { 4 } else { 2 };
                                let yield_amt = base_yield + if has_shovel { 2 } else { 0 };
                                self.ground_items.push(resources::GroundItem::new(
                                    tx as f32 + 0.5,
                                    ty as f32 + 0.5,
                                    ITEM_CLAY,
                                    yield_amt,
                                ));
                                events.push(GameEventKind::DugClay {
                                    pleb: pleb.name.clone(),
                                    amount: yield_amt,
                                });
                                pleb.work_target = None;
                                pleb.activity = PlebActivity::Idle;
                            } else {
                                pleb.activity = PlebActivity::Farming(0.0);
                            }
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
        let mut wall_placements: Vec<(i32, i32, u16, u16, u16)> = Vec::new();
        let mut dig_marks: Vec<(i32, i32)> = Vec::new(); // tiles to mark as dug (mud wall auto-dig)
        for pleb in self.plebs.iter_mut() {
            if pleb.is_enemy {
                continue;
            }
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
                                if bp.is_roof() {
                                    // Roof blueprint: mark as paid + flash
                                    self.roof_paid[tidx] = true;
                                    self.roof_flash[tidx] = 3.0; // show roof for 3 seconds
                                    self.grid_dirty = true;
                                    events.push(GameEventKind::Built {
                                        pleb: pleb.name.clone(),
                                        block: "roof".to_string(),
                                    });
                                } else if bp.is_wall() {
                                    // Mud walls: auto-dig nearby dirt tile
                                    let bp_bt = (bp.block_data & 0xFF) as u32;
                                    if bp_bt == BT_MUD_WALL {
                                        dig_marks.push((tx, ty));
                                    }
                                    wall_placements.push((
                                        tx,
                                        ty,
                                        bp.wall_edges,
                                        bp.wall_thickness,
                                        bp.wall_material,
                                    ));
                                    self.grid_dirty = true;
                                    events.push(GameEventKind::Built {
                                        pleb: pleb.name.clone(),
                                        block: block_defs::BlockRegistry::cached()
                                            .name(bp.block_data & 0xFF)
                                            .to_string(),
                                    });
                                } else {
                                    let mut placed = bp.block_data;
                                    // Fire blocks: default intensity (height byte)
                                    let placed_bt = (placed & 0xFF) as u32;
                                    if placed_bt == BT_FIREPLACE {
                                        placed = (placed & 0xFFFF00FF) | (5 << 8);
                                    } else if placed_bt == BT_CAMPFIRE {
                                        placed = (placed & 0xFFFF00FF) | (3 << 8);
                                    }
                                    self.grid_data[tidx] = placed;
                                    self.grid_dirty = true;
                                    events.push(GameEventKind::Built {
                                        pleb: pleb.name.clone(),
                                        block: block_defs::BlockRegistry::cached()
                                            .name(bp.block_data & 0xFF)
                                            .to_string(),
                                    });
                                }
                            }
                        }
                        self.active_work.remove(&(tx, ty));
                        // Chain to adjacent blueprint if one exists (build walls sequentially)
                        let mut chained = false;
                        for &(dx, dy) in &[(1i32, 0), (-1, 0), (0, 1), (0, -1)] {
                            let nx = tx + dx;
                            let ny = ty + dy;
                            if let Some(nbp) = self.blueprints.get(&(nx, ny)) {
                                if !self.active_work.contains(&(nx, ny)) {
                                    let ready = if nbp.is_roof() {
                                        pleb.inventory.count_of(ITEM_FIBER) >= 1
                                    } else if nbp.is_campfire() {
                                        pleb.inventory.count_of(ITEM_SCRAP_WOOD) >= 3
                                    } else {
                                        nbp.resources_met()
                                    };
                                    if ready {
                                        if nbp.is_roof() {
                                            pleb.inventory.remove(ITEM_FIBER, 1);
                                        } else if nbp.is_campfire() {
                                            pleb.inventory.remove(ITEM_SCRAP_WOOD, 3);
                                        }
                                        pleb.work_target = Some((nx, ny));
                                        pleb.activity = PlebActivity::Building(0.0);
                                        self.active_work.insert((nx, ny));
                                        chained = true;
                                        break;
                                    }
                                }
                            }
                        }
                        if !chained {
                            pleb.work_target = None;
                            pleb.activity = PlebActivity::Idle;
                        }
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

        // Apply deferred wall placements from blueprint completion
        for (tx, ty, edges, thickness, material) in wall_placements {
            self.place_wall_edge(tx, ty, edges, thickness, material);
        }

        // Apply dig marks from mud wall auto-dig — add hole to nearest dirt tile
        for (wx, wy) in dig_marks {
            'dig_search: for radius in 0..6i32 {
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        if dx.abs() != radius && dy.abs() != radius && radius > 0 {
                            continue;
                        }
                        let nx = wx + dx;
                        let ny = wy + dy;
                        if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 {
                            continue;
                        }
                        let nidx = (ny as u32 * GRID_W + nx as u32) as usize;
                        if (self.grid_data[nidx] & 0xFF) as u32 == BT_DIRT
                            && nidx < self.terrain_data.len()
                            && terrain_dig_holes(self.terrain_data[nidx]) < 7
                        {
                            terrain_add_dig_hole(&mut self.terrain_data[nidx]);
                            self.terrain_dirty = true;
                            break 'dig_search;
                        }
                    }
                }
            }
        }

        // --- Crafting: advance crafting progress ---
        for pleb in self.plebs.iter_mut() {
            if pleb.is_dead || pleb.is_enemy {
                continue;
            }
            if let PlebActivity::Crafting(recipe_id, progress) = pleb.activity {
                let recipe_reg = recipe_defs::RecipeRegistry::cached();
                if let Some(recipe) = recipe_reg.get(recipe_id) {
                    let new_progress = progress + dt * self.time_speed / recipe.time;
                    if new_progress >= 1.0 {
                        // Crafting complete — drop output on ground near pleb
                        self.ground_items.push(resources::GroundItem::new(
                            pleb.x,
                            pleb.y,
                            recipe.output.item,
                            recipe.output.count,
                        ));
                        events.push(GameEventKind::Crafted {
                            pleb: pleb.name.clone(),
                            recipe: recipe.name.clone(),
                        });
                        // Increment queue counter
                        if let Some((tx, ty)) = pleb.work_target {
                            let gidx = ty as u32 * GRID_W + tx as u32;
                            if let Some(queue) = self.craft_queues.get_mut(&gidx) {
                                if let Some(order) = queue
                                    .orders
                                    .iter_mut()
                                    .find(|o| o.recipe_id == recipe_id && o.completed < o.count)
                                {
                                    order.completed += 1;
                                }
                                // Clean up completed orders
                                queue.orders.retain(|o| o.completed < o.count);
                                // If more orders remain, start next one
                                if queue.pending() {
                                    // Re-check ingredients for next order at this station
                                    // (will be handled by the work assignment loop next frame)
                                }
                            }
                            self.active_work.remove(&(tx, ty));
                        }
                        pleb.work_target = None;
                        pleb.activity = PlebActivity::Idle;
                    } else {
                        pleb.activity = PlebActivity::Crafting(recipe_id, new_progress);
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
                if self.active_work.contains(&(bx, by)) {
                    continue;
                }
                let bp = &self.blueprints[&(bx, by)];

                // For roofs/campfires: check if any pleb has the special resource
                let special_ready = if bp.is_roof() {
                    self.plebs
                        .iter()
                        .any(|p| !p.is_enemy && p.inventory.count_of(ITEM_FIBER) >= 1)
                } else if bp.is_campfire() {
                    self.plebs
                        .iter()
                        .any(|p| !p.is_enemy && p.inventory.count_of(ITEM_SCRAP_WOOD) >= 3)
                } else {
                    true // standard blueprints don't need special check
                };
                let ready = if bp.is_roof() || bp.is_campfire() {
                    special_ready // only ready if someone has the material
                } else {
                    bp.resources_met()
                };
                if ready {
                    let needs_fiber = bp.is_roof();
                    let needs_sticks = bp.is_campfire();
                    let mut best: Option<(usize, f32)> = None;
                    for (i, pleb) in self.plebs.iter().enumerate() {
                        if pleb.is_enemy || pleb.work_target.is_some() {
                            continue;
                        }
                        if !matches!(pleb.activity, PlebActivity::Idle) {
                            continue;
                        }
                        if needs_fiber && pleb.inventory.count_of(ITEM_FIBER) == 0 {
                            continue;
                        }
                        if needs_sticks && pleb.inventory.count_of(ITEM_SCRAP_WOOD) < 3 {
                            continue;
                        }
                        let dist = ((pleb.x - bx as f32 - 0.5).powi(2)
                            + (pleb.y - by as f32 - 0.5).powi(2))
                        .sqrt();
                        if dist < 40.0 && (best.map_or(true, |(_, bd)| dist < bd)) {
                            best = Some((i, dist));
                        }
                    }
                    if let Some((pi, _)) = best {
                        let pleb = &mut self.plebs[pi];
                        let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                        let adj = adjacent_walkable(&self.grid_data, bx, by).unwrap_or((bx, by));
                        let path = astar_path_terrain_wd(
                            &self.grid_data,
                            &self.wall_data,
                            &self.terrain_data,
                            start,
                            adj,
                        );
                        if !path.is_empty() {
                            pleb.path = path;
                            pleb.path_idx = 0;
                            pleb.activity = PlebActivity::Walking;
                            pleb.work_target = Some((bx, by));
                            self.active_work.insert((bx, by));
                        }
                    }
                } else {
                    // Needs resources — find nearest material on ground and assign pleb to haul it
                    // Determine which material is needed
                    let need_item = if bp.is_roof() {
                        Some(ITEM_FIBER) // roof needs fiber (not tracked in standard fields)
                    } else if bp.is_campfire() {
                        Some(ITEM_SCRAP_WOOD) // campfire needs sticks
                    } else if bp.wood_delivered < bp.wood_needed {
                        Some(ITEM_LOG)
                    } else if bp.clay_delivered < bp.clay_needed {
                        Some(ITEM_CLAY)
                    } else if bp.plank_delivered < bp.plank_needed {
                        Some(ITEM_PLANK)
                    } else if bp.rock_delivered < bp.rock_needed {
                        Some(ITEM_ROCK)
                    } else if bp.rope_delivered < bp.rope_needed {
                        Some(ITEM_ROPE)
                    } else {
                        None
                    };

                    if let Some(needed_id) = need_item {
                        // Find nearest ground item of needed type
                        let mut best_item: Option<(usize, f32)> = None;
                        for (i, item) in self.ground_items.iter().enumerate() {
                            if item.stack.item_id == needed_id {
                                let d = ((item.x - bx as f32 - 0.5).powi(2)
                                    + (item.y - by as f32 - 0.5).powi(2))
                                .sqrt();
                                if best_item.is_none() || d < best_item.unwrap().1 {
                                    best_item = Some((i, d));
                                }
                            }
                        }
                        // Also check crates for the material (pick closest overall)
                        {
                            for (&cidx, cinv) in self.crate_contents.iter() {
                                if cinv.count_of(needed_id) > 0 {
                                    let cx2 = (cidx % GRID_W) as i32;
                                    let cy2 = (cidx / GRID_W) as i32;
                                    let d = ((cx2 as f32 + 0.5 - bx as f32 - 0.5).powi(2)
                                        + (cy2 as f32 + 0.5 - by as f32 - 0.5).powi(2))
                                    .sqrt();
                                    if best_item.is_none() || d < best_item.unwrap().1 {
                                        // Use crate position as pickup
                                        best_item = Some((usize::MAX, d)); // sentinel
                                    }
                                }
                            }
                        }
                        if let Some((wi, _)) = best_item {
                            let pickup_pos = if wi < self.ground_items.len() {
                                (
                                    self.ground_items[wi].x.floor() as i32,
                                    self.ground_items[wi].y.floor() as i32,
                                )
                            } else {
                                // From crate — find nearest crate with the material
                                let mut best_crate = (bx, by);
                                let mut best_d = f32::MAX;
                                for (&cidx, cinv) in self.crate_contents.iter() {
                                    if cinv.count_of(needed_id) > 0 {
                                        let cx2 = (cidx % GRID_W) as i32;
                                        let cy2 = (cidx / GRID_W) as i32;
                                        let d = ((cx2 - bx) as f32).powi(2)
                                            + ((cy2 - by) as f32).powi(2);
                                        if d < best_d {
                                            best_d = d;
                                            best_crate = (cx2, cy2);
                                        }
                                    }
                                }
                                best_crate
                            };
                            // Find nearest idle pleb
                            let mut best_pleb: Option<(usize, f32)> = None;
                            for (i, pleb) in self.plebs.iter().enumerate() {
                                if pleb.is_enemy || pleb.is_dead || pleb.work_target.is_some() {
                                    continue;
                                }
                                if !matches!(pleb.activity, PlebActivity::Idle) {
                                    continue;
                                }
                                let dist = ((pleb.x - pickup_pos.0 as f32 - 0.5).powi(2)
                                    + (pleb.y - pickup_pos.1 as f32 - 0.5).powi(2))
                                .sqrt();
                                if best_pleb.map_or(true, |(_, bd)| dist < bd) {
                                    best_pleb = Some((i, dist));
                                }
                            }
                            if let Some((pi, _)) = best_pleb {
                                let pleb = &mut self.plebs[pi];
                                let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                                let path = astar_path_terrain_wd(
                                    &self.grid_data,
                                    &self.wall_data,
                                    &self.terrain_data,
                                    start,
                                    pickup_pos,
                                );
                                if !path.is_empty() {
                                    pleb.path = path;
                                    pleb.path_idx = 0;
                                    pleb.activity = PlebActivity::Hauling;
                                    pleb.harvest_target = Some(pickup_pos);
                                    pleb.haul_target = Some((bx, by));
                                    self.active_work.insert((bx, by));
                                }
                            }
                        } else if needed_id == ITEM_SCRAP_WOOD || needed_id == ITEM_FIBER {
                            // No sticks/fiber on ground — auto-gather from nearest tree
                            let mut best_tree: Option<(i32, i32, f32)> = None;
                            for dy in -15i32..=15 {
                                for dx in -15i32..=15 {
                                    let tx2 = bx + dx;
                                    let ty2 = by + dy;
                                    if tx2 < 0
                                        || ty2 < 0
                                        || tx2 >= GRID_W as i32
                                        || ty2 >= GRID_H as i32
                                    {
                                        continue;
                                    }
                                    let tidx2 = (ty2 as u32 * GRID_W + tx2 as u32) as usize;
                                    if (self.grid_data[tidx2] & 0xFF) as u32 == BT_TREE {
                                        let d = (dx * dx + dy * dy) as f32;
                                        if best_tree.map_or(true, |(_, _, bd)| d < bd) {
                                            best_tree = Some((tx2, ty2, d));
                                        }
                                    }
                                }
                            }
                            if let Some((tree_x, tree_y, _)) = best_tree {
                                for pleb in self.plebs.iter_mut() {
                                    if pleb.is_enemy || pleb.is_dead || pleb.work_target.is_some() {
                                        continue;
                                    }
                                    if !matches!(pleb.activity, PlebActivity::Idle) {
                                        continue;
                                    }
                                    let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                                    let adj = adjacent_walkable(&self.grid_data, tree_x, tree_y)
                                        .unwrap_or((tree_x, tree_y));
                                    let path = astar_path_terrain_wd(
                                        &self.grid_data,
                                        &self.wall_data,
                                        &self.terrain_data,
                                        start,
                                        adj,
                                    );
                                    if !path.is_empty() {
                                        pleb.path = path;
                                        pleb.path_idx = 0;
                                        pleb.activity = PlebActivity::Walking;
                                        pleb.work_target = Some((tree_x, tree_y));
                                        pleb.harvest_target = Some((tree_x, tree_y));
                                        // Store blueprint as haul_target so pleb remembers
                                        // where to deliver after harvesting
                                        pleb.haul_target = Some((bx, by));
                                        self.active_work.insert((bx, by));
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Plebs arriving at blueprints: start building (if resources met) or deliver wood
        for pleb in self.plebs.iter_mut() {
            if pleb.is_enemy {
                continue;
            }
            // Walking pleb arriving at blueprint → start building
            if pleb.activity == PlebActivity::Walking {
                if let Some((tx, ty)) = pleb.work_target {
                    if let Some(bp) = self.blueprints.get(&(tx, ty)) {
                        let ready = if bp.is_roof() {
                            pleb.inventory.count_of(ITEM_FIBER) >= 1
                        } else if bp.is_campfire() {
                            pleb.inventory.count_of(ITEM_SCRAP_WOOD) >= 3
                        } else {
                            bp.resources_met()
                        };
                        if ready {
                            let dist = ((pleb.x - tx as f32 - 0.5).powi(2)
                                + (pleb.y - ty as f32 - 0.5).powi(2))
                            .sqrt();
                            if dist < 1.5 {
                                if bp.is_roof() {
                                    pleb.inventory.remove(ITEM_FIBER, 1);
                                } else if bp.is_campfire() {
                                    pleb.inventory.remove(ITEM_SCRAP_WOOD, 3);
                                }
                                pleb.activity = PlebActivity::Building(0.0);
                                pleb.path.clear();
                                pleb.path_idx = 0;
                            }
                        }
                    }
                }
            }
        }

        // Clean up stale active_work entries: remove any blueprint position
        // that no pleb references as work_target or haul_target
        self.active_work.retain(|pos| {
            self.plebs.iter().any(|p| {
                !p.is_enemy
                    && !p.is_dead
                    && (p.work_target == Some(*pos) || p.haul_target == Some(*pos))
            })
        });

        // Handle plebs arriving at ground item to eat:
        // Check proximity every frame (not just path_done) — pleb may walk close enough mid-path
        for pleb in self.plebs.iter_mut() {
            if pleb.is_enemy {
                continue;
            }
            let is_walking_or_idle =
                matches!(pleb.activity, PlebActivity::Walking | PlebActivity::Idle);
            let has_eat_target = pleb.harvest_target.is_some()
                && pleb.work_target.is_none()
                && pleb.haul_target.is_none();
            if is_walking_or_idle && has_eat_target {
                if let Some((tx, ty)) = pleb.harvest_target {
                    let dist = ((pleb.x - tx as f32 - 0.5).powi(2)
                        + (pleb.y - ty as f32 - 0.5).powi(2))
                    .sqrt();
                    if dist < 1.5 {
                        pleb.activity = PlebActivity::Eating;
                        pleb.path.clear();
                        pleb.path_idx = 0;
                    }
                }
            }
        }

        // --- Auto-haul ground items to storage zones ---
        if !self.ground_items.is_empty() {
            // Collect storage zone tiles
            let storage_tiles: Vec<(i32, i32)> = self
                .zones
                .iter()
                .filter(|z| z.kind == ZoneKind::Storage)
                .flat_map(|z| z.tiles.iter().copied())
                .collect();
            if !storage_tiles.is_empty() {
                // Find ground items NOT already on a storage zone tile
                let occupied: std::collections::HashSet<(i32, i32)> = self
                    .ground_items
                    .iter()
                    .map(|item| (item.x.floor() as i32, item.y.floor() as i32))
                    .collect();
                let empty_storage: Vec<(i32, i32)> = storage_tiles
                    .iter()
                    .filter(|t| !occupied.contains(t))
                    .copied()
                    .collect();

                // For each loose ground item (not on storage), try to assign a haul
                for gi in 0..self.ground_items.len() {
                    let item = &self.ground_items[gi];
                    let ix = item.x.floor() as i32;
                    let iy = item.y.floor() as i32;
                    // Skip items already on a storage zone tile
                    let on_storage = storage_tiles.contains(&(ix, iy));
                    if on_storage {
                        continue;
                    }
                    // Find nearest empty storage tile
                    let nearest_slot = empty_storage
                        .iter()
                        .map(|&(sx, sy)| {
                            let d = ((ix - sx).pow(2) + (iy - sy).pow(2)) as f32;
                            (sx, sy, d)
                        })
                        .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
                    let Some((sx, sy, _)) = nearest_slot else {
                        break;
                    };
                    // Find nearest idle pleb not already doing something
                    let mut best_pleb: Option<(usize, f32)> = None;
                    for (pi, pleb) in self.plebs.iter().enumerate() {
                        if pleb.is_enemy || pleb.work_target.is_some() || pleb.haul_target.is_some()
                        {
                            continue;
                        }
                        if !matches!(pleb.activity, PlebActivity::Idle) {
                            continue;
                        }
                        let dist = ((pleb.x - ix as f32 - 0.5).powi(2)
                            + (pleb.y - iy as f32 - 0.5).powi(2))
                        .sqrt();
                        if dist < 40.0 && (best_pleb.map_or(true, |(_, bd)| dist < bd)) {
                            best_pleb = Some((pi, dist));
                        }
                    }
                    if let Some((pi, _)) = best_pleb {
                        let pleb = &mut self.plebs[pi];
                        let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                        let path = astar_path_terrain_wd(
                            &self.grid_data,
                            &self.wall_data,
                            &self.terrain_data,
                            start,
                            (ix, iy),
                        );
                        if !path.is_empty() {
                            pleb.path = path;
                            pleb.path_idx = 0;
                            pleb.activity = PlebActivity::Hauling;
                            pleb.harvest_target = Some((ix, iy)); // pickup location
                            pleb.haul_target = Some((sx, sy)); // storage zone tile
                            break; // one haul assignment per tick to avoid overwhelm
                        }
                    }
                }
            }
        }

        // --- Sound shockwave damage ---
        // Approximate sound pressure at each pleb from active sources (CPU-side, no GPU readback).
        // Uses inverse-square falloff. Damage threshold ~100 dB at pleb position (amp ~3.16).
        if self.sound_enabled && self.sound_coupling > 0.001 {
            let damage_threshold = db_to_amplitude(100.0); // ~3.16 — only very loud sounds damage
            for pleb in &mut self.plebs {
                let mut max_pressure = 0.0f32;
                for src in &self.sound_sources {
                    let dx = pleb.x - src.x;
                    let dy = pleb.y - src.y;
                    let dist = (dx * dx + dy * dy).sqrt().max(0.5);
                    // Pressure falls off with 1/distance (cylindrical spreading in 2D)
                    let pressure = src.amplitude / dist;
                    max_pressure = max_pressure.max(pressure);
                }
                if max_pressure > damage_threshold {
                    // Damage proportional to how far above threshold, in dB
                    let excess_db = amplitude_to_db(max_pressure) - 100.0;
                    let damage = excess_db * 0.002 * dt; // gradual: ~30 dB excess = 0.06/sec
                    pleb.needs.health -= damage;
                    if damage > 0.005 {
                        let _db_at_pleb = amplitude_to_db(max_pressure);
                        events.push(GameEventKind::PlebHit {
                            pleb: pleb.name.clone(),
                            hp_pct: pleb.needs.health.max(0.0) * 100.0,
                        });
                    }
                }
            }
        }

        // --- Mental breaks: trigger at high stress ---
        for pleb in self.plebs.iter_mut() {
            if pleb.is_dead || pleb.is_enemy {
                continue;
            }

            // Tick existing mental break
            let break_state = if let PlebActivity::MentalBreak(ref k, r) = pleb.activity {
                Some((k.clone(), r))
            } else {
                None
            };
            if let Some((kind, remaining)) = break_state {
                if remaining <= dt * self.time_speed {
                    // Break over — stress drops, return to idle
                    pleb.needs.stress = needs::STRESS_POST_BREAK;
                    let kind_name = match kind {
                        MentalBreakKind::Daze => "daze",
                        MentalBreakKind::Binge => "binge",
                        MentalBreakKind::Tantrum => "tantrum",
                        MentalBreakKind::Collapse => "collapse",
                    };
                    events.push(GameEventKind::MentalBreakRecovered {
                        pleb: pleb.name.clone(),
                        kind: kind_name,
                    });
                    pleb.activity = PlebActivity::Idle;
                } else {
                    pleb.activity =
                        PlebActivity::MentalBreak(kind.clone(), remaining - dt * self.time_speed);
                    // Daze: random wandering
                    if matches!(kind, MentalBreakKind::Daze) && pleb.path.is_empty() {
                        let hash = (self
                            .frame_count
                            .wrapping_mul(2654435761)
                            .wrapping_add(pleb.id as u32 * 7919))
                            % 100;
                        if hash < 5 {
                            // occasionally pick a new wander target
                            let dx = (hash % 11) as i32 - 5;
                            let dy = ((hash / 11) % 11) as i32 - 5;
                            let tx = (pleb.x as i32 + dx).clamp(0, GRID_W as i32 - 1);
                            let ty = (pleb.y as i32 + dy).clamp(0, GRID_H as i32 - 1);
                            let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                            let path = astar_path_terrain_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                start,
                                (tx, ty),
                            );
                            if !path.is_empty() {
                                pleb.path = path;
                                pleb.path_idx = 0;
                            }
                        }
                    }
                }
                continue;
            }

            // Trigger new mental break
            if pleb.needs.stress >= needs::STRESS_BREAK_THRESHOLD
                && !matches!(pleb.activity, PlebActivity::MentalBreak(_, _))
                && !pleb.activity.is_crisis()
            {
                let hash = self
                    .frame_count
                    .wrapping_mul(2654435761)
                    .wrapping_add(pleb.id as u32 * 1013904223);
                let kind = match hash % 4 {
                    0 => MentalBreakKind::Daze,
                    1 => MentalBreakKind::Binge,
                    2 => MentalBreakKind::Tantrum,
                    _ => MentalBreakKind::Collapse,
                };
                let duration = match &kind {
                    MentalBreakKind::Daze => 30.0,
                    MentalBreakKind::Binge => 15.0,
                    MentalBreakKind::Tantrum => 10.0,
                    MentalBreakKind::Collapse => 20.0,
                };
                let kind_name = match &kind {
                    MentalBreakKind::Daze => "daze",
                    MentalBreakKind::Binge => "binge eating",
                    MentalBreakKind::Tantrum => "tantrum",
                    MentalBreakKind::Collapse => "collapse",
                };
                events.push(GameEventKind::MentalBreak {
                    pleb: pleb.name.clone(),
                    kind: kind_name,
                });
                pleb.path.clear();
                pleb.path_idx = 0;
                pleb.work_target = None;
                pleb.haul_target = None;
                pleb.harvest_target = None;
                pleb.activity = PlebActivity::MentalBreak(kind, duration);
            }
        }

        // --- Mark dead plebs as corpses ---
        for pleb in &mut self.plebs {
            if pleb.needs.health <= 0.0 && !pleb.is_dead {
                pleb.is_dead = true;
                pleb.activity = PlebActivity::Idle;
                pleb.path.clear();
                pleb.work_target = None;
                pleb.haul_target = None;
                pleb.harvest_target = None;
                events.push(GameEventKind::PlebDied(pleb.name.clone()));
            }
        }

        // Push all collected events to the game log + trigger notifications
        for event in events {
            if let Some((ncat, icon, title)) = event.notification() {
                let msg = event.message();
                self.notify(ncat, icon, title, &msg);
            }
            let cat = event.category();
            let msg = event.message();
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
    wall_data: &[u16],
    terrain: &[u32],
    dt: f32,
    time_speed: f32,
    ground_items: &mut Vec<resources::GroundItem>,
    time_of_day: f32,
) {
    // --- Activity state machine (works on inner activity for crisis) ---
    let inner_act = pleb.activity.inner().clone();
    let was_crisis = pleb.activity.is_crisis();
    let crisis_reason = pleb.activity.crisis_reason();

    match &inner_act {
        PlebActivity::Sleeping => {
            let fully_rested = pleb.needs.rest > 0.95;
            let cant_breathe = pleb.needs.breathing_state != BreathingState::Normal;
            // Wake up when: fully rested, can't breathe, OR shift says work time and rested enough
            let shift_wake =
                !pleb.schedule.is_sleep_time(time_of_day, DAY_DURATION) && pleb.needs.rest > 0.5;
            if fully_rested || cant_breathe || shift_wake {
                pleb.activity = PlebActivity::Idle;
            }
        }
        PlebActivity::Harvesting(progress) => {
            // Speed scales inversely with yield for tree gathering (more fiber = more time)
            let is_tree = pleb.harvest_target.map_or(false, |(hx, hy)| {
                let hidx = (hy as u32 * GRID_W + hx as u32) as usize;
                hidx < grid.len() && (grid[hidx] & 0xFF) as u32 == BT_TREE
            });
            let harvest_speed = if is_tree { 1.5 } else { 5.0 }; // trees: slower (bigger yield)
            let new_progress = progress + dt * time_speed * harvest_speed;
            if new_progress >= 1.0 {
                // Check what we're harvesting: tree → sticks, bush → berries
                let is_tree_target = pleb.harvest_target.map_or(false, |(hx, hy)| {
                    let hidx = (hy as u32 * GRID_W + hx as u32) as usize;
                    hidx < grid.len() && (grid[hidx] & 0xFF) as u32 == BT_TREE
                });
                if is_tree_target {
                    // Gather branches — bulk harvest: 5-10 sticks + 5-10 fiber
                    // Enough for a small roof in one trip
                    let rng = ((pleb.x * 137.3 + pleb.y * 311.7) as u32).wrapping_mul(2654435761);
                    let stick_count = 5 + (rng % 6) as u16; // 5-10
                    let fiber_count = 5 + ((rng >> 8) % 6) as u16; // 5-10
                    ground_items.push(resources::GroundItem::new(
                        pleb.x + 0.2,
                        pleb.y + 0.1,
                        ITEM_SCRAP_WOOD,
                        stick_count,
                    ));
                    ground_items.push(resources::GroundItem::new(
                        pleb.x - 0.2,
                        pleb.y + 0.2,
                        ITEM_FIBER,
                        fiber_count,
                    ));
                    log::info!(
                        "{} gathered {} sticks + {} fiber from tree",
                        pleb.name,
                        stick_count,
                        fiber_count
                    );
                } else {
                    // Berry bush harvest
                    ground_items.push(resources::GroundItem::new(pleb.x, pleb.y, ITEM_BERRIES, 3));
                    log::info!("{} harvested 3 berries", pleb.name);
                }
                pleb.harvest_target = None;
                if was_crisis {
                    pleb.activity = PlebActivity::Crisis(
                        Box::new(PlebActivity::Eating),
                        crisis_reason.unwrap_or("Starving"),
                    );
                } else if pleb.haul_target.is_some() && is_tree_target {
                    // Was gathering for a blueprint — auto-pickup dropped fiber
                    // and continue hauling to the blueprint
                    let mut picked_up = 0u16;
                    let mut gi_idx = 0;
                    while gi_idx < ground_items.len() {
                        let gi = &ground_items[gi_idx];
                        if gi.stack.item_id == ITEM_FIBER {
                            let d = (gi.x - pleb.x).powi(2) + (gi.y - pleb.y).powi(2);
                            if d < 4.0 {
                                let count = ground_items[gi_idx].stack.count;
                                pleb.inventory.add(ITEM_FIBER, count);
                                picked_up += count;
                                ground_items.remove(gi_idx);
                                continue;
                            }
                        }
                        gi_idx += 1;
                    }
                    if picked_up > 0 {
                        if let Some((cx, cy)) = pleb.haul_target {
                            let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                            let adj = adjacent_walkable(grid, cx, cy).unwrap_or((cx, cy));
                            let path = astar_path_terrain_wd(grid, wall_data, terrain, start, adj);
                            if !path.is_empty() {
                                pleb.path = path;
                                pleb.path_idx = 0;
                                pleb.activity = PlebActivity::Hauling;
                                pleb.harvest_target = None;
                            } else {
                                pleb.activity = PlebActivity::Idle;
                                pleb.haul_target = None;
                            }
                        } else {
                            pleb.activity = PlebActivity::Idle;
                        }
                    } else {
                        pleb.activity = PlebActivity::Idle;
                        pleb.haul_target = None;
                    }
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
            if pleb.inventory.count_of(ITEM_BERRIES) > 0 {
                pleb.inventory.remove(ITEM_BERRIES, 1);
                pleb.needs.hunger = (pleb.needs.hunger + BERRY_HUNGER_RESTORE).min(1.0);
                log::info!(
                    "{} ate a berry from inventory (hunger: {:.0}%)",
                    pleb.name,
                    pleb.needs.hunger * 100.0
                );
                ate = true;
            }
            // Try eating from ground item at harvest_target
            if !ate {
                if let Some((tx, ty)) = pleb.harvest_target {
                    if let Some(gi) = ground_items.iter_mut().position(|item| {
                        item.x.floor() as i32 == tx
                            && item.y.floor() as i32 == ty
                            && item.stack.item_id == ITEM_BERRIES
                    }) {
                        pleb.needs.hunger = (pleb.needs.hunger + BERRY_HUNGER_RESTORE).min(1.0);
                        if ground_items[gi].stack.count <= 1 {
                            ground_items.remove(gi);
                        } else {
                            ground_items[gi].stack.count -= 1;
                        }
                        log::info!(
                            "{} ate a berry from ground (hunger: {:.0}%)",
                            pleb.name,
                            pleb.needs.hunger * 100.0
                        );
                        ate = true;
                    }
                }
            }
            pleb.harvest_target = None;
            if was_crisis
                && pleb.needs.hunger < 0.3
                && (pleb.inventory.count_of(ITEM_BERRIES) > 0 || ate)
            {
                pleb.activity = PlebActivity::Crisis(
                    Box::new(PlebActivity::Eating),
                    crisis_reason.unwrap_or("Starving"),
                );
            } else {
                pleb.activity = PlebActivity::Idle;
            }
        }
        PlebActivity::Drinking(progress) => {
            let new_progress = progress + dt * time_speed / WELL_DRINK_TIME;
            if new_progress >= 1.0 {
                pleb.needs.thirst = (pleb.needs.thirst + WELL_THIRST_RESTORE).min(1.0);
                log::info!(
                    "{} drank from well (thirst: {:.0}%)",
                    pleb.name,
                    pleb.needs.thirst * 100.0
                );
                // Also fill any empty containers in inventory
                for stack in &mut pleb.inventory.stacks {
                    if stack.is_container() && stack.liquid.is_none() {
                        let cap = stack.liquid_capacity();
                        stack.liquid = Some((item_defs::LiquidType::Water, cap));
                        log::info!("{} filled {} with water", pleb.name, stack.label());
                        break; // fill one at a time
                    }
                }
                pleb.activity = PlebActivity::Idle;
                pleb.work_target = None;
            } else {
                pleb.activity = PlebActivity::Drinking(new_progress);
            }
        }
        _ => {}
    }

    // --- Crisis auto-behaviors (override player control) ---
    let is_idle_or_walk = matches!(
        pleb.activity.inner(),
        PlebActivity::Idle | PlebActivity::Walking
    );

    if pleb.needs.hunger < 0.10 && is_idle_or_walk {
        // CRISIS: Starving
        if pleb.inventory.count_of(ITEM_BERRIES) > 0 {
            pleb.activity = PlebActivity::Crisis(Box::new(PlebActivity::Eating), "Starving!");
        } else if let Some((bx, by)) = env.nearest_berry_bush {
            if env.near_berry_bush {
                pleb.harvest_target = Some((bx, by));
                pleb.activity =
                    PlebActivity::Crisis(Box::new(PlebActivity::Harvesting(0.0)), "Starving!");
                pleb.path.clear();
                pleb.path_idx = 0;
            } else {
                send_pleb_to(
                    pleb,
                    grid,
                    wall_data,
                    terrain,
                    (bx, by),
                    PlebActivity::Crisis(Box::new(PlebActivity::Walking), "Starving!"),
                );
            }
        }
    } else if pleb.needs.thirst < 0.10 && is_idle_or_walk {
        // CRISIS: Dehydrated — seek nearest well
        if let Some((wx, wy)) =
            find_nearest_well(grid, pleb.x.floor() as i32, pleb.y.floor() as i32)
        {
            let adj = adjacent_walkable(grid, wx, wy).unwrap_or((wx, wy));
            let dist =
                ((pleb.x - wx as f32 - 0.5).powi(2) + (pleb.y - wy as f32 - 0.5).powi(2)).sqrt();
            if dist < 1.5 {
                pleb.activity =
                    PlebActivity::Crisis(Box::new(PlebActivity::Drinking(0.0)), "Dehydrated!");
                pleb.work_target = Some((wx, wy));
            } else {
                send_pleb_to(
                    pleb,
                    grid,
                    wall_data,
                    terrain,
                    adj,
                    PlebActivity::Crisis(Box::new(PlebActivity::Walking), "Dehydrated!"),
                );
                pleb.work_target = Some((wx, wy));
            }
        }
    } else if pleb.needs.rest < 0.08 && is_idle_or_walk && !pleb.activity.is_crisis() {
        // CRISIS: Exhausted
        if env.near_bed {
            pleb.activity = PlebActivity::Crisis(Box::new(PlebActivity::Sleeping), "Exhausted!");
            pleb.path.clear();
            pleb.path_idx = 0;
        } else if let Some((bx, by)) = env.nearest_bed {
            send_pleb_to(
                pleb,
                grid,
                wall_data,
                terrain,
                (bx, by),
                PlebActivity::Crisis(Box::new(PlebActivity::Walking), "Exhausted!"),
            );
        } else {
            pleb.activity = PlebActivity::Crisis(Box::new(PlebActivity::Sleeping), "Collapsed!");
            pleb.path.clear();
            pleb.path_idx = 0;
        }
    } else if false && pleb.needs.warmth < 0.12 && is_idle_or_walk && !pleb.activity.is_crisis() {
        // CRISIS: Freezing — disabled for now (plebs can work while cold)
        if env.is_indoors || env.near_fire {
            // Already sheltered, just wait it out
        } else if let Some((bx, by)) = env.nearest_bed {
            // Run to nearest bed (likely indoors)
            send_pleb_to(
                pleb,
                grid,
                wall_data,
                terrain,
                (bx, by),
                PlebActivity::Crisis(Box::new(PlebActivity::Walking), "Freezing!"),
            );
        } else {
            // No shelter — huddle in place
            pleb.activity = PlebActivity::Crisis(Box::new(PlebActivity::Idle), "Freezing!");
        }
    }

    // CRISIS: Overheating — overrides ALL activities (even sleeping/harvesting)
    // Any pleb in dangerous heat drops everything and runs
    // Only triggers on idle/walking to prevent re-trigger loop when pleb arrives at cool tile
    let can_heat_flee = matches!(
        pleb.activity.inner(),
        PlebActivity::Idle | PlebActivity::Walking
    );
    if pleb.needs.air_temp > HEAT_CRISIS_TEMP
        && can_heat_flee
        && pleb.activity.crisis_reason() != Some("Overheating!")
    {
        let bx = pleb.x.floor() as i32;
        let by = pleb.y.floor() as i32;
        // Search from radius 3+ to avoid pathing to an adjacent tile that's equally hot
        if let Some(target) = find_cool_tile(grid, bx, by, 20) {
            send_pleb_to(
                pleb,
                grid,
                wall_data,
                terrain,
                target,
                PlebActivity::Crisis(Box::new(PlebActivity::Walking), "Overheating!"),
            );
        }
    } else if !pleb.activity.is_crisis() {
        // Non-crisis auto-behaviors
        if pleb.activity == PlebActivity::Idle || pleb.activity == PlebActivity::Walking {
            if pleb.needs.hunger < 0.25 && pleb.inventory.count_of(ITEM_BERRIES) > 0 {
                pleb.activity = PlebActivity::Eating;
            } else if pleb.needs.hunger < 0.25 && pleb.inventory.count_of(ITEM_BERRIES) == 0 {
                // Find nearest berries on the ground (storage zones or loose)
                let px = pleb.x.floor() as i32;
                let py = pleb.y.floor() as i32;
                let mut best_berry: Option<(i32, i32, f32)> = None;
                for item in ground_items.iter() {
                    if item.stack.item_id == ITEM_BERRIES {
                        let bx = item.x.floor() as i32;
                        let by = item.y.floor() as i32;
                        let d = ((px - bx).pow(2) + (py - by).pow(2)) as f32;
                        if d < 900.0 && (best_berry.is_none() || d < best_berry.unwrap().2) {
                            // within 30 tiles
                            best_berry = Some((bx, by, d));
                        }
                    }
                }
                if let Some((bx, by, _)) = best_berry {
                    let start = (px, py);
                    let path = astar_path_terrain_wd(grid, wall_data, terrain, start, (bx, by));
                    if !path.is_empty() {
                        pleb.path = path;
                        pleb.path_idx = 0;
                        pleb.activity = PlebActivity::Walking;
                        pleb.harvest_target = Some((bx, by)); // eat target
                        pleb.work_target = None;
                        pleb.haul_target = None;
                    }
                }
            } else if pleb.needs.thirst < 0.30 && pleb.work_target.is_none() {
                // Auto-drink: seek nearest well when thirsty (but not crisis)
                let px = pleb.x.floor() as i32;
                let py = pleb.y.floor() as i32;
                if let Some((wx, wy)) = find_nearest_well(grid, px, py) {
                    let dist = ((pleb.x - wx as f32 - 0.5).powi(2)
                        + (pleb.y - wy as f32 - 0.5).powi(2))
                    .sqrt();
                    if dist < 1.5 {
                        pleb.activity = PlebActivity::Drinking(0.0);
                        pleb.work_target = Some((wx, wy));
                    } else {
                        let adj = adjacent_walkable(grid, wx, wy).unwrap_or((wx, wy));
                        send_pleb_to(pleb, grid, wall_data, terrain, adj, PlebActivity::Walking);
                        pleb.work_target = Some((wx, wy));
                    }
                }
            } else if !matches!(pleb.activity, PlebActivity::Sleeping) {
                // Sleep when: shift says it's bedtime (unless override), OR very tired
                let is_bedtime = pleb.schedule.is_sleep_time(time_of_day, DAY_DURATION);
                let very_tired = pleb.needs.rest < 0.2;
                // Only auto-sleep if there's a bed available (early game: work through the night)
                let has_bed = env.near_bed || env.nearest_bed.is_some();
                let has_work = pleb.work_target.is_some() || pleb.haul_target.is_some();
                let should_sleep = (is_bedtime || very_tired) && has_bed && !has_work;
                if should_sleep {
                    if env.near_bed {
                        pleb.activity = PlebActivity::Sleeping;
                        pleb.path.clear();
                        pleb.path_idx = 0;
                    } else if let Some((bx, by)) = env.nearest_bed {
                        send_pleb_to(
                            pleb,
                            grid,
                            wall_data,
                            terrain,
                            (bx, by),
                            PlebActivity::Walking,
                        );
                    }
                }
            } else if pleb.needs.hunger < 0.4 && pleb.inventory.count_of(ITEM_BERRIES) == 0 {
                if env.near_berry_bush && pleb.harvest_target.is_none() {
                    if let Some((bx, by)) = env.nearest_berry_bush {
                        pleb.harvest_target = Some((bx, by));
                        pleb.activity = PlebActivity::Harvesting(0.0);
                        pleb.path.clear();
                        pleb.path_idx = 0;
                    }
                } else if pleb.harvest_target.is_none() {
                    if let Some((bx, by)) = env.nearest_berry_bush {
                        send_pleb_to(
                            pleb,
                            grid,
                            wall_data,
                            terrain,
                            (bx, by),
                            PlebActivity::Walking,
                        );
                    }
                }
            }
        }
    }
}

impl App {
    /// Generate contextual hints based on current game state.
    pub(crate) fn generate_hints(&mut self) {
        self.game_hints.clear();

        let item_reg = item_defs::ItemRegistry::cached();

        // Check blueprints for missing materials
        for (&(bx, by), bp) in &self.blueprints {
            if bp.resources_met() && !bp.is_roof() && !bp.is_campfire() {
                continue;
            }

            let mut missing = Vec::new();
            let mut items: Vec<u16> = Vec::new();
            let mut blocks: Vec<u32> = Vec::new();

            if bp.is_roof() {
                // Check if fiber exists on ground or in inventory
                let has_fiber = self
                    .ground_items
                    .iter()
                    .any(|gi| gi.stack.item_id == ITEM_FIBER)
                    || self
                        .plebs
                        .iter()
                        .any(|p| p.inventory.count_of(ITEM_FIBER) > 0);
                if !has_fiber {
                    missing.push("fiber");
                    items.push(ITEM_FIBER);
                    blocks.push(BT_TREE); // gather branches from trees
                }
            } else if bp.is_campfire() {
                let sticks_avail: u32 = self
                    .ground_items
                    .iter()
                    .filter(|gi| gi.stack.item_id == ITEM_SCRAP_WOOD)
                    .map(|gi| gi.stack.count as u32)
                    .sum::<u32>()
                    + self
                        .plebs
                        .iter()
                        .map(|p| p.inventory.count_of(ITEM_SCRAP_WOOD))
                        .sum::<u32>();
                if sticks_avail < 3 {
                    missing.push("sticks");
                    items.push(ITEM_SCRAP_WOOD);
                    blocks.push(BT_TREE);
                }
            } else {
                if bp.wood_delivered < bp.wood_needed {
                    let avail: u32 = self
                        .ground_items
                        .iter()
                        .filter(|gi| gi.stack.item_id == ITEM_LOG || gi.stack.item_id == ITEM_WOOD)
                        .map(|gi| gi.stack.count as u32)
                        .sum();
                    if avail == 0 {
                        missing.push("logs");
                        items.push(ITEM_LOG);
                        blocks.push(BT_TREE);
                    }
                }
                if bp.clay_delivered < bp.clay_needed {
                    let avail: u32 = self
                        .ground_items
                        .iter()
                        .filter(|gi| gi.stack.item_id == ITEM_CLAY)
                        .map(|gi| gi.stack.count as u32)
                        .sum();
                    if avail == 0 {
                        missing.push("clay");
                        items.push(ITEM_CLAY);
                    }
                }
                if bp.rock_delivered < bp.rock_needed {
                    let avail: u32 = self
                        .ground_items
                        .iter()
                        .filter(|gi| gi.stack.item_id == ITEM_ROCK)
                        .map(|gi| gi.stack.count as u32)
                        .sum();
                    if avail == 0 {
                        missing.push("rock");
                        items.push(ITEM_ROCK);
                        blocks.push(BT_ROCK);
                    }
                }
                if bp.plank_delivered < bp.plank_needed {
                    let avail: u32 = self
                        .ground_items
                        .iter()
                        .filter(|gi| gi.stack.item_id == ITEM_PLANK)
                        .map(|gi| gi.stack.count as u32)
                        .sum();
                    if avail == 0 {
                        missing.push("planks");
                        items.push(ITEM_PLANK);
                    }
                }
                if bp.rope_delivered < bp.rope_needed {
                    let avail: u32 = self
                        .ground_items
                        .iter()
                        .filter(|gi| gi.stack.item_id == ITEM_ROPE)
                        .map(|gi| gi.stack.count as u32)
                        .sum();
                    if avail == 0 {
                        missing.push("rope");
                        items.push(ITEM_ROPE);
                    }
                }
            }

            if !missing.is_empty() {
                let block_name = if bp.is_roof() {
                    "thatched roof".to_string()
                } else if bp.is_campfire() {
                    "campfire".to_string()
                } else {
                    let bt = bp.block_data & 0xFF;
                    block_defs::BlockRegistry::cached().name(bt).to_string()
                };
                let hint = GameHint {
                    text: format!("Need {} for {}", missing.join(" and "), block_name),
                    highlight_items: items,
                    highlight_blocks: blocks,
                };
                // Avoid duplicate hints
                if !self.game_hints.iter().any(|h| h.text == hint.text) {
                    self.game_hints.push(hint);
                }
            }
        }

        // Limit to top 3 hints
        self.game_hints.truncate(3);
    }
}

/// Helper: pathfind pleb to a target and set their activity. Returns true if path found.
fn send_pleb_to(
    pleb: &mut Pleb,
    grid: &[u32],
    wall_data: &[u16],
    terrain: &[u32],
    target: (i32, i32),
    activity: PlebActivity,
) -> bool {
    let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
    let path = astar_path_terrain_wd(grid, wall_data, terrain, start, target);
    if !path.is_empty() {
        pleb.path = path;
        pleb.path_idx = 0;
        pleb.activity = activity;
        true
    } else {
        false
    }
}
