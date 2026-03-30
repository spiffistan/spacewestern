//! UI drawing — all egui panels, overlays, debug tooltips.
//! Extracted from render() to keep main.rs manageable.

use crate::*;

impl App {
    /// Regenerate elevation, water table, and terrain from current params.
    fn regenerate_world_preview(&mut self) {
        let seed = self.terrain_params.seed;
        self.grid_data = grid::generate_world(seed);
        self.elevation_data = grid::generate_elevation_seeded(&self.grid_data, seed);
        self.water_table = grid::generate_water_table_seeded(&self.grid_data, seed);
        grid::adjust_water_table_for_elevation(&mut self.water_table, &self.elevation_data);
        self.terrain_data = grid::generate_terrain_with_params(
            &self.elevation_data,
            &self.water_table,
            &self.terrain_params,
        );
        self.terrain_dirty = true;
        self.grid_dirty = true;
    }

    /// Pixels-per-point scale factor for the current window.
    fn ppp(&self) -> f32 {
        self.window
            .as_ref()
            .map(|w| w.scale_factor() as f32)
            .unwrap_or(1.0)
    }

    /// Convert world coords to screen coords for egui overlay drawing.
    fn world_to_screen_ui(
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
    fn tile_px(&self, bp_cam: (f32, f32, f32, f32, f32)) -> f32 {
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
            GameState::CharGen => self.draw_chargen_screen(ctx),
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
                self.draw_selection_info(ctx);
                self.draw_notifications(ctx);
                self.draw_hints(ctx, bp_cam, bp_ppp);
                self.draw_conditions_bar(ctx);
                self.draw_game_log(ctx);
                self.draw_minimap(ctx);
            }
        }
    }

    fn draw_main_menu(&mut self, ctx: &egui::Context) {
        // Full-screen dark overlay
        egui::Area::new(egui::Id::new("main_menu_bg"))
            .anchor(egui::Align2::LEFT_TOP, [0.0, 0.0])
            .interactable(false)
            .show(ctx, |ui| {
                let screen = ctx.content_rect();
                ui.allocate_exact_size(screen.size(), egui::Sense::hover());
                ui.painter()
                    .rect_filled(screen, 0.0, egui::Color32::from_rgb(10, 12, 18));
            });

        egui::Area::new(egui::Id::new("main_menu"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(320.0);
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(
                        egui::RichText::new("RAYWORLD")
                            .size(36.0)
                            .strong()
                            .color(egui::Color32::from_rgb(200, 180, 120)),
                    );
                    ui.label(
                        egui::RichText::new("A colony survival game")
                            .size(14.0)
                            .color(egui::Color32::from_gray(140)),
                    );
                    ui.add_space(30.0);

                    let btn_size = egui::Vec2::new(200.0, 36.0);
                    let new_game_btn = ui.add_sized(
                        btn_size,
                        egui::Button::new(egui::RichText::new("New Game").size(16.0)),
                    );
                    if new_game_btn.hovered() && self.menu_hover_id != Some(new_game_btn.id) {
                        self.menu_hover_id = Some(new_game_btn.id);
                        if self.audio_output.is_none() {
                            self.audio_output = audio::AudioOutput::new();
                        }
                        if let Some(ref audio) = self.audio_output {
                            audio.play_click();
                        }
                    }
                    if new_game_btn.clicked() {
                        self.game_state = GameState::MapGen;
                    }
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!("v{}", include_str!("../VERSION").trim()))
                            .size(10.0)
                            .color(egui::Color32::from_gray(80)),
                    );
                });
            });
    }

    /// Play a click sound when a button is first hovered. Returns true if clicked.
    fn hover_click_button(&mut self, ui: &mut egui::Ui, text: egui::RichText) -> bool {
        let resp = ui.button(text);
        let btn_id = resp.id;
        if resp.hovered() {
            if self.menu_hover_id != Some(btn_id) {
                self.menu_hover_id = Some(btn_id);
                if self.audio_output.is_none() {
                    self.audio_output = audio::AudioOutput::new();
                }
                if let Some(ref audio) = self.audio_output {
                    audio.play_click();
                }
            }
        }
        resp.clicked()
    }

    fn draw_map_gen_screen(&mut self, ctx: &egui::Context) {
        // Dark overlay
        egui::Area::new(egui::Id::new("mapgen_bg"))
            .anchor(egui::Align2::LEFT_TOP, [0.0, 0.0])
            .interactable(false)
            .show(ctx, |ui| {
                let screen = ctx.content_rect();
                ui.allocate_exact_size(screen.size(), egui::Sense::hover());
                ui.painter()
                    .rect_filled(screen, 0.0, egui::Color32::from_rgb(10, 12, 18));
            });

        let mut start_game = false;

        egui::Window::new("New Game — Map Generator")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // ═══ LEFT: Sliders ═══
                    ui.vertical(|ui| {
                        ui.set_max_width(320.0);
                        ui.label(egui::RichText::new("Terrain").strong().size(15.0));
                        ui.add_space(4.0);

                        let slider = |ui: &mut egui::Ui,
                                      label: &str,
                                      val: &mut f32,
                                      color: egui::Color32| {
                            ui.horizontal(|ui| {
                                let (dot_rect, _) = ui.allocate_exact_size(
                                    egui::Vec2::splat(12.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter_at(dot_rect).rect_filled(dot_rect, 3.0, color);
                                ui.label(egui::RichText::new(label).size(13.0).monospace());
                                ui.spacing_mut().slider_width = 140.0;
                                ui.add(egui::Slider::new(val, 0.0..=1.0).show_value(false));
                            });
                        };

                        slider(
                            ui,
                            "Grass ",
                            &mut self.terrain_params.grass,
                            egui::Color32::from_rgb(107, 92, 56),
                        );
                        slider(
                            ui,
                            "Loam  ",
                            &mut self.terrain_params.loam,
                            egui::Color32::from_rgb(97, 77, 46),
                        );
                        slider(
                            ui,
                            "Clay  ",
                            &mut self.terrain_params.clay,
                            egui::Color32::from_rgb(128, 97, 64),
                        );
                        slider(
                            ui,
                            "Chalky",
                            &mut self.terrain_params.chalky,
                            egui::Color32::from_rgb(173, 168, 153),
                        );
                        slider(
                            ui,
                            "Rocky ",
                            &mut self.terrain_params.rocky,
                            egui::Color32::from_rgb(115, 107, 97),
                        );
                        slider(
                            ui,
                            "Gravel",
                            &mut self.terrain_params.gravel,
                            egui::Color32::from_rgb(122, 117, 107),
                        );
                        slider(
                            ui,
                            "Peat  ",
                            &mut self.terrain_params.peat,
                            egui::Color32::from_rgb(56, 46, 31),
                        );
                        slider(
                            ui,
                            "Marsh ",
                            &mut self.terrain_params.marsh,
                            egui::Color32::from_rgb(77, 89, 56),
                        );

                        ui.add_space(6.0);
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Ponds").size(13.0));
                            ui.spacing_mut().slider_width = 140.0;
                            ui.add(
                                egui::Slider::new(&mut self.terrain_params.pond_density, 0.0..=1.0)
                                    .show_value(false),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Tall Grass").size(13.0));
                            ui.spacing_mut().slider_width = 140.0;
                            ui.add(
                                egui::Slider::new(
                                    &mut self.terrain_params.grass_density,
                                    0.0..=1.0,
                                )
                                .show_value(false),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Seed").size(13.0));
                            let mut seed_i = self.terrain_params.seed as i32;
                            if ui
                                .add(egui::DragValue::new(&mut seed_i).range(0..=9999))
                                .changed()
                            {
                                self.terrain_params.seed = seed_i.max(0) as u32;
                            }
                            if self.hover_click_button(ui, egui::RichText::new("Random").size(13.0))
                            {
                                self.terrain_params.seed =
                                    (self.frame_count.wrapping_mul(2654435761)) % 10000;
                                self.regenerate_world_preview();
                            }
                        });

                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(format!("Map: {}×{}", GRID_W, GRID_H))
                                .size(11.0)
                                .weak(),
                        );
                        ui.add_space(6.0);
                        if self.hover_click_button(ui, egui::RichText::new("Preview").size(14.0)) {
                            self.regenerate_world_preview();
                        }
                    });

                    ui.separator();

                    // ═══ RIGHT: Preview ═══
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Preview").strong().size(15.0));
                        let preview_size = 420.0;
                        let (rect, _) = ui.allocate_exact_size(
                            egui::Vec2::splat(preview_size),
                            egui::Sense::hover(),
                        );
                        let painter = ui.painter_at(rect);
                        painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(20, 20, 20));

                        let scale = preview_size / GRID_W as f32;
                        let colors: [egui::Color32; 8] = [
                            egui::Color32::from_rgb(107, 92, 56),
                            egui::Color32::from_rgb(173, 168, 153),
                            egui::Color32::from_rgb(115, 107, 97),
                            egui::Color32::from_rgb(128, 97, 64),
                            egui::Color32::from_rgb(122, 117, 107),
                            egui::Color32::from_rgb(56, 46, 31),
                            egui::Color32::from_rgb(77, 89, 56),
                            egui::Color32::from_rgb(97, 77, 46),
                        ];
                        let step = 4u32;
                        let px_size = scale * step as f32;
                        for ty in (0..GRID_H).step_by(step as usize) {
                            for tx in (0..GRID_W).step_by(step as usize) {
                                let idx = (ty * GRID_W + tx) as usize;
                                if idx < self.terrain_data.len() {
                                    let tt = (self.terrain_data[idx] & 0xF) as usize;
                                    if tt < 8 {
                                        let px = rect.min.x + tx as f32 * scale;
                                        let py = rect.min.y + ty as f32 * scale;
                                        painter.rect_filled(
                                            egui::Rect::from_min_size(
                                                egui::pos2(px, py),
                                                egui::vec2(px_size, px_size),
                                            ),
                                            0.0,
                                            colors[tt],
                                        );
                                    }
                                }
                            }
                        }
                    });
                });

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if self.hover_click_button(ui, egui::RichText::new("← Back").size(14.0)) {
                        self.game_state = GameState::MainMenu;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.hover_click_button(
                            ui,
                            egui::RichText::new("Next →").size(16.0).strong(),
                        ) {
                            start_game = true;
                        }
                    });
                });
            });

        if start_game {
            self.regenerate_world_preview();
            compute_roof_heights_wd(&mut self.grid_data, &self.wall_data);
            self.wall_data = extract_wall_data_from_grid(&self.grid_data);
            self.doors = grid::extract_doors_from_wall_data(&self.wall_data);
            self.game_state = GameState::CharGen;
        }
    }

    fn draw_chargen_screen(&mut self, ctx: &egui::Context) {
        // Dark overlay
        egui::Area::new(egui::Id::new("chargen_bg"))
            .anchor(egui::Align2::LEFT_TOP, [0.0, 0.0])
            .interactable(false)
            .show(ctx, |ui| {
                let screen = ctx.content_rect();
                ui.allocate_exact_size(screen.size(), egui::Sense::hover());
                ui.painter()
                    .rect_filled(screen, 0.0, egui::Color32::from_rgb(10, 12, 18));
            });

        // Rotate preview
        self.chargen_preview_angle += ctx.input(|i| i.stable_dt) * 0.5;

        let mut start_game = false;

        egui::Window::new("Create Your Colonist")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(750.0);
                ui.horizontal(|ui| {
                    // === LEFT: Preview + lore ===
                    ui.vertical(|ui| {
                        ui.set_max_width(300.0);

                        let preview_size = 280.0;
                        let (rect, _) = ui.allocate_exact_size(
                            egui::Vec2::new(preview_size, preview_size),
                            egui::Sense::hover(),
                        );
                        let painter = ui.painter_at(rect);
                        painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(25, 28, 20));

                        let cx = rect.center().x;
                        let cy = rect.center().y + 25.0;
                        let body_scale = match self.chargen_body_type {
                            BodyType::Thin => 0.32,
                            BodyType::Medium => 0.38,
                            BodyType::Stocky => 0.44,
                        };
                        let s = preview_size * body_scale;
                        let dir_x = self.chargen_preview_angle.cos() * 0.03 * s;

                        let ell = |p: &egui::Painter,
                                   pos: egui::Pos2,
                                   rx: f32,
                                   ry: f32,
                                   col: egui::Color32| {
                            p.rect_filled(
                                egui::Rect::from_center_size(pos, egui::vec2(rx * 2.0, ry * 2.0)),
                                rx.min(ry),
                                col,
                            );
                        };
                        let to_col = |c: [f32; 3]| {
                            egui::Color32::from_rgb(
                                (c[0] * 255.0) as u8,
                                (c[1] * 255.0) as u8,
                                (c[2] * 255.0) as u8,
                            )
                        };
                        let to_dark = |c: [f32; 3]| {
                            egui::Color32::from_rgb(
                                (c[0] * 140.0) as u8,
                                (c[1] * 140.0) as u8,
                                (c[2] * 140.0) as u8,
                            )
                        };

                        ell(
                            &painter,
                            egui::pos2(cx, cy + s * 0.35),
                            s * 0.35,
                            s * 0.12,
                            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 60),
                        );
                        ell(
                            &painter,
                            egui::pos2(cx, cy + s * 0.28),
                            s * 0.12,
                            s * 0.06,
                            to_dark(self.chargen_pants),
                        );
                        ell(
                            &painter,
                            egui::pos2(cx, cy + s * 0.14),
                            s * 0.22,
                            s * 0.15,
                            to_col(self.chargen_pants),
                        );
                        ell(
                            &painter,
                            egui::pos2(cx + dir_x * 0.3, cy - s * 0.08),
                            s * 0.26,
                            s * 0.20,
                            to_col(self.chargen_shirt),
                        );
                        painter.circle_filled(
                            egui::pos2(cx + dir_x, cy - s * 0.32),
                            s * 0.16,
                            to_col(self.chargen_skin),
                        );
                        let hair_r = match self.chargen_hair_style {
                            3 => s * 0.04,
                            2 => s * 0.14,
                            1 => s * 0.10,
                            _ => s * 0.08,
                        };
                        ell(
                            &painter,
                            egui::pos2(cx + dir_x * 1.5, cy - s * 0.42),
                            hair_r,
                            hair_r * 0.7,
                            to_col(self.chargen_hair),
                        );

                        painter.text(
                            egui::pos2(cx, cy + s * 0.50),
                            egui::Align2::CENTER_TOP,
                            &self.chargen_name,
                            egui::FontId::proportional(16.0),
                            egui::Color32::from_gray(220),
                        );
                        painter.text(
                            egui::pos2(cx, cy + s * 0.50 + 20.0),
                            egui::Align2::CENTER_TOP,
                            self.chargen_backstory.name(),
                            egui::FontId::proportional(12.0),
                            egui::Color32::from_gray(140),
                        );

                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(self.chargen_backstory.description())
                                .size(11.0)
                                .italics()
                                .color(egui::Color32::from_gray(160)),
                        );
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "Ability: {}",
                                self.chargen_backstory.ability()
                            ))
                            .size(10.0)
                            .color(egui::Color32::from_rgb(180, 160, 100)),
                        );
                    });

                    ui.separator();

                    // === RIGHT: Controls ===
                    ui.vertical(|ui| {
                        ui.set_min_width(400.0);

                        ui.label(egui::RichText::new("Identity").strong().size(14.0));
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            ui.add_sized(
                                [160.0, 20.0],
                                egui::TextEdit::singleline(&mut self.chargen_name),
                            );
                            if ui.small_button("Random").clicked() {
                                self.chargen_name = random_name(self.frame_count);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Gender:");
                            if ui
                                .selectable_label(self.chargen_gender == Gender::Male, "Male")
                                .clicked()
                            {
                                self.chargen_gender = Gender::Male;
                            }
                            if ui
                                .selectable_label(self.chargen_gender == Gender::Female, "Female")
                                .clicked()
                            {
                                self.chargen_gender = Gender::Female;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Age:");
                            ui.add(egui::Slider::new(&mut self.chargen_age, 18..=65));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Build:");
                            for bt in &[BodyType::Thin, BodyType::Medium, BodyType::Stocky] {
                                if ui
                                    .selectable_label(self.chargen_body_type == *bt, bt.label())
                                    .clicked()
                                {
                                    self.chargen_body_type = *bt;
                                }
                            }
                        });

                        ui.add_space(6.0);
                        ui.separator();
                        ui.label(egui::RichText::new("Backstory").strong().size(14.0));
                        ui.add_space(2.0);
                        egui::ScrollArea::vertical()
                            .max_height(100.0)
                            .show(ui, |ui| {
                                for &bs in Backstory::ALL {
                                    let resp = ui
                                        .selectable_label(self.chargen_backstory == bs, bs.name());
                                    if resp.clicked() {
                                        self.chargen_backstory = bs;
                                    }
                                    if resp.hovered() {
                                        resp.on_hover_text(bs.description());
                                    }
                                }
                            });

                        ui.add_space(6.0);
                        ui.separator();
                        ui.label(egui::RichText::new("Appearance").strong().size(14.0));
                        ui.add_space(2.0);

                        // Skin tone palette
                        const SKIN_PALETTE: &[[f32; 3]] = &[
                            [0.96, 0.87, 0.77], // very light
                            [0.92, 0.78, 0.65], // light
                            [0.85, 0.68, 0.53], // fair
                            [0.76, 0.60, 0.46], // medium
                            [0.65, 0.48, 0.35], // olive
                            [0.55, 0.38, 0.26], // tan
                            [0.45, 0.30, 0.20], // brown
                            [0.36, 0.22, 0.14], // dark
                            [0.28, 0.16, 0.10], // very dark
                            [0.20, 0.12, 0.08], // deepest
                        ];
                        ui.horizontal(|ui| {
                            ui.label("Skin:");
                            let swatch = 16.0;
                            for &tone in SKIN_PALETTE {
                                let col = egui::Color32::from_rgb(
                                    (tone[0] * 255.0) as u8,
                                    (tone[1] * 255.0) as u8,
                                    (tone[2] * 255.0) as u8,
                                );
                                let selected = (self.chargen_skin[0] - tone[0]).abs() < 0.02
                                    && (self.chargen_skin[1] - tone[1]).abs() < 0.02;
                                let (rect, resp) = ui.allocate_exact_size(
                                    egui::vec2(swatch, swatch),
                                    egui::Sense::click(),
                                );
                                ui.painter().rect_filled(rect, 2.0, col);
                                if selected {
                                    ui.painter().rect_stroke(
                                        rect,
                                        2.0,
                                        egui::Stroke::new(2.0, egui::Color32::WHITE),
                                        egui::StrokeKind::Outside,
                                    );
                                }
                                if resp.clicked() {
                                    self.chargen_skin = tone;
                                }
                            }
                        });

                        // Hair color palette
                        const HAIR_PALETTE: &[[f32; 3]] = &[
                            [0.95, 0.90, 0.65], // blonde
                            [0.80, 0.55, 0.25], // strawberry
                            [0.55, 0.30, 0.12], // auburn
                            [0.40, 0.22, 0.10], // brown
                            [0.25, 0.15, 0.08], // dark brown
                            [0.10, 0.08, 0.06], // black
                            [0.60, 0.58, 0.55], // gray
                            [0.85, 0.82, 0.78], // white/silver
                            [0.65, 0.20, 0.12], // red
                            [0.45, 0.15, 0.08], // deep red
                        ];
                        ui.horizontal(|ui| {
                            ui.label("Hair:");
                            let swatch = 16.0;
                            for &tone in HAIR_PALETTE {
                                let col = egui::Color32::from_rgb(
                                    (tone[0] * 255.0) as u8,
                                    (tone[1] * 255.0) as u8,
                                    (tone[2] * 255.0) as u8,
                                );
                                let selected = (self.chargen_hair[0] - tone[0]).abs() < 0.02
                                    && (self.chargen_hair[1] - tone[1]).abs() < 0.02;
                                let (rect, resp) = ui.allocate_exact_size(
                                    egui::vec2(swatch, swatch),
                                    egui::Sense::click(),
                                );
                                ui.painter().rect_filled(rect, 2.0, col);
                                if selected {
                                    ui.painter().rect_stroke(
                                        rect,
                                        2.0,
                                        egui::Stroke::new(2.0, egui::Color32::WHITE),
                                        egui::StrokeKind::Outside,
                                    );
                                }
                                if resp.clicked() {
                                    self.chargen_hair = tone;
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Shirt:");
                            ui.color_edit_button_rgb(&mut self.chargen_shirt);
                            ui.label("Pants:");
                            ui.color_edit_button_rgb(&mut self.chargen_pants);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Hair style:");
                            for (i, name) in ["Short", "Medium", "Long", "Bald"].iter().enumerate()
                            {
                                if ui
                                    .selectable_label(self.chargen_hair_style == i as u8, *name)
                                    .clicked()
                                {
                                    self.chargen_hair_style = i as u8;
                                }
                            }
                        });

                        ui.add_space(6.0);
                        ui.separator();
                        ui.label(egui::RichText::new("Trait").strong().size(14.0));
                        ui.add_space(2.0);
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .selectable_label(self.chargen_trait.is_none(), "None")
                                .clicked()
                            {
                                self.chargen_trait = None;
                            }
                            for &t in PlebTrait::ALL {
                                let resp =
                                    ui.selectable_label(self.chargen_trait == Some(t), t.name());
                                if resp.clicked() {
                                    self.chargen_trait = Some(t);
                                }
                                if resp.hovered() {
                                    resp.on_hover_text(t.description());
                                }
                            }
                        });

                        ui.add_space(6.0);
                        ui.separator();
                        ui.label(egui::RichText::new("Skills").strong().size(14.0));
                        ui.add_space(2.0);
                        let skills = self.chargen_backstory.skills();
                        let names = Backstory::skill_names();
                        for (i, &name) in names.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("{:12}", name))
                                        .monospace()
                                        .size(11.0),
                                );
                                let val = skills[i];
                                let bar_w = 100.0;
                                let (bar_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(bar_w, 10.0),
                                    egui::Sense::hover(),
                                );
                                let fill_w = bar_w * val as f32 / 10.0;
                                let bar_col = if val >= 7 {
                                    egui::Color32::from_rgb(80, 180, 80)
                                } else if val >= 4 {
                                    egui::Color32::from_rgb(180, 160, 60)
                                } else {
                                    egui::Color32::from_rgb(120, 80, 60)
                                };
                                ui.painter().rect_filled(
                                    bar_rect,
                                    2.0,
                                    egui::Color32::from_gray(30),
                                );
                                ui.painter().rect_filled(
                                    egui::Rect::from_min_size(
                                        bar_rect.min,
                                        egui::vec2(fill_w, 10.0),
                                    ),
                                    2.0,
                                    bar_col,
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}", val))
                                        .monospace()
                                        .size(11.0),
                                );
                            });
                        }

                        ui.add_space(8.0);
                        if ui.button("Randomize All").clicked() {
                            let seed = self.frame_count;
                            self.chargen_name = random_name(seed);
                            let h = |s: u32, off: u32| -> f32 {
                                ((s.wrapping_add(off).wrapping_mul(2654435761)) & 0xFFFF) as f32
                                    / 65535.0
                            };
                            self.chargen_skin =
                                SKIN_PALETTE[(h(seed, 1) * SKIN_PALETTE.len() as f32) as usize
                                    % SKIN_PALETTE.len()];
                            self.chargen_hair =
                                HAIR_PALETTE[(h(seed, 4) * HAIR_PALETTE.len() as f32) as usize
                                    % HAIR_PALETTE.len()];
                            self.chargen_hair_style = (h(seed, 7) * 4.0) as u8;
                            self.chargen_shirt = [
                                0.15 + h(seed, 8) * 0.6,
                                0.15 + h(seed, 9) * 0.5,
                                0.15 + h(seed, 10) * 0.5,
                            ];
                            self.chargen_pants = [
                                0.15 + h(seed, 11) * 0.4,
                                0.15 + h(seed, 12) * 0.35,
                                0.10 + h(seed, 13) * 0.3,
                            ];
                            self.chargen_backstory = Backstory::ALL
                                [(h(seed, 14) * 10.0) as usize % Backstory::ALL.len()];
                            self.chargen_body_type =
                                [BodyType::Thin, BodyType::Medium, BodyType::Stocky]
                                    [(h(seed, 15) * 3.0) as usize % 3];
                            self.chargen_gender = if h(seed, 16) > 0.5 {
                                Gender::Female
                            } else {
                                Gender::Male
                            };
                            self.chargen_age = 20 + (h(seed, 17) * 40.0) as u8;
                            let trait_roll =
                                (h(seed, 18) * (PlebTrait::ALL.len() + 2) as f32) as usize;
                            self.chargen_trait = PlebTrait::ALL.get(trait_roll).copied();
                        }
                    });
                });

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if self.hover_click_button(ui, egui::RichText::new("← Back").size(14.0)) {
                        self.game_state = GameState::MapGen;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.hover_click_button(
                            ui,
                            egui::RichText::new("Begin →").size(16.0).strong(),
                        ) {
                            start_game = true;
                        }
                    });
                });
            });

        if start_game {
            let pleb = &mut self.plebs[0];
            pleb.name = self.chargen_name.clone();
            pleb.appearance.skin_r = self.chargen_skin[0];
            pleb.appearance.skin_g = self.chargen_skin[1];
            pleb.appearance.skin_b = self.chargen_skin[2];
            pleb.appearance.hair_r = self.chargen_hair[0];
            pleb.appearance.hair_g = self.chargen_hair[1];
            pleb.appearance.hair_b = self.chargen_hair[2];
            pleb.appearance.hair_style = self.chargen_hair_style as u32;
            pleb.appearance.shirt_r = self.chargen_shirt[0];
            pleb.appearance.shirt_g = self.chargen_shirt[1];
            pleb.appearance.shirt_b = self.chargen_shirt[2];
            pleb.appearance.pants_r = self.chargen_pants[0];
            pleb.appearance.pants_g = self.chargen_pants[1];
            pleb.appearance.pants_b = self.chargen_pants[2];
            self.game_state = GameState::Playing;
        }
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
                                if ui
                                    .selectable_label(*ov == FluidOverlay::Gases, "Gases")
                                    .clicked()
                                {
                                    *ov = if *ov == FluidOverlay::Gases {
                                        FluidOverlay::None
                                    } else {
                                        FluidOverlay::Gases
                                    };
                                }
                                if ui
                                    .selectable_label(*ov == FluidOverlay::Smoke, "Smoke")
                                    .clicked()
                                {
                                    *ov = if *ov == FluidOverlay::Smoke {
                                        FluidOverlay::None
                                    } else {
                                        FluidOverlay::Smoke
                                    };
                                }
                                if ui
                                    .selectable_label(*ov == FluidOverlay::O2, "O\u{2082}")
                                    .clicked()
                                {
                                    *ov = if *ov == FluidOverlay::O2 {
                                        FluidOverlay::None
                                    } else {
                                        FluidOverlay::O2
                                    };
                                }
                                if ui
                                    .selectable_label(*ov == FluidOverlay::CO2, "CO\u{2082}")
                                    .clicked()
                                {
                                    *ov = if *ov == FluidOverlay::CO2 {
                                        FluidOverlay::None
                                    } else {
                                        FluidOverlay::CO2
                                    };
                                }
                            });
                        });
                        ui.separator();
                        // Thermal group
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Thermal").size(9.0).weak());
                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_label(*ov == FluidOverlay::Temp, "Temp")
                                    .clicked()
                                {
                                    *ov = if *ov == FluidOverlay::Temp {
                                        FluidOverlay::None
                                    } else {
                                        FluidOverlay::Temp
                                    };
                                }
                            });
                        });
                        ui.separator();
                        // Physics group
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Physics").size(9.0).weak());
                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_label(*ov == FluidOverlay::Velocity, "Vel")
                                    .clicked()
                                {
                                    *ov = if *ov == FluidOverlay::Velocity {
                                        FluidOverlay::None
                                    } else {
                                        FluidOverlay::Velocity
                                    };
                                }
                                if ui
                                    .selectable_label(*ov == FluidOverlay::Pressure, "Pres")
                                    .clicked()
                                {
                                    *ov = if *ov == FluidOverlay::Pressure {
                                        FluidOverlay::None
                                    } else {
                                        FluidOverlay::Pressure
                                    };
                                }
                            });
                        });
                        ui.separator();
                        // Infrastructure
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Infra").size(9.0).weak());
                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_label(self.show_pipe_overlay, "Vent")
                                    .clicked()
                                {
                                    self.show_pipe_overlay = !self.show_pipe_overlay;
                                }
                                if ui
                                    .selectable_label(self.show_liquid_overlay, "Liquid")
                                    .clicked()
                                {
                                    self.show_liquid_overlay = !self.show_liquid_overlay;
                                }
                                if ui
                                    .selectable_label(self.show_flow_overlay, "Flow")
                                    .clicked()
                                {
                                    self.show_flow_overlay = !self.show_flow_overlay;
                                }
                                if ui
                                    .selectable_label(*ov == FluidOverlay::Power, "Volts")
                                    .clicked()
                                {
                                    *ov = if *ov == FluidOverlay::Power {
                                        FluidOverlay::None
                                    } else {
                                        FluidOverlay::Power
                                    };
                                }
                                if ui
                                    .selectable_label(*ov == FluidOverlay::PowerAmps, "Amps")
                                    .clicked()
                                {
                                    *ov = if *ov == FluidOverlay::PowerAmps {
                                        FluidOverlay::None
                                    } else {
                                        FluidOverlay::PowerAmps
                                    };
                                }
                                if ui
                                    .selectable_label(*ov == FluidOverlay::PowerWatts, "Watts")
                                    .clicked()
                                {
                                    *ov = if *ov == FluidOverlay::PowerWatts {
                                        FluidOverlay::None
                                    } else {
                                        FluidOverlay::PowerWatts
                                    };
                                }
                            });
                        });
                        // Environment group
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Environ").size(9.0).weak());
                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_label(*ov == FluidOverlay::Water, "Water")
                                    .clicked()
                                {
                                    *ov = if *ov == FluidOverlay::Water {
                                        FluidOverlay::None
                                    } else {
                                        FluidOverlay::Water
                                    };
                                }
                                if ui
                                    .selectable_label(*ov == FluidOverlay::WaterTable, "Table")
                                    .clicked()
                                {
                                    *ov = if *ov == FluidOverlay::WaterTable {
                                        FluidOverlay::None
                                    } else {
                                        FluidOverlay::WaterTable
                                    };
                                }
                                if ui
                                    .selectable_label(self.show_velocity_arrows, "Arrows")
                                    .clicked()
                                {
                                    self.show_velocity_arrows = !self.show_velocity_arrows;
                                }
                            });
                        });
                        ui.separator();
                        // Sound group
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Sound").size(9.0).weak());
                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_label(*ov == FluidOverlay::Sound, "Waves")
                                    .clicked()
                                {
                                    *ov = if *ov == FluidOverlay::Sound {
                                        FluidOverlay::None
                                    } else {
                                        FluidOverlay::Sound
                                    };
                                }
                            });
                        });
                        ui.separator();
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Terrain").size(9.0).weak());
                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_label(*ov == FluidOverlay::Terrain, "Type")
                                    .clicked()
                                {
                                    *ov = if *ov == FluidOverlay::Terrain {
                                        FluidOverlay::None
                                    } else {
                                        FluidOverlay::Terrain
                                    };
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
            let show_legend = self.fluid_overlay != FluidOverlay::None
                || self.show_pipe_overlay
                || self.show_liquid_overlay
                || self.show_flow_overlay;
            if show_legend {
                egui::Area::new(egui::Id::new("layer_legend"))
                    .anchor(egui::Align2::RIGHT_TOP, [-10.0, 80.0])
                    .interactable(false)
                    .show(ctx, |ui| {
                        egui::Frame::window(ui.style()).show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            match self.fluid_overlay {
                                FluidOverlay::Gases => {
                                    dot(
                                        ui,
                                        egui::Color32::from_rgb(230, 230, 235),
                                        "Smoke (white)",
                                    );
                                    dot(
                                        ui,
                                        egui::Color32::from_rgb(50, 100, 255),
                                        "O\u{2082} deficit (blue)",
                                    );
                                    dot(
                                        ui,
                                        egui::Color32::from_rgb(180, 200, 25),
                                        "CO\u{2082} (yellow-green)",
                                    );
                                }
                                FluidOverlay::Smoke => {
                                    grad(
                                        ui,
                                        &[
                                            (egui::Color32::from_rgb(0, 0, 0), "None"),
                                            (egui::Color32::from_rgb(200, 50, 0), "Low density"),
                                            (egui::Color32::from_rgb(255, 200, 0), "Medium"),
                                            (
                                                egui::Color32::from_rgb(255, 255, 255),
                                                "High density",
                                            ),
                                        ],
                                    );
                                }
                                FluidOverlay::O2 => {
                                    dot(
                                        ui,
                                        egui::Color32::from_rgb(25, 100, 255),
                                        "High O\u{2082}",
                                    );
                                    dot(ui, egui::Color32::from_rgb(230, 25, 0), "Low O\u{2082}");
                                }
                                FluidOverlay::CO2 => {
                                    dot(
                                        ui,
                                        egui::Color32::from_rgb(180, 200, 25),
                                        "High CO\u{2082}",
                                    );
                                    dot(ui, egui::Color32::from_rgb(40, 40, 40), "Low CO\u{2082}");
                                }
                                FluidOverlay::Temp => {
                                    grad(
                                        ui,
                                        &[
                                            (egui::Color32::from_rgb(38, 0, 102), "< -15\u{b0}C"),
                                            (egui::Color32::from_rgb(0, 25, 178), "0\u{b0}C"),
                                            (
                                                egui::Color32::from_rgb(178, 217, 178),
                                                "15-25\u{b0}C",
                                            ),
                                            (egui::Color32::from_rgb(255, 217, 76), "30-40\u{b0}C"),
                                            (egui::Color32::from_rgb(255, 115, 25), "50-60\u{b0}C"),
                                            (egui::Color32::from_rgb(230, 30, 13), "80-100\u{b0}C"),
                                            (egui::Color32::from_rgb(217, 25, 140), "200\u{b0}C"),
                                            (egui::Color32::from_rgb(153, 38, 217), "400\u{b0}C"),
                                            (egui::Color32::from_rgb(128, 76, 255), "500\u{b0}C+"),
                                        ],
                                    );
                                }
                                FluidOverlay::Velocity => {
                                    grad(
                                        ui,
                                        &[
                                            (egui::Color32::from_rgb(25, 38, 76), "Still"),
                                            (egui::Color32::from_rgb(50, 130, 200), "Slow"),
                                            (egui::Color32::from_rgb(100, 217, 255), "Fast"),
                                        ],
                                    );
                                    dot(ui, egui::Color32::WHITE, "Arrow = direction");
                                }
                                FluidOverlay::Pressure => {
                                    grad(
                                        ui,
                                        &[
                                            (egui::Color32::from_rgb(38, 64, 204), "Negative"),
                                            (egui::Color32::from_rgb(128, 128, 140), "Neutral"),
                                            (egui::Color32::from_rgb(217, 51, 38), "Positive"),
                                        ],
                                    );
                                }
                                FluidOverlay::Power => {
                                    ui.label(egui::RichText::new("Voltage").size(10.0).strong());
                                    grad(
                                        ui,
                                        &[
                                            (egui::Color32::from_rgb(25, 76, 25), "Low voltage"),
                                            (egui::Color32::from_rgb(50, 200, 50), "Normal"),
                                            (egui::Color32::from_rgb(230, 200, 25), "High load"),
                                            (egui::Color32::from_rgb(255, 50, 25), "Overload"),
                                        ],
                                    );
                                }
                                FluidOverlay::PowerAmps => {
                                    ui.label(
                                        egui::RichText::new("Current (Amps)").size(10.0).strong(),
                                    );
                                    grad(
                                        ui,
                                        &[
                                            (egui::Color32::from_rgb(15, 15, 30), "No current"),
                                            (egui::Color32::from_rgb(40, 80, 180), "Low"),
                                            (egui::Color32::from_rgb(100, 200, 255), "Medium"),
                                            (egui::Color32::from_rgb(255, 255, 220), "High"),
                                        ],
                                    );
                                }
                                FluidOverlay::PowerWatts => {
                                    ui.label(
                                        egui::RichText::new("Power (Watts)").size(10.0).strong(),
                                    );
                                    grad(
                                        ui,
                                        &[
                                            (egui::Color32::from_rgb(50, 200, 50), "Generating"),
                                            (egui::Color32::from_rgb(60, 60, 60), "Idle"),
                                            (egui::Color32::from_rgb(255, 100, 50), "Consuming"),
                                        ],
                                    );
                                }
                                FluidOverlay::Sound => {
                                    ui.label(
                                        egui::RichText::new("Compression (+)").size(10.0).strong(),
                                    );
                                    grad(
                                        ui,
                                        &[
                                            (
                                                egui::Color32::from_rgb(40, 40, 40),
                                                "10 dB  Leaves rustling",
                                            ),
                                            (
                                                egui::Color32::from_rgb(60, 70, 55),
                                                "40 dB  Home noise",
                                            ),
                                            (
                                                egui::Color32::from_rgb(160, 140, 20),
                                                "60 dB  Conversation",
                                            ),
                                            (
                                                egui::Color32::from_rgb(200, 100, 10),
                                                "80 dB  Alarm bell",
                                            ),
                                            (
                                                egui::Color32::from_rgb(255, 50, 10),
                                                "100 dB Gunshot \u{26a0}",
                                            ),
                                            (
                                                egui::Color32::from_rgb(220, 20, 80),
                                                "120 dB Thunder",
                                            ),
                                            (
                                                egui::Color32::from_rgb(140, 20, 200),
                                                "140 dB Blast wave",
                                            ),
                                            (
                                                egui::Color32::from_rgb(255, 255, 200),
                                                "180 dB Explosion",
                                            ),
                                        ],
                                    );
                                    ui.add_space(2.0);
                                    ui.label(
                                        egui::RichText::new("Rarefaction (-)").size(10.0).strong(),
                                    );
                                    grad(
                                        ui,
                                        &[
                                            (egui::Color32::from_rgb(30, 40, 70), "Low vacuum"),
                                            (egui::Color32::from_rgb(20, 50, 130), "Medium vacuum"),
                                            (
                                                egui::Color32::from_rgb(40, 80, 180),
                                                "Strong rarefaction",
                                            ),
                                        ],
                                    );
                                    ui.add_space(2.0);
                                    ui.label(
                                        egui::RichText::new("\u{26a0} > 100 dB at pleb = damage")
                                            .size(9.0)
                                            .weak(),
                                    );
                                }
                                FluidOverlay::Terrain => {
                                    ui.label(
                                        egui::RichText::new("Terrain Type").size(10.0).strong(),
                                    );
                                    grad(
                                        ui,
                                        &[
                                            (egui::Color32::from_rgb(107, 92, 56), "Grass"),
                                            (egui::Color32::from_rgb(173, 168, 153), "Chalky"),
                                            (egui::Color32::from_rgb(115, 107, 97), "Rocky"),
                                            (egui::Color32::from_rgb(128, 97, 64), "Clay"),
                                            (egui::Color32::from_rgb(122, 117, 107), "Gravel"),
                                            (egui::Color32::from_rgb(56, 46, 31), "Peat"),
                                            (egui::Color32::from_rgb(77, 89, 56), "Marsh"),
                                            (egui::Color32::from_rgb(97, 77, 46), "Loam"),
                                        ],
                                    );
                                }
                                _ => {}
                            }
                            // Ventilation overlay legend
                            if self.show_pipe_overlay {
                                if self.fluid_overlay != FluidOverlay::None {
                                    ui.separator();
                                }
                                ui.label(egui::RichText::new("Ventilation").strong().size(10.0));
                                dot(ui, egui::Color32::from_rgb(50, 100, 230), "O\u{2082} rich");
                                dot(ui, egui::Color32::from_rgb(200, 180, 25), "CO\u{2082}");
                                dot(ui, egui::Color32::from_rgb(128, 128, 128), "Smoke");
                                dot(ui, egui::Color32::from_rgb(230, 50, 30), "Hot gas");
                                ui.label(
                                    egui::RichText::new("Brighter = more pressure")
                                        .size(9.0)
                                        .weak(),
                                );
                            }
                            // Liquid overlay legend
                            if self.show_liquid_overlay {
                                if self.fluid_overlay != FluidOverlay::None
                                    || self.show_pipe_overlay
                                {
                                    ui.separator();
                                }
                                ui.label(egui::RichText::new("Liquid").strong().size(10.0));
                                dot(ui, egui::Color32::from_rgb(50, 120, 200), "Low pressure");
                                dot(ui, egui::Color32::from_rgb(100, 180, 255), "High pressure");
                            }
                            if self.show_flow_overlay {
                                if self.fluid_overlay != FluidOverlay::None
                                    || self.show_pipe_overlay
                                    || self.show_liquid_overlay
                                {
                                    ui.separator();
                                }
                                ui.label(egui::RichText::new("Pipe Flow").strong().size(10.0));
                                dot(ui, egui::Color32::from_rgb(80, 155, 255), "Slow");
                                dot(ui, egui::Color32::from_rgb(200, 255, 100), "Medium");
                                dot(ui, egui::Color32::from_rgb(255, 100, 0), "Fast");
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new("Wire Current").strong().size(10.0));
                                dot(ui, egui::Color32::from_rgb(140, 100, 230), "Low");
                                dot(ui, egui::Color32::from_rgb(200, 255, 155), "Medium");
                                dot(ui, egui::Color32::from_rgb(255, 200, 255), "High");
                            }
                        });
                    });
            }
        }

        // Version label below layers menu
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
                        ui.add(
                            egui::Slider::new(&mut speed, 0.1..=5.0)
                                .text("Speed")
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
                if ui.button("Main Menu").clicked() {
                    self.game_state = GameState::MainMenu;
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
                        ui.separator();
                        if ui
                            .selectable_label(self.fog_enabled, "Fog of War")
                            .clicked()
                        {
                            self.fog_enabled = !self.fog_enabled;
                            self.fog_dirty = true;
                            if !self.fog_enabled {
                                self.fog_texture_data.iter_mut().for_each(|v| *v = 255);
                                self.fog_dirty = true;
                            }
                        }
                        if self.fog_enabled
                            && ui
                                .selectable_label(self.fog_start_explored, "Pre-revealed Map")
                                .clicked()
                        {
                            self.fog_start_explored = !self.fog_start_explored;
                            if self.fog_start_explored {
                                self.fog_explored.iter_mut().for_each(|v| *v = 255);
                            }
                            self.fog_prev_tiles.clear();
                            self.fog_dirty = true;
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
                ui.menu_button("Admin", |ui| {
                    let pleb_label = format!("Add Colonist ({}/{})", self.plebs.len(), MAX_PLEBS);
                    if ui.button(pleb_label).clicked() {
                        self.placing_pleb = !self.placing_pleb;
                        if self.placing_pleb {
                            self.build_tool = BuildTool::None;
                        }
                        ui.close();
                    }
                    if self.placing_pleb {
                        ui.label(egui::RichText::new("Click to place").weak().size(10.0));
                    }
                    ui.separator();
                    if ui.button("Place Redskull Enemy").clicked() {
                        self.placing_enemy = true;
                        self.placing_pleb = false;
                        self.build_tool = BuildTool::None;
                        ui.close();
                    }
                    if self.placing_enemy {
                        ui.label(
                            egui::RichText::new("Click to place enemy")
                                .weak()
                                .size(10.0),
                        );
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
        // --- Inventory window (RPG-style, toggle with I key or click pleb name) ---
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
                    let a = &pleb.appearance;
                    let shirt = [a.shirt_r, a.shirt_g, a.shirt_b];
                    let skin = [a.skin_r, a.skin_g, a.skin_b];
                    let hair = [a.hair_r, a.hair_g, a.hair_b];

                    // Inventory window content
                    egui::Window::new(format!("{} — Inventory", pleb_name))
                        .collapsible(false)
                        .resizable(false)
                        .default_pos(egui::pos2(400.0, 200.0))
                        .show(ctx, |ui| {
                            ui.horizontal(|ui| {
                                // ═══ LEFT PANE: Character Sheet ═══
                                ui.vertical(|ui| {
                                    ui.set_min_width(160.0);

                                    // Portrait
                                    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(140.0, 80.0), egui::Sense::hover());
                                    let painter = ui.painter_at(rect);
                                    painter.rect_filled(rect, 6.0, egui::Color32::from_rgb(28, 30, 35));
                                    let c = rect.center();
                                    let shirt_c = egui::Color32::from_rgb((shirt[0]*255.0) as u8, (shirt[1]*255.0) as u8, (shirt[2]*255.0) as u8);
                                    let skin_c = egui::Color32::from_rgb((skin[0]*255.0) as u8, (skin[1]*255.0) as u8, (skin[2]*255.0) as u8);
                                    let hair_c = egui::Color32::from_rgb((hair[0]*255.0) as u8, (hair[1]*255.0) as u8, (hair[2]*255.0) as u8);
                                    // Body
                                    painter.circle_filled(c + egui::Vec2::new(0.0, 14.0), 18.0, shirt_c);
                                    // Head
                                    painter.circle_filled(c + egui::Vec2::new(0.0, -8.0), 12.0, skin_c);
                                    // Hair
                                    painter.circle_filled(c + egui::Vec2::new(0.0, -18.0), 7.0, hair_c);
                                    // Mood text under portrait
                                    let mood_col = if mood > 20.0 { egui::Color32::from_rgb(100, 200, 100) }
                                        else if mood > -20.0 { egui::Color32::from_rgb(180, 180, 120) }
                                        else { egui::Color32::from_rgb(200, 80, 80) };
                                    painter.text(egui::pos2(c.x, rect.max.y - 8.0), egui::Align2::CENTER_BOTTOM,
                                        mood_l, egui::FontId::proportional(10.0), mood_col);

                                    ui.add_space(4.0);

                                    // Equipment slots
                                    ui.label(egui::RichText::new("Equipment").size(10.0).strong());
                                    let equip_slot = |ui: &mut egui::Ui, label: &str, cat_filter: &str, slot_offset: usize| -> Option<usize> {
                                        let mut clicked = None;
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(label).size(9.0).weak());
                                            let slot_idx = slot_offset;
                                            let stack = if let Some(sel) = self.selected_pleb {
                                                self.plebs.get(sel).and_then(|p| {
                                                    p.inventory.stacks.iter().enumerate()
                                                        .find(|(_, s)| {
                                                            let def = item_defs::ItemRegistry::cached().get(s.item_id);
                                                            def.map(|d| d.category.as_str() == cat_filter).unwrap_or(false)
                                                        })
                                                        .map(|(i, s)| (i, s.clone()))
                                                })
                                            } else { None };
                                            let is_selected = self.inv_selected_slot == Some(slot_idx + 100); // offset to distinguish
                                            let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(36.0), egui::Sense::click());
                                            let painter = ui.painter_at(rect);
                                            let bg = if is_selected { egui::Color32::from_rgb(60, 80, 110) }
                                                else if response.hovered() { egui::Color32::from_rgb(50, 54, 62) }
                                                else { egui::Color32::from_rgb(35, 38, 44) };
                                            painter.rect_filled(rect, 4.0, bg);
                                            painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.0,
                                                if is_selected { egui::Color32::from_rgb(120, 160, 220) }
                                                else { egui::Color32::from_gray(55) }
                                            ), egui::StrokeKind::Outside);
                                            if let Some((_, ref s)) = stack {
                                                let icon = item_defs::ItemRegistry::cached().get(s.item_id)
                                                    .map(|d| d.icon.as_str()).unwrap_or("?");
                                                painter.text(rect.center(), egui::Align2::CENTER_CENTER,
                                                    icon, egui::FontId::proportional(16.0), egui::Color32::WHITE);
                                                // Liquid bar
                                                if let Some((_, amt)) = s.liquid {
                                                    let cap = s.liquid_capacity();
                                                    if cap > 0 {
                                                        let fill = amt as f32 / cap as f32;
                                                        let br = egui::Rect::from_min_size(
                                                            egui::pos2(rect.min.x + 2.0, rect.max.y - 4.0),
                                                            egui::vec2((rect.width() - 4.0) * fill, 3.0));
                                                        painter.rect_filled(br, 1.0, egui::Color32::from_rgb(60, 140, 220));
                                                    }
                                                }
                                                response.clone().on_hover_text(s.label());
                                            }
                                            if response.clicked() { clicked = stack.map(|(i, _)| i); }
                                        });
                                        clicked
                                    };
                                    let _tool_click = equip_slot(ui, "Tool ", "tool", 0);
                                    let _cont_click = equip_slot(ui, "Flask", "container", 1);

                                    ui.add_space(4.0);

                                    // Stats bars
                                    ui.label(egui::RichText::new("Vitals").size(10.0).strong());
                                    let bar = |ui: &mut egui::Ui, label: &str, val: f32, color: egui::Color32| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(label).size(9.0).monospace());
                                            let bar_w = 90.0;
                                            let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(bar_w, 8.0), egui::Sense::hover());
                                            let painter = ui.painter_at(rect);
                                            painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(25, 25, 28));
                                            painter.rect_filled(
                                                egui::Rect::from_min_size(rect.min, egui::Vec2::new(bar_w * val.clamp(0.0, 1.0), 8.0)),
                                                2.0, color);
                                        });
                                    };
                                    bar(ui, "HP ", health, egui::Color32::from_rgb(200, 60, 60));
                                    bar(ui, "FOD", hunger, egui::Color32::from_rgb(200, 160, 40));
                                    bar(ui, "H2O", thirst, egui::Color32::from_rgb(60, 140, 220));
                                    bar(ui, "RST", rest, egui::Color32::from_rgb(80, 120, 200));
                                    bar(ui, "WRM", warmth, egui::Color32::from_rgb(200, 100, 40));
                                    bar(ui, "O2 ", oxygen, egui::Color32::from_rgb(100, 200, 220));
                                    // Stress bar (inverted: high stress = bad)
                                    let stress_norm = (self.plebs.get(sel_idx).map(|p| p.needs.stress).unwrap_or(0.0) / 100.0).clamp(0.0, 1.0);
                                    let stress_col = if stress_norm < 0.5 { egui::Color32::from_rgb(80, 180, 80) }
                                        else if stress_norm < 0.7 { egui::Color32::from_rgb(200, 180, 60) }
                                        else { egui::Color32::from_rgb(200, 60, 60) };
                                    bar(ui, "STR", stress_norm, stress_col);
                                });

                                ui.separator();

                                // ═══ RIGHT PANE: Backpack Grid ═══
                                ui.vertical(|ui| {
                                    ui.set_min_width(200.0);
                                    ui.label(egui::RichText::new("Backpack").size(10.0).strong());

                                    let slot_size = 44.0;
                                    let cols = 4usize;
                                    let rows = 3usize;
                                    let total_slots = cols * rows;
                                    let selected = self.inv_selected_slot;

                                    let stacks: Vec<Option<item_defs::ItemStack>> = (0..total_slots)
                                        .map(|i| {
                                            if let Some(sel) = self.selected_pleb {
                                                self.plebs.get(sel)
                                                    .and_then(|p| p.inventory.stacks.get(i).cloned())
                                            } else { None }
                                        }).collect();

                                    let item_reg = item_defs::ItemRegistry::cached();
                                    let mut clicked_slot: Option<usize> = None;

                                    for row in 0..rows {
                                        ui.horizontal(|ui| {
                                            for col in 0..cols {
                                                let slot_idx = row * cols + col;
                                                let is_selected = selected == Some(slot_idx);
                                                let (rect, response) = ui.allocate_exact_size(
                                                    egui::Vec2::splat(slot_size), egui::Sense::click());
                                                let painter = ui.painter_at(rect);
                                                let bg = if is_selected { egui::Color32::from_rgb(55, 75, 105) }
                                                    else if response.hovered() { egui::Color32::from_rgb(48, 52, 60) }
                                                    else { egui::Color32::from_rgb(35, 38, 44) };
                                                painter.rect_filled(rect, 4.0, bg);
                                                painter.rect_stroke(rect, 4.0, egui::Stroke::new(
                                                    if is_selected { 2.0 } else { 1.0 },
                                                    if is_selected { egui::Color32::from_rgb(120, 160, 220) }
                                                    else { egui::Color32::from_gray(55) }
                                                ), egui::StrokeKind::Outside);

                                                if let Some(stack) = &stacks[slot_idx] {
                                                    let def = item_reg.get(stack.item_id);
                                                    let icon = def.map(|d| d.icon.as_str()).unwrap_or("?");
                                                    let cat = def.map(|d| d.category.as_str()).unwrap_or("");
                                                    // Category stripe
                                                    let stripe_col = match cat {
                                                        "tool" => Some(egui::Color32::from_rgb(70, 120, 70)),
                                                        "container" => Some(egui::Color32::from_rgb(50, 110, 170)),
                                                        "food" => Some(egui::Color32::from_rgb(170, 120, 40)),
                                                        _ => None,
                                                    };
                                                    if let Some(sc) = stripe_col {
                                                        painter.rect_filled(
                                                            egui::Rect::from_min_size(rect.min, egui::Vec2::new(rect.width(), 3.0)),
                                                            0.0, sc);
                                                    }
                                                    // Icon
                                                    painter.text(rect.center() + egui::Vec2::new(0.0, -2.0),
                                                        egui::Align2::CENTER_CENTER,
                                                        icon, egui::FontId::proportional(18.0), egui::Color32::WHITE);
                                                    // Count
                                                    if stack.count > 1 {
                                                        painter.text(rect.right_bottom() + egui::Vec2::new(-4.0, -2.0),
                                                            egui::Align2::RIGHT_BOTTOM,
                                                            format!("{}", stack.count),
                                                            egui::FontId::proportional(10.0),
                                                            egui::Color32::from_gray(200));
                                                    }
                                                    // Liquid bar
                                                    if let Some((_, amt)) = stack.liquid {
                                                        let cap = stack.liquid_capacity();
                                                        if cap > 0 {
                                                            let fill = amt as f32 / cap as f32;
                                                            let br = egui::Rect::from_min_size(
                                                                egui::pos2(rect.min.x + 2.0, rect.max.y - 5.0),
                                                                egui::vec2((rect.width() - 4.0) * fill, 3.0));
                                                            painter.rect_filled(br, 1.0, egui::Color32::from_rgb(60, 140, 220));
                                                        }
                                                    }
                                                    response.clone().on_hover_text(stack.label());
                                                }
                                                if response.clicked() { clicked_slot = Some(slot_idx); }
                                            }
                                        });
                                    }

                                    // Handle slot clicks
                                    if let Some(clicked) = clicked_slot {
                                        if let Some(prev) = self.inv_selected_slot {
                                            if prev == clicked {
                                                self.inv_selected_slot = None;
                                            } else {
                                                if let Some(sel_idx) = self.selected_pleb
                                                    && let Some(pleb) = self.plebs.get_mut(sel_idx)
                                                {
                                                    while pleb.inventory.stacks.len() <= prev.max(clicked) {
                                                        pleb.inventory.stacks.push(item_defs::ItemStack::new(0, 0));
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
                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        let has_sel = selected.is_some()
                                            && selected.map(|s| stacks.get(s).and_then(|x| x.as_ref()).is_some()).unwrap_or(false);
                                        if ui.add_enabled(has_sel, egui::Button::new(
                                            egui::RichText::new("\u{2b07} Drop").size(10.0)
                                        )).clicked()
                                            && let Some(slot) = selected
                                        {
                                            if let Some(sel_idx) = self.selected_pleb
                                                && let Some(pleb) = self.plebs.get_mut(sel_idx)
                                                && slot < pleb.inventory.stacks.len()
                                            {
                                                let stack = pleb.inventory.stacks.remove(slot);
                                                self.ground_items.push(resources::GroundItem {
                                                    x: pleb.x, y: pleb.y, stack,
                                                });
                                            }
                                            self.inv_selected_slot = None;
                                        }
                                        // Eat button (if selected item is food)
                                        let is_food = selected.and_then(|s| stacks.get(s))
                                            .and_then(|s| s.as_ref())
                                            .map(|s| item_reg.nutrition(s.item_id) > 0.0)
                                            .unwrap_or(false);
                                        if ui.add_enabled(is_food, egui::Button::new(
                                            egui::RichText::new("\u{1f374} Eat").size(10.0)
                                        )).clicked() {
                                            if let Some(sel_idx) = self.selected_pleb
                                                && let Some(pleb) = self.plebs.get_mut(sel_idx)
                                                && let Some(slot) = selected
                                                && slot < pleb.inventory.stacks.len()
                                            {
                                                let nutr = item_reg.nutrition(pleb.inventory.stacks[slot].item_id);
                                                pleb.needs.hunger = (pleb.needs.hunger + nutr).min(1.0);
                                                pleb.inventory.stacks[slot].count -= 1;
                                                if pleb.inventory.stacks[slot].count == 0 {
                                                    pleb.inventory.stacks.remove(slot);
                                                }
                                            }
                                            self.inv_selected_slot = None;
                                        }
                                    });
                                });
                            });
                        });
                    // Window closed via egui title bar X
                } else {
                    self.show_inventory = false;
                }
            } else {
                self.show_inventory = false;
            }
        }
    }

    fn draw_build_bar(&mut self, ctx: &egui::Context) {
        // --- Build categories (bottom-left, vertical 2-column grid, flows upward) ---
        let cat_s = 14.0;
        let mut categories: Vec<(&str, &str)> = vec![
            ("Walls", "\u{1f9f1}"),
            ("Floor", "\u{2b1c}"),
            ("Roof", "\u{1f3e0}"),
            ("Build", "\u{1f528}"),
            ("Craft", "\u{2692}"),
            ("Light", "\u{1f4a1}"),
            ("Power", "\u{26a1}"),
            ("Gas", "\u{1f4a8}"),
            ("Liquid", "\u{1f4a7}"),
            ("Zones", "\u{1f33e}"),
        ];
        if self.sandbox_mode {
            categories.push(("Sandbox", "\u{1f9ea}"));
        }

        egui::Area::new(egui::Id::new("build_categories"))
            .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
            .show(ctx, |ui| {
                egui::Frame::window(ui.style()).show(ui, |ui| {
                    // Tool buttons at top
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(
                                self.build_tool == BuildTool::Destroy,
                                egui::RichText::new("\u{274c} Destroy").size(cat_s),
                            )
                            .clicked()
                        {
                            self.build_tool = if self.build_tool == BuildTool::Destroy {
                                BuildTool::None
                            } else {
                                BuildTool::Destroy
                            };
                            self.build_category = None;
                        }
                        if ui
                            .selectable_label(
                                self.build_tool == BuildTool::Dig,
                                egui::RichText::new("\u{26cf} Dig").size(cat_s),
                            )
                            .clicked()
                        {
                            self.build_tool = if self.build_tool == BuildTool::Dig {
                                BuildTool::None
                            } else {
                                BuildTool::Dig
                            };
                            self.build_category = None;
                        }
                    });
                    ui.separator();
                    // 2-column category grid
                    egui::Grid::new("build_cat_grid")
                        .num_columns(2)
                        .spacing([4.0, 2.0])
                        .show(ui, |ui| {
                            for (i, &(name, icon)) in categories.iter().enumerate() {
                                let selected = self.build_category == Some(name);
                                let label = format!("{} {}", icon, name);
                                if ui
                                    .selectable_label(
                                        selected,
                                        egui::RichText::new(label).size(cat_s),
                                    )
                                    .clicked()
                                {
                                    if selected {
                                        self.build_category = None;
                                        self.build_tool = BuildTool::None;
                                        self.sandbox_tool = SandboxTool::None;
                                    } else {
                                        self.build_category = Some(name);
                                        self.world_sel = WorldSelection::none();
                                        self.selected_pleb = None;
                                        if name == "Sandbox" {
                                            self.build_tool = BuildTool::None;
                                        } else {
                                            self.sandbox_tool = SandboxTool::None;
                                        }
                                    }
                                }
                                if i % 2 == 1 {
                                    ui.end_row();
                                }
                            }
                        });
                });
            });

        // --- Build items / Selection actions panel (center bottom, single column) ---
        // Shows build tools when a category is active, or selection actions when items are selected.
        // These are mutually exclusive: selecting something closes build menu.
        let has_selection = !self.world_sel.is_empty();
        let show_build = self.build_category.is_some() && !has_selection;

        if show_build || has_selection {
            let cat = self.build_category;
            egui::Area::new(egui::Id::new("build_items"))
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -10.0])
                .show(ctx, |ui| {
                    egui::Frame::window(ui.style()).show(ui, |ui| {
                        if has_selection {
                            // --- Selection action buttons ---
                            self.draw_selection_actions_inner(ui);
                        } else if let Some(cat) = cat {
                            // --- Build tool items ---
                            let tile_size = 60.0;
                            let icon_s = 24.0;
                            let label_s = 11.0;

                            // Count items per category for 1 vs 2 column layout
                            let item_count: usize = match cat {
                                "Walls" => 2,
                                "Floor" => 4,
                                "Roof" => 2,
                                "Build" => 9,
                                "Craft" => 2,
                                "Light" => 6,
                                "Power" => 10,
                                "Gas" => 9,
                                "Liquid" => 5,
                                "Zones" => 2,
                                _ => 5,
                            };
                            let items_per_row = if item_count > 10 {
                                item_count.div_ceil(2)
                            } else {
                                item_count
                            };
                            // Horizontal rows, left-to-right, wrapping to 2nd row if >10
                            egui::Grid::new("build_items_grid")
                                .num_columns(items_per_row)
                                .spacing([4.0, 4.0])
                                .show(ui, |ui| {
                                    // Rebind icon_btn to add end_row tracking
                                    let col_counter = std::cell::Cell::new(0usize);
                                    let mut icon_btn =
                                        |ui: &mut egui::Ui,
                                         t: BuildTool,
                                         icon: &str,
                                         label: &str| {
                                            let selected = self.build_tool == t;
                                            let (rect, response) = ui.allocate_exact_size(
                                                egui::Vec2::splat(tile_size),
                                                egui::Sense::click(),
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
                                            painter.rect_stroke(
                                                rect,
                                                4.0,
                                                egui::Stroke::new(
                                                    1.0,
                                                    egui::Color32::from_gray(70),
                                                ),
                                                egui::StrokeKind::Outside,
                                            );
                                            painter.text(
                                                rect.center() + egui::Vec2::new(0.0, -6.0),
                                                egui::Align2::CENTER_CENTER,
                                                icon,
                                                egui::FontId::proportional(icon_s),
                                                egui::Color32::WHITE,
                                            );
                                            painter.text(
                                                rect.center() + egui::Vec2::new(0.0, 14.0),
                                                egui::Align2::CENTER_CENTER,
                                                label,
                                                egui::FontId::proportional(label_s),
                                                egui::Color32::from_gray(190),
                                            );
                                            if response.clicked() {
                                                self.build_tool = if self.build_tool == t {
                                                    BuildTool::None
                                                } else {
                                                    t
                                                };
                                            }
                                            let c = col_counter.get() + 1;
                                            col_counter.set(c);
                                            if c.is_multiple_of(items_per_row) {
                                                ui.end_row();
                                            }
                                        };
                                    match cat {
                                        "Walls" => {
                                            icon_btn(ui, BuildTool::Place(35), "\u{1f3da}", "Mud");
                                            icon_btn(ui, BuildTool::Place(21), "\u{1fab5}", "Wood");
                                        }
                                        "Floor" => {
                                            icon_btn(ui, BuildTool::Place(26), "\u{1fab5}", "Wood");
                                            icon_btn(ui, BuildTool::Place(27), "\u{2b1b}", "Stone");
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(28),
                                                "\u{2b1c}",
                                                "Flagstone",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::RemoveFloor,
                                                "\u{274c}",
                                                "Remove",
                                            );
                                        }
                                        "Roof" => {
                                            icon_btn(ui, BuildTool::Roof, "\u{1f3da}", "Thatch");
                                            icon_btn(
                                                ui,
                                                BuildTool::RemoveRoof,
                                                "\u{274c}",
                                                "Remove",
                                            );
                                        }
                                        "Build" => {
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(62),
                                                "\u{1fab5}",
                                                "Campfire",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(6),
                                                "\u{1f525}",
                                                "Fireplace",
                                            );
                                            icon_btn(ui, BuildTool::Place(9), "\u{1fa91}", "Bench");
                                            icon_btn(ui, BuildTool::Place(30), "\u{1f6cf}", "Bed");
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(33),
                                                "\u{1f4e6}",
                                                "Crate",
                                            );
                                            icon_btn(ui, BuildTool::Window, "\u{1fa9f}", "Window");
                                            icon_btn(ui, BuildTool::Door, "\u{1f6aa}", "Door");
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(29),
                                                "\u{1f4a5}",
                                                "Cannon",
                                            );
                                            icon_btn(ui, BuildTool::Place(59), "\u{1fa63}", "Well");
                                        }
                                        "Craft" => {
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(57),
                                                "\u{1f528}",
                                                "Workbench",
                                            );
                                            icon_btn(ui, BuildTool::Place(58), "\u{1f3ed}", "Kiln");
                                        }
                                        "Light" => {
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(7),
                                                "\u{1f4a1}",
                                                "Ceiling",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(10),
                                                "\u{1f9f4}",
                                                "Floor Lamp",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(11),
                                                "\u{1f4a1}",
                                                "Table Lamp",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(55),
                                                "\u{1f525}",
                                                "Wall Torch",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(56),
                                                "\u{1f4a1}",
                                                "Wall Lamp",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(48),
                                                "\u{1f526}",
                                                "Floodlight",
                                            );
                                        }
                                        "Power" => {
                                            icon_btn(ui, BuildTool::Place(36), "\u{26a1}", "Wire");
                                            icon_btn(ui, BuildTool::Place(37), "\u{2600}", "Solar");
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(38),
                                                "\u{1f50b}",
                                                "Bat S",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(39),
                                                "\u{1f50b}",
                                                "Bat M",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(40),
                                                "\u{1f50b}",
                                                "Bat L",
                                            );
                                            icon_btn(ui, BuildTool::Place(41), "\u{1f300}", "Wind");
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(42),
                                                "\u{1f518}",
                                                "Switch",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(43),
                                                "\u{1f39a}",
                                                "Dimmer",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(45),
                                                "\u{1f6d1}",
                                                "Breaker",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(51),
                                                "\u{2a2f}",
                                                "Bridge",
                                            );
                                        }
                                        "Gas" => {
                                            icon_btn(ui, BuildTool::Place(15), "\u{1f4a8}", "Pipe");
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(46),
                                                "\u{2298}",
                                                "Restrictor",
                                            );
                                            icon_btn(ui, BuildTool::Place(16), "\u{2699}", "Pump");
                                            icon_btn(ui, BuildTool::Place(17), "\u{1f6e2}", "Tank");
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(18),
                                                "\u{1f504}",
                                                "Valve",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(19),
                                                "\u{27a1}",
                                                "Outlet",
                                            );
                                            icon_btn(ui, BuildTool::Place(20), "\u{2b05}", "Inlet");
                                            icon_btn(ui, BuildTool::Place(12), "\u{1f32c}", "Fan");
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(50),
                                                "\u{2a2f}",
                                                "Bridge",
                                            );
                                        }
                                        "Liquid" => {
                                            icon_btn(ui, BuildTool::Place(49), "\u{1f4a7}", "Pipe");
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(52),
                                                "\u{1f6b0}",
                                                "Intake",
                                            );
                                            icon_btn(ui, BuildTool::Place(53), "\u{2699}", "Pump");
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(54),
                                                "\u{1f4a6}",
                                                "Output",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(50),
                                                "\u{2a2f}",
                                                "Bridge",
                                            );
                                        }
                                        "Zones" => {
                                            icon_btn(
                                                ui,
                                                BuildTool::GrowingZone,
                                                "\u{1f33f}",
                                                "Farm",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::StorageZone,
                                                "\u{1f4e6}",
                                                "Storage",
                                            );
                                        }
                                        "Sandbox" if self.sandbox_mode => {
                                            // handled below (outside icon_btn scope)
                                        }
                                        _ => {}
                                    }
                                });
                            // Zone work priority toggle (outside grid)
                            if self.build_category == Some("Zones") {
                                ui.separator();
                                let prio_label = match self.work_priority {
                                    zones::WorkPriority::PlantFirst => "Plant 1st",
                                    zones::WorkPriority::HarvestFirst => "Harvest 1st",
                                };
                                if ui.small_button(prio_label).clicked() {
                                    self.work_priority = match self.work_priority {
                                        zones::WorkPriority::PlantFirst => {
                                            zones::WorkPriority::HarvestFirst
                                        }
                                        zones::WorkPriority::HarvestFirst => {
                                            zones::WorkPriority::PlantFirst
                                        }
                                    };
                                }
                            }
                            // Sandbox tools (outside icon_btn scope)
                            if self.build_category == Some("Sandbox") && self.sandbox_mode {
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = egui::Vec2::new(4.0, 4.0);
                                    let sel = self.sandbox_tool == SandboxTool::Lightning;
                                    let (rect, response) = ui.allocate_exact_size(
                                        egui::Vec2::splat(60.0),
                                        egui::Sense::click(),
                                    );
                                    let painter = ui.painter_at(rect);
                                    let bg = if sel {
                                        egui::Color32::from_rgb(60, 80, 110)
                                    } else if response.hovered() {
                                        egui::Color32::from_rgb(55, 58, 65)
                                    } else {
                                        egui::Color32::from_rgb(40, 42, 48)
                                    };
                                    painter.rect_filled(rect, 4.0, bg);
                                    painter.rect_stroke(
                                        rect,
                                        4.0,
                                        egui::Stroke::new(1.0, egui::Color32::from_gray(70)),
                                        egui::StrokeKind::Outside,
                                    );
                                    painter.text(
                                        rect.center() + egui::Vec2::new(0.0, -6.0),
                                        egui::Align2::CENTER_CENTER,
                                        "\u{26a1}",
                                        egui::FontId::proportional(24.0),
                                        egui::Color32::YELLOW,
                                    );
                                    painter.text(
                                        rect.center() + egui::Vec2::new(0.0, 14.0),
                                        egui::Align2::CENTER_CENTER,
                                        "Lightning",
                                        egui::FontId::proportional(11.0),
                                        egui::Color32::from_gray(190),
                                    );
                                    if response.clicked() {
                                        self.sandbox_tool = if sel {
                                            SandboxTool::None
                                        } else {
                                            SandboxTool::Lightning
                                        };
                                        if self.sandbox_tool != SandboxTool::None {
                                            self.build_tool = BuildTool::None;
                                        }
                                    }
                                    // Water inject button
                                    let sel_w = self.sandbox_tool == SandboxTool::InjectWater;
                                    let (rect_w, resp_w) = ui.allocate_exact_size(
                                        egui::Vec2::splat(60.0),
                                        egui::Sense::click(),
                                    );
                                    let pw = ui.painter_at(rect_w);
                                    let bg_w = if sel_w {
                                        egui::Color32::from_rgb(40, 70, 110)
                                    } else if resp_w.hovered() {
                                        egui::Color32::from_rgb(55, 58, 65)
                                    } else {
                                        egui::Color32::from_rgb(40, 42, 48)
                                    };
                                    pw.rect_filled(rect_w, 4.0, bg_w);
                                    pw.rect_stroke(
                                        rect_w,
                                        4.0,
                                        egui::Stroke::new(1.0, egui::Color32::from_gray(70)),
                                        egui::StrokeKind::Outside,
                                    );
                                    pw.text(
                                        rect_w.center() + egui::Vec2::new(0.0, -6.0),
                                        egui::Align2::CENTER_CENTER,
                                        "\u{1f4a7}",
                                        egui::FontId::proportional(24.0),
                                        egui::Color32::from_rgb(80, 150, 255),
                                    );
                                    pw.text(
                                        rect_w.center() + egui::Vec2::new(0.0, 14.0),
                                        egui::Align2::CENTER_CENTER,
                                        "Water",
                                        egui::FontId::proportional(11.0),
                                        egui::Color32::from_gray(190),
                                    );
                                    if resp_w.clicked() {
                                        self.sandbox_tool = if sel_w {
                                            SandboxTool::None
                                        } else {
                                            SandboxTool::InjectWater
                                        };
                                        if self.sandbox_tool != SandboxTool::None {
                                            self.build_tool = BuildTool::None;
                                        }
                                    }
                                    // Drought button (click to trigger, not a click-on-map tool)
                                    let drought_active = self.has_condition("Drought");
                                    let drought_label = if drought_active {
                                        "End Drought"
                                    } else {
                                        "Drought"
                                    };
                                    let (rect_d, resp_d) = ui.allocate_exact_size(
                                        egui::Vec2::splat(60.0),
                                        egui::Sense::click(),
                                    );
                                    let pd = ui.painter_at(rect_d);
                                    let bg_d = if drought_active {
                                        egui::Color32::from_rgb(120, 60, 30)
                                    } else if resp_d.hovered() {
                                        egui::Color32::from_rgb(55, 58, 65)
                                    } else {
                                        egui::Color32::from_rgb(40, 42, 48)
                                    };
                                    pd.rect_filled(rect_d, 4.0, bg_d);
                                    pd.rect_stroke(
                                        rect_d,
                                        4.0,
                                        egui::Stroke::new(1.0, egui::Color32::from_gray(70)),
                                        egui::StrokeKind::Outside,
                                    );
                                    pd.text(
                                        rect_d.center() + egui::Vec2::new(0.0, -6.0),
                                        egui::Align2::CENTER_CENTER,
                                        "\u{2600}",
                                        egui::FontId::proportional(24.0),
                                        egui::Color32::from_rgb(255, 200, 50),
                                    );
                                    pd.text(
                                        rect_d.center() + egui::Vec2::new(0.0, 14.0),
                                        egui::Align2::CENTER_CENTER,
                                        drought_label,
                                        egui::FontId::proportional(11.0),
                                        egui::Color32::from_gray(190),
                                    );
                                    if resp_d.clicked() {
                                        if drought_active {
                                            self.conditions.retain(|c| c.name != "Drought");
                                            self.log_event(
                                                EventCategory::Weather,
                                                "Drought ended (sandbox)".to_string(),
                                            );
                                        } else {
                                            self.add_condition(
                                                "Drought",
                                                "\u{2600}",
                                                NotifCategory::Threat,
                                                90.0,
                                            );
                                            self.notify(
                                                NotifCategory::Threat,
                                                "\u{2600}",
                                                "Drought",
                                                "Sandbox: Drought triggered!",
                                            );
                                            self.log_event(
                                                EventCategory::Weather,
                                                "Drought triggered (sandbox)".to_string(),
                                            );
                                        }
                                    }

                                    // Ignite tool (click flammable block to set on fire)
                                    let sel_ign = self.sandbox_tool == SandboxTool::Ignite;
                                    let (rect_ign, resp_ign) = ui.allocate_exact_size(
                                        egui::Vec2::splat(60.0),
                                        egui::Sense::click(),
                                    );
                                    let pi = ui.painter_at(rect_ign);
                                    let bg_ign = if sel_ign {
                                        egui::Color32::from_rgb(140, 60, 20)
                                    } else if resp_ign.hovered() {
                                        egui::Color32::from_rgb(55, 58, 65)
                                    } else {
                                        egui::Color32::from_rgb(40, 42, 48)
                                    };
                                    pi.rect_filled(rect_ign, 4.0, bg_ign);
                                    pi.rect_stroke(
                                        rect_ign,
                                        4.0,
                                        egui::Stroke::new(1.0, egui::Color32::from_gray(70)),
                                        egui::StrokeKind::Outside,
                                    );
                                    pi.text(
                                        rect_ign.center() + egui::Vec2::new(0.0, -6.0),
                                        egui::Align2::CENTER_CENTER,
                                        "\u{1f525}",
                                        egui::FontId::proportional(24.0),
                                        egui::Color32::from_rgb(255, 120, 30),
                                    );
                                    pi.text(
                                        rect_ign.center() + egui::Vec2::new(0.0, 14.0),
                                        egui::Align2::CENTER_CENTER,
                                        "Ignite",
                                        egui::FontId::proportional(11.0),
                                        egui::Color32::from_gray(190),
                                    );
                                    if resp_ign.clicked() {
                                        self.sandbox_tool = if sel_ign {
                                            SandboxTool::None
                                        } else {
                                            SandboxTool::Ignite
                                        };
                                        if self.sandbox_tool != SandboxTool::None {
                                            self.build_tool = BuildTool::None;
                                        }
                                    }
                                    if sel_ign {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("Intensity")
                                                    .size(10.0)
                                                    .color(egui::Color32::from_gray(160)),
                                            );
                                            let slider = egui::Slider::new(
                                                &mut self.fire_intensity,
                                                0.2..=3.0,
                                            )
                                            .step_by(0.1)
                                            .show_value(true);
                                            ui.add(slider);
                                        });
                                    }

                                    // Sound placement tools (only when sound system is enabled)
                                    if self.sound_enabled {
                                        ui.separator();
                                        for (i, &(name, db, _, _, _)) in
                                            SANDBOX_SOUNDS.iter().enumerate()
                                        {
                                            let sel =
                                                self.sandbox_tool == SandboxTool::SoundPlace(i);
                                            let label = format!("{:.0}dB {}", db, name);
                                            if ui.selectable_label(sel, &label).clicked() {
                                                self.sandbox_tool = if sel {
                                                    SandboxTool::None
                                                } else {
                                                    SandboxTool::SoundPlace(i)
                                                };
                                                if self.sandbox_tool != SandboxTool::None {
                                                    self.build_tool = BuildTool::None;
                                                }
                                            }
                                        }
                                    }
                                });
                            }

                            // Hint bar below icons
                            let tool = &self.build_tool;
                            if *tool != BuildTool::None {
                                ui.separator();
                                let hint = match tool {
                                    BuildTool::Place(9)
                                    | BuildTool::Place(30)
                                    | BuildTool::Place(39) => {
                                        let r = if self.build_rotation == 0 { "H" } else { "V" };
                                        format!("Q/E [{}]", r)
                                    }
                                    BuildTool::Place(41) => {
                                        let d = if self.build_rotation.is_multiple_of(2) {
                                            "N↔S wind"
                                        } else {
                                            "E↔W wind"
                                        };
                                        format!("Q/E [{}]", d)
                                    }
                                    BuildTool::Place(11) => "On bench".to_string(),
                                    BuildTool::Place(12)
                                    | BuildTool::Place(16)
                                    | BuildTool::Place(20)
                                    | BuildTool::Place(19)
                                    | BuildTool::Place(29) => {
                                        let d = match self.build_rotation {
                                            0 => "N",
                                            1 => "E",
                                            2 => "S",
                                            _ => "W",
                                        };
                                        format!("Q/E [{}]", d)
                                    }
                                    BuildTool::Destroy
                                    | BuildTool::RemoveFloor
                                    | BuildTool::RemoveRoof => "Click/drag".to_string(),
                                    BuildTool::WoodBox => "Click to drop".to_string(),
                                    BuildTool::Window | BuildTool::Door => "Click wall".to_string(),
                                    BuildTool::Roof => "Drag (needs support)".to_string(),
                                    BuildTool::Dig => "Click to dig 20%".to_string(),
                                    _ => "Click/drag".to_string(),
                                };
                                ui.label(egui::RichText::new(hint).weak().size(13.0));
                            }
                        } // end else (build tools)
                    }); // Frame
                }); // Area
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

            // Draft button below colonist bar (when a pleb is selected)
            if let Some(sel_idx) = self.selected_pleb {
                if let Some(pleb) = self.plebs.get(sel_idx) {
                    if !pleb.is_enemy {
                        let is_drafted = pleb.drafted;
                        egui::Area::new(egui::Id::new("draft_button"))
                            .anchor(egui::Align2::CENTER_TOP, [0.0, 72.0])
                            .interactable(true)
                            .show(ctx, |ui| {
                                let (icon, label, col) = if is_drafted {
                                    ("\u{2694}", "Drafted", egui::Color32::from_rgb(220, 90, 60))
                                } else {
                                    ("\u{1f6e1}", "Draft", egui::Color32::from_rgb(120, 120, 120))
                                };
                                let btn_text = egui::RichText::new(format!("{} {}", icon, label))
                                    .size(13.0)
                                    .color(col);
                                let btn = ui
                                    .add_sized(egui::vec2(80.0, 26.0), egui::Button::new(btn_text));
                                if btn.clicked() {
                                    if let Some(p) = self.plebs.get_mut(sel_idx) {
                                        p.drafted = !p.drafted;
                                        if !p.drafted {
                                            p.work_target = None;
                                            p.haul_target = None;
                                            p.harvest_target = None;
                                        }
                                    }
                                }
                            });
                    }
                }
            }
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
                    if let Some(sel_idx) = self.selected_pleb {
                        let pleb = &mut self.plebs[sel_idx];
                        if !pleb.is_enemy && !pleb.activity.is_crisis() {
                            let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
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
                        if let Some((cx, cy)) = nearest_crate {
                            let pleb = &mut self.plebs[pi];
                            let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
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
                                self.selected_pleb = Some(pi);
                            }
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
                    }
                }
                ContextAction::MoveTo(wx, wy) => {
                    if let Some(sel_idx) = self.selected_pleb {
                        let pleb = &mut self.plebs[sel_idx];
                        let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
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
                }
                ContextAction::DigClay(dx, dy) => {
                    if let Some(sel_idx) = self.selected_pleb {
                        let pleb = &mut self.plebs[sel_idx];
                        let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
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
                    // Walk to tree, gather sticks (no axe, tree stays)
                    if let Some(sel_idx) = self.selected_pleb {
                        let pleb = &mut self.plebs[sel_idx];
                        let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                        let adj = adjacent_walkable(&self.grid_data, gx, gy).unwrap_or((gx, gy));
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

        // Info tool: hold Shift to inspect any block
        let ctrl_held = self.pressed_keys.contains(&KeyCode::ControlLeft)
            || self.pressed_keys.contains(&KeyCode::ControlRight);
        if ctrl_held {
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

    fn draw_world_overlays(
        &mut self,
        ctx: &egui::Context,
        bp_cam: (f32, f32, f32, f32, f32),
        blueprint_tiles: &[((i32, i32), u8)],
    ) {
        let bp_ppp = self.ppp();
        let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
        let tile_px = cam_zoom / self.render_scale / bp_ppp;

        // --- Grid overlay (G key) ---
        if self.show_grid && tile_px > 3.0 {
            let grid_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("grid_lines"),
            ));
            let _screen_rect = ctx.screen_rect();
            // Dark lines during day (contrast against bright ground), light at night
            let sun = self.camera.sun_intensity;
            let brightness = (200.0 * (1.0 - sun * 0.9)) as u8;
            let alpha = (15.0 + sun * 40.0) as u8;
            let grid_color =
                egui::Color32::from_rgba_unmultiplied(brightness, brightness, brightness, alpha);
            let grid_width = 1.0;
            // Compute visible tile range
            let min_x = ((cam_cx - cam_sw * 0.5 / cam_zoom).floor() as i32).max(0);
            let max_x = ((cam_cx + cam_sw * 0.5 / cam_zoom).ceil() as i32).min(GRID_W as i32);
            let min_y = ((cam_cy - cam_sh * 0.5 / cam_zoom).floor() as i32).max(0);
            let max_y = ((cam_cy + cam_sh * 0.5 / cam_zoom).ceil() as i32).min(GRID_H as i32);
            // Vertical lines
            for x in min_x..=max_x {
                let p0 = self.world_to_screen_ui(x as f32, min_y as f32, bp_cam);
                let p1 = self.world_to_screen_ui(x as f32, max_y as f32, bp_cam);
                grid_painter.line_segment([p0, p1], egui::Stroke::new(grid_width, grid_color));
            }
            // Horizontal lines
            for y in min_y..=max_y {
                let p0 = self.world_to_screen_ui(min_x as f32, y as f32, bp_cam);
                let p1 = self.world_to_screen_ui(max_x as f32, y as f32, bp_cam);
                grid_painter.line_segment([p0, p1], egui::Stroke::new(grid_width, grid_color));
            }
        }

        // --- Sub-grid overlay (Shift+G key) — 4x4 sub-grid around cursor ---
        if self.show_subgrid && tile_px > 8.0 {
            let subgrid_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("subgrid_lines"),
            ));
            let (hwx, hwy) = self.hover_world;
            let cx = hwx.floor() as i32;
            let cy = hwy.floor() as i32;
            let radius = 3i32;
            let sun = self.camera.sun_intensity;
            let sb = (200.0 * (1.0 - sun * 0.6)) as u8;
            let sub_color = egui::Color32::from_rgba_unmultiplied(
                sb,
                sb,
                (sb as f32 * 1.2).min(255.0) as u8,
                (20.0 + sun * 15.0) as u8,
            );
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let tx = cx + dx;
                    let ty = cy + dy;
                    if tx < 0 || ty < 0 || tx >= GRID_W as i32 || ty >= GRID_H as i32 {
                        continue;
                    }
                    // 4 sub-grid lines per tile (at 0.25, 0.5, 0.75)
                    for s in 1..4 {
                        let f = s as f32 * 0.25;
                        // Vertical sub-line
                        let p0 = self.world_to_screen_ui(tx as f32 + f, ty as f32, bp_cam);
                        let p1 = self.world_to_screen_ui(tx as f32 + f, ty as f32 + 1.0, bp_cam);
                        subgrid_painter.line_segment([p0, p1], egui::Stroke::new(1.0, sub_color));
                        // Horizontal sub-line
                        let p0 = self.world_to_screen_ui(tx as f32, ty as f32 + f, bp_cam);
                        let p1 = self.world_to_screen_ui(tx as f32 + 1.0, ty as f32 + f, bp_cam);
                        subgrid_painter.line_segment([p0, p1], egui::Stroke::new(1.0, sub_color));
                    }
                }
            }
        }

        // Selection drag rectangle (while dragging to multi-select)
        if let Some((sx, sy)) = self.select_drag_start {
            let (ex, ey) = self.hover_world;
            let p0 = self.world_to_screen_ui(sx.min(ex), sy.min(ey), bp_cam);
            let p1 = self.world_to_screen_ui(sx.max(ex), sy.max(ey), bp_cam);
            let sel_drag_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("select_drag"),
            ));
            sel_drag_painter.rect_filled(
                egui::Rect::from_min_max(p0, p1),
                0.0,
                egui::Color32::from_rgba_unmultiplied(100, 180, 255, 30),
            );
            sel_drag_painter.rect_filled(
                egui::Rect::from_min_max(p0, p1),
                0.0,
                egui::Color32::TRANSPARENT,
            );
            // Border
            let r = egui::Rect::from_min_max(p0, p1);
            let pts = [
                r.left_top(),
                r.right_top(),
                r.right_bottom(),
                r.left_bottom(),
                r.left_top(),
            ];
            for pair in pts.windows(2) {
                sel_drag_painter.line_segment(
                    [pair[0], pair[1]],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 180, 255)),
                );
            }
        }

        // World selection brackets (Rimworld-style corner markers per item)
        if !self.world_sel.is_empty() {
            let sel_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("world_selection"),
            ));
            let stroke = egui::Stroke::new(2.0, egui::Color32::WHITE);
            for item in &self.world_sel.items {
                // Plebs: use live position, not grid position
                let (wx0, wy0, wx1, wy1) = if let Some(pi) = item.pleb_idx {
                    if let Some(pleb) = self.plebs.get(pi) {
                        (pleb.x - 0.4, pleb.y - 0.4, pleb.x + 0.4, pleb.y + 0.4)
                    } else {
                        continue;
                    }
                } else {
                    (
                        item.x as f32,
                        item.y as f32,
                        (item.x + item.w) as f32,
                        (item.y + item.h) as f32,
                    )
                };
                let p0 = self.world_to_screen_ui(wx0, wy0, bp_cam);
                let p1 = self.world_to_screen_ui(wx1, wy1, bp_cam);
                let rect = egui::Rect::from_min_max(p0, p1);
                let bl = (rect.width().min(rect.height()) * 0.3).max(3.0);
                sel_painter.line_segment(
                    [rect.left_top(), rect.left_top() + egui::Vec2::new(bl, 0.0)],
                    stroke,
                );
                sel_painter.line_segment(
                    [rect.left_top(), rect.left_top() + egui::Vec2::new(0.0, bl)],
                    stroke,
                );
                sel_painter.line_segment(
                    [
                        rect.right_top(),
                        rect.right_top() + egui::Vec2::new(-bl, 0.0),
                    ],
                    stroke,
                );
                sel_painter.line_segment(
                    [
                        rect.right_top(),
                        rect.right_top() + egui::Vec2::new(0.0, bl),
                    ],
                    stroke,
                );
                sel_painter.line_segment(
                    [
                        rect.left_bottom(),
                        rect.left_bottom() + egui::Vec2::new(bl, 0.0),
                    ],
                    stroke,
                );
                sel_painter.line_segment(
                    [
                        rect.left_bottom(),
                        rect.left_bottom() + egui::Vec2::new(0.0, -bl),
                    ],
                    stroke,
                );
                sel_painter.line_segment(
                    [
                        rect.right_bottom(),
                        rect.right_bottom() + egui::Vec2::new(-bl, 0.0),
                    ],
                    stroke,
                );
                sel_painter.line_segment(
                    [
                        rect.right_bottom(),
                        rect.right_bottom() + egui::Vec2::new(0.0, -bl),
                    ],
                    stroke,
                );
            }
        }

        // Growing zone overlay — green tint on designated tiles
        if !self.zones.is_empty() {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let zone_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("zones"),
            ));
            for zone in &self.zones {
                let color = match zone.kind {
                    zones::ZoneKind::Growing => {
                        egui::Color32::from_rgba_unmultiplied(40, 160, 40, 35)
                    }
                    zones::ZoneKind::Storage => {
                        egui::Color32::from_rgba_unmultiplied(200, 80, 140, 35)
                    }
                };
                for &(tx, ty) in &zone.tiles {
                    let sx0 = ((tx as f32 - cam_cx) * cam_zoom + cam_sw * 0.5)
                        / self.render_scale
                        / bp_ppp;
                    let sy0 = ((ty as f32 - cam_cy) * cam_zoom + cam_sh * 0.5)
                        / self.render_scale
                        / bp_ppp;
                    let sx1 = ((tx as f32 + 1.0 - cam_cx) * cam_zoom + cam_sw * 0.5)
                        / self.render_scale
                        / bp_ppp;
                    let sy1 = ((ty as f32 + 1.0 - cam_cy) * cam_zoom + cam_sh * 0.5)
                        / self.render_scale
                        / bp_ppp;
                    zone_painter.rect_filled(
                        egui::Rect::from_min_max(egui::pos2(sx0, sy0), egui::pos2(sx1, sy1)),
                        0.0,
                        color,
                    );
                }
            }
        }

        // Construction blueprints — ghost blocks waiting to be built
        if !self.blueprints.is_empty() {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let tile_px = cam_zoom / self.render_scale / bp_ppp;
            let bp_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("construction_blueprints"),
            ));
            let screen_rect = ctx.content_rect();
            for (&(bx, by), bp) in &self.blueprints {
                let sx0 =
                    ((bx as f32 - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy0 =
                    ((by as f32 - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                let sx1 = ((bx as f32 + 1.0 - cam_cx) * cam_zoom + cam_sw * 0.5)
                    / self.render_scale
                    / bp_ppp;
                let sy1 = ((by as f32 + 1.0 - cam_cy) * cam_zoom + cam_sh * 0.5)
                    / self.render_scale
                    / bp_ppp;
                // Cull off-screen blueprints
                if sx1 < screen_rect.min.x
                    || sx0 > screen_rect.max.x
                    || sy1 < screen_rect.min.y
                    || sy0 > screen_rect.max.y
                {
                    continue;
                }
                let rect = egui::Rect::from_min_max(egui::pos2(sx0, sy0), egui::pos2(sx1, sy1));
                // Tint: blue if waiting for resources, green-blue if ready to build
                let tint = if bp.resources_met() {
                    egui::Color32::from_rgba_unmultiplied(60, 160, 120, 90)
                } else {
                    egui::Color32::from_rgba_unmultiplied(60, 120, 220, 90)
                };
                // Wall blueprint: render each edge strip
                if bp.is_wall() && bp.wall_thickness < 4 {
                    let wall_frac = bp.wall_thickness as f32 * 0.25;
                    let tw = sx1 - sx0;
                    let th = sy1 - sy0;
                    let edges = bp.wall_edges;
                    // Draw each edge that's set
                    for (bit, edge_idx) in [
                        (WD_EDGE_N, 0u8),
                        (WD_EDGE_E, 1),
                        (WD_EDGE_S, 2),
                        (WD_EDGE_W, 3),
                    ] {
                        if edges & bit != 0 {
                            let edge_rect = match edge_idx {
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
                            bp_painter.rect_filled(edge_rect, 0.0, tint);
                        }
                    }
                } else if (bp.block_data & 0xFF) == BT_CAMPFIRE {
                    // Campfire blueprint: draw only the 2x2 subtile area
                    let bp_flags = ((bp.block_data >> 16) & 0xFF) as u8;
                    let sub_x = (bp_flags >> 3) & 3;
                    let sub_y = (bp_flags >> 5) & 3;
                    let tw = sx1 - sx0;
                    let th = sy1 - sy0;
                    let sub_rect = egui::Rect::from_min_size(
                        egui::pos2(
                            sx0 + sub_x as f32 * tw * 0.25,
                            sy0 + sub_y as f32 * th * 0.25,
                        ),
                        egui::vec2(tw * 0.5, th * 0.5),
                    );
                    bp_painter.rect_filled(sub_rect, 0.0, tint);
                } else {
                    // Full-thickness wall or non-wall block
                    bp_painter.rect_filled(rect, 0.0, tint);
                }
                // Progress bar at bottom (construction progress)
                if bp.progress > 0.01 {
                    let bar_h = (tile_px * 0.08).max(2.0);
                    let bar_rect = egui::Rect::from_min_size(
                        egui::pos2(sx0, sy1 - bar_h),
                        egui::vec2((sx1 - sx0) * bp.progress, bar_h),
                    );
                    bp_painter.rect_filled(bar_rect, 0.0, egui::Color32::from_rgb(80, 200, 80));
                }
                // Resource + name label when zoomed in
                if tile_px > 6.0 {
                    let bt = bp.block_data & 0xFF;
                    let reg = block_defs::BlockRegistry::cached();
                    let name = reg.name(bt);
                    // Resource indicator
                    let res_text = if bp.wood_needed > 0
                        || bp.clay_needed > 0
                        || bp.plank_needed > 0
                        || bp.rock_needed > 0
                        || bp.rope_needed > 0
                    {
                        let color = if bp.resources_met() {
                            egui::Color32::from_rgb(80, 220, 80)
                        } else {
                            egui::Color32::from_rgb(255, 160, 60)
                        };
                        let mut parts = Vec::with_capacity(5);
                        if bp.wood_needed > 0 {
                            parts.push(format!("{}/{} wood", bp.wood_delivered, bp.wood_needed));
                        }
                        if bp.plank_needed > 0 {
                            parts.push(format!("{}/{} plank", bp.plank_delivered, bp.plank_needed));
                        }
                        if bp.rock_needed > 0 {
                            parts.push(format!("{}/{} rock", bp.rock_delivered, bp.rock_needed));
                        }
                        if bp.clay_needed > 0 {
                            parts.push(format!("{}/{} clay", bp.clay_delivered, bp.clay_needed));
                        }
                        if bp.rope_needed > 0 {
                            parts.push(format!("{}/{} rope", bp.rope_delivered, bp.rope_needed));
                        }
                        Some((parts.join(" "), color))
                    } else if bp.is_roof() {
                        Some(("1 fiber".to_string(), egui::Color32::from_rgb(255, 160, 60)))
                    } else if bp.is_campfire() {
                        Some((
                            "3 sticks".to_string(),
                            egui::Color32::from_rgb(255, 160, 60),
                        ))
                    } else {
                        None
                    };
                    Self::world_label(
                        &bp_painter,
                        egui::pos2(rect.center().x, rect.center().y - 3.0),
                        egui::Align2::CENTER_CENTER,
                        name,
                        9.0,
                        egui::Color32::from_rgba_unmultiplied(180, 200, 255, 220),
                    );
                    if let Some((res_label, res_color)) = res_text {
                        Self::world_label(
                            &bp_painter,
                            egui::pos2(rect.center().x, rect.center().y + 6.0),
                            egui::Align2::CENTER_CENTER,
                            &res_label,
                            8.0,
                            res_color,
                        );
                    }
                }
            }
        }

        // Blueprint preview — draw ghost overlay for placement
        if !blueprint_tiles.is_empty() {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;

            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("blueprint"),
            ));

            for &((tx, ty), state) in blueprint_tiles {
                // 0=invalid (red), 1=valid/new (blue), 2=upgrade (orange), 3=entryway (green)
                let color = match state {
                    1 => egui::Color32::from_rgba_unmultiplied(80, 180, 255, 80),
                    2 => egui::Color32::from_rgba_unmultiplied(255, 180, 40, 90),
                    3 => egui::Color32::from_rgba_unmultiplied(80, 220, 80, 90), // entryway
                    _ => egui::Color32::from_rgba_unmultiplied(255, 60, 60, 80),
                };

                let wx0 = tx as f32;
                let wy0 = ty as f32;
                // World → physical pixels → logical points (egui coords)
                let sx0 = ((wx0 - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy0 = ((wy0 - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                let sx1 =
                    ((wx0 + 1.0 - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy1 =
                    ((wy0 + 1.0 - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;

                // Diagonal wall: draw triangle showing which half is solid
                if self.build_tool == BuildTool::Place(44) {
                    // During drag, use per-tile variant from diag_preview
                    let variant = self
                        .diag_preview
                        .iter()
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
                } else if let Some((sub_x, sub_y)) = self.campfire_subtile {
                    // Campfire: only highlight the 2x2 subtile area
                    let tile_w = sx1 - sx0;
                    let tile_h = sy1 - sy0;
                    let sub_sx = sx0 + (sub_x as f32) * tile_w * 0.25;
                    let sub_sy = sy0 + (sub_y as f32) * tile_h * 0.25;
                    let sub_ex = sub_sx + tile_w * 0.5;
                    let sub_ey = sub_sy + tile_h * 0.5;
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(sub_sx, sub_sy),
                            egui::pos2(sub_ex, sub_ey),
                        ),
                        0.0,
                        color,
                    );
                } else {
                    painter.rect_filled(
                        egui::Rect::from_min_max(egui::pos2(sx0, sy0), egui::pos2(sx1, sy1)),
                        0.0,
                        color,
                    );
                }

                // Wind turbine: show wind direction arrows across the 2x2 area (first tile only)
                if self.build_tool == BuildTool::Place(41)
                    && tx == blueprint_tiles[0].0.0
                    && ty == blueprint_tiles[0].0.1
                {
                    // Draw arrows showing wind direction through the turbine
                    let center = egui::pos2(
                        (sx0 + sx1) / 2.0 + (sx1 - sx0) * 0.5,
                        (sy0 + sy1) / 2.0 + (sy1 - sy0) * 0.5,
                    );
                    let tile_size = (sx1 - sx0).max(1.0);
                    let (adx, ady) = if self.build_rotation.is_multiple_of(2) {
                        (0.0f32, -1.0f32) // N-S wind (blades face E-W)
                    } else {
                        (1.0f32, 0.0f32) // E-W wind (blades face N-S)
                    };
                    // Two arrows flanking the turbine
                    for &offset in &[-0.3f32, 0.3] {
                        let perp_off = egui::Vec2::new(
                            -ady * offset * tile_size * 2.0,
                            adx * offset * tile_size * 2.0,
                        );
                        let arrow_center = center + perp_off;
                        let arrow_len = tile_size * 1.5;
                        let tip = arrow_center
                            + egui::Vec2::new(adx * arrow_len * 0.5, ady * arrow_len * 0.5);
                        let tail = arrow_center
                            - egui::Vec2::new(adx * arrow_len * 0.5, ady * arrow_len * 0.5);
                        painter.line_segment(
                            [tail, tip],
                            egui::Stroke::new(
                                2.0,
                                egui::Color32::from_rgba_unmultiplied(100, 200, 255, 180),
                            ),
                        );
                        let perp = egui::Vec2::new(-ady, adx) * arrow_len * 0.15;
                        let head_base = arrow_center
                            + egui::Vec2::new(adx * arrow_len * 0.25, ady * arrow_len * 0.25);
                        painter.add(egui::Shape::convex_polygon(
                            vec![tip, head_base + perp, head_base - perp],
                            egui::Color32::from_rgba_unmultiplied(100, 200, 255, 180),
                            egui::Stroke::NONE,
                        ));
                    }
                }

                // Direction arrow for fan, pump, inlet, outlet
                if matches!(
                    self.build_tool,
                    BuildTool::Place(12)
                        | BuildTool::Place(16)
                        | BuildTool::Place(20)
                        | BuildTool::Place(19)
                ) {
                    let center = egui::pos2((sx0 + sx1) / 2.0, (sy0 + sy1) / 2.0);
                    let tile_size = (sx1 - sx0).max(1.0);
                    let (adx, ady) = match self.build_rotation {
                        0 => (0.0f32, -1.0f32),
                        1 => (1.0, 0.0),
                        2 => (0.0, 1.0),
                        _ => (-1.0, 0.0),
                    };
                    let arrow_len = tile_size * 0.8;
                    let tip =
                        center + egui::Vec2::new(adx * arrow_len * 0.5, ady * arrow_len * 0.5);
                    let tail =
                        center - egui::Vec2::new(adx * arrow_len * 0.5, ady * arrow_len * 0.5);
                    painter.line_segment([tail, tip], egui::Stroke::new(2.0, egui::Color32::WHITE));
                    let perp = egui::Vec2::new(-ady, adx) * arrow_len * 0.2;
                    let head_base =
                        center + egui::Vec2::new(adx * arrow_len * 0.2, ady * arrow_len * 0.2);
                    painter.add(egui::Shape::convex_polygon(
                        vec![tip, head_base + perp, head_base - perp],
                        egui::Color32::WHITE,
                        egui::Stroke::NONE,
                    ));
                }
            }
        }

        // Pleb placement ghost
        if self.placing_pleb {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
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
        if let Some(pleb) = sel_pleb_ref
            && pleb.path_idx < pleb.path.len()
        {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("pleb_path"),
            ));
            let to_screen = |wx: f32, wy: f32| -> egui::Pos2 {
                let sx = ((wx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy = ((wy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                egui::pos2(sx, sy)
            };
            // Draw from pleb's current position through remaining path
            // Color by elevation cost: green=flat, yellow=slight slope, red=steep uphill, blue=downhill
            let mut prev = to_screen(pleb.x, pleb.y);
            let mut prev_pos = (pleb.x.floor() as i32, pleb.y.floor() as i32);
            for i in pleb.path_idx..pleb.path.len() {
                let (px, py) = pleb.path[i];
                let next = to_screen(px as f32 + 0.5, py as f32 + 0.5);
                // Compute elevation difference for color
                let seg_color = if !self.elevation_data.is_empty() {
                    let prev_idx = (prev_pos.1 as u32 * GRID_W + prev_pos.0 as u32) as usize;
                    let next_idx = (py as u32 * GRID_W + px as u32) as usize;
                    let elev_diff = if prev_idx < self.elevation_data.len()
                        && next_idx < self.elevation_data.len()
                    {
                        self.elevation_data[next_idx] - self.elevation_data[prev_idx]
                    } else {
                        0.0
                    };
                    if elev_diff > 0.3 {
                        // Uphill: yellow → red
                        let t = ((elev_diff - 0.3) / 1.5).min(1.0);
                        egui::Color32::from_rgba_unmultiplied(
                            (255.0) as u8,
                            (255.0 - t * 200.0) as u8,
                            (100.0 - t * 100.0) as u8,
                            180,
                        )
                    } else if elev_diff < -0.3 {
                        // Downhill: cyan
                        let t = ((-elev_diff - 0.3) / 1.5).min(1.0);
                        egui::Color32::from_rgba_unmultiplied(
                            (100.0 - t * 50.0) as u8,
                            (200.0 + t * 55.0) as u8,
                            255,
                            180,
                        )
                    } else {
                        // Flat: green
                        egui::Color32::from_rgba_unmultiplied(100, 255, 100, 150)
                    }
                } else {
                    egui::Color32::from_rgba_unmultiplied(100, 255, 100, 150)
                };
                painter.line_segment([prev, next], egui::Stroke::new(2.0, seg_color));
                prev = next;
                prev_pos = (px, py);
            }
            // Draw target marker at end
            let Some(last) = pleb.path.last() else {
                return;
            };
            let end = to_screen(last.0 as f32 + 0.5, last.1 as f32 + 0.5);
            painter.circle_stroke(
                end,
                4.0,
                egui::Stroke::new(
                    2.0,
                    egui::Color32::from_rgba_unmultiplied(100, 255, 100, 200),
                ),
            );
            // Draw queued command waypoints as dashed line + numbered markers
            if !pleb.command_queue.is_empty() {
                let queue_color = egui::Color32::from_rgba_unmultiplied(255, 200, 100, 160); // orange
                let mut prev_q = end;
                for (qi, cmd) in pleb.command_queue.iter().enumerate() {
                    let (tx, ty) = match cmd {
                        PlebCommand::MoveTo(wx, wy) => (wx.floor() + 0.5, wy.floor() + 0.5),
                        PlebCommand::Harvest(x, y)
                        | PlebCommand::Haul(x, y)
                        | PlebCommand::Eat(x, y)
                        | PlebCommand::DigClay(x, y) => (*x as f32 + 0.5, *y as f32 + 0.5),
                        PlebCommand::HandCraft(_) => (pleb.x, pleb.y),
                        PlebCommand::GatherBranches(x, y) => (*x as f32 + 0.5, *y as f32 + 0.5),
                    };
                    let qp = to_screen(tx, ty);
                    // Dashed line to next waypoint
                    painter.line_segment([prev_q, qp], egui::Stroke::new(1.5, queue_color));
                    // Numbered circle
                    painter.circle_stroke(qp, 5.0, egui::Stroke::new(2.0, queue_color));
                    painter.text(
                        qp,
                        egui::Align2::CENTER_CENTER,
                        format!("{}", qi + 1),
                        egui::FontId::proportional(8.0),
                        queue_color,
                    );
                    prev_q = qp;
                }
            }
        }

        // Pipe overlay: draw pipe gas contents as colored blocks
        if self.show_pipe_overlay && !self.pipe_network.cells.is_empty() {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("pipe_overlay"),
            ));
            let to_screen = |wx: f32, wy: f32| -> egui::Pos2 {
                let sx = ((wx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy = ((wy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                egui::pos2(sx, sy)
            };
            let screen_rect = ctx.content_rect();
            for (&idx, cell) in &self.pipe_network.cells {
                let x = (idx % GRID_W) as f32;
                let y = (idx / GRID_W) as f32;
                let p0 = to_screen(x + 0.15, y + 0.15);
                let p1 = to_screen(x + 0.85, y + 0.85);
                // Cull off-screen cells
                if p1.x < screen_rect.min.x
                    || p0.x > screen_rect.max.x
                    || p1.y < screen_rect.min.y
                    || p0.y > screen_rect.max.y
                {
                    continue;
                }
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
                    (r * 255.0) as u8,
                    (g * 255.0) as u8,
                    (b * 255.0) as u8,
                    (alpha * 255.0) as u8,
                );
                painter.rect_filled(egui::Rect::from_min_max(p0, p1), 2.0, color);
                // Pressure indicator: small text
                if cam_zoom > 10.0 {
                    // only show text when zoomed in enough
                    let center = egui::pos2((p0.x + p1.x) / 2.0, (p0.y + p1.y) / 2.0);
                    Self::world_label(
                        &painter,
                        center,
                        egui::Align2::CENTER_CENTER,
                        &format!("{:.1}", cell.pressure),
                        10.0,
                        egui::Color32::WHITE,
                    );
                }
            }
        }

        // Liquid pipe overlay: draw liquid contents as blue-tinted blocks
        if self.show_liquid_overlay && !self.liquid_network.cells.is_empty() {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("liquid_overlay"),
            ));
            let to_screen = |wx: f32, wy: f32| -> egui::Pos2 {
                let sx = ((wx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy = ((wy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                egui::pos2(sx, sy)
            };
            let screen_rect = ctx.content_rect();
            for (&idx, cell) in &self.liquid_network.cells {
                if idx >= GRID_W * GRID_H {
                    continue;
                }
                let x = (idx % GRID_W) as f32;
                let y = (idx / GRID_W) as f32;
                let p0 = to_screen(x + 0.15, y + 0.15);
                let p1 = to_screen(x + 0.85, y + 0.85);
                if p1.x < screen_rect.min.x
                    || p0.x > screen_rect.max.x
                    || p1.y < screen_rect.min.y
                    || p0.y > screen_rect.max.y
                {
                    continue;
                }
                let pres = (cell.pressure / 2.0).clamp(0.0, 1.0);
                let r = (50.0 + pres * 50.0) as u8;
                let g = (100.0 + pres * 80.0) as u8;
                let b = (180.0 + pres * 75.0) as u8;
                let alpha = (80.0 + pres * 120.0) as u8;
                let color = egui::Color32::from_rgba_unmultiplied(r, g, b, alpha);
                painter.rect_filled(egui::Rect::from_min_max(p0, p1), 2.0, color);
                if cam_zoom > 10.0 {
                    let center = egui::pos2((p0.x + p1.x) / 2.0, (p0.y + p1.y) / 2.0);
                    Self::world_label(
                        &painter,
                        center,
                        egui::Align2::CENTER_CENTER,
                        &format!("{:.1}", cell.pressure),
                        10.0,
                        egui::Color32::WHITE,
                    );
                }
            }
        }

        // Flow overlay: arrows on pipes (pressure flow) and wires (current flow)
        if self.show_flow_overlay {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let tile_px = cam_zoom / self.render_scale / bp_ppp;
            let flow_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("flow_overlay"),
            ));
            let to_screen = |wx: f32, wy: f32| -> egui::Pos2 {
                let sx = ((wx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy = ((wy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                egui::pos2(sx, sy)
            };

            // Helper: draw a flow arrow at screen center with given direction, normalized magnitude, and tile_px
            let draw_arrow = |painter: &egui::Painter,
                              center: egui::Pos2,
                              dir_x: f32,
                              dir_y: f32,
                              norm_mag: f32,
                              tile_px: f32,
                              label: Option<String>,
                              is_wire: bool| {
                let arrow_len = norm_mag * 0.4 * tile_px;
                if arrow_len < 1.0 {
                    return;
                }
                // Color ramp: pipe=blue→cyan→yellow→red, wire=purple→cyan→yellow→white
                let color = if is_wire {
                    if norm_mag < 0.33 {
                        let t = norm_mag / 0.33;
                        egui::Color32::from_rgba_unmultiplied(
                            (120.0 + t * 40.0) as u8,
                            (60.0 + t * 140.0) as u8,
                            (200.0 + t * 55.0) as u8,
                            210,
                        )
                    } else if norm_mag < 0.66 {
                        let t = (norm_mag - 0.33) / 0.33;
                        egui::Color32::from_rgba_unmultiplied(
                            (160.0 + t * 95.0) as u8,
                            255,
                            (255.0 - t * 100.0) as u8,
                            210,
                        )
                    } else {
                        let t = (norm_mag - 0.66) / 0.34;
                        egui::Color32::from_rgba_unmultiplied(
                            255,
                            (255.0 - t * 55.0) as u8,
                            (155.0 + t * 100.0) as u8,
                            210,
                        )
                    }
                } else if norm_mag < 0.33 {
                    let t = norm_mag / 0.33;
                    egui::Color32::from_rgba_unmultiplied(
                        (30.0 + t * 50.0) as u8,
                        (100.0 + t * 155.0) as u8,
                        255,
                        200,
                    )
                } else if norm_mag < 0.66 {
                    let t = (norm_mag - 0.33) / 0.33;
                    egui::Color32::from_rgba_unmultiplied(
                        (80.0 + t * 175.0) as u8,
                        255,
                        (255.0 - t * 155.0) as u8,
                        200,
                    )
                } else {
                    let t = (norm_mag - 0.66) / 0.34;
                    egui::Color32::from_rgba_unmultiplied(
                        255,
                        (255.0 - t * 155.0) as u8,
                        (100.0 - t * 100.0) as u8,
                        200,
                    )
                };
                let tip = egui::pos2(center.x + dir_x * arrow_len, center.y + dir_y * arrow_len);
                let tail = egui::pos2(
                    center.x - dir_x * arrow_len * 0.3,
                    center.y - dir_y * arrow_len * 0.3,
                );
                let stroke_w = (1.0 + norm_mag * 2.0).min(3.0);
                painter.line_segment([tail, tip], egui::Stroke::new(stroke_w, color));
                let head_len = arrow_len * 0.35;
                let px = -dir_y;
                let py = dir_x;
                let h1 = egui::pos2(
                    tip.x - dir_x * head_len + px * head_len * 0.5,
                    tip.y - dir_y * head_len + py * head_len * 0.5,
                );
                let h2 = egui::pos2(
                    tip.x - dir_x * head_len - px * head_len * 0.5,
                    tip.y - dir_y * head_len - py * head_len * 0.5,
                );
                painter.line_segment([tip, h1], egui::Stroke::new(stroke_w, color));
                painter.line_segment([tip, h2], egui::Stroke::new(stroke_w, color));
                if let Some(lbl) = label {
                    Self::world_label(
                        painter,
                        egui::pos2(center.x, center.y + tile_px * 0.3),
                        egui::Align2::CENTER_TOP,
                        &lbl,
                        9.0,
                        egui::Color32::from_rgba_unmultiplied(200, 200, 200, 180),
                    );
                }
            };

            // --- Pipe flow arrows ---
            if !self.pipe_network.cells.is_empty() {
                let max_flow = self
                    .pipe_network
                    .cells
                    .values()
                    .map(|c| (c.flow_x * c.flow_x + c.flow_y * c.flow_y).sqrt())
                    .fold(0.001f32, f32::max);
                let screen_rect = ctx.content_rect();
                for (&idx, cell) in &self.pipe_network.cells {
                    let mag = (cell.flow_x * cell.flow_x + cell.flow_y * cell.flow_y).sqrt();
                    if mag < 0.001 {
                        continue;
                    }
                    let x = (idx % GRID_W) as f32;
                    let y = (idx / GRID_W) as f32;
                    let center = to_screen(x + 0.5, y + 0.5);
                    if center.x < screen_rect.min.x - tile_px
                        || center.x > screen_rect.max.x + tile_px
                        || center.y < screen_rect.min.y - tile_px
                        || center.y > screen_rect.max.y + tile_px
                    {
                        continue;
                    }
                    let norm_mag = (mag / max_flow).clamp(0.0, 1.0);
                    let label = if tile_px > 8.0 {
                        Some(format!("{:.2}", mag))
                    } else {
                        None
                    };
                    draw_arrow(
                        &flow_painter,
                        center,
                        cell.flow_x / mag,
                        cell.flow_y / mag,
                        norm_mag,
                        tile_px,
                        label,
                        false,
                    );
                }
            }

            // --- Wire current arrows (from voltage gradient) ---
            if !self.voltage_data.is_empty() {
                let gw = GRID_W as i32;
                let gh = GRID_H as i32;
                // Visible tile range
                let min_x = ((cam_cx - cam_sw * 0.5 / cam_zoom).floor() as i32).max(0);
                let max_x = ((cam_cx + cam_sw * 0.5 / cam_zoom).ceil() as i32).min(gw);
                let min_y = ((cam_cy - cam_sh * 0.5 / cam_zoom).floor() as i32).max(0);
                let max_y = ((cam_cy + cam_sh * 0.5 / cam_zoom).ceil() as i32).min(gh);
                // Find max current in visible area for normalization
                let mut max_current = 0.001f32;
                for ty in min_y..max_y {
                    for tx in min_x..max_x {
                        let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                        let b = self.grid_data[idx];
                        let bt = b & 0xFF;
                        let fl = (b >> 16) & 0xFF;
                        let is_cond = is_conductor_rs(bt, fl as u8);
                        if !is_cond {
                            continue;
                        }
                        let v = self.voltage_data[idx];
                        if v < 0.01 {
                            continue;
                        }
                        // Compute current vector: flows from high to low voltage
                        let mut cx_f = 0.0f32;
                        let mut cy_f = 0.0f32;
                        for &(dx, dy) in &[(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                            let nx = tx + dx;
                            let ny = ty + dy;
                            if nx < 0 || ny < 0 || nx >= gw || ny >= gh {
                                continue;
                            }
                            let nidx = (ny as u32 * GRID_W + nx as u32) as usize;
                            let nv = self.voltage_data[nidx];
                            let dv = v - nv; // positive = current flows toward neighbor
                            cx_f += dx as f32 * dv;
                            cy_f += dy as f32 * dv;
                        }
                        let cmag = (cx_f * cx_f + cy_f * cy_f).sqrt();
                        if cmag > max_current {
                            max_current = cmag;
                        }
                    }
                }
                // Draw arrows
                for ty in min_y..max_y {
                    for tx in min_x..max_x {
                        let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                        let b = self.grid_data[idx];
                        let bt = b & 0xFF;
                        let fl = (b >> 16) & 0xFF;
                        // Skip pipe tiles (already drawn above)
                        if (15..=20).contains(&bt) {
                            continue;
                        }
                        let is_cond = is_conductor_rs(bt, fl as u8);
                        if !is_cond {
                            continue;
                        }
                        let v = self.voltage_data[idx];
                        if v < 0.01 {
                            continue;
                        }
                        let mut cx_f = 0.0f32;
                        let mut cy_f = 0.0f32;
                        for &(dx, dy) in &[(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                            let nx = tx + dx;
                            let ny = ty + dy;
                            if nx < 0 || ny < 0 || nx >= gw || ny >= gh {
                                continue;
                            }
                            let nidx = (ny as u32 * GRID_W + nx as u32) as usize;
                            let nv = self.voltage_data[nidx];
                            let dv = v - nv;
                            cx_f += dx as f32 * dv;
                            cy_f += dy as f32 * dv;
                        }
                        let cmag = (cx_f * cx_f + cy_f * cy_f).sqrt();
                        if cmag < 0.01 {
                            continue;
                        }
                        let center = to_screen(tx as f32 + 0.5, ty as f32 + 0.5);
                        let norm_mag = (cmag / max_current).clamp(0.0, 1.0);
                        let label = if tile_px > 8.0 {
                            Some(format!("{:.1}A", cmag))
                        } else {
                            None
                        };
                        draw_arrow(
                            &flow_painter,
                            center,
                            cx_f / cmag,
                            cy_f / cmag,
                            norm_mag,
                            tile_px,
                            label,
                            true,
                        );
                    }
                }
            }
        }

        // Render cannon barrel direction overlays
        {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let tile_px = cam_zoom / self.render_scale / bp_ppp;
            let cannon_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("cannons"),
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
                cannon_painter.circle_filled(
                    egui::pos2(end_x, end_y),
                    barrel_w * 0.5,
                    egui::Color32::from_rgb(40, 38, 35),
                );
                cannon_painter.circle_filled(
                    egui::pos2(sx, sy),
                    barrel_w * 0.7,
                    egui::Color32::from_rgb(50, 48, 42),
                );
                if is_selected {
                    cannon_painter.circle_stroke(
                        egui::pos2(sx, sy),
                        barrel_len * 1.1,
                        egui::Stroke::new(
                            1.0,
                            egui::Color32::from_rgba_unmultiplied(100, 255, 100, 150),
                        ),
                    );
                }
            }
        }

        // Lightning flash overlay + bolt rendering
        if self.lightning_flash > 0.01 {
            let flash_alpha = (self.lightning_flash * 180.0).min(255.0) as u8;
            let flash_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("lightning_flash"),
            ));
            let screen_rect = egui::Rect::from_min_max(
                egui::pos2(0.0, 0.0),
                egui::pos2(ctx.content_rect().width(), ctx.content_rect().height()),
            );
            flash_painter.rect_filled(
                screen_rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(220, 225, 255, flash_alpha),
            );

            // Draw lightning bolt at strike location
            if let Some((lx, ly)) = self.lightning_strike {
                let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
                let strike_sx =
                    ((lx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let strike_sy =
                    ((ly - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                let bolt_alpha = (self.lightning_flash * 255.0).min(255.0) as u8;
                let bolt_col = egui::Color32::from_rgba_unmultiplied(220, 230, 255, bolt_alpha);
                let glow_col = egui::Color32::from_rgba_unmultiplied(150, 170, 255, bolt_alpha / 3);
                // Main bolt: thick jagged line from top of screen to strike
                let segments = 12;
                let mut prev = egui::pos2(strike_sx + 20.0, 0.0);
                for i in 1..=segments {
                    let t = i as f32 / segments as f32;
                    let jitter = (t * 23.7 + lx * 3.1).sin() * 25.0 * (1.0 - t * 0.5);
                    let next = egui::pos2(strike_sx + jitter, strike_sy * t);
                    let width = 4.0 * (1.0 - t * 0.5) * self.lightning_flash;
                    // Glow (wider, dimmer)
                    flash_painter
                        .line_segment([prev, next], egui::Stroke::new(width * 3.0, glow_col));
                    // Core (bright)
                    flash_painter.line_segment([prev, next], egui::Stroke::new(width, bolt_col));
                    prev = next;
                }
                // Branch bolt (smaller, offset)
                let mut prev2 = egui::pos2(strike_sx - 15.0, strike_sy * 0.3);
                for i in 1..=5 {
                    let t = i as f32 / 5.0;
                    let jitter = (t * 31.3 + ly * 2.7).sin() * 12.0;
                    let next2 =
                        egui::pos2(strike_sx + jitter * (1.0 - t), strike_sy * (0.3 + t * 0.7));
                    flash_painter.line_segment(
                        [prev2, next2],
                        egui::Stroke::new(2.0 * self.lightning_flash, bolt_col),
                    );
                    prev2 = next2;
                }
                // Impact circle
                let impact_r = 15.0 * self.lightning_flash;
                flash_painter.circle_filled(
                    egui::pos2(strike_sx, strike_sy),
                    impact_r,
                    egui::Color32::from_rgba_unmultiplied(255, 255, 240, bolt_alpha),
                );
                flash_painter.circle_stroke(
                    egui::pos2(strike_sx, strike_sy),
                    impact_r * 2.0,
                    egui::Stroke::new(
                        2.0,
                        egui::Color32::from_rgba_unmultiplied(180, 190, 255, bolt_alpha / 2),
                    ),
                );
            }
        }

        // Per-tile voltage labels when power overlay is active
        if matches!(
            self.fluid_overlay,
            FluidOverlay::Power | FluidOverlay::PowerAmps | FluidOverlay::PowerWatts
        ) && !self.voltage_data.is_empty()
        {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let tile_px = cam_zoom / self.render_scale / bp_ppp;
            if tile_px > 6.0 {
                // only show labels when zoomed in enough
                let label_painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Background,
                    egui::Id::new("voltage_labels"),
                ));
                let to_screen = |wx: f32, wy: f32| -> egui::Pos2 {
                    let sx = ((wx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                    let sy = ((wy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                    egui::pos2(sx, sy)
                };
                // Visible tile range
                let min_x = ((cam_cx - cam_sw * 0.5 / cam_zoom).floor() as i32).max(0);
                let max_x = ((cam_cx + cam_sw * 0.5 / cam_zoom).ceil() as i32).min(GRID_W as i32);
                let min_y = ((cam_cy - cam_sh * 0.5 / cam_zoom).floor() as i32).max(0);
                let max_y = ((cam_cy + cam_sh * 0.5 / cam_zoom).ceil() as i32).min(GRID_H as i32);
                for ty in min_y..max_y {
                    for tx in min_x..max_x {
                        let idx = (ty as u32 * GRID_W + tx as u32) as usize;
                        if idx >= self.voltage_data.len() {
                            continue;
                        }
                        let v = self.voltage_data[idx];
                        if v < 0.05 {
                            continue;
                        }
                        let b = self.grid_data[idx];
                        let bt = b & 0xFF;
                        let flags = (b >> 16) & 0xFF;
                        let is_cond = is_conductor_rs(bt, flags as u8);
                        if !is_cond {
                            continue;
                        }
                        let center = to_screen(tx as f32 + 0.5, ty as f32 + 0.5);
                        let text = if v >= 10.0 {
                            format!("{:.0}V", v)
                        } else {
                            format!("{:.1}V", v)
                        };
                        let color = if v > 15.0 {
                            egui::Color32::from_rgb(255, 120, 120) // red for overvoltage
                        } else if v > 1.0 {
                            egui::Color32::from_rgb(220, 255, 220) // green for normal
                        } else {
                            egui::Color32::from_rgb(150, 150, 150) // dim for trace
                        };
                        Self::world_label(
                            &label_painter,
                            center,
                            egui::Align2::CENTER_CENTER,
                            &text,
                            10.0,
                            color,
                        );
                    }
                }
            }
        }

        // Render power cables: squiggly lines from lights/fans to nearest wire
        {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let tile_px = cam_zoom / self.render_scale / bp_ppp;
            if tile_px > 3.0 {
                // only draw when zoomed in enough
                let cable_painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Background,
                    egui::Id::new("power_cables"),
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
                        if bt != 7 && bt != 10 && bt != 12 {
                            continue;
                        }
                        // Search for nearest wire within 3 tiles
                        let mut best_wire: Option<(i32, i32, f32)> = None;
                        for dy in -3i32..=3 {
                            for dx in -3i32..=3 {
                                let wx = x + dx;
                                let wy = y + dy;
                                if wx < 0 || wy < 0 || wx >= GRID_W as i32 || wy >= GRID_H as i32 {
                                    continue;
                                }
                                let widx = (wy as u32 * GRID_W + wx as u32) as usize;
                                if (self.grid_data[widx] & 0xFF) == 36 {
                                    let dist = ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt();
                                    if best_wire.is_none_or(|(_, _, d)| dist < d) {
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
                            let sag_mid = egui::pos2(
                                mid.x + perp_x / perp_len * sag,
                                mid.y + perp_y / perp_len * sag + sag * 0.5,
                            );
                            // Draw as segmented line through sag point
                            let cable_color = egui::Color32::from_rgb(70, 60, 45);
                            cable_painter
                                .line_segment([from, sag_mid], egui::Stroke::new(1.5, cable_color));
                            cable_painter
                                .line_segment([sag_mid, to], egui::Stroke::new(1.5, cable_color));
                            // Small connector dots at endpoints
                            cable_painter.circle_filled(from, 2.0, cable_color);
                            cable_painter.circle_filled(to, 2.0, cable_color);
                        }
                    }
                }
            }
        }

        // Render dig holes on terrain
        {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let tile_px = cam_zoom / self.render_scale / bp_ppp;
            // Roof completion flash: briefly show a tint over newly-built roof tiles
            {
                let flash_painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Background,
                    egui::Id::new("roof_flash"),
                ));
                let screen_rect = ctx.content_rect();
                for y in 0..GRID_H {
                    for x in 0..GRID_W {
                        let idx = (y * GRID_W + x) as usize;
                        if idx >= self.roof_flash.len() || self.roof_flash[idx] <= 0.0 {
                            continue;
                        }
                        let alpha = (self.roof_flash[idx] / 3.0 * 120.0).min(120.0) as u8;
                        let sx0 = ((x as f32 - cam_cx) * cam_zoom + cam_sw * 0.5)
                            / self.render_scale
                            / bp_ppp;
                        let sy0 = ((y as f32 - cam_cy) * cam_zoom + cam_sh * 0.5)
                            / self.render_scale
                            / bp_ppp;
                        let sx1 = ((x as f32 + 1.0 - cam_cx) * cam_zoom + cam_sw * 0.5)
                            / self.render_scale
                            / bp_ppp;
                        let sy1 = ((y as f32 + 1.0 - cam_cy) * cam_zoom + cam_sh * 0.5)
                            / self.render_scale
                            / bp_ppp;
                        if sx1 < screen_rect.min.x
                            || sx0 > screen_rect.max.x
                            || sy1 < screen_rect.min.y
                            || sy0 > screen_rect.max.y
                        {
                            continue;
                        }
                        let rect =
                            egui::Rect::from_min_max(egui::pos2(sx0, sy0), egui::pos2(sx1, sy1));
                        flash_painter.rect_filled(
                            rect,
                            0.0,
                            egui::Color32::from_rgba_unmultiplied(180, 160, 100, alpha),
                        );
                    }
                }
            }

            if tile_px > 3.0 {
                let hole_painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Background,
                    egui::Id::new("dig_holes"),
                ));
                let screen_rect = ctx.content_rect();
                let to_scr = |wx: f32, wy: f32| -> egui::Pos2 {
                    let sx = ((wx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                    let sy = ((wy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                    egui::pos2(sx, sy)
                };
                // Scan visible tiles
                let min_x = ((cam_cx - cam_sw * 0.5 / cam_zoom) as i32 - 1).max(0) as u32;
                let max_x =
                    ((cam_cx + cam_sw * 0.5 / cam_zoom) as i32 + 2).min(GRID_W as i32) as u32;
                let min_y = ((cam_cy - cam_sh * 0.5 / cam_zoom) as i32 - 1).max(0) as u32;
                let max_y =
                    ((cam_cy + cam_sh * 0.5 / cam_zoom) as i32 + 2).min(GRID_H as i32) as u32;
                let hole_dark = egui::Color32::from_rgba_unmultiplied(40, 30, 20, 140);
                let hole_edge = egui::Color32::from_rgba_unmultiplied(60, 45, 30, 100);
                for y in min_y..max_y {
                    for x in min_x..max_x {
                        let idx = (y * GRID_W + x) as usize;
                        if idx >= self.terrain_data.len() {
                            continue;
                        }
                        let holes = terrain_dig_holes(self.terrain_data[idx]);
                        if holes == 0 {
                            continue;
                        }
                        // Skip tiles with walls or blueprints (walls render on top)
                        if idx < self.wall_data.len() && wd_edges(self.wall_data[idx]) != 0 {
                            continue;
                        }
                        if self.blueprints.contains_key(&(x as i32, y as i32)) {
                            continue;
                        }
                        // Deterministic random hole positions within the tile
                        let hole_r = (tile_px * 0.08).max(1.5);
                        for h in 0..holes {
                            // Hash-based position within tile (0.15..0.85 range)
                            let seed = x.wrapping_mul(73856093)
                                ^ y.wrapping_mul(19349663)
                                ^ h.wrapping_mul(83492791);
                            let hx = 0.15 + ((seed & 0xFF) as f32 / 255.0) * 0.7;
                            let hy = 0.15 + (((seed >> 8) & 0xFF) as f32 / 255.0) * 0.7;
                            let hr = hole_r * (0.7 + ((seed >> 16) & 0xFF) as f32 / 255.0 * 0.6);
                            let center = to_scr(x as f32 + hx, y as f32 + hy);
                            if center.x < screen_rect.min.x - 5.0
                                || center.x > screen_rect.max.x + 5.0
                                || center.y < screen_rect.min.y - 5.0
                                || center.y > screen_rect.max.y + 5.0
                            {
                                continue;
                            }
                            // Dark hole with slightly lighter edge
                            hole_painter.circle_filled(center, hr * 1.3, hole_edge);
                            hole_painter.circle_filled(center, hr, hole_dark);
                        }
                    }
                }
            }
        }

        // Render blood stains
        if !self.blood_stains.is_empty() {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let blood_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("blood_stains"),
            ));
            for &(bx, by, timer) in &self.blood_stains {
                let sx = ((bx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy = ((by - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                let alpha = (timer / 3.0).clamp(0.0, 1.0); // fade over last 3 seconds
                let r = cam_zoom / self.render_scale / bp_ppp * 0.08; // small drops
                blood_painter.circle_filled(
                    egui::pos2(sx, sy),
                    r.max(1.5),
                    egui::Color32::from_rgba_unmultiplied(120, 10, 10, (alpha * 180.0) as u8),
                );
            }
        }

        // Render ground items (harvest drops)
        if !self.ground_items.is_empty() {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let tile_px = cam_zoom / self.render_scale / bp_ppp;
            let item_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("ground_items"),
            ));
            let to_screen = |wx: f32, wy: f32| -> egui::Pos2 {
                let sx = ((wx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy = ((wy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                egui::pos2(sx, sy)
            };
            let screen_rect = ctx.content_rect();
            for item in &self.ground_items {
                let center = to_screen(item.x, item.y);
                // Cull off-screen items
                if center.x < screen_rect.min.x - 20.0
                    || center.x > screen_rect.max.x + 20.0
                    || center.y < screen_rect.min.y - 20.0
                    || center.y > screen_rect.max.y + 20.0
                {
                    continue;
                }
                // Hide in fog of war
                if self.fog_enabled {
                    let fx = item.x.floor() as i32;
                    let fy = item.y.floor() as i32;
                    if fx >= 0 && fy >= 0 && fx < GRID_W as i32 && fy < GRID_H as i32 {
                        let fidx = (fy as u32 * GRID_W + fx as u32) as usize;
                        if fidx < self.fog_visibility.len() && self.fog_visibility[fidx] < 128 {
                            continue;
                        }
                    }
                }
                let r = (tile_px * 0.18).max(3.0);
                let n = item.stack.count;
                let iid = item.stack.item_id;
                if iid == item_defs::ITEM_BERRIES {
                    // Woven basket with clustered berries
                    let basket = egui::Color32::from_rgb(155, 110, 55);
                    let basket_dark = egui::Color32::from_rgb(120, 80, 35);
                    let berry = egui::Color32::from_rgb(90, 20, 130);
                    let berry_hi = egui::Color32::from_rgb(160, 60, 200);
                    // Basket body
                    item_painter.circle_filled(center, r * 0.85, basket);
                    item_painter.circle_filled(
                        egui::pos2(center.x, center.y + r * 0.15),
                        r * 0.65,
                        basket_dark,
                    );
                    // Weave lines
                    for i in 0..3u32 {
                        let y = center.y - r * 0.3 + i as f32 * r * 0.3;
                        item_painter.line_segment(
                            [
                                egui::pos2(center.x - r * 0.7, y),
                                egui::pos2(center.x + r * 0.7, y),
                            ],
                            egui::Stroke::new(r * 0.06, basket),
                        );
                    }
                    // Berry cluster on top
                    let count = (n as u32).min(5);
                    for i in 0..count {
                        let a = i as f32 * 1.25 + 0.5;
                        let dist = if i == 0 { 0.0 } else { r * 0.3 };
                        let bx = center.x + a.cos() * dist;
                        let by = center.y - r * 0.15 + a.sin() * dist * 0.6;
                        let br = r * 0.28;
                        item_painter.circle_filled(egui::pos2(bx, by), br, berry);
                        item_painter.circle_filled(
                            egui::pos2(bx - br * 0.25, by - br * 0.25),
                            br * 0.35,
                            berry_hi,
                        );
                    }
                } else if iid == item_defs::ITEM_ROCK {
                    // Irregular rock pile — overlapping angular shapes
                    let base = egui::Color32::from_rgb(105, 105, 100);
                    let light = egui::Color32::from_rgb(140, 138, 130);
                    let dark = egui::Color32::from_rgb(75, 75, 72);
                    let count = (n as u32).min(3);
                    for i in 0..count {
                        let ox = (i as f32 - 0.5) * r * 0.4;
                        let oy = (i as f32 - 0.5) * r * 0.25;
                        let sz = r * (0.7 - i as f32 * 0.08);
                        let rc = egui::pos2(center.x + ox, center.y + oy);
                        // Main rock body
                        item_painter.rect_filled(
                            egui::Rect::from_center_size(rc, egui::vec2(sz * 1.3, sz)),
                            sz * 0.35,
                            base,
                        );
                        // Highlight (top-left)
                        item_painter.circle_filled(
                            egui::pos2(rc.x - sz * 0.2, rc.y - sz * 0.15),
                            sz * 0.25,
                            light,
                        );
                        // Shadow (bottom-right)
                        item_painter.circle_filled(
                            egui::pos2(rc.x + sz * 0.25, rc.y + sz * 0.2),
                            sz * 0.2,
                            dark,
                        );
                    }
                } else if iid == item_defs::ITEM_WOOD {
                    // Log pile with end-grain circles and bark texture
                    let bark = egui::Color32::from_rgb(95, 60, 30);
                    let inner = egui::Color32::from_rgb(170, 130, 75);
                    let ring = egui::Color32::from_rgb(140, 105, 55);
                    let count = (n as u32).min(3);
                    for i in 0..count {
                        let ly = center.y - r * 0.55 + i as f32 * r * 0.55;
                        let lx = center.x + (i as f32 - 1.0) * r * 0.15;
                        let log_w = r * 1.3;
                        let log_h = r * 0.45;
                        // Bark body
                        item_painter.rect_filled(
                            egui::Rect::from_center_size(
                                egui::pos2(lx, ly),
                                egui::vec2(log_w, log_h),
                            ),
                            log_h * 0.4,
                            bark,
                        );
                        // End-grain circle (right side)
                        let end_x = lx + log_w * 0.42;
                        item_painter.circle_filled(egui::pos2(end_x, ly), log_h * 0.45, inner);
                        item_painter.circle_stroke(
                            egui::pos2(end_x, ly),
                            log_h * 0.25,
                            egui::Stroke::new(r * 0.05, ring),
                        );
                    }
                } else if iid == item_defs::ITEM_LOG {
                    // Big heavy log — larger than normal items
                    let bark = egui::Color32::from_rgb(85, 55, 28);
                    let bark_hi = egui::Color32::from_rgb(110, 72, 35);
                    let inner = egui::Color32::from_rgb(180, 140, 80);
                    let ring = egui::Color32::from_rgb(150, 115, 60);
                    let core = egui::Color32::from_rgb(130, 95, 50);
                    let lr = r * 1.6; // bigger than other items
                    let log_w = lr * 1.8;
                    let log_h = lr * 0.7;
                    // Bark body
                    item_painter.rect_filled(
                        egui::Rect::from_center_size(center, egui::vec2(log_w, log_h)),
                        log_h * 0.45,
                        bark,
                    );
                    // Bark highlight stripe
                    item_painter.line_segment(
                        [
                            egui::pos2(center.x - log_w * 0.35, center.y - log_h * 0.2),
                            egui::pos2(center.x + log_w * 0.35, center.y - log_h * 0.2),
                        ],
                        egui::Stroke::new(lr * 0.08, bark_hi),
                    );
                    // End-grain circle (right)
                    let end_r = log_h * 0.45;
                    let end_x = center.x + log_w * 0.4;
                    item_painter.circle_filled(egui::pos2(end_x, center.y), end_r, inner);
                    item_painter.circle_stroke(
                        egui::pos2(end_x, center.y),
                        end_r * 0.6,
                        egui::Stroke::new(lr * 0.06, ring),
                    );
                    item_painter.circle_filled(egui::pos2(end_x, center.y), end_r * 0.2, core);
                    // End-grain circle (left — cut face)
                    let end_x2 = center.x - log_w * 0.4;
                    item_painter.circle_filled(egui::pos2(end_x2, center.y), end_r, inner);
                    item_painter.circle_stroke(
                        egui::pos2(end_x2, center.y),
                        end_r * 0.6,
                        egui::Stroke::new(lr * 0.06, ring),
                    );
                    item_painter.circle_filled(egui::pos2(end_x2, center.y), end_r * 0.2, core);
                } else if iid == item_defs::ITEM_STONE_AXE {
                    // Stone axe: tapered handle + chipped stone head with binding
                    let handle = egui::Color32::from_rgb(120, 82, 40);
                    let head = egui::Color32::from_rgb(130, 128, 118);
                    let head_hi = egui::Color32::from_rgb(160, 158, 148);
                    let binding = egui::Color32::from_rgb(140, 120, 70);
                    // Handle (diagonal)
                    let h1 = egui::pos2(center.x - r * 0.5, center.y + r * 0.6);
                    let h2 = egui::pos2(center.x + r * 0.3, center.y - r * 0.4);
                    item_painter.line_segment([h1, h2], egui::Stroke::new(r * 0.22, handle));
                    // Stone head (wedge shape)
                    let hc = egui::pos2(center.x + r * 0.25, center.y - r * 0.35);
                    item_painter.rect_filled(
                        egui::Rect::from_center_size(hc, egui::vec2(r * 0.55, r * 0.75)),
                        r * 0.08,
                        head,
                    );
                    // Highlight chip
                    item_painter.circle_filled(
                        egui::pos2(hc.x - r * 0.08, hc.y - r * 0.12),
                        r * 0.12,
                        head_hi,
                    );
                    // Binding wrap
                    let bc = egui::pos2(center.x + r * 0.05, center.y - r * 0.05);
                    for i in 0..3u32 {
                        let by = bc.y - r * 0.08 + i as f32 * r * 0.08;
                        item_painter.line_segment(
                            [
                                egui::pos2(bc.x - r * 0.12, by),
                                egui::pos2(bc.x + r * 0.12, by),
                            ],
                            egui::Stroke::new(r * 0.06, binding),
                        );
                    }
                } else if iid == item_defs::ITEM_STONE_PICK {
                    // Stone pick: handle + narrow pointed head
                    let handle = egui::Color32::from_rgb(120, 82, 40);
                    let head = egui::Color32::from_rgb(125, 123, 115);
                    let head_hi = egui::Color32::from_rgb(155, 152, 142);
                    let binding = egui::Color32::from_rgb(140, 120, 70);
                    // Handle
                    let h1 = egui::pos2(center.x, center.y + r * 0.65);
                    let h2 = egui::pos2(center.x, center.y - r * 0.2);
                    item_painter.line_segment([h1, h2], egui::Stroke::new(r * 0.2, handle));
                    // Horizontal pick head
                    item_painter.rect_filled(
                        egui::Rect::from_center_size(
                            egui::pos2(center.x, center.y - r * 0.35),
                            egui::vec2(r * 1.2, r * 0.35),
                        ),
                        r * 0.06,
                        head,
                    );
                    // Pointed tips
                    item_painter.circle_filled(
                        egui::pos2(center.x - r * 0.55, center.y - r * 0.35),
                        r * 0.1,
                        head_hi,
                    );
                    item_painter.circle_filled(
                        egui::pos2(center.x + r * 0.55, center.y - r * 0.35),
                        r * 0.1,
                        head_hi,
                    );
                    // Binding
                    let bc = egui::pos2(center.x, center.y - r * 0.15);
                    for i in 0..2u32 {
                        let by = bc.y - r * 0.05 + i as f32 * r * 0.1;
                        item_painter.line_segment(
                            [
                                egui::pos2(bc.x - r * 0.1, by),
                                egui::pos2(bc.x + r * 0.1, by),
                            ],
                            egui::Stroke::new(r * 0.06, binding),
                        );
                    }
                } else if iid == item_defs::ITEM_WOODEN_SHOVEL {
                    // Shovel: tapered handle + rounded blade
                    let handle = egui::Color32::from_rgb(120, 82, 40);
                    let blade = egui::Color32::from_rgb(95, 70, 38);
                    let blade_edge = egui::Color32::from_rgb(75, 55, 30);
                    // Handle
                    item_painter.line_segment(
                        [
                            egui::pos2(center.x, center.y - r * 0.7),
                            egui::pos2(center.x, center.y + r * 0.2),
                        ],
                        egui::Stroke::new(r * 0.18, handle),
                    );
                    // Grip nub at top
                    item_painter.circle_filled(
                        egui::pos2(center.x, center.y - r * 0.7),
                        r * 0.14,
                        handle,
                    );
                    // Blade (rounded rectangle)
                    let blade_center = egui::pos2(center.x, center.y + r * 0.45);
                    item_painter.rect_filled(
                        egui::Rect::from_center_size(blade_center, egui::vec2(r * 0.7, r * 0.55)),
                        r * 0.2,
                        blade,
                    );
                    // Edge highlight at bottom
                    item_painter.line_segment(
                        [
                            egui::pos2(center.x - r * 0.3, center.y + r * 0.68),
                            egui::pos2(center.x + r * 0.3, center.y + r * 0.68),
                        ],
                        egui::Stroke::new(r * 0.08, blade_edge),
                    );
                } else if iid == item_defs::ITEM_FIBER {
                    // Bundled plant fiber — twisted strands
                    let green = egui::Color32::from_rgb(75, 135, 45);
                    let light = egui::Color32::from_rgb(110, 170, 65);
                    for i in 0..4u32 {
                        let offset = (i as f32 - 1.5) * r * 0.18;
                        let wave = (i as f32 * 1.3).sin() * r * 0.12;
                        let col = if i % 2 == 0 { green } else { light };
                        item_painter.line_segment(
                            [
                                egui::pos2(center.x + offset - wave, center.y + r * 0.55),
                                egui::pos2(center.x + offset + wave, center.y - r * 0.55),
                            ],
                            egui::Stroke::new(r * 0.13, col),
                        );
                    }
                    // Tie in middle
                    item_painter.line_segment(
                        [
                            egui::pos2(center.x - r * 0.25, center.y),
                            egui::pos2(center.x + r * 0.25, center.y),
                        ],
                        egui::Stroke::new(r * 0.08, egui::Color32::from_rgb(140, 120, 70)),
                    );
                } else if iid == item_defs::ITEM_SCRAP_WOOD {
                    // Scattered sticks/twigs
                    let colors = [
                        egui::Color32::from_rgb(140, 95, 48),
                        egui::Color32::from_rgb(125, 85, 40),
                        egui::Color32::from_rgb(150, 105, 55),
                    ];
                    let count = (n as u32).min(4);
                    for i in 0..count {
                        let a = i as f32 * 0.8 + 0.3;
                        let len = r * (0.7 + (i as f32 * 0.37).sin() * 0.3);
                        let cx = center.x + (i as f32 - 1.0) * r * 0.15;
                        let cy = center.y + (i as f32 - 1.0) * r * 0.1;
                        item_painter.line_segment(
                            [
                                egui::pos2(cx - a.cos() * len * 0.5, cy - a.sin() * len * 0.5),
                                egui::pos2(cx + a.cos() * len * 0.5, cy + a.sin() * len * 0.5),
                            ],
                            egui::Stroke::new(r * 0.14, colors[i as usize % 3]),
                        );
                    }
                } else if iid == item_defs::ITEM_CLAY {
                    // Clay lump with wet sheen
                    let base = egui::Color32::from_rgb(155, 100, 55);
                    let mid = egui::Color32::from_rgb(135, 85, 45);
                    let sheen = egui::Color32::from_rgb(180, 135, 85);
                    // Main body (slightly flattened)
                    item_painter.rect_filled(
                        egui::Rect::from_center_size(center, egui::vec2(r * 1.4, r * 1.0)),
                        r * 0.45,
                        base,
                    );
                    // Depth shadow
                    item_painter.circle_filled(
                        egui::pos2(center.x + r * 0.1, center.y + r * 0.1),
                        r * 0.45,
                        mid,
                    );
                    // Wet highlight
                    item_painter.circle_filled(
                        egui::pos2(center.x - r * 0.2, center.y - r * 0.15),
                        r * 0.2,
                        sheen,
                    );
                } else if iid == item_defs::ITEM_ROPE {
                    // Coiled rope with visible winds
                    let outer = egui::Color32::from_rgb(165, 140, 85);
                    let inner = egui::Color32::from_rgb(140, 115, 65);
                    let highlight = egui::Color32::from_rgb(190, 170, 115);
                    // Outer coil
                    item_painter.circle_stroke(center, r * 0.6, egui::Stroke::new(r * 0.28, outer));
                    // Inner coil
                    item_painter.circle_stroke(
                        center,
                        r * 0.35,
                        egui::Stroke::new(r * 0.12, inner),
                    );
                    // Highlight on top
                    item_painter.circle_filled(
                        egui::pos2(center.x - r * 0.15, center.y - r * 0.4),
                        r * 0.12,
                        highlight,
                    );
                    // Tail end
                    item_painter.line_segment(
                        [
                            egui::pos2(center.x + r * 0.5, center.y + r * 0.35),
                            egui::pos2(center.x + r * 0.7, center.y + r * 0.55),
                        ],
                        egui::Stroke::new(r * 0.12, outer),
                    );
                } else if iid == item_defs::ITEM_PLANK {
                    // Stacked planks with wood grain
                    let plank_col = egui::Color32::from_rgb(185, 150, 90);
                    let grain = egui::Color32::from_rgb(160, 125, 70);
                    let edge = egui::Color32::from_rgb(145, 110, 60);
                    let count = (n as u32).min(4);
                    for i in 0..count {
                        let py = center.y - r * 0.4 + i as f32 * r * 0.3;
                        let pw = r * 1.4;
                        let ph = r * 0.25;
                        let rect = egui::Rect::from_center_size(
                            egui::pos2(center.x, py),
                            egui::vec2(pw, ph),
                        );
                        item_painter.rect_filled(rect, ph * 0.15, plank_col);
                        // Grain line
                        item_painter.line_segment(
                            [
                                egui::pos2(center.x - pw * 0.4, py),
                                egui::pos2(center.x + pw * 0.4, py),
                            ],
                            egui::Stroke::new(r * 0.03, grain),
                        );
                        // Bottom edge shadow
                        item_painter.line_segment(
                            [
                                egui::pos2(rect.min.x, rect.max.y),
                                egui::pos2(rect.max.x, rect.max.y),
                            ],
                            egui::Stroke::new(r * 0.04, edge),
                        );
                    }
                } else if iid == item_defs::ITEM_WOODEN_BUCKET {
                    // Bucket: tapered cylinder with handle
                    let wood = egui::Color32::from_rgb(145, 105, 55);
                    let dark = egui::Color32::from_rgb(110, 78, 38);
                    let band = egui::Color32::from_rgb(90, 90, 85);
                    // Body
                    item_painter.rect_filled(
                        egui::Rect::from_center_size(
                            egui::pos2(center.x, center.y + r * 0.1),
                            egui::vec2(r * 1.0, r * 1.1),
                        ),
                        r * 0.15,
                        wood,
                    );
                    // Metal bands
                    for y_off in [-0.2f32, 0.3] {
                        item_painter.line_segment(
                            [
                                egui::pos2(center.x - r * 0.48, center.y + r * y_off),
                                egui::pos2(center.x + r * 0.48, center.y + r * y_off),
                            ],
                            egui::Stroke::new(r * 0.08, band),
                        );
                    }
                    // Handle
                    item_painter.circle_stroke(
                        egui::pos2(center.x, center.y - r * 0.5),
                        r * 0.3,
                        egui::Stroke::new(r * 0.08, dark),
                    );
                    // Liquid fill
                    if let Some((_, amt)) = item.stack.liquid {
                        let cap = item.stack.liquid_capacity();
                        if cap > 0 && amt > 0 {
                            let fill = amt as f32 / cap as f32;
                            let fill_h = r * 0.9 * fill;
                            item_painter.rect_filled(
                                egui::Rect::from_min_size(
                                    egui::pos2(center.x - r * 0.4, center.y + r * 0.55 - fill_h),
                                    egui::vec2(r * 0.8, fill_h),
                                ),
                                r * 0.08,
                                egui::Color32::from_rgba_unmultiplied(60, 130, 210, 160),
                            );
                        }
                    }
                } else if iid == item_defs::ITEM_CLAY_JUG || iid == item_defs::ITEM_UNFIRED_JUG {
                    // Amphora/jug shape
                    let col = if iid == item_defs::ITEM_CLAY_JUG {
                        egui::Color32::from_rgb(165, 100, 55) // fired: warm terracotta
                    } else {
                        egui::Color32::from_rgb(140, 120, 100) // unfired: grey-clay
                    };
                    let dark = egui::Color32::from_rgb(
                        (col.r() as i32 - 30).max(0) as u8,
                        (col.g() as i32 - 25).max(0) as u8,
                        (col.b() as i32 - 20).max(0) as u8,
                    );
                    // Body (wide)
                    item_painter.rect_filled(
                        egui::Rect::from_center_size(
                            egui::pos2(center.x, center.y + r * 0.1),
                            egui::vec2(r * 0.9, r * 0.9),
                        ),
                        r * 0.35,
                        col,
                    );
                    // Neck (narrow)
                    item_painter.rect_filled(
                        egui::Rect::from_center_size(
                            egui::pos2(center.x, center.y - r * 0.45),
                            egui::vec2(r * 0.35, r * 0.4),
                        ),
                        r * 0.08,
                        col,
                    );
                    // Rim
                    item_painter.line_segment(
                        [
                            egui::pos2(center.x - r * 0.25, center.y - r * 0.6),
                            egui::pos2(center.x + r * 0.25, center.y - r * 0.6),
                        ],
                        egui::Stroke::new(r * 0.1, dark),
                    );
                    // Liquid fill
                    if let Some((_, amt)) = item.stack.liquid {
                        let cap = item.stack.liquid_capacity();
                        if cap > 0 && amt > 0 {
                            let fill = amt as f32 / cap as f32;
                            let fill_h = r * 0.7 * fill;
                            item_painter.rect_filled(
                                egui::Rect::from_min_size(
                                    egui::pos2(center.x - r * 0.35, center.y + r * 0.45 - fill_h),
                                    egui::vec2(r * 0.7, fill_h),
                                ),
                                r * 0.15,
                                egui::Color32::from_rgba_unmultiplied(60, 130, 210, 140),
                            );
                        }
                    }
                } else {
                    // Generic item: colored circle with highlight
                    let col = egui::Color32::from_rgb(110, 105, 90);
                    item_painter.circle_filled(center, r * 0.8, col);
                    item_painter.circle_filled(
                        egui::pos2(center.x - r * 0.15, center.y - r * 0.15),
                        r * 0.25,
                        egui::Color32::from_rgb(140, 135, 120),
                    );
                }
                if tile_px > 6.0 {
                    let label = if item.stack.is_container() {
                        item.stack.label()
                    } else {
                        format!("{}x", n)
                    };
                    Self::world_label(
                        &item_painter,
                        egui::pos2(center.x, center.y + r + 2.0),
                        egui::Align2::CENTER_TOP,
                        &label,
                        9.0,
                        egui::Color32::WHITE,
                    );
                }

                // Tooltip on hover
                let mouse = ctx.input(|i| i.pointer.hover_pos());
                if let Some(mp) = mouse {
                    let dx = mp.x - center.x;
                    let dy = mp.y - center.y;
                    if dx * dx + dy * dy < r * r * 2.0 {
                        let item_reg = item_defs::ItemRegistry::cached();
                        let name = item_reg.name(iid);
                        let tip = if item.stack.is_container() {
                            format!("{} ({})", name, item.stack.label())
                        } else if n > 1 {
                            format!("{} x{}", name, n)
                        } else {
                            name.to_string()
                        };
                        egui::show_tooltip_at_pointer(
                            ctx,
                            egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("item_tip")),
                            egui::Id::new("item_tip_inner"),
                            |ui| {
                                ui.label(egui::RichText::new(tip).size(11.0));
                            },
                        );
                    }
                }
            }
        }

        // Render physics bodies
        if !self.physics_bodies.is_empty() {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
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
                                egui::Stroke::new(
                                    0.5,
                                    egui::Color32::from_rgba_unmultiplied(100, 100, 100, 80),
                                ),
                            );
                        }

                        // --- Cannonball (dark sphere with highlight) ---
                        let ball_r = 0.10 * tile_px;
                        let ball_pos = egui::pos2(gx, gy - z_offset);
                        painter.circle_filled(
                            ball_pos,
                            ball_r,
                            egui::Color32::from_rgb(40, 38, 35),
                        );
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
                                egui::Rect::from_min_max(
                                    egui::pos2(gx0, gy0),
                                    egui::pos2(gx1, gy1),
                                ),
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
                        let rotated: Vec<egui::Pos2> = corners
                            .iter()
                            .map(|&(cx, cy)| {
                                let sx = cx * half * scale_x;
                                let sy = cy * half * scale_y;
                                let rx = sx * cos_z - sy * sin_z;
                                let ry = sx * sin_z + sy * cos_z;
                                center_s + egui::Vec2::new(rx, ry)
                            })
                            .collect();
                        let brightness = (160.0 + body.z * 15.0).min(200.0) as u8;
                        let gb = (120.0 + body.z * 10.0).min(160.0) as u8;
                        let fill_color = egui::Color32::from_rgb(brightness, gb, 60);
                        let stroke_color = egui::Color32::from_rgb(100, 75, 35);
                        painter.add(egui::Shape::convex_polygon(
                            rotated.clone(),
                            fill_color,
                            egui::Stroke::new(1.5, stroke_color),
                        ));
                        for i in 0..3 {
                            let t = 0.25 + i as f32 * 0.25;
                            let lx = rotated[0].x + (rotated[3].x - rotated[0].x) * t;
                            let ly = rotated[0].y + (rotated[3].y - rotated[0].y) * t;
                            let rx = rotated[1].x + (rotated[2].x - rotated[1].x) * t;
                            let ry = rotated[1].y + (rotated[2].y - rotated[1].y) * t;
                            painter.line_segment(
                                [egui::pos2(lx, ly), egui::pos2(rx, ry)],
                                egui::Stroke::new(
                                    0.5,
                                    egui::Color32::from_rgba_unmultiplied(90, 65, 30, 100),
                                ),
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
                            [
                                egui::pos2(gx, gy - z_offset),
                                egui::pos2(gx + dx, gy - z_offset + dy),
                            ],
                            egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 240, 150)),
                        );

                        // Debug: kinematic label
                        if self.debug_bullet_slowmo {
                            let ke = 0.5 * body.mass * speed * speed;
                            let angle_deg = body.vy.atan2(body.vx).to_degrees();
                            let label = format!(
                                "v={:.1} m={:.3}\nKE={:.1} z={:.2}\nθ={:.0}° vz={:.1}",
                                speed, body.mass, ke, body.z, angle_deg, body.vz
                            );
                            let label_pos = egui::pos2(gx + 6.0, gy - z_offset - 6.0);
                            // Background
                            let galley = painter.layout_no_wrap(
                                label.clone(),
                                egui::FontId::monospace(9.0),
                                egui::Color32::from_rgb(255, 255, 200),
                            );
                            let text_rect =
                                egui::Align2::LEFT_BOTTOM.anchor_size(label_pos, galley.size());
                            painter.rect_filled(
                                text_rect.expand(2.0),
                                2.0,
                                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200),
                            );
                            painter.text(
                                label_pos,
                                egui::Align2::LEFT_BOTTOM,
                                &label,
                                egui::FontId::monospace(9.0),
                                egui::Color32::from_rgb(255, 255, 200),
                            );
                        }
                    }
                    physics::BodyType::Grenade => {
                        let shadow_scale = (1.0 - body.z * 0.15).max(0.2);
                        let shadow_r = 0.08 * shadow_scale * tile_px;
                        let (gx, gy) = to_screen(body.x, body.y);
                        let shadow_alpha = (150.0 * (1.0 - body.z * 0.06).max(0.15)) as u8;
                        painter.circle_filled(
                            egui::pos2(gx, gy),
                            shadow_r,
                            egui::Color32::from_rgba_unmultiplied(15, 15, 15, shadow_alpha),
                        );
                        let ball_r = 0.07 * tile_px;
                        let ball_pos = egui::pos2(gx, gy - z_offset);
                        painter.circle_filled(
                            ball_pos,
                            ball_r,
                            egui::Color32::from_rgb(40, 60, 30),
                        );
                        painter.circle_filled(
                            ball_pos + egui::Vec2::new(-ball_r * 0.2, -ball_r * 0.2),
                            ball_r * 0.3,
                            egui::Color32::from_rgb(70, 90, 50),
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
                            ui.add(
                                egui::Slider::new(&mut cell.pump_rate, 0.0..=20.0)
                                    .text("Rate")
                                    .step_by(0.5),
                            );
                            ui.label(
                                egui::RichText::new(format!("P: {:.1}", cell.pressure))
                                    .size(9.0)
                                    .weak(),
                            );
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
                        ui.add(
                            egui::Slider::new(&mut self.fluid_params.fan_speed, 0.0..=80.0)
                                .text("Speed")
                                .step_by(1.0),
                        );
                        if ui.small_button("Close").clicked() {
                            still_valid = false;
                        }
                    });
                });
            if !still_valid {
                self.block_sel.fan = None;
            }
        }

        // Dimmer / Restrictor slider popup (shared)
        if let Some(dimmer_idx) = self.block_sel.dimmer {
            let (dwx, dwy) = self.block_sel.dimmer_world;
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let sx = ((dwx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp + 20.0;
            let sy = ((dwy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp - 10.0;
            let mut still_valid = true;
            let didx = dimmer_idx as usize;
            let dblock = if didx < self.grid_data.len() {
                self.grid_data[didx]
            } else {
                0
            };
            let dbt = dblock & 0xFF;
            if dbt != 43 && dbt != 46 && dbt != 6 {
                still_valid = false;
            }
            if still_valid {
                let (title, max_val, mask) = match dbt {
                    43 => ("Dimmer", 10i32, 0xFFu32),
                    46 => ("Restrictor", 10i32, 0x0Fu32),
                    6 => ("Fireplace", 10i32, 0xFFu32),
                    _ => ("", 10i32, 0xFFu32),
                };
                let mut level = (((dblock >> 8) & mask) as i32).min(max_val);
                egui::Area::new(egui::Id::new("dimmer_slider"))
                    .fixed_pos(egui::pos2(sx, sy))
                    .show(ctx, |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.label(egui::RichText::new(title).strong().size(11.0));
                            let display = if dbt == 6 {
                                let temp = 100 + level * 50; // 100°C at 0, 600°C at 10
                                format!("{}°C ({:.0}%)", temp, level as f32 * 10.0)
                            } else if dbt == 46 {
                                format!("{:.0}% open", level as f32 * 10.0)
                            } else {
                                format!("{:.0}%", level as f32 * 10.0)
                            };
                            ui.label(egui::RichText::new(display).size(10.0));
                            ui.add(
                                egui::Slider::new(&mut level, 0..=max_val)
                                    .text("Level")
                                    .step_by(1.0),
                            );
                            // Write back: for restrictor preserve upper nibble (conn mask)
                            let new_h = if dbt == 46 {
                                let existing_upper = (dblock >> 8) & 0xF0;
                                existing_upper | (level as u32 & 0x0F)
                            } else {
                                level as u32 & 0xFF
                            };
                            let new_block = (dblock & 0xFFFF00FF) | (new_h << 8);
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
                if (cb & 0xFF) != 33 {
                    still_valid = false;
                }
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
                            ui.label(
                                egui::RichText::new(format!(
                                    "Storage Crate ({}/{})",
                                    total, CRATE_MAX_ITEMS
                                ))
                                .strong()
                                .size(12.0),
                            );
                            ui.separator();
                            if let Some(inv) = inv {
                                if !inv.stacks.is_empty() {
                                    for stack in &inv.stacks {
                                        ui.label(egui::RichText::new(stack.label()).size(11.0));
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

        // --- Crafting station recipe popup (workbench or kiln) ---
        if let Some(wb_idx) = self.block_sel.workbench {
            let (wwx, wwy) = self.block_sel.workbench_world;
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let sx = ((wwx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp + 20.0;
            let sy = ((wwy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp - 40.0;
            let mut still_valid = wb_idx < (GRID_W * GRID_H);
            let station_bt = if still_valid {
                let didx = wb_idx as usize;
                if didx < self.grid_data.len() {
                    let bt = block_type_rs(self.grid_data[didx]);
                    if bt == BT_WORKBENCH || bt == BT_KILN {
                        bt
                    } else {
                        still_valid = false;
                        0
                    }
                } else {
                    still_valid = false;
                    0
                }
            } else {
                0
            };
            if still_valid {
                let station_name = match station_bt {
                    BT_KILN => "kiln",
                    BT_SAW_HORSE => "saw_horse",
                    _ => "workbench",
                };
                let station_label = match station_bt {
                    BT_KILN => "Kiln",
                    BT_SAW_HORSE => "Saw Horse",
                    _ => "Workbench",
                };
                let recipe_reg = recipe_defs::RecipeRegistry::cached();
                let item_reg = item_defs::ItemRegistry::cached();
                let recipes = recipe_reg.for_station(station_name);
                let queue = self.craft_queues.entry(wb_idx).or_default();
                egui::Area::new(egui::Id::new("craft_station_popup"))
                    .fixed_pos(egui::pos2(sx, sy))
                    .show(ctx, |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.label(egui::RichText::new(station_label).strong().size(12.0));
                            ui.separator();

                            // Show queued orders
                            let mut remove_idx = None;
                            if !queue.orders.is_empty() {
                                ui.label(egui::RichText::new("Queue:").size(10.0).strong());
                                for (qi, order) in queue.orders.iter().enumerate() {
                                    let rname = recipe_reg
                                        .get(order.recipe_id)
                                        .map(|r| r.name.as_str())
                                        .unwrap_or("?");
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "  {} — {}/{}",
                                                rname, order.completed, order.count
                                            ))
                                            .size(10.0),
                                        );
                                        if ui.small_button("x").clicked() {
                                            remove_idx = Some(qi);
                                        }
                                    });
                                }
                                if let Some(ri) = remove_idx {
                                    queue.orders.remove(ri);
                                }
                                ui.separator();
                            }

                            // Add new orders
                            for recipe in &recipes {
                                let out_name = item_reg.name(recipe.output.item);
                                let ingredients =
                                    recipe_defs::RecipeRegistry::ingredients_label(recipe);
                                ui.horizontal(|ui| {
                                    let label = format!("{} ({})", recipe.name, ingredients);
                                    ui.label(egui::RichText::new(&label).size(10.0));
                                    ui.label(
                                        egui::RichText::new(format!("→ {}", out_name)).size(10.0),
                                    );
                                    for &n in &[1u16, 5, 10] {
                                        if ui.small_button(format!("+{}", n)).clicked() {
                                            // Add to queue (merge if same recipe)
                                            if let Some(existing) =
                                                queue.orders.iter_mut().find(|o| {
                                                    o.recipe_id == recipe.id
                                                        && o.completed < o.count
                                                })
                                            {
                                                existing.count += n;
                                            } else {
                                                queue.orders.push(CraftOrder {
                                                    recipe_id: recipe.id,
                                                    count: n,
                                                    completed: 0,
                                                });
                                            }
                                        }
                                    }
                                });
                            }
                            ui.separator();
                            if ui.small_button("Close").clicked() {
                                still_valid = false;
                            }
                        });
                    });
            }
            if !still_valid {
                self.block_sel.workbench = None;
            }
        }
    }

    /// Draw text with a dark shadow for readability on bright backgrounds.
    fn shadow_text(
        painter: &egui::Painter,
        pos: egui::Pos2,
        anchor: egui::Align2,
        text: &str,
        font: egui::FontId,
        color: egui::Color32,
    ) {
        // 30% opacity black background behind text
        let galley = painter.layout_no_wrap(text.to_string(), font.clone(), color);
        let text_rect = anchor.anchor_size(pos, galley.size());
        let pad = egui::Vec2::new(3.0, 1.0);
        painter.rect_filled(
            text_rect.expand2(pad),
            2.0,
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 77),
        );
        painter.text(pos, anchor, text, font, color);
    }

    fn world_label(
        painter: &egui::Painter,
        pos: egui::Pos2,
        anchor: egui::Align2,
        text: &str,
        size: f32,
        color: egui::Color32,
    ) {
        Self::shadow_text(
            painter,
            pos,
            anchor,
            text,
            egui::FontId::proportional(size),
            color,
        );
    }

    fn draw_world_labels(&mut self, ctx: &egui::Context, bp_cam: (f32, f32, f32, f32, f32)) {
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
                    egui::Order::Background,
                    egui::Id::new("world_labels"),
                ));

                // Pleb name + activity labels
                let screen_rect = ctx.content_rect();
                for pleb in &self.plebs {
                    let pos = to_screen(pleb.x, pleb.y + 0.7);
                    // Cull off-screen plebs
                    if pos.x < screen_rect.min.x - 60.0
                        || pos.x > screen_rect.max.x + 60.0
                        || pos.y < screen_rect.min.y - 60.0
                        || pos.y > screen_rect.max.y + 60.0
                    {
                        continue;
                    }

                    // Name label (always visible) — red when drafted
                    let name_color = if pleb.drafted {
                        egui::Color32::from_rgb(255, 120, 80)
                    } else if pleb.is_enemy {
                        egui::Color32::from_rgb(255, 50, 50)
                    } else if pleb.activity.is_crisis() {
                        egui::Color32::from_rgb(255, 80, 80)
                    } else {
                        egui::Color32::from_rgb(220, 220, 220)
                    };
                    let display_name = if pleb.drafted {
                        format!("[D] {}", pleb.name)
                    } else {
                        pleb.name.clone()
                    };
                    Self::shadow_text(
                        &label_painter,
                        pos,
                        egui::Align2::CENTER_TOP,
                        &display_name,
                        egui::FontId::proportional(11.0),
                        name_color,
                    );

                    // Activity label + intent (when not idle)
                    if tile_px > 10.0 {
                        let inner = pleb.activity.inner();
                        // Determine planting vs harvesting from work target
                        let work_action = if let Some((tx, ty)) = pleb.work_target {
                            let tidx = (ty as u32 * GRID_W + tx as u32) as usize;
                            if tidx < self.grid_data.len() {
                                let tbt = self.grid_data[tidx] & 0xFF;
                                if tbt == BT_WORKBENCH || tbt == BT_KILN {
                                    "Crafting"
                                } else if tbt == BT_WELL {
                                    "Drinking"
                                } else if tbt == BT_CROP || tbt == BT_BERRY_BUSH {
                                    "Harvesting"
                                } else if tbt == BT_TREE {
                                    "Chopping"
                                } else {
                                    "Planting"
                                }
                            } else {
                                "Working"
                            }
                        } else {
                            "Working"
                        };
                        let (act_text, act_color) = match inner {
                            PlebActivity::Idle => {
                                if pleb.work_target.is_some() {
                                    (Some(work_action), egui::Color32::from_rgb(120, 200, 80))
                                } else {
                                    (None, egui::Color32::GRAY)
                                }
                            }
                            PlebActivity::Walking => {
                                if pleb.work_target.is_some() {
                                    let label = match work_action {
                                        "Harvesting" | "Chopping" => "Walking to harvest",
                                        "Crafting" => "Walking to craft",
                                        "Drinking" => "Walking to drink",
                                        _ => "Walking to plant",
                                    };
                                    (Some(label), egui::Color32::from_rgb(120, 200, 80))
                                } else if pleb.harvest_target.is_some() {
                                    (
                                        Some("Walking to harvest"),
                                        egui::Color32::from_rgb(200, 180, 60),
                                    )
                                } else if pleb.haul_target.is_some() {
                                    (Some("Hauling"), egui::Color32::from_rgb(180, 140, 80))
                                } else {
                                    (None, egui::Color32::GRAY)
                                }
                            }
                            PlebActivity::Sleeping => {
                                (Some("Zzz..."), egui::Color32::from_rgb(120, 140, 200))
                            }
                            PlebActivity::Harvesting(_) => {
                                (Some("Harvesting"), egui::Color32::from_rgb(200, 180, 60))
                            }
                            PlebActivity::Eating => {
                                (Some("Eating"), egui::Color32::from_rgb(200, 160, 80))
                            }
                            PlebActivity::Hauling => {
                                (Some("Hauling"), egui::Color32::from_rgb(180, 140, 80))
                            }
                            PlebActivity::Farming(_) => {
                                (Some(work_action), egui::Color32::from_rgb(80, 200, 80))
                            }
                            PlebActivity::Building(_) => {
                                (Some("Building"), egui::Color32::from_rgb(100, 160, 220))
                            }
                            PlebActivity::Crafting(_, _) => {
                                (Some("Crafting"), egui::Color32::from_rgb(200, 160, 60))
                            }
                            PlebActivity::Drinking(_) => {
                                (Some("Drinking"), egui::Color32::from_rgb(80, 160, 220))
                            }
                            PlebActivity::MentalBreak(_, _) => {
                                (Some("Mental break!"), egui::Color32::from_rgb(200, 60, 200))
                            }
                            PlebActivity::Staggering(_) => {
                                (Some("Staggering!"), egui::Color32::from_rgb(255, 140, 40))
                            }
                            PlebActivity::Crisis(_, _) => (None, egui::Color32::GRAY),
                        };
                        if let Some(text) = act_text {
                            let act_pos = to_screen(pleb.x, pleb.y + 0.95);
                            Self::world_label(
                                &label_painter,
                                act_pos,
                                egui::Align2::CENTER_TOP,
                                text,
                                9.0,
                                act_color,
                            );
                        }

                        // Progress bar for farming/harvesting
                        let progress = match inner {
                            PlebActivity::Farming(p) => Some(*p),
                            PlebActivity::Harvesting(p) => Some(*p),
                            _ => None,
                        };
                        if let Some(prog) = progress {
                            let bar_pos = to_screen(pleb.x - 0.35, pleb.y + 1.1);
                            let bar_w = tile_px * 0.7;
                            let bar_h = tile_px * 0.06;
                            label_painter.rect_filled(
                                egui::Rect::from_min_size(bar_pos, egui::Vec2::new(bar_w, bar_h)),
                                1.0,
                                egui::Color32::from_rgb(30, 30, 30),
                            );
                            label_painter.rect_filled(
                                egui::Rect::from_min_size(
                                    bar_pos,
                                    egui::Vec2::new(bar_w * prog, bar_h),
                                ),
                                1.0,
                                egui::Color32::from_rgb(80, 200, 80),
                            );
                        }

                        // Work target line: show where pleb is heading (selected pleb only)
                        let is_selected = self.selected_pleb.is_some_and(|si| {
                            si < self.plebs.len() && std::ptr::eq(&self.plebs[si], pleb)
                        });
                        if is_selected && let Some((tx, ty)) = pleb.work_target {
                            let target_pos = to_screen(tx as f32 + 0.5, ty as f32 + 0.5);
                            let pleb_pos = to_screen(pleb.x, pleb.y);
                            label_painter.line_segment(
                                [pleb_pos, target_pos],
                                egui::Stroke::new(
                                    1.0,
                                    egui::Color32::from_rgba_unmultiplied(80, 200, 80, 100),
                                ),
                            );
                        }
                        // Crisis reason
                        if let Some(reason) = pleb.activity.crisis_reason() {
                            let crisis_pos = to_screen(pleb.x, pleb.y + 0.95);
                            Self::world_label(
                                &label_painter,
                                crisis_pos,
                                egui::Align2::CENTER_TOP,
                                reason,
                                10.0,
                                egui::Color32::from_rgb(255, 60, 60),
                            );
                        }
                    }
                }

                // Fire mode indicator above selected pleb
                if let Some(pleb) = self.selected_pleb.and_then(|i| self.plebs.get(i))
                    && !pleb.is_enemy
                {
                    let mode_pos = to_screen(pleb.x, pleb.y - 0.8);
                    let mode_text = if self.burst_mode { "BURST" } else { "SINGLE" };
                    Self::world_label(
                        &label_painter,
                        mode_pos,
                        egui::Align2::CENTER_BOTTOM,
                        mode_text,
                        9.0,
                        egui::Color32::from_rgb(180, 180, 100),
                    );
                }

                // Grenade charge bar above selected pleb
                if self.grenade_charging
                    && let Some(pleb) = self.selected_pleb.and_then(|i| self.plebs.get(i))
                {
                    let bar_pos = to_screen(pleb.x - 0.4, pleb.y - 0.6);
                    let bar_w = tile_px * 0.8;
                    let bar_h = tile_px * 0.08;
                    let charge = self.grenade_charge.clamp(0.0, 1.0);
                    // Background
                    label_painter.rect_filled(
                        egui::Rect::from_min_size(bar_pos, egui::Vec2::new(bar_w, bar_h)),
                        1.0,
                        egui::Color32::from_rgb(30, 30, 30),
                    );
                    // Fill (green to red as charge increases)
                    let r = (charge * 255.0) as u8;
                    let g = ((1.0 - charge) * 200.0) as u8;
                    label_painter.rect_filled(
                        egui::Rect::from_min_size(bar_pos, egui::Vec2::new(bar_w * charge, bar_h)),
                        1.0,
                        egui::Color32::from_rgb(r, g, 40),
                    );
                }
            }
        }
    }

    /// Draw selection action buttons inline (called from draw_build_bar when items are selected).
    fn draw_selection_actions_inner(&mut self, ui: &mut egui::Ui) {
        if self.world_sel.is_empty() {
            return;
        }

        let reg = block_defs::BlockRegistry::cached();
        let items: Vec<SelectedItem> = self.world_sel.items.clone();
        let count = items.len();
        if count == 0 {
            return;
        }

        // Determine common properties
        let all_removable = items
            .iter()
            .all(|item| reg.get(item.block_type).is_some_and(|d| d.is_removable));
        let all_same_type = items
            .iter()
            .all(|item| item.block_type == items[0].block_type);

        let pleb_count = items.iter().filter(|i| i.pleb_idx.is_some()).count();
        let block_count = count - pleb_count;

        // Label
        let label = if count == 1 && items[0].pleb_idx.is_some() {
            items[0]
                .pleb_idx
                .and_then(|pi| self.plebs.get(pi))
                .map_or("Pleb".to_string(), |p| p.name.clone())
        } else if count == 1 {
            reg.name(items[0].block_type).to_string()
        } else if all_same_type && items[0].pleb_idx.is_none() {
            format!("{}x {}", count, reg.name(items[0].block_type))
        } else {
            let mut parts = Vec::with_capacity(2);
            if pleb_count > 0 {
                parts.push(format!(
                    "{} pleb{}",
                    pleb_count,
                    if pleb_count > 1 { "s" } else { "" }
                ));
            }
            if block_count > 0 {
                parts.push(format!(
                    "{} block{}",
                    block_count,
                    if block_count > 1 { "s" } else { "" }
                ));
            }
            parts.join(", ")
        };

        // Action buttons: square icons with labels, same style as build bar
        // Collect available actions as (icon, label, id)
        let mut actions: Vec<(&str, &str, u32)> = Vec::with_capacity(4);
        if all_removable {
            actions.push(("\u{274c}", "Destroy", 0));
        }
        let any_harvestable = items.iter().any(|item| {
            item.pleb_idx.is_none() && reg.get(item.block_type).is_some_and(|d| d.is_harvestable)
        });
        if any_harvestable {
            actions.push(("\u{1f33e}", "Harvest", 1));
        }
        let bp_items: Vec<(i32, i32)> = items
            .iter()
            .filter(|i| self.blueprints.contains_key(&(i.x, i.y)))
            .map(|i| (i.x, i.y))
            .collect();
        if !bp_items.is_empty() {
            actions.push(("\u{1f6d1}", "Cancel", 2));
        }
        if count == 1 && items[0].block_type == BT_CRATE {
            actions.push(("\u{1f4e6}", "Inspect", 3));
        }

        // Title
        ui.label(egui::RichText::new(&label).strong().size(11.0));
        if !actions.is_empty() {
            ui.separator();
        }

        // Square action buttons in a single column
        let tile_size = 48.0;
        let icon_s = 20.0;
        let label_s = 9.0;
        for &(icon, act_label, id) in &actions {
            let (rect, response) =
                ui.allocate_exact_size(egui::Vec2::new(tile_size, tile_size), egui::Sense::click());
            let painter = ui.painter_at(rect);
            let bg = if response.hovered() {
                egui::Color32::from_rgb(60, 65, 75)
            } else {
                egui::Color32::from_rgb(42, 44, 50)
            };
            painter.rect_filled(rect, 4.0, bg);
            painter.rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(1.0, egui::Color32::from_gray(70)),
                egui::StrokeKind::Outside,
            );
            painter.text(
                rect.center() + egui::Vec2::new(0.0, -6.0),
                egui::Align2::CENTER_CENTER,
                icon,
                egui::FontId::proportional(icon_s),
                egui::Color32::WHITE,
            );
            painter.text(
                rect.center() + egui::Vec2::new(0.0, 14.0),
                egui::Align2::CENTER_CENTER,
                act_label,
                egui::FontId::proportional(label_s),
                egui::Color32::from_gray(190),
            );

            if response.clicked() {
                match id {
                    0 => {
                        let positions: Vec<(i32, i32)> = items.iter().map(|i| (i.x, i.y)).collect();
                        for (x, y) in positions {
                            self.destroy_block_at(x, y);
                        }
                        self.world_sel = WorldSelection::none();
                    }
                    1 => {
                        for item in &items {
                            if item.pleb_idx.is_some() {
                                continue;
                            }
                            if reg.get(item.block_type).is_some_and(|d| d.is_harvestable) {
                                self.manual_tasks
                                    .push(zones::WorkTask::Harvest(item.x, item.y));
                            }
                        }
                    }
                    2 => {
                        for (x, y) in &bp_items {
                            self.cancel_blueprint(*x, *y);
                        }
                        self.world_sel = WorldSelection::none();
                    }
                    3 => {
                        if count == 1 {
                            let item = &items[0];
                            let cidx = item.y as u32 * GRID_W + item.x as u32;
                            self.block_sel.crate_idx = Some(cidx);
                            self.block_sel.crate_world = (item.x as f32 + 0.5, item.y as f32 + 0.5);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Detail section below actions (for plants, plebs, etc.)
        if count == 1 && items[0].pleb_idx.is_none() {
            let item = &items[0];
            let tidx = (item.y as u32 * GRID_W + item.x as u32) as usize;
            if tidx < self.grid_data.len() {
                let tblock = self.grid_data[tidx];
                let wt = if tidx < self.water_table.len() {
                    self.water_table[tidx]
                } else {
                    -3.0
                };
                let timer = self.crop_timers.get(&(tidx as u32)).copied().unwrap_or(0.0);
                if let Some(cs) = zones::crop_status(
                    tblock,
                    tidx as u32,
                    timer,
                    self.time_of_day,
                    self.camera.sun_intensity,
                    self.camera.rain_intensity,
                    wt,
                    self.debug.water_level,
                ) {
                    ui.separator();
                    // Growth progress bar
                    let total_progress = (cs.stage as f32 + cs.progress) / 4.0;
                    let bar_w = 190.0;
                    let bar_h = 8.0;
                    let (bar_rect, _) =
                        ui.allocate_exact_size(egui::Vec2::new(bar_w, bar_h), egui::Sense::hover());
                    let bp = ui.painter_at(bar_rect);
                    bp.rect_filled(bar_rect, 2.0, egui::Color32::from_rgb(30, 30, 30));
                    bp.rect_filled(
                        egui::Rect::from_min_size(
                            bar_rect.min,
                            egui::Vec2::new(bar_w * total_progress, bar_h),
                        ),
                        2.0,
                        egui::Color32::from_rgb(60, 180, 60),
                    );
                    bp.text(
                        bar_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("{} {:.0}%", cs.stage_name, total_progress * 100.0),
                        egui::FontId::proportional(7.0),
                        egui::Color32::WHITE,
                    );

                    // Factor bars
                    ui.horizontal(|ui| {
                        let factor_bar =
                            |ui: &mut egui::Ui, label: &str, val: f32, col: egui::Color32| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(label)
                                            .size(8.0)
                                            .color(egui::Color32::GRAY),
                                    );
                                    let (r, _) = ui.allocate_exact_size(
                                        egui::Vec2::new(50.0, 4.0),
                                        egui::Sense::hover(),
                                    );
                                    let p = ui.painter_at(r);
                                    p.rect_filled(r, 1.0, egui::Color32::from_rgb(30, 30, 30));
                                    let fill_col = if val < 0.15 {
                                        egui::Color32::from_rgb(200, 50, 50)
                                    } else {
                                        col
                                    };
                                    p.rect_filled(
                                        egui::Rect::from_min_size(
                                            r.min,
                                            egui::Vec2::new(50.0 * val, 4.0),
                                        ),
                                        1.0,
                                        fill_col,
                                    );
                                });
                            };
                        factor_bar(
                            ui,
                            "Temp",
                            cs.temp_factor,
                            egui::Color32::from_rgb(200, 120, 40),
                        );
                        factor_bar(
                            ui,
                            "Sun",
                            cs.sun_factor,
                            egui::Color32::from_rgb(220, 200, 60),
                        );
                        factor_bar(
                            ui,
                            "Water",
                            cs.water_factor,
                            egui::Color32::from_rgb(60, 140, 220),
                        );
                    });

                    if cs.growth_rate < 0.01 {
                        ui.label(
                            egui::RichText::new(format!("\u{26a0} {}", cs.limiting))
                                .size(9.0)
                                .color(egui::Color32::from_rgb(255, 80, 80)),
                        );
                    }
                }
            }

            // Blueprint detail
            if let Some(bp) = self.blueprints.get(&(item.x, item.y)) {
                ui.separator();
                let status = if bp.resources_met() {
                    if bp.progress > 0.01 {
                        format!("Building {:.0}%", bp.progress * 100.0)
                    } else {
                        "Ready to build".to_string()
                    }
                } else {
                    {
                        let mut parts = Vec::with_capacity(3);
                        if bp.wood_needed > 0 {
                            parts.push(format!("{}/{} wood", bp.wood_delivered, bp.wood_needed));
                        }
                        if bp.plank_needed > 0 {
                            parts.push(format!("{}/{} plank", bp.plank_delivered, bp.plank_needed));
                        }
                        if bp.clay_needed > 0 {
                            parts.push(format!("{}/{} clay", bp.clay_delivered, bp.clay_needed));
                        }
                        format!("Needs: {}", parts.join(", "))
                    }
                };
                ui.label(egui::RichText::new(format!("Blueprint: {}", status)).size(9.0));
            }

            // Pipe/liquid network pressure detail
            let pidx = item.y as u32 * GRID_W + item.x as u32;
            let pbt = item.block_type & 0xFF;
            if pipes::is_gas_pipe_component(pbt)
                && let Some(cell) = self.pipe_network.cells.get(&pidx)
            {
                ui.separator();
                ui.label(
                    egui::RichText::new(format!(
                        "Gas: P={:.2}  Smoke={:.2}  O\u{2082}={:.2}  CO\u{2082}={:.2}  T={:.1}°C",
                        cell.pressure, cell.gas[0], cell.gas[1], cell.gas[2], cell.gas[3]
                    ))
                    .size(9.0),
                );
            }
            if pipes::is_liquid_pipe_component(pbt)
                && let Some(cell) = self.liquid_network.cells.get(&pidx)
            {
                ui.separator();
                ui.label(
                    egui::RichText::new(format!(
                        "Liquid: P={:.2}  T={:.1}°C",
                        cell.pressure, cell.gas[3]
                    ))
                    .size(9.0),
                );
            }
        }
    }

    /// Draw the in-game event log (bottom-right, scrolling).
    /// Draw event notification cards (right side, Rimworld-style).
    fn draw_notifications(&mut self, ctx: &egui::Context) {
        if self.notifications.is_empty() {
            return;
        }
        // Auto-expire after 10 seconds, remove dismissed
        let now = self.time_of_day;
        self.notifications.retain(|n| {
            !n.dismissed && {
                let age = (now - n.time_created).abs();
                age < 10.0 || (now < n.time_created) // handle day wrap
            }
        });

        let mut dismiss_id = None;
        // Stack from top-right, below the layers bar
        let start = self.notifications.len().saturating_sub(8);
        let notifs: Vec<(u32, &'static str, String, String, egui::Color32)> = self.notifications
            [start..]
            .iter()
            .map(|n| {
                (
                    n.id,
                    n.icon,
                    n.title.clone(),
                    n.description.clone(),
                    n.category.color(),
                )
            })
            .collect();
        for (i, (id, icon, title, desc, color)) in notifs.iter().enumerate() {
            let y_offset = 60.0 + i as f32 * 52.0;
            egui::Area::new(egui::Id::new(("notif_card", *id)))
                .anchor(egui::Align2::RIGHT_TOP, [-10.0, y_offset])
                .interactable(true)
                .show(ctx, |ui| {
                    let resp = egui::Frame::NONE
                        .fill(egui::Color32::from_rgba_unmultiplied(
                            color.r(),
                            color.g(),
                            color.b(),
                            210,
                        ))
                        .corner_radius(4.0)
                        .inner_margin(6.0)
                        .show(ui, |ui| {
                            ui.set_max_width(220.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(*icon).size(14.0));
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(title)
                                            .strong()
                                            .size(10.0)
                                            .color(egui::Color32::WHITE),
                                    );
                                    ui.label(
                                        egui::RichText::new(desc)
                                            .size(9.0)
                                            .color(egui::Color32::from_gray(215)),
                                    );
                                });
                            });
                        });
                    if resp.response.secondary_clicked() {
                        dismiss_id = Some(*id);
                    }
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

        // Pleb selection: show needs
        if let Some(pi) = item.pleb_idx {
            if let Some(pleb) = self.plebs.get(pi) {
                egui::Area::new(egui::Id::new("selection_info"))
                    .anchor(egui::Align2::RIGHT_TOP, [-10.0, 130.0])
                    .show(ctx, |ui| {
                        egui::Frame::window(ui.style()).show(ui, |ui| {
                            ui.set_min_width(180.0);
                            ui.label(egui::RichText::new(&pleb.name).strong().size(13.0));
                            let act = pleb.activity.inner();
                            let act_str = match act {
                                PlebActivity::Idle => "Idle",
                                PlebActivity::Walking => "Walking",
                                PlebActivity::Sleeping => "Sleeping",
                                PlebActivity::Harvesting(_) => "Harvesting",
                                PlebActivity::Eating => "Eating",
                                PlebActivity::Hauling => "Hauling",
                                PlebActivity::Farming(_) => "Farming",
                                PlebActivity::Building(_) => "Building",
                                PlebActivity::Crafting(_, _) => "Crafting",
                                PlebActivity::Drinking(_) => "Drinking",
                                PlebActivity::MentalBreak(_, _) => "Mental break",
                                PlebActivity::Staggering(_) => "Staggering",
                                PlebActivity::Crisis(_, _) => "Crisis",
                            };
                            ui.label(egui::RichText::new(act_str).size(10.0).weak());
                            ui.separator();

                            let bar =
                                |ui: &mut egui::Ui, label: &str, val: f32, color: egui::Color32| {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(label).size(10.0).monospace());
                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::Vec2::new(100.0, 8.0),
                                            egui::Sense::hover(),
                                        );
                                        let painter = ui.painter_at(rect);
                                        painter.rect_filled(
                                            rect,
                                            2.0,
                                            egui::Color32::from_rgb(30, 30, 30),
                                        );
                                        painter.rect_filled(
                                            egui::Rect::from_min_size(
                                                rect.min,
                                                egui::Vec2::new(100.0 * val.clamp(0.0, 1.0), 8.0),
                                            ),
                                            2.0,
                                            color,
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{:.0}%", val * 100.0))
                                                .size(9.0)
                                                .weak(),
                                        );
                                    });
                                };
                            bar(
                                ui,
                                "HP  ",
                                pleb.needs.health,
                                egui::Color32::from_rgb(80, 200, 80),
                            );
                            bar(
                                ui,
                                "HUN ",
                                pleb.needs.hunger,
                                egui::Color32::from_rgb(200, 160, 40),
                            );
                            bar(
                                ui,
                                "H2O ",
                                pleb.needs.thirst,
                                egui::Color32::from_rgb(60, 140, 220),
                            );
                            bar(
                                ui,
                                "RST ",
                                pleb.needs.rest,
                                egui::Color32::from_rgb(80, 120, 200),
                            );
                            bar(
                                ui,
                                "WRM ",
                                pleb.needs.warmth,
                                egui::Color32::from_rgb(200, 100, 40),
                            );
                            bar(
                                ui,
                                "O2  ",
                                pleb.needs.oxygen,
                                egui::Color32::from_rgb(100, 200, 220),
                            );

                            ui.separator();
                            if pleb.inventory.is_carrying() {
                                let inv_str: Vec<String> = pleb
                                    .inventory
                                    .stacks
                                    .iter()
                                    .filter(|s| s.count > 0)
                                    .map(|s| s.label())
                                    .collect();
                                ui.label(
                                    egui::RichText::new(inv_str.join(" | ")).size(10.0).weak(),
                                );
                            }
                            if ui.small_button("Inventory (I)").clicked() {
                                self.show_inventory = !self.show_inventory;
                                self.inv_selected_slot = None;
                            }
                        });
                    });
            }
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

    // --- Minimap: world overview (bottom-left corner, above build bar) ---
    fn draw_minimap(&self, ctx: &egui::Context) {
        // Temperature label above minimap
        let ambient_temp = self
            .selected_pleb
            .and_then(|i| self.plebs.get(i))
            .map(|p| p.needs.air_temp)
            .unwrap_or(15.0);
        {
            let temp_text = format!("{:.0}°C", ambient_temp);
            let color = if ambient_temp < 5.0 {
                egui::Color32::from_rgb(100, 160, 255)
            } else if ambient_temp > 40.0 {
                egui::Color32::from_rgb(255, 100, 60)
            } else {
                egui::Color32::from_gray(210)
            };
            let font = egui::FontId::proportional(13.0);
            let screen = ctx.input(|i| i.screen_rect());
            let pos = egui::pos2(screen.max.x - 10.0, screen.max.y - 320.0);
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("temp_label"),
            ));
            Self::shadow_text(
                &painter,
                pos,
                egui::Align2::RIGHT_TOP,
                &temp_text,
                font,
                color,
            );
        }

        let map_size = 120.0;
        let gw = GRID_W as f32;
        let gh = GRID_H as f32;

        egui::Area::new(egui::Id::new("minimap"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -180.0])
            .interactable(false)
            .show(ctx, |ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::Vec2::splat(map_size), egui::Sense::hover());
                let painter = ui.painter_at(rect);

                // Background
                painter.rect_filled(rect, 3.0, egui::Color32::from_rgb(20, 20, 25));
                painter.rect_stroke(
                    rect,
                    3.0,
                    egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
                    egui::StrokeKind::Outside,
                );

                let to_map = |wx: f32, wy: f32| -> egui::Pos2 {
                    egui::pos2(
                        rect.min.x + (wx / gw) * map_size,
                        rect.min.y + (wy / gh) * map_size,
                    )
                };

                // Terrain: sample every 4th tile for performance
                let step = 4u32;
                let px_size = map_size / (gw / step as f32);
                for ty in (0..GRID_H).step_by(step as usize) {
                    for tx in (0..GRID_W).step_by(step as usize) {
                        let idx = (ty * GRID_W + tx) as usize;
                        let block = self.grid_data[idx];
                        let bt = block & 0xFF;
                        let bh = block_height_rs(block) as u32;
                        let elev = if idx < self.elevation_data.len() {
                            self.elevation_data[idx]
                        } else {
                            0.0
                        };

                        let color = if bt == 3 {
                            egui::Color32::from_rgb(30, 60, 120) // water
                        } else if bh > 0 {
                            egui::Color32::from_rgb(100, 95, 90) // wall/structure
                        } else if bt == 8 || bt == 31 {
                            egui::Color32::from_rgb(30, 60, 25) // trees/bushes
                        } else {
                            // Terrain: color by elevation
                            let e = (elev * 12.0) as u8;
                            egui::Color32::from_rgb(35 + e, 40 + e / 2, 20 + e / 3)
                        };

                        let pos = to_map(tx as f32, ty as f32);
                        painter.rect_filled(
                            egui::Rect::from_min_size(pos, egui::Vec2::splat(px_size + 0.5)),
                            0.0,
                            color,
                        );
                    }
                }

                // Plebs as colored dots
                for pleb in &self.plebs {
                    let pos = to_map(pleb.x, pleb.y);
                    let col = if pleb.is_enemy {
                        egui::Color32::from_rgb(255, 50, 50)
                    } else {
                        egui::Color32::from_rgb(50, 220, 50)
                    };
                    painter.circle_filled(pos, 2.0, col);
                }

                // Camera viewport rectangle
                let half_w = self.camera.screen_w * 0.5 / self.camera.zoom;
                let half_h = self.camera.screen_h * 0.5 / self.camera.zoom;
                let vp_min = to_map(self.camera.center_x - half_w, self.camera.center_y - half_h);
                let vp_max = to_map(self.camera.center_x + half_w, self.camera.center_y + half_h);
                painter.rect_stroke(
                    egui::Rect::from_min_max(vp_min, vp_max),
                    1.0,
                    egui::Stroke::new(
                        1.0,
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 150),
                    ),
                    egui::StrokeKind::Outside,
                );
            });
    }

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
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(20, 20, 25, 200))
                    .rounding(6.0)
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
