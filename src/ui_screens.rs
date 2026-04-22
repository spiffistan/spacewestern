//! Game state screens — main menu, crash card, map generation, character creation, manifest.

use crate::*;

impl App {
    pub(crate) fn draw_main_menu(&mut self, ctx: &egui::Context) {
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

    pub(crate) fn draw_crash_card(&mut self, ctx: &egui::Context) {
        use crate::theme::palette;

        let screen = ctx.content_rect();

        // Full-screen overlay — darkens the world behind the card
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("crash_card_overlay"),
        ));
        painter.rect_filled(
            screen,
            0.0,
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 160),
        );

        // The card itself — styled like worn paper, rendered above the overlay
        let card_width = 420.0_f32.min(screen.width() - 40.0);
        egui::Area::new(egui::Id::new("crash_card"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .interactable(false)
            .order(egui::Order::Tooltip)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(palette::PARCHMENT_DARK)
                    .inner_margin(egui::Margin::same(28))
                    .stroke(egui::Stroke::new(1.0, palette::INK_GHOST))
                    .corner_radius(2.0)
                    .show(ui, |ui| {
                        ui.set_max_width(card_width);

                        // Ship name / title
                        ui.label(
                            egui::RichText::new("INCIDENT REPORT")
                                .size(10.0)
                                .color(palette::INK_FAINT)
                                .strong(),
                        );
                        ui.add_space(12.0);

                        // Main narrative text
                        ui.label(
                            egui::RichText::new("The Perdition broke apart at four thousand feet.")
                                .size(16.0)
                                .color(palette::INK)
                                .italics(),
                        );
                        ui.add_space(16.0);

                        let crew_count = self.plebs.len();
                        let body = format!(
                            "{} survivors. A field of wreckage.\n\
                             The forest is close. Dusk is closer.",
                            crew_count
                        );
                        ui.label(
                            egui::RichText::new(body)
                                .size(13.0)
                                .color(palette::INK_DIM)
                                .line_height(Some(20.0)),
                        );

                        ui.add_space(24.0);

                        // Crew roster
                        ui.label(
                            egui::RichText::new("SURVIVORS")
                                .size(9.0)
                                .color(palette::INK_FAINT)
                                .strong(),
                        );
                        ui.add_space(4.0);
                        for p in &self.plebs {
                            let role = if p.backstory_name.is_empty() {
                                String::new()
                            } else {
                                format!(" \u{2014} {}", p.backstory_name)
                            };
                            ui.label(
                                egui::RichText::new(format!("{}{}", p.name, role))
                                    .size(11.0)
                                    .color(palette::INK_DIM),
                            );
                        }

                        ui.add_space(24.0);

                        // Dismiss hint
                        ui.label(
                            egui::RichText::new("click anywhere to continue")
                                .size(10.0)
                                .color(palette::INK_GHOST)
                                .italics(),
                        );
                    });
            });

        // Dismiss handled via winit: mouse click in window_event, ESC in handle_keyboard
    }

    /// Play a click sound when a button is first hovered. Returns true if clicked.
    pub(crate) fn hover_click_button(&mut self, ui: &mut egui::Ui, text: egui::RichText) -> bool {
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

    pub(crate) fn draw_map_gen_screen(&mut self, ctx: &egui::Context) {
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
                        ui.label(egui::RichText::new("Landscape").strong().size(15.0));
                        ui.add_space(4.0);

                        let landscape_slider = |ui: &mut egui::Ui, label: &str, val: &mut f32| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(label).size(13.0).monospace());
                                ui.spacing_mut().slider_width = 140.0;
                                ui.add(egui::Slider::new(val, 0.0..=1.0).show_value(false));
                            });
                        };

                        landscape_slider(ui, "Hills  ", &mut self.terrain_params.hilliness);
                        landscape_slider(ui, "Water  ", &mut self.terrain_params.water_table);
                        landscape_slider(ui, "Trees  ", &mut self.terrain_params.tree_density);
                        landscape_slider(ui, "Ponds  ", &mut self.terrain_params.pond_density);
                        landscape_slider(ui, "Grass  ", &mut self.terrain_params.grass_density);
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
                        // Auto-regenerate when any param changes
                        if self.terrain_params != self.terrain_params_prev {
                            self.terrain_params_prev = self.terrain_params.clone();
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
                        let colors: [egui::Color32; 13] = [
                            egui::Color32::from_rgb(107, 92, 56),   // 0: grass
                            egui::Color32::from_rgb(173, 168, 153), // 1: chalky
                            egui::Color32::from_rgb(115, 107, 97),  // 2: rocky
                            egui::Color32::from_rgb(128, 97, 64),   // 3: clay
                            egui::Color32::from_rgb(122, 117, 107), // 4: gravel
                            egui::Color32::from_rgb(56, 46, 31),    // 5: peat
                            egui::Color32::from_rgb(77, 89, 56),    // 6: marsh
                            egui::Color32::from_rgb(97, 77, 46),    // 7: loam
                            egui::Color32::from_rgb(122, 76, 50),   // 8: iron-stained
                            egui::Color32::from_rgb(97, 115, 102),  // 9: copper-stained
                            egui::Color32::from_rgb(153, 148, 132), // 10: flint-bearing
                            egui::Color32::from_rgb(64, 51, 30),    // 11: leaf litter
                            egui::Color32::from_rgb(184, 166, 115), // 12: sand
                        ];
                        let step = 4u32;
                        let px_size = scale * step as f32;
                        for ty in (0..GRID_H).step_by(step as usize) {
                            for tx in (0..GRID_W).step_by(step as usize) {
                                let idx = (ty * GRID_W + tx) as usize;
                                if idx >= self.terrain_data.len() {
                                    continue;
                                }

                                let tt = (self.terrain_data[idx] & 0xF) as usize;
                                let bt = if idx < self.grid_data.len() {
                                    self.grid_data[idx] & 0xFF
                                } else {
                                    0
                                };

                                // Elevation shading
                                let elev = if idx < self.elevation_data.len() {
                                    self.elevation_data[idx]
                                } else {
                                    0.0
                                };
                                let e = (elev * 10.0).clamp(0.0, 30.0) as u8;

                                // Water: use pre-computed equilibrium depth (includes pooling)
                                let eq_depth = if idx < self.water_equilibrium.len() {
                                    self.water_equilibrium[idx]
                                } else {
                                    0.0
                                };
                                let has_water = eq_depth > 0.01 || bt == 3;
                                let seep = if idx < self.water_table.len() {
                                    self.water_table[idx] - elev
                                } else {
                                    -5.0
                                };
                                let is_damp = seep > -0.5 && !has_water;

                                let color = if has_water {
                                    let depth = eq_depth.clamp(0.0, 2.0);
                                    let d = (depth * 40.0) as u8;
                                    egui::Color32::from_rgb(
                                        20_u8.saturating_sub(d),
                                        45_u8.saturating_sub(d),
                                        100 + d.min(50),
                                    )
                                } else if bt == 8 {
                                    // BT_TREE
                                    egui::Color32::from_rgb(25 + e / 3, 55 + e / 2, 22 + e / 3)
                                } else if tt < 13 {
                                    let base = colors[tt];
                                    let mut r = base.r().saturating_add(e / 2);
                                    let mut g = base.g().saturating_add(e / 3);
                                    let mut b = base.b().saturating_add(e / 4);
                                    // Damp ground: darken + blue-shift where water table is close
                                    if is_damp {
                                        let damp_t = ((seep + 0.5) / 0.5).clamp(0.0, 1.0) as f32;
                                        r = (r as f32 * (1.0 - damp_t * 0.2)) as u8;
                                        g = (g as f32 * (1.0 - damp_t * 0.1)) as u8;
                                        b = (b as f32 * (1.0 + damp_t * 0.1)).min(255.0) as u8;
                                    }
                                    egui::Color32::from_rgb(r, g, b)
                                } else {
                                    egui::Color32::from_rgb(35 + e, 40 + e / 2, 20 + e / 3)
                                };

                                let px = rect.min.x + tx as f32 * scale;
                                let py = rect.min.y + ty as f32 * scale;
                                painter.rect_filled(
                                    egui::Rect::from_min_size(
                                        egui::pos2(px, py),
                                        egui::vec2(px_size, px_size),
                                    ),
                                    0.0,
                                    color,
                                );
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

    pub(crate) fn draw_chargen_screen(&mut self, ctx: &egui::Context) {
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
        self.chargen.preview_angle += ctx.input(|i| i.stable_dt) * 0.5;

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
                        let body_scale = match self.chargen.body_type {
                            BodyType::Thin => 0.32,
                            BodyType::Medium => 0.38,
                            BodyType::Stocky => 0.44,
                        };
                        let s = preview_size * body_scale;
                        let dir_x = self.chargen.preview_angle.cos() * 0.03 * s;

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
                            to_dark(self.chargen.pants),
                        );
                        ell(
                            &painter,
                            egui::pos2(cx, cy + s * 0.14),
                            s * 0.22,
                            s * 0.15,
                            to_col(self.chargen.pants),
                        );
                        ell(
                            &painter,
                            egui::pos2(cx + dir_x * 0.3, cy - s * 0.08),
                            s * 0.26,
                            s * 0.20,
                            to_col(self.chargen.shirt),
                        );
                        painter.circle_filled(
                            egui::pos2(cx + dir_x, cy - s * 0.32),
                            s * 0.16,
                            to_col(self.chargen.skin),
                        );
                        let hair_r = match self.chargen.hair_style {
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
                            to_col(self.chargen.hair),
                        );

                        painter.text(
                            egui::pos2(cx, cy + s * 0.50),
                            egui::Align2::CENTER_TOP,
                            &self.chargen.name,
                            egui::FontId::proportional(16.0),
                            egui::Color32::from_gray(220),
                        );
                        painter.text(
                            egui::pos2(cx, cy + s * 0.50 + 20.0),
                            egui::Align2::CENTER_TOP,
                            self.chargen.backstory.name(),
                            egui::FontId::proportional(12.0),
                            egui::Color32::from_gray(140),
                        );

                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(self.chargen.backstory.description())
                                .size(11.0)
                                .italics()
                                .color(egui::Color32::from_gray(160)),
                        );
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "Ability: {}",
                                self.chargen.backstory.ability()
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
                                egui::TextEdit::singleline(&mut self.chargen.name),
                            );
                            if ui.small_button("Random").clicked() {
                                self.chargen.name = random_name(self.frame_count);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Gender:");
                            if ui
                                .selectable_label(self.chargen.gender == Gender::Male, "Male")
                                .clicked()
                            {
                                self.chargen.gender = Gender::Male;
                            }
                            if ui
                                .selectable_label(self.chargen.gender == Gender::Female, "Female")
                                .clicked()
                            {
                                self.chargen.gender = Gender::Female;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Age:");
                            ui.add(egui::Slider::new(&mut self.chargen.age, 18..=65));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Build:");
                            for bt in &[BodyType::Thin, BodyType::Medium, BodyType::Stocky] {
                                if ui
                                    .selectable_label(self.chargen.body_type == *bt, bt.label())
                                    .clicked()
                                {
                                    self.chargen.body_type = *bt;
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
                                        .selectable_label(self.chargen.backstory == bs, bs.name());
                                    if resp.clicked() {
                                        self.chargen.backstory = bs;
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
                                let selected = (self.chargen.skin[0] - tone[0]).abs() < 0.02
                                    && (self.chargen.skin[1] - tone[1]).abs() < 0.02;
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
                                    self.chargen.skin = tone;
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
                                let selected = (self.chargen.hair[0] - tone[0]).abs() < 0.02
                                    && (self.chargen.hair[1] - tone[1]).abs() < 0.02;
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
                                    self.chargen.hair = tone;
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Shirt:");
                            ui.color_edit_button_rgb(&mut self.chargen.shirt);
                            ui.label("Pants:");
                            ui.color_edit_button_rgb(&mut self.chargen.pants);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Hair style:");
                            for (i, name) in ["Short", "Medium", "Long", "Bald"].iter().enumerate()
                            {
                                if ui
                                    .selectable_label(self.chargen.hair_style == i as u8, *name)
                                    .clicked()
                                {
                                    self.chargen.hair_style = i as u8;
                                }
                            }
                        });

                        ui.add_space(6.0);
                        ui.separator();
                        ui.label(egui::RichText::new("Trait").strong().size(14.0));
                        ui.add_space(2.0);
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .selectable_label(self.chargen.trait_pick.is_none(), "None")
                                .clicked()
                            {
                                self.chargen.trait_pick = None;
                            }
                            for &t in PlebTrait::ALL {
                                let resp = ui
                                    .selectable_label(self.chargen.trait_pick == Some(t), t.name());
                                if resp.clicked() {
                                    self.chargen.trait_pick = Some(t);
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
                        let chargen_skills = self.chargen.backstory.skills();
                        let names = Backstory::skill_names();
                        for (i, &name) in names.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("{:12}", name))
                                        .monospace()
                                        .size(11.0),
                                );
                                let val = chargen_skills[i] as f32;
                                let bar_w = 100.0;
                                let (bar_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(bar_w, 10.0),
                                    egui::Sense::hover(),
                                );
                                let fill_w = bar_w * val / 10.0;
                                let bar_col = if val >= 7.0 {
                                    egui::Color32::from_rgb(80, 180, 80)
                                } else if val >= 4.0 {
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
                                    egui::RichText::new(format!("{:.0}", val))
                                        .monospace()
                                        .size(11.0),
                                );
                            });
                        }

                        ui.add_space(8.0);
                        if ui.button("Randomize All").clicked() {
                            let seed = self.frame_count;
                            self.chargen.name = random_name(seed);
                            let h = |s: u32, off: u32| -> f32 {
                                ((s.wrapping_add(off).wrapping_mul(2654435761)) & 0xFFFF) as f32
                                    / 65535.0
                            };
                            self.chargen.skin =
                                SKIN_PALETTE[(h(seed, 1) * SKIN_PALETTE.len() as f32) as usize
                                    % SKIN_PALETTE.len()];
                            self.chargen.hair =
                                HAIR_PALETTE[(h(seed, 4) * HAIR_PALETTE.len() as f32) as usize
                                    % HAIR_PALETTE.len()];
                            self.chargen.hair_style = (h(seed, 7) * 4.0) as u8;
                            self.chargen.shirt = [
                                0.15 + h(seed, 8) * 0.6,
                                0.15 + h(seed, 9) * 0.5,
                                0.15 + h(seed, 10) * 0.5,
                            ];
                            self.chargen.pants = [
                                0.15 + h(seed, 11) * 0.4,
                                0.15 + h(seed, 12) * 0.35,
                                0.10 + h(seed, 13) * 0.3,
                            ];
                            self.chargen.backstory = Backstory::ALL
                                [(h(seed, 14) * 10.0) as usize % Backstory::ALL.len()];
                            self.chargen.body_type =
                                [BodyType::Thin, BodyType::Medium, BodyType::Stocky]
                                    [(h(seed, 15) * 3.0) as usize % 3];
                            self.chargen.gender = if h(seed, 16) > 0.5 {
                                Gender::Female
                            } else {
                                Gender::Male
                            };
                            self.chargen.age = 20 + (h(seed, 17) * 40.0) as u8;
                            let trait_roll =
                                (h(seed, 18) * (PlebTrait::ALL.len() + 2) as f32) as usize;
                            self.chargen.trait_pick = PlebTrait::ALL.get(trait_roll).copied();
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
            pleb.name = self.chargen.name.clone();
            pleb.appearance.skin_r = self.chargen.skin[0];
            pleb.appearance.skin_g = self.chargen.skin[1];
            pleb.appearance.skin_b = self.chargen.skin[2];
            pleb.appearance.hair_r = self.chargen.hair[0];
            pleb.appearance.hair_g = self.chargen.hair[1];
            pleb.appearance.hair_b = self.chargen.hair[2];
            pleb.appearance.hair_style = self.chargen.hair_style as u32;
            pleb.appearance.shirt_r = self.chargen.shirt[0];
            pleb.appearance.shirt_g = self.chargen.shirt[1];
            pleb.appearance.shirt_b = self.chargen.shirt[2];
            pleb.appearance.pants_r = self.chargen.pants[0];
            pleb.appearance.pants_g = self.chargen.pants[1];
            pleb.appearance.pants_b = self.chargen.pants[2];
            pleb.backstory_name = self.chargen.backstory.name().to_string();
            pleb.trait_name = self.chargen.trait_pick.map(|t| t.name().to_string());
            pleb.set_skills_from_legacy(self.chargen.backstory.skills());

            // Randomize other crew members with backstories and traits
            let backstories = types::Backstory::ALL;
            let traits = types::PlebTrait::ALL;
            for ci in 1..self.plebs.len() {
                let seed = (ci as u32 * 7919 + 42).wrapping_mul(2654435761);
                let bs_idx = (seed as usize) % backstories.len();
                let bs = backstories[bs_idx];
                self.plebs[ci].backstory_name = bs.name().to_string();
                self.plebs[ci].set_skills_from_legacy(bs.skills());
                // 70% chance to have a trait
                let trait_roll = ((seed >> 8) & 0xFF) as f32 / 255.0;
                if trait_roll < 0.7 {
                    let t_idx = ((seed >> 16) as usize) % traits.len();
                    self.plebs[ci].trait_name = Some(traits[t_idx].name().to_string());
                }
            }

            self.build_landing_pod();
            self.game_state = GameState::Playing;
            self.show_crash_card = true;
            self.crash_card_frame = self.frame_count;
            self.time_paused = true;
        }
    }

    pub(crate) fn draw_manifest_screen(&mut self, ctx: &egui::Context) {
        use crate::cards;
        use crate::theme::{palette, spacing, text};

        // Dark background
        egui::Area::new(egui::Id::new("manifest_bg"))
            .anchor(egui::Align2::LEFT_TOP, [0.0, 0.0])
            .order(egui::Order::Background)
            .interactable(false)
            .show(ctx, |ui| {
                let screen = ctx.content_rect();
                ui.allocate_exact_size(screen.size(), egui::Sense::hover());
                ui.painter()
                    .rect_filled(screen, 0.0, egui::Color32::from_rgb(20, 18, 15));
            });

        let crew_full = self.manifest.recruited.len() >= 3;
        let mut recruit_idx: Option<usize> = None;
        let mut do_pass = false;
        let mut do_land = false;

        let cw = 150.0;
        egui::Window::new("THE MANIFEST")
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("THE MANIFEST")
                            .size(22.0)
                            .strong()
                            .color(palette::AMBER),
                    );
                    ui.label(
                        egui::RichText::new("Choose your crew for the frontier")
                            .size(text::BODY)
                            .color(palette::INK_FAINT),
                    );
                });
                ui.add_space(spacing::LG);

                ui.columns(4, |cols| {
                    // --- Applicant cards (columns 0-2) ---
                    if !crew_full {
                        let item_reg = item_defs::ItemRegistry::cached();
                        for i in 0..self.manifest.applicants.len().min(3) {
                            let card = &self.manifest.applicants[i];
                            let ui = &mut cols[i];
                            let parch = egui::Color32::from_rgb(240, 232, 215);
                            let border_c = egui::Color32::from_rgb(170, 140, 80);
                            let ink = egui::Color32::from_rgb(40, 35, 25);
                            let ink_dim = egui::Color32::from_rgb(100, 90, 70);
                            let ink_faint = egui::Color32::from_rgb(140, 125, 100);

                            egui::Frame::NONE
                                .fill(parch)
                                .stroke(egui::Stroke::new(1.5, border_c))
                                .corner_radius(3.0)
                                .inner_margin(8)
                                .show(ui, |ui| {
                                    // Name
                                    ui.label(
                                        egui::RichText::new(&card.name)
                                            .size(13.0)
                                            .strong()
                                            .color(ink),
                                    );
                                    ui.label(
                                        egui::RichText::new(card.backstory.name())
                                            .size(9.0)
                                            .color(ink_dim),
                                    );
                                    ui.add_space(4.0);

                                    // Skills
                                    ui.label(
                                        egui::RichText::new("SKILLS")
                                            .size(8.0)
                                            .strong()
                                            .color(ink_faint),
                                    );
                                    for si in 0..6 {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(pleb::SKILL_SHORT[si])
                                                    .size(8.0)
                                                    .monospace()
                                                    .color(ink_dim),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.1}",
                                                    card.skills[si].value
                                                ))
                                                .size(8.0)
                                                .color(ink_faint),
                                            );
                                        });
                                    }
                                    ui.add_space(4.0);

                                    // Traits
                                    if let Some(ref t) = card.trait_visible {
                                        ui.label(
                                            egui::RichText::new(t.name())
                                                .size(9.0)
                                                .color(egui::Color32::from_rgb(55, 155, 45)),
                                        );
                                    }
                                    ui.label(
                                        egui::RichText::new("??? Hidden")
                                            .size(9.0)
                                            .color(egui::Color32::from_gray(100)),
                                    );
                                    ui.add_space(4.0);

                                    // Gear
                                    ui.horizontal_wrapped(|ui| {
                                        for &item_id in &card.gear_belt {
                                            if let Some(def) = item_reg.get(item_id) {
                                                ui.label(egui::RichText::new(&def.icon).size(12.0))
                                                    .on_hover_text(&def.name);
                                            }
                                        }
                                        for &(item_id, count) in &card.gear_inv {
                                            if let Some(def) = item_reg.get(item_id) {
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "{}×{}",
                                                        def.icon, count
                                                    ))
                                                    .size(9.0)
                                                    .color(ink_dim),
                                                );
                                            }
                                        }
                                    });

                                    // Quote
                                    ui.label(
                                        egui::RichText::new(format!("\"{}\"", card.quote))
                                            .size(8.0)
                                            .italics()
                                            .color(ink_faint),
                                    );
                                    ui.add_space(6.0);

                                    // Recruit button
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new("RECRUIT")
                                                    .size(10.0)
                                                    .strong()
                                                    .color(egui::Color32::from_rgb(40, 35, 20)),
                                            )
                                            .fill(egui::Color32::from_rgb(170, 145, 85))
                                            .corner_radius(2.0),
                                        )
                                        .clicked()
                                    {
                                        recruit_idx = Some(i);
                                    }
                                });
                        }
                    } else {
                        cols[0].vertical_centered(|ui| {
                            ui.add_space(60.0);
                            ui.label(
                                egui::RichText::new("Crew assembled.")
                                    .size(16.0)
                                    .color(palette::AMBER),
                            );
                        });
                    }

                    // --- Right column: recruited crew + actions ---
                    let ui = &mut cols[3];
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("CREW")
                                .size(text::HEADING)
                                .strong()
                                .color(palette::AMBER),
                        );
                        ui.add_space(spacing::SM);

                        for (i, card) in self.manifest.recruited.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("{}.", i + 1))
                                        .size(text::BODY)
                                        .color(palette::INK_FAINT),
                                );
                                ui.label(
                                    egui::RichText::new(&card.name)
                                        .size(text::BODY)
                                        .strong()
                                        .color(palette::WHITE),
                                );
                                ui.label(
                                    egui::RichText::new(card.backstory.name())
                                        .size(text::SMALL)
                                        .color(palette::INK_FAINT),
                                );
                            });
                        }
                        for i in self.manifest.recruited.len()..3 {
                            ui.label(
                                egui::RichText::new(format!("{}. ───", i + 1))
                                    .size(text::BODY)
                                    .color(egui::Color32::from_gray(50)),
                            );
                        }

                        ui.add_space(spacing::XL);

                        // Pass button
                        if !crew_full {
                            let pass_label = format!("PASS ({})", self.manifest.passes);
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new(pass_label)
                                            .size(text::BODY)
                                            .color(palette::INK),
                                    )
                                    .fill(palette::PARCHMENT_DARK)
                                    .corner_radius(2.0),
                                )
                                .clicked()
                            {
                                do_pass = true;
                            }
                        }

                        ui.add_space(spacing::MD);

                        // Land button (only when crew full)
                        if crew_full {
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("LAND")
                                            .size(14.0)
                                            .strong()
                                            .color(palette::INK),
                                    )
                                    .fill(palette::AMBER)
                                    .corner_radius(3.0)
                                    .min_size(egui::Vec2::new(120.0, 32.0)),
                                )
                                .clicked()
                            {
                                do_land = true;
                            }
                        }
                    });
                });
            });

        // --- Handle actions ---
        if let Some(idx) = recruit_idx {
            if idx < self.manifest.applicants.len() && self.manifest.recruited.len() < 3 {
                let card = self.manifest.applicants.remove(idx);
                self.manifest.recruited.push(card);
                // If not full yet, deal new cards
                if self.manifest.recruited.len() < 3 {
                    self.manifest.applicants.clear();
                    self.manifest.seed += 100;
                    for i in 0..3 {
                        self.manifest
                            .applicants
                            .push(ManifestCard::generate(self.manifest.seed + i));
                    }
                }
            }
        }
        if do_pass {
            self.manifest.passes += 1;
            self.manifest.applicants.clear();
            self.manifest.seed += 100;
            for i in 0..3 {
                self.manifest
                    .applicants
                    .push(ManifestCard::generate(self.manifest.seed + i));
            }
        }
        if do_land {
            // Create plebs from recruited crew (positions set by build_landing_pod)
            self.plebs.clear();
            for (i, card) in self.manifest.recruited.iter().enumerate() {
                let mut p = card.to_pleb(i, 0.0, 0.0);
                if i == 0 {
                    p.is_leader = true;
                }
                self.plebs.push(p);
            }
            self.next_pleb_id = self.plebs.len();
            self.selected_pleb = Some(0);
            self.build_landing_pod();
            self.game_state = GameState::Playing;
            self.show_crash_card = true;
            self.crash_card_frame = self.frame_count;
            self.time_paused = true;
        }
    }
}
