//! UI drawing — all egui panels, overlays, debug tooltips.
//! Extracted from render() to keep main.rs manageable.

use crate::*;

impl App {
    /// Pixels-per-point scale factor for the current window.
    pub(crate) fn ppp(&self) -> f32 {
        self.window
            .as_ref()
            .map(|w| w.scale_factor() as f32)
            .unwrap_or(1.0)
    }

    /// Convert world coords to screen coords for egui overlay drawing.
    pub(crate) fn world_to_screen_ui(
        &self,
        wx: f32,
        wy: f32,
        bp_cam: (f32, f32, f32, f32, f32),
    ) -> egui::Pos2 {
        let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
        let ppp = self.ppp();
        let sx = ((wx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / ppp;
        let sy = ((wy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / ppp;
        egui::pos2(sx, sy)
    }

    /// Tile size in screen pixels at current zoom.
    #[allow(dead_code)]
    pub(crate) fn tile_px(&self, bp_cam: (f32, f32, f32, f32, f32)) -> f32 {
        bp_cam.2 / self.render_scale / self.ppp()
    }

    pub fn draw_ui(
        &mut self,
        ctx: &egui::Context,
        bp_cam: (f32, f32, f32, f32, f32),
        blueprint_tiles: Vec<((i32, i32), u8)>,
        dt: f32,
    ) {
        // Reset hover sound debounce if no pointer interaction
        if ctx.input(|i| i.pointer.hover_pos().is_none()) {
            self.menu_hover_id = None;
        }

        // Font setup: Inter Medium for thicker UI text
        {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "Inter".to_owned(),
                egui::FontData::from_static(include_bytes!("../assets/fonts/Inter-Variable.ttf"))
                    .into(),
            );
            // Insert Inter as first priority for proportional family
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "Inter".to_owned());
            ctx.set_fonts(fonts);
        }
        {
            let mut style = (*ctx.style()).clone();
            style.text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(13.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Button,
                egui::FontId::new(13.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Small,
                egui::FontId::new(11.0, egui::FontFamily::Proportional),
            );
            ctx.set_style(style);
        }

        match self.game_state {
            GameState::MainMenu => self.draw_main_menu(ctx),
            GameState::MapGen => self.draw_map_gen_screen(ctx),
            GameState::CharGen => self.draw_manifest_screen(ctx),
            GameState::Playing => {
                let bp_ppp = self.ppp();
                self.draw_resource_bar(ctx);
                self.draw_layers_bar(ctx);
                self.draw_layer_legend(ctx);
                self.draw_menu_bar(ctx, dt);
                self.draw_inventory_window(ctx);
                self.draw_build_bar(ctx);
                self.draw_colonist_bar(ctx);
                self.draw_context_menus(ctx, bp_ppp, bp_cam);
                self.draw_overlays_and_popups(ctx, bp_cam, bp_ppp, dt);
                self.draw_world_overlays(ctx, bp_cam, &blueprint_tiles);
                self.draw_world_labels(ctx, bp_cam);
                self.draw_action_bar(ctx);
                self.draw_selection_info(ctx);
                self.draw_notifications(ctx);
                self.draw_hints(ctx, bp_cam, bp_ppp);
                self.draw_conditions_bar(ctx);
                self.draw_hover_info(ctx);
                self.draw_game_log(ctx);
                self.draw_minimap(ctx);
                self.draw_stone_lab(ctx);

                // Crash landing card (shown once on game start)
                if self.show_crash_card {
                    self.draw_crash_card(ctx);
                }

                // Pause overlay (darkened screen + centered text)
                if self.time_paused && !self.show_pause_menu && !self.show_crash_card {
                    let screen = ctx.content_rect();
                    let painter = ctx.layer_painter(egui::LayerId::new(
                        egui::Order::Foreground,
                        egui::Id::new("pause_overlay"),
                    ));
                    painter.rect_filled(
                        screen,
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 40),
                    );
                    painter.text(
                        screen.center(),
                        egui::Align2::CENTER_CENTER,
                        "PAUSED",
                        egui::FontId::proportional(28.0),
                        egui::Color32::from_rgba_unmultiplied(220, 220, 220, 180),
                    );
                    painter.text(
                        screen.center() + egui::Vec2::new(0.0, 28.0),
                        egui::Align2::CENTER_CENTER,
                        "Space to resume",
                        egui::FontId::proportional(12.0),
                        egui::Color32::from_rgba_unmultiplied(160, 160, 160, 140),
                    );
                }

                // Game menu (ESC when nothing selected)
                if self.show_pause_menu {
                    let screen = ctx.content_rect();
                    let painter = ctx.layer_painter(egui::LayerId::new(
                        egui::Order::Foreground,
                        egui::Id::new("game_menu_bg"),
                    ));
                    painter.rect_filled(
                        screen,
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 120),
                    );

                    egui::Area::new(egui::Id::new("game_menu"))
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .interactable(true)
                        .order(egui::Order::Foreground)
                        .show(ctx, |ui| {
                            egui::Frame::window(ui.style())
                                .fill(egui::Color32::from_rgb(25, 28, 32))
                                .show(ui, |ui| {
                                    ui.set_min_width(200.0);
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(8.0);
                                        ui.label(
                                            egui::RichText::new("RAYWORLD")
                                                .size(22.0)
                                                .strong()
                                                .color(egui::Color32::from_rgb(200, 190, 170)),
                                        );
                                        ui.add_space(4.0);
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "Day {} \u{2022} {}",
                                                (self.time_of_day / DAY_DURATION) as u32 + 1,
                                                {
                                                    let frac = self.time_of_day / DAY_DURATION;
                                                    let h = (frac * 24.0) as u32;
                                                    let m =
                                                        ((frac * 24.0 - h as f32) * 60.0) as u32;
                                                    format!("{:02}:{:02}", h, m)
                                                }
                                            ))
                                            .size(11.0)
                                            .color(egui::Color32::from_gray(120)),
                                        );
                                        ui.add_space(12.0);
                                        ui.separator();
                                        ui.add_space(8.0);

                                        let btn = |ui: &mut egui::Ui, text: &str| -> bool {
                                            ui.add_sized(
                                                egui::vec2(180.0, 32.0),
                                                egui::Button::new(
                                                    egui::RichText::new(text).size(14.0),
                                                ),
                                            )
                                            .clicked()
                                        };

                                        if btn(ui, "\u{25b6} Resume") {
                                            self.show_pause_menu = false;
                                            self.time_paused = false;
                                        }
                                        ui.add_space(4.0);
                                        if btn(ui, "\u{2699} Settings") {
                                            // TODO: settings screen
                                        }
                                        ui.add_space(4.0);
                                        if btn(ui, "\u{1f4be} Save Game") {
                                            // TODO: save
                                        }
                                        ui.add_space(4.0);
                                        if btn(ui, "\u{1f3e0} Main Menu") {
                                            self.show_pause_menu = false;
                                            self.time_paused = false;
                                            self.game_state = GameState::MainMenu;
                                        }
                                        ui.add_space(4.0);
                                        if btn(ui, "\u{274c} Quit Game") {
                                            std::process::exit(0);
                                        }
                                        ui.add_space(8.0);
                                    });
                                });
                        });
                }
            }
        }
    }

    fn draw_menu_bar(&mut self, ctx: &egui::Context, _dt: f32) {
        let mut time_val = self.time_of_day;
        let mut paused = self.time_paused;
        let mut speed = self.time_speed;
        let mut zoom = self.camera.zoom;
        let mut glass_light = self.camera.glass_light_mul;
        let mut indoor_glow = self.camera.indoor_glow_mul;
        let mut bleed = self.camera.light_bleed_mul;
        let mut foliage_opacity = self.camera.foliage_opacity;
        let mut foliage_variation = self.camera.foliage_variation;
        let mut oblique = self.camera.oblique_strength;
        let base_zoom = (self.camera.screen_w / 64.0).min(self.camera.screen_h / 64.0);

        // --- Top menu bar ---
        egui::TopBottomPanel::top("top_menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                // Time menu
                let day_frac = time_val / DAY_DURATION;
                let hours = (day_frac * 24.0) as u32;
                let minutes = ((day_frac * 24.0 - hours as f32) * 60.0) as u32;
                let phase = if day_frac < 0.15 {
                    "Night"
                } else if day_frac < 0.25 {
                    "Dawn"
                } else if day_frac < 0.75 {
                    "Day"
                } else if day_frac < 0.85 {
                    "Dusk"
                } else {
                    "Night"
                };
                // Fixed-width time label: pad phase to 5 chars so menu doesn't shift
                let time_label = format!("{:02}:{:02} {:5}", hours, minutes, phase);
                ui.menu_button(egui::RichText::new(time_label).monospace(), |ui| {
                    ui.add(
                        egui::Slider::new(&mut time_val, 0.0..=DAY_DURATION)
                            .text("Time")
                            .show_value(false),
                    );
                    ui.horizontal(|ui| {
                        if ui.button(if paused { "Play" } else { "Pause" }).clicked() {
                            paused = !paused;
                        }
                        let pct = (speed / 0.1 * 100.0).round() as u32;
                        ui.add(
                            egui::Slider::new(&mut speed, 0.05..=0.4)
                                .text(format!("{}%", pct))
                                .logarithmic(true),
                        );
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Night").clicked() {
                            time_val = DAY_DURATION * 0.0;
                            paused = true;
                            self.camera.force_refresh = 5.0;
                        }
                        if ui.button("Dawn").clicked() {
                            time_val = DAY_DURATION * 0.18;
                            paused = true;
                            self.camera.force_refresh = 5.0;
                        }
                        if ui.button("Day").clicked() {
                            time_val = DAY_DURATION * 0.5;
                            paused = true;
                            self.camera.force_refresh = 5.0;
                        }
                        if ui.button("Dusk").clicked() {
                            time_val = DAY_DURATION * 0.82;
                            paused = true;
                            self.camera.force_refresh = 5.0;
                        }
                    });
                });

                ui.separator();

                // Weather menu
                let weather_label = match &self.weather {
                    WeatherState::Clear => "Clear",
                    WeatherState::Cloudy => "Cloudy",
                    WeatherState::LightRain => "Light Rain",
                    WeatherState::HeavyRain => "Heavy Rain",
                };
                ui.menu_button(format!("Weather: {}", weather_label), |ui| {
                    if ui.button("Clear").clicked() {
                        self.weather = WeatherState::Clear;
                        self.weather_timer = 45.0;
                    }
                    if ui.button("Cloudy").clicked() {
                        self.weather = WeatherState::Cloudy;
                        self.weather_timer = 45.0;
                    }
                    if ui.button("Light Rain").clicked() {
                        self.weather = WeatherState::LightRain;
                        self.weather_timer = 45.0;
                    }
                    if ui.button("Heavy Rain").clicked() {
                        self.weather = WeatherState::HeavyRain;
                        self.weather_timer = 45.0;
                    }
                });

                ui.separator();

                // Menus with sliders/toggles should stay open on click
                let keep_open = egui::containers::menu::MenuConfig::new()
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);

                // Lighting menu
                egui::containers::menu::MenuButton::new("Lighting")
                    .config(keep_open.clone())
                    .ui(ui, |ui| {
                        ui.add(
                            egui::Slider::new(&mut glass_light, 0.0..=0.5)
                                .text("Window glow")
                                .step_by(0.01),
                        );
                        ui.add(
                            egui::Slider::new(&mut indoor_glow, 0.0..=1.0)
                                .text("Indoor glow")
                                .step_by(0.01),
                        );
                        ui.add(
                            egui::Slider::new(&mut bleed, 0.0..=2.0)
                                .text("Light bleed")
                                .step_by(0.01),
                        );
                        ui.separator();
                        ui.label("Foliage Shadows");
                        ui.add(
                            egui::Slider::new(&mut foliage_opacity, 0.0..=1.0)
                                .text("Canopy density")
                                .step_by(0.01),
                        );
                        ui.add(
                            egui::Slider::new(&mut foliage_variation, 0.0..=1.0)
                                .text("Tree variation")
                                .step_by(0.01),
                        );
                    });

                // Fluid menu
                egui::containers::menu::MenuButton::new("Fluid")
                    .config(keep_open.clone())
                    .ui(ui, |ui| {
                        let mut fluid_spd = self.fluid_speed;
                        ui.add(
                            egui::Slider::new(&mut fluid_spd, 0.0..=5.0)
                                .text("Fluid speed")
                                .step_by(0.1),
                        );
                        self.fluid_speed = fluid_spd;
                        ui.horizontal(|ui| {
                            ui.label("Wind:");
                            let mut wx = self.fluid_params.wind_x;
                            let mut wy = self.fluid_params.wind_y;
                            ui.add(
                                egui::Slider::new(&mut wx, -20.0..=20.0)
                                    .text("X")
                                    .step_by(0.5),
                            );
                            ui.add(
                                egui::Slider::new(&mut wy, -20.0..=20.0)
                                    .text("Y")
                                    .step_by(0.5),
                            );
                            self.fluid_params.wind_x = wx;
                            self.fluid_params.wind_y = wy;
                        });
                        let mut sr = self.fluid_params.smoke_rate;
                        ui.add(
                            egui::Slider::new(&mut sr, 0.0..=1.0)
                                .text("Smoke rate")
                                .step_by(0.05),
                        );
                        self.fluid_params.smoke_rate = sr;
                        let mut fs = self.fluid_params.fan_speed;
                        ui.add(
                            egui::Slider::new(&mut fs, 0.0..=50.0)
                                .text("Fan speed")
                                .step_by(1.0),
                        );
                        self.fluid_params.fan_speed = fs;
                        ui.add(
                            egui::Slider::new(&mut self.pipe_width, 1.0..=20.0)
                                .text("Pipe width")
                                .step_by(0.5),
                        );
                    });

                // Sound menu
                egui::containers::menu::MenuButton::new("Sound")
                    .config(keep_open.clone())
                    .ui(ui, |ui| {
                        ui.checkbox(&mut self.sound_enabled, "Enabled");
                        if self.sound_enabled {
                            ui.add(
                                egui::Slider::new(&mut self.sound_speed, 0.1..=2.0)
                                    .text("Wave speed")
                                    .step_by(0.05),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.sound_damping, 0.0..=0.05)
                                    .text("Damping")
                                    .step_by(0.001),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.sound_coupling, 0.0..=1.0)
                                    .text("Gas coupling")
                                    .step_by(0.01),
                            );
                            let mut iters = self.sound_iters_per_frame as i32;
                            ui.add(egui::Slider::new(&mut iters, 2..=16).text("Iterations"));
                            self.sound_iters_per_frame = iters as u32;
                            if !self.sound_sources.is_empty() {
                                ui.separator();
                                if ui.button("Clear all sounds").clicked() {
                                    self.sound_sources.clear();
                                }
                            }
                        }
                    });

                // Camera menu
                egui::containers::menu::MenuButton::new("Camera")
                    .config(keep_open.clone())
                    .ui(ui, |ui| {
                        let zoom_pct = zoom / base_zoom * 100.0;
                        ui.label(format!("Zoom: {:.0}%", zoom_pct));
                        ui.add(
                            egui::Slider::new(&mut zoom, base_zoom * 0.2..=base_zoom * 8.0)
                                .text("Zoom")
                                .show_value(false)
                                .logarithmic(true),
                        );
                        if ui.button("Reset zoom").clicked() {
                            zoom = base_zoom;
                        }
                        ui.separator();
                        ui.add(
                            egui::Slider::new(&mut oblique, 0.0..=0.3)
                                .text("Wall face tilt")
                                .step_by(0.005),
                        );
                        ui.separator();
                        ui.add(
                            egui::Slider::new(&mut self.camera_pan_speed, 100.0..=1000.0)
                                .text("Pan speed")
                                .step_by(25.0),
                        );
                    });

                // Admin menu (colonist placement)
                ui.separator();
                // Render menu — performance vs quality controls
                egui::containers::menu::MenuButton::new("Render")
                    .config(keep_open.clone())
                    .ui(ui, |ui| {
                        ui.set_min_width(220.0);

                        // --- Resolution ---
                        ui.label(egui::RichText::new("Resolution").strong().size(11.0));
                        let mut rs = self.render_scale;
                        ui.add(
                            egui::Slider::new(&mut rs, 0.15..=1.0)
                                .text("Render scale")
                                .step_by(0.05),
                        );
                        self.render_scale = rs;

                        ui.add(
                            egui::Slider::new(&mut self.camera.pleb_scale, 0.5..=3.0)
                                .text("Pleb size")
                                .step_by(0.1),
                        );

                        ui.separator();

                        // --- Shadows ---
                        ui.label(egui::RichText::new("Shadows").strong().size(11.0));
                        ui.label(
                            egui::RichText::new(format!(
                                "Intensity: {:.0}%",
                                self.camera.shadow_intensity * 100.0
                            ))
                            .size(10.0)
                            .weak(),
                        );

                        ui.separator();

                        // --- Raytrace ---
                        ui.label(egui::RichText::new("Raytrace").strong().size(11.0));
                        if ui
                            .selectable_label(self.enable_terrain_detail, "Terrain Detail")
                            .clicked()
                        {
                            self.enable_terrain_detail = !self.enable_terrain_detail;
                        }
                        ui.add(
                            egui::Slider::new(&mut self.terrain_ao_strength, 0.0..=5.0)
                                .text("Terrain AO")
                                .step_by(0.1),
                        );
                        if ui
                            .selectable_label(self.show_contours, "Contour Lines")
                            .clicked()
                        {
                            self.show_contours = !self.show_contours;
                        }
                        if self.show_contours {
                            ui.add(
                                egui::Slider::new(&mut self.contour_opacity, 0.1..=2.0)
                                    .text("Intensity")
                                    .step_by(0.05),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.contour_interval, 0.25..=5.0)
                                    .text("Spacing")
                                    .step_by(0.25),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.contour_major_mul, 2.0..=10.0)
                                    .text("Major Every")
                                    .step_by(1.0),
                            );
                        }
                        if ui
                            .selectable_label(self.enable_prox_glow, "Proximity Glow")
                            .clicked()
                        {
                            self.enable_prox_glow = !self.enable_prox_glow;
                        }
                        if ui
                            .selectable_label(self.enable_dir_bleed, "Light Bleed")
                            .clicked()
                        {
                            self.enable_dir_bleed = !self.enable_dir_bleed;
                        }
                        if ui
                            .selectable_label(self.enable_temporal, "Temporal AA")
                            .clicked()
                        {
                            self.enable_temporal = !self.enable_temporal;
                            self.camera.force_refresh = 10.0;
                        }

                        ui.separator();

                        // --- Lightmap ---
                        ui.label(egui::RichText::new("Lightmap").strong().size(11.0));
                        let mut lm_int = self.lightmap_interval as i32;
                        ui.add(egui::Slider::new(&mut lm_int, 1..=10).text("Update interval"));
                        self.lightmap_interval = lm_int as u32;
                        let mut lm_iter = self.lightmap_iterations as i32;
                        ui.add(egui::Slider::new(&mut lm_iter, 4..=40).text("Propagation steps"));
                        self.lightmap_iterations = lm_iter as u32;
                        // Keep odd to ensure correct ping-pong final output
                        if self.lightmap_iterations.is_multiple_of(2) {
                            self.lightmap_iterations += 1;
                        }

                        ui.separator();

                        // --- Fluid Sim ---
                        ui.label(egui::RichText::new("Fluid Sim").strong().size(11.0));
                        if ui
                            .selectable_label(self.hires_fluid, "HiRes (512x512)")
                            .clicked()
                        {
                            self.hires_fluid = !self.hires_fluid;
                        }
                        let mut fp = self.fluid_pressure_iters as i32;
                        ui.add(egui::Slider::new(&mut fp, 5..=50).text("Pressure iterations"));
                        self.fluid_pressure_iters = fp as u32;
                        // Keep odd for correct ping-pong output
                        if self.fluid_pressure_iters.is_multiple_of(2) {
                            self.fluid_pressure_iters += 1;
                        }
                    });

                if ui.button("Schedule").clicked() {
                    self.show_schedule = !self.show_schedule;
                }
                if ui.button("Priorities").clicked() {
                    self.show_priorities = !self.show_priorities;
                }
                // Debug menu
                egui::containers::menu::MenuButton::new("Debug")
                    .config(keep_open)
                    .ui(ui, |ui| {
                        if ui
                            .selectable_label(self.enable_ricochets, "Bullet Ricochets")
                            .clicked()
                        {
                            self.enable_ricochets = !self.enable_ricochets;
                        }
                        ui.separator();
                        if ui
                            .selectable_label(self.sandbox_mode, "Sandbox Mode")
                            .clicked()
                        {
                            self.sandbox_mode = !self.sandbox_mode;
                            if !self.sandbox_mode {
                                self.sandbox_tool = SandboxTool::None;
                                self.build_category = None;
                            }
                        }
                        ui.separator();
                        if ui
                            .selectable_label(self.debug_creatures_always, "Creatures (always)")
                            .clicked()
                        {
                            self.debug_creatures_always = !self.debug_creatures_always;
                            if !self.debug_creatures_always {
                                // Clear existing creatures when turning off
                                self.creatures.clear();
                            }
                        }
                        if ui
                            .selectable_label(self.debug_bullet_slowmo, "Bullet Slow-Mo")
                            .clicked()
                        {
                            self.debug_bullet_slowmo = !self.debug_bullet_slowmo;
                        }
                        if self.debug_bullet_slowmo {
                            ui.add(
                                egui::Slider::new(&mut self.debug_bullet_speed, 0.01..=1.0)
                                    .text("Speed")
                                    .logarithmic(true),
                            );
                        }
                        if ui
                            .selectable_label(self.debug_show_cover, "Show Cover Positions")
                            .clicked()
                        {
                            self.debug_show_cover = !self.debug_show_cover;
                        }
                        if ui
                            .selectable_label(self.debug_show_flock, "Show Flock Links")
                            .clicked()
                        {
                            self.debug_show_flock = !self.debug_show_flock;
                        }
                        ui.separator();
                        if ui
                            .selectable_label(self.fog.enabled, "Fog of War")
                            .clicked()
                        {
                            self.fog.enabled = !self.fog.enabled;
                            self.fog.dirty = true;
                            if !self.fog.enabled {
                                self.fog.texture_data.iter_mut().for_each(|v| *v = 255);
                                self.fog.dirty = true;
                            }
                        }
                        if self.fog.enabled
                            && ui
                                .selectable_label(self.fog.start_explored, "Pre-revealed Map")
                                .clicked()
                        {
                            self.fog.start_explored = !self.fog.start_explored;
                            if self.fog.start_explored {
                                self.fog.explored.iter_mut().for_each(|v| *v = 255);
                            }
                            self.fog.prev_tiles.clear();
                            self.fog.dirty = true;
                        }
                        ui.separator();
                        if ui.button("Stone Lab").clicked() {
                            self.show_stone_lab = !self.show_stone_lab;
                            self.stone_lab.gpu.enabled =
                                if self.show_stone_lab { 1.0 } else { 0.0 };
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Test Audio Beep").clicked() {
                            if self.audio_output.is_none() {
                                self.audio_output = audio::AudioOutput::new();
                            }
                            if let Some(ref audio) = self.audio_output {
                                audio.test_beep();
                            } else {
                                log::warn!("No audio output available");
                            }
                        }
                    });

                ui.separator();
                ui.menu_button("Creatures", |ui| {
                    // Colonist placement
                    let pleb_label = format!("Colonist ({}/{})", self.plebs.len(), MAX_PLEBS);
                    if ui.button(pleb_label).clicked() {
                        self.placing_pleb = !self.placing_pleb;
                        self.placing_enemy = false;
                        self.placing_creature = None;
                        if self.placing_pleb {
                            self.build_tool = BuildTool::None;
                        }
                        ui.close();
                    }

                    // Redskull enemy
                    if ui.button("Redskull Enemy").clicked() {
                        self.placing_enemy = true;
                        self.placing_pleb = false;
                        self.placing_creature = None;
                        self.build_tool = BuildTool::None;
                        ui.close();
                    }

                    ui.separator();

                    // All creature types from registry
                    let reg = creature_defs::CreatureRegistry::cached();
                    let defs: Vec<(u8, String)> =
                        reg.all().map(|(id, def)| (id, def.name.clone())).collect();
                    for (id, name) in defs {
                        let count = self
                            .creatures
                            .iter()
                            .filter(|c| c.species_id == id && !c.is_dead)
                            .count();
                        let label = format!("{name} ({count})");
                        if ui.button(label).clicked() {
                            self.placing_creature = Some(id);
                            self.placing_pleb = false;
                            self.placing_enemy = false;
                            self.build_tool = BuildTool::None;
                            ui.close();
                        }
                    }

                    // Status hint
                    if self.placing_pleb || self.placing_enemy || self.placing_creature.is_some() {
                        ui.separator();
                        ui.label(egui::RichText::new("Click map to place").weak().size(10.0));
                    }
                });
            });
            // Right-aligned FPS/version info (painted over the menu bar)
            let frame_ms = if self.fps_display > 0.0 {
                1000.0 / self.fps_display
            } else {
                0.0
            };
            let rw = self.camera.screen_w as u32;
            let rh = self.camera.screen_h as u32;
            let fps_text = format!(
                "v{} | {:.0} fps ({:.1}ms) | {}x{}",
                include_str!("../VERSION").trim(),
                self.fps_display,
                frame_ms,
                rw,
                rh
            );
            let bar_rect = ui.max_rect();
            let painter = ui.painter();
            painter.text(
                egui::pos2(bar_rect.max.x - 6.0, bar_rect.center().y),
                egui::Align2::RIGHT_CENTER,
                &fps_text,
                egui::FontId::monospace(11.0),
                egui::Color32::from_rgba_premultiplied(160, 160, 160, 180),
            );
        });
        self.time_of_day = time_val;
        self.time_paused = paused;
        self.time_speed = speed;
        self.camera.zoom = zoom;
        self.camera.glass_light_mul = glass_light;
        self.camera.indoor_glow_mul = indoor_glow;
        self.camera.light_bleed_mul = bleed;
        self.camera.foliage_opacity = foliage_opacity;
        self.camera.foliage_variation = foliage_variation;
        self.camera.oblique_strength = oblique;

        if self.show_pleb_help {
            egui::Window::new("Jeff Controls")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("WASD - Move Jeff");
                    ui.label("Q/E - Rotate (when selected)");
                    ui.label("Click Jeff - Select");
                    ui.label("Click ground - Move to (A*)");
                    ui.label("T - Toggle torch (fire)");
                    ui.label("G - Toggle headlamp");
                    ui.label("Escape - Deselect");
                    ui.add_space(5.0);
                    if ui.button("Got it!").clicked() {
                        self.show_pleb_help = false;
                    }
                });
        }
    }

    fn draw_inventory_window(&mut self, ctx: &egui::Context) {
        // --- BG2-style character window: equipment | portrait | stats, backpack below ---
        if self.show_inventory {
            if let Some(sel_idx) = self.selected_pleb {
                if let Some(pleb) = self.plebs.get(sel_idx) {
                    let pleb_name = pleb.name.clone();
                    let health = pleb.needs.health;
                    let hunger = pleb.needs.hunger;
                    let thirst = pleb.needs.thirst;
                    let rest = pleb.needs.rest;
                    let warmth = pleb.needs.warmth;
                    let oxygen = pleb.needs.oxygen;
                    let mood = pleb.needs.mood;
                    let mood_l = mood_label(mood);
                    let stress = pleb.needs.stress;
                    let a = &pleb.appearance;
                    let shirt = [a.shirt_r, a.shirt_g, a.shirt_b];
                    let skin = [a.skin_r, a.skin_g, a.skin_b];
                    let hair = [a.hair_r, a.hair_g, a.hair_b];
                    let pants = [a.pants_r, a.pants_g, a.pants_b];
                    let hair_style = a.hair_style;
                    let skills = pleb.skills.clone();
                    let backstory_name = pleb.backstory_name.clone();
                    let trait_name = pleb.trait_name.clone();
                    let is_enemy = pleb.is_enemy;
                    let p_bleeding = pleb.bleeding;
                    let p_suppression = pleb.suppression;
                    let p_crouching = pleb.crouching;
                    let p_drafted = pleb.drafted;
                    let p_leader = pleb.is_leader;
                    let p_rank = pleb.rank();
                    let p_fights = pleb.firefights_survived;
                    let p_kills = pleb.kills;
                    let p_headlight = pleb.headlight_mode;
                    let p_torch = pleb.torch_on;
                    let equipped_weapon_id = pleb.equipped_weapon;
                    let p_belt_capacity = pleb.equipment.belt_capacity;
                    let p_belt_items: Vec<Option<u16>> =
                        pleb.equipment.belt[..pleb.equipment.belt_capacity as usize].to_vec();
                    let p_active_item = pleb.equipment.active_item;
                    let p_nauseous = pleb.nauseous_timer > 0.0;
                    let p_smoke = pleb.smoke_exposure;
                    let p_water = {
                        let px = pleb.x.floor() as i32;
                        let py = pleb.y.floor() as i32;
                        if px >= 0 && py >= 0 && px < GRID_W as i32 && py < GRID_H as i32 {
                            let wi = (py as u32 * GRID_W + px as u32) as usize;
                            if wi < self.water_depth_cpu.len() {
                                self.water_depth_cpu[wi]
                            } else {
                                0.0
                            }
                        } else {
                            0.0
                        }
                    };

                    let p_event_log = pleb.event_log.clone();
                    let _p_activity = format!("{:?}", pleb.activity.inner());

                    let mut open = self.show_inventory;
                    egui::Window::new(pleb_name)
                        .open(&mut open)
                        .collapsible(false)
                        .resizable(false)
                        .default_pos(egui::pos2(400.0, 150.0))
                        .show(ctx, |ui| {
                            let item_reg = item_defs::ItemRegistry::cached();

                            // --- Styled bar helper ---
                            let bar_w = 100.0f32;
                            let bar_h = 7.0f32;
                            let bar =
                                |ui: &mut egui::Ui, label: &str, val: f32, color: egui::Color32| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(label)
                                                .size(9.0)
                                                .monospace()
                                                .color(egui::Color32::from_gray(160)),
                                        );
                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::Vec2::new(bar_w, bar_h),
                                            egui::Sense::hover(),
                                        );
                                        let painter = ui.painter_at(rect);
                                        painter.rect_filled(
                                            rect,
                                            3.0,
                                            egui::Color32::from_rgb(22, 24, 28),
                                        );
                                        let fill_w = bar_w * val.clamp(0.0, 1.0);
                                        if fill_w > 0.5 {
                                            painter.rect_filled(
                                                egui::Rect::from_min_size(
                                                    rect.min,
                                                    egui::Vec2::new(fill_w, bar_h),
                                                ),
                                                3.0,
                                                color,
                                            );
                                            let hi_rect = egui::Rect::from_min_size(
                                                rect.min,
                                                egui::Vec2::new(fill_w, bar_h * 0.4),
                                            );
                                            painter.rect_filled(
                                                hi_rect,
                                                3.0,
                                                egui::Color32::from_rgba_unmultiplied(
                                                    255, 255, 255, 18,
                                                ),
                                            );
                                        }
                                    });
                                };

                            // Equipment slot helper
                            let equip_slot = |ui: &mut egui::Ui,
                                              label: &str,
                                              icon: &str,
                                              name: &str,
                                              filled: bool| {
                                let slot_w = 100.0;
                                let slot_h = 36.0;
                                let (rect, resp) = ui.allocate_exact_size(
                                    egui::Vec2::new(slot_w, slot_h),
                                    egui::Sense::hover(),
                                );
                                let painter = ui.painter_at(rect);
                                let bg = if filled {
                                    egui::Color32::from_rgb(38, 42, 52)
                                } else {
                                    egui::Color32::from_rgb(28, 30, 36)
                                };
                                painter.rect_filled(rect, 4.0, bg);
                                painter.rect_stroke(
                                    rect,
                                    4.0,
                                    egui::Stroke::new(1.0, egui::Color32::from_gray(50)),
                                    egui::StrokeKind::Outside,
                                );
                                // Slot label top-left
                                painter.text(
                                    rect.min + egui::Vec2::new(4.0, 3.0),
                                    egui::Align2::LEFT_TOP,
                                    label,
                                    egui::FontId::proportional(8.0),
                                    egui::Color32::from_gray(100),
                                );
                                if filled {
                                    // Icon
                                    painter.text(
                                        egui::pos2(rect.min.x + 14.0, rect.center().y + 4.0),
                                        egui::Align2::CENTER_CENTER,
                                        icon,
                                        egui::FontId::proportional(16.0),
                                        egui::Color32::WHITE,
                                    );
                                    // Item name
                                    painter.text(
                                        egui::pos2(rect.min.x + 28.0, rect.center().y + 4.0),
                                        egui::Align2::LEFT_CENTER,
                                        name,
                                        egui::FontId::proportional(9.0),
                                        egui::Color32::from_gray(200),
                                    );
                                } else {
                                    painter.text(
                                        egui::pos2(rect.center().x, rect.center().y + 4.0),
                                        egui::Align2::CENTER_CENTER,
                                        "—",
                                        egui::FontId::proportional(11.0),
                                        egui::Color32::from_gray(60),
                                    );
                                }
                                resp
                            };

                            // ═══ TOP THREE PANES ═══
                            ui.horizontal(|ui| {
                                // ═══ LEFT PANE: Equipment Slots ═══
                                ui.vertical(|ui| {
                                    ui.set_min_width(108.0);
                                    ui.label(
                                        egui::RichText::new("Equipment")
                                            .size(9.0)
                                            .strong()
                                            .color(egui::Color32::from_gray(180)),
                                    );
                                    ui.add_space(2.0);

                                    // Belt slots
                                    if p_belt_capacity > 0 {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "Belt ({}/{})",
                                                p_belt_items.iter().filter(|s| s.is_some()).count(),
                                                p_belt_capacity
                                            ))
                                            .size(8.0)
                                            .color(egui::Color32::from_gray(130)),
                                        );
                                        for (i, slot) in p_belt_items.iter().enumerate() {
                                            let (icon, name, filled) = if let Some(id) = slot {
                                                if let Some(def) = item_reg.get(*id) {
                                                    (
                                                        def.icon.as_str().to_string(),
                                                        def.name.clone(),
                                                        true,
                                                    )
                                                } else {
                                                    ("?".to_string(), "Unknown".to_string(), true)
                                                }
                                            } else {
                                                (String::new(), String::new(), false)
                                            };
                                            let is_active = slot.is_some() && *slot == p_active_item;
                                            let label = if is_active {
                                                format!(">{}<", i + 1)
                                            } else {
                                                format!(" {} ", i + 1)
                                            };
                                            equip_slot(ui, &label, &icon, &name, filled);
                                            ui.add_space(1.0);
                                        }
                                    } else {
                                        ui.label(
                                            egui::RichText::new("No belt")
                                                .size(9.0)
                                                .color(egui::Color32::from_gray(100)),
                                        );
                                        ui.label(
                                            egui::RichText::new("Pockets only")
                                                .size(8.0)
                                                .color(egui::Color32::from_gray(80)),
                                        );
                                    }
                                    ui.add_space(2.0);
                                    // Light slot
                                    let (light_icon, light_name, has_light) =
                                        if p_headlight > 0 {
                                            let mode_name = match p_headlight {
                                                1 => "Wide",
                                                2 => "Normal",
                                                _ => "Focused",
                                            };
                                            (
                                                "\u{1f526}".to_string(),
                                                format!("Headlight ({})", mode_name),
                                                true,
                                            )
                                        } else if p_torch {
                                            ("\u{1f525}".to_string(), "Torch".to_string(), true)
                                        } else {
                                            (String::new(), String::new(), false)
                                        };
                                    equip_slot(ui, "Light", &light_icon, &light_name, has_light);

                                    // Status tags below equipment
                                    ui.add_space(6.0);
                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing.x = 3.0;
                                        let tag =
                                            |ui: &mut egui::Ui, text: &str, col: egui::Color32| {
                                                let tw =
                                                    text.len() as f32 * 5.5 + 6.0;
                                                let (rect, _) = ui.allocate_exact_size(
                                                    egui::Vec2::new(tw, 14.0),
                                                    egui::Sense::hover(),
                                                );
                                                let painter = ui.painter_at(rect);
                                                painter.rect_filled(
                                                    rect,
                                                    3.0,
                                                    col.linear_multiply(0.15),
                                                );
                                                painter.text(
                                                    rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    text,
                                                    egui::FontId::proportional(8.0),
                                                    col,
                                                );
                                            };
                                        tag(
                                            ui,
                                            p_rank.label(),
                                            egui::Color32::from_rgb(160, 160, 180),
                                        );
                                        if p_drafted {
                                            tag(ui, "Drafted", egui::Color32::from_rgb(220, 90, 60));
                                        }
                                        if p_leader {
                                            tag(
                                                ui,
                                                "Leader",
                                                egui::Color32::from_rgb(220, 200, 60),
                                            );
                                        }
                                        if p_crouching {
                                            tag(
                                                ui,
                                                "Crouched",
                                                egui::Color32::from_rgb(140, 180, 120),
                                            );
                                        }
                                        if p_bleeding > 0.1 {
                                            tag(
                                                ui,
                                                "Bleeding",
                                                egui::Color32::from_rgb(200, 50, 50),
                                            );
                                        }
                                        if p_suppression > 0.3 {
                                            tag(
                                                ui,
                                                "Suppressed",
                                                egui::Color32::from_rgb(200, 140, 60),
                                            );
                                        }
                                        if stress > 85.0 {
                                            tag(
                                                ui,
                                                "Breaking!",
                                                egui::Color32::from_rgb(240, 50, 50),
                                            );
                                        } else if stress > 70.0 {
                                            tag(
                                                ui,
                                                "Stressed",
                                                egui::Color32::from_rgb(220, 160, 40),
                                            );
                                        }
                                        if p_water > 0.3 {
                                            tag(
                                                ui,
                                                "Deep water",
                                                egui::Color32::from_rgb(80, 140, 220),
                                            );
                                        } else if p_water > 0.05 {
                                            tag(
                                                ui,
                                                "In water",
                                                egui::Color32::from_rgb(100, 160, 210),
                                            );
                                        }
                                        if health < 0.25 {
                                            tag(
                                                ui,
                                                "Wounded",
                                                egui::Color32::from_rgb(220, 60, 50),
                                            );
                                        }
                                        if p_fights > 0 {
                                            tag(
                                                ui,
                                                &format!("{} fights", p_fights),
                                                egui::Color32::from_rgb(140, 140, 150),
                                            );
                                        }
                                        if p_kills > 0 {
                                            tag(
                                                ui,
                                                &format!("{} kills", p_kills),
                                                egui::Color32::from_rgb(140, 140, 150),
                                            );
                                        }
                                    });
                                });

                                ui.separator();

                                // ═══ CENTER PANE: Large Pleb Portrait ═══
                                ui.vertical(|ui| {
                                    ui.set_min_width(130.0);
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::Vec2::new(130.0, 180.0),
                                        egui::Sense::hover(),
                                    );
                                    let painter = ui.painter_at(rect);
                                    // Dark background
                                    painter.rect_filled(
                                        rect,
                                        6.0,
                                        egui::Color32::from_rgb(18, 20, 26),
                                    );
                                    // Subtle border
                                    painter.rect_stroke(
                                        rect,
                                        6.0,
                                        egui::Stroke::new(1.0, egui::Color32::from_gray(40)),
                                        egui::StrokeKind::Outside,
                                    );

                                    let cx = rect.center().x;
                                    let base_y = rect.center().y + 30.0;

                                    let shirt_c = egui::Color32::from_rgb(
                                        (shirt[0] * 255.0) as u8,
                                        (shirt[1] * 255.0) as u8,
                                        (shirt[2] * 255.0) as u8,
                                    );
                                    let skin_c = egui::Color32::from_rgb(
                                        (skin[0] * 255.0) as u8,
                                        (skin[1] * 255.0) as u8,
                                        (skin[2] * 255.0) as u8,
                                    );
                                    let hair_c = egui::Color32::from_rgb(
                                        (hair[0] * 255.0) as u8,
                                        (hair[1] * 255.0) as u8,
                                        (hair[2] * 255.0) as u8,
                                    );
                                    let pants_c = egui::Color32::from_rgb(
                                        (pants[0] * 255.0) as u8,
                                        (pants[1] * 255.0) as u8,
                                        (pants[2] * 255.0) as u8,
                                    );

                                    // Legs (two rectangles)
                                    let leg_w = 8.0;
                                    let leg_h = 30.0;
                                    painter.rect_filled(
                                        egui::Rect::from_center_size(
                                            egui::pos2(cx - 7.0, base_y + 2.0),
                                            egui::Vec2::new(leg_w, leg_h),
                                        ),
                                        2.0,
                                        pants_c,
                                    );
                                    painter.rect_filled(
                                        egui::Rect::from_center_size(
                                            egui::pos2(cx + 7.0, base_y + 2.0),
                                            egui::Vec2::new(leg_w, leg_h),
                                        ),
                                        2.0,
                                        pants_c,
                                    );

                                    // Torso (shirt)
                                    painter.rect_filled(
                                        egui::Rect::from_center_size(
                                            egui::pos2(cx, base_y - 28.0),
                                            egui::Vec2::new(28.0, 32.0),
                                        ),
                                        4.0,
                                        shirt_c,
                                    );

                                    // Arms
                                    painter.rect_filled(
                                        egui::Rect::from_center_size(
                                            egui::pos2(cx - 19.0, base_y - 24.0),
                                            egui::Vec2::new(7.0, 26.0),
                                        ),
                                        2.0,
                                        shirt_c,
                                    );
                                    painter.rect_filled(
                                        egui::Rect::from_center_size(
                                            egui::pos2(cx + 19.0, base_y - 24.0),
                                            egui::Vec2::new(7.0, 26.0),
                                        ),
                                        2.0,
                                        shirt_c,
                                    );

                                    // Hands (skin circles)
                                    painter.circle_filled(
                                        egui::pos2(cx - 19.0, base_y - 9.0),
                                        4.0,
                                        skin_c,
                                    );
                                    painter.circle_filled(
                                        egui::pos2(cx + 19.0, base_y - 9.0),
                                        4.0,
                                        skin_c,
                                    );

                                    // Head (skin circle)
                                    painter.circle_filled(
                                        egui::pos2(cx, base_y - 54.0),
                                        14.0,
                                        skin_c,
                                    );

                                    // Hair
                                    match hair_style {
                                        0 => {} // bald
                                        1 => {
                                            // short
                                            painter.circle_filled(
                                                egui::pos2(cx, base_y - 60.0),
                                                10.0,
                                                hair_c,
                                            );
                                        }
                                        2 => {
                                            // medium
                                            painter.circle_filled(
                                                egui::pos2(cx, base_y - 60.0),
                                                12.0,
                                                hair_c,
                                            );
                                            painter.rect_filled(
                                                egui::Rect::from_center_size(
                                                    egui::pos2(cx, base_y - 48.0),
                                                    egui::Vec2::new(20.0, 8.0),
                                                ),
                                                2.0,
                                                hair_c,
                                            );
                                        }
                                        _ => {
                                            // long
                                            painter.circle_filled(
                                                egui::pos2(cx, base_y - 62.0),
                                                13.0,
                                                hair_c,
                                            );
                                            painter.rect_filled(
                                                egui::Rect::from_center_size(
                                                    egui::pos2(cx, base_y - 44.0),
                                                    egui::Vec2::new(22.0, 18.0),
                                                ),
                                                3.0,
                                                hair_c,
                                            );
                                        }
                                    }

                                    // Eyes (two small dots)
                                    let eye_col = egui::Color32::from_rgb(30, 30, 30);
                                    painter.circle_filled(
                                        egui::pos2(cx - 5.0, base_y - 54.0),
                                        2.0,
                                        eye_col,
                                    );
                                    painter.circle_filled(
                                        egui::pos2(cx + 5.0, base_y - 54.0),
                                        2.0,
                                        eye_col,
                                    );

                                    // Mood text below portrait
                                    let mood_col = if mood > 20.0 {
                                        egui::Color32::from_rgb(100, 200, 100)
                                    } else if mood > -20.0 {
                                        egui::Color32::from_rgb(180, 180, 120)
                                    } else {
                                        egui::Color32::from_rgb(200, 80, 80)
                                    };
                                    painter.text(
                                        egui::pos2(cx, rect.max.y - 8.0),
                                        egui::Align2::CENTER_BOTTOM,
                                        mood_l,
                                        egui::FontId::proportional(10.0),
                                        mood_col,
                                    );

                                    // Backstory + trait below portrait area
                                    if !is_enemy && !backstory_name.is_empty() {
                                        let bs_label = if let Some(ref tn) = trait_name {
                                            format!("{} — {}", backstory_name, tn)
                                        } else {
                                            backstory_name.clone()
                                        };
                                        ui.label(
                                            egui::RichText::new(&bs_label)
                                                .size(9.0)
                                                .color(egui::Color32::from_rgb(160, 170, 190)),
                                        );
                                    }
                                });

                                ui.separator();

                                // ═══ RIGHT PANE: Vitals + Skills ═══
                                ui.vertical(|ui| {
                                    ui.set_min_width(140.0);

                                    // --- Vitals frame ---
                                    egui::Frame::new()
                                        .inner_margin(egui::Margin::same(4))
                                        .stroke(egui::Stroke::new(
                                            1.0,
                                            egui::Color32::from_gray(40),
                                        ))
                                        .corner_radius(4.0)
                                        .show(ui, |ui| {
                                            ui.label(
                                                egui::RichText::new("Vitals")
                                                    .size(9.0)
                                                    .strong()
                                                    .color(egui::Color32::from_gray(180)),
                                            );
                                            bar(
                                                ui,
                                                "HP ",
                                                health,
                                                egui::Color32::from_rgb(190, 60, 60),
                                            );
                                            bar(
                                                ui,
                                                "FOD",
                                                hunger,
                                                egui::Color32::from_rgb(190, 150, 40),
                                            );
                                            bar(
                                                ui,
                                                "H2O",
                                                thirst,
                                                egui::Color32::from_rgb(50, 130, 210),
                                            );
                                            bar(
                                                ui,
                                                "RST",
                                                rest,
                                                egui::Color32::from_rgb(70, 110, 190),
                                            );
                                            bar(
                                                ui,
                                                "WRM",
                                                warmth,
                                                egui::Color32::from_rgb(190, 95, 35),
                                            );
                                            bar(
                                                ui,
                                                "O2 ",
                                                oxygen,
                                                egui::Color32::from_rgb(90, 190, 210),
                                            );
                                            let stress_norm =
                                                (stress / 100.0).clamp(0.0, 1.0);
                                            let stress_col = if stress_norm < 0.5 {
                                                egui::Color32::from_rgb(70, 170, 70)
                                            } else if stress_norm < 0.7 {
                                                egui::Color32::from_rgb(190, 170, 50)
                                            } else {
                                                egui::Color32::from_rgb(190, 55, 55)
                                            };
                                            bar(ui, "STR", stress_norm, stress_col);
                                        });

                                    ui.add_space(4.0);

                                    // --- Skills frame ---
                                    if !is_enemy {
                                        egui::Frame::new()
                                            .inner_margin(egui::Margin::same(4))
                                            .stroke(egui::Stroke::new(
                                                1.0,
                                                egui::Color32::from_gray(40),
                                            ))
                                            .corner_radius(4.0)
                                            .show(ui, |ui| {
                                                ui.label(
                                                    egui::RichText::new("Skills")
                                                        .size(9.0)
                                                        .strong()
                                                        .color(egui::Color32::from_gray(180)),
                                                );
                                                let skill_labels =
                                                    ["SHT", "MEL", "CRF", "FRM", "MED", "BLD"];
                                                let skill_names = [
                                                    "Shooting",
                                                    "Melee",
                                                    "Crafting",
                                                    "Farming",
                                                    "Medical",
                                                    "Construction",
                                                ];
                                                let skill_colors = [
                                                    egui::Color32::from_rgb(200, 120, 80),
                                                    egui::Color32::from_rgb(200, 80, 80),
                                                    egui::Color32::from_rgb(140, 180, 100),
                                                    egui::Color32::from_rgb(100, 170, 60),
                                                    egui::Color32::from_rgb(120, 160, 220),
                                                    egui::Color32::from_rgb(180, 150, 100),
                                                ];
                                                for (i, s) in skills.iter().enumerate() {
                                                    let resp = ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(skill_labels[i])
                                                                .size(9.0)
                                                                .monospace()
                                                                .color(
                                                                    egui::Color32::from_gray(160),
                                                                ),
                                                        );
                                                        let (rect, resp) =
                                                            ui.allocate_exact_size(
                                                                egui::Vec2::new(bar_w, bar_h),
                                                                egui::Sense::hover(),
                                                            );
                                                        let painter = ui.painter_at(rect);
                                                        painter.rect_filled(
                                                            rect,
                                                            3.0,
                                                            egui::Color32::from_rgb(22, 24, 28),
                                                        );
                                                        let fill =
                                                            (s.value / 10.0).clamp(0.0, 1.0);
                                                        let fill_w = bar_w * fill;
                                                        if fill_w > 0.5 {
                                                            painter.rect_filled(
                                                                egui::Rect::from_min_size(
                                                                    rect.min,
                                                                    egui::Vec2::new(
                                                                        fill_w, bar_h,
                                                                    ),
                                                                ),
                                                                3.0,
                                                                skill_colors[i],
                                                            );
                                                            painter.rect_filled(
                                                                egui::Rect::from_min_size(
                                                                    rect.min,
                                                                    egui::Vec2::new(
                                                                        fill_w,
                                                                        bar_h * 0.4,
                                                                    ),
                                                                ),
                                                                3.0,
                                                                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 18),
                                                            );
                                                        }
                                                        painter.text(
                                                            egui::pos2(
                                                                rect.min.x + fill_w - 1.0,
                                                                rect.center().y,
                                                            ),
                                                            egui::Align2::RIGHT_CENTER,
                                                            format!("{:.1}", s.value),
                                                            egui::FontId::proportional(7.0),
                                                            egui::Color32::from_rgba_unmultiplied(
                                                                255, 255, 255, 180,
                                                            ),
                                                        );
                                                        resp
                                                    });
                                                    let apt_text = if s.aptitude_known {
                                                        match s.aptitude {
                                                            -2 => " (Limited)",
                                                            -1 => " (Below avg)",
                                                            0 => "",
                                                            1 => " (Talented)",
                                                            2 => " (Gifted)",
                                                            _ => " (Genius)",
                                                        }
                                                    } else {
                                                        " (?)"
                                                    };
                                                    resp.inner.on_hover_text(format!(
                                                        "{}: {:.1} — {}{}",
                                                        skill_names[i],
                                                        s.value,
                                                        s.descriptor(),
                                                        apt_text,
                                                    ));
                                                }
                                            });
                                    }
                                });
                            });

                            // ═══ BOTTOM: Tabbed section ═══
                            ui.separator();
                            // Tab bar
                            ui.horizontal(|ui| {
                                let tab_btn = |ui: &mut egui::Ui, label: &str, idx: u8, current: u8| -> bool {
                                    let selected = current == idx;
                                    let color = if selected {
                                        egui::Color32::from_gray(200)
                                    } else {
                                        egui::Color32::from_gray(120)
                                    };
                                    let resp = ui.selectable_label(selected,
                                        egui::RichText::new(label).size(9.0).color(color));
                                    resp.clicked()
                                };
                                if tab_btn(ui, "Gear", 0, self.charsheet_tab) { self.charsheet_tab = 0; }
                                if tab_btn(ui, "Log", 1, self.charsheet_tab) { self.charsheet_tab = 1; }
                                if tab_btn(ui, "Modifiers", 2, self.charsheet_tab) { self.charsheet_tab = 2; }
                            });

                            if self.charsheet_tab == 0 {
                            // --- Gear tab: backpack grid ---

                            let slot_size = 40.0;
                            let cols = 8usize;
                            let rows = 2usize;
                            let total_slots = cols * rows;
                            let selected = self.inv_selected_slot;

                            let stacks: Vec<Option<item_defs::ItemStack>> = (0..total_slots)
                                .map(|i| {
                                    if let Some(sel) = self.selected_pleb {
                                        self.plebs
                                            .get(sel)
                                            .and_then(|p| p.inventory.stacks.get(i).cloned())
                                    } else {
                                        None
                                    }
                                })
                                .collect();

                            let mut clicked_slot: Option<usize> = None;

                            for row in 0..rows {
                                ui.horizontal(|ui| {
                                    for col in 0..cols {
                                        let slot_idx = row * cols + col;
                                        let is_selected = selected == Some(slot_idx);
                                        let (rect, response) = ui.allocate_exact_size(
                                            egui::Vec2::splat(slot_size),
                                            egui::Sense::click(),
                                        );
                                        let painter = ui.painter_at(rect);
                                        let bg = if is_selected {
                                            egui::Color32::from_rgb(55, 75, 105)
                                        } else if response.hovered() {
                                            egui::Color32::from_rgb(48, 52, 60)
                                        } else {
                                            egui::Color32::from_rgb(35, 38, 44)
                                        };
                                        painter.rect_filled(rect, 4.0, bg);
                                        painter.rect_stroke(
                                            rect,
                                            4.0,
                                            egui::Stroke::new(
                                                if is_selected { 2.0 } else { 1.0 },
                                                if is_selected {
                                                    egui::Color32::from_rgb(120, 160, 220)
                                                } else {
                                                    egui::Color32::from_gray(55)
                                                },
                                            ),
                                            egui::StrokeKind::Outside,
                                        );

                                        if let Some(stack) = &stacks[slot_idx] {
                                            // Check if this item is equipped
                                            let is_eq = equipped_weapon_id == Some(stack.item_id);
                                            if is_eq {
                                                painter.rect_stroke(
                                                    rect,
                                                    4.0,
                                                    egui::Stroke::new(
                                                        2.0,
                                                        egui::Color32::from_rgb(200, 170, 60),
                                                    ),
                                                    egui::StrokeKind::Inside,
                                                );
                                            }
                                            let def = item_reg.get(stack.item_id);
                                            let icon =
                                                def.map(|d| d.icon.as_str()).unwrap_or("?");
                                            let cat = def
                                                .map(|d| d.category.as_str())
                                                .unwrap_or("");
                                            let stripe_col = match cat {
                                                "tool" => {
                                                    Some(egui::Color32::from_rgb(70, 120, 70))
                                                }
                                                "container" => {
                                                    Some(egui::Color32::from_rgb(50, 110, 170))
                                                }
                                                "food" => {
                                                    Some(egui::Color32::from_rgb(170, 120, 40))
                                                }
                                                _ => None,
                                            };
                                            if let Some(sc) = stripe_col {
                                                painter.rect_filled(
                                                    egui::Rect::from_min_size(
                                                        rect.min,
                                                        egui::Vec2::new(rect.width(), 3.0),
                                                    ),
                                                    0.0,
                                                    sc,
                                                );
                                            }
                                            painter.text(
                                                rect.center() + egui::Vec2::new(0.0, -2.0),
                                                egui::Align2::CENTER_CENTER,
                                                icon,
                                                egui::FontId::proportional(16.0),
                                                egui::Color32::WHITE,
                                            );
                                            if is_eq {
                                                painter.text(
                                                    rect.left_top()
                                                        + egui::Vec2::new(3.0, 3.0),
                                                    egui::Align2::LEFT_TOP,
                                                    "E",
                                                    egui::FontId::proportional(8.0),
                                                    egui::Color32::from_rgb(200, 170, 60),
                                                );
                                            }
                                            if stack.count > 1 {
                                                painter.text(
                                                    rect.right_bottom()
                                                        + egui::Vec2::new(-4.0, -2.0),
                                                    egui::Align2::RIGHT_BOTTOM,
                                                    format!("{}", stack.count),
                                                    egui::FontId::proportional(10.0),
                                                    egui::Color32::from_gray(200),
                                                );
                                            }
                                            if let Some((_, amt)) = stack.liquid {
                                                let cap = stack.liquid_capacity();
                                                if cap > 0 {
                                                    let fill = amt as f32 / cap as f32;
                                                    let br = egui::Rect::from_min_size(
                                                        egui::pos2(
                                                            rect.min.x + 2.0,
                                                            rect.max.y - 5.0,
                                                        ),
                                                        egui::vec2(
                                                            (rect.width() - 4.0) * fill,
                                                            3.0,
                                                        ),
                                                    );
                                                    painter.rect_filled(
                                                        br,
                                                        1.0,
                                                        egui::Color32::from_rgb(60, 140, 220),
                                                    );
                                                }
                                            }
                                            response.clone().on_hover_text(stack.label());
                                        }
                                        if response.clicked() {
                                            clicked_slot = Some(slot_idx);
                                        }
                                    }
                                });
                            }

                            // Handle slot clicks (swap/select)
                            if let Some(clicked) = clicked_slot {
                                if let Some(prev) = self.inv_selected_slot {
                                    if prev == clicked {
                                        self.inv_selected_slot = None;
                                    } else {
                                        if let Some(sel_idx) = self.selected_pleb
                                            && let Some(pleb) = self.plebs.get_mut(sel_idx)
                                        {
                                            while pleb.inventory.stacks.len()
                                                <= prev.max(clicked)
                                            {
                                                pleb.inventory
                                                    .stacks
                                                    .push(item_defs::ItemStack::new(0, 0));
                                            }
                                            pleb.inventory.stacks.swap(prev, clicked);
                                            pleb.inventory.stacks.retain(|s| s.count > 0);
                                        }
                                        self.inv_selected_slot = None;
                                    }
                                } else if stacks[clicked].is_some() {
                                    self.inv_selected_slot = Some(clicked);
                                }
                            }

                            // Action buttons
                            ui.horizontal(|ui| {
                                let has_sel = selected.is_some()
                                    && selected
                                        .map(|s| {
                                            stacks.get(s).and_then(|x| x.as_ref()).is_some()
                                        })
                                        .unwrap_or(false);
                                if ui
                                    .add_enabled(
                                        has_sel,
                                        egui::Button::new(
                                            egui::RichText::new("\u{2b07} Drop").size(10.0),
                                        ),
                                    )
                                    .clicked()
                                    && let Some(slot) = selected
                                {
                                    if let Some(sel_idx) = self.selected_pleb
                                        && let Some(pleb) = self.plebs.get_mut(sel_idx)
                                        && slot < pleb.inventory.stacks.len()
                                    {
                                        let stack = pleb.inventory.stacks.remove(slot);
                                        self.ground_items.push(resources::GroundItem {
                                            x: pleb.x,
                                            y: pleb.y,
                                            stack,
                                        });
                                    }
                                    self.inv_selected_slot = None;
                                }
                                let is_food = selected
                                    .and_then(|s| stacks.get(s))
                                    .and_then(|s| s.as_ref())
                                    .map(|s| item_reg.nutrition(s.item_id) > 0.0)
                                    .unwrap_or(false);
                                if ui
                                    .add_enabled(
                                        is_food,
                                        egui::Button::new(
                                            egui::RichText::new("\u{1f374} Eat").size(10.0),
                                        ),
                                    )
                                    .clicked()
                                {
                                    if let Some(sel_idx) = self.selected_pleb
                                        && let Some(pleb) = self.plebs.get_mut(sel_idx)
                                        && let Some(slot) = selected
                                        && slot < pleb.inventory.stacks.len()
                                    {
                                        let nutr = item_reg
                                            .nutrition(pleb.inventory.stacks[slot].item_id);
                                        pleb.needs.hunger =
                                            (pleb.needs.hunger + nutr).min(1.0);
                                        pleb.inventory.stacks[slot].count -= 1;
                                        if pleb.inventory.stacks[slot].count == 0 {
                                            pleb.inventory.stacks.remove(slot);
                                        }
                                    }
                                    self.inv_selected_slot = None;
                                }
                            });

                            } else if self.charsheet_tab == 1 {
                            // --- Log tab: per-pleb event history ---
                            egui::ScrollArea::vertical()
                                .max_height(160.0)
                                .auto_shrink(false)
                                .show(ui, |ui| {
                                    if p_event_log.is_empty() {
                                        ui.label(
                                            egui::RichText::new("No events yet.")
                                                .size(9.0)
                                                .color(egui::Color32::from_gray(100)),
                                        );
                                    } else {
                                        for (time, msg) in p_event_log.iter().rev() {
                                            ui.horizontal(|ui| {
                                                let h = (time / DAY_DURATION * 24.0) % 24.0;
                                                let hh = h as u32;
                                                let mm = ((h - hh as f32) * 60.0) as u32;
                                                ui.label(
                                                    egui::RichText::new(format!("{:02}:{:02}", hh, mm))
                                                        .size(8.0)
                                                        .monospace()
                                                        .color(egui::Color32::from_gray(100)),
                                                );
                                                ui.label(
                                                    egui::RichText::new(msg)
                                                        .size(9.0)
                                                        .color(egui::Color32::from_gray(190)),
                                                );
                                            });
                                        }
                                    }
                                });

                            } else if self.charsheet_tab == 2 {
                            // --- Modifiers tab: derived buffs/debuffs ---
                            egui::ScrollArea::vertical()
                                .max_height(160.0)
                                .auto_shrink(false)
                                .show(ui, |ui| {
                                    // Each modifier: (name, description, color)
                                    let mut mods: Vec<(&str, String, egui::Color32)> = Vec::new();

                                    // Needs-based
                                    if hunger > 0.8 {
                                        mods.push(("Well Fed", "Hunger satisfied".into(),
                                            egui::Color32::from_rgb(80, 180, 80)));
                                    } else if hunger < 0.20 {
                                        mods.push(("Hungry", format!("Hunger: {:.0}%", hunger * 100.0),
                                            egui::Color32::from_rgb(200, 140, 40)));
                                    }
                                    if hunger < 0.08 {
                                        mods.push(("Starving", "Health damage from starvation".into(),
                                            egui::Color32::from_rgb(220, 50, 50)));
                                    }
                                    if thirst > 0.8 {
                                        mods.push(("Hydrated", "Thirst satisfied".into(),
                                            egui::Color32::from_rgb(60, 150, 200)));
                                    } else if thirst < 0.20 {
                                        mods.push(("Thirsty", format!("Thirst: {:.0}%", thirst * 100.0),
                                            egui::Color32::from_rgb(40, 120, 200)));
                                    }
                                    if rest > 0.8 {
                                        mods.push(("Rested", "Well rested".into(),
                                            egui::Color32::from_rgb(100, 140, 200)));
                                    } else if rest < 0.20 {
                                        mods.push(("Exhausted", format!("Rest: {:.0}%", rest * 100.0),
                                            egui::Color32::from_rgb(140, 100, 180)));
                                    }
                                    if warmth < 0.25 {
                                        mods.push(("Cold", format!("Warmth: {:.0}%", warmth * 100.0),
                                            egui::Color32::from_rgb(80, 140, 200)));
                                    }
                                    if oxygen < 0.5 {
                                        mods.push(("Low Oxygen", format!("O2: {:.0}%", oxygen * 100.0),
                                            egui::Color32::from_rgb(200, 80, 80)));
                                    }

                                    // Health-based
                                    if health < 0.25 {
                                        mods.push(("Wounded", format!("Health: {:.0}%", health * 100.0),
                                            egui::Color32::from_rgb(200, 60, 50)));
                                    }
                                    if p_bleeding > 0.1 {
                                        mods.push(("Bleeding", format!("Rate: {:.0}%", p_bleeding * 100.0),
                                            egui::Color32::from_rgb(200, 40, 40)));
                                    }
                                    if p_nauseous {
                                        mods.push(("Nauseous", "Ate raw food — nutrition wasted".into(),
                                            egui::Color32::from_rgb(140, 160, 50)));
                                    }
                                    if p_smoke > 0.3 {
                                        mods.push(("Choking", format!("Smoke: {:.0}% — mood penalty, work slowed", p_smoke * 100.0),
                                            egui::Color32::from_rgb(140, 130, 100)));
                                    } else if p_smoke > 0.1 {
                                        mods.push(("Smoky", format!("Smoke: {:.0}% — eyes watering", p_smoke * 100.0),
                                            egui::Color32::from_rgb(160, 150, 120)));
                                    }

                                    // Mental
                                    let stress_norm = stress / 100.0;
                                    if stress_norm > 0.85 {
                                        mods.push(("Breaking!", "Near mental collapse".into(),
                                            egui::Color32::from_rgb(220, 40, 40)));
                                    } else if stress_norm > 0.7 {
                                        mods.push(("Stressed", format!("Stress: {:.0}%", stress),
                                            egui::Color32::from_rgb(200, 160, 40)));
                                    } else if stress_norm < 0.2 {
                                        mods.push(("Calm", "Low stress".into(),
                                            egui::Color32::from_rgb(80, 170, 80)));
                                    }
                                    if mood > 30.0 {
                                        mods.push(("Happy", format!("Mood: {:.0}", mood),
                                            egui::Color32::from_rgb(80, 180, 100)));
                                    } else if mood < -30.0 {
                                        mods.push(("Unhappy", format!("Mood: {:.0}", mood),
                                            egui::Color32::from_rgb(180, 80, 60)));
                                    }

                                    // Combat/status
                                    if p_drafted {
                                        mods.push(("Drafted", "Under direct command".into(),
                                            egui::Color32::from_rgb(200, 100, 60)));
                                    }
                                    if p_crouching {
                                        mods.push(("Crouched", "Reduced profile, slower movement".into(),
                                            egui::Color32::from_rgb(120, 160, 100)));
                                    }
                                    if p_suppression > 0.3 {
                                        mods.push(("Suppressed", format!("{:.0}% accuracy penalty", p_suppression * 100.0),
                                            egui::Color32::from_rgb(200, 140, 50)));
                                    }
                                    if p_leader {
                                        mods.push(("Leader", "Morale aura for nearby allies".into(),
                                            egui::Color32::from_rgb(220, 200, 60)));
                                    }
                                    if p_water > 0.3 {
                                        mods.push(("Deep Water", "Movement severely slowed".into(),
                                            egui::Color32::from_rgb(60, 120, 200)));
                                    } else if p_water > 0.05 {
                                        mods.push(("Wading", "Movement slowed".into(),
                                            egui::Color32::from_rgb(80, 140, 190)));
                                    }

                                    // Trait (permanent buff)
                                    if let Some(ref tn) = trait_name {
                                        let desc = crate::types::PlebTrait::ALL
                                            .iter()
                                            .find(|t| t.name() == tn.as_str())
                                            .map(|t| t.description())
                                            .unwrap_or("");
                                        mods.push((
                                            tn.as_str(),
                                            desc.to_string(),
                                            egui::Color32::from_rgb(180, 160, 60),
                                        ));
                                    }

                                    // Rank
                                    let rank_label = p_rank.label();
                                    if rank_label != "Green" {
                                        mods.push((rank_label, format!("{} fights, {} kills", p_fights, p_kills),
                                            egui::Color32::from_rgb(160, 160, 180)));
                                    }

                                    // Skill speed modifiers (show if notably high or low)
                                    for &(idx, name) in &[(2usize, "Crafting"), (3, "Farming"), (5, "Construction")] {
                                        if let Some(s) = skills.get(idx) {
                                            let mult = s.speed_mult();
                                            let desc = s.descriptor();
                                            if s.value >= 6.5 {
                                                mods.push((name, format!("{} ({:.1}) — {:.0}% speed", desc, s.value, mult * 100.0),
                                                egui::Color32::from_rgb(80, 170, 80)));
                                            } else if s.value <= 3.0 {
                                                mods.push((name, format!("{} ({:.1}) — {:.0}% speed", desc, s.value, mult * 100.0),
                                                    egui::Color32::from_rgb(180, 100, 60)));
                                            }
                                        }
                                    }

                                    if mods.is_empty() {
                                        ui.label(
                                            egui::RichText::new("No active modifiers.")
                                                .size(9.0)
                                                .color(egui::Color32::from_gray(100)),
                                        );
                                    } else {
                                        for (name, desc, color) in &mods {
                                            ui.horizontal(|ui| {
                                                // Colored dot
                                                let (dot_rect, _) = ui.allocate_exact_size(
                                                    egui::Vec2::new(6.0, 12.0),
                                                    egui::Sense::hover(),
                                                );
                                                ui.painter_at(dot_rect).circle_filled(
                                                    dot_rect.center(),
                                                    3.0,
                                                    *color,
                                                );
                                                ui.label(
                                                    egui::RichText::new(*name)
                                                        .size(9.0)
                                                        .strong()
                                                        .color(*color),
                                                );
                                                ui.label(
                                                    egui::RichText::new(desc.as_str())
                                                        .size(8.0)
                                                        .color(egui::Color32::from_gray(140)),
                                                );
                                            });
                                        }
                                    }
                                });
                            }

                        });
                    if !open {
                        self.show_inventory = false;
                    }
                } else {
                    self.show_inventory = false;
                }
            } else {
                self.show_inventory = false;
            }
        }
    }

    fn draw_colonist_bar(&mut self, ctx: &egui::Context) {
        // --- Colonist bar (top center, like Rimworld) ---
        if !self.plebs.is_empty() {
            // Collect pleb data for display (avoid borrow issues)
            struct PlebDisplay {
                idx: usize,
                name: String,
                shirt: [f32; 3],
                skin: [f32; 3],
                hair: [f32; 3],
                health: f32,
                hunger: f32,
                thirst: f32,
                rest: f32,
                warmth: f32,
                oxygen: f32,
                mood: f32,
                mood_label: &'static str,
                breath_pct: f32, // 0-1, breath remaining
                breathing_label: &'static str,
                breathing_state: BreathingState,
                air_o2: f32,
                air_co2: f32,
                activity: String,
                inventory_label: String,
                is_crisis: bool,
                crisis_reason: Option<&'static str>,
                shift_label: &'static str,
                group_id: Option<u8>,
            }
            let pleb_display: Vec<PlebDisplay> = self
                .plebs
                .iter()
                .enumerate()
                .filter(|(_, p)| !p.is_enemy)
                .map(|(i, p)| {
                    let a = &p.appearance;
                    PlebDisplay {
                        idx: i,
                        name: p.name.clone(),
                        shirt: [a.shirt_r, a.shirt_g, a.shirt_b],
                        skin: [a.skin_r, a.skin_g, a.skin_b],
                        hair: [a.hair_r, a.hair_g, a.hair_b],
                        health: p.needs.health,
                        hunger: p.needs.hunger,
                        thirst: p.needs.thirst,
                        rest: p.needs.rest,
                        warmth: p.needs.warmth,
                        oxygen: p.needs.oxygen,
                        mood: p.needs.mood,
                        mood_label: mood_label(p.needs.mood),
                        breath_pct: p.needs.breath_remaining / 30.0,
                        breathing_label: breathing_label(&p.needs.breathing_state),
                        breathing_state: p.needs.breathing_state.clone(),
                        air_o2: p.needs.air_o2,
                        air_co2: p.needs.air_co2,
                        activity: {
                            let inner = p.activity.inner();
                            let act_str = match inner {
                                PlebActivity::Idle => "Idle".to_string(),
                                PlebActivity::Walking => {
                                    if p.work_target.is_some() {
                                        "Walking to task".to_string()
                                    } else {
                                        "Walking".to_string()
                                    }
                                }
                                PlebActivity::Sleeping => "Sleeping".to_string(),
                                PlebActivity::Harvesting(pr) => {
                                    format!("Harvesting {:.0}%", pr * 100.0)
                                }
                                PlebActivity::Eating => "Eating".to_string(),
                                PlebActivity::Hauling => {
                                    if p.haul_target.is_some() {
                                        "Hauling to crate".to_string()
                                    } else {
                                        "Hauling".to_string()
                                    }
                                }
                                PlebActivity::Farming(pr) => {
                                    // Determine if planting or harvesting from work target block type
                                    let action = if let Some((tx, ty)) = p.work_target {
                                        let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                                        if tidx < self.grid_data.len() {
                                            let tbt = self.grid_data[tidx] & 0xFF;
                                            if tbt == BT_CROP || tbt == BT_BERRY_BUSH {
                                                "Harvesting"
                                            } else if tbt == BT_TREE {
                                                "Chopping"
                                            } else {
                                                "Planting"
                                            }
                                        } else {
                                            "Farming"
                                        }
                                    } else {
                                        "Farming"
                                    };
                                    format!("{} {:.0}%", action, pr * 100.0)
                                }
                                PlebActivity::Building(pr) => {
                                    format!("Building {:.0}%", pr * 100.0)
                                }
                                PlebActivity::Crafting(rid, pr) => {
                                    let rname = recipe_defs::RecipeRegistry::cached()
                                        .get(*rid)
                                        .map(|r| r.name.as_str())
                                        .unwrap_or("item");
                                    format!("Crafting {} {:.0}%", rname, pr * 100.0)
                                }
                                PlebActivity::Drinking(pr) => {
                                    format!("Drinking {:.0}%", pr * 100.0)
                                }
                                PlebActivity::MentalBreak(k, _) => {
                                    let kind = match k {
                                        MentalBreakKind::Daze => "Daze",
                                        MentalBreakKind::Binge => "Binge eating",
                                        MentalBreakKind::Tantrum => "Tantrum",
                                        MentalBreakKind::Collapse => "Collapsed",
                                    };
                                    format!("Mental break: {}", kind)
                                }
                                PlebActivity::Digging => {
                                    // Compute dig progress from elevation
                                    if let Some((tx, ty)) = p.work_target {
                                        let cur = crate::terrain::sample_elevation(
                                            &self.sub_elevation,
                                            tx as f32 + 0.5,
                                            ty as f32 + 0.5,
                                        );
                                        let base = self
                                            .dig_zones
                                            .first()
                                            .and_then(|dz| {
                                                dz.base_elevations.get(&(tx, ty)).copied()
                                            })
                                            .unwrap_or(cur);
                                        let target = self
                                            .dig_zones
                                            .first()
                                            .map(|dz| dz.target_depth)
                                            .unwrap_or(0.8);
                                        let progress = if target > 0.01 {
                                            ((base - cur) / target).clamp(0.0, 1.0)
                                        } else {
                                            0.0
                                        };
                                        format!("Digging {:.0}%", progress * 100.0)
                                    } else {
                                        "Digging".to_string()
                                    }
                                }
                                PlebActivity::Filling => {
                                    if let Some((tx, ty)) = p.work_target {
                                        let cur = crate::terrain::sample_elevation(
                                            &self.sub_elevation,
                                            tx as f32 + 0.5,
                                            ty as f32 + 0.5,
                                        );
                                        let target_h = self
                                            .berm_zones
                                            .first()
                                            .map(|bz| bz.target_height)
                                            .unwrap_or(cur + 0.5);
                                        let progress = if target_h > cur {
                                            0.0 // just starting
                                        } else {
                                            1.0
                                        };
                                        format!("Berm {:.0}%", progress * 100.0)
                                    } else {
                                        "Building berm".to_string()
                                    }
                                }
                                PlebActivity::Butchering(pr) => {
                                    format!("Butchering {:.0}%", pr * 100.0)
                                }
                                PlebActivity::Cooking(pr) => {
                                    format!("Cooking {:.0}%", pr * 100.0)
                                }
                                PlebActivity::Fishing(pr) => {
                                    format!("Fishing {:.0}%", pr * 100.0)
                                }
                                PlebActivity::Mining(pr) => {
                                    format!("Mining {:.0}%", pr * 100.0)
                                }
                                PlebActivity::Staggering(_) => "Staggering!".to_string(),
                                PlebActivity::Crisis(_, _) => "Crisis".to_string(),
                            };
                            if let Some(reason) = p.activity.crisis_reason() {
                                format!("{} ({})", act_str, reason)
                            } else {
                                act_str
                            }
                        },
                        inventory_label: p.inventory.carrying_label(),
                        is_crisis: p.activity.is_crisis(),
                        crisis_reason: p.activity.crisis_reason(),
                        shift_label: p.schedule.preset.label(),
                        group_id: p.group_id,
                    }
                })
                .collect();

            egui::Area::new(egui::Id::new("colonist_bar"))
                .anchor(egui::Align2::CENTER_TOP, [0.0, 10.0])
                .show(ctx, |ui| {
                    egui::Frame::window(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for pd in &pleb_display {
                                let is_sel = self.selected_pleb == Some(pd.idx);
                                let card_w = 48.0;
                                let card_h = 56.0;
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::new(card_w, card_h),
                                    egui::Sense::click(),
                                );
                                let painter = ui.painter_at(rect);

                                // Background (red tint during crisis)
                                let bg = if pd.is_crisis {
                                    egui::Color32::from_rgb(100, 40, 40)
                                } else if is_sel {
                                    egui::Color32::from_rgb(60, 100, 60)
                                } else {
                                    egui::Color32::from_rgb(50, 55, 65)
                                };
                                painter.rect_filled(rect, 4.0, bg);

                                // Crisis border: thin red frame
                                if pd.is_crisis {
                                    let inset = rect.shrink(1.5);
                                    painter.rect_filled(
                                        rect,
                                        4.0,
                                        egui::Color32::from_rgb(180, 30, 30),
                                    );
                                    painter.rect_filled(inset, 3.0, bg);
                                }

                                // Selection border
                                if is_sel {
                                    painter.rect_stroke(
                                        rect,
                                        4.0,
                                        egui::Stroke::new(
                                            1.5,
                                            egui::Color32::from_rgb(100, 200, 100),
                                        ),
                                        egui::StrokeKind::Outside,
                                    );
                                }

                                // Portrait area
                                let portrait_center = rect.center() + egui::Vec2::new(0.0, -4.0);

                                // Body (shirt color)
                                let shirt_c = egui::Color32::from_rgb(
                                    (pd.shirt[0] * 255.0) as u8,
                                    (pd.shirt[1] * 255.0) as u8,
                                    (pd.shirt[2] * 255.0) as u8,
                                );
                                painter.circle_filled(
                                    portrait_center + egui::Vec2::new(0.0, 8.0),
                                    10.0,
                                    shirt_c,
                                );

                                // Head (skin color)
                                let skin_c = egui::Color32::from_rgb(
                                    (pd.skin[0] * 255.0) as u8,
                                    (pd.skin[1] * 255.0) as u8,
                                    (pd.skin[2] * 255.0) as u8,
                                );
                                painter.circle_filled(
                                    portrait_center + egui::Vec2::new(0.0, -2.0),
                                    6.0,
                                    skin_c,
                                );

                                // Hair
                                let hair_c = egui::Color32::from_rgb(
                                    (pd.hair[0] * 255.0) as u8,
                                    (pd.hair[1] * 255.0) as u8,
                                    (pd.hair[2] * 255.0) as u8,
                                );
                                painter.circle_filled(
                                    portrait_center + egui::Vec2::new(0.0, -6.0),
                                    4.0,
                                    hair_c,
                                );

                                // Name
                                let name_pos = rect.center_bottom() + egui::Vec2::new(0.0, -2.0);
                                painter.text(
                                    name_pos,
                                    egui::Align2::CENTER_BOTTOM,
                                    &pd.name,
                                    egui::FontId::proportional(8.0),
                                    egui::Color32::WHITE,
                                );

                                // Health bar
                                let bar_y = rect.max.y - 5.0;
                                let bar_x = rect.min.x + 2.0;
                                let bar_w = rect.width() - 4.0;
                                let bar_rect = egui::Rect::from_min_size(
                                    egui::Pos2::new(bar_x, bar_y),
                                    egui::Vec2::new(bar_w, 2.0),
                                );
                                painter.rect_filled(
                                    bar_rect,
                                    1.0,
                                    egui::Color32::from_rgb(40, 40, 40),
                                );
                                let health_color = if pd.health > 0.5 {
                                    egui::Color32::from_rgb(80, 200, 80)
                                } else if pd.health > 0.25 {
                                    egui::Color32::from_rgb(200, 200, 40)
                                } else {
                                    egui::Color32::from_rgb(200, 40, 40)
                                };
                                painter.rect_filled(
                                    egui::Rect::from_min_size(
                                        bar_rect.min,
                                        egui::Vec2::new(bar_w * pd.health, 2.0),
                                    ),
                                    1.0,
                                    health_color,
                                );

                                // Group number badge (top-left corner)
                                if let Some(gid) = pd.group_id {
                                    let badge_pos = rect.left_top() + egui::Vec2::new(3.0, 2.0);
                                    painter.text(
                                        badge_pos,
                                        egui::Align2::LEFT_TOP,
                                        &format!("{}", gid),
                                        egui::FontId::proportional(8.0),
                                        egui::Color32::from_rgb(120, 200, 220),
                                    );
                                }

                                if response.clicked() {
                                    if is_sel {
                                        // Click again on selected pleb → toggle inventory
                                        self.show_inventory = !self.show_inventory;
                                    } else {
                                        self.selected_pleb = Some(pd.idx);
                                    }
                                }
                                if response.secondary_clicked() {
                                    self.selected_pleb = if is_sel { None } else { Some(pd.idx) };
                                    self.show_inventory = false;
                                }
                            }
                        });
                    });
                });
        }

        // Schedule window (all plebs)
        if self.show_schedule {
            let mut open = true;
            egui::Window::new("Schedule")
                .open(&mut open)
                .default_pos(egui::pos2(200.0, 200.0))
                .resizable(false)
                .show(ctx, |ui| {
                    let bar_w = 240.0;
                    let cell_w = bar_w / 24.0;
                    let bar_h = 14.0;
                    // Hour labels header
                    ui.horizontal(|ui| {
                        ui.add_space(120.0); // name + dropdown width
                        let (header_rect, _) = ui.allocate_exact_size(
                            egui::Vec2::new(bar_w, 12.0),
                            egui::Sense::hover(),
                        );
                        let hp = ui.painter_at(header_rect);
                        for h in 0..24 {
                            let x = header_rect.min.x + (h as f32 + 0.5) * cell_w;
                            if h % 3 == 0 {
                                hp.text(
                                    egui::pos2(x, header_rect.center().y),
                                    egui::Align2::CENTER_CENTER,
                                    format!("{:02}", h),
                                    egui::FontId::proportional(7.0),
                                    egui::Color32::from_gray(160),
                                );
                            }
                        }
                    });
                    ui.separator();
                    // Per-pleb rows
                    let friendly: Vec<usize> = (0..self.plebs.len())
                        .filter(|&i| !self.plebs[i].is_enemy)
                        .collect();
                    for &pi in &friendly {
                        ui.horizontal(|ui| {
                            // Name
                            let name = self.plebs[pi].name.clone();
                            ui.label(egui::RichText::new(&name).size(10.0).strong());
                            // Shift dropdown
                            let preset = self.plebs[pi].schedule.preset;
                            egui::ComboBox::from_id_salt(format!("shift_{}", pi))
                                .selected_text(preset.label())
                                .width(55.0)
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_label(preset == PlebShift::Day, "Day")
                                        .clicked()
                                    {
                                        self.plebs[pi].schedule.apply_preset(PlebShift::Day);
                                    }
                                    if ui
                                        .selectable_label(preset == PlebShift::Night, "Night")
                                        .clicked()
                                    {
                                        self.plebs[pi].schedule.apply_preset(PlebShift::Night);
                                    }
                                });
                            // 24-hour bar with clickable cells
                            let (bar_rect, bar_response) = ui.allocate_exact_size(
                                egui::Vec2::new(bar_w, bar_h),
                                egui::Sense::click(),
                            );
                            let bp = ui.painter_at(bar_rect);
                            bp.rect_filled(bar_rect, 2.0, egui::Color32::from_rgb(25, 25, 25));
                            for h in 0..24usize {
                                let x0 = bar_rect.min.x + h as f32 * cell_w;
                                let x1 = x0 + cell_w;
                                let is_work = self.plebs[pi].schedule.hours[h];
                                let col = if is_work {
                                    egui::Color32::from_rgb(55, 115, 55) // green = work
                                } else {
                                    egui::Color32::from_rgb(35, 45, 95) // blue = sleep
                                };
                                bp.rect_filled(
                                    egui::Rect::from_min_max(
                                        egui::pos2(x0 + 0.5, bar_rect.min.y + 0.5),
                                        egui::pos2(x1 - 0.5, bar_rect.max.y - 0.5),
                                    ),
                                    1.0,
                                    col,
                                );
                            }
                            // Click to toggle individual hours
                            if bar_response.clicked()
                                && let Some(pos) = bar_response.interact_pointer_pos()
                            {
                                let h = ((pos.x - bar_rect.min.x) / cell_w) as usize;
                                if h < 24 {
                                    self.plebs[pi].schedule.hours[h] =
                                        !self.plebs[pi].schedule.hours[h];
                                    self.plebs[pi].schedule.preset = PlebShift::Custom;
                                }
                            }
                            // Current time marker
                            let hour_frac = self.time_of_day / DAY_DURATION;
                            let mx = bar_rect.min.x + hour_frac * bar_w;
                            bp.line_segment(
                                [
                                    egui::pos2(mx, bar_rect.min.y),
                                    egui::pos2(mx, bar_rect.max.y),
                                ],
                                egui::Stroke::new(1.5, egui::Color32::WHITE),
                            );
                        });
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Click hours to toggle.")
                                .weak()
                                .size(9.0),
                        );
                        ui.colored_label(egui::Color32::from_rgb(55, 115, 55), "\u{25a0} Work");
                        ui.colored_label(egui::Color32::from_rgb(35, 45, 95), "\u{25a0} Sleep");
                    });
                });
            if !open {
                self.show_schedule = false;
            }
        }

        // Work Priorities window
        if self.show_priorities {
            let mut open = true;
            egui::Window::new("Work Priorities")
                .open(&mut open)
                .default_pos(egui::pos2(400.0, 200.0))
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new("Click to cycle: 1 (high) → 2 → 3 → off → 1")
                            .weak()
                            .size(9.0),
                    );
                    ui.separator();

                    egui::Grid::new("work_prio_grid")
                        .num_columns(1 + zones::WORK_TYPE_COUNT)
                        .spacing([2.0, 2.0])
                        .show(ui, |ui| {
                            // Header row
                            ui.label(egui::RichText::new("Name").size(10.0).strong());
                            for name in &zones::WORK_TYPE_NAMES {
                                ui.label(egui::RichText::new(*name).size(10.0).strong());
                            }
                            ui.end_row();

                            let friendly_indices: Vec<usize> = (0..self.plebs.len())
                                .filter(|&i| !self.plebs[i].is_enemy && !self.plebs[i].is_dead)
                                .collect();
                            for pi in friendly_indices {
                                let name = self.plebs[pi].name.clone();
                                ui.label(egui::RichText::new(&name).size(10.0));
                                for wt in 0..zones::WORK_TYPE_COUNT {
                                    let prio = self.plebs[pi].work_priorities[wt];
                                    let (label, color) = match prio {
                                        1 => ("1", egui::Color32::from_rgb(80, 200, 80)),
                                        2 => ("2", egui::Color32::from_rgb(200, 200, 60)),
                                        3 => ("3", egui::Color32::from_rgb(160, 120, 60)),
                                        _ => ("-", egui::Color32::from_gray(80)),
                                    };
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new(label).size(11.0).color(color),
                                            )
                                            .min_size(egui::Vec2::new(28.0, 18.0)),
                                        )
                                        .clicked()
                                    {
                                        self.plebs[pi].work_priorities[wt] = match prio {
                                            1 => 2,
                                            2 => 3,
                                            3 => 0,
                                            _ => 1,
                                        };
                                    }
                                }
                                ui.end_row();
                            }
                        });
                });
            if !open {
                self.show_priorities = false;
            }
        }
    }

    fn draw_context_menus(
        &mut self,
        ctx: &egui::Context,
        bp_ppp: f32,
        _bp_cam: (f32, f32, f32, f32, f32),
    ) {
        self.draw_context_menu_popup(ctx, bp_ppp);
    }

    fn draw_context_menu_popup(&mut self, ctx: &egui::Context, bp_ppp: f32) {
        let menu = match &self.context_menu {
            Some(m) => m,
            None => return,
        };
        let mut close = false;
        let mut chosen_action: Option<ContextAction> = None;

        egui::Area::new(egui::Id::new("context_menu"))
            .order(egui::Order::Tooltip) // render on top of everything
            .fixed_pos(egui::Pos2::new(
                menu.screen_x / bp_ppp,
                menu.screen_y / bp_ppp,
            ))
            .show(ctx, |ui| {
                let shift_held = self.pressed_keys.contains(&KeyCode::ShiftLeft)
                    || self.pressed_keys.contains(&KeyCode::ShiftRight);
                egui::Frame::menu(ui.style()).show(ui, |ui| {
                    for (label, action, enabled) in &menu.actions {
                        let text = if shift_held {
                            egui::RichText::new(format!("{} [queue]", label)).size(11.0)
                        } else {
                            egui::RichText::new(label.as_str()).size(11.0)
                        };
                        let text = if *enabled { text } else { text.weak() };
                        let btn = ui.add_enabled(*enabled, egui::Button::new(text));
                        if btn.clicked() {
                            chosen_action = Some(action.clone());
                            close = true;
                        }
                    }
                });
            });

        // Execute chosen action (shift = queue, otherwise immediate)
        if let Some(action) = chosen_action {
            let shift_held = self.pressed_keys.contains(&KeyCode::ShiftLeft)
                || self.pressed_keys.contains(&KeyCode::ShiftRight);

            // Convert ContextAction to PlebCommand for queueing
            let as_command = match &action {
                ContextAction::Harvest(hx, hy) => Some(PlebCommand::Harvest(*hx, *hy)),
                ContextAction::Haul(hx, hy) => Some(PlebCommand::Haul(*hx, *hy)),
                ContextAction::MoveTo(wx, wy) => Some(PlebCommand::MoveTo(*wx, *wy)),
                ContextAction::DigClay(dx, dy) => Some(PlebCommand::DigClay(*dx, *dy)),
                ContextAction::HandCraft(rid) => Some(PlebCommand::HandCraft(*rid)),
                ContextAction::GatherBranches(x, y) => Some(PlebCommand::GatherBranches(*x, *y)),
                ContextAction::Eat(idx) => {
                    // Convert item index to grid coords for stable queueing
                    if *idx < self.ground_items.len() {
                        let item = &self.ground_items[*idx];
                        Some(PlebCommand::Eat(
                            item.x.floor() as i32,
                            item.y.floor() as i32,
                        ))
                    } else {
                        None
                    }
                }
                ContextAction::Butcher(bx, by) => Some(PlebCommand::Butcher(*bx, *by)),
                ContextAction::Fish(bx, by) => Some(PlebCommand::Fish(*bx, *by)),
                ContextAction::Mine(bx, by) => Some(PlebCommand::Mine(*bx, *by)),
                ContextAction::OpenSalvageCrate(bx, by) => {
                    Some(PlebCommand::OpenSalvageCrate(*bx, *by))
                }
                ContextAction::FireAt(_)
                | ContextAction::ThrowGrenade(_, _)
                | ContextAction::Hunt(_)
                | ContextAction::Equip(_) => None,
            };

            // If shift held and pleb is busy, queue the command instead of executing
            if shift_held && let Some(cmd) = as_command {
                if let Some(sel_idx) = self.selected_pleb {
                    let pleb = &mut self.plebs[sel_idx];
                    if !pleb.is_enemy && !pleb.activity.is_crisis() {
                        pleb.command_queue.push(cmd);
                    }
                }
                self.context_menu = None;
                return;
            }

            // Immediate execution — clear queue (new direct order replaces queued ones)
            if let Some(sel_idx) = self.selected_pleb {
                self.plebs[sel_idx].command_queue.clear();
            }

            match action {
                ContextAction::Harvest(hx, hy) => {
                    if let Some(sel_idx) = self.selected_pleb
                        && !self.plebs[sel_idx].is_enemy
                        && !self.plebs[sel_idx].activity.is_crisis()
                    {
                        self.send_pleb_to_target(sel_idx, hx, hy);
                    }
                }
                ContextAction::Haul(hx, hy) => {
                    // Use selected pleb if available, otherwise find nearest
                    let mut best_pleb: Option<usize> = self.selected_pleb;
                    if best_pleb.is_none() {
                        let mut best_dist = f32::MAX;
                        for (i, p) in self.plebs.iter().enumerate() {
                            if p.activity.is_crisis()
                                || p.inventory.is_carrying()
                                || p.is_enemy
                                || p.is_dead
                            {
                                continue;
                            }
                            let dist = ((p.x - hx as f32 - 0.5).powi(2)
                                + (p.y - hy as f32 - 0.5).powi(2))
                            .sqrt();
                            if dist < best_dist {
                                best_pleb = Some(i);
                                best_dist = dist;
                            }
                        }
                    }
                    if let Some(pi) = best_pleb {
                        let nearest_crate = find_nearest_crate(&self.grid_data, hx, hy);
                        let pleb = &mut self.plebs[pi];
                        let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                        let adj = adjacent_walkable(&self.grid_data, hx, hy).unwrap_or((hx, hy));
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
                            pleb.activity = PlebActivity::Hauling;
                            pleb.haul_target = nearest_crate; // None is OK — just picks up
                            pleb.harvest_target = Some((hx, hy));
                            self.selected_pleb = Some(pi);
                        }
                    }
                }
                ContextAction::Eat(item_idx) => {
                    if let Some(sel_idx) = self.selected_pleb
                        && item_idx < self.ground_items.len()
                    {
                        let item = &self.ground_items[item_idx];
                        let ix = item.x.floor() as i32;
                        let iy = item.y.floor() as i32;
                        let pleb = &mut self.plebs[sel_idx];
                        if !pleb.is_enemy && !pleb.activity.is_crisis() {
                            let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                            let dist = ((pleb.x - ix as f32 - 0.5).powi(2)
                                + (pleb.y - iy as f32 - 0.5).powi(2))
                            .sqrt();
                            if dist < 1.5 {
                                // Close enough — eat directly
                                pleb.harvest_target = Some((ix, iy));
                                pleb.activity = PlebActivity::Eating;
                                pleb.work_target = None;
                                pleb.haul_target = None;
                                pleb.path.clear();
                            } else {
                                // Walk there first, eat on arrival
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
                    }
                }
                ContextAction::MoveTo(wx, wy) => {
                    // Move all group members (or just the selected pleb)
                    let move_indices: Vec<usize> = if !self.selected_group.is_empty() {
                        self.selected_group.clone()
                    } else if let Some(idx) = self.selected_pleb {
                        vec![idx]
                    } else {
                        vec![]
                    };
                    let offsets = crate::comms::spread_offsets(
                        move_indices.len(),
                        self.combat.flock_spacing.min_spacing(),
                    );
                    for (k, &pi) in move_indices.iter().enumerate() {
                        if let Some(pleb) = self.plebs.get_mut(pi) {
                            let (ox, oy) = offsets[k];
                            let goal = ((wx + ox).floor() as i32, (wy + oy).floor() as i32);
                            let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
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
                                pleb.clear_targets();
                            }
                        }
                    }
                    self.move_marker = Some((wx.floor() + 0.5, wy.floor() + 0.5, 2.0));
                }
                ContextAction::DigClay(dx, dy) => {
                    if let Some(sel_idx) = self.selected_pleb {
                        self.send_pleb_to_target(sel_idx, dx, dy);
                    }
                }
                ContextAction::HandCraft(recipe_id) => {
                    if let Some(sel_idx) = self.selected_pleb {
                        let pleb = &mut self.plebs[sel_idx];
                        if !pleb.is_enemy && !pleb.activity.is_crisis() {
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
                    }
                }
                ContextAction::GatherBranches(gx, gy) => {
                    if let Some(sel_idx) = self.selected_pleb {
                        if self.send_pleb_to_target(sel_idx, gx, gy) {
                            self.plebs[sel_idx].harvest_target = Some((gx, gy));
                        }
                    }
                }
                ContextAction::FireAt(target_idx) => {
                    // Fire at target with all group members (or just selected pleb)
                    let fire_indices: Vec<usize> = if !self.selected_group.is_empty() {
                        self.selected_group.clone()
                    } else if let Some(idx) = self.selected_pleb {
                        vec![idx]
                    } else {
                        vec![]
                    };
                    let target_pos = self.plebs.get(target_idx).map(|e| (e.x, e.y));
                    for &pi in &fire_indices {
                        if let Some(pleb) = self.plebs.get_mut(pi) {
                            if !pleb.is_enemy && !pleb.is_dead {
                                if !pleb.drafted {
                                    pleb.drafted = true;
                                }
                                pleb.prefer_ranged = true;
                                pleb.update_equipped_weapon();
                                pleb.aim_target = Some(target_idx);
                                pleb.aim_progress = 0.0;
                                pleb.swing_progress = 0.0;
                                pleb.path.clear();
                                pleb.path_idx = 0;
                                if let Some((ex, ey)) = target_pos {
                                    pleb.angle = (ey - pleb.y).atan2(ex - pleb.x);
                                }
                                pleb.set_bubble(pleb::BubbleKind::Icon('!', [220, 50, 40]), 1.5);
                            }
                        }
                    }
                }
                ContextAction::ThrowGrenade(tx, ty) => {
                    if let Some(sel_idx) = self.selected_pleb {
                        let pleb = &self.plebs[sel_idx];
                        if !pleb.is_dead {
                            let dx = tx - pleb.x;
                            let dy = ty - pleb.y;
                            let dist = (dx * dx + dy * dy).sqrt().max(0.1);
                            let power = (dist / 18.0).clamp(0.2, 1.0);
                            let spawn_x = pleb.x + dx / dist * 0.5;
                            let spawn_y = pleb.y + dy / dist * 0.5;
                            self.physics_bodies
                                .push(PhysicsBody::new_grenade(spawn_x, spawn_y, dx, dy, power));
                            if self.sound_enabled {
                                self.sound_sources.push(SoundSource {
                                    x: pleb.x,
                                    y: pleb.y,
                                    amplitude: types::db_to_amplitude(60.0),
                                    frequency: 200.0,
                                    phase: 0.0,
                                    pattern: 3,
                                    duration: 0.08,
                                    fresh: true,
                                });
                            }
                            self.move_marker = Some((tx, ty, 1.5));
                        }
                    }
                }
                ContextAction::Hunt(creature_idx) => {
                    // Assign selected pleb(s) to hunt a creature
                    let hunt_indices: Vec<usize> = if !self.selected_group.is_empty() {
                        self.selected_group.clone()
                    } else if let Some(idx) = self.selected_pleb {
                        vec![idx]
                    } else {
                        vec![]
                    };
                    let creature_pos = self.creatures.get(creature_idx).map(|c| (c.x, c.y));
                    for &pi in &hunt_indices {
                        if let Some(pleb) = self.plebs.get_mut(pi) {
                            if !pleb.is_enemy && !pleb.is_dead {
                                pleb.hunt_target = Some(creature_idx);
                                pleb.drafted = true;
                                pleb.prefer_ranged = true;
                                pleb.update_equipped_weapon();
                                pleb.aim_target = None;
                                pleb.aim_progress = 0.0;
                                // Path toward the creature
                                if let Some((cx, cy)) = creature_pos {
                                    let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                                    let goal = (cx.floor() as i32, cy.floor() as i32);
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
                                pleb.set_bubble(
                                    pleb::BubbleKind::Icon('\u{1f3af}', [180, 140, 60]),
                                    2.0,
                                );
                            }
                        }
                    }
                }
                ContextAction::Butcher(bx, by)
                | ContextAction::Fish(bx, by)
                | ContextAction::Mine(bx, by)
                | ContextAction::OpenSalvageCrate(bx, by) => {
                    if let Some(sel_idx) = self.selected_pleb
                        && !self.plebs[sel_idx].is_enemy
                        && !self.plebs[sel_idx].activity.is_crisis()
                    {
                        self.send_pleb_to_target(sel_idx, bx, by);
                    }
                }
                ContextAction::Equip(gi_idx) => {
                    // Pick up ground item and equip it (weapon → belt slot, belt → equip layer)
                    if let Some(sel_idx) = self.selected_pleb
                        && let Some(pleb) = self.plebs.get_mut(sel_idx)
                        && !pleb.is_dead
                    {
                        if gi_idx < self.ground_items.len() {
                            let item = self.ground_items.remove(gi_idx);
                            let item_id = item.stack.item_id;
                            let item_reg = item_defs::ItemRegistry::cached();
                            let def = item_reg.get(item_id);
                            let is_belt = def.is_some_and(|d| d.is_belt);
                            let is_belt_item = def.is_some_and(|d| d.is_belt_item());

                            if is_belt {
                                // Equipping a belt: drop old belt if any, equip new one
                                if pleb.equipment.belt_capacity > 0 {
                                    // Drop old belt + its contents
                                    let old_belt_id = pleb.equipment.belt_item;
                                    if old_belt_id > 0 {
                                        self.ground_items.push(resources::GroundItem::new(
                                            pleb.x,
                                            pleb.y,
                                            old_belt_id,
                                            1,
                                        ));
                                    }
                                    // Drop belt contents back to inventory
                                    for i in 0..pleb.equipment.belt_capacity as usize {
                                        if let Some(id) = pleb.equipment.belt[i].take() {
                                            pleb.inventory.add(id, 1);
                                        }
                                    }
                                    pleb.equipment.belt_capacity = 0;
                                    pleb.equipment.belt_item = 0;
                                    pleb.equipment.active_item = None;
                                    pleb.equipped_weapon = None;
                                }
                                // Equip new belt + auto-migrate tools from inventory
                                pleb.equipment.equip_belt(item_id);
                                pleb.equipment
                                    .auto_migrate_from_inventory(&mut pleb.inventory);
                                pleb.update_equipped_weapon();
                                pleb.log_event(
                                    self.camera.time,
                                    format!(
                                        "Equipped {}",
                                        def.map(|d| d.name.as_str()).unwrap_or("belt")
                                    ),
                                );
                            } else if is_belt_item {
                                // Weapon/tool: add to belt if possible, else inventory
                                if pleb.equipment.belt_capacity > 0
                                    && pleb.equipment.add_to_belt(item_id)
                                {
                                    pleb.update_equipped_weapon();
                                } else {
                                    pleb.inventory.add_stack(item.stack.clone());
                                    pleb.equipped_weapon = Some(item_id);
                                }
                            } else {
                                pleb.inventory.add_stack(item.stack.clone());
                            }
                        }
                    }
                }
            }
        }

        // Close on click outside
        if !close && self.context_menu.is_some() {
            let pointer_over_ui = ctx.is_pointer_over_area();
            let any_click = ctx.input(|i| i.pointer.any_pressed());
            if any_click && !pointer_over_ui {
                close = true;
            }
        }
        if close {
            self.context_menu = None;
        }
    }

    fn draw_overlays_and_popups(
        &mut self,
        ctx: &egui::Context,
        bp_cam: (f32, f32, f32, f32, f32),
        _bp_ppp: f32,
        _dt: f32,
    ) {
        // Wind compass (upper-left, below top menu bar)
        {
            let wx = self.fluid_params.wind_x;
            let wy = self.fluid_params.wind_y;
            let wind_mag = (wx * wx + wy * wy).sqrt();
            egui::Area::new(egui::Id::new("wind_compass"))
                .anchor(egui::Align2::LEFT_TOP, [10.0, 32.0])
                .interactable(false)
                .show(ctx, |ui| {
                    let size = 56.0;
                    let (resp, painter) =
                        ui.allocate_painter(egui::Vec2::splat(size), egui::Sense::hover());
                    let center = resp.rect.center();
                    let radius = size * 0.45;
                    // Circle background
                    painter.circle_filled(
                        center,
                        radius,
                        egui::Color32::from_rgba_unmultiplied(30, 30, 40, 200),
                    );
                    painter.circle_stroke(
                        center,
                        radius,
                        egui::Stroke::new(1.0, egui::Color32::from_gray(100)),
                    );
                    // NSEW labels
                    let label_color = egui::Color32::from_gray(130);
                    let label_font = egui::FontId::proportional(9.0);
                    let label_r = radius + 1.0;
                    painter.text(
                        center + egui::Vec2::new(0.0, -label_r),
                        egui::Align2::CENTER_BOTTOM,
                        "N",
                        label_font.clone(),
                        label_color,
                    );
                    painter.text(
                        center + egui::Vec2::new(0.0, label_r),
                        egui::Align2::CENTER_TOP,
                        "S",
                        label_font.clone(),
                        label_color,
                    );
                    painter.text(
                        center + egui::Vec2::new(label_r, 0.0),
                        egui::Align2::LEFT_CENTER,
                        "E",
                        label_font.clone(),
                        label_color,
                    );
                    painter.text(
                        center + egui::Vec2::new(-label_r, 0.0),
                        egui::Align2::RIGHT_CENTER,
                        "W",
                        label_font,
                        label_color,
                    );
                    // Tick marks at cardinal directions
                    let tick_color = egui::Color32::from_gray(80);
                    for &(dx, dy) in &[(0.0f32, -1.0f32), (0.0, 1.0), (1.0, 0.0), (-1.0, 0.0)] {
                        let inner = center + egui::Vec2::new(dx, dy) * (radius - 4.0);
                        let outer = center + egui::Vec2::new(dx, dy) * (radius - 1.0);
                        painter.line_segment([inner, outer], egui::Stroke::new(1.0, tick_color));
                    }
                    // Wind arrow
                    if wind_mag > 0.1 {
                        let dir_x = wx / wind_mag;
                        let dir_y = wy / wind_mag;
                        let arrow_len = (radius - 6.0) * (wind_mag / 20.0).clamp(0.3, 1.0);
                        let tip = center + egui::Vec2::new(dir_x * arrow_len, dir_y * arrow_len);
                        let tail = center
                            - egui::Vec2::new(dir_x * arrow_len * 0.3, dir_y * arrow_len * 0.3);
                        painter.line_segment(
                            [tail, tip],
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(200, 220, 255)),
                        );
                        // Arrowhead
                        let perp = egui::Vec2::new(-dir_y, dir_x) * arrow_len * 0.25;
                        let head_base = center
                            + egui::Vec2::new(dir_x * arrow_len * 0.55, dir_y * arrow_len * 0.55);
                        painter.add(egui::Shape::convex_polygon(
                            vec![tip, head_base + perp, head_base - perp],
                            egui::Color32::from_rgb(200, 220, 255),
                            egui::Stroke::NONE,
                        ));
                    } else {
                        painter.text(
                            center,
                            egui::Align2::CENTER_CENTER,
                            "·",
                            egui::FontId::proportional(14.0),
                            egui::Color32::from_gray(150),
                        );
                    }
                    // Wind speed label below compass
                    painter.text(
                        resp.rect.center_bottom() + egui::Vec2::new(0.0, 10.0),
                        egui::Align2::CENTER_TOP,
                        format!("{:.0}", wind_mag),
                        egui::FontId::proportional(9.0),
                        egui::Color32::from_gray(150),
                    );
                    // Sun position indicator (yellow dot on compass rim)
                    if self.camera.sun_intensity > 0.01 {
                        let sun_x = self.camera.sun_dir_x;
                        let sun_y = self.camera.sun_dir_y;
                        let sun_len = (sun_x * sun_x + sun_y * sun_y).sqrt().max(0.001);
                        let sun_pos = center
                            + egui::Vec2::new(sun_x / sun_len, sun_y / sun_len) * (radius - 2.0);
                        let sun_bright = (self.camera.sun_intensity * 255.0).min(255.0) as u8;
                        painter.circle_filled(
                            sun_pos,
                            3.5,
                            egui::Color32::from_rgb(255, sun_bright, 30),
                        );
                    }
                });
        }

        self.draw_popups(ctx, bp_cam);
    }

    fn draw_popups(&mut self, ctx: &egui::Context, bp_cam: (f32, f32, f32, f32, f32)) {
        let bp_ppp = self.ppp();

        // Info tool: hold Alt to inspect any block
        let alt_held = self.pressed_keys.contains(&KeyCode::AltLeft)
            || self.pressed_keys.contains(&KeyCode::AltRight);
        if alt_held {
            let (wx, wy) = self.hover_world;
            let bx = wx.floor() as i32;
            let by = wy.floor() as i32;

            let in_bounds = bx >= 0 && by >= 0 && bx < GRID_W as i32 && by < GRID_H as i32;
            let cursor_screen = ctx.input(|i| i.pointer.hover_pos().unwrap_or(egui::Pos2::ZERO));

            egui::Area::new(egui::Id::new("info_tooltip"))
                .fixed_pos(cursor_screen + egui::Vec2::new(15.0, 15.0))
                .interactable(false)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_max_width(260.0);
                        let heading = |ui: &mut egui::Ui, icon: &str, text: &str| {
                            ui.label(
                                egui::RichText::new(format!("{} {}", icon, text))
                                    .strong()
                                    .size(11.0),
                            );
                        };
                        let row = |ui: &mut egui::Ui, label: &str, value: String| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(label).weak().size(10.0));
                                ui.label(egui::RichText::new(value).monospace().size(10.0));
                            });
                        };
                        let row_color =
                            |ui: &mut egui::Ui,
                             label: &str,
                             value: String,
                             color: egui::Color32| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(label).weak().size(10.0));
                                    ui.label(
                                        egui::RichText::new(value)
                                            .monospace()
                                            .size(10.0)
                                            .color(color),
                                    );
                                });
                            };

                        // --- Header: position + block type ---
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("({:.0}, {:.0})", wx, wy))
                                    .monospace()
                                    .size(10.0)
                                    .weak(),
                            );
                            if in_bounds {
                                let idx = (by as u32 * GRID_W + bx as u32) as usize;
                                let block = self.grid_data[idx];
                                let bt = block & 0xFF;
                                let bh = block_height_rs(block) as u32;
                                let flags = (block >> 16) & 0xFF;
                                let reg = block_defs::BlockRegistry::cached();
                                let type_name = reg.name(bt);
                                let mut label = type_name.to_string();
                                if bh > 0 {
                                    label += &format!(" h:{}", bh);
                                }
                                if flags & 2 != 0 {
                                    label += " \u{1f3e0}";
                                } // roofed
                                if flags & 1 != 0 {
                                    label += if flags & 4 != 0 {
                                        " \u{1f6aa}\u{2705}"
                                    } else {
                                        " \u{1f6aa}\u{274c}"
                                    };
                                }
                                ui.label(egui::RichText::new(label).strong().size(11.0));
                            }
                        });

                        if !in_bounds {
                            return;
                        }
                        let idx = (by as u32 * GRID_W + bx as u32) as usize;
                        let block = self.grid_data[idx];
                        let bt = block & 0xFF;
                        let bh = block_height_rs(block) as u32;

                        // --- Elevation + Terrain ---
                        let elev = if idx < self.elevation_data.len() {
                            self.elevation_data[idx]
                        } else {
                            0.0
                        };
                        if elev > 0.05 || idx < self.terrain_data.len() {
                            ui.separator();
                            heading(ui, "\u{26f0}", "Terrain");
                            if elev > 0.05 {
                                row(ui, "Elevation", format!("{:.1}", elev));
                            }
                            if idx < self.terrain_data.len() {
                                let td = self.terrain_data[idx];
                                let tt = terrain_type(td);
                                let tt_name = match tt {
                                    0 => "Grass",
                                    1 => "Chalky",
                                    2 => "Rocky",
                                    3 => "Clay",
                                    4 => "Gravel",
                                    5 => "Peat",
                                    6 => "Marsh",
                                    7 => "Loam",
                                    _ => "?",
                                };
                                let tr = terrain_richness(td);
                                let veg = (td >> 4) & 0x1F;
                                row(ui, "Type", tt_name.to_string());
                                row(ui, "Soil", format!("{}/31", tr));
                                row(ui, "Vegetation", format!("{}/31", veg));
                            }
                        }

                        // --- Atmosphere ---
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let [smoke_r, o2, co2, temp] = self.debug.fluid_density;
                            let is_solid = bh > 0
                                && (bt == 1
                                    || bt == 4
                                    || bt == 5
                                    || bt == 14
                                    || (21..=25).contains(&bt)
                                    || bt == 35);
                            let is_pipe = pipes::is_pipe_component(bt);
                            ui.separator();
                            heading(ui, "\u{1f32b}", "Atmosphere");
                            if is_solid {
                                row(
                                    ui,
                                    "Wall temp",
                                    format!("{:.1}\u{b0}C", self.debug.block_temp),
                                );
                            } else if is_pipe {
                                row(
                                    ui,
                                    "Pipe temp",
                                    format!("{:.1}\u{b0}C", self.debug.block_temp),
                                );
                            } else {
                                let temp_col = if temp > 40.0 {
                                    egui::Color32::from_rgb(255, 120, 50)
                                } else if temp < 5.0 {
                                    egui::Color32::from_rgb(100, 150, 255)
                                } else {
                                    egui::Color32::from_rgb(180, 220, 180)
                                };
                                let o2_col = if o2 < 0.7 {
                                    egui::Color32::from_rgb(255, 80, 80)
                                } else {
                                    egui::Color32::from_rgb(150, 200, 150)
                                };
                                row_color(ui, "Temp", format!("{:.1}\u{b0}C", temp), temp_col);
                                row_color(ui, "O\u{2082}", format!("{:.2}", o2), o2_col);
                                if smoke_r > 0.01 {
                                    row(ui, "Smoke", format!("{:.3}", smoke_r));
                                }
                                if co2 > 0.01 {
                                    row(ui, "CO\u{2082}", format!("{:.3}", co2));
                                }
                            }
                        }

                        // --- Power ---
                        if self.debug.voltage > 0.01 {
                            ui.separator();
                            heading(ui, "\u{26a1}", "Power");
                            let v = self.debug.voltage;
                            let v_col = if v > 15.0 {
                                egui::Color32::from_rgb(255, 80, 80)
                            } else if v > 1.0 {
                                egui::Color32::from_rgb(120, 255, 120)
                            } else {
                                egui::Color32::from_rgb(150, 150, 150)
                            };
                            row_color(ui, "Voltage", format!("{:.1}V", v), v_col);
                            let amps = v / 10.0;
                            row(ui, "Current", format!("{:.2}A", amps));
                            row(ui, "Power", format!("{:.1}W", v * amps));
                        }

                        // --- Pipes ---
                        {
                            let pidx = by as u32 * GRID_W + bx as u32;
                            let is_gas = pipes::is_gas_pipe_component(bt);
                            let is_liq = pipes::is_liquid_pipe_component(bt);
                            if is_gas {
                                if let Some(cell) = self.pipe_network.cells.get(&pidx) {
                                    ui.separator();
                                    heading(ui, "\u{1f4a8}", "Gas Pipe");
                                    row(ui, "Pressure", format!("{:.2}", cell.pressure));
                                    row(ui, "Temp", format!("{:.1}\u{b0}C", cell.gas[3]));
                                    if cell.gas[0] > 0.01 {
                                        row(ui, "Smoke", format!("{:.3}", cell.gas[0]));
                                    }
                                    row(ui, "O\u{2082}", format!("{:.3}", cell.gas[1]));
                                    if cell.gas[2] > 0.01 {
                                        row(ui, "CO\u{2082}", format!("{:.3}", cell.gas[2]));
                                    }
                                }
                            } else if is_liq
                                && let Some(cell) = self.liquid_network.cells.get(&pidx)
                            {
                                ui.separator();
                                heading(ui, "\u{1f4a7}", "Liquid Pipe");
                                row(ui, "Pressure", format!("{:.2}", cell.pressure));
                                row(ui, "Temp", format!("{:.1}\u{b0}C", cell.gas[3]));
                            }
                        }

                        // --- Water ---
                        {
                            let wt = if idx < self.water_table.len() {
                                self.water_table[idx]
                            } else {
                                -2.0
                            };
                            let sw = self.debug.water_level;
                            if sw > 0.005 || wt > -1.0 {
                                ui.separator();
                                heading(ui, "\u{1f4a7}", "Water");
                                if sw > 0.005 {
                                    let label = if sw > 0.5 {
                                        "flooded"
                                    } else if sw > 0.15 {
                                        "puddle"
                                    } else {
                                        "moist"
                                    };
                                    row(ui, "Surface", format!("{:.2} ({})", sw, label));
                                }
                                let wt_label = if wt > 0.0 {
                                    "spring"
                                } else if wt > -0.5 {
                                    "wet"
                                } else {
                                    "moderate"
                                };
                                row(ui, "Table", format!("{:.1} ({})", wt, wt_label));
                            }
                        }

                        // --- Zone ---
                        {
                            let in_growing = self.zones.iter().any(|z| {
                                z.kind == zones::ZoneKind::Growing && z.tiles.contains(&(bx, by))
                            });
                            if in_growing {
                                ui.separator();
                                heading(ui, "\u{1f33e}", "Growing Zone");
                            }
                        }

                        // --- Crop ---
                        {
                            let cb = self.grid_data[idx];
                            let wt = if idx < self.water_table.len() {
                                self.water_table[idx]
                            } else {
                                -3.0
                            };
                            let timer = self.crop_timers.get(&(idx as u32)).copied().unwrap_or(0.0);
                            if let Some(cs) = zones::crop_status(
                                cb,
                                idx as u32,
                                timer,
                                self.time_of_day,
                                self.camera.sun_intensity,
                                self.camera.rain_intensity,
                                wt,
                                self.debug.water_level,
                            ) {
                                ui.separator();
                                heading(
                                    ui,
                                    "\u{1f331}",
                                    &format!("{} ({:.0}%)", cs.stage_name, cs.progress * 100.0),
                                );
                                let rate_col = if cs.growth_rate > 0.7 {
                                    egui::Color32::from_rgb(120, 255, 120)
                                } else if cs.growth_rate > 0.3 {
                                    egui::Color32::from_rgb(255, 220, 80)
                                } else {
                                    egui::Color32::from_rgb(255, 80, 80)
                                };
                                row_color(
                                    ui,
                                    "Growth",
                                    format!("{:.0}%  {}", cs.growth_rate * 100.0, cs.limiting),
                                    rate_col,
                                );
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "T:{:.0}%",
                                            cs.temp_factor * 100.0
                                        ))
                                        .size(9.0)
                                        .weak(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "S:{:.0}%",
                                            cs.sun_factor * 100.0
                                        ))
                                        .size(9.0)
                                        .weak(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "W:{:.0}%",
                                            cs.water_factor * 100.0
                                        ))
                                        .size(9.0)
                                        .weak(),
                                    );
                                });
                            }
                        }

                        // --- Material ---
                        {
                            let mats = crate::materials::build_material_table();
                            if (bt as usize) < mats.len() {
                                let m = &mats[bt as usize];
                                if m.heat_capacity > 0.0 || m.conductivity > 0.0 {
                                    ui.separator();
                                    heading(ui, "\u{1f9f1}", "Material");
                                    row(ui, "Heat cap", format!("{:.1}", m.heat_capacity));
                                    row(ui, "Conductivity", format!("{:.3}", m.conductivity));
                                }
                            }
                        }
                    });
                });
        }

        // Single-tile hover preview (before drag starts, shows what will be placed)
        if self.drag_start.is_none()
            && matches!(self.build_tool, BuildTool::Place(_) | BuildTool::Destroy)
        {
            let (hwx, hwy) = self.hover_world;
            let hx = hwx.floor() as i32;
            let hy = hwy.floor() as i32;
            if hx >= 0 && hy >= 0 && hx < GRID_W as i32 && hy < GRID_H as i32 {
                let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
                let sx0 =
                    ((hx as f32 - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy0 =
                    ((hy as f32 - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                let sx1 = ((hx as f32 + 1.0 - cam_cx) * cam_zoom + cam_sw * 0.5)
                    / self.render_scale
                    / bp_ppp;
                let sy1 = ((hy as f32 + 1.0 - cam_cy) * cam_zoom + cam_sh * 0.5)
                    / self.render_scale
                    / bp_ppp;
                let color = egui::Color32::from_rgba_unmultiplied(60, 140, 255, 120);
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Background,
                    egui::Id::new("hover_preview"),
                ));
                let is_wall_hover =
                    matches!(self.build_tool, BuildTool::Place(id) if is_wall_block(id));
                if is_wall_hover && self.wall_thickness < 4 {
                    let tw = sx1 - sx0;
                    let th = sy1 - sy0;
                    let wall_frac = self.wall_thickness as f32 * 0.25;
                    let edge = self.build_rotation as u8 % 4;
                    let wall_rect = match edge {
                        0 => egui::Rect::from_min_size(
                            egui::pos2(sx0, sy0),
                            egui::vec2(tw, th * wall_frac),
                        ),
                        1 => egui::Rect::from_min_size(
                            egui::pos2(sx0 + tw * (1.0 - wall_frac), sy0),
                            egui::vec2(tw * wall_frac, th),
                        ),
                        2 => egui::Rect::from_min_size(
                            egui::pos2(sx0, sy0 + th * (1.0 - wall_frac)),
                            egui::vec2(tw, th * wall_frac),
                        ),
                        _ => egui::Rect::from_min_size(
                            egui::pos2(sx0, sy0),
                            egui::vec2(tw * wall_frac, th),
                        ),
                    };
                    painter.rect_filled(wall_rect, 0.0, color);
                } else {
                    painter.rect_filled(
                        egui::Rect::from_min_max(egui::pos2(sx0, sy0), egui::pos2(sx1, sy1)),
                        0.0,
                        color,
                    );
                }
            }
        }

        // Drag shape preview (walls=hollow rect, pipes=line, destroy=filled rect)
        if let Some((sx, sy)) = self.drag_start
            && self.mouse_dragged
        {
            let (hwx, hwy) = self.hover_world;
            let (ex, ey) = (hwx.floor() as i32, hwy.floor() as i32);
            let tiles = match self.build_tool {
                BuildTool::Destroy
                | BuildTool::Roof
                | BuildTool::RemoveFloor
                | BuildTool::RemoveRoof => Self::filled_rect_tiles(sx, sy, ex, ey),
                BuildTool::Place(id) => {
                    let reg = crate::block_defs::BlockRegistry::cached();
                    let shape = reg
                        .get(id)
                        .and_then(|d| d.placement.as_ref())
                        .and_then(|p| p.drag.as_ref());
                    match shape {
                        Some(crate::block_defs::DragShape::Line) => {
                            Self::line_tiles(sx, sy, ex, ey)
                        }
                        Some(crate::block_defs::DragShape::FilledRect) => {
                            Self::filled_rect_tiles(sx, sy, ex, ey)
                        }
                        Some(crate::block_defs::DragShape::HollowRect) => {
                            let pleb_pos = self
                                .selected_pleb
                                .and_then(|pi| self.plebs.get(pi).map(|p| (p.x, p.y)));
                            Self::hollow_rect_tiles_with_entry(sx, sy, ex, ey, pleb_pos).0
                        }
                        _ => Vec::new(),
                    }
                }
                _ => Vec::new(),
            };
            if !tiles.is_empty() {
                let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Background,
                    egui::Id::new("drag_preview"),
                ));
                let is_destroy = self.build_tool == BuildTool::Destroy;
                let is_remove_floor = self.build_tool == BuildTool::RemoveFloor;
                let is_remove_roof = self.build_tool == BuildTool::RemoveRoof;
                let is_roof = self.build_tool == BuildTool::Roof;
                for (tx, ty) in &tiles {
                    let color = if is_destroy {
                        egui::Color32::from_rgba_unmultiplied(255, 50, 50, 100)
                    } else if is_remove_floor {
                        let valid =
                            if *tx < 0 || *ty < 0 || *tx >= GRID_W as i32 || *ty >= GRID_H as i32 {
                                false
                            } else {
                                let tidx = (*ty as u32 * GRID_W + *tx as u32) as usize;
                                let tb = self.grid_data[tidx];
                                let tbt = tb & 0xFF;
                                matches!(tbt, 26..=28)
                            };
                        if valid {
                            egui::Color32::from_rgba_unmultiplied(255, 50, 50, 100)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(255, 60, 60, 40)
                        }
                    } else if is_remove_roof {
                        let valid =
                            if *tx < 0 || *ty < 0 || *tx >= GRID_W as i32 || *ty >= GRID_H as i32 {
                                false
                            } else {
                                let tidx = (*ty as u32 * GRID_W + *tx as u32) as usize;
                                let tb = self.grid_data[tidx];
                                (tb >> 16) & 2 != 0
                            };
                        if valid {
                            egui::Color32::from_rgba_unmultiplied(255, 50, 50, 100)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(255, 60, 60, 40)
                        }
                    } else if is_roof {
                        // Roof preview: blue=valid support, red=no support
                        if Self::can_support_roof_wd(&self.grid_data, &self.wall_data, *tx, *ty) {
                            egui::Color32::from_rgba_unmultiplied(100, 160, 255, 100)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(255, 60, 60, 80)
                        }
                    } else {
                        // Validate each tile individually
                        let is_wire_tool = matches!(self.build_tool, BuildTool::Place(36));
                        let is_pipe_tool = matches!(self.build_tool, BuildTool::Place(15));
                        let valid =
                            if *tx < 0 || *ty < 0 || *tx >= GRID_W as i32 || *ty >= GRID_H as i32 {
                                false
                            } else if is_wire_tool {
                                true // wire can go anywhere
                            } else {
                                let tidx = (*ty as u32 * GRID_W + *tx as u32) as usize;
                                let tb = self.grid_data[tidx];
                                let tbt = tb & 0xFF;
                                let tbh = block_height_rs(tb) as u32;
                                // Allow placement on empty ground OR on existing same-type block
                                ((tbt == 0 || tbt == 2) && tbh == 0) || (is_pipe_tool && tbt == 15) // pipe on pipe = merge connections
                            };
                        if valid {
                            egui::Color32::from_rgba_unmultiplied(60, 140, 255, 120)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(255, 50, 50, 100)
                        }
                    };
                    let wx0 = *tx as f32;
                    let wy0 = *ty as f32;
                    let sx0 =
                        ((wx0 - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                    let sy0 =
                        ((wy0 - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                    let sx1 = ((wx0 + 1.0 - cam_cx) * cam_zoom + cam_sw * 0.5)
                        / self.render_scale
                        / bp_ppp;
                    let sy1 = ((wy0 + 1.0 - cam_cy) * cam_zoom + cam_sh * 0.5)
                        / self.render_scale
                        / bp_ppp;
                    let tile_rect =
                        egui::Rect::from_min_max(egui::pos2(sx0, sy0), egui::pos2(sx1, sy1));

                    // Thin wall preview: show sub-grid pattern
                    let is_wall_tool =
                        matches!(self.build_tool, BuildTool::Place(id) if is_wall_block(id));
                    if is_wall_tool && self.wall_thickness < 4 && !is_destroy {
                        let (min_x, max_x) = (sx.min(ex), sx.max(ex));
                        let (min_y, max_y) = (sy.min(ey), sy.max(ey));
                        let (edge, is_corner) = Self::thin_wall_edge_for_rect(
                            *tx,
                            *ty,
                            min_x,
                            max_x,
                            min_y,
                            max_y,
                            self.build_rotation,
                        );
                        let thick = self.wall_thickness;
                        let wall_frac = thick as f32 * 0.25;
                        let tw = sx1 - sx0;
                        let th = sy1 - sy0;
                        // Draw primary edge sub-rect
                        let primary_rect = match edge {
                            0 => egui::Rect::from_min_size(
                                egui::pos2(sx0, sy0),
                                egui::vec2(tw, th * wall_frac),
                            ),
                            1 => egui::Rect::from_min_size(
                                egui::pos2(sx0 + tw * (1.0 - wall_frac), sy0),
                                egui::vec2(tw * wall_frac, th),
                            ),
                            2 => egui::Rect::from_min_size(
                                egui::pos2(sx0, sy0 + th * (1.0 - wall_frac)),
                                egui::vec2(tw, th * wall_frac),
                            ),
                            _ => egui::Rect::from_min_size(
                                egui::pos2(sx0, sy0),
                                egui::vec2(tw * wall_frac, th),
                            ),
                        };
                        painter.rect_filled(primary_rect, 0.0, color);
                        if is_corner {
                            let next_edge = (edge + 1) % 4;
                            let corner_rect = match next_edge {
                                0 => egui::Rect::from_min_size(
                                    egui::pos2(sx0, sy0),
                                    egui::vec2(tw, th * wall_frac),
                                ),
                                1 => egui::Rect::from_min_size(
                                    egui::pos2(sx0 + tw * (1.0 - wall_frac), sy0),
                                    egui::vec2(tw * wall_frac, th),
                                ),
                                2 => egui::Rect::from_min_size(
                                    egui::pos2(sx0, sy0 + th * (1.0 - wall_frac)),
                                    egui::vec2(tw, th * wall_frac),
                                ),
                                _ => egui::Rect::from_min_size(
                                    egui::pos2(sx0, sy0),
                                    egui::vec2(tw * wall_frac, th),
                                ),
                            };
                            painter.rect_filled(corner_rect, 0.0, color);
                        }
                    } else {
                        painter.rect_filled(tile_rect, 0.0, color);
                    }
                }
                // Draw direction arrows on pipe/wire line tiles
                let is_line =
                    matches!(self.build_tool, BuildTool::Place(15) | BuildTool::Place(36));
                if is_line && tiles.len() > 1 {
                    let arrow_col = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 160);
                    for ti in 0..tiles.len() {
                        let (tx, ty) = tiles[ti];
                        let wx0 = tx as f32;
                        let wy0 = ty as f32;
                        let sx0 =
                            ((wx0 - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                        let sy0 =
                            ((wy0 - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                        let sx1 = ((wx0 + 1.0 - cam_cx) * cam_zoom + cam_sw * 0.5)
                            / self.render_scale
                            / bp_ppp;
                        let sy1 = ((wy0 + 1.0 - cam_cy) * cam_zoom + cam_sh * 0.5)
                            / self.render_scale
                            / bp_ppp;
                        let center = egui::pos2((sx0 + sx1) * 0.5, (sy0 + sy1) * 0.5);
                        let tile_sz = (sx1 - sx0).max(1.0);
                        // Arrow: toward next tile, or same direction as previous for last tile
                        let has_next = ti + 1 < tiles.len();
                        let has_prev = ti > 0;
                        let (adx, ady) = if has_next {
                            let (nx, ny) = tiles[ti + 1];
                            ((nx - tx) as f32, (ny - ty) as f32)
                        } else if has_prev {
                            let (px, py) = tiles[ti - 1];
                            ((tx - px) as f32, (ty - py) as f32)
                        } else {
                            (0.0, 0.0)
                        };
                        if adx != 0.0 || ady != 0.0 {
                            let alen = tile_sz * 0.3;
                            let tip = center + egui::Vec2::new(adx * alen, ady * alen);
                            let perp = egui::Vec2::new(-ady, adx) * alen * 0.4;
                            let base = center + egui::Vec2::new(adx * alen * 0.2, ady * alen * 0.2);
                            painter.add(egui::Shape::convex_polygon(
                                vec![tip, base + perp, base - perp],
                                arrow_col,
                                egui::Stroke::NONE,
                            ));
                        }
                    }
                }
            }
        }
    }

    fn draw_notifications(&mut self, ctx: &egui::Context) {
        if self.notifications.is_empty() {
            return;
        }
        let now = self.time_of_day;
        // Expiry: threats persist until clicked, warnings 12s, info/positive 7s
        self.notifications.retain(|n| {
            if n.dismissed {
                return false;
            }
            let age = (now - n.time_created).abs();
            if now < n.time_created {
                return true; // day wrapped
            }
            match n.category {
                types::NotifCategory::Threat => true, // sticky until dismissed
                types::NotifCategory::Warning => age < 12.0,
                _ => age < 7.0,
            }
        });

        let mut dismiss_id = None;
        // Left edge, show max 3 recent notes + overflow badge
        let max_visible = 3;
        let total = self.notifications.len();
        let overflow = total.saturating_sub(max_visible);
        let start = total.saturating_sub(max_visible);
        let notifs: Vec<(u32, &'static str, String, String, types::NotifCategory, f32)> = self
            .notifications[start..]
            .iter()
            .map(|n| {
                (
                    n.id,
                    n.icon,
                    n.title.clone(),
                    n.description.clone(),
                    n.category,
                    n.time_created,
                )
            })
            .collect();

        for (i, (id, icon, title, desc, category, created)) in notifs.iter().enumerate() {
            let y_offset = 60.0 + i as f32 * 48.0;
            // Random jitter per note for slight visual disorder
            let hash = id.wrapping_mul(2654435761);
            let x_jitter = ((hash & 0xFF) as f32 / 255.0 - 0.5) * 4.0; // ±2px

            // Fade: new notes are opaque, old ones dim
            let age = (now - created).abs();
            let alpha = if *category == types::NotifCategory::Threat {
                // Threats pulse
                let pulse = (age * 3.0).sin() * 0.05 + 0.95;
                (pulse * 255.0) as u8
            } else {
                let fade_start = match category {
                    types::NotifCategory::Warning => 8.0,
                    _ => 4.0,
                };
                if age > fade_start {
                    let t = ((age - fade_start) / 3.0).min(1.0);
                    (255.0 * (1.0 - t * 0.6)) as u8
                } else {
                    255
                }
            };

            // Seal color by category
            let seal_color = match category {
                types::NotifCategory::Threat => egui::Color32::from_rgb(180, 40, 40),
                types::NotifCategory::Warning => egui::Color32::from_rgb(190, 160, 50),
                types::NotifCategory::Positive => egui::Color32::from_rgb(50, 150, 70),
                types::NotifCategory::Info => egui::Color32::from_rgb(120, 120, 130),
            };

            egui::Area::new(egui::Id::new(("notif_card", *id)))
                .anchor(egui::Align2::LEFT_TOP, [8.0 + x_jitter, y_offset])
                .interactable(true)
                .show(ctx, |ui| {
                    // Parchment background
                    let parchment = egui::Color32::from_rgba_unmultiplied(235, 225, 205, alpha);
                    let border = egui::Color32::from_rgba_unmultiplied(170, 155, 130, alpha);

                    let resp = egui::Frame::NONE
                        .fill(parchment)
                        .stroke(egui::Stroke::new(0.5, border))
                        .corner_radius(2.0)
                        .inner_margin(egui::Margin {
                            left: 6,
                            right: 8,
                            top: 4,
                            bottom: 4,
                        })
                        .show(ui, |ui| {
                            ui.set_max_width(200.0);
                            ui.horizontal(|ui| {
                                // Seal/mark
                                let (seal_rect, _) = ui.allocate_exact_size(
                                    egui::Vec2::new(8.0, 8.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter_at(seal_rect).circle_filled(
                                    seal_rect.center(),
                                    4.0,
                                    seal_color,
                                );
                                // Icon + text
                                ui.label(egui::RichText::new(*icon).size(12.0).color(
                                    egui::Color32::from_rgba_unmultiplied(60, 50, 40, alpha),
                                ));
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new(title).strong().size(9.5).color(
                                        egui::Color32::from_rgba_unmultiplied(40, 35, 25, alpha),
                                    ));
                                    if !desc.is_empty() {
                                        ui.label(egui::RichText::new(desc).size(8.5).color(
                                            egui::Color32::from_rgba_unmultiplied(
                                                90, 80, 65, alpha,
                                            ),
                                        ));
                                    }
                                });
                            });
                        });
                    if resp.response.clicked() || resp.response.secondary_clicked() {
                        dismiss_id = Some(*id);
                    }
                });
        }
        // Overflow badge
        if overflow > 0 {
            let badge_y = 60.0 + notifs.len() as f32 * 48.0;
            egui::Area::new(egui::Id::new("notif_overflow"))
                .anchor(egui::Align2::LEFT_TOP, [10.0, badge_y])
                .interactable(false)
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new(format!("... +{} more", overflow))
                            .size(8.0)
                            .color(egui::Color32::from_rgba_unmultiplied(140, 130, 110, 180)),
                    );
                });
        }
        if let Some(id) = dismiss_id
            && let Some(n) = self.notifications.iter_mut().find(|n| n.id == id)
        {
            n.dismissed = true;
        }
    }

    /// Draw active conditions bar (below menu bar, persistent labels).
    fn draw_conditions_bar(&self, ctx: &egui::Context) {
        if self.conditions.is_empty() {
            return;
        }
        egui::Area::new(egui::Id::new("conditions_bar"))
            .anchor(egui::Align2::CENTER_TOP, [0.0, 30.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for cond in &self.conditions {
                        let color = cond.category.color();
                        egui::Frame::NONE
                            .fill(egui::Color32::from_rgba_unmultiplied(
                                color.r(),
                                color.g(),
                                color.b(),
                                220,
                            ))
                            .corner_radius(3.0)
                            .inner_margin(6.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(cond.icon).size(12.0));
                                    ui.label(
                                        egui::RichText::new(&cond.name)
                                            .strong()
                                            .size(10.0)
                                            .color(egui::Color32::WHITE),
                                    );
                                    if cond.duration > 0.0 {
                                        let pct = (cond.remaining / cond.duration * 100.0) as u32;
                                        ui.label(
                                            egui::RichText::new(format!("{}%", pct))
                                                .size(9.0)
                                                .color(egui::Color32::from_gray(200)),
                                        );
                                    }
                                });
                            });
                    }
                });
            });
    }

    /// Ambient hover info — styled tile summary panel near the cursor.
    fn draw_hover_info(&self, ctx: &egui::Context) {
        let (wx, wy) = self.hover_world;
        let bx = wx.floor() as i32;
        let by = wy.floor() as i32;
        if bx < 0 || by < 0 || bx >= GRID_W as i32 || by >= GRID_H as i32 {
            return;
        }
        let idx = (by as u32 * GRID_W + bx as u32) as usize;
        let block = self.grid_data[idx];
        let bt = block & 0xFF;
        let bh = block_height_rs(block) as u32;
        let flags = (block >> 16) & 0xFF;

        let reg = block_defs::BlockRegistry::cached();
        let type_name = reg.name(bt);

        let elev = if idx < self.elevation_data.len() {
            self.elevation_data[idx]
        } else {
            0.0
        };

        #[cfg(not(target_arch = "wasm32"))]
        let temp = self.debug.fluid_density[3];
        #[cfg(target_arch = "wasm32")]
        let temp = 15.0_f32;

        // Terrain type + vegetation
        let (terrain_name, terrain_veg) = if idx < self.terrain_data.len() {
            let td = self.terrain_data[idx];
            let veg = ((td >> 4) & 0x1F) as f32 / 31.0;
            let name = match terrain_type(td) {
                0 => "Grassland",
                1 => "Chalky Ground",
                2 => "Rocky Ground",
                3 => "Clay Soil",
                4 => "Gravel",
                5 => "Peatland",
                6 => "Marsh",
                7 => "Loam",
                8 => "Iron-Stained Ground",
                9 => "Copper-Stained Ground",
                10 => "Flint-Bearing Ground",
                11 => "Forest Floor",
                12 => "Sandy Ground",
                _ => "",
            };
            let veg_label = if veg > 0.65 {
                "Dense vegetation"
            } else if veg > 0.4 {
                "Moderate vegetation"
            } else if veg > 0.15 {
                "Sparse scrub"
            } else {
                "Barren"
            };
            (name, veg_label)
        } else {
            ("", "")
        };

        // Room
        let room_label = {
            let ridx = idx;
            if ridx < self.room_map.len() {
                let room_id = self.room_map[ridx];
                if room_id > 0 {
                    self.detected_rooms
                        .iter()
                        .find(|r| r.id == room_id)
                        .map(|r| format!("{} ({} tiles)", r.label, r.size))
                } else {
                    None
                }
            } else {
                None
            }
        };

        // Ground items
        let ground_items: Vec<String> = self
            .ground_items
            .iter()
            .filter(|gi| gi.x.floor() as i32 == bx && gi.y.floor() as i32 == by)
            .map(|gi| {
                let name = item_defs::ItemRegistry::cached()
                    .name(gi.stack.item_id)
                    .to_string();
                if gi.stack.count > 1 {
                    format!("{}x {}", gi.stack.count, name)
                } else {
                    name
                }
            })
            .collect();

        // Water depth
        let water_depth = {
            let widx = idx;
            if widx < self.water_depth_cpu.len() {
                self.water_depth_cpu[widx]
            } else {
                0.0
            }
        };

        // Is it a block worth naming or just ground?
        let is_ground = bt == BT_GROUND || bt == BT_AIR;
        let is_flora = block_defs::BlockRegistry::cached()
            .get(bt)
            .is_some_and(|d| d.is_plant);

        // Colors
        let bg = egui::Color32::from_rgba_unmultiplied(18, 20, 25, 220);
        let border = egui::Color32::from_gray(50);
        let title_col = egui::Color32::from_gray(230);
        let label_col = egui::Color32::from_gray(140);
        let value_col = egui::Color32::from_gray(195);
        let accent = egui::Color32::from_rgb(170, 145, 85);

        // Position: anchored above the game log in bottom-right
        let log_offset = if self.game_log.is_empty() {
            10.0
        } else {
            170.0
        };

        egui::Area::new(egui::Id::new("hover_info_panel"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -log_offset])
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(bg)
                    .corner_radius(4.0)
                    .inner_margin(8)
                    .stroke(egui::Stroke::new(1.0, border))
                    .show(ui, |ui| {
                        ui.set_max_width(180.0);

                        // Title: block name or terrain name
                        let title = if is_ground {
                            if terrain_name.is_empty() {
                                "Ground".to_string()
                            } else {
                                terrain_name.to_string()
                            }
                        } else {
                            type_name.to_string()
                        };
                        ui.label(
                            egui::RichText::new(&title)
                                .size(12.0)
                                .strong()
                                .color(title_col),
                        );

                        // Coordinates (subtle)
                        ui.label(
                            egui::RichText::new(format!("{}, {}", bx, by))
                                .size(9.0)
                                .color(label_col),
                        );

                        // Terrain details for ground tiles
                        if is_ground && !terrain_name.is_empty() {
                            ui.add_space(2.0);
                            // Vegetation
                            if !terrain_veg.is_empty() {
                                ui.label(
                                    egui::RichText::new(terrain_veg)
                                        .size(10.0)
                                        .color(egui::Color32::from_rgb(100, 140, 80)),
                                );
                            }
                        }

                        // Flora name (for plant tiles)
                        if is_flora && !is_ground {
                            ui.add_space(2.0);
                            if !terrain_name.is_empty() {
                                ui.label(
                                    egui::RichText::new(terrain_name)
                                        .size(10.0)
                                        .color(label_col),
                                );
                            }
                        }

                        // Environment: temp, elevation, water
                        ui.add_space(3.0);
                        let mut env_line = format!("{:.0}°C", temp);
                        if elev > 0.05 {
                            env_line.push_str(&format!("   elev {:.1}", elev));
                        }
                        if water_depth > 0.01 {
                            env_line.push_str(&format!("   water {:.1}", water_depth));
                        }
                        ui.label(egui::RichText::new(&env_line).size(9.5).color(value_col));

                        // Modifiers
                        if flags & 2 != 0 {
                            ui.label(egui::RichText::new("Roofed").size(9.5).color(label_col));
                        }
                        if flags & 1 != 0 {
                            let door_state = if flags & 4 != 0 {
                                "Door (open)"
                            } else {
                                "Door (closed)"
                            };
                            ui.label(egui::RichText::new(door_state).size(9.5).color(label_col));
                        }

                        // Room
                        if let Some(ref rl) = room_label {
                            ui.label(egui::RichText::new(rl).size(9.5).color(accent));
                        }

                        // Ground items
                        if !ground_items.is_empty() {
                            ui.add_space(2.0);
                            for item in &ground_items {
                                ui.label(
                                    egui::RichText::new(format!("  {}", item))
                                        .size(9.5)
                                        .color(egui::Color32::from_rgb(140, 160, 180)),
                                );
                            }
                        }
                    });
            });
    }

    fn draw_game_log(&self, ctx: &egui::Context) {
        if self.game_log.is_empty() {
            return;
        }

        egui::Area::new(egui::Id::new("game_log"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
            .show(ctx, |ui| {
                egui::Frame::window(ui.style())
                    .fill(egui::Color32::from_rgba_unmultiplied(20, 20, 25, 200))
                    .show(ui, |ui| {
                        ui.set_max_width(320.0);
                        ui.set_max_height(150.0);
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .stick_to_bottom(true)
                            .max_height(140.0)
                            .show(ui, |ui| {
                                for event in self.game_log.iter() {
                                    let [r, g, b] = event.category.color();
                                    let time_frac = event.time / DAY_DURATION;
                                    let hours = (time_frac * 24.0) as u32;
                                    let minutes = ((time_frac * 24.0 - hours as f32) * 60.0) as u32;
                                    let text = format!(
                                        "{:02}:{:02} {} {}",
                                        hours,
                                        minutes,
                                        event.category.icon(),
                                        event.message
                                    );
                                    ui.label(
                                        egui::RichText::new(text)
                                            .size(9.0)
                                            .color(egui::Color32::from_rgb(r, g, b)),
                                    );
                                }
                            });
                    });
            });
    }

    // --- Resource bar: colony-wide totals (top-left, below menu bar) ---
    fn draw_resource_bar(&self, ctx: &egui::Context) {
        // Count all resources: ground items + crate contents + pleb inventories
        let mut totals: std::collections::HashMap<u16, u32> = std::collections::HashMap::new();
        for item in &self.ground_items {
            *totals.entry(item.stack.item_id).or_default() += item.stack.count as u32;
        }
        for inv in self.crate_contents.values() {
            for stack in &inv.stacks {
                *totals.entry(stack.item_id).or_default() += stack.count as u32;
            }
        }
        for pleb in &self.plebs {
            if !pleb.is_enemy {
                for stack in &pleb.inventory.stacks {
                    *totals.entry(stack.item_id).or_default() += stack.count as u32;
                }
            }
        }
        let pleb_count = self
            .plebs
            .iter()
            .filter(|p| !p.is_enemy && !p.is_dead)
            .count();
        let bp_count = self.blueprints.len();

        // Sort by item ID for stable display order
        let mut sorted: Vec<(u16, u32)> = totals.into_iter().filter(|&(_, c)| c > 0).collect();
        sorted.sort_by_key(|&(id, _)| id);

        let item_reg = item_defs::ItemRegistry::cached();
        egui::Area::new(egui::Id::new("resource_bar"))
            .anchor(egui::Align2::LEFT_TOP, [10.0, 100.0])
            .interactable(false)
            .show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(
                    egui::Vec2::new(
                        240.0,
                        (2 + sorted.len() + if bp_count > 0 { 1 } else { 0 }) as f32 * 16.0,
                    ),
                    egui::Sense::hover(),
                );
                let painter = ui.painter_at(rect);
                let font = egui::FontId::proportional(12.0);
                let white = egui::Color32::from_gray(230);
                let mut y = rect.min.y;
                let x = rect.min.x;
                let line_h = 16.0;

                Self::shadow_text(
                    &painter,
                    egui::pos2(x, y),
                    egui::Align2::LEFT_TOP,
                    &format!("\u{1f464} {} colonists", pleb_count),
                    font.clone(),
                    white,
                );
                y += line_h;

                for (item_id, count) in &sorted {
                    let def = item_reg.get(*item_id);
                    let icon = def.map(|d| d.icon.as_str()).unwrap_or("?");
                    let name = def.map(|d| d.name.as_str()).unwrap_or("Unknown");
                    Self::shadow_text(
                        &painter,
                        egui::pos2(x, y),
                        egui::Align2::LEFT_TOP,
                        &format!("{} {} {}", icon, count, name),
                        font.clone(),
                        white,
                    );
                    y += line_h;
                }
                if bp_count > 0 {
                    Self::shadow_text(
                        &painter,
                        egui::pos2(x, y),
                        egui::Align2::LEFT_TOP,
                        &format!("\u{1f3d7} {} pending", bp_count),
                        font,
                        egui::Color32::from_gray(160),
                    );
                }
            });
    }

    // --- Bottom-center action bar for selected plebs ---
    fn draw_action_bar(&mut self, ctx: &egui::Context) {
        // Collect selected pleb indices
        let indices: Vec<usize> = if !self.selected_group.is_empty() {
            self.selected_group.clone()
        } else if let Some(idx) = self.selected_pleb {
            vec![idx]
        } else {
            return;
        };

        // Pleb selected → close build menu (mutually exclusive)
        if self.build_category.is_some() || self.build_tool != BuildTool::None {
            self.build_category = None;
            self.build_tool = BuildTool::None;
            self.terrain_tool = None;
        }

        // Filter to living friendly plebs
        let friendly: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|&i| self.plebs.get(i).is_some_and(|p| !p.is_dead && !p.is_enemy))
            .collect();
        if friendly.is_empty() {
            return;
        }

        // Snapshot pleb state
        let all_drafted = friendly.iter().all(|&i| self.plebs[i].drafted);
        let any_drafted = friendly.iter().any(|&i| self.plebs[i].drafted);
        let all_crouching = friendly.iter().all(|&i| self.plebs[i].crouching);
        let all_prefer_ranged = friendly.iter().all(|&i| self.plebs[i].prefer_ranged);
        let count = friendly.len();

        let reg = crate::item_defs::ItemRegistry::cached();
        let any_ranged_weapon = friendly.iter().any(|&i| {
            let p = &self.plebs[i];
            p.drafted
                && p.equipped_weapon
                    .and_then(|wid| reg.get(wid))
                    .is_some_and(|d| d.is_ranged_weapon())
        });
        let is_burst = self.combat.burst_mode;
        let is_attack_mode = self.combat.attack_mode;
        let is_grenade_targeting = self.combat.grenade_targeting;
        let grenade_arc = self.combat.grenade_arc;
        let cur_aim_mode = self.combat.aim_mode;
        let cur_spacing = self.combat.flock_spacing;
        let any_headlight = friendly.iter().any(|&i| self.plebs[i].headlight_mode > 0);
        let headlight_mode = if count == 1 {
            self.plebs[friendly[0]].headlight_mode
        } else {
            // Use first pleb's mode for display
            friendly
                .iter()
                .find(|&&i| self.plebs[i].headlight_mode > 0)
                .map(|&i| self.plebs[i].headlight_mode)
                .unwrap_or(0)
        };

        let any_leader = friendly.iter().any(|&i| self.plebs[i].is_leader);
        let leader_can_command = any_leader
            && friendly
                .iter()
                .any(|&i| self.plebs[i].is_leader && self.plebs[i].command_cooldown <= 0.0);

        // Weapon info for single pleb
        let weapon_info: Option<(String, u8, u8)> = if count == 1 && any_drafted {
            let p = &self.plebs[friendly[0]];
            p.equipped_weapon.and_then(|wid| {
                reg.get(wid)
                    .map(|d| (d.name.clone(), p.ammo_loaded, d.magazine_size))
            })
        } else {
            None
        };
        let has_weapon = weapon_info.is_some() || (count > 1 && any_drafted);
        let any_moving = friendly
            .iter()
            .any(|&i| self.plebs[i].path_idx < self.plebs[i].path.len());

        // First pleb's appearance
        let first = &self.plebs[friendly[0]];
        let portrait_name = if count == 1 {
            first.name.clone()
        } else {
            format!("{} plebs", count)
        };
        let skin = [
            first.appearance.skin_r,
            first.appearance.skin_g,
            first.appearance.skin_b,
        ];
        let hair = [
            first.appearance.hair_r,
            first.appearance.hair_g,
            first.appearance.hair_b,
        ];
        let shirt = [
            first.appearance.shirt_r,
            first.appearance.shirt_g,
            first.appearance.shirt_b,
        ];
        let hp = first.needs.health;
        let activity_str = if count == 1 {
            match first.activity.inner() {
                PlebActivity::Idle => "Idle",
                PlebActivity::Walking => "Walking",
                PlebActivity::Sleeping => "Sleeping",
                PlebActivity::Harvesting(_) => "Harvesting",
                PlebActivity::Eating => "Eating",
                PlebActivity::Hauling => "Hauling",
                PlebActivity::Farming(_) => "Farming",
                PlebActivity::Building(_) => "Building",
                PlebActivity::Crafting(_, _) => "Crafting",
                PlebActivity::Digging => "Digging",
                PlebActivity::Filling => "Building berm",
                PlebActivity::Drinking(_) => "Drinking",
                PlebActivity::MentalBreak(_, _) => "Mental break",
                PlebActivity::Butchering(_) => "Butchering",
                PlebActivity::Cooking(_) => "Cooking",
                PlebActivity::Fishing(_) => "Fishing",
                PlebActivity::Mining(_) => "Mining",
                PlebActivity::Staggering(_) => "Staggering",
                PlebActivity::Crisis(_, _) => "Crisis",
            }
        } else {
            ""
        };

        let tile_size = 56.0f32;
        let tile_pad = 4.0f32;
        let corner = 6.0f32;
        let hint_font = egui::FontId::proportional(8.0);
        let hint_color = egui::Color32::from_rgba_premultiplied(180, 180, 180, 160);
        let label_font = egui::FontId::proportional(9.0);
        let label_color = egui::Color32::from_rgb(200, 200, 200);
        let icon_font = egui::FontId::proportional(20.0);
        let default_bg = egui::Color32::from_rgb(45, 48, 55);

        let draw_hint = |painter: &egui::Painter, rect: egui::Rect, key: &str| {
            painter.text(
                rect.right_top() + egui::Vec2::new(-3.0, 3.0),
                egui::Align2::RIGHT_TOP,
                key,
                hint_font.clone(),
                hint_color,
            );
        };

        // Lighten a background color on hover
        let hover_bg = |bg: egui::Color32, hovered: bool| -> egui::Color32 {
            if !hovered {
                return bg;
            }
            egui::Color32::from_rgb(
                bg.r().saturating_add(20),
                bg.g().saturating_add(20),
                bg.b().saturating_add(20),
            )
        };

        // Count tiles
        let mut n_tiles = 2; // portrait + draft
        n_tiles += 1; // move
        if has_weapon {
            n_tiles += 1; // weapon/attack
        }
        if any_drafted {
            n_tiles += 1; // melee/ranged
        }
        if any_ranged_weapon {
            n_tiles += 1; // fire mode
            n_tiles += 1; // aim mode
        }
        if any_drafted {
            n_tiles += 2; // grenade + arc
        }
        if any_drafted || any_moving {
            n_tiles += 1; // hold position
        }
        if any_drafted {
            n_tiles += 1; // crouch
        }
        if any_headlight {
            n_tiles += 1; // headlight beam
        }
        if count > 1 {
            n_tiles += 1; // spacing
        }
        if any_leader && any_drafted {
            n_tiles += 1; // command
        }
        n_tiles += 1; // info

        let bar_w = n_tiles as f32 * (tile_size + tile_pad) + tile_pad;

        egui::Area::new(egui::Id::new("action_bar"))
            .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -10.0])
            .interactable(true)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgba_premultiplied(30, 32, 36, 230))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(tile_pad as i8))
                    .show(ui, |ui| {
                        ui.set_min_width(bar_w);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = egui::Vec2::new(tile_pad, 0.0);

                            // --- Portrait ---
                            {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::splat(tile_size),
                                    egui::Sense::click(),
                                );
                                let painter = ui.painter_at(rect);
                                painter.rect_filled(
                                    rect,
                                    corner,
                                    hover_bg(default_bg, response.hovered()),
                                );

                                if count == 1 {
                                    // Mini pleb sprite
                                    let center = rect.center() + egui::Vec2::new(0.0, -6.0);
                                    // Body
                                    let shirt_c = egui::Color32::from_rgb(
                                        (shirt[0] * 255.0) as u8,
                                        (shirt[1] * 255.0) as u8,
                                        (shirt[2] * 255.0) as u8,
                                    );
                                    painter.circle_filled(
                                        center + egui::Vec2::new(0.0, 7.0),
                                        8.0,
                                        shirt_c,
                                    );
                                    // Head
                                    let skin_c = egui::Color32::from_rgb(
                                        (skin[0] * 255.0) as u8,
                                        (skin[1] * 255.0) as u8,
                                        (skin[2] * 255.0) as u8,
                                    );
                                    painter.circle_filled(
                                        center + egui::Vec2::new(0.0, -2.0),
                                        5.0,
                                        skin_c,
                                    );
                                    // Hair
                                    let hair_c = egui::Color32::from_rgb(
                                        (hair[0] * 255.0) as u8,
                                        (hair[1] * 255.0) as u8,
                                        (hair[2] * 255.0) as u8,
                                    );
                                    painter.circle_filled(
                                        center + egui::Vec2::new(0.0, -5.0),
                                        3.5,
                                        hair_c,
                                    );
                                    // HP bar
                                    let bar_y = rect.max.y - 16.0;
                                    let bar_x = rect.min.x + 4.0;
                                    let bar_w = rect.width() - 8.0;
                                    let bar_rect = egui::Rect::from_min_size(
                                        egui::Pos2::new(bar_x, bar_y),
                                        egui::Vec2::new(bar_w, 3.0),
                                    );
                                    painter.rect_filled(
                                        bar_rect,
                                        1.0,
                                        egui::Color32::from_rgb(30, 30, 30),
                                    );
                                    let hp_col = if hp > 0.5 {
                                        egui::Color32::from_rgb(80, 200, 80)
                                    } else if hp > 0.25 {
                                        egui::Color32::from_rgb(200, 160, 40)
                                    } else {
                                        egui::Color32::from_rgb(200, 60, 60)
                                    };
                                    painter.rect_filled(
                                        egui::Rect::from_min_size(
                                            bar_rect.min,
                                            egui::Vec2::new(bar_w * hp.clamp(0.0, 1.0), 3.0),
                                        ),
                                        1.0,
                                        hp_col,
                                    );
                                } else {
                                    // Multi-select: group icon
                                    painter.text(
                                        rect.center() + egui::Vec2::new(0.0, -4.0),
                                        egui::Align2::CENTER_CENTER,
                                        "\u{1f465}", // people silhouette
                                        egui::FontId::proportional(20.0),
                                        egui::Color32::from_rgb(180, 190, 200),
                                    );
                                }
                                // Name / count label
                                painter.text(
                                    rect.center_bottom() + egui::Vec2::new(0.0, -2.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    &portrait_name,
                                    egui::FontId::proportional(8.0),
                                    egui::Color32::WHITE,
                                );
                                // Click: open character window
                                if response.clicked() {
                                    self.show_inventory = !self.show_inventory;
                                    self.inv_selected_slot = None;
                                }
                                // Activity tooltip (single pleb only)
                                if count == 1 && !activity_str.is_empty() {
                                    response.on_hover_text(activity_str);
                                }
                            }

                            // --- Draft/Undraft [D] ---
                            {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::splat(tile_size),
                                    egui::Sense::click(),
                                );
                                let painter = ui.painter_at(rect);
                                let bg = if all_drafted {
                                    egui::Color32::from_rgb(80, 40, 35)
                                } else {
                                    default_bg
                                };
                                painter.rect_filled(rect, corner, hover_bg(bg, response.hovered()));
                                let (icon, label) = if all_drafted {
                                    ("\u{2694}", "Drafted")
                                } else {
                                    ("\u{1f6e1}", "Draft")
                                };
                                painter.text(
                                    rect.center() + egui::Vec2::new(0.0, -6.0),
                                    egui::Align2::CENTER_CENTER,
                                    icon,
                                    icon_font.clone(),
                                    egui::Color32::WHITE,
                                );
                                painter.text(
                                    rect.center_bottom() + egui::Vec2::new(0.0, -4.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    label,
                                    label_font.clone(),
                                    label_color,
                                );
                                draw_hint(&painter, rect, "D");
                                if response.clicked() {
                                    let new_state = !all_drafted;
                                    for &i in &friendly {
                                        if let Some(p) = self.plebs.get_mut(i) {
                                            p.drafted = new_state;
                                            p.update_equipped_weapon();
                                            if !new_state {
                                                p.work_target = None;
                                                p.haul_target = None;
                                                p.harvest_target = None;
                                            }
                                        }
                                    }
                                }
                            }

                            // --- Move [M] ---
                            {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::splat(tile_size),
                                    egui::Sense::click(),
                                );
                                let painter = ui.painter_at(rect);
                                let bg = if self.combat.move_mode {
                                    egui::Color32::from_rgb(40, 65, 55)
                                } else {
                                    default_bg
                                };
                                painter.rect_filled(rect, corner, hover_bg(bg, response.hovered()));
                                painter.text(
                                    rect.center() + egui::Vec2::new(0.0, -6.0),
                                    egui::Align2::CENTER_CENTER,
                                    "\u{1f463}",
                                    icon_font.clone(),
                                    egui::Color32::WHITE,
                                );
                                painter.text(
                                    rect.center_bottom() + egui::Vec2::new(0.0, -4.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    "Move",
                                    label_font.clone(),
                                    label_color,
                                );
                                draw_hint(&painter, rect, "M");
                                if response.clicked() {
                                    self.combat.move_mode = !self.combat.move_mode;
                                }
                            }

                            // --- Weapon / Attack [A] ---
                            if has_weapon {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::splat(tile_size),
                                    egui::Sense::click(),
                                );
                                let painter = ui.painter_at(rect);
                                let bg = if is_attack_mode {
                                    egui::Color32::from_rgb(90, 35, 35)
                                } else {
                                    default_bg
                                };
                                painter.rect_filled(rect, corner, hover_bg(bg, response.hovered()));
                                if let Some((ref wpn_name, ammo, mag)) = weapon_info {
                                    let is_ranged = any_ranged_weapon;
                                    let icon = if is_ranged { "\u{1f52b}" } else { "\u{1fa93}" };
                                    painter.text(
                                        rect.center() + egui::Vec2::new(0.0, -8.0),
                                        egui::Align2::CENTER_CENTER,
                                        icon,
                                        icon_font.clone(),
                                        egui::Color32::WHITE,
                                    );
                                    if is_ranged {
                                        let ammo_str = format!("{}/{}", ammo, mag);
                                        let ammo_col = if ammo == 0 {
                                            egui::Color32::from_rgb(220, 80, 60)
                                        } else {
                                            egui::Color32::from_rgb(180, 200, 180)
                                        };
                                        painter.text(
                                            rect.center() + egui::Vec2::new(0.0, 6.0),
                                            egui::Align2::CENTER_CENTER,
                                            &ammo_str,
                                            egui::FontId::proportional(10.0),
                                            ammo_col,
                                        );
                                    }
                                    painter.text(
                                        rect.center_bottom() + egui::Vec2::new(0.0, -4.0),
                                        egui::Align2::CENTER_BOTTOM,
                                        wpn_name,
                                        egui::FontId::proportional(7.0),
                                        label_color,
                                    );
                                } else {
                                    painter.text(
                                        rect.center() + egui::Vec2::new(0.0, -6.0),
                                        egui::Align2::CENTER_CENTER,
                                        "\u{1f3af}",
                                        icon_font.clone(),
                                        egui::Color32::WHITE,
                                    );
                                    painter.text(
                                        rect.center_bottom() + egui::Vec2::new(0.0, -4.0),
                                        egui::Align2::CENTER_BOTTOM,
                                        "Attack",
                                        label_font.clone(),
                                        label_color,
                                    );
                                }
                                draw_hint(&painter, rect, "A");
                                if response.clicked() {
                                    self.combat.attack_mode = !self.combat.attack_mode;
                                }
                            }

                            // --- Melee/Ranged [R] ---
                            if any_drafted {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::splat(tile_size),
                                    egui::Sense::click(),
                                );
                                let painter = ui.painter_at(rect);
                                let bg = if all_prefer_ranged {
                                    egui::Color32::from_rgb(40, 50, 75)
                                } else {
                                    default_bg
                                };
                                painter.rect_filled(rect, corner, hover_bg(bg, response.hovered()));
                                let (icon, label) = if all_prefer_ranged {
                                    ("\u{1f52b}", "Ranged")
                                } else {
                                    ("\u{1fa93}", "Melee")
                                };
                                painter.text(
                                    rect.center() + egui::Vec2::new(0.0, -6.0),
                                    egui::Align2::CENTER_CENTER,
                                    icon,
                                    icon_font.clone(),
                                    egui::Color32::WHITE,
                                );
                                painter.text(
                                    rect.center_bottom() + egui::Vec2::new(0.0, -4.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    label,
                                    label_font.clone(),
                                    label_color,
                                );
                                draw_hint(&painter, rect, "R");
                                if response.clicked() {
                                    let new_state = !all_prefer_ranged;
                                    for &i in &friendly {
                                        if let Some(p) = self.plebs.get_mut(i) {
                                            p.prefer_ranged = new_state;
                                            p.update_equipped_weapon();
                                        }
                                    }
                                }
                            }

                            // --- Aim mode ---
                            if any_ranged_weapon {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::splat(tile_size),
                                    egui::Sense::click(),
                                );
                                let painter = ui.painter_at(rect);
                                let bg = match cur_aim_mode {
                                    0 => egui::Color32::from_rgb(70, 50, 35), // snap: warm
                                    2 => egui::Color32::from_rgb(35, 50, 70), // precise: cool
                                    _ => default_bg,
                                };
                                painter.rect_filled(rect, corner, hover_bg(bg, response.hovered()));
                                let (icon, label) = match cur_aim_mode {
                                    0 => ("\u{26a1}", "Snap"),     // lightning = fast
                                    2 => ("\u{1f3af}", "Precise"), // target = accurate
                                    _ => ("\u{2696}", "Normal"),   // scales = balanced
                                };
                                painter.text(
                                    rect.center() + egui::Vec2::new(0.0, -6.0),
                                    egui::Align2::CENTER_CENTER,
                                    icon,
                                    icon_font.clone(),
                                    egui::Color32::WHITE,
                                );
                                painter.text(
                                    rect.center_bottom() + egui::Vec2::new(0.0, -4.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    label,
                                    label_font.clone(),
                                    label_color,
                                );
                                if response.clicked() {
                                    self.combat.aim_mode = (self.combat.aim_mode + 1) % 3;
                                }
                            }

                            // --- Fire mode [X] ---
                            if any_ranged_weapon {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::splat(tile_size),
                                    egui::Sense::click(),
                                );
                                let painter = ui.painter_at(rect);
                                let bg = if is_burst {
                                    egui::Color32::from_rgb(75, 55, 30)
                                } else {
                                    default_bg
                                };
                                painter.rect_filled(rect, corner, hover_bg(bg, response.hovered()));
                                let (icon, label) = if is_burst {
                                    ("\u{1f4a5}", "Burst")
                                } else {
                                    ("\u{1f3af}", "Single")
                                };
                                painter.text(
                                    rect.center() + egui::Vec2::new(0.0, -6.0),
                                    egui::Align2::CENTER_CENTER,
                                    icon,
                                    icon_font.clone(),
                                    egui::Color32::WHITE,
                                );
                                painter.text(
                                    rect.center_bottom() + egui::Vec2::new(0.0, -4.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    label,
                                    label_font.clone(),
                                    label_color,
                                );
                                draw_hint(&painter, rect, "X");
                                if response.clicked() {
                                    self.combat.burst_mode = !self.combat.burst_mode;
                                }
                            }

                            // --- Grenade [B] ---
                            if any_drafted {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::splat(tile_size),
                                    egui::Sense::click(),
                                );
                                let painter = ui.painter_at(rect);
                                let bg = if is_grenade_targeting {
                                    egui::Color32::from_rgb(90, 50, 25)
                                } else {
                                    default_bg
                                };
                                painter.rect_filled(rect, corner, hover_bg(bg, response.hovered()));
                                painter.text(
                                    rect.center() + egui::Vec2::new(0.0, -6.0),
                                    egui::Align2::CENTER_CENTER,
                                    "\u{1f4a3}",
                                    icon_font.clone(),
                                    egui::Color32::WHITE,
                                );
                                painter.text(
                                    rect.center_bottom() + egui::Vec2::new(0.0, -4.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    "Throw",
                                    label_font.clone(),
                                    label_color,
                                );
                                draw_hint(&painter, rect, "B");
                                if response.clicked() {
                                    self.combat.grenade_targeting = !self.combat.grenade_targeting;
                                }
                            }

                            // --- Grenade Arc ---
                            if any_drafted {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::splat(tile_size),
                                    egui::Sense::click(),
                                );
                                let painter = ui.painter_at(rect);
                                let bg = match grenade_arc {
                                    0 => egui::Color32::from_rgb(55, 55, 40),
                                    2 => egui::Color32::from_rgb(40, 45, 60),
                                    _ => default_bg,
                                };
                                painter.rect_filled(rect, corner, hover_bg(bg, response.hovered()));
                                // Visual arc indicator: small curved line
                                let (icon, label) = match grenade_arc {
                                    0 => ("\u{27a1}", "Flat"), // right arrow
                                    2 => ("\u{2b06}", "Lob"),  // up arrow
                                    _ => ("\u{2197}", "Arc"),  // diagonal arrow
                                };
                                painter.text(
                                    rect.center() + egui::Vec2::new(0.0, -6.0),
                                    egui::Align2::CENTER_CENTER,
                                    icon,
                                    icon_font.clone(),
                                    egui::Color32::WHITE,
                                );
                                painter.text(
                                    rect.center_bottom() + egui::Vec2::new(0.0, -4.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    label,
                                    label_font.clone(),
                                    label_color,
                                );
                                if response.clicked() {
                                    self.combat.grenade_arc = (self.combat.grenade_arc + 1) % 3;
                                }
                            }

                            // --- Hold Position [S] ---
                            if any_drafted || any_moving {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::splat(tile_size),
                                    egui::Sense::click(),
                                );
                                let painter = ui.painter_at(rect);
                                painter.rect_filled(
                                    rect,
                                    corner,
                                    hover_bg(default_bg, response.hovered()),
                                );
                                painter.text(
                                    rect.center() + egui::Vec2::new(0.0, -6.0),
                                    egui::Align2::CENTER_CENTER,
                                    "\u{270b}",
                                    icon_font.clone(),
                                    egui::Color32::WHITE,
                                );
                                painter.text(
                                    rect.center_bottom() + egui::Vec2::new(0.0, -4.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    "Stop",
                                    label_font.clone(),
                                    label_color,
                                );
                                draw_hint(&painter, rect, "S");
                                if response.clicked() {
                                    for &i in &friendly {
                                        if let Some(p) = self.plebs.get_mut(i) {
                                            p.path.clear();
                                            p.path_idx = 0;
                                            p.hunt_target = None;
                                            p.aim_target = None;
                                            p.aim_pos = None;
                                            p.aim_progress = 0.0;
                                            if matches!(
                                                p.activity,
                                                PlebActivity::Walking | PlebActivity::Idle
                                            ) {
                                                p.activity = PlebActivity::Idle;
                                            }
                                        }
                                    }
                                }
                            }

                            // --- Crouch [C] ---
                            if any_drafted {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::splat(tile_size),
                                    egui::Sense::click(),
                                );
                                let painter = ui.painter_at(rect);
                                let bg = if all_crouching {
                                    egui::Color32::from_rgb(55, 65, 45)
                                } else {
                                    default_bg
                                };
                                painter.rect_filled(rect, corner, hover_bg(bg, response.hovered()));
                                let (icon, label) = if all_crouching {
                                    ("\u{1f9ce}", "Crouch") // kneeling person
                                } else {
                                    ("\u{1f9cd}", "Stand") // standing person
                                };
                                painter.text(
                                    rect.center() + egui::Vec2::new(0.0, -6.0),
                                    egui::Align2::CENTER_CENTER,
                                    icon,
                                    icon_font.clone(),
                                    egui::Color32::WHITE,
                                );
                                painter.text(
                                    rect.center_bottom() + egui::Vec2::new(0.0, -4.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    label,
                                    label_font.clone(),
                                    label_color,
                                );
                                draw_hint(&painter, rect, "C");
                                if response.clicked() {
                                    let new_state = !all_crouching;
                                    for &i in &friendly {
                                        if let Some(p) = self.plebs.get_mut(i) {
                                            p.crouching = new_state;
                                            p.peek_timer = 0.0;
                                        }
                                    }
                                }
                            }

                            // --- Headlight beam [G] ---
                            if any_headlight {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::splat(tile_size),
                                    egui::Sense::click(),
                                );
                                let painter = ui.painter_at(rect);
                                let bg = match headlight_mode {
                                    1 => egui::Color32::from_rgb(55, 60, 45), // wide: greenish
                                    3 => egui::Color32::from_rgb(50, 50, 70), // focused: bluish
                                    _ => default_bg,                          // normal
                                };
                                painter.rect_filled(rect, corner, hover_bg(bg, response.hovered()));
                                let (icon, label) = match headlight_mode {
                                    1 => ("\u{1f506}", "Wide"),   // dim sun = wide beam
                                    3 => ("\u{1f526}", "Focus"),  // flashlight = focused
                                    _ => ("\u{1f4a1}", "Normal"), // light bulb = normal
                                };
                                painter.text(
                                    rect.center() + egui::Vec2::new(0.0, -6.0),
                                    egui::Align2::CENTER_CENTER,
                                    icon,
                                    icon_font.clone(),
                                    egui::Color32::WHITE,
                                );
                                painter.text(
                                    rect.center_bottom() + egui::Vec2::new(0.0, -4.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    label,
                                    label_font.clone(),
                                    label_color,
                                );
                                draw_hint(&painter, rect, "G");
                                if response.clicked() {
                                    // Cycle: wide → normal → focused → wide
                                    let new_mode = match headlight_mode {
                                        1 => 2u8,
                                        2 => 3,
                                        _ => 1,
                                    };
                                    for &i in &friendly {
                                        if let Some(p) = self.plebs.get_mut(i) {
                                            if p.headlight_mode > 0 {
                                                p.headlight_mode = new_mode;
                                            }
                                        }
                                    }
                                }
                            }

                            // --- Spacing (group only) ---
                            if count > 1 {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::splat(tile_size),
                                    egui::Sense::click(),
                                );
                                let painter = ui.painter_at(rect);
                                let bg = match cur_spacing {
                                    comms::FlockSpacing::Tight => {
                                        egui::Color32::from_rgb(75, 50, 40)
                                    }
                                    comms::FlockSpacing::Normal => default_bg,
                                    comms::FlockSpacing::Spread => {
                                        egui::Color32::from_rgb(40, 55, 70)
                                    }
                                };
                                painter.rect_filled(rect, corner, hover_bg(bg, response.hovered()));
                                // Dots pattern to indicate spacing
                                let icon = match cur_spacing {
                                    comms::FlockSpacing::Tight => "\u{2022}\u{2022}", // ••
                                    comms::FlockSpacing::Normal => "\u{2022} \u{2022}", // • •
                                    comms::FlockSpacing::Spread => "\u{2022}  \u{2022}", // •  •
                                };
                                painter.text(
                                    rect.center() + egui::Vec2::new(0.0, -6.0),
                                    egui::Align2::CENTER_CENTER,
                                    icon,
                                    egui::FontId::proportional(16.0),
                                    egui::Color32::WHITE,
                                );
                                painter.text(
                                    rect.center_bottom() + egui::Vec2::new(0.0, -4.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    cur_spacing.label(),
                                    label_font.clone(),
                                    label_color,
                                );
                                if response.clicked() {
                                    self.combat.flock_spacing = cur_spacing.cycle();
                                }
                            }

                            // --- Command [V] (leader only) ---
                            if any_leader && any_drafted {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::splat(tile_size),
                                    egui::Sense::click(),
                                );
                                let painter = ui.painter_at(rect);
                                let bg = if leader_can_command {
                                    egui::Color32::from_rgb(60, 55, 40)
                                } else {
                                    egui::Color32::from_rgb(35, 35, 38)
                                };
                                painter.rect_filled(rect, corner, hover_bg(bg, response.hovered()));
                                painter.text(
                                    rect.center() + egui::Vec2::new(0.0, -6.0),
                                    egui::Align2::CENTER_CENTER,
                                    "\u{1f4e3}", // megaphone
                                    icon_font.clone(),
                                    if leader_can_command {
                                        egui::Color32::WHITE
                                    } else {
                                        egui::Color32::from_rgb(100, 100, 100)
                                    },
                                );
                                painter.text(
                                    rect.center_bottom() + egui::Vec2::new(0.0, -4.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    "Rally",
                                    label_font.clone(),
                                    label_color,
                                );
                                draw_hint(&painter, rect, "V");
                                if response.clicked() && leader_can_command {
                                    // Issue Rally shout from the first leader
                                    for &i in &friendly {
                                        if self.plebs[i].is_leader
                                            && self.plebs[i].command_cooldown <= 0.0
                                        {
                                            self.plebs[i].command_cooldown =
                                                morale::COMMAND_COOLDOWN;
                                            self.plebs[i].set_bubble(
                                                pleb::BubbleKind::Text("Hold the line!".into()),
                                                2.0,
                                            );
                                            // Emit rally shout for processing
                                            self.combat.pending_command_shouts.push(comms::Shout {
                                                kind: comms::ShoutKind::Rally,
                                                x: self.plebs[i].x,
                                                y: self.plebs[i].y,
                                                pleb_idx: i,
                                                is_enemy: false,
                                            });
                                            break;
                                        }
                                    }
                                }
                                if response.secondary_clicked() && leader_can_command {
                                    // Right-click: cycle command (Advance)
                                    for &i in &friendly {
                                        if self.plebs[i].is_leader
                                            && self.plebs[i].command_cooldown <= 0.0
                                        {
                                            self.plebs[i].command_cooldown =
                                                morale::COMMAND_COOLDOWN;
                                            self.plebs[i].set_bubble(
                                                pleb::BubbleKind::Text("Move up!".into()),
                                                2.0,
                                            );
                                            self.combat.pending_command_shouts.push(comms::Shout {
                                                kind: comms::ShoutKind::Advance,
                                                x: self.plebs[i].x,
                                                y: self.plebs[i].y,
                                                pleb_idx: i,
                                                is_enemy: false,
                                            });
                                            break;
                                        }
                                    }
                                }
                            }

                            // --- Info [I] ---
                            {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::splat(tile_size),
                                    egui::Sense::click(),
                                );
                                let painter = ui.painter_at(rect);
                                let bg = if self.show_inventory {
                                    egui::Color32::from_rgb(40, 65, 50)
                                } else {
                                    default_bg
                                };
                                painter.rect_filled(rect, corner, hover_bg(bg, response.hovered()));
                                painter.text(
                                    rect.center() + egui::Vec2::new(0.0, -6.0),
                                    egui::Align2::CENTER_CENTER,
                                    "\u{1f4cb}",
                                    icon_font.clone(),
                                    egui::Color32::WHITE,
                                );
                                painter.text(
                                    rect.center_bottom() + egui::Vec2::new(0.0, -4.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    "Info",
                                    label_font.clone(),
                                    label_color,
                                );
                                draw_hint(&painter, rect, "I");
                                if response.clicked() {
                                    self.show_inventory = !self.show_inventory;
                                    self.inv_selected_slot = None;
                                }
                            }
                        });
                    });
            });
    }

    // --- Selection info panel: details about selected block ---
    fn draw_selection_info(&mut self, ctx: &egui::Context) {
        if self.world_sel.is_empty() {
            return;
        }
        let items = &self.world_sel.items;
        if items.len() != 1 {
            return;
        } // only single selection
        let item = &items[0];

        // Pleb selection: handled by bottom action bar
        if item.pleb_idx.is_some() {
            return;
        }

        // Block selection: show block info
        let bx = item.x;
        let by = item.y;
        if bx < 0 || by < 0 || bx >= GRID_W as i32 || by >= GRID_H as i32 {
            return;
        }
        let idx = (by as u32 * GRID_W + bx as u32) as usize;
        let block = self.grid_data[idx];
        let bt = block & 0xFF;
        let bh = block_height_rs(block) as u32;
        let flags = (block >> 16) & 0xFF;
        let reg = block_defs::BlockRegistry::cached();
        let type_name = reg.name(bt);

        egui::Area::new(egui::Id::new("selection_info"))
            .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
            .show(ctx, |ui| {
                egui::Frame::window(ui.style()).show(ui, |ui| {
                    ui.set_min_width(160.0);
                    ui.label(egui::RichText::new(type_name).strong().size(13.0));

                    let mut details = Vec::new();
                    if bh > 0 {
                        details.push(format!("Height: {}", bh));
                    }
                    if flags & 2 != 0 {
                        details.push("Roofed".to_string());
                    }
                    if flags & 1 != 0 {
                        details.push(
                            if flags & 4 != 0 {
                                "Door: Open"
                            } else {
                                "Door: Closed"
                            }
                            .to_string(),
                        );
                    }
                    // Elevation
                    if idx < self.elevation_data.len() {
                        let elev = self.elevation_data[idx];
                        if elev > 0.1 {
                            details.push(format!("Elevation: {:.1}", elev));
                        }
                    }
                    // Power
                    if self.gfx.is_some() {
                        // Voltage info from debug readback
                        if self.debug.voltage > 0.01 {
                            details.push(format!("Voltage: {:.1}V", self.debug.voltage));
                        }
                    }
                    // Ground items at this tile
                    for gi in &self.ground_items {
                        if gi.x.floor() as i32 == bx && gi.y.floor() as i32 == by {
                            details.push(gi.stack.label());
                            if gi.stack.is_container() {
                                let cap = gi.stack.liquid_capacity();
                                if let Some((liq, amt)) = gi.stack.liquid {
                                    let liq_name = match liq {
                                        item_defs::LiquidType::Water => "Water",
                                    };
                                    details.push(format!("{}: {}/{}", liq_name, amt, cap));
                                } else {
                                    details.push(format!("Empty (capacity: {})", cap));
                                }
                            }
                        }
                    }
                    // Blueprint
                    if let Some(bp) = self.blueprints.get(&(bx, by)) {
                        details.push(format!("Build: {:.0}%", bp.progress * 100.0));
                        if bp.wood_needed > 0 {
                            details.push(format!("Wood: {}/{}", bp.wood_delivered, bp.wood_needed));
                        }
                        if bp.plank_needed > 0 {
                            details
                                .push(format!("Plank: {}/{}", bp.plank_delivered, bp.plank_needed));
                        }
                        if bp.clay_needed > 0 {
                            details.push(format!("Clay: {}/{}", bp.clay_delivered, bp.clay_needed));
                        }
                    }

                    if !details.is_empty() {
                        for d in &details {
                            ui.label(egui::RichText::new(d).size(10.0).weak());
                        }
                    }
                });
            });
    }

    // --- Stone Lab: procedural stone material iteration tool ---
    fn draw_stone_lab(&mut self, ctx: &egui::Context) {
        if !self.show_stone_lab {
            return;
        }

        egui::Window::new("Stone Lab")
            .collapsible(true)
            .resizable(false)
            .default_pos([10.0, 40.0])
            .show(ctx, |ui| {
                // Preset buttons
                ui.horizontal(|ui| {
                    for (i, (name, _)) in StoneLab::PRESETS.iter().enumerate() {
                        let selected = self.stone_lab.preset == i;
                        if ui.selectable_label(selected, *name).clicked() {
                            self.stone_lab.apply_preset(i);
                        }
                    }
                });
                ui.separator();

                let sl = &mut self.stone_lab.gpu;
                sl.enabled = 1.0;

                // Base color
                ui.label(egui::RichText::new("Base Color").size(10.0).strong());
                ui.horizontal(|ui| {
                    ui.spacing_mut().slider_width = 100.0;
                    ui.add(egui::Slider::new(&mut sl.base_r, 0.0..=1.0).text("R"));
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().slider_width = 100.0;
                    ui.add(egui::Slider::new(&mut sl.base_g, 0.0..=1.0).text("G"));
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().slider_width = 100.0;
                    ui.add(egui::Slider::new(&mut sl.base_b, 0.0..=1.0).text("B"));
                });

                ui.separator();
                ui.label(egui::RichText::new("Surface").size(10.0).strong());
                ui.add(egui::Slider::new(&mut sl.grain_scale, 1.0..=25.0).text("Grain scale"));
                ui.add(egui::Slider::new(&mut sl.grain_strength, 0.0..=1.0).text("Grain str"));
                ui.add(egui::Slider::new(&mut sl.roughness, 0.0..=1.0).text("Roughness"));

                ui.separator();
                ui.label(egui::RichText::new("Cracks").size(10.0).strong());
                ui.add(egui::Slider::new(&mut sl.crack_density, 0.0..=1.0).text("Density"));
                ui.add(egui::Slider::new(&mut sl.crack_width, 0.0..=1.0).text("Width"));

                ui.separator();
                ui.label(egui::RichText::new("Strata").size(10.0).strong());
                ui.add(egui::Slider::new(&mut sl.strata_strength, 0.0..=1.0).text("Strength"));
                ui.add(egui::Slider::new(&mut sl.strata_scale, 1.0..=15.0).text("Scale"));

                ui.separator();
                ui.label(egui::RichText::new("Weathering").size(10.0).strong());
                ui.add(egui::Slider::new(&mut sl.weathering, 0.0..=1.0).text("Amount"));
                ui.horizontal(|ui| {
                    ui.spacing_mut().slider_width = 60.0;
                    ui.add(egui::Slider::new(&mut sl.weather_r, 0.0..=1.0).text("R"));
                    ui.add(egui::Slider::new(&mut sl.weather_g, 0.0..=1.0).text("G"));
                    ui.add(egui::Slider::new(&mut sl.weather_b, 0.0..=1.0).text("B"));
                });

                ui.separator();
                ui.add(egui::Slider::new(&mut sl.specular, 0.0..=1.0).text("Specular"));
            });
    }

    // --- Minimap: world overview (bottom-left corner, above build bar) ---
    fn draw_hints(&mut self, ctx: &egui::Context, bp_cam: (f32, f32, f32, f32, f32), bp_ppp: f32) {
        if self.game_hints.is_empty() {
            self.active_hint_idx = None;
            return;
        }

        let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;

        // Draw hint bar at top-center
        egui::Area::new(egui::Id::new("game_hints"))
            .anchor(egui::Align2::CENTER_TOP, [0.0, 32.0])
            .interactable(true)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgba_unmultiplied(20, 20, 25, 200))
                    .corner_radius(6.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        let mut hovered_idx = None;
                        for (i, hint) in self.game_hints.iter().enumerate() {
                            let label = ui.label(
                                egui::RichText::new(&hint.text)
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(255, 200, 100)),
                            );
                            if label.hovered() {
                                hovered_idx = Some(i);
                            }
                        }
                        self.active_hint_idx = hovered_idx;
                    });
            });

        // Highlight resources on map when hint is hovered
        if let Some(idx) = self.active_hint_idx
            && idx < self.game_hints.len()
        {
            let hint = &self.game_hints[idx];
            let highlight_items = hint.highlight_items.clone();
            let highlight_blocks = hint.highlight_blocks.clone();

            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("hint_highlight"),
            ));

            let to_screen = |wx: f32, wy: f32| -> egui::Pos2 {
                let sx = ((wx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy = ((wy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                egui::pos2(sx, sy)
            };

            let screen_rect = ctx.content_rect();
            let pulse = (self.frame_count as f32 * 0.08).sin() * 0.3 + 0.7;
            let highlight_col =
                egui::Color32::from_rgba_unmultiplied(255, 200, 50, (pulse * 120.0) as u8);
            let tile_px = cam_zoom / self.render_scale / bp_ppp;

            // Highlight matching ground items
            for item in &self.ground_items {
                if highlight_items.contains(&item.stack.item_id) {
                    let center = to_screen(item.x, item.y);
                    if center.x > screen_rect.min.x - 20.0
                        && center.x < screen_rect.max.x + 20.0
                        && center.y > screen_rect.min.y - 20.0
                        && center.y < screen_rect.max.y + 20.0
                    {
                        let r = (tile_px * 0.3).max(4.0);
                        painter.circle_stroke(center, r, egui::Stroke::new(2.0, highlight_col));
                    }
                }
            }

            // Highlight matching blocks (e.g. trees for "gather branches")
            if !highlight_blocks.is_empty() {
                let min_x = ((cam_cx - cam_sw * 0.5 / cam_zoom) as i32 - 1).max(0) as u32;
                let max_x =
                    ((cam_cx + cam_sw * 0.5 / cam_zoom) as i32 + 2).min(GRID_W as i32) as u32;
                let min_y = ((cam_cy - cam_sh * 0.5 / cam_zoom) as i32 - 1).max(0) as u32;
                let max_y =
                    ((cam_cy + cam_sh * 0.5 / cam_zoom) as i32 + 2).min(GRID_H as i32) as u32;
                for y in min_y..max_y {
                    for x in min_x..max_x {
                        let idx2 = (y * GRID_W + x) as usize;
                        if idx2 < self.grid_data.len() {
                            let bt = self.grid_data[idx2] & 0xFF;
                            if highlight_blocks.contains(&bt) {
                                let center = to_screen(x as f32 + 0.5, y as f32 + 0.5);
                                let r = (tile_px * 0.4).max(3.0);
                                painter.circle_stroke(
                                    center,
                                    r,
                                    egui::Stroke::new(1.5, highlight_col),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
