//! Simulation update — time, sun, plebs, physics, pipes, doors.
//! Extracted from render() to keep main.rs manageable.

use crate::item_defs::*;
use crate::recipe_defs;
use crate::zones::*;
use crate::*;

/// Pleb activity speed multiplier. Since time_speed 0.1 = "100%", activities need
/// this boost to feel responsive. Pleb movement is decoupled (real-time), but
/// farming, building, mining, cooking etc. scale with time_speed × this constant.
const ACTION_SPEED_MUL: f32 = 10.0;

/// Smoothly interpolate between two angles, handling wraparound.
#[inline]
fn lerp_angle(from: f32, to: f32, t: f32) -> f32 {
    let diff = ((to - from) + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    from + diff * t
}

const TURN_SPEED_WALK: f32 = 10.0; // radians/sec — smooth walking turn
const TURN_SPEED_COMBAT: f32 = 16.0; // radians/sec — faster snap for aiming

/// Find a cover position near `pos` that puts a low wall between the pleb and the threat.
fn find_cover_position(
    grid: &[u32],
    px: f32,
    py: f32,
    threat_x: f32,
    threat_y: f32,
    search_radius: i32,
) -> Option<(i32, i32)> {
    let bx = px.floor() as i32;
    let by = py.floor() as i32;
    let mut best: Option<(i32, i32, f32)> = None;

    for dy in -search_radius..=search_radius {
        for dx in -search_radius..=search_radius {
            let wx = bx + dx;
            let wy = by + dy;
            if wx < 0 || wy < 0 || wx >= GRID_W as i32 || wy >= GRID_H as i32 {
                continue;
            }
            let idx = (wy as u32 * GRID_W + wx as u32) as usize;
            let block = grid[idx];
            if block_type_rs(block) != BT_LOW_WALL {
                continue;
            }

            // Check adjacent walkable tiles as potential cover positions
            for &(adx, ady) in &[(0i32, -1), (0, 1), (-1, 0), (1, 0)] {
                let cx = wx + adx;
                let cy = wy + ady;
                if cx < 0 || cy < 0 || cx >= GRID_W as i32 || cy >= GRID_H as i32 {
                    continue;
                }
                let cidx = (cy as u32 * GRID_W + cx as u32) as usize;
                let cblock = grid[cidx];
                let cbt = block_type_rs(cblock);
                let cbh = block_height_rs(cblock);
                // Must be walkable (air/dirt/floor at height 0)
                if cbh > 0 && !bt_is!(cbt, BT_TREE, BT_BERRY_BUSH, BT_CROP) {
                    continue;
                }

                // Wall must be between cover position and threat (dot product check)
                let wall_dx = (wx - cx) as f32;
                let wall_dy = (wy - cy) as f32;
                let threat_dx = threat_x - cx as f32;
                let threat_dy = threat_y - cy as f32;
                let dot = wall_dx * threat_dx + wall_dy * threat_dy;
                if dot <= 0.0 {
                    continue; // wall is on wrong side
                }

                let dist_sq = (cx as f32 + 0.5 - px).powi(2) + (cy as f32 + 0.5 - py).powi(2);
                let score = dot / (dist_sq + 1.0);
                if best.map_or(true, |(_, _, s)| score > s) {
                    best = Some((cx, cy, score));
                }
            }
        }
    }
    best.map(|(x, y, _)| (x, y))
}

/// Find the nearest walkable tile adjacent to any low wall (no threat direction needed).
pub(crate) fn find_nearest_cover(
    grid: &[u32],
    px: f32,
    py: f32,
    search_radius: i32,
) -> Option<(i32, i32)> {
    let bx = px.floor() as i32;
    let by = py.floor() as i32;
    let mut best: Option<(i32, i32, f32)> = None;

    for dy in -search_radius..=search_radius {
        for dx in -search_radius..=search_radius {
            let wx = bx + dx;
            let wy = by + dy;
            if wx < 0 || wy < 0 || wx >= GRID_W as i32 || wy >= GRID_H as i32 {
                continue;
            }
            let idx = (wy as u32 * GRID_W + wx as u32) as usize;
            if block_type_rs(grid[idx]) != BT_LOW_WALL {
                continue;
            }
            // Check adjacent walkable tiles
            for &(adx, ady) in &[(0i32, -1), (0, 1), (-1, 0), (1, 0)] {
                let cx = wx + adx;
                let cy = wy + ady;
                if cx < 0 || cy < 0 || cx >= GRID_W as i32 || cy >= GRID_H as i32 {
                    continue;
                }
                let cidx = (cy as u32 * GRID_W + cx as u32) as usize;
                let cbh = block_height_rs(grid[cidx]);
                let cbt = block_type_rs(grid[cidx]);
                if cbh > 0 && !bt_is!(cbt, BT_TREE, BT_BERRY_BUSH, BT_CROP) {
                    continue;
                }
                let dist_sq = (cx as f32 + 0.5 - px).powi(2) + (cy as f32 + 0.5 - py).powi(2);
                if best.map_or(true, |(_, _, d)| dist_sq < d) {
                    best = Some((cx, cy, dist_sq));
                }
            }
        }
    }
    best.map(|(x, y, _)| (x, y))
}

/// Check if pleb at (px,py) is adjacent to a low wall that faces toward (tx,ty).
/// Returns true if the pleb is "behind cover" relative to the target.
fn is_behind_cover(grid: &[u32], wall_data: &[u16], px: f32, py: f32, tx: f32, ty: f32) -> bool {
    let bx = px.floor() as i32;
    let by = py.floor() as i32;
    let threat_dx = tx - px;
    let threat_dy = ty - py;
    // Check 4 adjacent tiles for a low wall between pleb and threat
    for &(adx, ady) in &[(0i32, -1), (0, 1), (-1, 0), (1, 0)] {
        let wx = bx + adx;
        let wy = by + ady;
        if wx < 0 || wy < 0 || wx >= GRID_W as i32 || wy >= GRID_H as i32 {
            continue;
        }
        let idx = (wy as u32 * GRID_W + wx as u32) as usize;
        // Check wall_data for low wall edges
        if idx < wall_data.len() {
            let wd = wall_data[idx];
            if wd_edges(wd) != 0 && wd_height(wd) > 0 && wd_height(wd) < 3 {
                // Wall tile is toward the threat?
                let dot = adx as f32 * threat_dx + ady as f32 * threat_dy;
                if dot > 0.0 {
                    return true;
                }
            }
        }
        // Also check grid_data
        let block = grid[idx];
        let bt = block_type_rs(block);
        let bh = block_height_rs(block);
        if bt == BT_LOW_WALL && bh > 0 {
            let dot = adx as f32 * threat_dx + ady as f32 * threat_dy;
            if dot > 0.0 {
                return true;
            }
        }
    }
    false
}

/// Check if a low wall provides cover between two points (simplified DDA walk).
fn has_low_wall_cover(grid: &[u32], sx: f32, sy: f32, tx: f32, ty: f32) -> bool {
    let dx = tx - sx;
    let dy = ty - sy;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 0.5 {
        return false;
    }
    let steps = (dist * 2.0) as i32; // 2 checks per tile
    for i in 1..steps {
        let t = i as f32 / steps as f32;
        let cx = (sx + dx * t).floor() as i32;
        let cy = (sy + dy * t).floor() as i32;
        if cx < 0 || cy < 0 || cx >= GRID_W as i32 || cy >= GRID_H as i32 {
            continue;
        }
        let idx = (cy as u32 * GRID_W + cx as u32) as usize;
        let block = grid[idx];
        if block_type_rs(block) == BT_LOW_WALL && block_height_rs(block) > 0 {
            return true;
        }
    }
    false
}

/// Check if any adjacent tile has a low wall (for morale cover recovery).
fn has_low_wall_cover_any_direction(grid: &[u32], px: f32, py: f32) -> bool {
    let bx = px.floor() as i32;
    let by = py.floor() as i32;
    for &(dx, dy) in &[(0, -1), (0, 1), (-1, 0), (1, 0)] {
        let nx = bx + dx;
        let ny = by + dy;
        if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_H as i32 {
            continue;
        }
        let idx = (ny as u32 * GRID_W + nx as u32) as usize;
        if block_type_rs(grid[idx]) == BT_LOW_WALL && block_height_rs(grid[idx]) > 0 {
            return true;
        }
    }
    false
}

impl App {
    /// Pathfind with water awareness (uses App's water table + elevation).
    pub(crate) fn pathfind(&self, start: (i32, i32), goal: (i32, i32)) -> Vec<(i32, i32)> {
        pleb::astar_path_terrain_water_wd(
            &self.grid_data,
            &self.wall_data,
            &self.terrain_data,
            &self.water_depth_cpu,
            start,
            goal,
        )
    }

    /// Update all simulation state. Returns frame delta time.
    pub(crate) fn update_simulation(&mut self) -> f32 {
        let mut events: Vec<GameEventKind> = Vec::with_capacity(16);

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
                    format!("{} ended", name),
                    "Conditions returning to normal.",
                );
            }
            self.conditions
                .retain(|c| c.remaining > 0.0 || c.duration == 0.0);

            self.drought_check_timer -= dt_game;
            if self.drought_check_timer <= 0.0 {
                self.drought_check_timer = 60.0 + (self.time_of_day * 137.0) % 60.0;
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

        // Dusk warning + night tracking
        {
            let day_frac = (self.time_of_day / DAY_DURATION).rem_euclid(1.0);
            // Dusk warning at 75% of day
            if day_frac > 0.73 && day_frac < 0.77 && !self.dusk_warned {
                self.dusk_warned = true;
                if self.night_count == 0 {
                    self.notify(
                        types::NotifCategory::Warning,
                        "\u{1f319}",
                        "Night approaches",
                        "Duskweavers hunt in darkness. Build walls and light fires.",
                    );
                } else {
                    self.notify(
                        types::NotifCategory::Warning,
                        "\u{1f319}",
                        "Night approaches",
                        "Prepare your defenses.",
                    );
                }
            }
            // Reset dusk warning flag + count night at dawn
            if day_frac > 0.16 && day_frac < 0.20 && self.dusk_warned {
                self.dusk_warned = false;
                self.night_count += 1;
            }
        }

        // --- Contextual hints (one-shot, bit-flagged) ---
        if !self.time_paused {
            let mut hf = self.hint_flags;
            // Hint 0: first hunger drop below 0.5
            if (hf & 1) == 0
                && self
                    .plebs
                    .iter()
                    .any(|p| !p.is_dead && !p.is_enemy && p.needs.hunger < 0.5)
            {
                hf |= 1;
                self.notify(
                    types::NotifCategory::Info,
                    "\u{1f356}",
                    "Getting hungry",
                    "Forage berries from bushes, or hunt dusthares for meat.",
                );
            }
            // Hint 1: first pleb kill (creature dies)
            if (hf & 2) == 0 && self.creatures.iter().any(|c| c.is_dead && !c.dropped_loot) {
                hf |= 2;
                self.notify(
                    types::NotifCategory::Info,
                    "\u{1f52a}",
                    "Fresh kill",
                    "Right-click the carcass to butcher it. Needs a knife. Cook the meat at a campfire.",
                );
            }
            self.hint_flags = hf;
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
                                fresh: true,
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
            // --- Fire fuel consumption ---
            // Campfires and fireplaces consume fuel over time.
            // Height byte = fuel level (1-5). At 0, fire goes out.
            // Check every 120 frames (~2 game-seconds).
            if self.frame_count % 120 == 17 {
                const FUEL_CONSUME_CHANCE: u32 = 8; // ~1 in 8 checks = fuel lasts ~16 game-seconds per level
                let grid_size = (GRID_W * GRID_H) as usize;
                for idx in 0..grid_size {
                    let bt = self.grid_data[idx] & 0xFF;
                    if bt != BT_CAMPFIRE && bt != BT_FIREPLACE {
                        continue;
                    }
                    let fuel = (self.grid_data[idx] >> 8) & 0xFF;
                    if fuel == 0 {
                        continue; // already out
                    }
                    let hash = (idx as u32)
                        .wrapping_mul(2654435761)
                        .wrapping_add(self.frame_count as u32);
                    if hash % FUEL_CONSUME_CHANCE == 0 {
                        let new_fuel = fuel - 1;
                        self.grid_data[idx] = (self.grid_data[idx] & 0xFFFF00FF) | (new_fuel << 8);
                        self.grid_dirty = true;
                    }
                }
            }

            // --- Charcoal mound conversion ---
            // Height byte: 0=empty/done, 1=loaded (needs lighting), 2=smoldering, 3-5=converting
            // Smoldering mounds slowly convert: height decreases over time.
            // When height reaches 0: replace with BT_GROUND + drop charcoal items.
            if self.frame_count % 240 == 33 {
                const MOUND_CONVERT_CHANCE: u32 = 15; // ~1 game-minute per stage
                let grid_size = (GRID_W * GRID_H) as usize;
                for idx in 0..grid_size {
                    let bt = self.grid_data[idx] & 0xFF;
                    if bt != BT_CHARCOAL_MOUND {
                        continue;
                    }
                    let stage = (self.grid_data[idx] >> 8) & 0xFF;
                    if stage < 2 {
                        continue; // not lit yet
                    }
                    let hash = (idx as u32)
                        .wrapping_mul(2654435761)
                        .wrapping_add(self.frame_count as u32);
                    if hash % MOUND_CONVERT_CHANCE == 0 {
                        if stage <= 2 {
                            // Done! Drop charcoal and remove mound
                            let mx = (idx as u32 % GRID_W) as f32 + 0.5;
                            let my = (idx as u32 / GRID_W) as f32 + 0.5;
                            self.ground_items.push(resources::GroundItem::new(
                                mx,
                                my,
                                item_defs::ITEM_CHARCOAL,
                                4, // 4 charcoal per mound
                            ));
                            let roof = self.grid_data[idx] & 0xFF000000;
                            self.grid_data[idx] = make_block(BT_GROUND as u8, 0, 0) | roof;
                            self.grid_dirty = true;
                        } else {
                            let new_stage = stage - 1;
                            self.grid_data[idx] =
                                (self.grid_data[idx] & 0xFFFF00FF) | (new_stage << 8);
                            self.grid_dirty = true;
                        }
                    }
                }
            }

            // Vegetation regrowth: slow, realistic timescales
            // Check every ~4 game-seconds (240 frames) for performance
            // Berry bushes: ~5-7 game-days per berry (1 day = 60s)
            // Dustwhisker: ~8 game-days to respawn on empty ground
            // Hollow Reed: ~18 game-days, only near existing reeds
            // Thornbrake: ~30 game-days, only near existing thornbrake
            if self.frame_count % 240 == 0 {
                let grid_size = (GRID_W * GRID_H) as usize;
                // Chance per check: checks happen every 4s game-time.
                // 1 game-day = 60s = 15 checks. For N-day regrow: chance = 1/(15*N)
                const BERRY_CHANCE: u32 = 90; // 1/90 ≈ 6 game-days per berry
                const WHISKER_CHANCE: u32 = 120; // 1/120 ≈ 8 game-days
                const REED_CHANCE: u32 = 270; // 1/270 ≈ 18 game-days
                const THORN_CHANCE: u32 = 450; // 1/450 ≈ 30 game-days

                for idx in 0..grid_size {
                    let bt = self.grid_data[idx] & 0xFF;
                    let hash = (idx as u32)
                        .wrapping_mul(2654435761)
                        .wrapping_add(self.frame_count as u32);

                    // Berry bush: regrow berries on existing bushes
                    if bt == BT_BERRY_BUSH {
                        let berries = (self.grid_data[idx] >> 8) & 0xFF;
                        if berries < 4 && hash % BERRY_CHANCE == 0 {
                            self.grid_data[idx] =
                                (self.grid_data[idx] & 0xFFFF00FF) | ((berries + 1) << 8);
                            self.grid_dirty = true;
                        }
                        continue;
                    }

                    // Only regrow on empty ground tiles
                    if bt != BT_GROUND {
                        continue;
                    }

                    // Dustwhisker: regrows on open ground (not near trees/rocks)
                    if hash % WHISKER_CHANCE == 0 {
                        // Check terrain suitability: not rocky, not waterlogged
                        let has_neighbor_tree = idx >= GRID_W as usize
                            && (self.grid_data[idx - GRID_W as usize] & 0xFF) == BT_TREE;
                        if !has_neighbor_tree {
                            self.grid_data[idx] = make_block(BT_DUSTWHISKER as u8, 2, 0);
                            self.grid_dirty = true;
                        }
                        continue;
                    }

                    // Hollow Reed: only regrows adjacent to existing reeds
                    if hash % REED_CHANCE == 0 {
                        let x = (idx as u32) % GRID_W;
                        let y = (idx as u32) / GRID_W;
                        let has_neighbor_reed = [
                            idx.wrapping_sub(1),
                            idx + 1,
                            idx.wrapping_sub(GRID_W as usize),
                            idx + GRID_W as usize,
                        ]
                        .iter()
                        .any(|&ni| ni < grid_size && (self.grid_data[ni] & 0xFF) == BT_HOLLOW_REED);
                        if has_neighbor_reed && x > 0 && y > 0 {
                            self.grid_data[idx] = make_block(BT_HOLLOW_REED as u8, 3, 0);
                            self.grid_dirty = true;
                        }
                        continue;
                    }

                    // Thornbrake: only regrows adjacent to existing thornbrake
                    if hash % THORN_CHANCE == 0 {
                        let has_neighbor_thorn = [
                            idx.wrapping_sub(1),
                            idx + 1,
                            idx.wrapping_sub(GRID_W as usize),
                            idx + GRID_W as usize,
                        ]
                        .iter()
                        .any(|&ni| ni < grid_size && (self.grid_data[ni] & 0xFF) == BT_THORNBRAKE);
                        if has_neighbor_thorn {
                            self.grid_data[idx] = make_block(BT_THORNBRAKE as u8, 2, 0);
                            self.grid_dirty = true;
                        }
                    }
                    // Saltbrush and Duskbloom: no regrowth (finite resources)
                }
            }

            // --- Food spoilage: decay freshness (throttled to every 15 frames) ---
            if self.frame_count % 15 == 7 {
                let item_reg = item_defs::ItemRegistry::cached();
                let dt_game = dt * self.time_speed * 15.0; // compensate for throttle
                // Ground items: full spoil rate
                let mut spoiled_ground = Vec::new();
                for (i, gi) in self.ground_items.iter_mut().enumerate() {
                    if let Some(def) = item_reg.get(gi.stack.item_id) {
                        if def.spoil_time > 0.0 {
                            gi.stack.freshness -= dt_game / def.spoil_time;
                            if gi.stack.freshness <= 0.0 {
                                spoiled_ground.push(i);
                            }
                        }
                    }
                }
                for &i in spoiled_ground.iter().rev() {
                    let name = item_reg
                        .name(self.ground_items[i].stack.item_id)
                        .to_string();
                    self.ground_items.remove(i);
                    self.notify(
                        types::NotifCategory::Warning,
                        "\u{26a0}",
                        "Spoiled",
                        &format!("{} spoiled on the ground", name),
                    );
                }
                // Pleb inventory: full spoil rate
                for pleb in &mut self.plebs {
                    if pleb.is_dead {
                        continue;
                    }
                    let mut spoiled = false;
                    for s in &mut pleb.inventory.stacks {
                        if let Some(def) = item_reg.get(s.item_id) {
                            if def.spoil_time > 0.0 {
                                s.freshness -= dt_game / def.spoil_time;
                                if s.freshness <= 0.0 {
                                    spoiled = true;
                                }
                            }
                        }
                    }
                    if spoiled {
                        pleb.inventory.stacks.retain(|s| {
                            let def = item_reg.get(s.item_id);
                            let perishable = def.is_some_and(|d| d.spoil_time > 0.0);
                            !perishable || s.freshness > 0.0
                        });
                        pleb.log_event(self.time_of_day, "Food spoiled".into());
                        pleb.set_bubble(pleb::BubbleKind::Thought("Food went bad...".into()), 2.0);
                    }
                }
                // Crate contents: half spoil rate (storage benefit)
                for cinv in self.crate_contents.values_mut() {
                    cinv.stacks.iter_mut().for_each(|s| {
                        if let Some(def) = item_reg.get(s.item_id) {
                            if def.spoil_time > 0.0 {
                                s.freshness -= dt_game / def.spoil_time * 0.5;
                            }
                        }
                    });
                    cinv.stacks.retain(|s| s.freshness > 0.0 || s.count == 0);
                }
            } // end spoilage throttle

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
            FluidOverlay::Dust => 16.0,
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
                            fresh: true,
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

        // --- Enemy random walk AI (with cover-seeking) ---
        // Collect friendly positions for threat detection
        let friendly_positions: Vec<(f32, f32)> = self
            .plebs
            .iter()
            .filter(|p| !p.is_enemy && !p.is_dead)
            .map(|p| (p.x, p.y))
            .collect();

        for pleb in self.plebs.iter_mut() {
            if !pleb.is_enemy {
                continue;
            }
            pleb.wander_timer -= dt * self.time_speed;

            // Wounded retreat: badly hurt enemies flee from combat
            if pleb.needs.health < 0.35 && pleb.needs.health > 0.0 && !pleb.is_dead {
                if let Some(&(fx, fy)) = friendly_positions.first() {
                    // Find nearest friendly for flee direction
                    let mut nearest_d = f32::MAX;
                    let mut flee_from = (fx, fy);
                    for &(ffx, ffy) in &friendly_positions {
                        let d = ((pleb.x - ffx).powi(2) + (pleb.y - ffy).powi(2)).sqrt();
                        if d < nearest_d {
                            nearest_d = d;
                            flee_from = (ffx, ffy);
                        }
                    }
                    if nearest_d < 25.0 && pleb.path_idx >= pleb.path.len() {
                        // Flee in opposite direction from nearest friendly
                        let flee_dx = pleb.x - flee_from.0;
                        let flee_dy = pleb.y - flee_from.1;
                        let flee_len = (flee_dx * flee_dx + flee_dy * flee_dy).sqrt().max(0.1);
                        let flee_x = (pleb.x + flee_dx / flee_len * 15.0)
                            .clamp(1.0, GRID_W as f32 - 2.0)
                            as i32;
                        let flee_y = (pleb.y + flee_dy / flee_len * 15.0)
                            .clamp(1.0, GRID_H as f32 - 2.0)
                            as i32;
                        let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                        let path = pleb::astar_path_terrain_water_wd(
                            &self.grid_data,
                            &self.wall_data,
                            &self.terrain_data,
                            &self.water_depth_cpu,
                            start,
                            (flee_x, flee_y),
                        );
                        if !path.is_empty() {
                            pleb.path = path;
                            pleb.path_idx = 0;
                            pleb.aim_target = None;
                            pleb.aim_progress = 0.0;
                            pleb.set_bubble(pleb::BubbleKind::Text("Retreating!".into()), 2.0);
                        }
                        pleb.wander_timer = 5.0;
                        continue; // skip normal wander logic
                    }
                }
            }

            if pleb.wander_timer <= 0.0 && pleb.path_idx >= pleb.path.len() {
                // Check for nearby threats
                let nearest_threat: Option<(f32, f32, f32)> = friendly_positions
                    .iter()
                    .map(|&(fx, fy)| {
                        let d = ((pleb.x - fx).powi(2) + (pleb.y - fy).powi(2)).sqrt();
                        (fx, fy, d)
                    })
                    .filter(|&(_, _, d)| d < 25.0)
                    .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap());

                // If under threat and has ranged weapon, seek cover
                // But if already at a cover position, stay put (don't re-pathfind)
                let already_at_cover = if let Some((threat_x, threat_y, _)) = nearest_threat {
                    is_behind_cover(
                        &self.grid_data,
                        &self.wall_data,
                        pleb.x,
                        pleb.y,
                        threat_x,
                        threat_y,
                    )
                } else {
                    false
                };

                if already_at_cover {
                    // Stay at cover, just reset timer
                    pleb.wander_timer = 3.0;
                } else {
                    let sought_cover = if let Some((threat_x, threat_y, _)) = nearest_threat {
                        if pleb.prefer_ranged {
                            find_cover_position(
                                &self.grid_data,
                                pleb.x,
                                pleb.y,
                                threat_x,
                                threat_y,
                                8,
                            )
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if let Some((cx, cy)) = sought_cover {
                        let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                        let path = pleb::astar_path_terrain_water_wd(
                            &self.grid_data,
                            &self.wall_data,
                            &self.terrain_data,
                            &self.water_depth_cpu,
                            start,
                            (cx, cy),
                        );
                        if !path.is_empty() {
                            pleb.path = path;
                            pleb.path_idx = 0;
                        }
                        pleb.wander_timer = 3.0;
                    } else {
                        // Normal random wander
                        let seed = ((pleb.x * 137.0 + pleb.y * 311.0 + self.time_of_day * 1000.0)
                            as u32)
                            .wrapping_mul(2654435761);
                        let dx = ((seed & 0xFF) as f32 / 255.0 - 0.5) * 16.0;
                        let dy = (((seed >> 8) & 0xFF) as f32 / 255.0 - 0.5) * 16.0;
                        let target_x = (pleb.x + dx).clamp(1.0, GRID_W as f32 - 2.0) as i32;
                        let target_y = (pleb.y + dy).clamp(1.0, GRID_H as f32 - 2.0) as i32;
                        let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                        let path = pleb::astar_path_terrain_water_wd(
                            &self.grid_data,
                            &self.wall_data,
                            &self.terrain_data,
                            &self.water_depth_cpu,
                            start,
                            (target_x, target_y),
                        );
                        if !path.is_empty() {
                            pleb.path = path;
                            pleb.path_idx = 0;
                        }
                    }
                    // Next wander in 5-15 seconds
                    let timer_seed = ((pleb.x * 137.0 + pleb.y * 311.0 + self.time_of_day * 1000.0)
                        as u32)
                        .wrapping_mul(2654435761);
                    pleb.wander_timer = 5.0 + ((timer_seed >> 16) & 0xFF) as f32 / 255.0 * 10.0;
                } // end else (not already at cover)
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

        // --- Flocking: compute group movement adjustments ---
        let flock_enemy_pos: Vec<(f32, f32)> = self
            .plebs
            .iter()
            .filter(|p| p.is_enemy && !p.is_dead)
            .map(|p| (p.x, p.y))
            .collect();
        let flock_adjustments =
            comms::compute_flocking(&self.plebs, &flock_enemy_pos, self.flock_spacing);

        // --- Update all plebs ---
        let move_speed = 8.0f32; // tiles/sec base (NOT scaled by time_speed)
        let sel = self.selected_pleb;

        for (i, pleb) in self.plebs.iter_mut().enumerate() {
            if pleb.is_dead {
                continue;
            } // corpses don't act

            // Tick down stagger (hit stun)
            if pleb.stagger_timer > 0.0 {
                pleb.stagger_timer = (pleb.stagger_timer - dt * self.time_speed).max(0.0);
            }
            if pleb.weapon_swap_timer > 0.0 {
                pleb.weapon_swap_timer = (pleb.weapon_swap_timer - dt * self.time_speed).max(0.0);
            }
            if pleb.suppression > 0.0 {
                pleb.suppression = (pleb.suppression - dt * 0.5).max(0.0);
            }
            // Nausea timer decay
            if pleb.nauseous_timer > 0.0 {
                pleb.nauseous_timer = (pleb.nauseous_timer - dt * self.time_speed).max(0.0);
            }
            // Command cooldown
            if pleb.command_cooldown > 0.0 {
                pleb.command_cooldown = (pleb.command_cooldown - dt * self.time_speed).max(0.0);
            }
            // Firefight tracking: detect combat end after 5s with no target
            if pleb.aim_target.is_some() || pleb.aim_pos.is_some() {
                pleb.combat_participated = true;
                pleb.no_enemy_timer = 0.0;
            } else if pleb.combat_participated {
                pleb.no_enemy_timer += dt * self.time_speed;
                if pleb.no_enemy_timer > 5.0 {
                    // Firefight ended — survived
                    pleb.firefights_survived = pleb.firefights_survived.saturating_add(1);
                    pleb.combat_participated = false;
                    pleb.no_enemy_timer = 0.0;
                }
            }
            // Smooth crouch transition (~0.25s)
            {
                let target = if pleb.crouching { 1.0f32 } else { 0.0 };
                let speed = 4.0 * self.time_speed; // 1/0.25s
                if pleb.crouch_progress < target {
                    pleb.crouch_progress = (pleb.crouch_progress + dt * speed).min(target);
                } else if pleb.crouch_progress > target {
                    pleb.crouch_progress = (pleb.crouch_progress - dt * speed).max(target);
                }
            }

            // Combat stress: suppression builds stress, safe conditions recover it
            morale::tick_suppression_stress(pleb, dt * self.time_speed);

            // Tick down bubble timer
            if let Some((_, ref mut timer)) = pleb.bubble {
                *timer -= dt * self.time_speed;
                if *timer <= 0.0 {
                    pleb.bubble = None;
                }
            }

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
                    let target_angle = ndy.atan2(ndx);
                    pleb.angle =
                        lerp_angle(pleb.angle, target_angle, (TURN_SPEED_WALK * dt).min(1.0));
                    // Injury slowdown: sqrt(health) curve — 50% HP → 71% speed, 25% HP → 50% speed
                    let injury_mul = pleb.needs.health.clamp(0.05, 1.0).sqrt();
                    // Bleeding drag: heavy bleeding slows further
                    let bleed_mul = 1.0 - pleb.bleeding * 0.3; // bleeding 1.0 → 70% speed
                    // Stagger: frozen briefly after being hit
                    let stagger_mul = if pleb.stagger_timer > 0.0 { 0.0 } else { 1.0 };
                    // Crouch: 40% speed when crouch-walking
                    let crouch_mul = if pleb.crouching { 0.4 } else { 1.0 };
                    // Water: smooth speed gradient based on depth
                    // Puddles (<0.05) = no penalty, ankle (0.1) = 85%, knee (0.3) = 40%, waist (0.6) = 10%
                    let water_mul = {
                        let wx = pleb.x.floor() as i32;
                        let wy = pleb.y.floor() as i32;
                        if wx >= 0 && wy >= 0 && wx < GRID_W as i32 && wy < GRID_H as i32 {
                            let widx = (wy as u32 * GRID_W + wx as u32) as usize;
                            if widx < self.water_depth_cpu.len() {
                                let wd = self.water_depth_cpu[widx];
                                if wd > 0.05 {
                                    // Smooth curve: 1.0 at depth 0.05 → 0.08 at depth 1.0
                                    (1.0 - ((wd - 0.05) / 0.95).min(1.0)).powi(2) * 0.92 + 0.08
                                } else {
                                    1.0
                                }
                            } else {
                                1.0
                            }
                        } else {
                            1.0
                        }
                    };
                    let wet_mul = needs::wetness_speed_mult(pleb.needs.wetness);
                    // Movement speed is NOT multiplied by time_speed — plebs move at
                    // a fixed visual rate. Only activities/day-night scale with sim speed.
                    let effective_speed = move_speed
                        * speed_mul
                        * injury_mul
                        * bleed_mul
                        * stagger_mul
                        * crouch_mul
                        * water_mul
                        * wet_mul;
                    // Apply flocking adjustment (separation/cohesion forces)
                    let flock = flock_adjustments.iter().find(|a| a.pleb_idx == i);
                    let flock_speed = flock.map(|a| a.speed_mul).unwrap_or(1.0);
                    let flock_dx = flock.map(|a| a.dx).unwrap_or(0.0) * dt;
                    let flock_dy = flock.map(|a| a.dy).unwrap_or(0.0) * dt;
                    let raw_step = effective_speed * flock_speed * dt;
                    // Clamp step to not overshoot the waypoint (prevents oscillation)
                    let clamped_step = raw_step.min(dist);
                    let step_x = ndx * clamped_step + flock_dx;
                    let step_y = ndy * clamped_step + flock_dy;
                    let nx = pleb.x + step_x;
                    let ny = pleb.y + step_y;
                    // Check walkability AND wall edge crossings
                    let old_tx = pleb.x.floor() as i32;
                    let old_ty = pleb.y.floor() as i32;
                    let can_move = |mx: f32, my: f32| -> bool {
                        if !is_walkable_pos_wd(&self.grid_data, &self.wall_data, mx, my) {
                            return false;
                        }
                        // Very deep water (>1.0) blocks movement entirely
                        let wdx = mx.floor() as i32;
                        let wdy = my.floor() as i32;
                        if wdx >= 0 && wdy >= 0 && wdx < GRID_W as i32 && wdy < GRID_H as i32 {
                            let widx = (wdy as u32 * GRID_W + wdx as u32) as usize;
                            if widx < self.water_depth_cpu.len() && self.water_depth_cpu[widx] > 1.0
                            {
                                return false;
                            }
                        }
                        // Check if movement crosses a tile boundary with a wall edge
                        let new_tx = mx.floor() as i32;
                        let new_ty = my.floor() as i32;
                        if (new_tx != old_tx || new_ty != old_ty)
                            && edge_blocked_wd(
                                &self.grid_data,
                                &self.wall_data,
                                old_tx,
                                old_ty,
                                new_tx,
                                new_ty,
                            )
                        {
                            return false;
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
            } else {
                // Idle pleb: still apply separation force to prevent stacking
                if let Some(flock) = flock_adjustments.iter().find(|a| a.pleb_idx == i) {
                    let fdx = flock.dx * dt;
                    let fdy = flock.dy * dt;
                    if fdx.abs() > 0.0001 || fdy.abs() > 0.0001 {
                        let nx = pleb.x + fdx;
                        let ny = pleb.y + fdy;
                        if is_walkable_pos_wd(&self.grid_data, &self.wall_data, nx, ny) {
                            pleb.x = nx;
                            pleb.y = ny;
                        }
                    }
                }
            }
        }

        // --- Update pleb needs and auto-behaviors ---
        {
            // Collect enemy positions for flee detection
            let enemy_pos_for_flee: Vec<(f32, f32)> = self
                .plebs
                .iter()
                .filter(|p| p.is_enemy && !p.is_dead)
                .map(|p| (p.x, p.y))
                .collect();

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
                    self.camera.rain_intensity,
                );

                // --- Trait modifiers applied after needs tick ---
                if !pleb.is_enemy {
                    let t = dt * self.time_speed;
                    // DesertBlood: thirst decays 30% slower (add back 30% of what was lost)
                    if pleb.has_trait("Desert Blood") {
                        pleb.needs.thirst = (pleb.needs.thirst + 0.004 * 0.3 * t).min(1.0);
                    }
                    // Weathered: health capped at 1.25x (absorb more damage)
                    // (Applied as effective health — stored as >1.0 for display)
                }

                // --- Smoke discomfort: mood penalty, cough, speed reduction ---
                if !pleb.is_enemy && !pleb.is_dead {
                    let smoke_level = air.map(|a| a.smoke).unwrap_or(0.0);
                    pleb.smoke_exposure = smoke_level;
                    if smoke_level > 0.1 {
                        let t = dt * self.time_speed;
                        if smoke_level > 0.5 {
                            // Suffocating: health damage
                            pleb.needs.health -= 0.005 * t;
                            pleb.needs.mood -= 0.5 * t;
                        } else if smoke_level > 0.3 {
                            // Choking: mood penalty + work speed implied by mood
                            pleb.needs.mood -= 0.3 * t;
                        } else {
                            // Eyes watering: mild mood hit
                            pleb.needs.mood -= 0.1 * t;
                        }
                        // Cough bubble (throttled: only when no bubble active)
                        if smoke_level > 0.2 && pleb.bubble.is_none() {
                            // Random cough timing based on smoke density
                            let cough_seed =
                                (pleb.x * 97.3 + pleb.y * 211.7 + self.time_of_day * 1000.0) as u32;
                            let cough_roll =
                                (cough_seed.wrapping_mul(2654435761) & 0xFFFF) as f32 / 65535.0;
                            // Higher smoke = more frequent coughs
                            if cough_roll < smoke_level * 0.02 {
                                pleb.set_bubble(pleb::BubbleKind::Thought("*cough*".into()), 1.5);
                            }
                        }
                    }
                }

                // --- Need emotes: thought bubbles when needs drop below thresholds ---
                if !pleb.is_enemy && !pleb.is_dead {
                    let mut f = pleb.need_emote_flags;
                    let gt = self.time_of_day;
                    let px = pleb.x;
                    let py = pleb.y;
                    // Hunger: low=0.35, critical=0.12
                    if pleb.needs.hunger < 0.35 && (f & 1) == 0 {
                        f |= 1;
                        pleb.set_bubble(pleb::BubbleKind::Thought("Getting hungry...".into()), 3.0);
                        pleb.log_event(gt, "Feeling hungry".into());
                        if self.sound_enabled {
                            self.sound_sources.push(types::SoundSource {
                                x: px,
                                y: py,
                                amplitude: types::db_to_amplitude(45.0),
                                frequency: 500.0,
                                phase: 0.0,
                                pattern: 2,
                                duration: 0.08,
                                fresh: true,
                            });
                        }
                    }
                    if pleb.needs.hunger < 0.12 && (f & 2) == 0 {
                        f |= 2;
                        pleb.set_bubble(pleb::BubbleKind::Thought("So hungry...".into()), 3.0);
                        pleb.log_event(gt, "Starving!".into());
                    }
                    if pleb.needs.hunger > 0.5 {
                        f &= !3;
                    } // reset hunger flags on recovery
                    // Thirst: low=0.35, critical=0.12
                    if pleb.needs.thirst < 0.35 && (f & 4) == 0 {
                        f |= 4;
                        pleb.set_bubble(pleb::BubbleKind::Thought("Need water...".into()), 3.0);
                        pleb.log_event(gt, "Feeling thirsty".into());
                        if self.sound_enabled {
                            self.sound_sources.push(types::SoundSource {
                                x: px,
                                y: py,
                                amplitude: types::db_to_amplitude(45.0),
                                frequency: 600.0,
                                phase: 0.0,
                                pattern: 2,
                                duration: 0.08,
                                fresh: true,
                            });
                        }
                    }
                    if pleb.needs.thirst < 0.12 && (f & 8) == 0 {
                        f |= 8;
                        pleb.set_bubble(pleb::BubbleKind::Thought("So thirsty...".into()), 3.0);
                        pleb.log_event(gt, "Dehydrating!".into());
                    }
                    if pleb.needs.thirst > 0.5 {
                        f &= !12;
                    }
                    // Rest: low=0.25, critical=0.10
                    if pleb.needs.rest < 0.25 && (f & 16) == 0 {
                        f |= 16;
                        pleb.set_bubble(pleb::BubbleKind::Thought("Tired...".into()), 3.0);
                        pleb.log_event(gt, "Exhausted".into());
                    }
                    if pleb.needs.rest > 0.4 {
                        f &= !48;
                    }
                    // Warmth: low=0.25
                    if pleb.needs.warmth < 0.25 && (f & 64) == 0 {
                        f |= 64;
                        pleb.set_bubble(pleb::BubbleKind::Thought("So cold...".into()), 3.0);
                        pleb.log_event(gt, "Freezing".into());
                    }
                    if pleb.needs.warmth > 0.4 {
                        f &= !192;
                    }
                    pleb.need_emote_flags = f;

                    // Wetness emotes (separate flag field)
                    let w = pleb.needs.wetness;
                    let wf = pleb.wetness_emote;
                    if w > 0.5 && (wf & 1) == 0 {
                        pleb.wetness_emote |= 1;
                        pleb.set_bubble(pleb::BubbleKind::Thought("Getting soaked...".into()), 3.0);
                        pleb.log_event(gt, "Getting wet in the rain".into());
                    }
                    if w > 0.8 && (wf & 2) == 0 {
                        pleb.wetness_emote |= 2;
                        pleb.set_bubble(pleb::BubbleKind::Thought("Drenched!".into()), 3.0);
                        pleb.log_event(gt, "Soaked to the bone".into());
                    }
                    // Reset when dried off
                    if w < 0.2 {
                        pleb.wetness_emote = 0;
                    }
                }

                // Undrafted flee: civilians run to safety when enemies are near
                if !pleb.is_enemy
                    && !pleb.drafted
                    && !pleb.is_dead
                    && !pleb.activity.is_crisis()
                    && matches!(pleb.activity, PlebActivity::Idle | PlebActivity::Walking)
                {
                    let nearest_enemy_dist = enemy_pos_for_flee
                        .iter()
                        .map(|&(ex, ey)| ((pleb.x - ex).powi(2) + (pleb.y - ey).powi(2)).sqrt())
                        .fold(f32::MAX, f32::min);

                    if nearest_enemy_dist < 15.0 {
                        // Find nearest roofed (indoor) tile within 20 tiles
                        let bx = pleb.x.floor() as i32;
                        let by = pleb.y.floor() as i32;
                        let mut best_indoor: Option<(i32, i32, f32)> = None;
                        for dy in -20..=20i32 {
                            for ddx in -20..=20i32 {
                                let cx = bx + ddx;
                                let cy = by + dy;
                                if cx < 0 || cy < 0 || cx >= GRID_W as i32 || cy >= GRID_H as i32 {
                                    continue;
                                }
                                let idx = (cy as u32 * GRID_W + cx as u32) as usize;
                                if roof_height_rs(self.grid_data[idx]) > 0 {
                                    let d = (ddx * ddx + dy * dy) as f32;
                                    if best_indoor.is_none_or(|(_, _, bd)| d < bd) {
                                        best_indoor = Some((cx, cy, d));
                                    }
                                }
                            }
                        }
                        if let Some((ix, iy, _)) = best_indoor {
                            let start = (bx, by);
                            let path = pleb::astar_path_terrain_water_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                &self.water_depth_cpu,
                                start,
                                (ix, iy),
                            );
                            if !path.is_empty() {
                                pleb.path = path;
                                pleb.path_idx = 0;
                                pleb.activity = PlebActivity::Crisis(
                                    Box::new(PlebActivity::Walking),
                                    "taking_cover",
                                );
                                pleb.clear_targets();
                                pleb.set_bubble(pleb::BubbleKind::Icon('!', [200, 40, 40]), 2.0);
                            }
                        }
                    }
                }

                let was_crisis = pleb.activity.is_crisis();
                tick_pleb_activity(
                    pleb,
                    &env,
                    &mut self.grid_data,
                    &self.wall_data,
                    &self.terrain_data,
                    dt,
                    self.time_speed,
                    &mut self.ground_items,
                    self.time_of_day,
                );
                // Log new crisis
                if pleb.activity.is_crisis()
                    && !was_crisis
                    && let Some(reason) = pleb.activity.crisis_reason()
                {
                    events.push(GameEventKind::CrisisStarted {
                        pleb: pleb.name.clone(),
                        reason,
                    });
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
                    // Phase 1: at pickup location → pick up item
                    // at_pickup required (not just path_done) so carrying plebs
                    // redirected to fetch more don't skip pickup
                    if at_pickup {
                        if let Some((rx, ry)) = pleb.harvest_target {
                            let ridx = (ry as u32 * GRID_W + rx as u32) as usize;
                            let is_rock =
                                ridx < self.grid_data.len() && (self.grid_data[ridx] & 0xFF) == 34;
                            if is_rock {
                                pleb.draw_tool("pick");
                                let roof_bits = self.grid_data[ridx] & 0xFF000000;
                                let flag_bits = (self.grid_data[ridx] >> 16) & 2;
                                self.grid_data[ridx] =
                                    make_block(2, 0, flag_bits as u8) | roof_bits;
                                self.grid_dirty = true;
                                // Stone Pick: quarry 4 rock, bare hands: 1 rock
                                let has_pick = pleb.has_tool("pick");
                                let base_yield: u16 = if has_pick { 4 } else { 1 };
                                let rock_yield =
                                    (base_yield as f32 * pleb.mining_yield_mult()).ceil() as u16;
                                pleb.inventory.add(ITEM_ROCK, rock_yield);
                                pleb.harvest_target = None;
                                events.push(GameEventKind::PickedUp {
                                    pleb: pleb.name.clone(),
                                    count: rock_yield,
                                    item: "rock".into(),
                                });
                                pleb.gain_xp_logged(
                                    pleb::SKILL_CONSTRUCTION,
                                    10.0,
                                    self.time_of_day,
                                );
                            } else if let Some(wi) = {
                                // Prefer the specific item type the blueprint needs
                                let prefer_id: Option<u16> =
                                    pleb.haul_target.and_then(|(cx, cy)| {
                                        self.blueprints.get(&(cx, cy)).and_then(|bp| {
                                            if bp.is_roof() {
                                                Some(ITEM_FIBER)
                                            } else if bp.uses_sticks() {
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
                                // Auto-equip if picked up a weapon/tool
                                if pleb.equipped_weapon.is_none() {
                                    pleb.update_equipped_weapon();
                                }
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
                                    let path = pleb::astar_path_terrain_water_wd(
                                        &self.grid_data,
                                        &self.wall_data,
                                        &self.terrain_data,
                                        &self.water_depth_cpu,
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
                            let bp_ref = self.blueprints.get(&(cx, cy));
                            let is_blueprint = bp_ref.is_some();
                            let bp_is_roof = bp_ref.is_some_and(|bp| bp.is_roof());
                            let is_crate = {
                                let ci = (cy as u32 * GRID_W + cx as u32) as usize;
                                ci < self.grid_data.len()
                                    && block_type_rs(self.grid_data[ci]) == BT_CRATE
                            };
                            if is_blueprint {
                                // Deliver materials to blueprint
                                if let Some(bp) = self.blueprints.get_mut(&(cx, cy)) {
                                    if bp.wood_delivered < bp.wood_needed {
                                        let is_cf = bp.uses_sticks();
                                        let have = if is_cf {
                                            pleb.inventory.count_of(ITEM_SCRAP_WOOD) as u32
                                        } else {
                                            pleb.inventory.wood()
                                        };
                                        let deliver = have.min(bp.wood_needed - bp.wood_delivered);
                                        bp.wood_delivered += deliver;
                                        if is_cf {
                                            pleb.inventory.remove(ITEM_SCRAP_WOOD, deliver as u16);
                                        } else {
                                            // Consume logs first, then wood
                                            let mut remaining = deliver as u16;
                                            let log_take = remaining
                                                .min(pleb.inventory.count_of(ITEM_LOG) as u16);
                                            if log_take > 0 {
                                                pleb.inventory.remove(ITEM_LOG, log_take);
                                                remaining -= log_take;
                                            }
                                            if remaining > 0 {
                                                pleb.inventory.remove(ITEM_WOOD, remaining);
                                            }
                                        }
                                        if deliver > 0 {
                                            let mat = if is_cf { "sticks" } else { "wood" };
                                            events.push(GameEventKind::Delivered {
                                                pleb: pleb.name.clone(),
                                                material: mat,
                                                amount: deliver,
                                            });
                                        }
                                    }
                                    if bp.clay_delivered < bp.clay_needed {
                                        let have = pleb.inventory.count_of(ITEM_CLAY);
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
                                        let have = pleb.inventory.count_of(ITEM_PLANK);
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
                                        let have = pleb.inventory.count_of(ITEM_ROCK);
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
                                        let have = pleb.inventory.count_of(ITEM_ROPE);
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
                                    self.blueprints.get(&(cx, cy)).is_some_and(|bp| {
                                        if bp.is_roof() {
                                            pleb.inventory.count_of(ITEM_FIBER) >= 1
                                        } else {
                                            bp.resources_met()
                                        }
                                    });
                                if start_building {
                                    if bp_is_roof {
                                        pleb.inventory.remove(ITEM_FIBER, 1);
                                    }
                                    pleb.activity = PlebActivity::Building(0.0);
                                    pleb.work_target = Some((cx, cy));
                                    self.active_work.insert((cx, cy));
                                } else if bp_is_roof {
                                    // Not enough special materials — fetch more immediately
                                    let fetch_id = ITEM_FIBER;
                                    // Find nearest matching ground item
                                    let mut nearest: Option<(i32, i32, f32)> = None;
                                    for gi in self.ground_items.iter() {
                                        if gi.stack.item_id == fetch_id {
                                            let d = (gi.x - cx as f32 - 0.5).powi(2)
                                                + (gi.y - cy as f32 - 0.5).powi(2);
                                            if nearest.is_none_or(|(_, _, bd)| d < bd) {
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
                                // Deposit carried items in crate, but keep equipped items
                                let cidx = cy as u32 * GRID_W + cx as u32;
                                let inv = self.crate_contents.entry(cidx).or_default();
                                let mut keep = Vec::new();
                                let carried: Vec<ItemStack> =
                                    pleb.inventory.stacks.drain(..).collect();
                                for stack in carried {
                                    if pleb.is_equipped(stack.item_id) {
                                        keep.push(stack);
                                    } else if !inv.add_stack(stack.clone()) {
                                        pleb.inventory.add_stack(stack);
                                    }
                                }
                                for s in keep {
                                    pleb.inventory.stacks.push(s);
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
                                // Check if only equipped items remain
                                let only_equipped = pleb
                                    .inventory
                                    .stacks
                                    .iter()
                                    .all(|s| pleb.is_equipped(s.item_id));
                                if !pleb.inventory.is_carrying() || only_equipped {
                                    // All non-equipped items deposited
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
                                        let path = pleb::astar_path_terrain_water_wd(
                                            &self.grid_data,
                                            &self.wall_data,
                                            &self.terrain_data,
                                            &self.water_depth_cpu,
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
                                        // No other crate — drop remaining non-equipped items
                                        let to_drop: Vec<ItemStack> =
                                            pleb.inventory.stacks.drain(..).collect();
                                        let mut keep = Vec::new();
                                        for stack in to_drop {
                                            if pleb.is_equipped(stack.item_id) {
                                                keep.push(stack);
                                            } else {
                                                self.ground_items.push(resources::GroundItem {
                                                    x: cx as f32 + 0.5,
                                                    y: cy as f32 + 0.5,
                                                    stack,
                                                });
                                            }
                                        }
                                        pleb.inventory.stacks = keep;
                                        pleb.haul_target = None;
                                        pleb.activity = PlebActivity::Idle;
                                        events.push(GameEventKind::Dropped(pleb.name.clone()));
                                    }
                                }
                            } else {
                                // Drop at storage zone tile (keep equipped)
                                let to_drop: Vec<ItemStack> =
                                    pleb.inventory.stacks.drain(..).collect();
                                let mut keep = Vec::new();
                                for stack in to_drop {
                                    if pleb.is_equipped(stack.item_id) {
                                        keep.push(stack);
                                    } else {
                                        self.ground_items.push(resources::GroundItem {
                                            x: cx as f32 + 0.5,
                                            y: cy as f32 + 0.5,
                                            stack,
                                        });
                                    }
                                }
                                pleb.inventory.stacks = keep;
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
                            let path = pleb::astar_path_terrain_water_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                &self.water_depth_cpu,
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
                        let path = pleb::astar_path_terrain_water_wd(
                            &self.grid_data,
                            &self.wall_data,
                            &self.terrain_data,
                            &self.water_depth_cpu,
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
                            let path = pleb::astar_path_terrain_water_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                &self.water_depth_cpu,
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
                            let path = pleb::astar_path_terrain_water_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                &self.water_depth_cpu,
                                start,
                                adj,
                            );
                            if !path.is_empty() {
                                pleb.path = path;
                                pleb.path_idx = 0;
                                pleb.activity = PlebActivity::Walking;
                                pleb.work_target = Some((hx, hy));
                                pleb.harvest_target = Some((hx, hy));
                                pleb.haul_target = None;
                            }
                        }
                        PlebCommand::Haul(hx, hy) => {
                            let crate_target = find_nearest_crate(&self.grid_data, hx, hy);
                            let path = pleb::astar_path_terrain_water_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                &self.water_depth_cpu,
                                start,
                                (hx, hy),
                            );
                            if !path.is_empty() {
                                pleb.path = path;
                                pleb.path_idx = 0;
                                pleb.activity = PlebActivity::Hauling;
                                pleb.haul_target = crate_target;
                                pleb.harvest_target = Some((hx, hy));
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
                                let path = pleb::astar_path_terrain_water_wd(
                                    &self.grid_data,
                                    &self.wall_data,
                                    &self.terrain_data,
                                    &self.water_depth_cpu,
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
                            let path = pleb::astar_path_terrain_water_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                &self.water_depth_cpu,
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
                            let path = pleb::astar_path_terrain_water_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                &self.water_depth_cpu,
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
                        PlebCommand::Butcher(bx, by) => {
                            let dist = ((pleb.x - bx as f32 - 0.5).powi(2)
                                + (pleb.y - by as f32 - 0.5).powi(2))
                            .sqrt();
                            if dist < 1.8 && pleb.has_tool("knife") {
                                pleb.draw_tool("knife");
                                pleb.activity = PlebActivity::Butchering(0.0);
                                pleb.work_target = Some((bx, by));
                                pleb.path.clear();
                            } else if dist < 1.8 {
                                // No knife — can't butcher
                                pleb.set_bubble(
                                    pleb::BubbleKind::Thought("Need a knife...".into()),
                                    2.5,
                                );
                                pleb.log_event(self.time_of_day, "Can't butcher — no knife".into());
                                pleb.activity = PlebActivity::Idle;
                                pleb.work_target = None;
                            } else {
                                let path = pleb::astar_path_terrain_water_wd(
                                    &self.grid_data,
                                    &self.wall_data,
                                    &self.terrain_data,
                                    &self.water_depth_cpu,
                                    start,
                                    (bx, by),
                                );
                                if !path.is_empty() {
                                    pleb.path = path;
                                    pleb.path_idx = 0;
                                    pleb.activity = PlebActivity::Walking;
                                    pleb.work_target = Some((bx, by));
                                }
                            }
                        }
                        PlebCommand::Fish(fx, fy) => {
                            let dist = ((pleb.x - fx as f32 - 0.5).powi(2)
                                + (pleb.y - fy as f32 - 0.5).powi(2))
                            .sqrt();
                            let has_line = pleb.has_tool("fishing")
                                || pleb.inventory.count_of(ITEM_FISHING_LINE) > 0;
                            if dist < 1.8 && has_line {
                                pleb.draw_tool("fishing");
                                pleb.activity = PlebActivity::Fishing(0.0);
                                pleb.work_target = Some((fx, fy));
                                pleb.path.clear();
                            } else if dist < 1.8 {
                                pleb.set_bubble(
                                    pleb::BubbleKind::Thought("Need a fishing line...".into()),
                                    2.5,
                                );
                                pleb.log_event(
                                    self.time_of_day,
                                    "Can't fish — no fishing line".into(),
                                );
                                pleb.activity = PlebActivity::Idle;
                                pleb.work_target = None;
                            } else {
                                let adj =
                                    adjacent_walkable(&self.grid_data, fx, fy).unwrap_or((fx, fy));
                                let path = pleb::astar_path_terrain_water_wd(
                                    &self.grid_data,
                                    &self.wall_data,
                                    &self.terrain_data,
                                    &self.water_depth_cpu,
                                    start,
                                    adj,
                                );
                                if !path.is_empty() {
                                    pleb.path = path;
                                    pleb.path_idx = 0;
                                    pleb.activity = PlebActivity::Walking;
                                    pleb.work_target = Some((fx, fy));
                                }
                            }
                        }
                        PlebCommand::Mine(mx, my) => {
                            let adj =
                                adjacent_walkable(&self.grid_data, mx, my).unwrap_or((mx, my));
                            let path = pleb::astar_path_terrain_water_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                &self.water_depth_cpu,
                                start,
                                adj,
                            );
                            if !path.is_empty() {
                                pleb.path = path;
                                pleb.path_idx = 0;
                                pleb.activity = PlebActivity::Walking;
                                pleb.work_target = Some((mx, my));
                                pleb.harvest_target = Some((mx, my));
                                pleb.haul_target = None;
                            }
                        }
                    }
                }

                // Footprint stamping: drop prints at sub-tile intervals while walking
                let moved_dx = pleb.x - pleb.prev_x;
                let moved_dy = pleb.y - pleb.prev_y;
                let moved_dist = moved_dx * moved_dx + moved_dy * moved_dy;
                if moved_dist > 0.15 * 0.15 && !pleb.is_dead {
                    // Alternate left/right with slight offset from center
                    let side = (self.footprint_idx & 1) as f32 * 2.0 - 1.0; // -1 or +1
                    let perp_x = -moved_dy.signum() * 0.06 * side;
                    let perp_y = moved_dx.signum() * 0.06 * side;
                    let fp = (
                        pleb.x + perp_x,
                        pleb.y + perp_y,
                        pleb.angle,
                        0.0f32, // age starts at 0
                    );
                    let cap = self.footprints.len();
                    self.footprints[self.footprint_idx % cap] = fp;
                    self.footprint_idx += 1;
                }

                pleb.prev_x = pleb.x;
                pleb.prev_y = pleb.y;
            }
        }

        // Age footprints
        {
            let dt_game = dt * self.time_speed;
            for fp in &mut self.footprints {
                if fp.3 >= 0.0 {
                    fp.3 += dt_game;
                }
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
                            fresh: true,
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
                self.camera.pleb_headlight = pleb.headlight_mode as f32;
            }
        } else if let Some(pleb) = self.plebs.first() {
            // Show first pleb even if not selected (for lighting)
            self.camera.pleb_x = pleb.x;
            self.camera.pleb_y = pleb.y;
            self.camera.pleb_angle = pleb.angle;
            self.camera.pleb_selected = 0.0;
            self.camera.pleb_torch = if pleb.torch_on { 1.0 } else { 0.0 };
            self.camera.pleb_headlight = pleb.headlight_mode as f32;
        } else {
            self.camera.pleb_x = 0.0;
            self.camera.pleb_y = 0.0;
            self.camera.pleb_torch = 0.0;
            self.camera.pleb_headlight = 0.0;
        }

        // (Auto-close replaced by physical door tick above — doors swing shut via damping/latch)

        // --- Pleb communication: shouts + flocking ---
        {
            let shouts = comms::collect_shouts(&mut self.plebs, dt);
            // Emit sound sources for shouts
            for shout in &shouts {
                if self.sound_enabled {
                    self.sound_sources.push(SoundSource {
                        x: shout.x,
                        y: shout.y,
                        amplitude: types::db_to_amplitude(shout.kind.sound_db()),
                        frequency: shout.kind.sound_freq() * if shout.is_enemy { 1.7 } else { 1.0 }, // enemies higher pitch
                        phase: 0.0,
                        pattern: 1, // sine
                        duration: 0.15,
                        fresh: true,
                    });
                }
            }
            // Include command shouts from UI (Rally, Advance, FallBack)
            let cmd_shouts: Vec<comms::Shout> = self.pending_command_shouts.drain(..).collect();
            for shout in &cmd_shouts {
                if self.sound_enabled {
                    self.sound_sources.push(SoundSource {
                        x: shout.x,
                        y: shout.y,
                        amplitude: types::db_to_amplitude(shout.kind.sound_db()),
                        frequency: shout.kind.sound_freq(),
                        phase: 0.0,
                        pattern: 1,
                        duration: 0.2,
                        fresh: true,
                    });
                }
            }
            let mut all_shouts = shouts;
            all_shouts.extend(cmd_shouts);
            comms::process_shouts(
                &mut self.plebs,
                &all_shouts,
                &self.grid_data,
                &self.wall_data,
            );
        }

        // --- Combat: drafted friendlies + all enemies auto-target and fight ---
        // Melee strike actions deferred to avoid borrow issues: (attacker_idx, target_idx)
        struct MeleeStrike {
            attacker: usize,
            target: usize,
            damage: f32,
            knockback: f32,
            bleed: f32,
            weapon_type: u8,
            hit: bool, // false = missed
        }
        let mut melee_strikes: Vec<MeleeStrike> = Vec::new();
        // (x, y, dx, dy, name, spread, gun_z, target_z, target_dist, shooter_idx)
        let fire_actions: Vec<(f32, f32, f32, f32, String, f32, f32, f32, f32, usize)>;
        {
            // Collect positions by faction
            let enemies: Vec<(usize, f32, f32)> = self
                .plebs
                .iter()
                .enumerate()
                .filter(|(_, p)| p.is_enemy && !p.is_dead)
                .map(|(i, p)| (i, p.x, p.y))
                .collect();
            let friendlies: Vec<(usize, f32, f32)> = self
                .plebs
                .iter()
                .enumerate()
                .filter(|(_, p)| !p.is_enemy && !p.is_dead)
                .map(|(i, p)| (i, p.x, p.y))
                .collect();

            let mut pending_fires: Vec<(f32, f32, f32, f32, String, f32, f32, f32, f32, usize)> =
                Vec::new();

            // Bare fist fallback stats
            const FIST_DAMAGE: f32 = 0.03;
            const FIST_SPEED: f32 = 0.8;
            const FIST_RANGE: f32 = 0.8;
            const FIST_KNOCKBACK: f32 = 1.5;

            for pi in 0..self.plebs.len() {
                if self.plebs[pi].is_dead {
                    continue;
                }

                let is_enemy = self.plebs[pi].is_enemy;
                let is_drafted = self.plebs[pi].drafted;

                // Friendlies only fight when drafted; enemies always fight
                if !is_enemy && !is_drafted {
                    continue;
                }

                let combat_range = 25.0f32;
                let targets = if is_enemy { &friendlies } else { &enemies };

                let px = self.plebs[pi].x;
                let py = self.plebs[pi].y;
                let mut best: Option<(usize, f32, f32, f32)> = None;
                for &(ti, tx, ty) in targets {
                    let d = ((px - tx).powi(2) + (py - ty).powi(2)).sqrt();
                    if d < combat_range && best.map_or(true, |(_, bd, _, _)| d < bd) {
                        best = Some((ti, d, tx, ty));
                    }
                }

                // Get equipped weapon stats
                let reg = item_defs::ItemRegistry::cached();
                let wpn_def = self.plebs[pi].equipped_weapon.and_then(|id| reg.get(id));

                let mut use_melee = !wpn_def.map_or(false, |d| d.is_ranged_weapon());

                // Auto-switch: ranged pleb with enemy in melee range → swap to melee
                // Only if pleb has a melee weapon in inventory
                const CLOSE_QUARTERS_RANGE: f32 = 2.0;
                if !use_melee {
                    if let Some((_, close_dist, _, _)) = best {
                        if close_dist < CLOSE_QUARTERS_RANGE {
                            // Check if pleb has a melee weapon to switch to
                            let has_melee =
                                self.plebs[pi].inventory.stacks.iter().any(|s| {
                                    reg.get(s.item_id).map_or(false, |d| d.is_melee_weapon())
                                });
                            if has_melee && self.plebs[pi].weapon_swap_timer <= 0.0 {
                                // Initiate swap: delay based on melee skill (faster for experienced fighters)
                                let melee_skill = self.plebs[pi].skills[1].value;
                                let swap_time = (0.8 - melee_skill * 0.05).max(0.3); // 0.8s (skill 0) to 0.35s (skill 9)
                                self.plebs[pi].weapon_swap_timer = swap_time;
                                self.plebs[pi].prefer_ranged = false;
                                self.plebs[pi].update_equipped_weapon();
                                self.plebs[pi].aim_progress = 0.0;
                                self.plebs[pi].swing_progress = 0.0;
                                self.plebs[pi].set_bubble(
                                    pleb::BubbleKind::Text("Switching...".into()),
                                    swap_time,
                                );
                                use_melee = true;
                            }
                        }
                    }
                }

                // Melee stats (from equipped melee weapon or fists)
                let melee_wpn = if use_melee {
                    wpn_def.filter(|d| d.is_melee_weapon())
                } else {
                    None
                };
                let (melee_damage, melee_speed, melee_range, melee_kb, melee_bleed, wpn_type) =
                    if let Some(w) = melee_wpn {
                        (
                            w.melee_damage,
                            w.melee_speed,
                            w.melee_range,
                            w.melee_knockback,
                            w.melee_bleed,
                            w.weapon_type,
                        )
                    } else if use_melee {
                        (
                            FIST_DAMAGE,
                            FIST_SPEED,
                            FIST_RANGE,
                            FIST_KNOCKBACK,
                            0.0,
                            0u8,
                        )
                    } else {
                        (0.0, 0.0, 0.0, 0.0, 0.0, 0u8)
                    };

                // Manual aim_pos overrides automatic target selection (for ranged only)
                if let Some((ax, ay)) = self.plebs[pi].aim_pos {
                    if !use_melee {
                        let dx = ax - px;
                        let dy = ay - py;
                        let dist = (dx * dx + dy * dy).sqrt();
                        let target_angle = dy.atan2(dx);
                        self.plebs[pi].angle = lerp_angle(
                            self.plebs[pi].angle,
                            target_angle,
                            (TURN_SPEED_COMBAT * dt).min(1.0),
                        );

                        // Stop moving to fire
                        self.plebs[pi].path.clear();
                        self.plebs[pi].path_idx = 0;

                        let mag_size = wpn_def.map(|w| w.magazine_size).unwrap_or(6);
                        let reload_base = wpn_def.map(|w| w.reload_time).unwrap_or(2.5);

                        if self.plebs[pi].reload_timer > 0.0 {
                            self.plebs[pi].reload_timer -= dt * self.time_speed;
                            self.plebs[pi].aim_progress = 0.0;
                            if self.plebs[pi].reload_timer <= 0.0 {
                                self.plebs[pi].reload_timer = 0.0;
                                // Transfer rounds from inventory to magazine
                                let ammo_type = wpn_def.map(|w| w.ammo_type.as_str()).unwrap_or("");
                                let ammo_item = {
                                    let ireg = item_defs::ItemRegistry::cached();
                                    let mut found = None;
                                    for s in &self.plebs[pi].inventory.stacks {
                                        if let Some(d) = ireg.get(s.item_id) {
                                            if d.ammo_type == ammo_type
                                                && d.category == "ammo"
                                                && s.count > 0
                                            {
                                                found = Some(s.item_id);
                                                break;
                                            }
                                        }
                                    }
                                    found
                                };
                                if let Some(ammo_id) = ammo_item {
                                    let have = self.plebs[pi].inventory.count_of(ammo_id) as u8;
                                    let load = have.min(mag_size);
                                    self.plebs[pi].inventory.remove(ammo_id, load as u16);
                                    self.plebs[pi].ammo_loaded = load;
                                } else {
                                    self.plebs[pi].ammo_loaded = 0; // no ammo found
                                }
                            }
                        } else if self.plebs[pi].ammo_loaded == 0 && mag_size > 0 {
                            // Check if pleb has matching ammo before starting reload
                            let ammo_type = wpn_def.map(|w| w.ammo_type.as_str()).unwrap_or("");
                            let has_ammo = {
                                let ireg = item_defs::ItemRegistry::cached();
                                self.plebs[pi].inventory.stacks.iter().any(|s| {
                                    ireg.get(s.item_id).is_some_and(|d| {
                                        d.ammo_type == ammo_type
                                            && d.category == "ammo"
                                            && s.count > 0
                                    })
                                })
                            };
                            if has_ammo {
                                let rld_seed =
                                    (px * 97.3 + py * 211.7 + self.time_of_day * 5003.0) as u32;
                                let rld_rng =
                                    (rld_seed.wrapping_mul(2654435761) & 0xFFFF) as f32 / 65535.0;
                                let rld_time = reload_base * (0.75 + rld_rng * 0.5);
                                self.plebs[pi].reload_timer = rld_time;
                                self.plebs[pi].aim_progress = 0.0;
                                self.plebs[pi].set_bubble(
                                    pleb::BubbleKind::Text("Reloading...".into()),
                                    rld_time,
                                );
                            } else {
                                self.plebs[pi].set_bubble(
                                    pleb::BubbleKind::Thought("Out of ammo...".into()),
                                    2.0,
                                );
                                self.plebs[pi].aim_progress = 0.0;
                            }
                        } else {
                            let shooting_skill = self.plebs[pi].skills[0].value;
                            let pleb_hash = (self.plebs[pi].id as u32).wrapping_mul(2654435761);
                            let pleb_rng = (pleb_hash & 0xFFFF) as f32 / 65535.0;
                            let base_aim = wpn_def.map(|w| w.ranged_aim_speed).unwrap_or(2.0);
                            let skill_aim_mul = (1.0 - shooting_skill * 0.05).max(0.5);
                            let aim_variance = 0.8 + pleb_rng * 0.4;
                            let stress_aim =
                                morale::aim_speed_multiplier(self.plebs[pi].needs.stress);
                            let rank_aim = self.plebs[pi].rank().aim_modifier();
                            let aim_speed = base_aim * skill_aim_mul * aim_variance * rank_aim
                                / stress_aim.max(0.1);
                            let patience = (0.6 + shooting_skill * 0.035 + pleb_rng * 0.1).min(1.0);

                            self.plebs[pi].aim_progress =
                                (self.plebs[pi].aim_progress + dt / aim_speed).min(1.0);

                            if self.plebs[pi].aim_progress >= patience {
                                let aim_quality = self.plebs[pi].aim_progress;
                                let base_spread = wpn_def.map(|w| w.ranged_spread).unwrap_or(0.20);
                                let skill_factor = (shooting_skill / 9.0).clamp(0.0, 1.0);
                                let aim_tightening =
                                    1.8 - aim_quality * 1.4 * (0.3 + skill_factor * 0.7);
                                let suppress_mul = 1.0 + self.plebs[pi].suppression * 2.0;
                                let stress_spread =
                                    morale::spread_multiplier(self.plebs[pi].needs.stress);
                                let spread =
                                    (base_spread * aim_tightening * suppress_mul * stress_spread)
                                        .max(0.02);
                                let gun_z = 1.0;
                                let target_z = 0.5; // aiming at ground level
                                let target_dist = dist.max(0.5);
                                let ndx = dx / dist.max(0.01);
                                let ndy = dy / dist.max(0.01);
                                let name = self.plebs[pi].name.clone();
                                pending_fires.push((
                                    px,
                                    py,
                                    ndx,
                                    ndy,
                                    name,
                                    spread,
                                    gun_z,
                                    target_z,
                                    target_dist,
                                    pi,
                                ));
                                self.plebs[pi].ammo_loaded =
                                    self.plebs[pi].ammo_loaded.saturating_sub(1);
                                let shot_seed = (px * 137.3
                                    + py * 311.7
                                    + self.time_of_day * 7919.0
                                    + self.plebs[pi].ammo_loaded as f32 * 41.0)
                                    as u32;
                                let shot_rng =
                                    (shot_seed.wrapping_mul(2654435761) & 0xFFFF) as f32 / 65535.0;
                                let cooldown = 0.2 + shot_rng * 0.6;
                                self.plebs[pi].aim_progress = -cooldown;
                                // Clear aim_pos after firing (single shot at position)
                                self.plebs[pi].aim_pos = None;
                            }
                        }
                        continue; // skip normal target logic
                    } else {
                        // Can't melee a position — clear it
                        self.plebs[pi].aim_pos = None;
                    }
                }

                if let Some((target_idx, dist, tx, ty)) = best {
                    let dx = tx - px;
                    let dy = ty - py;
                    let target_angle = dy.atan2(dx);
                    self.plebs[pi].angle = lerp_angle(
                        self.plebs[pi].angle,
                        target_angle,
                        (TURN_SPEED_COMBAT * dt).min(1.0),
                    );

                    // "!" on first combat engagement only (aim_progress >= 0 = fresh, < 0 = was in combat)
                    if self.plebs[pi].aim_target.is_none() && self.plebs[pi].aim_progress >= 0.0 {
                        self.plebs[pi].set_bubble(pleb::BubbleKind::Icon('!', [220, 50, 40]), 1.5);
                    }

                    // If enemy is behind cover, don't charge through it — stay and fight
                    let at_cover = is_enemy
                        && is_behind_cover(&self.grid_data, &self.wall_data, px, py, tx, ty);
                    if at_cover && use_melee {
                        // Behind cover with melee — stand ground, don't pathfind into wall
                        self.plebs[pi].path.clear();
                        self.plebs[pi].path_idx = 0;
                        self.plebs[pi].aim_target = Some(target_idx);
                        // Only swing if target is actually in melee range
                        if dist <= melee_range {
                            self.plebs[pi].swing_progress =
                                (self.plebs[pi].swing_progress + dt / melee_speed).min(1.0);
                            if self.plebs[pi].swing_progress >= 1.0 {
                                let melee_skill = self.plebs[pi].skills[1].value;
                                let hit_chance = (0.40 + melee_skill * 0.06).clamp(0.05, 0.95);
                                let roll_seed =
                                    (px * 137.3 + py * 311.7 + self.time_of_day * 7919.0) as u32;
                                let roll =
                                    (roll_seed.wrapping_mul(2654435761) & 0xFFFF) as f32 / 65535.0;
                                melee_strikes.push(MeleeStrike {
                                    attacker: pi,
                                    target: target_idx,
                                    damage: melee_damage,
                                    knockback: melee_kb,
                                    bleed: melee_bleed,
                                    weapon_type: wpn_type,
                                    hit: roll < hit_chance,
                                });
                                self.plebs[pi].swing_progress = 0.0;
                            }
                        }
                    } else if use_melee {
                        // Clear ranged state
                        self.plebs[pi].aim_progress = 0.0;

                        // Can't attack while swapping weapons
                        if self.plebs[pi].weapon_swap_timer > 0.0 {
                            self.plebs[pi].swing_progress = 0.0;
                            self.plebs[pi].aim_target = Some(target_idx);
                        } else if dist > melee_range {
                            // Phase 1: Close distance — pathfind toward target
                            self.plebs[pi].swing_progress = 0.0;

                            // Only repath when current path is exhausted or target moved far
                            let needs_repath = self.plebs[pi].path_idx >= self.plebs[pi].path.len()
                                || self.plebs[pi].path.last().map_or(true, |&(gx, gy)| {
                                    let tdx = gx as f32 + 0.5 - tx;
                                    let tdy = gy as f32 + 0.5 - ty;
                                    tdx * tdx + tdy * tdy > 4.0 // target moved >2 tiles from path end
                                });
                            if needs_repath {
                                let start = (px.floor() as i32, py.floor() as i32);
                                // Enemies flank: approach from the side instead of head-on
                                let goal = if is_enemy && dist > 4.0 {
                                    let adx = tx - px;
                                    let ady = ty - py;
                                    let alen = (adx * adx + ady * ady).sqrt().max(0.1);
                                    // Perpendicular offset, alternating side by pleb ID
                                    let side = if self.plebs[pi].id % 2 == 0 {
                                        1.0
                                    } else {
                                        -1.0
                                    };
                                    let flank_x = (tx + (-ady / alen) * 3.0 * side)
                                        .clamp(1.0, GRID_W as f32 - 2.0)
                                        .floor()
                                        as i32;
                                    let flank_y = (ty + (adx / alen) * 3.0 * side)
                                        .clamp(1.0, GRID_H as f32 - 2.0)
                                        .floor()
                                        as i32;
                                    // Use flank if walkable, else direct
                                    if is_walkable_pos(
                                        &self.grid_data,
                                        flank_x as f32 + 0.5,
                                        flank_y as f32 + 0.5,
                                    ) {
                                        (flank_x, flank_y)
                                    } else {
                                        (tx.floor() as i32, ty.floor() as i32)
                                    }
                                } else {
                                    (tx.floor() as i32, ty.floor() as i32)
                                };
                                self.plebs[pi].path =
                                    pleb::astar_path(&self.grid_data, start, goal);
                                self.plebs[pi].path_idx = 0;
                            }
                        } else {
                            // Phase 2 & 3: In range — windup and strike
                            self.plebs[pi].path.clear();
                            self.plebs[pi].path_idx = 0;

                            if self.plebs[pi].aim_target == Some(target_idx) {
                                self.plebs[pi].swing_progress =
                                    (self.plebs[pi].swing_progress + dt / melee_speed).min(1.0);
                            } else {
                                self.plebs[pi].aim_target = Some(target_idx);
                                self.plebs[pi].swing_progress = 0.0;
                            }

                            // Strike!
                            if self.plebs[pi].swing_progress >= 1.0 {
                                // Hit probability: 40% base + melee skill * 6% (0-9 → 0-54%)
                                let melee_skill = self.plebs[pi].skills[1].value;
                                let cover_mod =
                                    if has_low_wall_cover(&self.grid_data, px, py, tx, ty) {
                                        -0.25
                                    } else {
                                        0.0
                                    };
                                let hit_chance =
                                    (0.40 + melee_skill * 0.06 + cover_mod).clamp(0.05, 0.95);
                                let roll_seed =
                                    (px * 137.3 + py * 311.7 + self.time_of_day * 7919.0) as u32;
                                let roll =
                                    (roll_seed.wrapping_mul(2654435761) & 0xFFFF) as f32 / 65535.0;
                                let did_hit = roll < hit_chance;

                                melee_strikes.push(MeleeStrike {
                                    attacker: pi,
                                    target: target_idx,
                                    damage: melee_damage,
                                    knockback: melee_kb,
                                    bleed: melee_bleed,
                                    weapon_type: wpn_type,
                                    hit: did_hit,
                                });
                                self.plebs[pi].swing_progress = 0.0;
                            }
                        }
                    } else {
                        // Ranged path: stop moving to aim and fire
                        self.plebs[pi].swing_progress = 0.0;

                        // Drafted friendlies: auto-sidestep to nearby cover
                        if !is_enemy
                            && is_drafted
                            && self.plebs[pi].path_idx >= self.plebs[pi].path.len()
                            && !is_behind_cover(&self.grid_data, &self.wall_data, px, py, tx, ty)
                        {
                            if let Some((cx, cy)) =
                                find_cover_position(&self.grid_data, px, py, tx, ty, 3)
                            {
                                let cd = ((cx as f32 + 0.5 - px).powi(2)
                                    + (cy as f32 + 0.5 - py).powi(2))
                                .sqrt();
                                if cd < 3.5 {
                                    let start = (px.floor() as i32, py.floor() as i32);
                                    self.plebs[pi].path =
                                        pleb::astar_path(&self.grid_data, start, (cx, cy));
                                    self.plebs[pi].path_idx = 0;
                                }
                            }
                        } else {
                            self.plebs[pi].path.clear();
                            self.plebs[pi].path_idx = 0;
                        }

                        // Auto-crouch: behind low wall facing enemy → crouch
                        let behind_low_wall = has_low_wall_cover(&self.grid_data, px, py, tx, ty);
                        if behind_low_wall && !self.plebs[pi].crouching {
                            self.plebs[pi].crouching = true;
                        }
                        // Suppression-forced crouch (>0.6)
                        if self.plebs[pi].suppression > 0.6 && !self.plebs[pi].crouching {
                            self.plebs[pi].crouching = true;
                        }

                        // Peek-fire cycle: crouched plebs must peek to aim
                        if self.plebs[pi].crouching {
                            self.plebs[pi].peek_timer -= dt * self.time_speed;
                        }

                        // --- Reload check ---
                        let mag_size = wpn_def.map(|w| w.magazine_size).unwrap_or(6);
                        let reload_base = wpn_def.map(|w| w.reload_time).unwrap_or(2.5);

                        if self.plebs[pi].reload_timer > 0.0 {
                            // Currently reloading — count down, can't fire
                            self.plebs[pi].reload_timer -= dt * self.time_speed;
                            self.plebs[pi].aim_progress = 0.0;
                            if self.plebs[pi].reload_timer <= 0.0 {
                                self.plebs[pi].reload_timer = 0.0;
                                self.plebs[pi].ammo_loaded = mag_size;
                            }
                        } else if self.plebs[pi].ammo_loaded == 0 && mag_size > 0 {
                            // Empty — start reload with ±25% variance
                            let rld_seed =
                                (px * 97.3 + py * 211.7 + self.time_of_day * 5003.0) as u32;
                            let rld_rng =
                                (rld_seed.wrapping_mul(2654435761) & 0xFFFF) as f32 / 65535.0;
                            let rld_time = reload_base * (0.75 + rld_rng * 0.5);
                            self.plebs[pi].reload_timer = rld_time;
                            self.plebs[pi].aim_progress = 0.0;
                            self.plebs[pi].set_bubble(
                                pleb::BubbleKind::Text("Reloading...".into()),
                                rld_time,
                            );
                        } else {
                            // --- Aiming & firing ---
                            let shooting_skill = self.plebs[pi].skills[0].value;

                            // Per-pleb stable random factor
                            let pleb_hash = (self.plebs[pi].id as u32).wrapping_mul(2654435761);
                            let pleb_rng = (pleb_hash & 0xFFFF) as f32 / 65535.0;

                            // Aim speed with per-pleb variance
                            let base_aim = wpn_def.map(|w| w.ranged_aim_speed).unwrap_or(2.0);
                            let enemy_penalty = if is_enemy { 1.4 } else { 1.0 };
                            let skill_aim_mul = (1.0 - shooting_skill * 0.05).max(0.5);
                            let aim_variance = 0.8 + pleb_rng * 0.4;
                            let stress_aim =
                                morale::aim_speed_multiplier(self.plebs[pi].needs.stress);
                            let rank_aim = self.plebs[pi].rank().aim_modifier();
                            let aim_speed =
                                base_aim * enemy_penalty * skill_aim_mul * aim_variance * rank_aim
                                    / stress_aim.max(0.1);

                            // Fire threshold: skilled shooters wait longer
                            let patience = (0.6 + shooting_skill * 0.035 + pleb_rng * 0.1).min(1.0);

                            // Peek-fire: crouched plebs must peek up before aim advances
                            let is_crouched = self.plebs[pi].crouching;
                            if is_crouched && self.plebs[pi].peek_timer <= 0.0 {
                                // Not peeking yet — start peek
                                self.plebs[pi].peek_timer = 0.6; // peek window duration
                            }
                            let can_aim = !is_crouched || self.plebs[pi].peek_timer > 0.0;

                            if self.plebs[pi].aim_target == Some(target_idx) {
                                if can_aim {
                                    self.plebs[pi].aim_progress =
                                        (self.plebs[pi].aim_progress + dt / aim_speed).min(1.0);
                                }
                            } else {
                                self.plebs[pi].aim_target = Some(target_idx);
                                self.plebs[pi].aim_progress = 0.0;
                            }

                            if self.plebs[pi].aim_progress >= patience {
                                // --- Fire! ---
                                let aim_quality = self.plebs[pi].aim_progress;
                                let base_spread = wpn_def.map(|w| w.ranged_spread).unwrap_or(0.20);
                                let skill_factor = (shooting_skill / 9.0).clamp(0.0, 1.0);
                                let aim_tightening =
                                    1.8 - aim_quality * 1.4 * (0.3 + skill_factor * 0.7);
                                // Suppression + stress penalty
                                let suppress_mul = 1.0 + self.plebs[pi].suppression * 2.0;
                                let stress_spread =
                                    morale::spread_multiplier(self.plebs[pi].needs.stress);
                                let spread =
                                    (base_spread * aim_tightening * suppress_mul * stress_spread)
                                        .max(0.02);

                                // Gun height: use pleb's Z-height (crouched+peeking=0.7, standing=1.0)
                                let gun_z = self.plebs[pi].z_height();

                                // Target height: use target pleb's Z-height
                                let target_z = self
                                    .plebs
                                    .get(target_idx)
                                    .map(|t| t.z_height())
                                    .unwrap_or(1.0);
                                let target_dist = dist;

                                let name = self.plebs[pi].name.clone();
                                pending_fires.push((
                                    px,
                                    py,
                                    dx,
                                    dy,
                                    name,
                                    spread,
                                    gun_z,
                                    target_z,
                                    target_dist,
                                    pi,
                                ));

                                // Consume ammo
                                self.plebs[pi].ammo_loaded =
                                    self.plebs[pi].ammo_loaded.saturating_sub(1);

                                // Random cooldown between shots (varies per shot)
                                let shot_seed = (px * 137.3
                                    + py * 311.7
                                    + self.time_of_day * 7919.0
                                    + self.plebs[pi].ammo_loaded as f32 * 41.0)
                                    as u32;
                                let shot_rng =
                                    (shot_seed.wrapping_mul(2654435761) & 0xFFFF) as f32 / 65535.0;
                                let cooldown = 0.2 + shot_rng * 0.6; // 0.2–0.8s
                                self.plebs[pi].aim_progress = -cooldown;

                                // Duck back after firing when crouched
                                if self.plebs[pi].crouching {
                                    self.plebs[pi].peek_timer = -0.8; // stay ducked 0.8s before next peek
                                }
                            }
                        }
                    }
                } else {
                    // No target found — use grace period before clearing combat state
                    // (prevents oscillation when target is at range boundary)
                    if self.plebs[pi].aim_target.is_some() {
                        // Decay aim progress as a grace timer — only clear after it fully drains
                        self.plebs[pi].aim_progress -= dt * 2.0;
                        if self.plebs[pi].aim_progress < -1.0 {
                            self.plebs[pi]
                                .set_bubble(pleb::BubbleKind::Icon('?', [80, 140, 220]), 1.0);
                            self.plebs[pi].aim_target = None;
                            self.plebs[pi].aim_progress = 0.0;
                            self.plebs[pi].swing_progress = 0.0;
                            // Stand up when combat ends (unless manually crouched)
                            self.plebs[pi].crouching = false;
                            self.plebs[pi].peek_timer = 0.0;
                        }
                    }
                }
            }
            fire_actions = pending_fires;
        }

        // --- Morale recovery: tick for all living plebs ---
        {
            for pi in 0..self.plebs.len() {
                if self.plebs[pi].is_dead {
                    continue;
                }
                let px = self.plebs[pi].x;
                let py = self.plebs[pi].y;
                // Count nearby allies (same faction, within 8 tiles)
                let is_enemy = self.plebs[pi].is_enemy;
                let mut nearby_allies = 0u32;
                let mut any_panicking = false;
                let mut near_leader = false;
                let mut near_hardened = false;
                for pj in 0..self.plebs.len() {
                    if pj == pi || self.plebs[pj].is_dead || self.plebs[pj].is_enemy != is_enemy {
                        continue;
                    }
                    let d = (self.plebs[pj].x - px).powi(2) + (self.plebs[pj].y - py).powi(2);
                    if d < 64.0 {
                        // 8² tiles
                        nearby_allies += 1;
                        if self.plebs[pj].needs.stress >= morale::BREAK_THRESHOLD {
                            any_panicking = true;
                        }
                        if self.plebs[pj].is_leader {
                            near_leader = true;
                        }
                        if self.plebs[pj].rank() == pleb::CombatRank::Hardened {
                            near_hardened = true;
                        }
                    }
                }
                // Check if behind cover (any low wall between pleb and nearest enemy)
                let in_cover = self.plebs[pi].crouching
                    && has_low_wall_cover_any_direction(&self.grid_data, px, py);

                morale::tick_recovery(
                    &mut self.plebs[pi],
                    dt * self.time_speed,
                    in_cover,
                    nearby_allies,
                    near_leader,
                    near_hardened,
                );

                // Panic contagion: nearby ally breaking spreads stress
                if any_panicking {
                    morale::apply_stress(
                        &mut self.plebs[pi],
                        morale::STRESS_ALLY_PANICKING * dt * self.time_speed,
                    );
                }

                // Stress-induced breaking: flee when stress > 85
                if morale::should_break(&self.plebs[pi])
                    && self.plebs[pi].aim_target.is_some()
                    && !self.plebs[pi].is_enemy
                {
                    self.plebs[pi].aim_target = None;
                    self.plebs[pi].aim_pos = None;
                    self.plebs[pi].aim_progress = 0.0;
                    self.plebs[pi].set_bubble(pleb::BubbleKind::Icon('!', [220, 50, 40]), 2.0);
                    // Flee away from nearest enemy
                    let my_enemy = self.plebs[pi].is_enemy;
                    let flee_dir = self
                        .plebs
                        .iter()
                        .filter(|p| !p.is_dead && p.is_enemy != my_enemy)
                        .map(|p| {
                            let fdx = px - p.x;
                            let fdy = py - p.y;
                            let fd = (fdx * fdx + fdy * fdy).sqrt().max(0.1);
                            (fdx / fd, fdy / fd, fd)
                        })
                        .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
                        .map(|(fx, fy, _)| (fx, fy))
                        .unwrap_or((0.0, 1.0));
                    let flee_x = (px + flee_dir.0 * 8.0).clamp(1.0, GRID_W as f32 - 2.0);
                    let flee_y = (py + flee_dir.1 * 8.0).clamp(1.0, GRID_H as f32 - 2.0);
                    let start = (px.floor() as i32, py.floor() as i32);
                    let goal = (flee_x.floor() as i32, flee_y.floor() as i32);
                    let path = pleb::astar_path(&self.grid_data, start, goal);
                    if !path.is_empty() {
                        self.plebs[pi].path = path;
                        self.plebs[pi].path_idx = 0;
                    }
                }
            }
        }

        // Apply deferred melee strikes (only hits deal damage)
        for strike in &melee_strikes {
            if !strike.hit {
                continue;
            }
            let (ax, ay) = (self.plebs[strike.attacker].x, self.plebs[strike.attacker].y);
            if let Some(target) = self.plebs.get_mut(strike.target) {
                if target.is_dead {
                    continue;
                }
                target.needs.health -= strike.damage;
                target.bleeding = (target.bleeding + strike.bleed).min(1.0);
                target.stagger_timer = 0.15;
                morale::apply_stress(target, morale::STRESS_WOUNDED);

                // Knockback in hit direction
                let dx = target.x - ax;
                let dy = target.y - ay;
                let d = (dx * dx + dy * dy).sqrt().max(0.01);
                target.knockback_vx += dx / d * strike.knockback;
                target.knockback_vy += dy / d * strike.knockback;
            }
        }
        // Blood spatter + sound + log for melee strikes
        for strike in &melee_strikes {
            let (ax, ay) = (self.plebs[strike.attacker].x, self.plebs[strike.attacker].y);
            let (tx, ty) = (self.plebs[strike.target].x, self.plebs[strike.target].y);
            let attacker_name = self.plebs[strike.attacker].name.clone();
            let target_name = self.plebs[strike.target].name.clone();

            if strike.hit {
                // Blood spray: 3-6 drops in hit direction
                let hit_angle = (ty - ay).atan2(tx - ax);
                let drop_count =
                    3 + ((ax * 137.3 + ty * 311.7 + self.time_of_day * 999.0) as u32 % 4);
                for i in 0..drop_count {
                    let seed = (i as f32 * 73.1 + ax * 41.3 + ty * 97.7) as u32;
                    let rng = (seed.wrapping_mul(2654435761) & 0xFFFF) as f32 / 65535.0;
                    let spread = (rng - 0.5) * 1.0;
                    let dist = 0.3 + rng * 0.7;
                    let bx = tx + (hit_angle + spread).cos() * dist;
                    let by = ty + (hit_angle + spread).sin() * dist;
                    self.blood_stains.push((bx, by, 5.0));
                }

                // Impact sound (hit)
                if self.sound_enabled {
                    let (pattern, freq) = if strike.weapon_type == item_defs::WEAPON_SHOVEL {
                        (3u32, 200.0f32)
                    } else {
                        (4u32, 2000.0f32)
                    };
                    self.sound_sources.push(SoundSource {
                        x: tx,
                        y: ty,
                        amplitude: types::db_to_amplitude(80.0),
                        frequency: freq,
                        phase: 0.0,
                        pattern,
                        duration: 0.08,
                        fresh: true,
                    });
                }

                let verb = if strike.weapon_type == 0 {
                    "punches"
                } else {
                    "strikes"
                };
                events.push(GameEventKind::Generic(
                    types::EventCategory::Combat,
                    format!("{} {} {}!", attacker_name, verb, target_name),
                ));
            } else {
                // Miss: lighter whoosh sound
                if self.sound_enabled {
                    self.sound_sources.push(SoundSource {
                        x: tx,
                        y: ty,
                        amplitude: types::db_to_amplitude(60.0),
                        frequency: 1500.0,
                        phase: 0.0,
                        pattern: 4, // slash whoosh
                        duration: 0.04,
                        fresh: true,
                    });
                }
                events.push(GameEventKind::Generic(
                    types::EventCategory::Combat,
                    format!("{} swings at {} — miss!", attacker_name, target_name),
                ));
                self.plebs[strike.attacker].set_bubble(pleb::BubbleKind::Text("Miss!".into()), 0.8);
            }
        }

        // Cap blood stains
        if self.blood_stains.len() > 200 {
            let excess = self.blood_stains.len() - 200;
            self.blood_stains.drain(0..excess);
        }

        // Apply deferred ranged fire actions (with accuracy spread + aimed arc)
        for (fx, fy, dx, dy, name, base_spread, gun_z, target_z, target_dist, shooter_idx) in
            fire_actions
        {
            let rng_seed = (fx * 137.3 + fy * 311.7 + self.time_of_day * 1000.0) as u32;
            let rng_val = (rng_seed.wrapping_mul(2654435761) & 0xFFFF) as f32 / 65535.0;
            // Horizontal spread
            let spread_angle = (rng_val - 0.5) * 2.0 * base_spread;
            let cos_s = spread_angle.cos();
            let sin_s = spread_angle.sin();
            let sdx = dx * cos_s - dy * sin_s;
            let sdy = dx * sin_s + dy * cos_s;

            let mut bullet =
                PhysicsBody::new_bullet_aimed(fx, fy, sdx, sdy, gun_z, target_z, target_dist);
            bullet.shooter_pleb = Some(shooter_idx);
            // Vertical spread: affects vz (poor shots aim too high or too low)
            // Wider vertical spread when target is behind cover (smaller exposed area)
            let vert_mul = if target_z < 0.9 { 25.0 } else { 15.0 };
            let rng_seed2 = rng_seed.wrapping_mul(1013904223).wrapping_add(0xBEEF);
            let rng_val2 = (rng_seed2 & 0xFFFF) as f32 / 65535.0;
            let vz_spread = (rng_val2 - 0.5) * base_spread * vert_mul;
            bullet.vz += vz_spread;

            self.physics_bodies.push(bullet);
            if self.sound_enabled {
                self.sound_sources.push(SoundSource {
                    x: fx,
                    y: fy,
                    amplitude: types::db_to_amplitude(105.0),
                    frequency: 0.0,
                    phase: 0.0,
                    pattern: 5, // gunshot
                    duration: 0.12,
                    fresh: true,
                });
            }
            events.push(GameEventKind::Generic(
                types::EventCategory::Combat,
                format!("{} fires!", name),
            ));
        }

        // --- Physics tick ---
        {
            let sel_pleb = self.selected_pleb.and_then(|i| self.plebs.get(i));
            let pleb_data = sel_pleb.map(|p| (p.x, p.y, 0.0f32, 0.0f32, p.angle));
            // Collect pleb positions for bullet collision (with Z-height for crouch)
            let pleb_positions: Vec<(f32, f32, usize, f32)> = self
                .plebs
                .iter()
                .enumerate()
                .map(|(i, p)| (p.x, p.y, i, p.z_height()))
                .collect();
            // Extract sound source data for physics body force coupling
            let sound_data: Vec<(f32, f32, f32)> = self
                .sound_sources
                .iter()
                .map(|s| (s.x, s.y, s.amplitude))
                .collect();
            // Collect creature positions for bullet collision
            let creature_positions: Vec<(f32, f32, usize, f32)> = self
                .creatures
                .iter()
                .enumerate()
                .filter(|(_, c)| !c.is_dead)
                .map(|(i, c)| {
                    let radius = crate::creature_defs::CreatureRegistry::cached()
                        .get(c.species_id)
                        .map(|d| d.body_radius)
                        .unwrap_or(0.2);
                    (c.x, c.y, i, radius)
                })
                .collect();
            let physics_dt = if self.debug_bullet_slowmo {
                dt * self.debug_bullet_speed
            } else {
                dt
            };
            let (impacts, bullet_hits, explosion_events) = tick_bodies(
                &mut self.physics_bodies,
                physics_dt,
                &self.grid_data,
                &self.wall_data,
                self.fluid_params.wind_x,
                self.fluid_params.wind_y,
                pleb_data,
                &pleb_positions,
                &creature_positions,
                self.selected_pleb,
                self.enable_ricochets,
                &sound_data,
            );

            // Apply bullet hits to entities
            let mut kill_credits: Vec<usize> = Vec::new(); // shooter indices
            let mut shooting_xp_credits: Vec<usize> = Vec::new(); // shooters who hit creatures
            for hit in &bullet_hits {
                match hit.target {
                    physics::HitTarget::Pleb(pi) => {
                        if let Some(pleb) = self.plebs.get_mut(pi) {
                            let dmg = projectile_def(PROJ_BULLET).hit_damage;
                            events.push(GameEventKind::PlebHit {
                                pleb: pleb.name.clone(),
                                hp_pct: (pleb.needs.health - dmg).max(0.0) * 100.0,
                            });
                            let lethal =
                                pleb.needs.health > 0.01 && pleb.needs.health - dmg <= 0.01;
                            pleb.needs.health -= dmg;
                            pleb.bleeding = (pleb.bleeding + 0.25).min(1.0);
                            pleb.stagger_timer = 0.25;
                            morale::apply_stress(pleb, morale::STRESS_WOUNDED);
                            if lethal {
                                if let Some(si) = hit.shooter {
                                    kill_credits.push(si);
                                }
                            }
                            self.fluid_params.splat_x = hit.x;
                            self.fluid_params.splat_y = hit.y;
                            self.fluid_params.splat_radius = 0.3;
                            self.fluid_params.splat_active = 1.0;
                            // Blood spray from bullet hit
                            for i in 0..3u32 {
                                let seed = (hit.x * 41.3 + hit.y * 97.7 + i as f32 * 73.1) as u32;
                                let rng = (seed.wrapping_mul(2654435761) & 0xFFFF) as f32 / 65535.0;
                                let bx = hit.x + (rng - 0.5) * 0.8;
                                let by = hit.y + (rng * 0.7 - 0.35) * 0.8;
                                self.blood_stains.push((bx, by, 5.0));
                            }
                        }
                        // Bullet-flesh impact sound
                        if self.sound_enabled {
                            self.sound_sources.push(SoundSource {
                                x: hit.x,
                                y: hit.y,
                                amplitude: types::db_to_amplitude(75.0),
                                frequency: 0.0,
                                phase: 0.0,
                                pattern: 3, // thud
                                duration: 0.06,
                                fresh: true,
                            });
                        }
                    }
                    physics::HitTarget::Creature(ci) => {
                        if let Some(creature) = self.creatures.get_mut(ci) {
                            let dmg = projectile_def(PROJ_BULLET).hit_damage;
                            let species_name = creature_defs::CreatureRegistry::cached()
                                .name(creature.species_id)
                                .to_string();

                            // Apply damage
                            creature.health -= dmg * 100.0; // scale: bullet damage 0.2 × 100 = 20 HP

                            // Determine hit description based on hit position relative to creature
                            let body_part = "body"; // simplified for now

                            // Bleeding: set a bleed timer
                            creature.bleeding = (creature.bleeding + 0.3).min(1.0);

                            // Decloak on hit (Hollowcall becomes visible for 10s)
                            if creature.species_id == creature_defs::CREATURE_HOLLOWCALL {
                                creature.uncloak_timer = 10.0;
                            }

                            // Shooter name
                            let shooter = self
                                .selected_pleb
                                .and_then(|i| self.plebs.get(i))
                                .map(|p| p.name.as_str())
                                .unwrap_or("Someone");

                            if creature.health <= 0.0 {
                                creature.is_dead = true;
                                creature.corpse_timer = 60.0;
                                events.push(GameEventKind::Generic(
                                    types::EventCategory::Combat,
                                    format!(
                                        "{} hit {} in the {}. {} is dead!",
                                        shooter, species_name, body_part, species_name
                                    ),
                                ));
                            } else {
                                // Non-aggressive creatures flee when hit
                                let cdef = creature_defs::CreatureRegistry::cached()
                                    .get(creature.species_id);
                                if cdef.is_some_and(|d| !d.aggressive) {
                                    let away_x = creature.x - hit.x;
                                    let away_y = creature.y - hit.y;
                                    let len = (away_x * away_x + away_y * away_y).sqrt().max(0.1);
                                    creature.state = crate::creatures::CreatureState::Scatter(
                                        away_x / len,
                                        away_y / len,
                                    );
                                    creature.state_timer = 0.0;
                                    creature.path.clear();
                                }
                                let bleed_desc = if creature.bleeding > 0.7 {
                                    "bleeding profusely"
                                } else if creature.bleeding > 0.3 {
                                    "bleeding"
                                } else {
                                    "scratched"
                                };
                                events.push(GameEventKind::Generic(
                                    types::EventCategory::Combat,
                                    format!(
                                        "{} hit {} in the {}. {} is {}.",
                                        shooter, species_name, body_part, species_name, bleed_desc
                                    ),
                                ));
                            }

                            // Blood splat in fluid sim
                            self.fluid_params.splat_x = hit.x;
                            self.fluid_params.splat_y = hit.y;
                            self.fluid_params.splat_radius = 0.2;
                            self.fluid_params.splat_active = 1.0;
                            // Track shooter for XP
                            if let Some(si) = hit.shooter {
                                shooting_xp_credits.push(si);
                            }
                        }
                    }
                }
            }

            // Award shooting XP to plebs who hit creatures
            for si in shooting_xp_credits {
                if let Some(shooter_pleb) = self.plebs.get_mut(si) {
                    if let Some(new_level) = shooter_pleb.gain_xp(pleb::SKILL_SHOOTING, 5.0) {
                        shooter_pleb.log_event(
                            self.time_of_day,
                            format!(
                                "{} improved to {:.1}",
                                pleb::SKILL_NAMES[pleb::SKILL_SHOOTING],
                                new_level
                            ),
                        );
                    }
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

                // Debug: track any bullet impact
                if impact.projectile_id == PROJ_BULLET {
                    events.push(GameEventKind::Generic(
                        types::EventCategory::Combat,
                        format!("IMPACT at ({:.0},{:.0})", impact.x, impact.y),
                    ));
                }
                // Impact sound (bullets use pattern 6, others use impulse)
                if def.impact.sound_db > 0.0 && self.sound_enabled {
                    let snd_pattern = if impact.projectile_id == PROJ_BULLET {
                        6u32
                    } else {
                        0u32
                    };
                    self.sound_sources.push(SoundSource {
                        x: impact.x,
                        y: impact.y,
                        amplitude: db_to_amplitude(def.impact.sound_db),
                        frequency: 0.0,
                        phase: 0.0,
                        pattern: snd_pattern,
                        duration: def.impact.sound_duration,
                        fresh: true,
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

                // Fuse gas emission — only for toxic grenades (gas[0] > 0.1)
                if let Some(fuse) = &def.fuse {
                    if fuse.gas[0] > 0.1 {
                        self.grenade_impacts.push((impact.x, impact.y));
                    }
                }
            }

            // Suppression: bullets impacting near plebs increase their suppression
            for impact in &impacts {
                if impact.projectile_id != PROJ_BULLET {
                    continue;
                }
                for pleb in &mut self.plebs {
                    if pleb.is_dead {
                        continue;
                    }
                    let d = ((pleb.x - impact.x).powi(2) + (pleb.y - impact.y).powi(2)).sqrt();
                    if d < 2.5 {
                        let amount = 0.15 * (1.0 - d / 2.5); // closer = more suppression
                        pleb.suppression = (pleb.suppression + amount).min(1.0);
                    }
                }
            }

            // Apply kill credits
            for si in kill_credits {
                if let Some(shooter) = self.plebs.get_mut(si) {
                    shooter.kills += 1;
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

                // Spawn fragments (shrapnel) radiating outward
                if expl.def.damage > 0.0 {
                    let frag_count = 8 + ((expl.x * 73.1 + expl.y * 137.3) as u32 % 5);
                    let frag_z = 0.8; // detonation height
                    for fi in 0..frag_count {
                        let seed = (fi as f32 * 97.3 + expl.x * 41.7 + expl.y * 311.1) as u32;
                        let h = |off: u32| -> f32 {
                            let v = seed
                                .wrapping_mul(2654435761)
                                .wrapping_add(off.wrapping_mul(1013904223));
                            (v >> 16) as f32 / 65535.0
                        };
                        let angle = h(1) * std::f32::consts::TAU; // 0–360 degrees
                        let elev = h(2) * 0.7 - 0.2; // -0.2 to 0.5 radians (mostly flat, some up)
                        let speed = 40.0 + h(3) * 40.0; // 40–80 tiles/s
                        self.physics_bodies.push(PhysicsBody::new_fragment(
                            expl.x, expl.y, frag_z, angle, elev, speed,
                        ));
                    }
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
                    // Explosion stress (within 6 tiles)
                    if dist < 6.0 {
                        let stress_falloff = 1.0 - dist / 6.0;
                        morale::apply_stress(
                            pleb,
                            morale::STRESS_EXPLOSION_NEARBY * stress_falloff,
                        );
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
                        pattern: 7, // explosion boom
                        duration: expl.def.sound_duration.max(0.3),
                        fresh: true,
                    });
                }

                // Fluid burst: smoke + heat injection
                self.fluid_params.splat_x = expl.x;
                self.fluid_params.splat_y = expl.y;
                self.fluid_params.splat_vx = 0.0;
                self.fluid_params.splat_vy = 0.0;
                self.fluid_params.splat_radius = radius.min(5.0);
                self.fluid_params.splat_active = 1.0;

                // GPU dust injection for explosion
                self.dust_injections.push(dust::DustInjection {
                    x: expl.x,
                    y: expl.y,
                    radius: radius * 0.8,
                    density: 2.0,
                });

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
                let temp_factor = if !(CROP_TEMP_MIN..=CROP_TEMP_MAX).contains(&approx_temp) {
                    0.0
                } else if (CROP_OPTIMAL_LOW..=CROP_OPTIMAL_HIGH).contains(&approx_temp) {
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

        // --- Update CPU water depth mirror for pathfinding ---
        // Combines: BT_WATER blocks, seep formula, fill tool injections, single-tile GPU readback
        if self.frame_count.is_multiple_of(15)
            && !self.water_table.is_empty()
            && !self.sub_elevation.is_empty()
        {
            let wt_offset = self.camera.water_table_offset;
            let grid_size = self
                .water_depth_cpu
                .len()
                .min(self.water_table.len())
                .min(self.grid_data.len());
            for idx in 0..grid_size {
                let bt = block_type_rs(self.grid_data[idx]);
                if bt == BT_WATER {
                    self.water_depth_cpu[idx] = 2.0;
                    continue;
                }
                let tx = (idx % GRID_W as usize) as f32 + 0.5;
                let ty = (idx / GRID_W as usize) as f32 + 0.5;
                let sub_elev = terrain::sample_elevation(&self.sub_elevation, tx, ty);
                let wt_depth = (self.water_table[idx] + wt_offset) - sub_elev;
                if wt_depth > 0.0 {
                    // Seep will fill this tile — mark as water
                    self.water_depth_cpu[idx] = self.water_depth_cpu[idx].max(wt_depth);
                } else {
                    // Decay to match GPU drain rate (~50% in 1.5s)
                    // At 15-frame intervals: ~4 updates/sec, need 0.87^(4*1.5)≈0.43
                    self.water_depth_cpu[idx] *= 0.87;
                    if self.water_depth_cpu[idx] < 0.01 {
                        self.water_depth_cpu[idx] = 0.0;
                    }
                }
            }
        }

        // --- Terrain compaction decay (natural path fading) ---
        // Decay a batch of random tiles each frame so unused paths slowly fade
        if !self.time_paused && self.frame_count.is_multiple_of(30) {
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
                    // Decay roughness: disturbed earth settles over time (1 in 16 chance per sample)
                    if terrain_roughness(self.terrain_data[idx]) > 0 && (hash >> 12) % 16 == 0 {
                        let cur = terrain_roughness(self.terrain_data[idx]);
                        self.terrain_data[idx] =
                            (self.terrain_data[idx] & !(0x3 << 13)) | ((cur - 1) << 13);
                    }
                    // Slowly heal dig holes (1 in 8 chance per sample when raining, 1 in 30 otherwise)
                    if terrain_dig_holes(self.terrain_data[idx]) > 0 {
                        let heal_chance = if self.camera.rain_intensity > 0.3 {
                            8
                        } else {
                            30
                        };
                        if (hash >> 8).is_multiple_of(heal_chance) {
                            terrain_remove_dig_hole(&mut self.terrain_data[idx]);
                        }
                    }
                }
                self.terrain_dirty = true;
            }
        }

        // --- Alien fauna: spawn, tick, sound, cleanup ---
        self.tick_creatures(dt, &mut events);

        // --- Work queue: assign idle friendly plebs to tasks by priority ---
        {
            let mut farm_tasks =
                generate_work_tasks(&self.zones, &self.grid_data, GRID_W, &self.active_work);
            // Earthwork tasks (dig/fill) — assigned under WORK_BUILD priority
            let mut earthwork_tasks: Vec<WorkTask> = Vec::new();
            earthwork_tasks.extend(zones::generate_dig_tasks(
                &self.dig_zones,
                &self.sub_elevation,
                &self.active_work,
            ));
            earthwork_tasks.extend(zones::generate_fill_tasks(
                &self.berm_zones,
                &self.sub_elevation,
                &self.active_work,
            ));
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
                if pleb.is_enemy || pleb.is_dead || pleb.drafted {
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
                                    if best.is_none_or(|(_, bd)| dist < bd) {
                                        best = Some((i, dist));
                                    }
                                }
                                if let Some((task_idx, _)) = best {
                                    let task = &farm_tasks[task_idx];
                                    let (tx, ty) = task.position();
                                    let task_name = match task {
                                        WorkTask::Plant(_, _) => "plant",
                                        WorkTask::Harvest(_, _) => "harvest",
                                        _ => "work",
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
                                    let path = pleb::astar_path_terrain_water_wd(
                                        &self.grid_data,
                                        &self.wall_data,
                                        &self.terrain_data,
                                        &self.water_depth_cpu,
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
                                    if best.is_none_or(|(_, bd)| dist < bd) {
                                        best = Some(((ix, iy), dist));
                                    }
                                }
                                if let Some(((ix, iy), _)) = best
                                    && let Some((cx, cy)) =
                                        find_nearest_crate(&self.grid_data, ix, iy)
                                {
                                    let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                                    let path = pleb::astar_path_terrain_water_wd(
                                        &self.grid_data,
                                        &self.wall_data,
                                        &self.terrain_data,
                                        &self.water_depth_cpu,
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
                                        events.push(GameEventKind::AutoHauling(pleb.name.clone()));
                                        assigned = true;
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
                                    if best.is_none_or(|(_, bd)| dist < bd) {
                                        best = Some(((sx, sy, gidx), dist));
                                    }
                                }
                                if let Some(((sx, sy, _gidx), _)) = best {
                                    let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                                    let adj = adjacent_walkable(&self.grid_data, sx, sy)
                                        .unwrap_or((sx, sy));
                                    let path = pleb::astar_path_terrain_water_wd(
                                        &self.grid_data,
                                        &self.wall_data,
                                        &self.terrain_data,
                                        &self.water_depth_cpu,
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
                            zones::WORK_BUILD => {
                                // Find nearest earthwork task (dig/fill)
                                let mut best: Option<(usize, f32)> = None;
                                for (i, task) in earthwork_tasks.iter().enumerate() {
                                    let (tx, ty) = task.position();
                                    let dist = (pleb.x - tx as f32 - 0.5).powi(2)
                                        + (pleb.y - ty as f32 - 0.5).powi(2);
                                    if best.is_none_or(|(_, bd)| dist < bd) {
                                        best = Some((i, dist));
                                    }
                                }
                                if let Some((task_idx, _)) = best {
                                    let task = &earthwork_tasks[task_idx];
                                    let (tx, ty) = task.position();
                                    let task_name = match task {
                                        WorkTask::Dig(_, _) => "dig",
                                        WorkTask::Fill(_, _) => "fill",
                                        _ => "work",
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
                                    let path = pleb::astar_path_terrain_water_wd(
                                        &self.grid_data,
                                        &self.wall_data,
                                        &self.terrain_data,
                                        &self.water_depth_cpu,
                                        start,
                                        (tx, ty),
                                    );
                                    if !path.is_empty() {
                                        pleb.path = path;
                                        pleb.path_idx = 0;
                                        pleb.activity = PlebActivity::Walking;
                                    }
                                    assigned = true;
                                    earthwork_tasks.remove(task_idx);
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
                    let has_axe = pleb.has_tool("axe");
                    // Trees require Stone Axe — cancel if pleb doesn't have one
                    let is_tree_target = pleb.work_target.is_some_and(|(tx, ty)| {
                        let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                        tidx < self.grid_data.len() && (self.grid_data[tidx] & 0xFF) == BT_TREE
                    });
                    if is_tree_target && !has_axe {
                        pleb.activity = PlebActivity::Idle;
                        pleb.work_target = None;
                        continue;
                    }

                    let base_speed = if is_tree_target {
                        0.50 // ~2s with axe (required)
                    } else {
                        0.4 // ~2.5s for crops/bushes
                    };
                    let new_progress = progress
                        + dt * self.time_speed
                            * ACTION_SPEED_MUL
                            * base_speed
                            * pleb.farming_speed();
                    if new_progress >= 1.0 {
                        // Complete the task
                        if let Some((tx, ty)) = pleb.work_target {
                            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                            if tidx < self.grid_data.len() {
                                let tblock = self.grid_data[tidx];
                                let tbt = tblock & 0xFF;
                                if tbt == BT_GROUND {
                                    let roof_h = tblock & 0xFF000000;
                                    let fflags = block_flags_rs(tblock) as u32;
                                    self.grid_data[tidx] =
                                        make_block(BT_CROP as u8, CROP_PLANTED as u8, fflags as u8)
                                            | roof_h;
                                    self.crop_timers.insert(tidx as u32, 0.0);
                                    self.grid_dirty = true;
                                    events.push(GameEventKind::Planted(pleb.name.clone()));
                                    pleb.gain_xp_logged(
                                        pleb::SKILL_FARMING,
                                        10.0,
                                        self.time_of_day,
                                    );
                                } else if tbt == BT_CROP {
                                    let roof_h = tblock & 0xFF000000;
                                    let fflags = block_flags_rs(tblock) as u32;
                                    self.grid_data[tidx] =
                                        make_block(BT_GROUND as u8, 0, fflags as u8) | roof_h;
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
                                    pleb.gain_xp_logged(
                                        pleb::SKILL_FARMING,
                                        10.0,
                                        self.time_of_day,
                                    );
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
                                    pleb.gain_xp_logged(
                                        pleb::SKILL_FARMING,
                                        10.0,
                                        self.time_of_day,
                                    );
                                } else if crate::block_defs::BlockRegistry::cached()
                                    .get(tbt)
                                    .is_some_and(|d| d.is_harvestable)
                                {
                                    // Alien flora harvest — set harvest_target and switch to Harvesting activity
                                    // (actual item drops handled in tick_pleb_activity Harvesting branch)
                                    pleb.harvest_target = Some((tx, ty));
                                    pleb.activity = PlebActivity::Harvesting(0.0);
                                    pleb.work_target = None;
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
                                                    make_block(BT_GROUND as u8, 0, nflags as u8)
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
                                    pleb.gain_xp_logged(
                                        pleb::SKILL_FARMING,
                                        10.0,
                                        self.time_of_day,
                                    );
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

                // Handle Digging activity: pleb digs terrain at work_target
                if pleb.activity == PlebActivity::Digging {
                    if let Some((tx, ty)) = pleb.work_target {
                        // Swing animation drives dig strokes (shovel = full speed, bare hands = 50%)
                        let dig_speed = if pleb.has_tool("shovel") {
                            terrain::DIG_SPEED_SHOVEL
                        } else {
                            terrain::DIG_SPEED_SHOVEL * 0.5
                        };
                        pleb.swing_progress += dt * self.time_speed * ACTION_SPEED_MUL * dig_speed;
                        if pleb.swing_progress >= 1.0 {
                            pleb.swing_progress = 0.0;
                            // Apply dig stroke at the work target tile center
                            let wx = tx as f32 + 0.5;
                            let wy = ty as f32 + 0.5;
                            let target_depth = self
                                .dig_zones
                                .first()
                                .map(|dz| dz.target_depth)
                                .unwrap_or(0.8);
                            let original_elev =
                                terrain::sample_elevation(&self.sub_elevation, wx, wy);
                            let target_elev = original_elev - target_depth;
                            let dirt = terrain::apply_dig_stroke(
                                &mut self.sub_elevation,
                                wx,
                                wy,
                                terrain::DIG_DEPTH_PER_STROKE,
                                |_, _| target_elev,
                            );
                            if dirt > 0.001 {
                                self.sub_elevation_dirty = true;
                                // Mark terrain as freshly disturbed (max roughness, zero compaction)
                                let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                                if tidx < self.terrain_data.len() {
                                    // Set roughness to max (3) — bits 13-14
                                    self.terrain_data[tidx] |= 0x3 << 13;
                                    // Zero compaction — bits 24-28
                                    self.terrain_data[tidx] &= !0x1F000000;
                                    self.terrain_dirty = true;
                                }
                                // Produce dirt resource
                                let dirt_items = (dirt * terrain::DIRT_PER_VOLUME) as u16;
                                if dirt_items > 0 {
                                    self.ground_items.push(resources::GroundItem::new(
                                        pleb.x,
                                        pleb.y,
                                        item_defs::ITEM_CLAY, // use clay as dirt for now
                                        dirt_items,
                                    ));
                                }
                            } else {
                                // No more dirt to dig — done with this tile
                                self.active_work.remove(&(tx, ty));
                                pleb.work_target = None;
                                pleb.activity = PlebActivity::Idle;
                            }
                        }
                    } else {
                        pleb.activity = PlebActivity::Idle;
                    }
                }

                // Handle Filling activity: pleb dumps dirt at berm zone
                if pleb.activity == PlebActivity::Filling {
                    if let Some((tx, ty)) = pleb.work_target {
                        pleb.swing_progress +=
                            dt * self.time_speed * ACTION_SPEED_MUL * terrain::DIG_SPEED_SHOVEL;
                        if pleb.swing_progress >= 1.0 {
                            pleb.swing_progress = 0.0;
                            let wx = tx as f32 + 0.5;
                            let wy = ty as f32 + 0.5;
                            let target_h = self
                                .berm_zones
                                .first()
                                .map(|bz| bz.target_height)
                                .unwrap_or(1.0);
                            let dirt_used = terrain::apply_fill_stroke(
                                &mut self.sub_elevation,
                                wx,
                                wy,
                                terrain::FILL_HEIGHT_PER_STROKE,
                                |_, _| target_h,
                            );
                            if dirt_used > 0.001 {
                                self.sub_elevation_dirty = true;
                                // Consume dirt from inventory
                                let consume =
                                    (dirt_used * terrain::DIRT_PER_VOLUME).max(1.0) as u16;
                                pleb.inventory.remove(item_defs::ITEM_CLAY, consume);
                                // Mark terrain as disturbed
                                let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                                if tidx < self.terrain_data.len() {
                                    self.terrain_data[tidx] |= 0x3 << 13; // max roughness
                                    self.terrain_data[tidx] &= !0x1F000000; // zero compaction
                                    self.terrain_dirty = true;
                                }
                                // Check if pleb ran out of dirt
                                if pleb.inventory.count_of(item_defs::ITEM_CLAY) == 0 {
                                    self.active_work.remove(&(tx, ty));
                                    pleb.work_target = None;
                                    pleb.activity = PlebActivity::Idle;
                                }
                            } else {
                                // Berm complete at this tile
                                self.active_work.remove(&(tx, ty));
                                pleb.work_target = None;
                                pleb.activity = PlebActivity::Idle;
                            }
                        }
                    } else {
                        pleb.activity = PlebActivity::Idle;
                    }
                }

                // Arrived at work target: start farming or crafting
                let has_work = pleb.work_target.is_some();
                let path_done = pleb.path_idx >= pleb.path.len();
                let is_walking_or_idle =
                    pleb.activity == PlebActivity::Walking || pleb.activity == PlebActivity::Idle;
                if has_work
                    && path_done
                    && is_walking_or_idle
                    && let Some((tx, ty)) = pleb.work_target
                {
                    let dist = ((pleb.x - tx as f32 - 0.5).powi(2)
                        + (pleb.y - ty as f32 - 0.5).powi(2))
                    .sqrt();
                    if dist < 1.5 {
                        // Butcher check first: creature corpse at target takes priority
                        let has_corpse = self.creatures.iter().any(|c| {
                            c.is_dead
                                && !c.dropped_loot
                                && c.x.floor() as i32 == tx
                                && c.y.floor() as i32 == ty
                        });
                        if has_corpse && pleb.has_tool("knife") {
                            pleb.draw_tool("knife");
                            pleb.activity = PlebActivity::Butchering(0.0);
                            pleb.path.clear();
                        } else if has_corpse {
                            // No knife — can't butcher
                            pleb.set_bubble(
                                pleb::BubbleKind::Thought("Need a knife...".into()),
                                2.5,
                            );
                            pleb.log_event(self.time_of_day, "Can't butcher — no knife".into());
                            pleb.activity = PlebActivity::Idle;
                            pleb.work_target = None;
                        } else {
                            // Fishing: arrived near water with line
                            let widx = (ty as u32 * GRID_W + tx as u32) as usize;
                            let has_water = widx < self.water_depth_cpu.len()
                                && self.water_depth_cpu[widx] > 0.3;
                            let has_line = pleb.has_tool("fishing")
                                || pleb.inventory.count_of(ITEM_FISHING_LINE) > 0;
                            if has_water && has_line {
                                pleb.draw_tool("fishing");
                                pleb.activity = PlebActivity::Fishing(0.0);
                                pleb.path.clear();
                            } else if has_water {
                                pleb.set_bubble(
                                    pleb::BubbleKind::Thought("Need a fishing line...".into()),
                                    2.5,
                                );
                                pleb.activity = PlebActivity::Idle;
                                pleb.work_target = None;
                            } else {
                                let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                                let tbt = if tidx < self.grid_data.len() {
                                    block_type_rs(self.grid_data[tidx])
                                } else {
                                    0
                                };

                                // Branch gathering: arrived at tree without axe on belt
                                if tbt == BT_TREE
                                    && pleb.harvest_target.is_some()
                                    && !pleb.has_tool("axe")
                                {
                                    pleb.activity = PlebActivity::Harvesting(0.0);
                                    pleb.work_target = None;
                                    pleb.path.clear();
                                    pleb.path_idx = 0;
                                // Rock: start sub-tile mining
                                } else if tbt == BT_ROCK && pleb.harvest_target.is_some() {
                                    // Create mining grid if needed
                                    let (mx, my) = pleb.harvest_target.unwrap();
                                    if !self.mining_grids.contains_key(&(mx, my)) {
                                        let rt = mining::rock_type_at(mx, my);
                                        self.mining_grids.insert(
                                            (mx, my),
                                            mining::generate_mining_grid(mx, my, rt),
                                        );
                                    }
                                    pleb.draw_tool("pick");
                                    pleb.activity = PlebActivity::Mining(0.0);
                                    pleb.path.clear();
                                    pleb.path_idx = 0;
                                // Alien flora: direct harvest on arrival
                                } else if crate::block_defs::BlockRegistry::cached()
                                    .get(tbt)
                                    .is_some_and(|d| d.is_harvestable)
                                    && pleb.harvest_target.is_some()
                                {
                                    pleb.activity = PlebActivity::Harvesting(0.0);
                                    pleb.work_target = None;
                                    pleb.path.clear();
                                    pleb.path_idx = 0;
                                } else if (tbt == BT_FIREPLACE || tbt == BT_CAMPFIRE)
                                    && (pleb.inventory.count_of(ITEM_RAW_MEAT) > 0
                                        || pleb.inventory.count_of(ITEM_RAW_FISH) > 0)
                                {
                                    // Arrived at campfire with raw food: start cooking
                                    pleb.activity = PlebActivity::Cooking(0.0);
                                    pleb.path.clear();
                                } else if tbt == BT_WORKBENCH
                                    || tbt == BT_KILN
                                    || tbt == BT_SAW_HORSE
                                {
                                    // Try to start crafting from queue
                                    let gidx = ty as u32 * GRID_W + tx as u32;
                                    let started = if let Some(queue) = self.craft_queues.get(&gidx)
                                    {
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
                                        let Some(recipe) = recipe_reg.get(recipe_id) else {
                                            self.active_work.remove(&(tx, ty));
                                            pleb.work_target = None;
                                            pleb.activity = PlebActivity::Idle;
                                            continue;
                                        };
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
                                                    if self.ground_items[i].stack.item_id
                                                        == ing.item
                                                    {
                                                        let take = self.ground_items[i]
                                                            .stack
                                                            .count
                                                            .min(need);
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
                                } else if self
                                    .dig_zones
                                    .iter()
                                    .any(|dz| dz.tiles.contains(&(tx, ty)))
                                {
                                    // Dig zone target: start digging (shovel auto-draw for speed bonus)
                                    pleb.draw_tool("shovel");
                                    pleb.activity = PlebActivity::Digging;
                                } else if self
                                    .berm_zones
                                    .iter()
                                    .any(|bz| bz.tiles.contains(&(tx, ty)))
                                {
                                    // Berm zone target: start filling (requires dirt in inventory)
                                    if pleb.inventory.count_of(item_defs::ITEM_CLAY) > 0 {
                                        pleb.activity = PlebActivity::Filling;
                                    } else {
                                        // No dirt — release task, go find some
                                        self.active_work.remove(&(tx, ty));
                                        pleb.work_target = None;
                                        pleb.activity = PlebActivity::Idle;
                                    }
                                } else if tbt == BT_GROUND
                                    && tidx < self.terrain_data.len()
                                    && terrain_dig_holes(self.terrain_data[tidx]) < 7
                                {
                                    // Dig earth: add a hole (no elevation change, tile stays BT_GROUND)
                                    let is_clay_terrain =
                                        terrain_type(self.terrain_data[tidx]) == TERRAIN_CLAY;
                                    terrain_add_dig_hole(&mut self.terrain_data[tidx]);
                                    self.terrain_dirty = true;
                                    let has_shovel =
                                        pleb.inventory.count_of(ITEM_WOODEN_SHOVEL) > 0;
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
                                    // Auto-draw the appropriate tool for this task
                                    if tbt == BT_TREE {
                                        pleb.draw_tool("axe");
                                    } else if crate::block_defs::BlockRegistry::cached()
                                        .get(tbt)
                                        .is_some_and(|d| d.is_harvestable)
                                    {
                                        pleb.draw_tool("knife"); // soft bonus, not required
                                    }
                                    pleb.activity = PlebActivity::Farming(0.0);
                                }
                            } // end else (not water)
                        } // end else (no corpse)
                    } else {
                        // Too far — release task and retry
                        self.active_work.remove(&(tx, ty));
                        pleb.work_target = None;
                        pleb.activity = PlebActivity::Idle;
                    }
                }
            }
        }

        // --- Construction: plebs build blueprints ---
        // 1. Handle Building activity progress
        let mut wall_placements: Vec<(i32, i32, u16, u16, u16, u16)> = Vec::new(); // edges, thick, mat, height
        let mut dig_marks: Vec<(i32, i32)> = Vec::new(); // tiles to mark as dug (mud wall auto-dig)
        for pleb in self.plebs.iter_mut() {
            if pleb.is_enemy {
                continue;
            }
            if let PlebActivity::Building(progress) = &pleb.activity {
                if let Some((tx, ty)) = pleb.work_target {
                    let new_progress = if let Some(bp) = self.blueprints.get(&(tx, ty)) {
                        progress
                            + dt * self.time_speed * ACTION_SPEED_MUL * pleb.construction_speed()
                                / bp.build_time
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
                                    // Wattle & daub: auto-dig nearby dirt tile for daub
                                    let bp_bt = bp.block_data & 0xFF;
                                    if bp_bt == BT_MUD_WALL {
                                        dig_marks.push((tx, ty));
                                    }
                                    wall_placements.push((
                                        tx,
                                        ty,
                                        bp.wall_edges,
                                        bp.wall_thickness,
                                        bp.wall_material,
                                        bp.wall_height,
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
                                    let placed_bt = placed & 0xFF;
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
                        pleb.gain_xp_logged(pleb::SKILL_CONSTRUCTION, 20.0, self.time_of_day);
                        // Chain to adjacent blueprint if one exists (build walls sequentially)
                        let mut chained = false;
                        for &(dx, dy) in &[(1i32, 0), (-1, 0), (0, 1), (0, -1)] {
                            let nx = tx + dx;
                            let ny = ty + dy;
                            if let Some(nbp) = self.blueprints.get(&(nx, ny))
                                && !self.active_work.contains(&(nx, ny))
                            {
                                let ready = if nbp.is_roof() {
                                    pleb.inventory.count_of(ITEM_FIBER) >= 1
                                } else {
                                    nbp.resources_met()
                                };
                                if ready {
                                    if nbp.is_roof() {
                                        pleb.inventory.remove(ITEM_FIBER, 1);
                                    }
                                    pleb.work_target = Some((nx, ny));
                                    pleb.activity = PlebActivity::Building(0.0);
                                    self.active_work.insert((nx, ny));
                                    chained = true;
                                    break;
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
        for (tx, ty, edges, thickness, material, height) in wall_placements {
            self.place_wall_edge_h(tx, ty, edges, thickness, material, height);
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
                        if (self.grid_data[nidx] & 0xFF) == BT_GROUND
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
                    let new_progress = progress
                        + dt * self.time_speed * ACTION_SPEED_MUL * pleb.crafting_speed()
                            / recipe.time;
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
                        pleb.gain_xp_logged(pleb::SKILL_CRAFTING, 15.0, self.time_of_day);
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

        // --- Butchering: tick progress, drop meat on completion ---
        for pleb in self.plebs.iter_mut() {
            if pleb.is_dead || pleb.is_enemy {
                continue;
            }
            if let PlebActivity::Butchering(progress) = pleb.activity {
                let new_progress = progress + dt * self.time_speed * ACTION_SPEED_MUL * 0.25;
                if new_progress >= 1.0 {
                    // Find the creature corpse at work_target and mark it butchered
                    if let Some((wx, wy)) = pleb.work_target {
                        for c in &mut self.creatures {
                            if c.is_dead
                                && !c.dropped_loot
                                && c.x.floor() as i32 == wx
                                && c.y.floor() as i32 == wy
                            {
                                c.dropped_loot = true;
                                c.corpse_timer = 0.0; // remove corpse
                                if let Some(def) =
                                    creature_defs::CreatureRegistry::cached().get(c.species_id)
                                {
                                    if def.drops_item > 0 {
                                        self.ground_items.push(resources::GroundItem::new(
                                            pleb.x,
                                            pleb.y,
                                            def.drops_item,
                                            1,
                                        ));
                                        events.push(GameEventKind::Generic(
                                            types::EventCategory::General,
                                            format!("{} butchered a {}", pleb.name, def.name),
                                        ));
                                    }
                                }
                                break;
                            }
                        }
                    }
                    pleb.gain_xp_logged(pleb::SKILL_CRAFTING, 15.0, self.time_of_day);
                    pleb.activity = PlebActivity::Idle;
                    pleb.work_target = None;
                } else {
                    pleb.activity = PlebActivity::Butchering(new_progress);
                }
            }
        }

        // --- Cooking: tick progress, consume raw → produce cooked ---
        for pleb in self.plebs.iter_mut() {
            if pleb.is_dead || pleb.is_enemy {
                continue;
            }
            if let PlebActivity::Cooking(progress) = pleb.activity {
                let new_progress = progress + dt * self.time_speed * ACTION_SPEED_MUL * 0.2;
                if new_progress >= 1.0 {
                    // Convert raw food → cooked food (meat first, then fish)
                    if pleb.inventory.count_of(ITEM_RAW_MEAT) > 0 {
                        pleb.inventory.remove(ITEM_RAW_MEAT, 1);
                        pleb.inventory.add(ITEM_COOKED_MEAT, 1);
                        pleb.log_event(self.time_of_day, "Cooked meat".into());
                        events.push(GameEventKind::Generic(
                            types::EventCategory::General,
                            format!("{} cooked meat", pleb.name),
                        ));
                    } else if pleb.inventory.count_of(ITEM_RAW_FISH) > 0 {
                        pleb.inventory.remove(ITEM_RAW_FISH, 1);
                        pleb.inventory.add(ITEM_COOKED_FISH, 1);
                        pleb.log_event(self.time_of_day, "Cooked fish".into());
                        events.push(GameEventKind::Generic(
                            types::EventCategory::General,
                            format!("{} cooked fish", pleb.name),
                        ));
                    }
                    pleb.gain_xp_logged(pleb::SKILL_CRAFTING, 10.0, self.time_of_day);
                    pleb.activity = PlebActivity::Idle;
                    pleb.work_target = None;
                } else {
                    pleb.activity = PlebActivity::Cooking(new_progress);
                }
            }
        }

        // --- Fishing: tick progress, roll for catch on completion ---
        for pleb in self.plebs.iter_mut() {
            if pleb.is_dead || pleb.is_enemy {
                continue;
            }
            if let PlebActivity::Fishing(progress) = pleb.activity {
                let new_progress = progress + dt * self.time_speed * ACTION_SPEED_MUL * 0.05;
                if new_progress >= 1.0 {
                    // Hash-based catch roll: 35% chance
                    let seed = (pleb.x.to_bits())
                        .wrapping_mul(2654435761)
                        .wrapping_add(pleb.y.to_bits())
                        .wrapping_add((self.time_of_day * 1000.0) as u32);
                    let roll = types::hash_f32(seed);
                    if roll < 0.35 {
                        pleb.inventory.add(ITEM_RAW_FISH, 1);
                        pleb.log_event(self.time_of_day, "Caught a fish!".into());
                        events.push(GameEventKind::Generic(
                            types::EventCategory::General,
                            format!("{} caught a fish", pleb.name),
                        ));
                        pleb.gain_xp_logged(pleb::SKILL_FARMING, 10.0, self.time_of_day);
                        pleb.activity = PlebActivity::Idle;
                        pleb.work_target = None;
                    } else {
                        pleb.log_event(self.time_of_day, "No catch...".into());
                        pleb.gain_xp_failure_logged(pleb::SKILL_FARMING, 8.0, self.time_of_day);
                        // Restart fishing (loop)
                        pleb.activity = PlebActivity::Fishing(0.0);
                    }
                } else {
                    pleb.activity = PlebActivity::Fishing(new_progress);
                }
            }
        }

        // --- Mining: chip sub-cells, drop materials ---
        for pleb in self.plebs.iter_mut() {
            if pleb.is_dead || pleb.is_enemy {
                continue;
            }
            if let PlebActivity::Mining(progress) = pleb.activity {
                let has_pick = pleb.has_tool("pick");
                let mine_speed = if has_pick { 0.33 } else { 0.12 }; // pick: ~3s, hands: ~8s per cell
                let new_progress = progress + dt * self.time_speed * ACTION_SPEED_MUL * mine_speed;
                if new_progress >= 1.0 {
                    // Mine one sub-cell
                    if let Some((mx, my)) = pleb.harvest_target {
                        // Determine facing direction from pleb position
                        let dx = mx as f32 + 0.5 - pleb.x;
                        let dy = my as f32 + 0.5 - pleb.y;
                        let face = if dx.abs() > dy.abs() {
                            if dx > 0.0 { 3u8 } else { 1u8 } // west or east face
                        } else if dy > 0.0 {
                            0u8 // north face (pleb south of rock)
                        } else {
                            2u8 // south face
                        };

                        let mut mined_mat = None;
                        if let Some(grid) = self.mining_grids.get_mut(&(mx, my)) {
                            if let Some((sx, sy)) = mining::next_mine_target(grid, face) {
                                mined_mat = mining::mine_cell(grid, sx, sy);
                            }
                        }

                        if let Some(mat) = mined_mat {
                            // Drop appropriate item
                            let drop_item = match mat {
                                mining::MAT_HOST => Some((ITEM_ROCK, 1u16)),
                                mining::MAT_IRON => Some((item_defs::ITEM_ROCK, 1)), // TODO: iron ore item
                                mining::MAT_COPPER => Some((item_defs::ITEM_ROCK, 1)), // TODO: copper ore
                                mining::MAT_FLINT => Some((item_defs::ITEM_ROCK, 1)), // TODO: flint item
                                mining::MAT_COAL => Some((item_defs::ITEM_ROCK, 1)), // TODO: coal item
                                mining::MAT_CRYSTAL => Some((item_defs::ITEM_ROCK, 1)), // TODO: crystal
                                _ => None, // void: nothing drops
                            };
                            if let Some((item_id, count)) = drop_item {
                                self.ground_items.push(resources::GroundItem::new(
                                    pleb.x, pleb.y, item_id, count,
                                ));
                            }
                            if mat != mining::MAT_HOST && mat != mining::MAT_VOID {
                                let name = mining::material_name(mat);
                                pleb.log_event(self.time_of_day, format!("Found {}!", name));
                            }
                            self.grid_dirty = true;
                        }

                        // Check if fully mined → remove rock block
                        let fully_mined = self
                            .mining_grids
                            .get(&(mx, my))
                            .is_some_and(|g| mining::is_fully_mined(g));
                        if fully_mined {
                            let idx = (my as u32 * GRID_W + mx as u32) as usize;
                            if idx < self.grid_data.len() {
                                let roof = self.grid_data[idx] & 0xFF000000;
                                self.grid_data[idx] = make_block(BT_GROUND as u8, 0, 0) | roof;
                            }
                            self.mining_grids.remove(&(mx, my));
                            pleb.harvest_target = None;
                            pleb.work_target = None;
                            pleb.activity = PlebActivity::Idle;
                            self.grid_dirty = true;
                        } else {
                            // Continue mining next cell
                            pleb.activity = PlebActivity::Mining(0.0);
                        }
                    } else {
                        pleb.activity = PlebActivity::Idle;
                    }
                } else {
                    pleb.activity = PlebActivity::Mining(new_progress);
                }
            }
        }

        // --- Auto-tend fires: idle pleb near a dying fire adds fuel ---
        if self.frame_count % 60 == 11 {
            for pleb in self.plebs.iter_mut() {
                if pleb.is_dead || pleb.is_enemy {
                    continue;
                }
                if !matches!(pleb.activity, PlebActivity::Idle) {
                    continue;
                }
                // Check for nearby low-fuel fires (within 3 tiles)
                let bx = pleb.x.floor() as i32;
                let by = pleb.y.floor() as i32;
                let mut best_fire: Option<(i32, i32, u32)> = None;
                for dy in -3..=3i32 {
                    for dx in -3..=3i32 {
                        let fx = bx + dx;
                        let fy = by + dy;
                        if fx < 0 || fy < 0 || fx >= GRID_W as i32 || fy >= GRID_H as i32 {
                            continue;
                        }
                        let fidx = (fy as u32 * GRID_W + fx as u32) as usize;
                        let bt = self.grid_data[fidx] & 0xFF;
                        if bt != BT_CAMPFIRE && bt != BT_FIREPLACE {
                            continue;
                        }
                        let fuel = (self.grid_data[fidx] >> 8) & 0xFF;
                        if fuel <= 2 {
                            // Low fuel — needs tending
                            if best_fire.is_none() || fuel < best_fire.unwrap().2 {
                                best_fire = Some((fx, fy, fuel));
                            }
                        }
                    }
                }
                if let Some((fx, fy, _fuel)) = best_fire {
                    // Check if pleb has fuel items (sticks, logs, or charcoal)
                    let has_sticks = pleb.inventory.count_of(ITEM_SCRAP_WOOD) >= 3;
                    let has_log = pleb.inventory.count_of(item_defs::ITEM_LOG) > 0;
                    let has_charcoal = pleb.inventory.count_of(item_defs::ITEM_CHARCOAL) > 0;
                    if has_sticks || has_log || has_charcoal {
                        let fidx = (fy as u32 * GRID_W + fx as u32) as usize;
                        let fuel = (self.grid_data[fidx] >> 8) & 0xFF;
                        // Add fuel: charcoal adds 3 levels, log adds 2, sticks add 1
                        let (add, item_id, consume) = if has_charcoal {
                            (3u32, item_defs::ITEM_CHARCOAL, 1u16)
                        } else if has_log {
                            (2, item_defs::ITEM_LOG, 1)
                        } else {
                            (1, ITEM_SCRAP_WOOD, 3)
                        };
                        let new_fuel = (fuel + add).min(5);
                        self.grid_data[fidx] =
                            (self.grid_data[fidx] & 0xFFFF00FF) | (new_fuel << 8);
                        pleb.inventory.remove(item_id, consume);
                        pleb.log_event(self.time_of_day, "Tended the fire".into());
                        self.grid_dirty = true;
                    }
                }
            }
        }

        // --- Auto-assign cooking: idle pleb with raw food near a lit campfire ---
        // Throttled: only scan for campfires every ~0.5s (not every frame)
        if self.frame_count % 30 == 5 {
            let campfire_positions: Vec<(i32, i32)> = (0..(GRID_W * GRID_H) as usize)
                .filter(|&idx| {
                    let bt = self.grid_data[idx] & 0xFF;
                    let h = (self.grid_data[idx] >> 8) & 0xFF;
                    (bt == BT_FIREPLACE || bt == BT_CAMPFIRE) && h > 0
                })
                .map(|idx| ((idx as u32 % GRID_W) as i32, (idx as u32 / GRID_W) as i32))
                .collect();

            if !campfire_positions.is_empty() {
                for pleb in self.plebs.iter_mut() {
                    if pleb.is_dead
                        || pleb.is_enemy
                        || pleb.drafted
                        || !matches!(pleb.activity, PlebActivity::Idle)
                        || (pleb.inventory.count_of(ITEM_RAW_MEAT) == 0
                            && pleb.inventory.count_of(ITEM_RAW_FISH) == 0)
                    {
                        continue;
                    }
                    // Find nearest lit campfire
                    let mut best: Option<((i32, i32), f32)> = None;
                    for &(cx, cy) in &campfire_positions {
                        let d = ((pleb.x - cx as f32 - 0.5).powi(2)
                            + (pleb.y - cy as f32 - 0.5).powi(2))
                        .sqrt();
                        if d < 30.0 && best.map_or(true, |(_, bd)| d < bd) {
                            best = Some(((cx, cy), d));
                        }
                    }
                    if let Some(((cx, cy), dist)) = best {
                        if dist < 1.8 {
                            // Already near campfire: start cooking
                            pleb.activity = PlebActivity::Cooking(0.0);
                            pleb.work_target = Some((cx, cy));
                        } else {
                            // Path to campfire
                            let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                            let adj =
                                adjacent_walkable(&self.grid_data, cx, cy).unwrap_or((cx, cy));
                            let path = pleb::astar_path_terrain_water_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                &self.water_depth_cpu,
                                start,
                                adj,
                            );
                            if !path.is_empty() {
                                pleb.path = path;
                                pleb.path_idx = 0;
                                pleb.activity = PlebActivity::Walking;
                                pleb.work_target = Some((cx, cy));
                            }
                        }
                    }
                }
            }
        } // end campfire throttle

        // 2. Auto-assign idle plebs to blueprint tasks (haul resources or build)
        if !self.blueprints.is_empty() {
            let bp_positions: Vec<(i32, i32)> = self.blueprints.keys().copied().collect();
            for &(bx, by) in &bp_positions {
                if self.active_work.contains(&(bx, by)) {
                    continue;
                }
                let bp = &self.blueprints[&(bx, by)];

                // Check if blueprint is ready to build
                let ready = if bp.is_roof() {
                    self.plebs
                        .iter()
                        .any(|p| !p.is_enemy && p.inventory.count_of(ITEM_FIBER) >= 1)
                } else {
                    bp.resources_met()
                };
                if ready {
                    let needs_fiber = bp.is_roof();
                    let mut best: Option<(usize, f32)> = None;
                    for (i, pleb) in self.plebs.iter().enumerate() {
                        if pleb.is_enemy || pleb.drafted || pleb.work_target.is_some() {
                            continue;
                        }
                        if !matches!(pleb.activity, PlebActivity::Idle) {
                            continue;
                        }
                        if needs_fiber && pleb.inventory.count_of(ITEM_FIBER) == 0 {
                            continue;
                        }
                        let dist = ((pleb.x - bx as f32 - 0.5).powi(2)
                            + (pleb.y - by as f32 - 0.5).powi(2))
                        .sqrt();
                        if dist < 40.0 && (best.is_none_or(|(_, bd)| dist < bd)) {
                            best = Some((i, dist));
                        }
                    }
                    if let Some((pi, _)) = best {
                        let pleb = &mut self.plebs[pi];
                        let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                        let adj = adjacent_walkable(&self.grid_data, bx, by).unwrap_or((bx, by));
                        let path = pleb::astar_path_terrain_water_wd(
                            &self.grid_data,
                            &self.wall_data,
                            &self.terrain_data,
                            &self.water_depth_cpu,
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
                    } else if bp.wood_delivered < bp.wood_needed {
                        if bp.uses_sticks() {
                            Some(ITEM_SCRAP_WOOD)
                        } else {
                            Some(ITEM_LOG)
                        }
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
                                if best_item.is_none_or(|(_, bd)| d < bd) {
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
                                    if best_item.is_none_or(|(_, bd)| d < bd) {
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
                            // Find nearest idle (undrafted) pleb
                            let mut best_pleb: Option<(usize, f32)> = None;
                            for (i, pleb) in self.plebs.iter().enumerate() {
                                if pleb.is_enemy
                                    || pleb.is_dead
                                    || pleb.drafted
                                    || pleb.work_target.is_some()
                                {
                                    continue;
                                }
                                if !matches!(pleb.activity, PlebActivity::Idle) {
                                    continue;
                                }
                                let dist = ((pleb.x - pickup_pos.0 as f32 - 0.5).powi(2)
                                    + (pleb.y - pickup_pos.1 as f32 - 0.5).powi(2))
                                .sqrt();
                                if best_pleb.is_none_or(|(_, bd)| dist < bd) {
                                    best_pleb = Some((i, dist));
                                }
                            }
                            if let Some((pi, _)) = best_pleb {
                                let pleb = &mut self.plebs[pi];
                                let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                                let path = pleb::astar_path_terrain_water_wd(
                                    &self.grid_data,
                                    &self.wall_data,
                                    &self.terrain_data,
                                    &self.water_depth_cpu,
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
                                    if (self.grid_data[tidx2] & 0xFF) == BT_TREE {
                                        let d = (dx * dx + dy * dy) as f32;
                                        if best_tree.is_none_or(|(_, _, bd)| d < bd) {
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
                                    let path = pleb::astar_path_terrain_water_wd(
                                        &self.grid_data,
                                        &self.wall_data,
                                        &self.terrain_data,
                                        &self.water_depth_cpu,
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
            if pleb.activity == PlebActivity::Walking
                && let Some((tx, ty)) = pleb.work_target
                && let Some(bp) = self.blueprints.get(&(tx, ty))
            {
                let ready = if bp.is_roof() {
                    pleb.inventory.count_of(ITEM_FIBER) >= 1
                } else if bp.uses_sticks() {
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
                        } else if bp.uses_sticks() {
                            pleb.inventory.remove(ITEM_SCRAP_WOOD, 3);
                        }
                        pleb.activity = PlebActivity::Building(0.0);
                        pleb.path.clear();
                        pleb.path_idx = 0;
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
            if is_walking_or_idle
                && has_eat_target
                && let Some((tx, ty)) = pleb.harvest_target
            {
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

        // --- Auto-haul ground items to storage zones (throttled) ---
        if !self.ground_items.is_empty() && self.frame_count % 30 == 10 {
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
                    // Find nearest idle undrafted pleb
                    let mut best_pleb: Option<(usize, f32)> = None;
                    for (pi, pleb) in self.plebs.iter().enumerate() {
                        if pleb.is_enemy
                            || pleb.drafted
                            || pleb.work_target.is_some()
                            || pleb.haul_target.is_some()
                        {
                            continue;
                        }
                        if !matches!(pleb.activity, PlebActivity::Idle) {
                            continue;
                        }
                        let dist = ((pleb.x - ix as f32 - 0.5).powi(2)
                            + (pleb.y - iy as f32 - 0.5).powi(2))
                        .sqrt();
                        if dist < 40.0 && (best_pleb.is_none_or(|(_, bd)| dist < bd)) {
                            best_pleb = Some((pi, dist));
                        }
                    }
                    if let Some((pi, _)) = best_pleb {
                        let pleb = &mut self.plebs[pi];
                        let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                        let path = pleb::astar_path_terrain_water_wd(
                            &self.grid_data,
                            &self.wall_data,
                            &self.terrain_data,
                            &self.water_depth_cpu,
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
                            let path = pleb::astar_path_terrain_water_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                &self.water_depth_cpu,
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

        // --- Pleb bleeding: blood loss + blood drops (mirrors creature bleeding) ---
        for pleb in &mut self.plebs {
            if pleb.is_dead || pleb.bleeding <= 0.0 {
                continue;
            }
            pleb.needs.health -= pleb.bleeding * 0.02 * dt * pleb.bleed_resist();
            // Finish off plebs with negligible health (prevents lingering near-death)
            if pleb.needs.health < 0.02 && pleb.needs.health > 0.0 {
                pleb.needs.health = 0.0;
            }
            pleb.bleeding = (pleb.bleeding - 0.05 * dt).max(0.0);
            pleb.blood_drop_timer -= dt;
            if pleb.blood_drop_timer <= 0.0 {
                pleb.blood_drop_timer = 0.5 / pleb.bleeding.max(0.1);
                self.blood_stains.push((pleb.x, pleb.y, 5.0));
            }
        }

        // --- Critical health / heavy bleeding bubbles ---
        for pleb in &mut self.plebs {
            if pleb.is_dead {
                continue;
            }
            if pleb.needs.health < 0.2 && pleb.needs.health > 0.0 {
                pleb.set_bubble(pleb::BubbleKind::Icon('!', [200, 40, 40]), 2.0);
            }
        }

        // --- Mark dead plebs as corpses ---
        let mut death_positions: Vec<(f32, f32, bool, bool)> = Vec::new(); // (x, y, is_enemy, is_leader)
        for pleb in &mut self.plebs {
            if pleb.needs.health <= 0.01 && !pleb.is_dead {
                death_positions.push((pleb.x, pleb.y, pleb.is_enemy, pleb.is_leader));
                pleb.is_dead = true;
                pleb.needs.health = 0.0;
                pleb.activity = PlebActivity::Idle;
                pleb.path.clear();
                pleb.work_target = None;
                pleb.haul_target = None;
                pleb.harvest_target = None;
                pleb.aim_target = None;
                pleb.aim_progress = 0.0;
                pleb.swing_progress = 0.0;
                events.push(GameEventKind::PlebDied(pleb.name.clone()));
            }
        }
        // Apply morale effects from deaths to nearby plebs
        for &(dx, dy, died_enemy, died_leader) in &death_positions {
            for pleb in &mut self.plebs {
                if pleb.is_dead {
                    continue;
                }
                let d = (pleb.x - dx).powi(2) + (pleb.y - dy).powi(2);
                if d > 144.0 {
                    continue;
                } // 12 tiles
                if pleb.is_enemy == died_enemy {
                    // Same faction died: stress (extra if leader)
                    let stress = if died_leader {
                        morale::LEADER_DEATH_STRESS
                    } else {
                        morale::STRESS_ALLY_DIED
                    };
                    morale::apply_stress(pleb, stress);
                } else {
                    // Enemy died: relief + kill credit
                    morale::apply_relief(pleb, morale::RELIEF_ENEMY_KILLED);
                }
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

        // --- Room detection (throttled: every 60 frames or on grid change) ---
        if self.frame_count % 60 == 15 || self.grid_dirty {
            let (rooms, map) = rooms::detect_rooms(&self.grid_data, &self.wall_data, &self.doors);
            self.detected_rooms = rooms;
            self.room_map = map;
        }

        dt
    }

    /// Tick alien fauna: spawning, FSM behavior, sound, cleanup.
    fn tick_creatures(&mut self, dt: f32, events: &mut Vec<GameEventKind>) {
        use crate::creature_defs::{
            CREATURE_DUSKWEAVER, CREATURE_DUSTHARE, CREATURE_HOLLOWCALL, CreatureRegistry,
        };
        use crate::creatures::{Creature, CreatureState, MAX_CREATURES};

        let dt_game = dt * self.time_speed;
        let day_frac = (self.time_of_day / DAY_DURATION).rem_euclid(1.0);
        let is_night = day_frac > 0.85 || day_frac < 0.15;
        let force_creatures = self.debug_creatures_always;
        let approaching_dusk = force_creatures || (day_frac > 0.80 && day_frac < 0.85);
        let approaching_dawn = !force_creatures && (day_frac > 0.15 && day_frac < 0.20);
        let reg = CreatureRegistry::cached();
        let gw = GRID_W as i32;
        let gh = GRID_H as i32;

        // --- Pleb hunting behavior: follow creature, aim, shoot ---
        let hunt_range = 12.0f32; // max range to start aiming
        let hunt_follow = 18.0f32; // re-path if creature moves beyond this
        for pleb in &mut self.plebs {
            if pleb.is_dead || pleb.is_enemy {
                continue;
            }
            let ci = match pleb.hunt_target {
                Some(ci) => ci,
                None => continue,
            };
            let creature = match self.creatures.get(ci) {
                Some(c) if !c.is_dead => c,
                _ => {
                    // Target dead or invalid — stop hunting
                    pleb.hunt_target = None;
                    pleb.aim_pos = None;
                    continue;
                }
            };
            let dx = creature.x - pleb.x;
            let dy = creature.y - pleb.y;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < hunt_range {
                // In range: aim at creature position (precision shot)
                pleb.aim_pos = Some((creature.x, creature.y));
                pleb.angle = dy.atan2(dx);
                // Stop walking, focus on aiming
                if matches!(pleb.activity, PlebActivity::Walking) {
                    pleb.path.clear();
                    pleb.activity = PlebActivity::Idle;
                }
            } else {
                // Out of range: follow the creature
                pleb.aim_pos = None;
                pleb.aim_progress = 0.0;
                // Re-path periodically toward creature
                if pleb.path.is_empty() || pleb.path_idx >= pleb.path.len() || dist > hunt_follow {
                    let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                    let goal = (creature.x.floor() as i32, creature.y.floor() as i32);
                    let path = pleb::astar_path_terrain_water_wd(
                        &self.grid_data,
                        &self.wall_data,
                        &self.terrain_data,
                        &self.water_depth_cpu,
                        start,
                        goal,
                    );
                    if !path.is_empty() {
                        pleb.path = path;
                        pleb.path_idx = 0;
                        pleb.activity = PlebActivity::Walking;
                    }
                }
            }
        }

        // --- Spawn at dusk ---
        self.creature_spawn_timer -= dt_game;
        if self.creature_spawn_timer <= 0.0 {
            self.creature_spawn_timer = 8.0 + (self.frame_count as f32 * 0.1).sin().abs() * 7.0;

            if (approaching_dusk || is_night || force_creatures)
                && self.creatures.len() < MAX_CREATURES
            {
                // Spawn a duskweaver pack at a random map edge
                let has_duskweavers = self
                    .creatures
                    .iter()
                    .any(|c| c.species_id == CREATURE_DUSKWEAVER && !c.is_dead);
                if !has_duskweavers {
                    let pack_id = self.next_pack_id;
                    self.next_pack_id = self.next_pack_id.wrapping_add(1);
                    let def = reg.get(CREATURE_DUSKWEAVER);
                    // Escalation: night 1 = 1-2, night 2 = 2-3, night 3+ = full packs
                    let pack_size = def.map_or(4, |d| {
                        let (lo, hi) = match self.night_count {
                            0 => (1u8, 2u8),               // first night: timid
                            1 => (2, 4),                   // second night: small pack
                            2 => (3, 5),                   // third night: growing
                            _ => (d.pack_min, d.pack_max), // full packs
                        };
                        let range = hi.saturating_sub(lo) as u32 + 1;
                        let rng = (self.time_of_day * 1000.0) as u32 % range;
                        lo + rng as u8
                    }) as usize;
                    // Pick an edge
                    let edge = (self.next_pack_id % 4) as i32;
                    let (base_x, base_y) = match edge {
                        0 => (2.0, (gh / 2) as f32),
                        1 => ((gw - 3) as f32, (gh / 2) as f32),
                        2 => ((gw / 2) as f32, 2.0),
                        _ => ((gw / 2) as f32, (gh - 3) as f32),
                    };
                    for i in 0..pack_size.min(MAX_CREATURES - self.creatures.len()) {
                        let offset = (i as f32) * 0.8;
                        let cx = base_x + (i as f32 * 1.3).sin() * offset;
                        let cy = base_y + (i as f32 * 2.1).cos() * offset;
                        self.creatures
                            .push(Creature::new(CREATURE_DUSKWEAVER, cx, cy, pack_id));
                    }
                }

                // Spawn hollowcalls
                let has_hollowcall = self
                    .creatures
                    .iter()
                    .any(|c| c.species_id == CREATURE_HOLLOWCALL && !c.is_dead);
                if !has_hollowcall && self.creatures.len() < MAX_CREATURES {
                    // Distant position, far from any pleb
                    let hx = if day_frac > 0.5 {
                        20.0
                    } else {
                        (gw - 20) as f32
                    };
                    let hy = if self.next_pack_id % 2 == 0 {
                        20.0
                    } else {
                        (gh - 20) as f32
                    };
                    let pack_id = self.next_pack_id;
                    self.next_pack_id = self.next_pack_id.wrapping_add(1);
                    self.creatures
                        .push(Creature::new(CREATURE_HOLLOWCALL, hx, hy, pack_id));
                }
            }
        }

        // --- Dawn: despawn nocturnal creatures ---
        if approaching_dawn {
            for c in &mut self.creatures {
                let def = reg.get(c.species_id);
                if def.is_some_and(|d| d.nocturnal) && c.state != CreatureState::Despawn {
                    c.state = CreatureState::Despawn;
                    c.state_timer = 2.0;
                }
            }
        }

        // --- FSM tick ---
        // Collect light source positions for safe-zone checks
        // Pleb lights are always fresh; campfires only scanned every 30 frames
        let light_sources: Vec<(f32, f32, f32)> = {
            let mut lights: Vec<(f32, f32, f32)> = Vec::with_capacity(16);
            // Pleb torches and headlights (always current)
            for p in &self.plebs {
                if p.is_dead || p.is_enemy {
                    continue;
                }
                if p.torch_on {
                    lights.push((p.x, p.y, 5.0));
                }
                if p.headlight_mode > 0 {
                    lights.push((p.x, p.y, 8.0));
                }
            }
            // Campfires (reuse cached positions from cooking scan if available,
            // otherwise do a quick scan of placed campfires — typically < 5)
            for idx in 0..(GRID_W * GRID_H) as usize {
                let bt = self.grid_data[idx] & 0xFF;
                let h = (self.grid_data[idx] >> 8) & 0xFF;
                if (bt == BT_FIREPLACE || bt == BT_CAMPFIRE) && h > 0 {
                    let lx = (idx as u32 % GRID_W) as f32 + 0.5;
                    let ly = (idx as u32 / GRID_W) as f32 + 0.5;
                    lights.push((lx, ly, 6.0));
                }
            }
            lights
        };

        // Check if a position is in a lit safe zone
        let is_lit = |x: f32, y: f32| -> bool {
            light_sources
                .iter()
                .any(|&(lx, ly, r)| (x - lx).powi(2) + (y - ly).powi(2) < r * r)
        };

        // Pleb positions with exposure info: (x, y, has_personal_light, is_roofed, is_lit_area)
        let pleb_positions: Vec<(f32, f32, bool, bool, bool)> = self
            .plebs
            .iter()
            .filter(|p| !p.is_dead && !p.is_enemy)
            .map(|p| {
                let has_light = p.torch_on || p.headlight_mode > 0;
                let bx = p.x.floor() as i32;
                let by = p.y.floor() as i32;
                let is_roofed = if bx >= 0 && by >= 0 && bx < GRID_W as i32 && by < GRID_H as i32 {
                    let idx = (by as u32 * GRID_W + bx as u32) as usize;
                    idx < self.grid_data.len() && roof_height_rs(self.grid_data[idx]) > 0
                } else {
                    false
                };
                let in_light = is_lit(p.x, p.y);
                (p.x, p.y, has_light, is_roofed, in_light)
            })
            .collect();

        for ci in 0..self.creatures.len() {
            let c = &self.creatures[ci];
            if c.is_dead {
                continue;
            }
            let def = match reg.get(c.species_id) {
                Some(d) => d,
                None => continue,
            };
            let speed = def.speed * self.time_speed;

            match c.state.clone() {
                CreatureState::Idle => {
                    // Wander + scan for targets
                    let c = &mut self.creatures[ci];
                    c.state_timer += dt_game;

                    // Move along path if we have one
                    if c.path_idx < c.path.len() && speed > 0.0 {
                        let (tx, ty) = c.path[c.path_idx];
                        let dx = tx as f32 + 0.5 - c.x;
                        let dy = ty as f32 + 0.5 - c.y;
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist < 0.3 {
                            c.path_idx += 1;
                        } else {
                            let step = speed * dt;
                            c.x += dx / dist * step;
                            c.y += dy / dist * step;
                            c.angle = dy.atan2(dx);
                        }
                    } else if c.state_timer > 3.0 && speed > 0.0 {
                        // Pick a new wander target — random point, biased toward map center
                        c.state_timer = 0.0;
                        let seed = (c.x * 137.3 + c.y * 311.7 + c.sound_timer * 53.1) as u32;
                        let hash = seed.wrapping_mul(2654435761);
                        let rx = (hash % (gw as u32 - 20)) as i32 + 10;
                        let ry = ((hash >> 16) % (gh as u32 - 20)) as i32 + 10;
                        // Limit path distance to ~30 tiles to avoid expensive A*
                        let dx = (rx - c.x as i32).clamp(-30, 30);
                        let dy = (ry - c.y as i32).clamp(-30, 30);
                        let wx = (c.x as i32 + dx).clamp(3, gw - 4);
                        let wy = (c.y as i32 + dy).clamp(3, gh - 4);
                        c.path = astar_path_terrain_wd(
                            &self.grid_data,
                            &self.wall_data,
                            &self.terrain_data,
                            (c.x as i32, c.y as i32),
                            (wx, wy),
                        );
                        c.path_idx = 0;
                    }

                    // Duskweaver: hunt exposed colonists (not roofed, not in lit area)
                    if c.species_id == CREATURE_DUSKWEAVER && c.state_timer > 2.0 {
                        // Am I in a lit zone? If so, flee
                        let self_in_light = is_lit(c.x, c.y);
                        if self_in_light {
                            // Flee from nearest light
                            let edge_x = if c.x < (gw / 2) as f32 {
                                1.0
                            } else {
                                (gw - 2) as f32
                            };
                            let edge_y = if c.y < (gh / 2) as f32 {
                                1.0
                            } else {
                                (gh - 2) as f32
                            };
                            c.state = CreatureState::Flee(edge_x, edge_y);
                            c.state_timer = 0.0;
                            c.path = astar_path_terrain_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                (c.x as i32, c.y as i32),
                                (edge_x as i32, edge_y as i32),
                            );
                            c.path_idx = 0;
                        } else {
                            // Find nearest EXPOSED pleb (not roofed, not in lit area)
                            let mut best_target: Option<(f32, f32, f32)> = None;
                            for &(px, py, _has_light, is_roofed, in_light) in &pleb_positions {
                                if is_roofed || in_light {
                                    continue; // safe — can't target
                                }
                                let d = (px - c.x).powi(2) + (py - c.y).powi(2);
                                if d < 900.0 && best_target.map_or(true, |(_, _, bd)| d < bd) {
                                    // 30 tile detection range
                                    best_target = Some((px, py, d));
                                }
                            }

                            if let Some((tx, ty, _)) = best_target {
                                c.state = CreatureState::Stalk(tx, ty);
                                c.state_timer = 0.0;
                                c.path = astar_path_terrain_wd(
                                    &self.grid_data,
                                    &self.wall_data,
                                    &self.terrain_data,
                                    (c.x as i32, c.y as i32),
                                    (tx as i32, ty as i32),
                                );
                                c.path_idx = 0;
                            }
                        }
                    }

                    // Hollowcall: invisible stalker — creeps toward isolated plebs
                    if c.species_id == CREATURE_HOLLOWCALL && c.state_timer > 4.0 {
                        let self_in_light = is_lit(c.x, c.y);
                        if self_in_light {
                            // Flee from light (more aggressively than duskweavers)
                            let edge_x = if c.x < (gw / 2) as f32 {
                                1.0
                            } else {
                                (gw - 2) as f32
                            };
                            let edge_y = if c.y < (gh / 2) as f32 {
                                1.0
                            } else {
                                (gh - 2) as f32
                            };
                            c.state = CreatureState::Flee(edge_x, edge_y);
                            c.state_timer = 0.0;
                            c.path = astar_path_terrain_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                (c.x as i32, c.y as i32),
                                (edge_x as i32, edge_y as i32),
                            );
                            c.path_idx = 0;
                        } else if c.uncloak_timer <= 0.0 {
                            // Only hunt when cloaked — flee when exposed
                            let mut best_target: Option<(f32, f32, f32)> = None;
                            for &(px, py, _has_light, is_roofed, in_light) in &pleb_positions {
                                if is_roofed || in_light {
                                    continue;
                                }
                                let d = (px - c.x).powi(2) + (py - c.y).powi(2);
                                if d < 1600.0 && best_target.map_or(true, |(_, _, bd)| d < bd) {
                                    best_target = Some((px, py, d));
                                }
                            }
                            if let Some((tx, ty, _)) = best_target {
                                c.state = CreatureState::Stalk(tx, ty);
                                c.state_timer = 0.0;
                                c.path = astar_path_terrain_wd(
                                    &self.grid_data,
                                    &self.wall_data,
                                    &self.terrain_data,
                                    (c.x as i32, c.y as i32),
                                    (tx as i32, ty as i32),
                                );
                                c.path_idx = 0;
                            }
                        } else {
                            // Decloaked — flee until cloak restores
                            let edge_x = if c.x < (gw / 2) as f32 {
                                1.0
                            } else {
                                (gw - 2) as f32
                            };
                            let edge_y = if c.y < (gh / 2) as f32 {
                                1.0
                            } else {
                                (gh - 2) as f32
                            };
                            c.state = CreatureState::Flee(edge_x, edge_y);
                            c.state_timer = 0.0;
                            c.path = astar_path_terrain_wd(
                                &self.grid_data,
                                &self.wall_data,
                                &self.terrain_data,
                                (c.x as i32, c.y as i32),
                                (edge_x as i32, edge_y as i32),
                            );
                            c.path_idx = 0;
                        }
                    }
                }

                CreatureState::Stalk(tx, ty) => {
                    let c = &mut self.creatures[ci];
                    c.state_timer += dt_game;

                    // Move along path
                    if c.path_idx < c.path.len() && speed > 0.0 {
                        let (px, py) = c.path[c.path_idx];
                        let dx = px as f32 + 0.5 - c.x;
                        let dy = py as f32 + 0.5 - c.y;
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist < 0.3 {
                            c.path_idx += 1;
                        } else {
                            let step = speed * dt;
                            c.x += dx / dist * step;
                            c.y += dy / dist * step;
                            c.angle = dy.atan2(dx);
                        }
                    }

                    // Near target → attack the nearest pleb
                    let dist_to_target = (c.x - tx).powi(2) + (c.y - ty).powi(2);
                    if dist_to_target < 2.25 {
                        // Find nearest pleb for attack
                        let mut attack_idx: Option<usize> = None;
                        let mut attack_d = f32::MAX;
                        for (pi, p) in self.plebs.iter().enumerate() {
                            if p.is_dead || p.is_enemy {
                                continue;
                            }
                            let d = (p.x - c.x).powi(2) + (p.y - c.y).powi(2);
                            if d < 4.0 && d < attack_d {
                                attack_idx = Some(pi);
                                attack_d = d;
                            }
                        }
                        if let Some(pi) = attack_idx {
                            c.state = CreatureState::Attack(pi);
                            c.state_timer = 0.0;
                        } else {
                            // No pleb nearby (went inside?) — back to idle
                            c.state = CreatureState::Idle;
                            c.state_timer = 0.0;
                        }
                    }

                    // Flee if entering a lit zone
                    if is_lit(c.x, c.y) {
                        let edge_x = if c.x < (gw / 2) as f32 {
                            1.0
                        } else {
                            (gw - 2) as f32
                        };
                        let edge_y = if c.y < (gh / 2) as f32 {
                            1.0
                        } else {
                            (gh - 2) as f32
                        };
                        c.state = CreatureState::Flee(edge_x, edge_y);
                        c.state_timer = 0.0;
                        c.path = astar_path_terrain_wd(
                            &self.grid_data,
                            &self.wall_data,
                            &self.terrain_data,
                            (c.x as i32, c.y as i32),
                            (edge_x as i32, edge_y as i32),
                        );
                        c.path_idx = 0;
                    }

                    // Flee if too many colonists nearby (pack intimidation)
                    let nearby_colonists = pleb_positions
                        .iter()
                        .filter(|&&(px, py, _, _, _)| {
                            (px - c.x).powi(2) + (py - c.y).powi(2) < 64.0
                        })
                        .count();
                    if nearby_colonists >= def.flee_group_size as usize {
                        let edge_x = if c.x < (gw / 2) as f32 {
                            1.0
                        } else {
                            (gw - 2) as f32
                        };
                        let edge_y = if c.y < (gh / 2) as f32 {
                            1.0
                        } else {
                            (gh - 2) as f32
                        };
                        c.state = CreatureState::Flee(edge_x, edge_y);
                        c.state_timer = 0.0;
                        c.path = astar_path_terrain_wd(
                            &self.grid_data,
                            &self.wall_data,
                            &self.terrain_data,
                            (c.x as i32, c.y as i32),
                            (edge_x as i32, edge_y as i32),
                        );
                        c.path_idx = 0;
                    }

                    // Timeout
                    if c.state_timer > 30.0 {
                        c.state = CreatureState::Idle;
                        c.state_timer = 0.0;
                    }
                }

                CreatureState::Steal(sx, sy) => {
                    let c = &mut self.creatures[ci];
                    c.state_timer += dt_game;
                    if c.state_timer > 2.0 {
                        // Remove the ground item
                        if let Some(gi_idx) = self
                            .ground_items
                            .iter()
                            .position(|gi| gi.x.floor() as i32 == sx && gi.y.floor() as i32 == sy)
                        {
                            let name = ItemRegistry::cached()
                                .name(self.ground_items[gi_idx].stack.item_id)
                                .to_string();
                            self.ground_items.remove(gi_idx);
                            events.push(GameEventKind::Generic(
                                types::EventCategory::Combat,
                                format!("Duskweaver stole {}!", name),
                            ));
                        }
                        // Flee after stealing
                        let edge_x = if c.x < (gw / 2) as f32 {
                            1.0
                        } else {
                            (gw - 2) as f32
                        };
                        let edge_y = if c.y < (gh / 2) as f32 {
                            1.0
                        } else {
                            (gh - 2) as f32
                        };
                        c.state = CreatureState::Flee(edge_x, edge_y);
                        c.state_timer = 0.0;
                        c.path = astar_path_terrain_wd(
                            &self.grid_data,
                            &self.wall_data,
                            &self.terrain_data,
                            (c.x as i32, c.y as i32),
                            (edge_x as i32, edge_y as i32),
                        );
                        c.path_idx = 0;
                    }
                }

                CreatureState::Attack(pleb_idx) => {
                    let c = &mut self.creatures[ci];
                    // Hit-and-run: deal damage once, then flee
                    if let Some(pleb) = self.plebs.get_mut(pleb_idx) {
                        if !pleb.is_dead {
                            pleb.needs.health = (pleb.needs.health - def.damage / 100.0).max(0.0);
                            events.push(GameEventKind::Generic(
                                types::EventCategory::Combat,
                                format!("Duskweaver attacked {}!", pleb.name),
                            ));
                        }
                    }
                    let edge_x = if c.x < (gw / 2) as f32 {
                        1.0
                    } else {
                        (gw - 2) as f32
                    };
                    let edge_y = if c.y < (gh / 2) as f32 {
                        1.0
                    } else {
                        (gh - 2) as f32
                    };
                    c.state = CreatureState::Flee(edge_x, edge_y);
                    c.state_timer = 0.0;
                    c.path = astar_path_terrain_wd(
                        &self.grid_data,
                        &self.wall_data,
                        &self.terrain_data,
                        (c.x as i32, c.y as i32),
                        (edge_x as i32, edge_y as i32),
                    );
                    c.path_idx = 0;
                }

                CreatureState::Flee(fx, fy) => {
                    let c = &mut self.creatures[ci];
                    c.state_timer += dt_game;

                    // Move along path
                    if c.path_idx < c.path.len() && speed > 0.0 {
                        let (px, py) = c.path[c.path_idx];
                        let dx = px as f32 + 0.5 - c.x;
                        let dy = py as f32 + 0.5 - c.y;
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist < 0.3 {
                            c.path_idx += 1;
                        } else {
                            let step = speed * 1.5 * dt; // flee faster
                            c.x += dx / dist * step;
                            c.y += dy / dist * step;
                            c.angle = dy.atan2(dx);
                        }
                    }

                    // Near edge or path done → idle
                    let near_edge =
                        c.x < 3.0 || c.x > (gw - 3) as f32 || c.y < 3.0 || c.y > (gh - 3) as f32;
                    if near_edge || c.path_idx >= c.path.len() {
                        c.state = CreatureState::Idle;
                        c.state_timer = 0.0;
                    }

                    let _ = (fx, fy); // used for pathfinding target already
                }

                CreatureState::Browse => {
                    // Passive fauna: short hops, pause, scan for threats
                    let c = &mut self.creatures[ci];
                    c.state_timer += dt_game;

                    // Follow path (hop movement)
                    if c.path_idx < c.path.len() && speed > 0.0 {
                        let (tx, ty) = c.path[c.path_idx];
                        let dx = tx as f32 + 0.5 - c.x;
                        let dy = ty as f32 + 0.5 - c.y;
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist < 0.3 {
                            c.path_idx += 1;
                            c.hop_phase = 0.0; // land
                        } else {
                            let step = speed * dt;
                            c.x += dx / dist * step;
                            c.y += dy / dist * step;
                            c.angle = dy.atan2(dx);
                            // Advance hop cycle
                            c.hop_phase = (c.hop_phase + dt * 4.0) % 1.0;
                        }
                    } else {
                        c.hop_phase = 0.0; // grounded when idle
                    }

                    // Pick a new short hop destination every 2-4 seconds
                    if c.path_idx >= c.path.len() && c.state_timer > 2.0 {
                        c.state_timer = 0.0;
                        let seed =
                            ((c.x * 137.3 + c.y * 311.7 + c.sound_timer * 53.1) * 1000.0) as u32;
                        let hash = seed.wrapping_mul(2654435761);
                        // Short hops: 2-4 tiles
                        let hop_dist = 2 + (hash % 3) as i32;
                        let angle_idx = (hash >> 8) % 8;
                        let angles: [(i32, i32); 8] = [
                            (1, 0),
                            (1, 1),
                            (0, 1),
                            (-1, 1),
                            (-1, 0),
                            (-1, -1),
                            (0, -1),
                            (1, -1),
                        ];
                        let (adx, ady) = angles[angle_idx as usize];
                        let wx = (c.x as i32 + adx * hop_dist).clamp(3, gw - 4);
                        let wy = (c.y as i32 + ady * hop_dist).clamp(3, gh - 4);
                        c.path = astar_path_terrain_wd(
                            &self.grid_data,
                            &self.wall_data,
                            &self.terrain_data,
                            (c.x as i32, c.y as i32),
                            (wx, wy),
                        );
                        c.path_idx = 0;
                    }

                    // Scan for nearby plebs → scatter
                    let flee_r = def.flee_radius;
                    if flee_r > 0.0 {
                        let flee_r2 = flee_r * flee_r;
                        let mut closest_threat: Option<(f32, f32, f32)> = None;
                        for &(px, py, _, _, _) in &pleb_positions {
                            let d2 = (px - c.x).powi(2) + (py - c.y).powi(2);
                            if d2 < flee_r2 && closest_threat.map_or(true, |(_, _, bd)| d2 < bd) {
                                closest_threat = Some((px, py, d2));
                            }
                        }
                        if let Some((px, py, _)) = closest_threat {
                            // Flee away from the threat
                            let away_x = c.x - px;
                            let away_y = c.y - py;
                            let len = (away_x * away_x + away_y * away_y).sqrt().max(0.1);
                            c.state = CreatureState::Scatter(away_x / len, away_y / len);
                            c.state_timer = 0.0;
                            c.path.clear();
                            c.path_idx = 0;
                        }
                    }
                }

                CreatureState::Scatter(away_x, away_y) => {
                    // Burst flee for a short distance, then resume browsing
                    let c = &mut self.creatures[ci];
                    c.state_timer += dt_game;

                    // Sprint away at 1.5x speed
                    let sprint = speed * 1.5 * dt;
                    // Add some randomness to scatter direction
                    let seed = ((c.x * 73.1 + c.y * 137.3) * 100.0) as u32;
                    let jitter = (seed.wrapping_mul(2654435761) & 0xFFFF) as f32 / 65535.0 - 0.5;
                    let jx = away_x + jitter * 0.3;
                    let jy = away_y + jitter * 0.3;
                    let len = (jx * jx + jy * jy).sqrt().max(0.1);
                    c.x = (c.x + jx / len * sprint).clamp(2.0, (gw - 2) as f32);
                    c.y = (c.y + jy / len * sprint).clamp(2.0, (gh - 2) as f32);
                    c.angle = jy.atan2(jx);
                    c.hop_phase = (c.hop_phase + dt * 6.0) % 1.0; // fast hopping

                    // After ~1.5 seconds, return to browsing
                    if c.state_timer > 1.5 {
                        c.state = CreatureState::Browse;
                        c.state_timer = 0.0;
                        c.hop_phase = 0.0;
                    }
                }

                CreatureState::Despawn => {
                    let c = &mut self.creatures[ci];
                    c.state_timer -= dt_game;
                    if c.state_timer <= 0.0 {
                        c.is_dead = true; // will be removed in cleanup
                    }
                }
            }

            // --- Sound emission ---
            let c = &self.creatures[ci];
            if !c.is_dead && def.sound_amplitude_db > 0.0 {
                let c = &mut self.creatures[ci];
                c.sound_timer -= dt_game;
                if c.sound_timer <= 0.0 {
                    c.sound_timer = def.sound_interval
                        + (c.x * 7.3 + c.y * 11.1).sin().abs() * def.sound_interval * 0.3;
                    let duration = if c.species_id == CREATURE_HOLLOWCALL {
                        4.0
                    } else {
                        0.3
                    };
                    self.sound_sources.push(types::SoundSource {
                        x: c.x,
                        y: c.y,
                        amplitude: types::db_to_amplitude(def.sound_amplitude_db),
                        frequency: def.sound_frequency,
                        phase: 0.0,
                        pattern: def.sound_pattern,
                        duration,
                        fresh: true,
                    });
                }
            }
        }

        // --- Uncloak timer + position tracking ---
        for c in &mut self.creatures {
            if c.uncloak_timer > 0.0 {
                c.uncloak_timer = (c.uncloak_timer - dt_game).max(0.0);
            }
            c.prev_x = c.x;
            c.prev_y = c.y;
        }

        // --- Bleeding: blood loss + blood drops on ground ---
        for c in &mut self.creatures {
            if c.is_dead || c.bleeding <= 0.0 {
                continue;
            }
            // Blood loss reduces health over time
            let blood_loss = c.bleeding * 2.0 * dt_game; // bleeding 1.0 = 2 HP/sec
            c.health -= blood_loss;
            if c.health <= 0.0 {
                c.is_dead = true;
                c.corpse_timer = 60.0;
            }

            // Bleeding slowly clots (unless wound is severe)
            c.bleeding = (c.bleeding - 0.02 * dt_game).max(0.0);

            // Drop blood marks on the ground
            c.blood_drop_timer -= dt_game;
            if c.blood_drop_timer <= 0.0 {
                c.blood_drop_timer = 0.5 / c.bleeding.max(0.1); // faster drips = more bleeding
                self.blood_stains.push((c.x, c.y, 3.0)); // (x, y, fade_timer)
            }
        }

        // --- Fade blood stains ---
        for stain in &mut self.blood_stains {
            stain.2 -= dt_game;
        }
        self.blood_stains.retain(|s| s.2 > 0.0);

        // Dead creatures remain as corpses until butchered or timer expires
        // (no auto-drop — butchering required for loot)

        // --- Dusthare reproduction: slow population recovery ---
        {
            let dusthare_count = self
                .creatures
                .iter()
                .filter(|c| c.species_id == CREATURE_DUSTHARE && !c.is_dead)
                .count();
            // If at least 2 alive and under cap, chance to spawn a new one
            if dusthare_count >= 2 && dusthare_count < 20 && self.creatures.len() < MAX_CREATURES {
                self.dusthare_repro_timer -= dt_game;
                if self.dusthare_repro_timer <= 0.0 {
                    // Pick a random living dusthare to spawn near
                    let parents: Vec<(f32, f32)> = self
                        .creatures
                        .iter()
                        .filter(|c| c.species_id == CREATURE_DUSTHARE && !c.is_dead)
                        .map(|c| (c.x, c.y))
                        .collect();
                    if let Some(&(px, py)) = parents.first() {
                        let seed = (px * 137.3 + py * 311.7) as u32;
                        let hash = seed.wrapping_mul(2654435761);
                        let ox = ((hash & 0xFF) as f32 / 255.0 - 0.5) * 6.0;
                        let oy = (((hash >> 8) & 0xFF) as f32 / 255.0 - 0.5) * 6.0;
                        let nx = (px + ox).clamp(5.0, (gw - 5) as f32);
                        let ny = (py + oy).clamp(5.0, (gh - 5) as f32);
                        let pid = self.next_pack_id;
                        self.next_pack_id = self.next_pack_id.wrapping_add(1);
                        self.creatures
                            .push(Creature::new(CREATURE_DUSTHARE, nx, ny, pid));
                    }
                    // Next reproduction in 90-150 game seconds
                    self.dusthare_repro_timer =
                        90.0 + ((self.time_of_day * 1000.0) as u32 % 60) as f32;
                }
            }
        }

        // --- Snare catch mechanic: check every ~60 game-seconds ---
        // Uses frame_count to amortize (check ~once per real second)
        if self.frame_count % 60 == 30 {
            let grid_size = (GRID_W * GRID_H) as usize;
            // Collect snare positions with remaining durability
            let snares: Vec<(i32, i32, usize)> = (0..grid_size)
                .filter(|&idx| {
                    let bt = self.grid_data[idx] & 0xFF;
                    let h = (self.grid_data[idx] >> 8) & 0xFF;
                    bt == BT_SNARE && h > 0
                })
                .map(|idx| {
                    (
                        (idx as u32 % GRID_W) as i32,
                        (idx as u32 / GRID_W) as i32,
                        idx,
                    )
                })
                .collect();

            for (sx, sy, sidx) in &snares {
                // 25% catch chance per check (~1 check per real second, so ~15 game-seconds between attempts)
                let hash = (*sidx as u32)
                    .wrapping_mul(2654435761)
                    .wrapping_add(self.frame_count as u32);
                if hash % 4 != 0 {
                    continue;
                }
                // Find nearest living dusthare within 15 tiles
                let catch_range_sq = 15.0 * 15.0;
                let mut caught_idx: Option<usize> = None;
                let mut best_d = f32::MAX;
                for (ci, c) in self.creatures.iter().enumerate() {
                    if c.is_dead || c.species_id != CREATURE_DUSTHARE {
                        continue;
                    }
                    let d = (c.x - *sx as f32 - 0.5).powi(2) + (c.y - *sy as f32 - 0.5).powi(2);
                    if d < catch_range_sq && d < best_d {
                        best_d = d;
                        caught_idx = Some(ci);
                    }
                }
                if let Some(ci) = caught_idx {
                    // Caught! Kill creature, move corpse to snare (needs butchering)
                    self.creatures[ci].is_dead = true;
                    self.creatures[ci].corpse_timer = 60.0; // long enough to butcher
                    self.creatures[ci].x = *sx as f32 + 0.5;
                    self.creatures[ci].y = *sy as f32 + 0.5;

                    // Decrement snare durability (sidx bounds-checked in scan above)
                    let h = if *sidx < self.grid_data.len() {
                        (self.grid_data[*sidx] >> 8) & 0xFF
                    } else {
                        0
                    };
                    if *sidx < self.grid_data.len() {
                        if h > 1 {
                            self.grid_data[*sidx] =
                                (self.grid_data[*sidx] & 0xFFFF00FF) | ((h - 1) << 8);
                        } else {
                            // Broken: set height to 0
                            self.grid_data[*sidx] = self.grid_data[*sidx] & 0xFFFF00FF;
                        }
                    }
                    self.grid_dirty = true;

                    events.push(GameEventKind::Generic(
                        types::EventCategory::General,
                        format!(
                            "Snare caught a dusthare! ({} uses left)",
                            h.saturating_sub(1)
                        ),
                    ));
                }
            }
        }

        // --- Corpse timer + cleanup ---
        for c in &mut self.creatures {
            if c.is_dead && c.corpse_timer > 0.0 {
                c.corpse_timer -= dt_game;
            }
        }
        self.creatures
            .retain(|c| !c.is_dead || c.corpse_timer > 0.0);
    }
}

/// Tick a single pleb's activity state machine and auto-behaviors.
/// Handles: sleep/harvest/eat progress, crisis triggers (starving/exhausted/overheating),
/// and non-crisis auto-behaviors (auto-eat, auto-sleep, auto-harvest).
fn tick_pleb_activity(
    pleb: &mut Pleb,
    env: &needs::EnvSample,
    grid: &mut [u32],
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
            let is_tree = pleb.harvest_target.is_some_and(|(hx, hy)| {
                let hidx = (hy as u32 * GRID_W + hx as u32) as usize;
                hidx < grid.len() && (grid[hidx] & 0xFF) == BT_TREE
            });
            let knife_bonus = if !is_tree && pleb.has_tool("knife") {
                1.3
            } else {
                1.0
            };
            let harvest_speed =
                if is_tree { 1.5 } else { 5.0 } * knife_bonus * pleb.farming_speed();
            let new_progress = progress + dt * time_speed * ACTION_SPEED_MUL * harvest_speed;
            if new_progress >= 1.0 {
                // Check what we're harvesting: tree → sticks, bush → berries
                let is_tree_target = pleb.harvest_target.is_some_and(|(hx, hy)| {
                    let hidx = (hy as u32 * GRID_W + hx as u32) as usize;
                    hidx < grid.len() && (grid[hidx] & 0xFF) == BT_TREE
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
                } else if let Some((hx, hy)) = pleb.harvest_target {
                    let hidx = (hy as u32 * GRID_W + hx as u32) as usize;
                    if hidx < grid.len() {
                        let hbt = grid[hidx] & 0xFF;
                        let h_left = (grid[hidx] >> 8) & 0xFF;
                        match hbt {
                            BT_BERRY_BUSH => {
                                if h_left > 0 {
                                    let new_count = h_left - 1;
                                    grid[hidx] = (grid[hidx] & 0xFFFF00FF) | (new_count << 8);
                                    ground_items.push(resources::GroundItem::new(
                                        pleb.x,
                                        pleb.y,
                                        ITEM_BERRIES,
                                        1,
                                    ));
                                }
                            }
                            BT_DUSTWHISKER => {
                                // Harvest fiber from grass, then remove plant
                                let rng = ((pleb.x * 97.3 + pleb.y * 211.7) as u32)
                                    .wrapping_mul(2654435761);
                                let count = 2 + (rng % 2) as u16; // 2-3 fiber
                                ground_items.push(resources::GroundItem::new(
                                    pleb.x, pleb.y, ITEM_FIBER, count,
                                ));
                                grid[hidx] = make_block(BT_GROUND as u8, 0, 0);
                                log::info!(
                                    "{} gathered {} fiber from dustwhisker",
                                    pleb.name,
                                    count
                                );
                            }
                            BT_HOLLOW_REED => {
                                let rng = ((pleb.x * 113.3 + pleb.y * 277.7) as u32)
                                    .wrapping_mul(2654435761);
                                let count = 2 + (rng % 2) as u16; // 2-3 reed stalks
                                ground_items.push(resources::GroundItem::new(
                                    pleb.x,
                                    pleb.y,
                                    ITEM_REED_STALK,
                                    count,
                                ));
                                grid[hidx] = make_block(BT_GROUND as u8, 0, 0);
                                log::info!("{} gathered {} reed stalks", pleb.name, count);
                            }
                            BT_THORNBRAKE => {
                                let rng = ((pleb.x * 73.3 + pleb.y * 191.7) as u32)
                                    .wrapping_mul(2654435761);
                                let stick_count = 2 + (rng % 2) as u16;
                                let thorn_count = 1 + (rng >> 8 & 1) as u16; // 1-2
                                ground_items.push(resources::GroundItem::new(
                                    pleb.x + 0.1,
                                    pleb.y,
                                    ITEM_SCRAP_WOOD,
                                    stick_count,
                                ));
                                ground_items.push(resources::GroundItem::new(
                                    pleb.x - 0.1,
                                    pleb.y,
                                    ITEM_THORNS,
                                    thorn_count,
                                ));
                                grid[hidx] = make_block(BT_GROUND as u8, 0, 0);
                                log::info!(
                                    "{} harvested thornbrake: {} sticks + {} thorns",
                                    pleb.name,
                                    stick_count,
                                    thorn_count
                                );
                            }
                            BT_SALTBRUSH => {
                                let rng = ((pleb.x * 53.3 + pleb.y * 317.7) as u32)
                                    .wrapping_mul(2654435761);
                                let count = 1 + (rng % 2) as u16; // 1-2 salt
                                ground_items.push(resources::GroundItem::new(
                                    pleb.x, pleb.y, ITEM_SALT, count,
                                ));
                                grid[hidx] = make_block(BT_GROUND as u8, 0, 0);
                                log::info!("{} gathered {} salt from saltbrush", pleb.name, count);
                            }
                            BT_DUSKBLOOM => {
                                // Time-dependent harvest: nectar at night, dried petals by day
                                let day_frac = (time_of_day / DAY_DURATION).fract();
                                let is_blooming = day_frac > 0.75 || day_frac < 0.15;
                                if is_blooming {
                                    ground_items.push(resources::GroundItem::new(
                                        pleb.x,
                                        pleb.y,
                                        ITEM_NECTAR,
                                        1,
                                    ));
                                    log::info!(
                                        "{} harvested nectar from blooming duskbloom",
                                        pleb.name
                                    );
                                } else {
                                    ground_items.push(resources::GroundItem::new(
                                        pleb.x,
                                        pleb.y,
                                        ITEM_DRIED_PETALS,
                                        1,
                                    ));
                                    log::info!(
                                        "{} collected dried petals from duskbloom",
                                        pleb.name
                                    );
                                }
                                grid[hidx] = make_block(BT_GROUND as u8, 0, 0);
                            }
                            _ => {}
                        }
                    }
                }
                pleb.gain_xp_logged(pleb::SKILL_FARMING, 10.0, time_of_day);
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
            let item_reg = ItemRegistry::cached();

            // Sickness roll helper: returns true if nausea triggered
            let sickness_roll = |pleb: &mut Pleb, food_id: u16, time: f32| -> bool {
                // StoneEater trait: immune to food sickness
                if pleb.immune_to_food_sickness() {
                    return false;
                }
                let chance = item_reg
                    .get(food_id)
                    .map(|d| d.sickness_chance)
                    .unwrap_or(0.0);
                if chance > 0.0 {
                    // Simple hash-based RNG from pleb position + time
                    let seed = (pleb.x * 137.3 + pleb.y * 311.7 + time * 53.1) as u32;
                    let roll = (seed.wrapping_mul(2654435761) & 0xFFFF) as f32 / 65535.0;
                    if roll < chance {
                        pleb.nauseous_timer = 30.0;
                        pleb.needs.mood -= 10.0;
                        pleb.set_bubble(pleb::BubbleKind::Thought("Ugh... feel sick".into()), 3.0);
                        pleb.log_event(time, "Got nauseous from raw food".into());
                        return true;
                    }
                }
                false
            };

            // Try eating from inventory first (best food = highest nutrition)
            if let Some((food_id, nutrition)) = pleb.inventory.best_food() {
                pleb.inventory.remove(food_id, 1);
                let gt = time_of_day;
                if sickness_roll(pleb, food_id, gt) {
                    // Nausea: nutrition wasted (vomited)
                    log::info!("{} ate raw food and got sick", pleb.name);
                } else {
                    pleb.needs.hunger = (pleb.needs.hunger + nutrition).min(1.0);
                }
                ate = true;
            }
            // Try eating from ground item at harvest_target (any food)
            if !ate
                && let Some((tx, ty)) = pleb.harvest_target
                && let Some(gi) = ground_items.iter_mut().position(|item| {
                    item.x.floor() as i32 == tx
                        && item.y.floor() as i32 == ty
                        && item_reg
                            .get(item.stack.item_id)
                            .is_some_and(|d| d.nutrition > 0.0)
                })
            {
                let ground_food_id = ground_items[gi].stack.item_id;
                let food_nutr = item_reg
                    .get(ground_food_id)
                    .map(|d| d.nutrition)
                    .unwrap_or(0.1);
                if ground_items[gi].stack.count <= 1 {
                    ground_items.remove(gi);
                } else {
                    ground_items[gi].stack.count -= 1;
                }
                let gt = time_of_day;
                if sickness_roll(pleb, ground_food_id, gt) {
                    // Nausea: nutrition wasted
                } else {
                    pleb.needs.hunger = (pleb.needs.hunger + food_nutr).min(1.0);
                }
                ate = true;
            }
            pleb.harvest_target = None;
            if was_crisis && pleb.needs.hunger < 0.3 && (pleb.inventory.has_food() || ate) {
                pleb.activity = PlebActivity::Crisis(
                    Box::new(PlebActivity::Eating),
                    crisis_reason.unwrap_or("Starving"),
                );
            } else {
                pleb.activity = PlebActivity::Idle;
            }
        }
        PlebActivity::Drinking(progress) => {
            let new_progress = progress + dt * time_speed * ACTION_SPEED_MUL / WELL_DRINK_TIME;
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
        PlebActivity::Butchering(_) => {
            // Handled in main tick (needs creature access)
        }
        PlebActivity::Fishing(_) => {
            // Handled in main tick (needs ground_items access)
        }
        PlebActivity::Mining(_) => {
            // Handled in main tick (needs mining_grids + ground_items access)
        }
        _ => {}
    }

    // --- Crisis auto-behaviors (override player control) ---
    let is_idle_or_walk = matches!(
        pleb.activity.inner(),
        PlebActivity::Idle | PlebActivity::Walking
    );

    if pleb.needs.hunger < 0.10 && is_idle_or_walk {
        // CRISIS: Starving — eat any food from inventory, or seek berry bush
        if pleb.inventory.has_food() {
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
    } else if false {
        // DISABLED: Freezing crisis (plebs can work while cold)
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
    } else if !pleb.activity.is_crisis() && !pleb.drafted {
        // Non-crisis auto-behaviors (only when undrafted)
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
                        if d < 900.0 && best_berry.is_none_or(|(_, _, bd)| d < bd) {
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
                } else if pleb.harvest_target.is_none()
                    && let Some((bx, by)) = env.nearest_berry_bush
                {
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

impl App {
    /// Generate contextual hints based on current game state.
    pub(crate) fn generate_hints(&mut self) {
        self.game_hints.clear();

        let _item_reg = item_defs::ItemRegistry::cached();

        // Check blueprints for missing materials
        for (&(_bx, _by), bp) in &self.blueprints {
            if bp.resources_met() && !bp.is_roof() && !bp.uses_sticks() {
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
            } else if bp.uses_sticks() {
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
                } else if bp.uses_sticks() {
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
