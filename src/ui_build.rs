//! Build menu, layer controls, and selection actions.

use crate::*;

impl App {
    pub(crate) fn draw_layers_bar(&mut self, ctx: &egui::Context) {
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
                            ui.add(
                                egui::Slider::new(&mut self.water_speed, 0.0..=8.0)
                                    .text("Flow")
                                    .step_by(0.5),
                            );
                            ui.add(
                                egui::Slider::new(
                                    &mut self.camera.water_table_offset,
                                    -10.0..=10.0,
                                )
                                .text("Table")
                                .step_by(0.5),
                            );
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
                                if ui
                                    .selectable_label(*ov == FluidOverlay::Dust, "Dust")
                                    .clicked()
                                {
                                    *ov = if *ov == FluidOverlay::Dust {
                                        FluidOverlay::None
                                    } else {
                                        FluidOverlay::Dust
                                    };
                                }
                            });
                        });
                    });
                });
            });
    }

    pub(crate) fn draw_layer_legend(&mut self, ctx: &egui::Context) {
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

    pub(crate) fn draw_build_bar(&mut self, ctx: &egui::Context) {
        // --- Build categories: organized by player intent, not construction type ---
        // Colors: muted, warm, each category has a distinct accent
        struct CatStyle {
            name: &'static str,
            icon: &'static str,
            accent: egui::Color32,
        }
        let categories = [
            CatStyle {
                name: "Survival",
                icon: "\u{1f525}",
                accent: egui::Color32::from_rgb(195, 130, 55),
            },
            CatStyle {
                name: "Shelter",
                icon: "\u{1f3e0}",
                accent: egui::Color32::from_rgb(140, 105, 65),
            },
            CatStyle {
                name: "Light",
                icon: "\u{1f4a1}",
                accent: egui::Color32::from_rgb(190, 165, 50),
            },
            CatStyle {
                name: "Food",
                icon: "\u{1f33f}",
                accent: egui::Color32::from_rgb(85, 140, 55),
            },
            CatStyle {
                name: "Craft",
                icon: "\u{2692}",
                accent: egui::Color32::from_rgb(105, 120, 140),
            },
            CatStyle {
                name: "Power",
                icon: "\u{26a1}",
                accent: egui::Color32::from_rgb(70, 120, 175),
            },
            CatStyle {
                name: "Pipes",
                icon: "\u{1f4a8}",
                accent: egui::Color32::from_rgb(85, 140, 130),
            },
            CatStyle {
                name: "Zones",
                icon: "\u{1f4cd}",
                accent: egui::Color32::from_rgb(140, 105, 140),
            },
        ];

        egui::Area::new(egui::Id::new("build_categories"))
            .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgba_unmultiplied(25, 28, 32, 230))
                    .corner_radius(6.0)
                    .inner_margin(6)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(50)))
                    .show(ui, |ui| {
                        // Tool buttons (always visible)
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            let tool_btn = |ui: &mut egui::Ui,
                                            tool: BuildTool,
                                            icon: &str,
                                            label: &str,
                                            current: &BuildTool|
                             -> bool {
                                let sel = *current == tool;
                                let bg = if sel {
                                    egui::Color32::from_rgb(160, 60, 50)
                                } else {
                                    egui::Color32::from_rgb(45, 40, 38)
                                };
                                let resp = ui.add(
                                    egui::Button::new(
                                        egui::RichText::new(format!("{} {}", icon, label))
                                            .size(11.0)
                                            .color(egui::Color32::from_gray(210)),
                                    )
                                    .fill(bg)
                                    .corner_radius(3.0)
                                    .min_size(egui::Vec2::new(60.0, 22.0)),
                                );
                                resp.clicked()
                            };
                            if tool_btn(
                                ui,
                                BuildTool::Destroy,
                                "\u{274c}",
                                "Destroy",
                                &self.build_tool,
                            ) {
                                self.build_tool = if self.build_tool == BuildTool::Destroy {
                                    BuildTool::None
                                } else {
                                    BuildTool::Destroy
                                };
                                self.build_category = None;
                            }
                            if tool_btn(ui, BuildTool::DigZone, "\u{26cf}", "Dig", &self.build_tool)
                            {
                                self.build_tool = if self.build_tool == BuildTool::DigZone {
                                    BuildTool::None
                                } else {
                                    BuildTool::DigZone
                                };
                                self.build_category = None;
                            }
                            // Order buttons
                            if tool_btn(
                                ui,
                                BuildTool::OrderChop,
                                "\u{1fa93}",
                                "Chop",
                                &self.build_tool,
                            ) {
                                self.build_tool = if self.build_tool == BuildTool::OrderChop {
                                    BuildTool::None
                                } else {
                                    BuildTool::OrderChop
                                };
                                self.build_category = None;
                            }
                            if tool_btn(
                                ui,
                                BuildTool::OrderMine,
                                "\u{26cf}",
                                "Mine",
                                &self.build_tool,
                            ) {
                                self.build_tool = if self.build_tool == BuildTool::OrderMine {
                                    BuildTool::None
                                } else {
                                    BuildTool::OrderMine
                                };
                                self.build_category = None;
                            }
                            if tool_btn(
                                ui,
                                BuildTool::OrderHarvest,
                                "\u{1f33f}",
                                "Harvest",
                                &self.build_tool,
                            ) {
                                self.build_tool = if self.build_tool == BuildTool::OrderHarvest {
                                    BuildTool::None
                                } else {
                                    BuildTool::OrderHarvest
                                };
                                self.build_category = None;
                            }
                        });

                        ui.add_space(4.0);

                        // Category buttons — single column, colored accent bars
                        for cat in &categories {
                            let selected = self.build_category == Some(cat.name);
                            let bg = if selected {
                                egui::Color32::from_rgb(
                                    (cat.accent.r() as u16 * 60 / 100) as u8 + 20,
                                    (cat.accent.g() as u16 * 60 / 100) as u8 + 20,
                                    (cat.accent.b() as u16 * 60 / 100) as u8 + 20,
                                )
                            } else {
                                egui::Color32::from_rgb(35, 37, 42)
                            };
                            let text_col = if selected {
                                egui::Color32::from_gray(240)
                            } else {
                                egui::Color32::from_gray(170)
                            };

                            let resp = ui.add(
                                egui::Button::new(
                                    egui::RichText::new(format!("{} {}", cat.icon, cat.name))
                                        .size(12.0)
                                        .color(text_col),
                                )
                                .fill(bg)
                                .corner_radius(3.0)
                                .min_size(egui::Vec2::new(110.0, 24.0)),
                            );

                            // Colored accent bar on left edge when selected
                            if selected {
                                let bar = egui::Rect::from_min_size(
                                    resp.rect.left_top(),
                                    egui::Vec2::new(3.0, resp.rect.height()),
                                );
                                ui.painter().rect_filled(bar, 1.0, cat.accent);
                            }

                            if resp.clicked() {
                                if selected {
                                    self.build_category = None;
                                    self.build_tool = BuildTool::None;
                                    self.sandbox_tool = SandboxTool::None;
                                    self.terrain_tool = None;
                                } else {
                                    self.build_category = Some(cat.name);
                                    self.world_sel = WorldSelection::none();
                                    self.selected_pleb = None;
                                    self.selected_group.clear();
                                    self.terrain_tool = None;
                                    self.sandbox_tool = SandboxTool::None;
                                }
                            }
                        }
                    });
            });

        // --- Build items / Selection actions panel (center bottom, single column) ---
        // Shows build tools when a category is active, or selection actions when items are selected.
        // These are mutually exclusive: selecting something closes build menu.
        // Skip selection actions panel when only plebs are selected (action bar handles that)
        let has_pleb_only_sel =
            !self.world_sel.is_empty() && self.world_sel.items.iter().all(|i| i.pleb_idx.is_some());
        let has_selection = !self.world_sel.is_empty() && !has_pleb_only_sel;
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

                            let item_count: usize = match cat {
                                "Survival" => 8,
                                "Shelter" => 12,
                                "Light" => 6,
                                "Food" => 5,
                                "Craft" => 6,
                                "Power" => 10,
                                "Pipes" => 12,
                                "Zones" => 4,
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
                                    let end_row_check = |ui: &mut egui::Ui| {
                                        let c = col_counter.get() + 1;
                                        col_counter.set(c);
                                        if c.is_multiple_of(items_per_row) {
                                            ui.end_row();
                                        }
                                    };
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
                                            end_row_check(ui);
                                        };
                                    // Locked/grayed-out build item — visible but not clickable
                                    let mut locked_btn =
                                        |ui: &mut egui::Ui,
                                         icon: &str,
                                         label: &str,
                                         needs: &str| {
                                            let (rect, response) = ui.allocate_exact_size(
                                                egui::Vec2::splat(tile_size),
                                                egui::Sense::hover(),
                                            );
                                            let painter = ui.painter_at(rect);
                                            painter.rect_filled(
                                                rect,
                                                4.0,
                                                egui::Color32::from_rgb(32, 33, 36),
                                            );
                                            painter.rect_stroke(
                                                rect,
                                                4.0,
                                                egui::Stroke::new(
                                                    1.0,
                                                    egui::Color32::from_gray(45),
                                                ),
                                                egui::StrokeKind::Outside,
                                            );
                                            painter.text(
                                                rect.center() + egui::Vec2::new(0.0, -6.0),
                                                egui::Align2::CENTER_CENTER,
                                                icon,
                                                egui::FontId::proportional(icon_s),
                                                egui::Color32::from_gray(70),
                                            );
                                            painter.text(
                                                rect.center() + egui::Vec2::new(0.0, 14.0),
                                                egui::Align2::CENTER_CENTER,
                                                label,
                                                egui::FontId::proportional(label_s),
                                                egui::Color32::from_gray(65),
                                            );
                                            if response.hovered() {
                                                response.on_hover_text(needs);
                                            }
                                            end_row_check(ui);
                                        };
                                    match cat {
                                        "Survival" => {
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(62),
                                                "\u{1f525}",
                                                "Campfire",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(70),
                                                "\u{1f4a8}",
                                                "Charcoal",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(63),
                                                "\u{1f9f1}",
                                                "Low Fence",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(60),
                                                "\u{2b1c}",
                                                "Rough Floor",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(64),
                                                "\u{1fa9c}",
                                                "Snare",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(13),
                                                "\u{267b}",
                                                "Compost",
                                            );
                                            locked_btn(
                                                ui,
                                                "\u{1f9f1}",
                                                "Palisade",
                                                "Needs: logs, rope",
                                            );
                                            locked_btn(
                                                ui,
                                                "\u{1fa64}",
                                                "Smokehouse",
                                                "Needs: planks, clay",
                                            );
                                        }
                                        "Shelter" => {
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(35),
                                                "\u{1f3da}",
                                                "Wattle",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(21),
                                                "\u{1fab5}",
                                                "Wood Wall",
                                            );
                                            icon_btn(ui, BuildTool::Place(1), "\u{1f9f1}", "Stone");
                                            icon_btn(ui, BuildTool::Door, "\u{1f6aa}", "Door");
                                            icon_btn(
                                                ui,
                                                BuildTool::WindowOpening,
                                                "\u{25a1}",
                                                "Window",
                                            );
                                            icon_btn(ui, BuildTool::Window, "\u{1fa9f}", "Glass");
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(26),
                                                "\u{1fab5}",
                                                "Wood Floor",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(27),
                                                "\u{2b1b}",
                                                "Stone Floor",
                                            );
                                            icon_btn(ui, BuildTool::Roof, "\u{1f3e0}", "Roof");
                                            icon_btn(ui, BuildTool::Place(30), "\u{1f6cf}", "Bed");
                                            icon_btn(ui, BuildTool::Place(9), "\u{1fa91}", "Bench");
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(74),
                                                "\u{1fa91}",
                                                "Stool",
                                            );
                                        }
                                        "Light" => {
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(55),
                                                "\u{1f525}",
                                                "Torch",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(6),
                                                "\u{1f525}",
                                                "Fireplace",
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
                                        "Food" => {
                                            icon_btn(ui, BuildTool::Place(59), "\u{1fa63}", "Well");
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(75),
                                                "\u{1f4cf}",
                                                "Dry Rack",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(33),
                                                "\u{1f4e6}",
                                                "Crate",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::GrowingZone,
                                                "\u{1f33f}",
                                                "Farm Zone",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::StorageZone,
                                                "\u{1f4e6}",
                                                "Storage",
                                            );
                                        }
                                        "Craft" => {
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(73),
                                                "\u{1fa91}",
                                                "Table",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(57),
                                                "\u{1f528}",
                                                "Workbench",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(61),
                                                "\u{1fa9a}",
                                                "Saw Horse",
                                            );
                                            icon_btn(ui, BuildTool::Place(58), "\u{1f3ed}", "Kiln");
                                            locked_btn(
                                                ui,
                                                "\u{2692}",
                                                "Forge",
                                                "Needs: clay, iron ore",
                                            );
                                            locked_btn(
                                                ui,
                                                "\u{1f9f5}",
                                                "Loom",
                                                "Needs: planks, fiber",
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
                                        "Pipes" => {
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(15),
                                                "\u{1f4a8}",
                                                "Gas Pipe",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(16),
                                                "\u{2699}",
                                                "Gas Pump",
                                            );
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
                                                BuildTool::Place(46),
                                                "\u{2298}",
                                                "Restrictor",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(49),
                                                "\u{1f4a7}",
                                                "Liq Pipe",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(52),
                                                "\u{1f6b0}",
                                                "Intake",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(53),
                                                "\u{2699}",
                                                "Liq Pump",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(54),
                                                "\u{1f4a6}",
                                                "Output",
                                            );
                                        }
                                        "Zones" => {
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(29),
                                                "\u{1f4a5}",
                                                "Cannon",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::RemoveFloor,
                                                "\u{274c}",
                                                "Rm Floor",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::RemoveRoof,
                                                "\u{274c}",
                                                "Rm Roof",
                                            );
                                            icon_btn(
                                                ui,
                                                BuildTool::Place(50),
                                                "\u{2a2f}",
                                                "Bridge",
                                            );
                                            // Terrain tools
                                            let bz_sel = self.build_tool == BuildTool::BermZone;
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        egui::RichText::new("\u{26f0} Berm")
                                                            .size(8.0),
                                                    )
                                                    .selected(bz_sel),
                                                )
                                                .clicked()
                                            {
                                                self.build_tool = if bz_sel {
                                                    BuildTool::None
                                                } else {
                                                    BuildTool::BermZone
                                                };
                                                self.terrain_tool = None;
                                            }
                                            // Water tools
                                            let wf_sel = self.build_tool == BuildTool::WaterFill;
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        egui::RichText::new("\u{1f4a7} Fill")
                                                            .size(8.0),
                                                    )
                                                    .selected(wf_sel),
                                                )
                                                .clicked()
                                            {
                                                self.build_tool = if wf_sel {
                                                    BuildTool::None
                                                } else {
                                                    BuildTool::WaterFill
                                                };
                                                self.terrain_tool = None;
                                            }
                                            if wf_sel {
                                                ui.add(
                                                    egui::Slider::new(
                                                        &mut self.water_speed,
                                                        0.0..=8.0,
                                                    )
                                                    .text("Flow")
                                                    .step_by(0.5),
                                                );
                                            }
                                            let dig_sel = self.build_tool == BuildTool::Dig;
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        egui::RichText::new("\u{26cf} Drain")
                                                            .size(8.0),
                                                    )
                                                    .selected(dig_sel),
                                                )
                                                .clicked()
                                            {
                                                self.build_tool = if dig_sel {
                                                    BuildTool::None
                                                } else {
                                                    BuildTool::Dig
                                                };
                                                self.terrain_tool = None;
                                            }
                                        }
                                        _ => {}
                                    }
                                });
                            // Zone work priority toggle (outside grid)
                            if self.build_category == Some("Food") {
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
                                    BuildTool::Window
                                    | BuildTool::Door
                                    | BuildTool::WindowOpening => "Click wall".to_string(),
                                    BuildTool::Roof => "Drag (needs support)".to_string(),
                                    BuildTool::Dig => "Sandbox: instant dig".to_string(),
                                    BuildTool::DigZone => "Drag to mark dig area".to_string(),
                                    BuildTool::BermZone => "Drag to mark berm area".to_string(),
                                    BuildTool::WaterFill => "Hold to fill with water".to_string(),
                                    _ => "Click/drag".to_string(),
                                };
                                ui.label(egui::RichText::new(hint).weak().size(13.0));
                            }
                        } // end else (build tools)
                    }); // Frame
                }); // Area
        }
    }

    pub(crate) fn draw_selection_actions_inner(&mut self, ui: &mut egui::Ui) {
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
}
