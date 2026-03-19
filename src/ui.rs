//! UI drawing — all egui panels, overlays, debug tooltips.
//! Extracted from render() to keep main.rs manageable.

use crate::*;

impl App {
    pub fn draw_ui(&mut self, ctx: &egui::Context, bp_cam: (f32,f32,f32,f32,f32), blueprint_tiles: Vec<((i32,i32), bool)>, dt: f32) {
        let bp_ppp = self.window.as_ref().map(|w| w.scale_factor() as f32).unwrap_or(1.0);
        // Version label in top-right corner — kept here for now
        egui::Area::new(egui::Id::new("version_label"))
            .anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0])
            .show(ctx, |ui| {
                ui.label(egui::RichText::new(format!("v{} | {:.0} fps", include_str!("../VERSION").trim(), self.fps_display)).color(egui::Color32::from_rgba_premultiplied(200, 200, 200, 180)).size(14.0));
            });

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
        egui::Window::new("Controls")
            .default_pos([10.0, 10.0])
            .default_width(300.0)
            .resizable(false)
            .show(ctx, |ui| {
                // Time of day as hours:minutes for display
                let day_frac = time_val / DAY_DURATION;
                let hours = (day_frac * 24.0) as u32;
                let minutes = ((day_frac * 24.0 - hours as f32) * 60.0) as u32;

                // Determine phase
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

                ui.label(format!("{:02}:{:02} - {}", hours, minutes, phase));
                ui.add(egui::Slider::new(&mut time_val, 0.0..=DAY_DURATION)
                    .text("Time")
                    .show_value(false));
                ui.horizontal(|ui| {
                    if ui.button(if paused { "Play" } else { "Pause" }).clicked() {
                        paused = !paused;
                    }
                    ui.add(egui::Slider::new(&mut speed, 0.1..=5.0)
                        .text("Speed")
                        .logarithmic(true));
                });
                ui.horizontal(|ui| {
                    if ui.button("Night").clicked()  { time_val = DAY_DURATION * 0.0; paused = true; self.camera.force_refresh = 5.0; }
                    if ui.button("Dawn").clicked()   { time_val = DAY_DURATION * 0.18; paused = true; self.camera.force_refresh = 5.0; }
                    if ui.button("Day").clicked()    { time_val = DAY_DURATION * 0.5; paused = true; self.camera.force_refresh = 5.0; }
                    if ui.button("Dusk").clicked()   { time_val = DAY_DURATION * 0.82; paused = true; self.camera.force_refresh = 5.0; }
                });

                ui.separator();

                let zoom_pct = zoom / base_zoom * 100.0;
                ui.label(format!("Zoom: {:.0}%", zoom_pct));
                ui.add(egui::Slider::new(&mut zoom, base_zoom * 0.05..=base_zoom * 8.0)
                    .text("Zoom")
                    .show_value(false)
                    .logarithmic(true));
                if ui.button("Reset zoom").clicked() {
                    zoom = base_zoom;
                }
                let mut rs = self.render_scale;
                ui.add(egui::Slider::new(&mut rs, 0.15..=1.0)
                    .text("Render quality")
                    .step_by(0.05));
                self.render_scale = rs;

                ui.separator();
                ui.label("Lighting");
                ui.add(egui::Slider::new(&mut glass_light, 0.0..=0.5)
                    .text("Window glow")
                    .step_by(0.01));
                ui.add(egui::Slider::new(&mut indoor_glow, 0.0..=1.0)
                    .text("Indoor glow")
                    .step_by(0.01));
                ui.add(egui::Slider::new(&mut bleed, 0.0..=2.0)
                    .text("Light bleed")
                    .step_by(0.01));

                ui.separator();
                ui.label("Foliage Shadows");
                ui.add(egui::Slider::new(&mut foliage_opacity, 0.0..=1.0)
                    .text("Canopy density")
                    .step_by(0.01));
                ui.add(egui::Slider::new(&mut foliage_variation, 0.0..=1.0)
                    .text("Tree variation")
                    .step_by(0.01));

                ui.separator();
                ui.label("Fluid Sim");
                let mut fluid_spd = self.fluid_speed;
                ui.add(egui::Slider::new(&mut fluid_spd, 0.0..=5.0)
                    .text("Fluid speed")
                    .step_by(0.1));
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
                ui.add(egui::Slider::new(&mut sr, 0.0..=1.0)
                    .text("Smoke rate")
                    .step_by(0.05));
                self.fluid_params.smoke_rate = sr;
                let mut fs = self.fluid_params.fan_speed;
                ui.add(egui::Slider::new(&mut fs, 0.0..=50.0)
                    .text("Fan speed")
                    .step_by(1.0));
                self.fluid_params.fan_speed = fs;
                ui.add(egui::Slider::new(&mut self.pipe_width, 1.0..=20.0)
                    .text("Pipe width")
                    .step_by(0.5));

                ui.separator();
                ui.label("Camera");
                ui.add(egui::Slider::new(&mut oblique, 0.0..=0.3)
                    .text("Wall face tilt")
                    .step_by(0.005));
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

        // --- Build categories (left bottom, Rimworld-style) ---
        egui::Area::new(egui::Id::new("build_categories"))
            .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -20.0])
            .show(ctx, |ui| {
                egui::Frame::window(ui.style()).show(ui, |ui| {
                    ui.set_max_width(65.0);
                    let s = 11.0;
                    let categories = [
                        ("Walls", "\u{1f9f1}"), ("Floor", "\u{2b1c}"), ("Build", "\u{1f527}"),
                        ("Opening", "\u{1f6aa}"), ("Piping", "\u{1f529}"), ("Physics", "\u{1f4e6}"),
                    ];
                    for &(name, icon) in &categories {
                        let selected = self.build_category == Some(name);
                        let label = format!("{} {}", icon, name);
                        if ui.selectable_label(selected, egui::RichText::new(label).size(s)).clicked() {
                            if selected {
                                self.build_category = None;
                                self.build_tool = BuildTool::None;
                            } else {
                                self.build_category = Some(name);
                            }
                        }
                    }
                    ui.separator();
                    let s2 = 10.0;
                    if ui.selectable_label(self.build_tool == BuildTool::Destroy, egui::RichText::new("\u{274c} Destroy").size(s2)).clicked() {
                        self.build_tool = if self.build_tool == BuildTool::Destroy { BuildTool::None } else { BuildTool::Destroy };
                        self.build_category = None;
                    }
                });
            });

        // --- Build items panel (right of categories) ---
        if let Some(cat) = self.build_category {
            egui::Area::new(egui::Id::new("build_items"))
                .anchor(egui::Align2::LEFT_BOTTOM, [90.0, -20.0])
                .show(ctx, |ui| {
                    egui::Frame::window(ui.style()).show(ui, |ui| {
                        ui.set_max_width(90.0);
                        let s = 11.0;
                        let tool = &mut self.build_tool;
                        macro_rules! btn {
                            ($t:expr, $label:expr) => {
                                if ui.selectable_label(*tool == $t, egui::RichText::new($label).size(s)).clicked() {
                                    *tool = if *tool == $t { BuildTool::None } else { $t };
                                }
                            };
                        }
                        match cat {
                            "Walls" => {
                                btn!(BuildTool::WoodWall, "Wood");
                                btn!(BuildTool::SteelWall, "Steel");
                                btn!(BuildTool::SandstoneWall, "Sandstone");
                                btn!(BuildTool::GraniteWall, "Granite");
                                btn!(BuildTool::LimestoneWall, "Limestone");
                            }
                            "Floor" => {
                                btn!(BuildTool::WoodFloor, "Wood Floor");
                                btn!(BuildTool::StoneFloor, "Stone Floor");
                                btn!(BuildTool::ConcreteFloor, "Concrete");
                                btn!(BuildTool::Roof, "Add Roof");
                                btn!(BuildTool::RemoveFloor, "Remove Floor");
                                btn!(BuildTool::RemoveRoof, "Remove Roof");
                            }
                            "Build" => {
                                btn!(BuildTool::Fireplace, "Fireplace");
                                btn!(BuildTool::Bench, "Bench");
                                btn!(BuildTool::Bed, "Bed");
                                btn!(BuildTool::Fan, "Fan");
                                btn!(BuildTool::Compost, "Compost");
                                btn!(BuildTool::BerryBush, "Berry Bush");
                                btn!(BuildTool::Cannon, "Cannon");
                                btn!(BuildTool::ElectricLight, "Ceiling Light");
                                btn!(BuildTool::StandingLamp, "Floor Lamp");
                                btn!(BuildTool::TableLamp, "Table Lamp");
                            }
                            "Opening" => {
                                btn!(BuildTool::Window, "Window");
                                btn!(BuildTool::Door, "Door");
                            }
                            "Piping" => {
                                btn!(BuildTool::Pipe, "Pipe");
                                btn!(BuildTool::Pump, "Pump");
                                btn!(BuildTool::Tank, "Tank");
                                btn!(BuildTool::Valve, "Valve");
                                btn!(BuildTool::Outlet, "Outlet");
                                btn!(BuildTool::Inlet, "Inlet");
                            }
                            "Physics" => {
                                btn!(BuildTool::WoodBox, "Wood Box");
                                let pleb_label = format!("Add Colonist ({}/{})", self.plebs.len(), MAX_PLEBS);
                                if ui.button(egui::RichText::new(pleb_label).size(s)).clicked() {
                                    self.placing_pleb = !self.placing_pleb;
                                    if self.placing_pleb { *tool = BuildTool::None; }
                                }
                            }
                            _ => {}
                        }
                        if *tool != BuildTool::None {
                            ui.separator();
                            let hint = match *tool {
                                BuildTool::Bench => { let r = if self.build_rotation == 0 { "H" } else { "V" }; format!("Q/E [{}]", r) }
                                BuildTool::TableLamp => "On bench".to_string(),
                                BuildTool::Fan | BuildTool::Pump | BuildTool::Inlet | BuildTool::Outlet => {
                                    let d = match self.build_rotation { 0=>"N", 1=>"E", 2=>"S", _=>"W" };
                                    format!("Q/E [{}]", d)
                                }
                                BuildTool::Destroy | BuildTool::RemoveFloor | BuildTool::RemoveRoof => "Click/drag".to_string(),
                                BuildTool::WoodBox => "Click to drop".to_string(),
                                BuildTool::Window | BuildTool::Door => "Click wall".to_string(),
                                BuildTool::Roof => "Drag (needs wall support)".to_string(),
                                _ => "Click/drag to place".to_string(),
                            };
                            ui.label(egui::RichText::new(hint).weak().size(9.0));
                        }
                    });
                });
        }

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
                    activity: format!("{}", match &p.activity {
                        PlebActivity::Idle => "Idle".to_string(),
                        PlebActivity::Walking => "Walking".to_string(),
                        PlebActivity::Sleeping => "Sleeping".to_string(),
                        PlebActivity::Harvesting(p) => format!("Harvesting {:.0}%", p * 100.0),
                        PlebActivity::Eating => "Eating".to_string(),
                    }),
                    berries: p.inventory.berries,
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

                                // Background
                                let bg = if is_sel {
                                    egui::Color32::from_rgb(60, 100, 60)
                                } else {
                                    egui::Color32::from_rgb(50, 55, 65)
                                };
                                painter.rect_filled(rect, 4.0, bg);

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
                                    let berry_str = if pd.berries > 0 {
                                        format!("{} | {}x Berry", pd.activity, pd.berries)
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
                                    self.selected_pleb = if is_sel { None } else { Some(pd.idx) };
                                }
                            }
                        });
                    });
                });
        }

        // --- Context menu (right-click on selected pleb) ---
        if let Some((mx, my)) = self.context_menu {
            if let Some(sel_idx) = self.selected_pleb {
                if let Some(pleb) = self.plebs.get(sel_idx) {
                    let has_berries = pleb.inventory.berries > 0;
                    let is_sleeping = pleb.activity == PlebActivity::Sleeping;
                    let hunger = pleb.needs.hunger;
                    let rest = pleb.needs.rest;
                    let pleb_name = pleb.name.clone();

                    let mut close_menu = false;
                    egui::Area::new(egui::Id::new("pleb_context_menu"))
                        .fixed_pos(egui::Pos2::new(mx / bp_ppp, my / bp_ppp))
                        .show(ctx, |ui| {
                            egui::Frame::menu(ui.style()).show(ui, |ui| {
                                ui.label(egui::RichText::new(&pleb_name).strong().size(11.0));
                                ui.separator();

                                if has_berries {
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

        // Close context menu on left click elsewhere
        if self.mouse_pressed && self.context_menu.is_some() {
            self.context_menu = None;
        }

        // --- Overlay bar (bottom-right) ---
        egui::Area::new(egui::Id::new("overlay_bar"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -20.0])
            .show(ctx, |ui| {
                egui::Frame::window(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let ov = &mut self.fluid_overlay;
                        if ui.selectable_label(*ov == FluidOverlay::None, "Off").clicked() {
                            *ov = FluidOverlay::None;
                        }
                        if ui.selectable_label(*ov == FluidOverlay::Gases, "Gases").clicked() {
                            *ov = if *ov == FluidOverlay::Gases { FluidOverlay::None } else { FluidOverlay::Gases };
                        }
                        if ui.selectable_label(*ov == FluidOverlay::Smoke, "Smoke").clicked() {
                            *ov = if *ov == FluidOverlay::Smoke { FluidOverlay::None } else { FluidOverlay::Smoke };
                        }
                        if ui.selectable_label(*ov == FluidOverlay::O2, "O2").clicked() {
                            *ov = if *ov == FluidOverlay::O2 { FluidOverlay::None } else { FluidOverlay::O2 };
                        }
                        if ui.selectable_label(*ov == FluidOverlay::CO2, "CO2").clicked() {
                            *ov = if *ov == FluidOverlay::CO2 { FluidOverlay::None } else { FluidOverlay::CO2 };
                        }
                        if ui.selectable_label(*ov == FluidOverlay::Temp, "Temp").clicked() {
                            *ov = if *ov == FluidOverlay::Temp { FluidOverlay::None } else { FluidOverlay::Temp };
                        }
                        if ui.selectable_label(*ov == FluidOverlay::HeatFlow, "Heat").clicked() {
                            *ov = if *ov == FluidOverlay::HeatFlow { FluidOverlay::None } else { FluidOverlay::HeatFlow };
                        }
                        ui.separator();
                        if ui.selectable_label(*ov == FluidOverlay::Velocity, "Vel").clicked() {
                            *ov = if *ov == FluidOverlay::Velocity { FluidOverlay::None } else { FluidOverlay::Velocity };
                        }
                        if ui.selectable_label(*ov == FluidOverlay::Pressure, "Pres").clicked() {
                            *ov = if *ov == FluidOverlay::Pressure { FluidOverlay::None } else { FluidOverlay::Pressure };
                        }
                        ui.separator();
                        if ui.selectable_label(self.show_pipe_overlay, "Pipes").clicked() {
                            self.show_pipe_overlay = !self.show_pipe_overlay;
                        }
                        let mut debug = self.debug_mode;
                        if ui.selectable_label(debug, "Debug").clicked() {
                            debug = !debug;
                        }
                        self.debug_mode = debug;
                        if ui.selectable_label(self.enable_prox_glow, "Glow").clicked() {
                            self.enable_prox_glow = !self.enable_prox_glow;
                        }
                        if ui.selectable_label(self.enable_dir_bleed, "Bleed").clicked() {
                            self.enable_dir_bleed = !self.enable_dir_bleed;
                        }
                        if ui.selectable_label(self.enable_temporal, "Temporal").clicked() {
                            self.enable_temporal = !self.enable_temporal;
                            self.camera.force_refresh = 10.0;
                        }
                    });
                });
            });

        // Gas legend (shown when a gas overlay is active)
        if matches!(self.fluid_overlay, FluidOverlay::Gases | FluidOverlay::Smoke | FluidOverlay::O2 | FluidOverlay::CO2) {
            egui::Area::new(egui::Id::new("gas_legend"))
                .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -55.0])
                .interactable(false)
                .show(ctx, |ui| {
                    egui::Frame::window(ui.style()).show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        let s = 10.0;
                        let dot = |ui: &mut egui::Ui, col: egui::Color32, label: &str| {
                            ui.horizontal(|ui| {
                                let (r, p) = ui.allocate_painter(egui::Vec2::splat(s), egui::Sense::hover());
                                p.rect_filled(r.rect, 2.0, col);
                                ui.label(egui::RichText::new(label).size(10.0));
                            });
                        };
                        dot(ui, egui::Color32::from_rgb(230, 230, 235), "Smoke");
                        dot(ui, egui::Color32::from_rgb(50, 100, 255), "O\u{2082} deficit");
                        dot(ui, egui::Color32::from_rgb(180, 200, 25), "CO\u{2082}");
                    });
                });
        }

        // Wind direction compass (bottom-right, above overlay bar)
        {
            let wx = self.fluid_params.wind_x;
            let wy = self.fluid_params.wind_y;
            let wind_mag = (wx * wx + wy * wy).sqrt();
            egui::Area::new(egui::Id::new("wind_compass"))
                .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -60.0])
                .interactable(false)
                .show(ctx, |ui| {
                    let size = 40.0;
                    let (resp, painter) = ui.allocate_painter(egui::Vec2::splat(size), egui::Sense::hover());
                    let center = resp.rect.center();
                    // Circle background
                    painter.circle_filled(center, size * 0.45, egui::Color32::from_rgba_unmultiplied(30, 30, 40, 180));
                    painter.circle_stroke(center, size * 0.45, egui::Stroke::new(1.0, egui::Color32::from_gray(100)));
                    if wind_mag > 0.1 {
                        let dir_x = wx / wind_mag;
                        let dir_y = wy / wind_mag;
                        let arrow_len = size * 0.35 * (wind_mag / 20.0).min(1.0).max(0.3);
                        let tip = center + egui::Vec2::new(dir_x * arrow_len, dir_y * arrow_len);
                        let tail = center - egui::Vec2::new(dir_x * arrow_len * 0.3, dir_y * arrow_len * 0.3);
                        // Arrow shaft
                        painter.line_segment([tail, tip], egui::Stroke::new(2.0, egui::Color32::from_rgb(200, 220, 255)));
                        // Arrowhead
                        let perp = egui::Vec2::new(-dir_y, dir_x) * arrow_len * 0.3;
                        let head_base = center + egui::Vec2::new(dir_x * arrow_len * 0.5, dir_y * arrow_len * 0.5);
                        painter.add(egui::Shape::convex_polygon(
                            vec![tip, head_base + perp, head_base - perp],
                            egui::Color32::from_rgb(200, 220, 255),
                            egui::Stroke::NONE,
                        ));
                    } else {
                        painter.text(center, egui::Align2::CENTER_CENTER, "·", egui::FontId::proportional(14.0), egui::Color32::from_gray(150));
                    }
                });
        }

        // Debug tooltip at cursor position (also shows when holding Shift)
        let shift_held = self.pressed_keys.contains(&KeyCode::ShiftLeft)
            || self.pressed_keys.contains(&KeyCode::ShiftRight);
        if self.debug_mode || shift_held {
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
                let type_name = match bt {
                    0 => "air", 1 => "stone", 2 => "dirt", 3 => "water",
                    4 => "wall", 5 => "glass", 6 => "fire", 7 => "e-light",
                    8 => "tree", 9 => "bench", 10 => "floor-lamp", 11 => "table-lamp",
                    12 => "fan", 13 => "compost", 14 => "insulated",
                    15 => "pipe", 16 => "pump", 17 => "tank", 18 => "valve",
                    19 => "outlet", 20 => "inlet",
                    21 => "wood-wall", 22 => "steel-wall", 23 => "sandstone",
                    24 => "granite", 25 => "limestone",
                    26 => "wood-floor", 27 => "stone-floor", 28 => "concrete",
                    _ => "?",
                };
                let roof = if flags & 2 != 0 { " R" } else { "" };
                let door = if flags & 1 != 0 { if flags & 4 != 0 { " D:open" } else { " D:shut" } } else { "" };
                block_info = format!("{}(h{}){}{}", type_name, bh, roof, door);
            }

            #[cfg(not(target_arch = "wasm32"))]
            let gas_info = {
                let [smoke_r, o2, co2, temp] = self.debug_fluid_density;
                format!("Smoke: {:.3}\nO2: {:.3}\nCO2: {:.3}\nTemp: {:.1}°C", smoke_r, o2, co2, temp)
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

            let tip = format!(
                "({:.1}, {:.1})\n{}\n{}{}",
                wx, wy, block_info, gas_info, pipe_info
            );

            // Position tooltip near cursor
            let cursor_screen = ctx.input(|i| {
                i.pointer.hover_pos().unwrap_or(egui::Pos2::ZERO)
            });
            egui::Area::new(egui::Id::new("debug_tooltip"))
                .fixed_pos(cursor_screen + egui::Vec2::new(15.0, 15.0))
                .interactable(false)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.label(egui::RichText::new(tip).monospace().size(12.0));
                    });
                });
        }

        // Drag shape preview (walls=hollow rect, pipes=line, destroy=filled rect)
        if let Some((sx, sy)) = self.drag_start {
            if self.mouse_dragged {
                let (hwx, hwy) = self.hover_world;
                let (ex, ey) = (hwx.floor() as i32, hwy.floor() as i32);
                let tiles = match self.build_tool {
                    BuildTool::Pipe => Self::line_tiles(sx, sy, ex, ey),
                    BuildTool::Destroy => Self::filled_rect_tiles(sx, sy, ex, ey),
                    BuildTool::WoodFloor | BuildTool::StoneFloor | BuildTool::ConcreteFloor
                    | BuildTool::Roof | BuildTool::RemoveFloor | BuildTool::RemoveRoof => {
                        Self::filled_rect_tiles(sx, sy, ex, ey)
                    }
                    BuildTool::WoodWall | BuildTool::SteelWall | BuildTool::SandstoneWall
                    | BuildTool::GraniteWall | BuildTool::LimestoneWall => {
                        Self::hollow_rect_tiles(sx, sy, ex, ey)
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
                            let valid = if *tx < 0 || *ty < 0 || *tx >= GRID_W as i32 || *ty >= GRID_H as i32 {
                                false
                            } else {
                                let tidx = (*ty as u32 * GRID_W + *tx as u32) as usize;
                                let tb = self.grid_data[tidx];
                                let tbt = tb & 0xFF;
                                let tbh = (tb >> 8) & 0xFF;
                                (tbt == 0 || tbt == 2) && tbh == 0
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
                }
            }
        }

        // Blueprint preview — draw ghost overlay for placement
        if !blueprint_tiles.is_empty() {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;

            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("blueprint"),
            ));

            for &((tx, ty), valid) in &blueprint_tiles {
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

                painter.rect_filled(
                    egui::Rect::from_min_max(egui::pos2(sx0, sy0), egui::pos2(sx1, sy1)),
                    0.0,
                    color,
                );

                // Direction arrow for fan, pump, inlet, outlet
                if matches!(self.build_tool, BuildTool::Fan | BuildTool::Pump | BuildTool::Inlet | BuildTool::Outlet) {
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
                let is_selected = self.selected_cannon == Some(cannon_idx);
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
                }
            }
        }

        // Pump speed slider popup
        if let Some(pump_idx) = self.selected_pump {
            let (pwx, pwy) = self.selected_pump_world;
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
                self.selected_pump = None;
            }
        }

        // Fan speed slider popup
        if let Some(_fan_idx) = self.selected_fan {
            let (fwx, fwy) = self.selected_fan_world;
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
                self.selected_fan = None;
            }
        }
    }
}
