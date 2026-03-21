//! UI drawing — all egui panels, overlays, debug tooltips.
//! Extracted from render() to keep main.rs manageable.

use crate::*;

impl App {
    /// Pixels-per-point scale factor for the current window.
    fn ppp(&self) -> f32 {
        self.window.as_ref().map(|w| w.scale_factor() as f32).unwrap_or(1.0)
    }

    /// Convert world coords to screen coords for egui overlay drawing.
    fn world_to_screen_ui(&self, wx: f32, wy: f32, bp_cam: (f32,f32,f32,f32,f32)) -> egui::Pos2 {
        let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
        let ppp = self.ppp();
        let sx = ((wx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / ppp;
        let sy = ((wy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / ppp;
        egui::pos2(sx, sy)
    }

    /// Tile size in screen pixels at current zoom.
    fn tile_px(&self, bp_cam: (f32,f32,f32,f32,f32)) -> f32 {
        bp_cam.2 / self.render_scale / self.ppp()
    }

    pub fn draw_ui(&mut self, ctx: &egui::Context, bp_cam: (f32,f32,f32,f32,f32), blueprint_tiles: Vec<((i32,i32), bool)>, dt: f32) {
        let bp_ppp = self.ppp();
        self.draw_layers_bar(ctx);
        self.draw_layer_legend(ctx);
        self.draw_menu_bar(ctx, dt);
        self.draw_inventory_window(ctx);
        self.draw_build_bar(ctx);
        self.draw_colonist_bar(ctx);
        self.draw_context_menus(ctx, bp_ppp, bp_cam);
        self.draw_world_overlays(ctx, bp_cam, &blueprint_tiles);
        self.draw_world_labels(ctx, bp_cam);
    }

    fn draw_layers_bar(&mut self, ctx: &egui::Context) {
        // --- Layers bar (top-right, horizontal groups with labels above) ---
        egui::Area::new(egui::Id::new("layers_menu"))
            .anchor(egui::Align2::RIGHT_TOP, [-10.0, 32.0])
            .show(ctx, |ui| {
                egui::Frame::window(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let ov = &mut self.fluid_overlay;
                        // Atmosphere group
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Atmosphere").size(9.0).weak());
                            ui.horizontal(|ui| {
                                if ui.selectable_label(*ov == FluidOverlay::Gases, "Gases").clicked() {
                                    *ov = if *ov == FluidOverlay::Gases { FluidOverlay::None } else { FluidOverlay::Gases };
                                }
                                if ui.selectable_label(*ov == FluidOverlay::Smoke, "Smoke").clicked() {
                                    *ov = if *ov == FluidOverlay::Smoke { FluidOverlay::None } else { FluidOverlay::Smoke };
                                }
                                if ui.selectable_label(*ov == FluidOverlay::O2, "O\u{2082}").clicked() {
                                    *ov = if *ov == FluidOverlay::O2 { FluidOverlay::None } else { FluidOverlay::O2 };
                                }
                                if ui.selectable_label(*ov == FluidOverlay::CO2, "CO\u{2082}").clicked() {
                                    *ov = if *ov == FluidOverlay::CO2 { FluidOverlay::None } else { FluidOverlay::CO2 };
                                }
                            });
                        });
                        ui.separator();
                        // Thermal group
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Thermal").size(9.0).weak());
                            ui.horizontal(|ui| {
                                if ui.selectable_label(*ov == FluidOverlay::Temp, "Temp").clicked() {
                                    *ov = if *ov == FluidOverlay::Temp { FluidOverlay::None } else { FluidOverlay::Temp };
                                }
                                if ui.selectable_label(*ov == FluidOverlay::HeatFlow, "Heat").clicked() {
                                    *ov = if *ov == FluidOverlay::HeatFlow { FluidOverlay::None } else { FluidOverlay::HeatFlow };
                                }
                            });
                        });
                        ui.separator();
                        // Physics group
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Physics").size(9.0).weak());
                            ui.horizontal(|ui| {
                                if ui.selectable_label(*ov == FluidOverlay::Velocity, "Vel").clicked() {
                                    *ov = if *ov == FluidOverlay::Velocity { FluidOverlay::None } else { FluidOverlay::Velocity };
                                }
                                if ui.selectable_label(*ov == FluidOverlay::Pressure, "Pres").clicked() {
                                    *ov = if *ov == FluidOverlay::Pressure { FluidOverlay::None } else { FluidOverlay::Pressure };
                                }
                            });
                        });
                        ui.separator();
                        // Infrastructure
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Infra").size(9.0).weak());
                            ui.horizontal(|ui| {
                                if ui.selectable_label(self.show_pipe_overlay, "Pipes").clicked() {
                                    self.show_pipe_overlay = !self.show_pipe_overlay;
                                }
                                if ui.selectable_label(*ov == FluidOverlay::Power, "Volts").clicked() {
                                    *ov = if *ov == FluidOverlay::Power { FluidOverlay::None } else { FluidOverlay::Power };
                                }
                                if ui.selectable_label(*ov == FluidOverlay::PowerAmps, "Amps").clicked() {
                                    *ov = if *ov == FluidOverlay::PowerAmps { FluidOverlay::None } else { FluidOverlay::PowerAmps };
                                }
                                if ui.selectable_label(*ov == FluidOverlay::PowerWatts, "Watts").clicked() {
                                    *ov = if *ov == FluidOverlay::PowerWatts { FluidOverlay::None } else { FluidOverlay::PowerWatts };
                                }
                            });
                        });
                    });
                });
            });

    }

    fn draw_layer_legend(&mut self, ctx: &egui::Context) {
        // --- Layer legend (below layers menu, top-right) ---
        {
            let s = 12.0;
            let dot = |ui: &mut egui::Ui, col: egui::Color32, label: &str| {
                ui.horizontal(|ui| {
                    let (r, p) = ui.allocate_painter(egui::Vec2::splat(s), egui::Sense::hover());
                    p.rect_filled(r.rect, 2.0, col);
                    ui.label(egui::RichText::new(label).size(11.0));
                });
            };
            let grad = |ui: &mut egui::Ui, colors: &[(egui::Color32, &str)]| {
                for &(col, label) in colors {
                    dot(ui, col, label);
                }
            };
            let show_legend = self.fluid_overlay != FluidOverlay::None || self.show_pipe_overlay;
            if show_legend {
                egui::Area::new(egui::Id::new("layer_legend"))
                    .anchor(egui::Align2::RIGHT_TOP, [-10.0, 80.0])
                    .interactable(false)
                    .show(ctx, |ui| {
                        egui::Frame::window(ui.style()).show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            match self.fluid_overlay {
                                FluidOverlay::Gases => {
                                    dot(ui, egui::Color32::from_rgb(230, 230, 235), "Smoke (white)");
                                    dot(ui, egui::Color32::from_rgb(50, 100, 255), "O\u{2082} deficit (blue)");
                                    dot(ui, egui::Color32::from_rgb(180, 200, 25), "CO\u{2082} (yellow-green)");
                                }
                                FluidOverlay::Smoke => {
                                    grad(ui, &[
                                        (egui::Color32::from_rgb(0, 0, 0), "None"),
                                        (egui::Color32::from_rgb(200, 50, 0), "Low density"),
                                        (egui::Color32::from_rgb(255, 200, 0), "Medium"),
                                        (egui::Color32::from_rgb(255, 255, 255), "High density"),
                                    ]);
                                }
                                FluidOverlay::O2 => {
                                    dot(ui, egui::Color32::from_rgb(25, 100, 255), "High O\u{2082}");
                                    dot(ui, egui::Color32::from_rgb(230, 25, 0), "Low O\u{2082}");
                                }
                                FluidOverlay::CO2 => {
                                    dot(ui, egui::Color32::from_rgb(180, 200, 25), "High CO\u{2082}");
                                    dot(ui, egui::Color32::from_rgb(40, 40, 40), "Low CO\u{2082}");
                                }
                                FluidOverlay::Temp => {
                                    grad(ui, &[
                                        (egui::Color32::from_rgb(38, 0, 102), "< -15\u{b0}C"),
                                        (egui::Color32::from_rgb(0, 25, 178), "0\u{b0}C"),
                                        (egui::Color32::from_rgb(178, 217, 178), "15-25\u{b0}C"),
                                        (egui::Color32::from_rgb(255, 217, 76), "30-40\u{b0}C"),
                                        (egui::Color32::from_rgb(255, 115, 25), "50-60\u{b0}C"),
                                        (egui::Color32::from_rgb(217, 25, 25), "80-100\u{b0}C"),
                                        (egui::Color32::from_rgb(255, 255, 153), "> 200\u{b0}C"),
                                    ]);
                                }
                                FluidOverlay::HeatFlow => {
                                    dot(ui, egui::Color32::from_rgb(255, 100, 50), "Hot flow");
                                    dot(ui, egui::Color32::from_rgb(50, 100, 255), "Cold flow");
                                    dot(ui, egui::Color32::from_rgb(180, 180, 180), "Neutral");
                                }
                                FluidOverlay::Velocity => {
                                    grad(ui, &[
                                        (egui::Color32::from_rgb(25, 38, 76), "Still"),
                                        (egui::Color32::from_rgb(50, 130, 200), "Slow"),
                                        (egui::Color32::from_rgb(100, 217, 255), "Fast"),
                                    ]);
                                    dot(ui, egui::Color32::WHITE, "Arrow = direction");
                                }
                                FluidOverlay::Pressure => {
                                    grad(ui, &[
                                        (egui::Color32::from_rgb(38, 64, 204), "Negative"),
                                        (egui::Color32::from_rgb(128, 128, 140), "Neutral"),
                                        (egui::Color32::from_rgb(217, 51, 38), "Positive"),
                                    ]);
                                }
                                FluidOverlay::Power => {
                                    ui.label(egui::RichText::new("Voltage").size(10.0).strong());
                                    grad(ui, &[
                                        (egui::Color32::from_rgb(25, 76, 25), "Low voltage"),
                                        (egui::Color32::from_rgb(50, 200, 50), "Normal"),
                                        (egui::Color32::from_rgb(230, 200, 25), "High load"),
                                        (egui::Color32::from_rgb(255, 50, 25), "Overload"),
                                    ]);
                                }
                                FluidOverlay::PowerAmps => {
                                    ui.label(egui::RichText::new("Current (Amps)").size(10.0).strong());
                                    grad(ui, &[
                                        (egui::Color32::from_rgb(15, 15, 30), "No current"),
                                        (egui::Color32::from_rgb(40, 80, 180), "Low"),
                                        (egui::Color32::from_rgb(100, 200, 255), "Medium"),
                                        (egui::Color32::from_rgb(255, 255, 220), "High"),
                                    ]);
                                }
                                FluidOverlay::PowerWatts => {
                                    ui.label(egui::RichText::new("Power (Watts)").size(10.0).strong());
                                    grad(ui, &[
                                        (egui::Color32::from_rgb(50, 200, 50), "Generating"),
                                        (egui::Color32::from_rgb(60, 60, 60), "Idle"),
                                        (egui::Color32::from_rgb(255, 100, 50), "Consuming"),
                                    ]);
                                }
                                _ => {}
                            }
                            // Pipe overlay legend (can be active alongside fluid overlays)
                            if self.show_pipe_overlay {
                                if self.fluid_overlay != FluidOverlay::None {
                                    ui.separator();
                                }
                                ui.label(egui::RichText::new("Pipes").strong().size(10.0));
                                dot(ui, egui::Color32::from_rgb(50, 100, 230), "O\u{2082} rich");
                                dot(ui, egui::Color32::from_rgb(200, 180, 25), "CO\u{2082}");
                                dot(ui, egui::Color32::from_rgb(128, 128, 128), "Smoke");
                                dot(ui, egui::Color32::from_rgb(230, 50, 30), "Hot gas");
                                ui.label(egui::RichText::new("Brighter = more pressure").size(9.0).weak());
                            }
                        });
                    });
            }
        }

        // Version label below layers menu
        egui::Area::new(egui::Id::new("version_label"))
            .anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0])
            .interactable(false)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new(format!("v{} | {:.0} fps", include_str!("../VERSION").trim(), self.fps_display)).color(egui::Color32::from_rgba_premultiplied(200, 200, 200, 180)).size(12.0));
            });

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
            egui::menu::bar(ui, |ui| {
                // Time menu
                let day_frac = time_val / DAY_DURATION;
                let hours = (day_frac * 24.0) as u32;
                let minutes = ((day_frac * 24.0 - hours as f32) * 60.0) as u32;
                let phase = if day_frac < 0.15 { "Night" }
                    else if day_frac < 0.25 { "Dawn" }
                    else if day_frac < 0.75 { "Day" }
                    else if day_frac < 0.85 { "Dusk" }
                    else { "Night" };
                let time_label = format!("{:02}:{:02} {}", hours, minutes, phase);
                ui.menu_button(time_label, |ui| {
                    ui.add(egui::Slider::new(&mut time_val, 0.0..=DAY_DURATION)
                        .text("Time").show_value(false));
                    ui.horizontal(|ui| {
                        if ui.button(if paused { "Play" } else { "Pause" }).clicked() { paused = !paused; }
                        ui.add(egui::Slider::new(&mut speed, 0.1..=5.0).text("Speed").logarithmic(true));
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Night").clicked()  { time_val = DAY_DURATION * 0.0; paused = true; self.camera.force_refresh = 5.0; }
                        if ui.button("Dawn").clicked()   { time_val = DAY_DURATION * 0.18; paused = true; self.camera.force_refresh = 5.0; }
                        if ui.button("Day").clicked()    { time_val = DAY_DURATION * 0.5; paused = true; self.camera.force_refresh = 5.0; }
                        if ui.button("Dusk").clicked()   { time_val = DAY_DURATION * 0.82; paused = true; self.camera.force_refresh = 5.0; }
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
                    if ui.button("Clear").clicked() { self.weather = WeatherState::Clear; self.weather_timer = 45.0; }
                    if ui.button("Cloudy").clicked() { self.weather = WeatherState::Cloudy; self.weather_timer = 45.0; }
                    if ui.button("Light Rain").clicked() { self.weather = WeatherState::LightRain; self.weather_timer = 45.0; }
                    if ui.button("Heavy Rain").clicked() { self.weather = WeatherState::HeavyRain; self.weather_timer = 45.0; }
                });

                ui.separator();

                // Lighting menu
                ui.menu_button("Lighting", |ui| {
                    ui.add(egui::Slider::new(&mut glass_light, 0.0..=0.5).text("Window glow").step_by(0.01));
                    ui.add(egui::Slider::new(&mut indoor_glow, 0.0..=1.0).text("Indoor glow").step_by(0.01));
                    ui.add(egui::Slider::new(&mut bleed, 0.0..=2.0).text("Light bleed").step_by(0.01));
                    ui.separator();
                    ui.label("Foliage Shadows");
                    ui.add(egui::Slider::new(&mut foliage_opacity, 0.0..=1.0).text("Canopy density").step_by(0.01));
                    ui.add(egui::Slider::new(&mut foliage_variation, 0.0..=1.0).text("Tree variation").step_by(0.01));
                });

                // Fluid menu
                ui.menu_button("Fluid", |ui| {
                    let mut fluid_spd = self.fluid_speed;
                    ui.add(egui::Slider::new(&mut fluid_spd, 0.0..=5.0).text("Fluid speed").step_by(0.1));
                    self.fluid_speed = fluid_spd;
                    ui.horizontal(|ui| {
                        ui.label("Wind:");
                        let mut wx = self.fluid_params.wind_x;
                        let mut wy = self.fluid_params.wind_y;
                        ui.add(egui::Slider::new(&mut wx, -20.0..=20.0).text("X").step_by(0.5));
                        ui.add(egui::Slider::new(&mut wy, -20.0..=20.0).text("Y").step_by(0.5));
                        self.fluid_params.wind_x = wx;
                        self.fluid_params.wind_y = wy;
                    });
                    let mut sr = self.fluid_params.smoke_rate;
                    ui.add(egui::Slider::new(&mut sr, 0.0..=1.0).text("Smoke rate").step_by(0.05));
                    self.fluid_params.smoke_rate = sr;
                    let mut fs = self.fluid_params.fan_speed;
                    ui.add(egui::Slider::new(&mut fs, 0.0..=50.0).text("Fan speed").step_by(1.0));
                    self.fluid_params.fan_speed = fs;
                    ui.add(egui::Slider::new(&mut self.pipe_width, 1.0..=20.0).text("Pipe width").step_by(0.5));
                });

                // Camera menu
                ui.menu_button("Camera", |ui| {
                    let zoom_pct = zoom / base_zoom * 100.0;
                    ui.label(format!("Zoom: {:.0}%", zoom_pct));
                    ui.add(egui::Slider::new(&mut zoom, base_zoom * 0.05..=base_zoom * 8.0)
                        .text("Zoom").show_value(false).logarithmic(true));
                    if ui.button("Reset zoom").clicked() { zoom = base_zoom; }
                    ui.separator();
                    ui.add(egui::Slider::new(&mut oblique, 0.0..=0.3).text("Wall face tilt").step_by(0.005));
                });

                // Admin menu (colonist placement)
                ui.separator();
                // Render menu (glow, bleed, temporal)
                ui.menu_button("Render", |ui| {
                    let mut rs = self.render_scale;
                    ui.add(egui::Slider::new(&mut rs, 0.15..=1.0).text("Quality").step_by(0.05));
                    self.render_scale = rs;
                    ui.separator();
                    if ui.selectable_label(self.enable_prox_glow, "Proximity Glow").clicked() {
                        self.enable_prox_glow = !self.enable_prox_glow;
                    }
                    if ui.selectable_label(self.enable_dir_bleed, "Light Bleed").clicked() {
                        self.enable_dir_bleed = !self.enable_dir_bleed;
                    }
                    if ui.selectable_label(self.enable_temporal, "Temporal AA").clicked() {
                        self.enable_temporal = !self.enable_temporal;
                        self.camera.force_refresh = 10.0;
                    }
                });

                // Debug menu
                ui.menu_button("Debug", |ui| {
                    if ui.selectable_label(self.enable_ricochets, "Bullet Ricochets").clicked() {
                        self.enable_ricochets = !self.enable_ricochets;
                    }
                });

                ui.separator();
                ui.menu_button("Admin", |ui| {
                    let pleb_label = format!("Add Colonist ({}/{})", self.plebs.len(), MAX_PLEBS);
                    if ui.button(pleb_label).clicked() {
                        self.placing_pleb = !self.placing_pleb;
                        if self.placing_pleb { self.build_tool = BuildTool::None; }
                        ui.close_menu();
                    }
                    if self.placing_pleb {
                        ui.label(egui::RichText::new("Click to place").weak().size(10.0));
                    }
                });
            });
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
        // --- Inventory window (RPG-style, toggle with I key or click pleb name) ---
        if self.show_inventory {
            if let Some(sel_idx) = self.selected_pleb {
                if let Some(pleb) = self.plebs.get(sel_idx) {
                    let pleb_name = pleb.name.clone();
                    let carrying = pleb.inventory.carrying;
                    let rocks = pleb.inventory.rocks;
                    let berries = pleb.inventory.berries;
                    let health = pleb.needs.health;
                    let hunger = pleb.needs.hunger;
                    let rest = pleb.needs.rest;
                    let warmth = pleb.needs.warmth;
                    let oxygen = pleb.needs.oxygen;
                    let mood = pleb.needs.mood;
                    let mood_l = mood_label(mood);
                    let a = &pleb.appearance;
                    let shirt = [a.shirt_r, a.shirt_g, a.shirt_b];
                    let skin = [a.skin_r, a.skin_g, a.skin_b];
                    let hair = [a.hair_r, a.hair_g, a.hair_b];

                    let mut close_inv = false;
                    egui::Window::new("Inventory")
                        .collapsible(false)
                        .resizable(false)
                        .default_width(200.0)
                        .anchor(egui::Align2::RIGHT_TOP, [-10.0, 40.0])
                        .show(ctx, |ui| {
                            // Portrait header
                            ui.horizontal(|ui| {
                                // Mini portrait
                                let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(36.0, 44.0), egui::Sense::hover());
                                let painter = ui.painter_at(rect);
                                painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(35, 38, 45));
                                let center = rect.center();
                                let shirt_c = egui::Color32::from_rgb((shirt[0]*255.0) as u8, (shirt[1]*255.0) as u8, (shirt[2]*255.0) as u8);
                                let skin_c = egui::Color32::from_rgb((skin[0]*255.0) as u8, (skin[1]*255.0) as u8, (skin[2]*255.0) as u8);
                                let hair_c = egui::Color32::from_rgb((hair[0]*255.0) as u8, (hair[1]*255.0) as u8, (hair[2]*255.0) as u8);
                                painter.circle_filled(center + egui::Vec2::new(0.0, 6.0), 10.0, shirt_c);
                                painter.circle_filled(center + egui::Vec2::new(0.0, -4.0), 7.0, skin_c);
                                painter.circle_filled(center + egui::Vec2::new(0.0, -10.0), 4.0, hair_c);

                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new(&pleb_name).strong().size(14.0));
                                    ui.label(egui::RichText::new(mood_l).size(10.0).color(
                                        if mood > 20.0 { egui::Color32::from_rgb(100, 200, 100) }
                                        else if mood > -20.0 { egui::Color32::from_rgb(180, 180, 120) }
                                        else { egui::Color32::from_rgb(200, 80, 80) }
                                    ));
                                });
                            });

                            ui.separator();

                            // Stats bars
                            let bar = |ui: &mut egui::Ui, label: &str, val: f32, color: egui::Color32| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(label).size(10.0).monospace());
                                    let bar_w = 120.0;
                                    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(bar_w, 10.0), egui::Sense::hover());
                                    let painter = ui.painter_at(rect);
                                    painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(30, 30, 30));
                                    painter.rect_filled(
                                        egui::Rect::from_min_size(rect.min, egui::Vec2::new(bar_w * val.clamp(0.0, 1.0), 10.0)),
                                        2.0, color,
                                    );
                                    ui.label(egui::RichText::new(format!("{:.0}%", val * 100.0)).size(9.0).weak());
                                });
                            };
                            bar(ui, "HP  ", health, egui::Color32::from_rgb(200, 60, 60));
                            bar(ui, "Food", hunger, egui::Color32::from_rgb(200, 160, 40));
                            bar(ui, "Rest", rest, egui::Color32::from_rgb(80, 120, 200));
                            bar(ui, "Warm", warmth, egui::Color32::from_rgb(200, 100, 40));
                            bar(ui, "O2  ", oxygen, egui::Color32::from_rgb(100, 200, 220));

                            ui.separator();
                            ui.label(egui::RichText::new("Inventory").strong().size(11.0));

                            // Carrying slot
                            ui.horizontal(|ui| {
                                let (slot, _) = ui.allocate_exact_size(egui::Vec2::splat(28.0), egui::Sense::hover());
                                let painter = ui.painter_at(slot);
                                painter.rect_filled(slot, 3.0, egui::Color32::from_rgb(45, 48, 55));
                                painter.rect_stroke(slot, 3.0, egui::Stroke::new(1.0, egui::Color32::from_gray(80)), egui::StrokeKind::Outside);
                                if carrying.is_some() {
                                    // Draw rock icon
                                    painter.circle_filled(slot.center(), 8.0, egui::Color32::from_rgb(90, 85, 78));
                                    painter.circle_filled(slot.center() + egui::Vec2::new(-2.0, -2.0), 3.0, egui::Color32::from_rgb(110, 105, 98));
                                }
                                ui.vertical(|ui| {
                                    ui.label(egui::RichText::new("Hands").size(9.0).weak());
                                    if let Some(c) = carrying {
                                        ui.label(egui::RichText::new(c).size(10.0));
                                    } else {
                                        ui.label(egui::RichText::new("Empty").size(10.0).weak());
                                    }
                                });
                            });

                            // Inventory items
                            if berries > 0 || rocks > 0 {
                                ui.add_space(4.0);
                                if rocks > 0 {
                                    ui.horizontal(|ui| {
                                        let (slot, _) = ui.allocate_exact_size(egui::Vec2::splat(22.0), egui::Sense::hover());
                                        let painter = ui.painter_at(slot);
                                        painter.rect_filled(slot, 2.0, egui::Color32::from_rgb(40, 42, 48));
                                        painter.circle_filled(slot.center(), 6.0, egui::Color32::from_rgb(80, 76, 70));
                                        ui.label(egui::RichText::new(format!("Rock x{}", rocks)).size(10.0));
                                    });
                                }
                                if berries > 0 {
                                    ui.horizontal(|ui| {
                                        let (slot, _) = ui.allocate_exact_size(egui::Vec2::splat(22.0), egui::Sense::hover());
                                        let painter = ui.painter_at(slot);
                                        painter.rect_filled(slot, 2.0, egui::Color32::from_rgb(40, 42, 48));
                                        painter.circle_filled(slot.center(), 6.0, egui::Color32::from_rgb(180, 40, 60));
                                        ui.label(egui::RichText::new(format!("Berry x{}", berries)).size(10.0));
                                    });
                                }
                            } else if carrying.is_none() {
                                ui.label(egui::RichText::new("No items").weak().size(10.0));
                            }

                            ui.separator();
                            if ui.small_button("Close").clicked() {
                                close_inv = true;
                            }
                        });
                    if close_inv {
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

    fn draw_build_bar(&mut self, ctx: &egui::Context) {
        // --- Build categories (bottom bar, horizontal, Rimworld-style) ---
        let cat_s = 18.0; // category font size
        egui::Area::new(egui::Id::new("build_categories"))
            .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
            .show(ctx, |ui| {
                egui::Frame::window(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let categories = [
                            ("Walls", "\u{1f9f1}"), ("Floor", "\u{2b1c}"), ("Build", "\u{1f527}"),
                            ("Opening", "\u{1f6aa}"), ("Piping", "\u{1f529}"), ("Vent", "\u{1f4a8}"), ("Power", "\u{26a1}"), ("Physics", "\u{1f4e6}"),
                        ];
                        for &(name, icon) in &categories {
                            let selected = self.build_category == Some(name);
                            let label = format!("{} {}", icon, name);
                            if ui.selectable_label(selected, egui::RichText::new(label).size(cat_s)).clicked() {
                                if selected {
                                    self.build_category = None;
                                    self.build_tool = BuildTool::None;
                                } else {
                                    self.build_category = Some(name);
                                }
                            }
                        }
                        ui.separator();
                        if ui.selectable_label(self.build_tool == BuildTool::Destroy, egui::RichText::new("\u{274c} Destroy").size(cat_s)).clicked() {
                            self.build_tool = if self.build_tool == BuildTool::Destroy { BuildTool::None } else { BuildTool::Destroy };
                            self.build_category = None;
                        }
                    });
                });
            });

        // --- Build items panel (horizontal squares flowing right, above categories) ---
        if let Some(cat) = self.build_category {
            egui::Area::new(egui::Id::new("build_items"))
                .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -55.0])
                .show(ctx, |ui| {
                    egui::Frame::window(ui.style()).show(ui, |ui| {
                        let tool = &mut self.build_tool;
                        let tile_size = 60.0;
                        let icon_s = 24.0;
                        let label_s = 11.0;

                        // Square icon button helper
                        let mut icon_btn = |ui: &mut egui::Ui, t: BuildTool, icon: &str, label: &str| {
                            let selected = *tool == t;
                            let (rect, response) = ui.allocate_exact_size(
                                egui::Vec2::splat(tile_size), egui::Sense::click(),
                            );
                            let painter = ui.painter_at(rect);
                            let bg = if selected {
                                egui::Color32::from_rgb(60, 80, 110)
                            } else if response.hovered() {
                                egui::Color32::from_rgb(55, 58, 65)
                            } else {
                                egui::Color32::from_rgb(40, 42, 48)
                            };
                            painter.rect_filled(rect, 4.0, bg);
                            painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_gray(70)), egui::StrokeKind::Outside);
                            painter.text(rect.center() + egui::Vec2::new(0.0, -6.0), egui::Align2::CENTER_CENTER,
                                icon, egui::FontId::proportional(icon_s), egui::Color32::WHITE);
                            painter.text(rect.center() + egui::Vec2::new(0.0, 14.0), egui::Align2::CENTER_CENTER,
                                label, egui::FontId::proportional(label_s), egui::Color32::from_gray(190));
                            if response.clicked() {
                                *tool = if *tool == t { BuildTool::None } else { t };
                            }
                        };

                        // Items flow horizontally
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::Vec2::new(4.0, 4.0);
                            match cat {
                                "Walls" => {
                                    icon_btn(ui, BuildTool::Place(21), "\u{1fab5}", "Wood");
                                    icon_btn(ui, BuildTool::Place(22), "\u{2699}", "Steel");
                                    icon_btn(ui, BuildTool::Place(23), "\u{1faa8}", "Sandstone");
                                    icon_btn(ui, BuildTool::Place(24), "\u{26f0}", "Granite");
                                    icon_btn(ui, BuildTool::Place(25), "\u{1f532}", "Limestone");
                                    icon_btn(ui, BuildTool::Place(35), "\u{1f3da}", "Mud");
                                    icon_btn(ui, BuildTool::Place(44), "\u{25e3}", "Diagonal");
                                }
                                "Floor" => {
                                    icon_btn(ui, BuildTool::Place(26), "\u{1fab5}", "Wood");
                                    icon_btn(ui, BuildTool::Place(27), "\u{2b1b}", "Stone");
                                    icon_btn(ui, BuildTool::Place(28), "\u{2b1c}", "Concrete");
                                    icon_btn(ui, BuildTool::Roof, "\u{1f3e0}", "Roof");
                                    icon_btn(ui, BuildTool::RemoveFloor, "\u{274c}", "Rm Floor");
                                    icon_btn(ui, BuildTool::RemoveRoof, "\u{274c}", "Rm Roof");
                                }
                                "Build" => {
                                    icon_btn(ui, BuildTool::Place(6), "\u{1f525}", "Fire");
                                    icon_btn(ui, BuildTool::Place(9), "\u{1fa91}", "Bench");
                                    icon_btn(ui, BuildTool::Place(30), "\u{1f6cf}", "Bed");
                                    icon_btn(ui, BuildTool::Place(33), "\u{1f4e6}", "Crate");
                                    icon_btn(ui, BuildTool::Place(13), "\u{267b}", "Compost");
                                    icon_btn(ui, BuildTool::Place(31), "\u{1fad0}", "Berries");
                                    icon_btn(ui, BuildTool::Place(29), "\u{1f4a5}", "Cannon");
                                    icon_btn(ui, BuildTool::Dig, "\u{26cf}", "Dig");
                                }
                                "Opening" => {
                                    icon_btn(ui, BuildTool::Window, "\u{1fa9f}", "Window");
                                    icon_btn(ui, BuildTool::Door, "\u{1f6aa}", "Door");
                                }
                                "Piping" => {
                                    icon_btn(ui, BuildTool::Place(15), "\u{1f4a7}", "Pipe");
                                    icon_btn(ui, BuildTool::Place(16), "\u{2699}", "Pump");
                                    icon_btn(ui, BuildTool::Place(17), "\u{1f6e2}", "Tank");
                                    icon_btn(ui, BuildTool::Place(18), "\u{1f504}", "Valve");
                                    icon_btn(ui, BuildTool::Place(19), "\u{27a1}", "Outlet");
                                    icon_btn(ui, BuildTool::Place(20), "\u{2b05}", "Inlet");
                                }
                                "Power" => {
                                    icon_btn(ui, BuildTool::Place(36), "\u{26a1}", "Wire");
                                    icon_btn(ui, BuildTool::Place(37), "\u{2600}", "Solar");
                                    icon_btn(ui, BuildTool::Place(38), "\u{1f50b}", "Bat S");
                                    icon_btn(ui, BuildTool::Place(39), "\u{1f50b}", "Bat M");
                                    icon_btn(ui, BuildTool::Place(40), "\u{1f50b}", "Bat L");
                                    icon_btn(ui, BuildTool::Place(41), "\u{1f300}", "Wind");
                                    icon_btn(ui, BuildTool::Place(42), "\u{1f518}", "Switch");
                                    icon_btn(ui, BuildTool::Place(43), "\u{1f39a}", "Dimmer");
                                    icon_btn(ui, BuildTool::Place(7), "\u{1f4a1}", "Ceiling");
                                    icon_btn(ui, BuildTool::Place(10), "\u{1f9f4}", "Floor Lamp");
                                    icon_btn(ui, BuildTool::Place(11), "\u{1f4a1}", "Table");
                                }
                                "Vent" => {
                                    icon_btn(ui, BuildTool::Place(12), "\u{1f4a8}", "Fan");
                                }
                                "Physics" => {
                                    icon_btn(ui, BuildTool::WoodBox, "\u{1f4e6}", "Box");
                                }
                                _ => {}
                            }
                        });
                        // Hint bar below icons
                        let tool = &self.build_tool;
                        if *tool != BuildTool::None {
                            ui.separator();
                            let hint = match tool {
                                BuildTool::Place(9) | BuildTool::Place(30) | BuildTool::Place(39) => { let r = if self.build_rotation == 0 { "H" } else { "V" }; format!("Q/E [{}]", r) }
                                BuildTool::Place(41) => { let d = if self.build_rotation % 2 == 0 { "N↔S wind" } else { "E↔W wind" }; format!("Q/E [{}]", d) }
                                BuildTool::Place(11) => "On bench".to_string(),
                                BuildTool::Place(12) | BuildTool::Place(16) | BuildTool::Place(20) | BuildTool::Place(19) | BuildTool::Place(29) => {
                                    let d = match self.build_rotation { 0=>"N", 1=>"E", 2=>"S", _=>"W" };
                                    format!("Q/E [{}]", d)
                                }
                                BuildTool::Destroy | BuildTool::RemoveFloor | BuildTool::RemoveRoof => "Click/drag".to_string(),
                                BuildTool::WoodBox => "Click to drop".to_string(),
                                BuildTool::Window | BuildTool::Door => "Click wall".to_string(),
                                BuildTool::Roof => "Drag (needs support)".to_string(),
                                BuildTool::Dig => "Click to dig 20%".to_string(),
                                _ => "Click/drag".to_string(),
                            };
                            ui.label(egui::RichText::new(hint).weak().size(13.0));
                        }
                    });
                });
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
                rest: f32,
                warmth: f32,
                oxygen: f32,
                mood: f32,
                mood_label: &'static str,
                breath_pct: f32,            // 0-1, breath remaining
                breathing_label: &'static str,
                breathing_state: BreathingState,
                air_o2: f32,
                air_co2: f32,
                activity: String,
                berries: u32,
                rocks: u32,
                carrying: Option<&'static str>,
                is_crisis: bool,
                crisis_reason: Option<&'static str>,
            }
            let pleb_display: Vec<PlebDisplay> = self.plebs.iter().enumerate().map(|(i, p)| {
                let a = &p.appearance;
                PlebDisplay {
                    idx: i,
                    name: p.name.clone(),
                    shirt: [a.shirt_r, a.shirt_g, a.shirt_b],
                    skin: [a.skin_r, a.skin_g, a.skin_b],
                    hair: [a.hair_r, a.hair_g, a.hair_b],
                    health: p.needs.health,
                    hunger: p.needs.hunger,
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
                            PlebActivity::Walking => "Walking".to_string(),
                            PlebActivity::Sleeping => "Sleeping".to_string(),
                            PlebActivity::Harvesting(pr) => format!("Harvesting {:.0}%", pr * 100.0),
                            PlebActivity::Eating => "Eating".to_string(),
                            PlebActivity::Hauling => "Hauling".to_string(),
                            PlebActivity::Crisis(_, _) => "Crisis".to_string(),
                        };
                        if let Some(reason) = p.activity.crisis_reason() {
                            format!("{} ({})", act_str, reason)
                        } else {
                            act_str
                        }
                    },
                    berries: p.inventory.berries,
                    rocks: p.inventory.rocks,
                    carrying: p.inventory.carrying,
                    is_crisis: p.activity.is_crisis(),
                    crisis_reason: p.activity.crisis_reason(),
                }
            }).collect();

            egui::Area::new(egui::Id::new("colonist_bar"))
                .anchor(egui::Align2::CENTER_TOP, [0.0, 10.0])
                .show(ctx, |ui| {
                    egui::Frame::window(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for pd in &pleb_display {
                                let is_sel = self.selected_pleb == Some(pd.idx);
                                let card_w = if is_sel { 110.0 } else { 48.0 };
                                let card_h = if is_sel { 90.0 } else { 56.0 };
                                let (rect, response) = ui.allocate_exact_size(egui::Vec2::new(card_w, card_h), egui::Sense::click());
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
                                    painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(180, 30, 30));
                                    painter.rect_filled(inset, 3.0, bg);
                                }

                                // Portrait area
                                let portrait_center = if is_sel {
                                    rect.left_center() + egui::Vec2::new(20.0, -6.0)
                                } else {
                                    rect.center() + egui::Vec2::new(0.0, -4.0)
                                };

                                // Body (shirt color)
                                let shirt_c = egui::Color32::from_rgb(
                                    (pd.shirt[0] * 255.0) as u8, (pd.shirt[1] * 255.0) as u8, (pd.shirt[2] * 255.0) as u8);
                                painter.circle_filled(portrait_center + egui::Vec2::new(0.0, 8.0), 10.0, shirt_c);

                                // Head (skin color)
                                let skin_c = egui::Color32::from_rgb(
                                    (pd.skin[0] * 255.0) as u8, (pd.skin[1] * 255.0) as u8, (pd.skin[2] * 255.0) as u8);
                                painter.circle_filled(portrait_center + egui::Vec2::new(0.0, -2.0), 6.0, skin_c);

                                // Hair
                                let hair_c = egui::Color32::from_rgb(
                                    (pd.hair[0] * 255.0) as u8, (pd.hair[1] * 255.0) as u8, (pd.hair[2] * 255.0) as u8);
                                painter.circle_filled(portrait_center + egui::Vec2::new(0.0, -6.0), 4.0, hair_c);

                                // Name
                                let name_pos = if is_sel {
                                    rect.left_bottom() + egui::Vec2::new(20.0, -2.0)
                                } else {
                                    rect.center_bottom() + egui::Vec2::new(0.0, -2.0)
                                };
                                painter.text(
                                    name_pos,
                                    egui::Align2::CENTER_BOTTOM,
                                    &pd.name,
                                    egui::FontId::proportional(8.0),
                                    egui::Color32::WHITE,
                                );

                                // Health bar (real value)
                                let bar_y = if is_sel { rect.min.y + 4.0 } else { rect.max.y - 5.0 };
                                let bar_x = rect.min.x + 2.0;
                                let bar_w = if is_sel { 36.0 } else { rect.width() - 4.0 };
                                let bar_rect = egui::Rect::from_min_size(
                                    egui::Pos2::new(bar_x, bar_y),
                                    egui::Vec2::new(bar_w, 2.0),
                                );
                                painter.rect_filled(bar_rect, 1.0, egui::Color32::from_rgb(40, 40, 40));
                                let health_color = if pd.health > 0.5 {
                                    egui::Color32::from_rgb(80, 200, 80)
                                } else if pd.health > 0.25 {
                                    egui::Color32::from_rgb(200, 200, 40)
                                } else {
                                    egui::Color32::from_rgb(200, 40, 40)
                                };
                                painter.rect_filled(
                                    egui::Rect::from_min_size(bar_rect.min, egui::Vec2::new(bar_w * pd.health, 2.0)),
                                    1.0, health_color,
                                );

                                // Expanded needs display when selected
                                if is_sel {
                                    let needs_x = rect.min.x + 40.0;
                                    let needs_y = rect.min.y + 8.0;
                                    let bar_h = 3.0;
                                    let spacing = 8.5;
                                    let need_w = 62.0;

                                    let needs_data: [(&str, f32, egui::Color32); 5] = [
                                        ("HUN", pd.hunger, egui::Color32::from_rgb(200, 160, 40)),
                                        ("RST", pd.rest, egui::Color32::from_rgb(80, 120, 200)),
                                        ("WRM", pd.warmth, egui::Color32::from_rgb(200, 100, 40)),
                                        ("O2", pd.oxygen, egui::Color32::from_rgb(100, 200, 220)),
                                        ("BRE", pd.breath_pct, egui::Color32::from_rgb(150, 180, 255)),
                                    ];

                                    for (i, (label, val, color)) in needs_data.iter().enumerate() {
                                        let y = needs_y + i as f32 * spacing;
                                        // Label — flash red for critical
                                        let label_color = if *val < 0.2 {
                                            egui::Color32::from_rgb(255, 80, 80)
                                        } else {
                                            egui::Color32::GRAY
                                        };
                                        painter.text(
                                            egui::Pos2::new(needs_x, y),
                                            egui::Align2::LEFT_TOP,
                                            *label,
                                            egui::FontId::proportional(7.0),
                                            label_color,
                                        );
                                        // Bar background
                                        let br = egui::Rect::from_min_size(
                                            egui::Pos2::new(needs_x + 20.0, y + 1.0),
                                            egui::Vec2::new(need_w - 20.0, bar_h),
                                        );
                                        painter.rect_filled(br, 1.0, egui::Color32::from_rgb(30, 30, 30));
                                        // Bar fill
                                        painter.rect_filled(
                                            egui::Rect::from_min_size(br.min, egui::Vec2::new((need_w - 20.0) * val.clamp(0.0, 1.0), bar_h)),
                                            1.0, *color,
                                        );
                                    }

                                    // Breathing state label (flashing when critical)
                                    let breath_y = needs_y + 5.0 * spacing;
                                    let breath_color = match pd.breathing_state {
                                        BreathingState::Normal => egui::Color32::from_rgb(120, 180, 120),
                                        BreathingState::HoldingBreath => egui::Color32::from_rgb(220, 200, 80),
                                        BreathingState::Gasping => egui::Color32::from_rgb(255, 60, 60),
                                    };
                                    painter.text(
                                        egui::Pos2::new(needs_x, breath_y),
                                        egui::Align2::LEFT_TOP,
                                        pd.breathing_label,
                                        egui::FontId::proportional(7.0),
                                        breath_color,
                                    );

                                    // Activity + inventory line
                                    let info_y = breath_y + 9.0;
                                    let mut inv_parts = Vec::new();
                                    if pd.berries > 0 { inv_parts.push(format!("{}x Berry", pd.berries)); }
                                    if pd.rocks > 0 { inv_parts.push(format!("{}x Rock", pd.rocks)); }
                                    if let Some(c) = pd.carrying { inv_parts.push(format!("[{}]", c)); }
                                    let berry_str = if !inv_parts.is_empty() {
                                        format!("{} | {}", pd.activity, inv_parts.join(" "))
                                    } else {
                                        pd.activity.clone()
                                    };
                                    painter.text(
                                        egui::Pos2::new(needs_x, info_y),
                                        egui::Align2::LEFT_TOP,
                                        &berry_str,
                                        egui::FontId::proportional(6.5),
                                        egui::Color32::from_rgb(160, 160, 140),
                                    );

                                    // Mood label at bottom-right of card
                                    let mood_color = if pd.mood > 20.0 {
                                        egui::Color32::from_rgb(100, 200, 100)
                                    } else if pd.mood > -20.0 {
                                        egui::Color32::from_rgb(180, 180, 120)
                                    } else {
                                        egui::Color32::from_rgb(200, 80, 80)
                                    };
                                    painter.text(
                                        egui::Pos2::new(rect.max.x - 4.0, rect.max.y - 4.0),
                                        egui::Align2::RIGHT_BOTTOM,
                                        pd.mood_label,
                                        egui::FontId::proportional(7.0),
                                        mood_color,
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

    }

    fn draw_context_menus(&mut self, ctx: &egui::Context, bp_ppp: f32, bp_cam: (f32,f32,f32,f32,f32)) {
        // --- Context menu (right-click on selected pleb) ---
        if let Some((mx, my)) = self.context_menu {
            if let Some(sel_idx) = self.selected_pleb {
                if let Some(pleb) = self.plebs.get(sel_idx) {
                    let has_berries = pleb.inventory.berries > 0;
                    let is_carrying = pleb.inventory.carrying.is_some();
                    let carrying_label = pleb.inventory.carrying.unwrap_or("").to_string();
                    let is_sleeping = *pleb.activity.inner() == PlebActivity::Sleeping;
                    let in_crisis = pleb.activity.is_crisis();
                    let hunger = pleb.needs.hunger;
                    let rest = pleb.needs.rest;
                    let pleb_name = pleb.name.clone();

                    let mut close_menu = false;
                    egui::Area::new(egui::Id::new("pleb_context_menu"))
                        .fixed_pos(egui::Pos2::new(mx / bp_ppp, my / bp_ppp))
                        .show(ctx, |ui| {
                            egui::Frame::menu(ui.style()).show(ui, |ui| {
                                ui.label(egui::RichText::new(&pleb_name).strong().size(11.0));
                                if in_crisis {
                                    ui.label(egui::RichText::new("(Crisis - cannot give orders)")
                                        .size(9.0).color(egui::Color32::from_rgb(255, 80, 80)));
                                }
                                ui.separator();

                                if in_crisis {
                                    ui.label(egui::RichText::new("Actions unavailable").size(10.0).color(egui::Color32::GRAY));
                                } else if has_berries {
                                    let label = format!("Eat Berry ({:.0}% hunger)", hunger * 100.0);
                                    if ui.button(egui::RichText::new(label).size(10.0)).clicked() {
                                        if let Some(p) = self.plebs.get_mut(sel_idx) {
                                            p.activity = PlebActivity::Eating;
                                        }
                                        close_menu = true;
                                    }
                                } else {
                                    ui.label(egui::RichText::new("No food").size(10.0).color(egui::Color32::GRAY));
                                }

                                ui.separator();

                                if is_sleeping {
                                    if ui.button(egui::RichText::new("Wake up").size(10.0)).clicked() {
                                        if let Some(p) = self.plebs.get_mut(sel_idx) {
                                            p.activity = PlebActivity::Idle;
                                        }
                                        close_menu = true;
                                    }
                                } else {
                                    let sleep_label = format!("Sleep ({:.0}% rest)", rest * 100.0);
                                    if ui.button(egui::RichText::new(sleep_label).size(10.0)).clicked() {
                                        if let Some(p) = self.plebs.get_mut(sel_idx) {
                                            // Check if near bed
                                            let day_frac = self.time_of_day / DAY_DURATION;
                                            let env = sample_environment(&self.grid_data, p.x, p.y, day_frac);
                                            if env.near_bed {
                                                p.activity = PlebActivity::Sleeping;
                                                p.path.clear();
                                                p.path_idx = 0;
                                            } else if let Some((bx, by)) = env.nearest_bed {
                                                let start = (p.x.floor() as i32, p.y.floor() as i32);
                                                let path = astar_path(&self.grid_data, start, (bx, by));
                                                if !path.is_empty() {
                                                    p.path = path;
                                                    p.path_idx = 0;
                                                    p.activity = PlebActivity::Walking;
                                                }
                                            }
                                        }
                                        close_menu = true;
                                    }
                                }

                                if ui.button(egui::RichText::new("Harvest berries").size(10.0)).clicked() {
                                    if let Some(p) = self.plebs.get_mut(sel_idx) {
                                        let day_frac = self.time_of_day / DAY_DURATION;
                                        let env = sample_environment(&self.grid_data, p.x, p.y, day_frac);
                                        if env.near_berry_bush {
                                            if let Some((bx, by)) = env.nearest_berry_bush {
                                                p.harvest_target = Some((bx, by));
                                                p.activity = PlebActivity::Harvesting(0.0);
                                                p.path.clear();
                                                p.path_idx = 0;
                                            }
                                        } else if let Some((bx, by)) = env.nearest_berry_bush {
                                            let start = (p.x.floor() as i32, p.y.floor() as i32);
                                            let path = astar_path(&self.grid_data, start, (bx, by));
                                            if !path.is_empty() {
                                                p.path = path;
                                                p.path_idx = 0;
                                                p.activity = PlebActivity::Walking;
                                            }
                                        }
                                    }
                                    close_menu = true;
                                }

                                // Carrying actions
                                if is_carrying && !in_crisis {
                                    ui.separator();
                                    ui.label(egui::RichText::new(format!("Carrying: {}", carrying_label)).size(10.0));
                                    // Drop item here
                                    if ui.button(egui::RichText::new("Drop here").size(10.0)).clicked() {
                                        if let Some(p) = self.plebs.get_mut(sel_idx) {
                                            if p.inventory.carrying == Some("Rock") {
                                                // Place rock at pleb's feet
                                                let rx = p.x.floor() as i32;
                                                let ry = p.y.floor() as i32;
                                                if rx >= 0 && ry >= 0 && rx < GRID_W as i32 && ry < GRID_H as i32 {
                                                    let ridx = (ry as u32 * GRID_W + rx as u32) as usize;
                                                    let rb = self.grid_data[ridx];
                                                    let rbt = rb & 0xFF;
                                                    if rbt == 2 || rbt == 0 { // dirt or air = empty ground
                                                        let roof_bits = rb & 0xFF000000;
                                                        let flag_bits = (rb >> 16) & 2;
                                                        self.grid_data[ridx] = make_block(34, 0, flag_bits as u8) | roof_bits;
                                                        self.grid_dirty = true;
                                                        p.inventory.rocks = p.inventory.rocks.saturating_sub(1);
                                                    }
                                                }
                                                p.inventory.carrying = None;
                                            }
                                            p.activity = PlebActivity::Idle;
                                            p.haul_target = None;
                                            p.path.clear();
                                            p.path_idx = 0;
                                        }
                                        close_menu = true;
                                    }
                                    // Haul to nearest storage
                                    if ui.button(egui::RichText::new("Haul to storage").size(10.0)).clicked() {
                                        if let Some(p) = self.plebs.get_mut(sel_idx) {
                                            let px = p.x.floor() as i32;
                                            let py = p.y.floor() as i32;
                                            if let Some((cx, cy)) = find_nearest_crate(&self.grid_data, px, py) {
                                                let adj = adjacent_walkable(&self.grid_data, cx, cy).unwrap_or((cx, cy));
                                                let start = (px, py);
                                                let path = astar_path(&self.grid_data, start, adj);
                                                if !path.is_empty() {
                                                    p.path = path;
                                                    p.path_idx = 0;
                                                    p.activity = PlebActivity::Hauling;
                                                    p.haul_target = Some((cx, cy));
                                                    p.harvest_target = None; // already carrying
                                                }
                                            }
                                        }
                                        close_menu = true;
                                    }
                                }

                                ui.separator();
                                if ui.button(egui::RichText::new("Cancel").size(10.0)).clicked() {
                                    close_menu = true;
                                }
                            });
                        });
                    if close_menu {
                        self.context_menu = None;
                    }
                } else {
                    self.context_menu = None;
                }
            } else {
                self.context_menu = None;
            }
        }

        // Close context menu when clicking anywhere (pointer not over menu)
        if self.context_menu.is_some() {
            let pointer_over_ui = ctx.is_pointer_over_area();
            let any_click = ctx.input(|i| i.pointer.any_pressed());
            if any_click && !pointer_over_ui {
                self.context_menu = None;
            }
        }

        // --- Rock context menu (right-click or ctrl-click on a rock) ---
        if let Some((mx, my, rx, ry)) = self.rock_context_menu {
            let mut close_rock_menu = false;
            // Verify rock still exists
            let rock_valid = if rx >= 0 && ry >= 0 && rx < GRID_W as i32 && ry < GRID_H as i32 {
                (self.grid_data[(ry as u32 * GRID_W + rx as u32) as usize] & 0xFF) == 34
            } else { false };
            if rock_valid {
                egui::Area::new(egui::Id::new("rock_context_menu"))
                    .fixed_pos(egui::Pos2::new(mx / bp_ppp, my / bp_ppp))
                    .show(ctx, |ui| {
                        egui::Frame::menu(ui.style()).show(ui, |ui| {
                            ui.label(egui::RichText::new("Rock").strong().size(12.0));
                            ui.separator();
                            // Find nearest pleb that can haul
                            let mut best_pleb: Option<(usize, f32)> = None;
                            for (i, p) in self.plebs.iter().enumerate() {
                                if p.activity.is_crisis() || p.inventory.carrying.is_some() { continue; }
                                let dist = ((p.x - rx as f32 - 0.5).powi(2) + (p.y - ry as f32 - 0.5).powi(2)).sqrt();
                                if best_pleb.is_none() || dist < best_pleb.unwrap().1 {
                                    best_pleb = Some((i, dist));
                                }
                            }
                            // Find nearest crate (wide search)
                            let nearest_crate = find_nearest_crate(&self.grid_data, rx, ry);
                            if let Some((pleb_idx, _)) = best_pleb {
                                let pleb_name = self.plebs[pleb_idx].name.clone();
                                if let Some((cx, cy)) = nearest_crate {
                                    let label = format!("Haul to storage ({})", pleb_name);
                                    if ui.button(egui::RichText::new(label).size(11.0)).clicked() {
                                        let pleb = &mut self.plebs[pleb_idx];
                                        let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                                        let path = astar_path(&self.grid_data, start, (rx, ry));
                                        if !path.is_empty() {
                                            pleb.path = path;
                                            pleb.path_idx = 0;
                                            pleb.activity = PlebActivity::Hauling;
                                            pleb.haul_target = Some((cx, cy));
                                            pleb.harvest_target = Some((rx, ry));
                                            self.selected_pleb = Some(pleb_idx);
                                        }
                                        close_rock_menu = true;
                                    }
                                } else {
                                    ui.label(egui::RichText::new("No storage crate on map").weak().size(10.0));
                                }
                            } else {
                                ui.label(egui::RichText::new("No colonist available").weak().size(10.0));
                            }
                            ui.separator();
                            if ui.button(egui::RichText::new("Cancel").size(10.0)).clicked() {
                                close_rock_menu = true;
                            }
                        });
                    });
            } else {
                close_rock_menu = true;
            }
            if close_rock_menu {
                self.rock_context_menu = None;
            }
        }
        // Close rock menu when clicking anywhere (pointer not over menu)
        if self.rock_context_menu.is_some() {
            let pointer_over_ui = ctx.is_pointer_over_area();
            let any_click = ctx.input(|i| i.pointer.any_pressed());
            if any_click && !pointer_over_ui {
                self.rock_context_menu = None;
            }
        }


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
                    let (resp, painter) = ui.allocate_painter(egui::Vec2::splat(size), egui::Sense::hover());
                    let center = resp.rect.center();
                    let radius = size * 0.45;
                    // Circle background
                    painter.circle_filled(center, radius, egui::Color32::from_rgba_unmultiplied(30, 30, 40, 200));
                    painter.circle_stroke(center, radius, egui::Stroke::new(1.0, egui::Color32::from_gray(100)));
                    // NSEW labels
                    let label_color = egui::Color32::from_gray(130);
                    let label_font = egui::FontId::proportional(9.0);
                    let label_r = radius + 1.0;
                    painter.text(center + egui::Vec2::new(0.0, -label_r), egui::Align2::CENTER_BOTTOM, "N", label_font.clone(), label_color);
                    painter.text(center + egui::Vec2::new(0.0, label_r), egui::Align2::CENTER_TOP, "S", label_font.clone(), label_color);
                    painter.text(center + egui::Vec2::new(label_r, 0.0), egui::Align2::LEFT_CENTER, "E", label_font.clone(), label_color);
                    painter.text(center + egui::Vec2::new(-label_r, 0.0), egui::Align2::RIGHT_CENTER, "W", label_font, label_color);
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
                        let arrow_len = (radius - 6.0) * (wind_mag / 20.0).min(1.0).max(0.3);
                        let tip = center + egui::Vec2::new(dir_x * arrow_len, dir_y * arrow_len);
                        let tail = center - egui::Vec2::new(dir_x * arrow_len * 0.3, dir_y * arrow_len * 0.3);
                        painter.line_segment([tail, tip], egui::Stroke::new(2.0, egui::Color32::from_rgb(200, 220, 255)));
                        // Arrowhead
                        let perp = egui::Vec2::new(-dir_y, dir_x) * arrow_len * 0.25;
                        let head_base = center + egui::Vec2::new(dir_x * arrow_len * 0.55, dir_y * arrow_len * 0.55);
                        painter.add(egui::Shape::convex_polygon(
                            vec![tip, head_base + perp, head_base - perp],
                            egui::Color32::from_rgb(200, 220, 255),
                            egui::Stroke::NONE,
                        ));
                    } else {
                        painter.text(center, egui::Align2::CENTER_CENTER, "·", egui::FontId::proportional(14.0), egui::Color32::from_gray(150));
                    }
                    // Wind speed label below compass
                    painter.text(
                        resp.rect.center_bottom() + egui::Vec2::new(0.0, 10.0),
                        egui::Align2::CENTER_TOP,
                        format!("{:.0}", wind_mag),
                        egui::FontId::proportional(9.0),
                        egui::Color32::from_gray(150),
                    );
                });
        }

        // Info tool: hold Shift to inspect any block
        let shift_held = self.pressed_keys.contains(&KeyCode::ShiftLeft)
            || self.pressed_keys.contains(&KeyCode::ShiftRight);
        if shift_held {
            let (wx, wy) = self.hover_world;
            let bx = wx.floor() as i32;
            let by = wy.floor() as i32;

            let mut block_info = String::from("OOB");
            if bx >= 0 && by >= 0 && bx < GRID_W as i32 && by < GRID_H as i32 {
                let idx = (by as u32 * GRID_W + bx as u32) as usize;
                let block = self.grid_data[idx];
                let bt = block & 0xFF;
                let bh = (block >> 8) & 0xFF;
                let flags = (block >> 16) & 0xFF;
                let reg = block_defs::BlockRegistry::cached();
                let type_name = reg.name(bt as u8);
                let mut tags = String::new();
                if flags & 2 != 0 { tags.push_str(" [Roofed]"); }
                if flags & 1 != 0 { tags.push_str(if flags & 4 != 0 { " [Door:Open]" } else { " [Door:Closed]" }); }
                if bh > 0 { block_info = format!("{} (h:{}){}", type_name, bh, tags); }
                else { block_info = format!("{}{}", type_name, tags); }
            }

            #[cfg(not(target_arch = "wasm32"))]
            let gas_info = {
                let [smoke_r, o2, co2, temp] = self.debug.fluid_density;
                // Check if this is a solid block (wall) — show block temp instead of air data
                let (is_solid_wall, is_pipe_block) = if bx >= 0 && by >= 0 && bx < GRID_W as i32 && by < GRID_H as i32 {
                    let ib = self.grid_data[(by as u32 * GRID_W + bx as u32) as usize];
                    let ibt = ib & 0xFF;
                    let ibh = (ib >> 8) & 0xFF;
                    let solid = ibh > 0 && (ibt == 1 || ibt == 4 || ibt == 5 || ibt == 14 || (ibt >= 21 && ibt <= 25) || ibt == 35);
                    let pipe = ibt >= 15 && ibt <= 20;
                    (solid, pipe)
                } else { (false, false) };
                let voltage_str = if self.debug.voltage > 0.01 {
                    let v = self.debug.voltage;
                    // Estimate current from voltage (I = V/R, assume R ≈ 10Ω for display)
                    let r = 10.0f32; // approximate resistance
                    let amps = v / r;
                    let watts = v * amps;
                    format!("\n⚡ {:.1}V | {:.2}A | {:.1}W", v, amps, watts)
                } else { String::new() };
                if is_solid_wall {
                    format!("Smoke: —\nO2: —\nCO2: —\nWall temp: {:.1}°C{}", self.debug.block_temp, voltage_str)
                } else if is_pipe_block {
                    format!("Smoke: —\nO2: —\nCO2: —\nPipe temp: {:.1}°C{}", self.debug.block_temp, voltage_str)
                } else {
                    format!("Smoke: {:.3}\nO2: {:.3}\nCO2: {:.3}\nTemp: {:.1}°C{}", smoke_r, o2, co2, temp, voltage_str)
                }
            };
            #[cfg(target_arch = "wasm32")]
            let gas_info = String::from("(gas readback: native only)");

            // Show pipe state if hovering over a pipe component
            let pipe_info = if bx >= 0 && by >= 0 && bx < GRID_W as i32 && by < GRID_H as i32 {
                let pidx = by as u32 * GRID_W + bx as u32;
                let b = self.grid_data[pidx as usize];
                let pbt = b & 0xFF;
                if pbt >= 15 && pbt <= 20 {
                    if let Some(cell) = self.pipe_network.cells.get(&pidx) {
                        format!("\n--- Pipe ---\nPressure: {:.2}\nSmoke: {:.3}\nO2: {:.3}\nCO2: {:.3}\nTemp: {:.1}°C\nVol: {:.0}",
                            cell.pressure, cell.gas[0], cell.gas[1], cell.gas[2], cell.gas[3], cell.volume)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            // Material thermal properties
            let mat_info = if bx >= 0 && by >= 0 && bx < GRID_W as i32 && by < GRID_H as i32 {
                let bt = self.grid_data[(by as u32 * GRID_W + bx as u32) as usize] & 0xFF;
                let mats = crate::materials::build_material_table();
                if (bt as usize) < mats.len() {
                    let m = &mats[bt as usize];
                    if m.heat_capacity > 0.0 || m.conductivity > 0.0 {
                        format!("\nHeat cap: {:.1} | Cond: {:.3}", m.heat_capacity, m.conductivity)
                    } else { String::new() }
                } else { String::new() }
            } else { String::new() };

            let tip = format!(
                "\u{1f4cd} ({:.1}, {:.1})\n{}\n{}{}{}",
                wx, wy, block_info, gas_info, mat_info, pipe_info
            );

            // Position tooltip near cursor
            let cursor_screen = ctx.input(|i| {
                i.pointer.hover_pos().unwrap_or(egui::Pos2::ZERO)
            });
            egui::Area::new(egui::Id::new("info_tooltip"))
                .fixed_pos(cursor_screen + egui::Vec2::new(15.0, 15.0))
                .interactable(false)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.label(egui::RichText::new("\u{1f50d} Info").strong().size(13.0));
                        ui.label(egui::RichText::new(tip).monospace().size(11.0));
                    });
                });
        }

        // Drag shape preview (walls=hollow rect, pipes=line, destroy=filled rect)
        if let Some((sx, sy)) = self.drag_start {
            if self.mouse_dragged {
                let (hwx, hwy) = self.hover_world;
                let (ex, ey) = (hwx.floor() as i32, hwy.floor() as i32);
                let tiles = match self.build_tool {
                    BuildTool::Destroy | BuildTool::Roof | BuildTool::RemoveFloor | BuildTool::RemoveRoof => {
                        Self::filled_rect_tiles(sx, sy, ex, ey)
                    }
                    BuildTool::Place(id) => {
                        let reg = crate::block_defs::BlockRegistry::cached();
                        let shape = reg.get(id).and_then(|d| d.placement.as_ref()).and_then(|p| p.drag.as_ref());
                        match shape {
                            Some(crate::block_defs::DragShape::Line) => Self::line_tiles(sx, sy, ex, ey),
                            Some(crate::block_defs::DragShape::FilledRect) => Self::filled_rect_tiles(sx, sy, ex, ey),
                            Some(crate::block_defs::DragShape::HollowRect) => Self::hollow_rect_tiles(sx, sy, ex, ey),
                            _ => Vec::new(),
                        }
                    }
                    _ => Vec::new(),
                };
                if !tiles.is_empty() {
                    let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
                    let painter = ctx.layer_painter(egui::LayerId::new(
                        egui::Order::Foreground, egui::Id::new("drag_preview"),
                    ));
                    let is_destroy = self.build_tool == BuildTool::Destroy;
                    let is_remove_floor = self.build_tool == BuildTool::RemoveFloor;
                    let is_remove_roof = self.build_tool == BuildTool::RemoveRoof;
                    let is_roof = self.build_tool == BuildTool::Roof;
                    for (tx, ty) in &tiles {
                        let color = if is_destroy {
                            egui::Color32::from_rgba_unmultiplied(255, 50, 50, 100)
                        } else if is_remove_floor {
                            let valid = if *tx < 0 || *ty < 0 || *tx >= GRID_W as i32 || *ty >= GRID_H as i32 {
                                false
                            } else {
                                let tidx = (*ty as u32 * GRID_W + *tx as u32) as usize;
                                let tb = self.grid_data[tidx];
                                let tbt = tb & 0xFF;
                                matches!(tbt, 26 | 27 | 28)
                            };
                            if valid {
                                egui::Color32::from_rgba_unmultiplied(255, 50, 50, 100)
                            } else {
                                egui::Color32::from_rgba_unmultiplied(255, 60, 60, 40)
                            }
                        } else if is_remove_roof {
                            let valid = if *tx < 0 || *ty < 0 || *tx >= GRID_W as i32 || *ty >= GRID_H as i32 {
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
                            if Self::can_support_roof(&self.grid_data, *tx, *ty) {
                                egui::Color32::from_rgba_unmultiplied(100, 160, 255, 100)
                            } else {
                                egui::Color32::from_rgba_unmultiplied(255, 60, 60, 80)
                            }
                        } else {
                            // Validate each tile individually
                            let is_wire_tool = matches!(self.build_tool, BuildTool::Place(36));
                            let is_pipe_tool = matches!(self.build_tool, BuildTool::Place(15));
                            let valid = if *tx < 0 || *ty < 0 || *tx >= GRID_W as i32 || *ty >= GRID_H as i32 {
                                false
                            } else if is_wire_tool {
                                true // wire can go anywhere
                            } else {
                                let tidx = (*ty as u32 * GRID_W + *tx as u32) as usize;
                                let tb = self.grid_data[tidx];
                                let tbt = tb & 0xFF;
                                let tbh = (tb >> 8) & 0xFF;
                                // Allow placement on empty ground OR on existing same-type block
                                ((tbt == 0 || tbt == 2) && tbh == 0)
                                    || (is_pipe_tool && tbt == 15) // pipe on pipe = merge connections
                            };
                            if valid {
                                egui::Color32::from_rgba_unmultiplied(80, 180, 255, 80)
                            } else {
                                egui::Color32::from_rgba_unmultiplied(255, 50, 50, 100)
                            }
                        };
                        let wx0 = *tx as f32;
                        let wy0 = *ty as f32;
                        let sx0 = ((wx0 - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                        let sy0 = ((wy0 - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                        let sx1 = ((wx0 + 1.0 - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                        let sy1 = ((wy0 + 1.0 - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                        painter.rect_filled(
                            egui::Rect::from_min_max(egui::pos2(sx0, sy0), egui::pos2(sx1, sy1)),
                            0.0, color,
                        );
                    }
                    // Draw direction arrows on pipe/wire line tiles
                    let is_line = matches!(self.build_tool, BuildTool::Place(15) | BuildTool::Place(36));
                    if is_line && tiles.len() > 1 {
                        let arrow_col = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 160);
                        for ti in 0..tiles.len() {
                            let (tx, ty) = tiles[ti];
                            let wx0 = tx as f32;
                            let wy0 = ty as f32;
                            let sx0 = ((wx0 - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                            let sy0 = ((wy0 - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                            let sx1 = ((wx0 + 1.0 - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                            let sy1 = ((wy0 + 1.0 - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
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
                                    arrow_col, egui::Stroke::NONE,
                                ));
                            }
                        }
                    }
                }
            }
        }

    }

    fn draw_world_overlays(&mut self, ctx: &egui::Context, bp_cam: (f32,f32,f32,f32,f32), blueprint_tiles: &[((i32,i32), bool)]) {
        let bp_ppp = self.ppp();
        // Blueprint preview — draw ghost overlay for placement
        if !blueprint_tiles.is_empty() {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;

            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("blueprint"),
            ));

            for &((tx, ty), valid) in blueprint_tiles {
                let color = if valid {
                    egui::Color32::from_rgba_unmultiplied(80, 180, 255, 80)
                } else {
                    egui::Color32::from_rgba_unmultiplied(255, 60, 60, 80)
                };

                let wx0 = tx as f32;
                let wy0 = ty as f32;
                // World → physical pixels → logical points (egui coords)
                let sx0 = ((wx0 - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy0 = ((wy0 - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                let sx1 = ((wx0 + 1.0 - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy1 = ((wy0 + 1.0 - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;

                // Diagonal wall: draw triangle showing which half is solid
                if self.build_tool == BuildTool::Place(44) {
                    // During drag, use per-tile variant from diag_preview
                    let variant = self.diag_preview.iter()
                        .find(|&&(dx, dy, _)| dx == tx && dy == ty)
                        .map(|&(_, _, v)| v as u32)
                        .unwrap_or(self.build_rotation % 4);
                    let tl = egui::pos2(sx0, sy0);
                    let tr = egui::pos2(sx1, sy0);
                    let bl = egui::pos2(sx0, sy1);
                    let br = egui::pos2(sx1, sy1);
                    let tri = match variant {
                        0 => vec![tr, bl, br], // / solid below-right
                        1 => vec![tl, bl, br], // \ solid below-left
                        2 => vec![tl, tr, bl], // / solid above-left
                        _ => vec![tl, tr, br], // \ solid above-right
                    };
                    painter.add(egui::Shape::convex_polygon(tri, color, egui::Stroke::NONE));
                } else {
                    painter.rect_filled(
                        egui::Rect::from_min_max(egui::pos2(sx0, sy0), egui::pos2(sx1, sy1)),
                        0.0,
                        color,
                    );
                }

                // Wind turbine: show wind direction arrows across the 2x2 area (first tile only)
                if self.build_tool == BuildTool::Place(41) && tx == blueprint_tiles[0].0.0 && ty == blueprint_tiles[0].0.1 {
                    // Draw arrows showing wind direction through the turbine
                    let center = egui::pos2(
                        (sx0 + sx1) / 2.0 + (sx1 - sx0) * 0.5,
                        (sy0 + sy1) / 2.0 + (sy1 - sy0) * 0.5,
                    );
                    let tile_size = (sx1 - sx0).max(1.0);
                    let (adx, ady) = if self.build_rotation % 2 == 0 {
                        (0.0f32, -1.0f32) // N-S wind (blades face E-W)
                    } else {
                        (1.0f32, 0.0f32) // E-W wind (blades face N-S)
                    };
                    // Two arrows flanking the turbine
                    for &offset in &[-0.3f32, 0.3] {
                        let perp_off = egui::Vec2::new(-ady * offset * tile_size * 2.0, adx * offset * tile_size * 2.0);
                        let arrow_center = center + perp_off;
                        let arrow_len = tile_size * 1.5;
                        let tip = arrow_center + egui::Vec2::new(adx * arrow_len * 0.5, ady * arrow_len * 0.5);
                        let tail = arrow_center - egui::Vec2::new(adx * arrow_len * 0.5, ady * arrow_len * 0.5);
                        painter.line_segment([tail, tip], egui::Stroke::new(2.0, egui::Color32::from_rgba_unmultiplied(100, 200, 255, 180)));
                        let perp = egui::Vec2::new(-ady, adx) * arrow_len * 0.15;
                        let head_base = arrow_center + egui::Vec2::new(adx * arrow_len * 0.25, ady * arrow_len * 0.25);
                        painter.add(egui::Shape::convex_polygon(
                            vec![tip, head_base + perp, head_base - perp],
                            egui::Color32::from_rgba_unmultiplied(100, 200, 255, 180), egui::Stroke::NONE,
                        ));
                    }
                }

                // Direction arrow for fan, pump, inlet, outlet
                if matches!(self.build_tool, BuildTool::Place(12) | BuildTool::Place(16) | BuildTool::Place(20) | BuildTool::Place(19)) {
                    let center = egui::pos2((sx0 + sx1) / 2.0, (sy0 + sy1) / 2.0);
                    let tile_size = (sx1 - sx0).max(1.0);
                    let (adx, ady) = match self.build_rotation {
                        0 => (0.0f32, -1.0f32),
                        1 => (1.0, 0.0),
                        2 => (0.0, 1.0),
                        _ => (-1.0, 0.0),
                    };
                    let arrow_len = tile_size * 0.8;
                    let tip = center + egui::Vec2::new(adx * arrow_len * 0.5, ady * arrow_len * 0.5);
                    let tail = center - egui::Vec2::new(adx * arrow_len * 0.5, ady * arrow_len * 0.5);
                    painter.line_segment([tail, tip], egui::Stroke::new(2.0, egui::Color32::WHITE));
                    let perp = egui::Vec2::new(-ady, adx) * arrow_len * 0.2;
                    let head_base = center + egui::Vec2::new(adx * arrow_len * 0.2, ady * arrow_len * 0.2);
                    painter.add(egui::Shape::convex_polygon(
                        vec![tip, head_base + perp, head_base - perp],
                        egui::Color32::WHITE, egui::Stroke::NONE,
                    ));
                }
            }
        }

        // Pleb placement ghost
        if self.placing_pleb {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("pleb_ghost"),
            ));
            let (hwx, hwy) = self.hover_world;
            let valid = is_walkable_pos(&self.grid_data, hwx, hwy);
            let color = if valid {
                egui::Color32::from_rgba_unmultiplied(80, 200, 255, 120)
            } else {
                egui::Color32::from_rgba_unmultiplied(255, 60, 60, 120)
            };
            let center_sx = ((hwx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
            let center_sy = ((hwy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
            let radius = 0.4 * cam_zoom / self.render_scale / bp_ppp;
            painter.circle_filled(egui::pos2(center_sx, center_sy), radius, color);
        }

        // Draw selected pleb A* path line
        let sel_pleb_ref = self.selected_pleb.and_then(|i| self.plebs.get(i));
        if let Some(pleb) = sel_pleb_ref {
            if pleb.path_idx < pleb.path.len() {
                let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Foreground,
                    egui::Id::new("pleb_path"),
                ));
                let to_screen = |wx: f32, wy: f32| -> egui::Pos2 {
                    let sx = ((wx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                    let sy = ((wy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                    egui::pos2(sx, sy)
                };
                // Draw from pleb's current position through remaining path
                let mut prev = to_screen(pleb.x, pleb.y);
                for i in pleb.path_idx..pleb.path.len() {
                    let (px, py) = pleb.path[i];
                    let next = to_screen(px as f32 + 0.5, py as f32 + 0.5);
                    painter.line_segment(
                        [prev, next],
                        egui::Stroke::new(2.0, egui::Color32::from_rgba_unmultiplied(100, 255, 100, 150)),
                    );
                    prev = next;
                }
                // Draw target marker at end
                let last = pleb.path.last().unwrap();
                let end = to_screen(last.0 as f32 + 0.5, last.1 as f32 + 0.5);
                painter.circle_stroke(end, 4.0, egui::Stroke::new(2.0, egui::Color32::from_rgba_unmultiplied(100, 255, 100, 200)));
            }
        }

        // Pipe overlay: draw pipe gas contents as colored blocks
        if self.show_pipe_overlay && !self.pipe_network.cells.is_empty() {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("pipe_overlay"),
            ));
            let to_screen = |wx: f32, wy: f32| -> egui::Pos2 {
                let sx = ((wx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy = ((wy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                egui::pos2(sx, sy)
            };
            for (&idx, cell) in &self.pipe_network.cells {
                let x = (idx % GRID_W) as f32;
                let y = (idx / GRID_W) as f32;
                let p0 = to_screen(x + 0.15, y + 0.15);
                let p1 = to_screen(x + 0.85, y + 0.85);
                // Color by gas content: smoke=gray, O2=blue, CO2=yellow, temp=red
                let smoke = cell.gas[0].min(1.0);
                let o2 = cell.gas[1].min(1.0);
                let co2 = cell.gas[2].min(1.0);
                let temp = ((cell.gas[3] - 15.0) / 100.0).clamp(0.0, 1.0);
                let pres = (cell.pressure / 2.0).clamp(0.0, 1.0);
                // Mix: blue for O2-rich, yellow for CO2, gray for smoke, red for hot
                let r = ((1.0 - o2) * 0.5 + co2 * 0.8 + temp * 0.9 + smoke * 0.5).min(1.0);
                let g = (o2 * 0.4 + co2 * 0.7 + temp * 0.2).min(1.0);
                let b = (o2 * 0.9 + smoke * 0.3).min(1.0);
                let alpha = (0.3 + pres * 0.4).min(0.7);
                let color = egui::Color32::from_rgba_unmultiplied(
                    (r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, (alpha * 255.0) as u8,
                );
                painter.rect_filled(
                    egui::Rect::from_min_max(p0, p1),
                    2.0,
                    color,
                );
                // Pressure indicator: small text
                if cam_zoom > 10.0 { // only show text when zoomed in enough
                    let center = egui::pos2((p0.x + p1.x) / 2.0, (p0.y + p1.y) / 2.0);
                    painter.text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        format!("{:.1}", cell.pressure),
                        egui::FontId::proportional(8.0),
                        egui::Color32::WHITE,
                    );
                }
            }
        }

        // Render cannon barrel direction overlays
        {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let tile_px = cam_zoom / self.render_scale / bp_ppp;
            let cannon_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground, egui::Id::new("cannons"),
            ));
            for (&cannon_idx, &angle) in &self.cannon_angles {
                let cx = (cannon_idx % GRID_W) as f32 + 0.5;
                let cy = (cannon_idx / GRID_W) as f32 + 0.5;
                let sx = ((cx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy = ((cy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                let barrel_len = 0.55 * tile_px;
                let barrel_w = 0.10 * tile_px;
                let end_x = sx + angle.cos() * barrel_len;
                let end_y = sy + angle.sin() * barrel_len;
                let is_selected = self.block_sel.cannon == Some(cannon_idx);
                let barrel_color = if is_selected {
                    egui::Color32::from_rgb(80, 75, 65)
                } else {
                    egui::Color32::from_rgb(55, 52, 48)
                };
                cannon_painter.line_segment(
                    [egui::pos2(sx, sy), egui::pos2(end_x, end_y)],
                    egui::Stroke::new(barrel_w, barrel_color),
                );
                cannon_painter.circle_filled(egui::pos2(end_x, end_y), barrel_w * 0.5, egui::Color32::from_rgb(40, 38, 35));
                cannon_painter.circle_filled(egui::pos2(sx, sy), barrel_w * 0.7, egui::Color32::from_rgb(50, 48, 42));
                if is_selected {
                    cannon_painter.circle_stroke(egui::pos2(sx, sy), barrel_len * 1.1,
                        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(100, 255, 100, 150)));
                }
            }
        }

        // Lightning flash overlay + bolt rendering
        if self.lightning_flash > 0.01 {
            let flash_alpha = (self.lightning_flash * 180.0).min(255.0) as u8;
            let flash_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground, egui::Id::new("lightning_flash"),
            ));
            let screen_rect = egui::Rect::from_min_max(
                egui::pos2(0.0, 0.0),
                egui::pos2(ctx.screen_rect().width(), ctx.screen_rect().height()),
            );
            flash_painter.rect_filled(
                screen_rect, 0.0,
                egui::Color32::from_rgba_unmultiplied(220, 225, 255, flash_alpha),
            );

            // Draw lightning bolt at strike location (use last known strike)
            if let Some((lx, ly)) = self.lightning_strike {
                let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
                let strike_sx = ((lx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let strike_sy = ((ly - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                let bolt_col = egui::Color32::from_rgba_unmultiplied(200, 210, 255, flash_alpha);
                // Jagged bolt from top of screen to strike point
                let top_x = strike_sx + (self.lightning_flash * 20.0).sin() * 30.0;
                let mut prev = egui::pos2(top_x, 0.0);
                let segments = 8;
                for i in 1..=segments {
                    let t = i as f32 / segments as f32;
                    let jitter_x = (t * 17.3 + self.lightning_flash * 7.0).sin() * 15.0 * (1.0 - t);
                    let next = egui::pos2(
                        strike_sx + jitter_x,
                        strike_sy * t,
                    );
                    flash_painter.line_segment([prev, next], egui::Stroke::new(3.0 - t * 2.0, bolt_col));
                    prev = next;
                }
                // Bright circle at impact
                flash_painter.circle_filled(
                    egui::pos2(strike_sx, strike_sy),
                    8.0 * self.lightning_flash,
                    egui::Color32::from_rgba_unmultiplied(255, 255, 240, flash_alpha),
                );
            }
        }

        // Render power cables: squiggly lines from lights/fans to nearest wire
        {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let tile_px = cam_zoom / self.render_scale / bp_ppp;
            if tile_px > 3.0 { // only draw when zoomed in enough
                let cable_painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Foreground, egui::Id::new("power_cables"),
                ));
                let to_screen = |wx: f32, wy: f32| -> egui::Pos2 {
                    let sx = ((wx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                    let sy = ((wy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                    egui::pos2(sx, sy)
                };
                // Scan visible area for consumers (lights, fans) and find nearest wire
                let vx0 = (cam_cx - cam_sw * 0.5 / cam_zoom).floor() as i32 - 1;
                let vy0 = (cam_cy - cam_sh * 0.5 / cam_zoom).floor() as i32 - 1;
                let vx1 = (cam_cx + cam_sw * 0.5 / cam_zoom).ceil() as i32 + 1;
                let vy1 = (cam_cy + cam_sh * 0.5 / cam_zoom).ceil() as i32 + 1;
                for y in vy0.max(0)..vy1.min(GRID_H as i32) {
                    for x in vx0.max(0)..vx1.min(GRID_W as i32) {
                        let idx = (y as u32 * GRID_W + x as u32) as usize;
                        let bt = self.grid_data[idx] & 0xFF;
                        // Consumer blocks that auto-connect: electric light (7), floor lamp (10), fan (12)
                        if bt != 7 && bt != 10 && bt != 12 { continue; }
                        // Search for nearest wire within 3 tiles
                        let mut best_wire: Option<(i32, i32, f32)> = None;
                        for dy in -3i32..=3 {
                            for dx in -3i32..=3 {
                                let wx = x + dx;
                                let wy = y + dy;
                                if wx < 0 || wy < 0 || wx >= GRID_W as i32 || wy >= GRID_H as i32 { continue; }
                                let widx = (wy as u32 * GRID_W + wx as u32) as usize;
                                if (self.grid_data[widx] & 0xFF) == 36 {
                                    let dist = ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt();
                                    if best_wire.is_none() || dist < best_wire.unwrap().2 {
                                        best_wire = Some((wx, wy, dist));
                                    }
                                }
                            }
                        }
                        if let Some((wire_x, wire_y, _)) = best_wire {
                            let from = to_screen(x as f32 + 0.5, y as f32 + 0.5);
                            let to = to_screen(wire_x as f32 + 0.5, wire_y as f32 + 0.5);
                            // Draw squiggly cable: midpoint offset + curve
                            let mid = egui::pos2((from.x + to.x) * 0.5, (from.y + to.y) * 0.5);
                            let perp_x = -(to.y - from.y);
                            let perp_y = to.x - from.x;
                            let perp_len = (perp_x * perp_x + perp_y * perp_y).sqrt().max(1.0);
                            let sag = tile_px * 0.15; // cable sag amount
                            let sag_mid = egui::pos2(mid.x + perp_x / perp_len * sag, mid.y + perp_y / perp_len * sag + sag * 0.5);
                            // Draw as segmented line through sag point
                            let cable_color = egui::Color32::from_rgb(70, 60, 45);
                            cable_painter.line_segment([from, sag_mid], egui::Stroke::new(1.5, cable_color));
                            cable_painter.line_segment([sag_mid, to], egui::Stroke::new(1.5, cable_color));
                            // Small connector dots at endpoints
                            cable_painter.circle_filled(from, 2.0, cable_color);
                            cable_painter.circle_filled(to, 2.0, cable_color);
                        }
                    }
                }
            }
        }

        // Render physics bodies
        if !self.physics_bodies.is_empty() {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("physics_bodies"),
            ));
            let to_screen = |wx: f32, wy: f32| -> (f32, f32) {
                let sx = ((wx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy = ((wy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                (sx, sy)
            };
            let tile_px = cam_zoom / self.render_scale / bp_ppp; // pixels per tile

            for body in &self.physics_bodies {
                let size = body.size;
                let z_offset = body.z * tile_px * 0.4;

                match body.body_type {
                    physics::BodyType::Cannonball => {
                        // --- Cannonball shadow (circle on ground, scales with height) ---
                        let shadow_scale = (1.0 - body.z * 0.15).max(0.2);
                        let shadow_r = 0.12 * shadow_scale * tile_px;
                        let (gx, gy) = to_screen(body.x, body.y);
                        let shadow_alpha = (180.0 * (1.0 - body.z * 0.06).max(0.15)) as u8;
                        painter.circle_filled(
                            egui::pos2(gx, gy),
                            shadow_r,
                            egui::Color32::from_rgba_unmultiplied(15, 15, 15, shadow_alpha),
                        );
                        // Trajectory guide: dotted line from shadow to ball showing height
                        if body.z > 0.3 {
                            let ball_screen = egui::pos2(gx, gy - z_offset);
                            painter.line_segment(
                                [egui::pos2(gx, gy), ball_screen],
                                egui::Stroke::new(0.5, egui::Color32::from_rgba_unmultiplied(100, 100, 100, 80)),
                            );
                        }

                        // --- Cannonball (dark sphere with highlight) ---
                        let ball_r = 0.10 * tile_px;
                        let ball_pos = egui::pos2(gx, gy - z_offset);
                        painter.circle_filled(ball_pos, ball_r, egui::Color32::from_rgb(40, 38, 35));
                        // Specular highlight
                        painter.circle_filled(
                            ball_pos + egui::Vec2::new(-ball_r * 0.3, -ball_r * 0.3),
                            ball_r * 0.35,
                            egui::Color32::from_rgb(90, 88, 82),
                        );
                    }
                    physics::BodyType::WoodBox => {
                        // --- Shadow ---
                        if body.z > 0.05 {
                            let shadow_scale = 1.0 - (body.z * 0.1).min(0.5);
                            let ss = size * shadow_scale;
                            let (gx0, gy0) = to_screen(body.x - ss, body.y - ss * 0.6);
                            let (gx1, gy1) = to_screen(body.x + ss, body.y + ss * 0.6);
                            let shadow_alpha = (150.0 * (1.0 - body.z * 0.08).max(0.2)) as u8;
                            painter.rect_filled(
                                egui::Rect::from_min_max(egui::pos2(gx0, gy0), egui::pos2(gx1, gy1)),
                                tile_px * 0.1,
                                egui::Color32::from_rgba_unmultiplied(20, 20, 20, shadow_alpha),
                            );
                        }

                        // --- Rotated box ---
                        let center = to_screen(body.x, body.y);
                        let center_s = egui::pos2(center.0, center.1 - z_offset);
                        let half = size * tile_px;
                        let cos_z = body.rot_z.cos();
                        let sin_z = body.rot_z.sin();
                        let scale_x = 1.0 - body.rot_y.sin().abs() * 0.3;
                        let scale_y = 1.0 - body.rot_x.sin().abs() * 0.3;
                        let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
                        let rotated: Vec<egui::Pos2> = corners.iter().map(|&(cx, cy)| {
                            let sx = cx * half * scale_x;
                            let sy = cy * half * scale_y;
                            let rx = sx * cos_z - sy * sin_z;
                            let ry = sx * sin_z + sy * cos_z;
                            center_s + egui::Vec2::new(rx, ry)
                        }).collect();
                        let brightness = (160.0 + body.z * 15.0).min(200.0) as u8;
                        let gb = (120.0 + body.z * 10.0).min(160.0) as u8;
                        let fill_color = egui::Color32::from_rgb(brightness, gb, 60);
                        let stroke_color = egui::Color32::from_rgb(100, 75, 35);
                        painter.add(egui::Shape::convex_polygon(
                            rotated.clone(), fill_color, egui::Stroke::new(1.5, stroke_color),
                        ));
                        for i in 0..3 {
                            let t = 0.25 + i as f32 * 0.25;
                            let lx = rotated[0].x + (rotated[3].x - rotated[0].x) * t;
                            let ly = rotated[0].y + (rotated[3].y - rotated[0].y) * t;
                            let rx = rotated[1].x + (rotated[2].x - rotated[1].x) * t;
                            let ry = rotated[1].y + (rotated[2].y - rotated[1].y) * t;
                            painter.line_segment(
                                [egui::pos2(lx, ly), egui::pos2(rx, ry)],
                                egui::Stroke::new(0.5, egui::Color32::from_rgba_unmultiplied(90, 65, 30, 100)),
                            );
                        }
                    }
                    physics::BodyType::Bullet => {
                        // Bullet: tiny bright tracer
                        let (gx, gy) = to_screen(body.x, body.y);
                        let trail_len = 0.3 * tile_px;
                        let speed = (body.vx * body.vx + body.vy * body.vy).sqrt().max(0.001);
                        let dx = -body.vx / speed * trail_len;
                        let dy = -body.vy / speed * trail_len;
                        painter.line_segment(
                            [egui::pos2(gx, gy - z_offset), egui::pos2(gx + dx, gy - z_offset + dy)],
                            egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 240, 150)),
                        );
                    }
                    physics::BodyType::Grenade => {
                        let shadow_scale = (1.0 - body.z * 0.15).max(0.2);
                        let shadow_r = 0.08 * shadow_scale * tile_px;
                        let (gx, gy) = to_screen(body.x, body.y);
                        let shadow_alpha = (150.0 * (1.0 - body.z * 0.06).max(0.15)) as u8;
                        painter.circle_filled(
                            egui::pos2(gx, gy), shadow_r,
                            egui::Color32::from_rgba_unmultiplied(15, 15, 15, shadow_alpha),
                        );
                        let ball_r = 0.07 * tile_px;
                        let ball_pos = egui::pos2(gx, gy - z_offset);
                        painter.circle_filled(ball_pos, ball_r, egui::Color32::from_rgb(40, 60, 30));
                        painter.circle_filled(
                            ball_pos + egui::Vec2::new(-ball_r * 0.2, -ball_r * 0.2),
                            ball_r * 0.3, egui::Color32::from_rgb(70, 90, 50),
                        );
                    }
                }
            }
        }

        // Pump speed slider popup
        if let Some(pump_idx) = self.block_sel.pump {
            let (pwx, pwy) = self.block_sel.pump_world;
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let sx = ((pwx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp + 20.0;
            let sy = ((pwy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp - 10.0;
            let mut still_valid = false;
            egui::Area::new(egui::Id::new("pump_slider"))
                .fixed_pos(egui::pos2(sx, sy))
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        if let Some(cell) = self.pipe_network.cells.get_mut(&pump_idx) {
                            ui.label(egui::RichText::new("Pump").strong().size(11.0));
                            ui.add(egui::Slider::new(&mut cell.pump_rate, 0.0..=20.0)
                                .text("Rate")
                                .step_by(0.5));
                            ui.label(egui::RichText::new(format!("P: {:.1}", cell.pressure)).size(9.0).weak());
                            still_valid = true;
                        }
                        if ui.small_button("Close").clicked() {
                            still_valid = false;
                        }
                    });
                });
            if !still_valid {
                self.block_sel.pump = None;
            }
        }

        // Fan speed slider popup
        if let Some(_fan_idx) = self.block_sel.fan {
            let (fwx, fwy) = self.block_sel.fan_world;
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let sx = ((fwx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp + 20.0;
            let sy = ((fwy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp - 10.0;
            let mut still_valid = true;
            egui::Area::new(egui::Id::new("fan_slider"))
                .fixed_pos(egui::pos2(sx, sy))
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.label(egui::RichText::new("Fan").strong().size(11.0));
                        ui.add(egui::Slider::new(&mut self.fluid_params.fan_speed, 0.0..=80.0)
                            .text("Speed")
                            .step_by(1.0));
                        if ui.small_button("Close").clicked() {
                            still_valid = false;
                        }
                    });
                });
            if !still_valid {
                self.block_sel.fan = None;
            }
        }

        // Dimmer slider popup
        if let Some(dimmer_idx) = self.block_sel.dimmer {
            let (dwx, dwy) = self.block_sel.dimmer_world;
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let sx = ((dwx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp + 20.0;
            let sy = ((dwy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp - 10.0;
            let mut still_valid = true;
            // Read current dimmer level from block height (0-10 = 0-100%)
            let didx = dimmer_idx as usize;
            let dblock = if didx < self.grid_data.len() { self.grid_data[didx] } else { 0 };
            if (dblock & 0xFF) != 43 { still_valid = false; }
            if still_valid {
                let mut level = ((dblock >> 8) & 0xFF) as i32;
                egui::Area::new(egui::Id::new("dimmer_slider"))
                    .fixed_pos(egui::pos2(sx, sy))
                    .show(ctx, |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.label(egui::RichText::new("Dimmer").strong().size(11.0));
                            let pct = level as f32 * 10.0;
                            ui.label(egui::RichText::new(format!("{:.0}%", pct)).size(10.0));
                            ui.add(egui::Slider::new(&mut level, 0..=10)
                                .text("Level")
                                .step_by(1.0));
                            // Write back to block height
                            let new_block = (dblock & 0xFFFF00FF) | ((level as u32 & 0xFF) << 8);
                            if new_block != dblock {
                                self.grid_data[didx] = new_block;
                                self.grid_dirty = true;
                            }
                            if ui.small_button("Close").clicked() {
                                still_valid = false;
                            }
                        });
                    });
            }
            if !still_valid {
                self.block_sel.dimmer = None;
            }
        }

        // Storage crate inspection popup
        if let Some(crate_idx) = self.block_sel.crate_idx {
            let (cwx, cwy) = self.block_sel.crate_world;
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let sx = ((cwx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp + 20.0;
            let sy = ((cwy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp - 10.0;
            let mut still_valid = true;
            // Check the crate still exists
            if (crate_idx as usize) < self.grid_data.len() {
                let cb = self.grid_data[crate_idx as usize];
                if (cb & 0xFF) != 33 { still_valid = false; }
            } else {
                still_valid = false;
            }
            if still_valid {
                let inv = self.crate_contents.get(&crate_idx);
                egui::Area::new(egui::Id::new("crate_popup"))
                    .fixed_pos(egui::pos2(sx, sy))
                    .show(ctx, |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            let total = inv.map(|i| i.total()).unwrap_or(0);
                            ui.label(egui::RichText::new(format!("Storage Crate ({}/{})", total, CRATE_MAX_ITEMS)).strong().size(12.0));
                            ui.separator();
                            if let Some(inv) = inv {
                                let has_items = inv.rocks > 0 || inv.berries > 0;
                                if has_items {
                                    if inv.rocks > 0 {
                                        ui.label(egui::RichText::new(format!("Rocks: {}", inv.rocks)).size(11.0));
                                    }
                                    if inv.berries > 0 {
                                        ui.label(egui::RichText::new(format!("Berries: {}", inv.berries)).size(11.0));
                                    }
                                } else {
                                    ui.label(egui::RichText::new("Empty").weak().size(10.0));
                                }
                            } else {
                                ui.label(egui::RichText::new("Empty").weak().size(10.0));
                            }
                            ui.separator();
                            if ui.small_button("Close").clicked() {
                                still_valid = false;
                            }
                        });
                    });
            }
            if !still_valid {
                self.block_sel.crate_idx = None;
            }
        }

    }

    fn draw_world_labels(&mut self, ctx: &egui::Context, bp_cam: (f32,f32,f32,f32,f32)) {
        // --- World labels: pleb names, activity, key items ---
        {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let ppp = self.ppp();
            let tile_px = cam_zoom / self.render_scale / ppp;

            // Only show labels when zoomed in enough
            if tile_px > 6.0 {
                let to_screen = |wx: f32, wy: f32| -> egui::Pos2 {
                    let sx = ((wx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / ppp;
                    let sy = ((wy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / ppp;
                    egui::pos2(sx, sy)
                };

                let label_painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Foreground, egui::Id::new("world_labels"),
                ));

                // Pleb name + activity labels
                for pleb in &self.plebs {
                    let pos = to_screen(pleb.x, pleb.y + 0.7);

                    // Name label (always visible)
                    let name_color = if pleb.is_enemy {
                        egui::Color32::from_rgb(255, 50, 50)
                    } else if pleb.activity.is_crisis() {
                        egui::Color32::from_rgb(255, 80, 80)
                    } else {
                        egui::Color32::from_rgb(220, 220, 220)
                    };
                    label_painter.text(
                        pos,
                        egui::Align2::CENTER_TOP,
                        &pleb.name,
                        egui::FontId::proportional(9.0),
                        name_color,
                    );

                    // Activity label (when not idle and zoomed in enough)
                    if tile_px > 12.0 {
                        let inner = pleb.activity.inner();
                        let act_text = match inner {
                            PlebActivity::Idle => None,
                            PlebActivity::Walking => None, // too noisy
                            PlebActivity::Sleeping => Some("Zzz..."),
                            PlebActivity::Harvesting(_) => Some("Harvesting"),
                            PlebActivity::Eating => Some("Eating"),
                            PlebActivity::Hauling => Some("Hauling"),
                            PlebActivity::Crisis(_, _) => None,
                        };
                        if let Some(text) = act_text {
                            let act_pos = to_screen(pleb.x, pleb.y + 0.95);
                            label_painter.text(
                                act_pos,
                                egui::Align2::CENTER_TOP,
                                text,
                                egui::FontId::proportional(7.0),
                                egui::Color32::from_rgb(180, 180, 140),
                            );
                        }
                        // Crisis reason
                        if let Some(reason) = pleb.activity.crisis_reason() {
                            let crisis_pos = to_screen(pleb.x, pleb.y + 0.95);
                            label_painter.text(
                                crisis_pos,
                                egui::Align2::CENTER_TOP,
                                reason,
                                egui::FontId::proportional(8.0),
                                egui::Color32::from_rgb(255, 60, 60),
                            );
                        }
                    }
                }

                // Fire mode indicator above selected pleb
                if let Some(pleb) = self.selected_pleb.and_then(|i| self.plebs.get(i)) {
                    if !pleb.is_enemy {
                        let mode_pos = to_screen(pleb.x, pleb.y - 0.8);
                        let mode_text = if self.burst_mode { "BURST" } else { "SINGLE" };
                        label_painter.text(
                            mode_pos, egui::Align2::CENTER_BOTTOM, mode_text,
                            egui::FontId::proportional(7.0),
                            egui::Color32::from_rgb(180, 180, 100),
                        );
                    }
                }

                // Grenade charge bar above selected pleb
                if self.grenade_charging {
                    if let Some(pleb) = self.selected_pleb.and_then(|i| self.plebs.get(i)) {
                        let bar_pos = to_screen(pleb.x - 0.4, pleb.y - 0.6);
                        let bar_w = tile_px * 0.8;
                        let bar_h = tile_px * 0.08;
                        let charge = self.grenade_charge.clamp(0.0, 1.0);
                        // Background
                        label_painter.rect_filled(
                            egui::Rect::from_min_size(bar_pos, egui::Vec2::new(bar_w, bar_h)),
                            1.0, egui::Color32::from_rgb(30, 30, 30),
                        );
                        // Fill (green to red as charge increases)
                        let r = (charge * 255.0) as u8;
                        let g = ((1.0 - charge) * 200.0) as u8;
                        label_painter.rect_filled(
                            egui::Rect::from_min_size(bar_pos, egui::Vec2::new(bar_w * charge, bar_h)),
                            1.0, egui::Color32::from_rgb(r, g, 40),
                        );
                    }
                }
            }
        }
    }
}
