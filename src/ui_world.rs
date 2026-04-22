//! World rendering — overlays, labels, minimap.
//! The heaviest UI methods, drawing per-tile world information.

use crate::*;

impl App {
    pub(crate) fn draw_world_overlays(
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
            let _screen_rect = ctx.content_rect();
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
                // Plebs have the ring from the shader — skip the box
                if item.pleb_idx.is_some() {
                    continue;
                }
                let (wx0, wy0, wx1, wy1) = {
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
                    zones::ZoneKind::Dig => {
                        egui::Color32::from_rgba_unmultiplied(140, 100, 50, 45) // brown
                    }
                    zones::ZoneKind::Berm => {
                        egui::Color32::from_rgba_unmultiplied(180, 140, 60, 45) // tan
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
                // 2.5D preview: south face strip below the blueprint
                if bp.is_wall() || is_wall_block(bp.block_data & 0xFF) {
                    let tw = sx1 - sx0;
                    let face_h = tw * 0.12; // face height proportional to tile size
                    let face_tint = if bp.resources_met() {
                        egui::Color32::from_rgba_unmultiplied(40, 120, 90, 70)
                    } else {
                        egui::Color32::from_rgba_unmultiplied(40, 90, 170, 70)
                    };
                    // Face strip directly below the tile
                    let face_rect =
                        egui::Rect::from_min_size(egui::pos2(sx0, sy1), egui::vec2(tw, face_h));
                    // Only show for edges that face south (bit 2 = S)
                    let has_south = if bp.is_wall() {
                        (bp.wall_edges & 4) != 0 || bp.wall_thickness >= 4
                    } else {
                        true // full block walls always have south face
                    };
                    if has_south {
                        bp_painter.rect_filled(face_rect, 0.0, face_tint);
                        // Darker bottom for AO effect
                        let ao_rect = egui::Rect::from_min_size(
                            egui::pos2(sx0, sy1 + face_h * 0.6),
                            egui::vec2(tw, face_h * 0.4),
                        );
                        bp_painter.rect_filled(
                            ao_rect,
                            0.0,
                            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 30),
                        );
                    }
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
                        let color = if bp.wood_delivered >= bp.wood_needed {
                            egui::Color32::from_rgb(100, 200, 100)
                        } else {
                            egui::Color32::from_rgb(255, 160, 60)
                        };
                        Some((
                            format!("{}/{} sticks", bp.wood_delivered, bp.wood_needed),
                            color,
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

                // 2.5D preview: south face strip for wall placements
                let is_wall_tool = matches!(
                    self.build_tool,
                    BuildTool::Place(bt) if is_wall_block(bt)
                );
                if is_wall_tool && state != 0 {
                    let tw = sx1 - sx0;
                    let face_h = tw * 0.12;
                    let face_color = egui::Color32::from_rgba_unmultiplied(60, 140, 200, 60);
                    painter.rect_filled(
                        egui::Rect::from_min_size(egui::pos2(sx0, sy1), egui::vec2(tw, face_h)),
                        0.0,
                        face_color,
                    );
                    // AO at bottom of face
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(sx0, sy1 + face_h * 0.6),
                            egui::vec2(tw, face_h * 0.4),
                        ),
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 25),
                    );
                }

                // Door/window preview on hovered wall tile
                if matches!(
                    self.build_tool,
                    BuildTool::Door | BuildTool::WindowOpening | BuildTool::Window
                ) && state != 0
                {
                    let tw = sx1 - sx0;
                    let face_h = tw * 0.12;
                    // Show existing wall face + the door/window overlay
                    let feature_color = match self.build_tool {
                        BuildTool::Door => egui::Color32::from_rgba_unmultiplied(140, 100, 50, 100),
                        BuildTool::WindowOpening => {
                            egui::Color32::from_rgba_unmultiplied(40, 40, 40, 100)
                        }
                        BuildTool::Window => {
                            egui::Color32::from_rgba_unmultiplied(100, 150, 200, 80)
                        }
                        _ => egui::Color32::TRANSPARENT,
                    };
                    // Feature in center of the face
                    let feat_left = sx0 + tw * 0.25;
                    let feat_right = sx0 + tw * 0.75;
                    let feat_top = sy1;
                    let feat_bottom = sy1 + face_h;
                    // Wall face background
                    painter.rect_filled(
                        egui::Rect::from_min_size(egui::pos2(sx0, sy1), egui::vec2(tw, face_h)),
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(120, 110, 90, 70),
                    );
                    // Feature opening/panel
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(feat_left, feat_top + face_h * 0.1),
                            egui::pos2(feat_right, feat_bottom - face_h * 0.05),
                        ),
                        0.0,
                        feature_color,
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
                        PlebCommand::GatherBranches(x, y)
                        | PlebCommand::Butcher(x, y)
                        | PlebCommand::Fish(x, y)
                        | PlebCommand::Mine(x, y)
                        | PlebCommand::OpenSalvageCrate(x, y) => (*x as f32 + 0.5, *y as f32 + 0.5),
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

        // Terrain brush preview (dots showing influence)
        if let Some(tool) = self.terrain_tool {
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let brush_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("terrain_brush"),
            ));
            let (hx, hy) = self.hover_world;
            let cx_t = hx.floor() as i32;
            let cy_t = hy.floor() as i32;
            let r = self.terrain_brush_radius as i32;
            let sigma = r as f32 / 2.5;
            for dy in -r..=r {
                for dx in -r..=r {
                    let w = (-(dx * dx + dy * dy) as f32 / (2.0 * sigma * sigma)).exp();
                    if w < 0.05 {
                        continue;
                    }
                    let tx = cx_t + dx;
                    let ty = cy_t + dy;
                    let sx = ((tx as f32 + 0.5 - cam_cx) * cam_zoom + cam_sw * 0.5)
                        / self.render_scale
                        / bp_ppp;
                    let sy = ((ty as f32 + 0.5 - cam_cy) * cam_zoom + cam_sh * 0.5)
                        / self.render_scale
                        / bp_ppp;
                    let dot_r = 2.0 + w * 5.0;
                    let alpha = (w * 200.0).min(200.0) as u8;
                    let col = match tool {
                        TerrainTool::Raise => {
                            egui::Color32::from_rgba_unmultiplied(80, 220, 80, alpha)
                        }
                        TerrainTool::Lower => {
                            egui::Color32::from_rgba_unmultiplied(220, 80, 80, alpha)
                        }
                        TerrainTool::Flatten => {
                            egui::Color32::from_rgba_unmultiplied(220, 200, 60, alpha)
                        }
                        TerrainTool::Smooth => {
                            egui::Color32::from_rgba_unmultiplied(80, 140, 220, alpha)
                        }
                    };
                    brush_painter.circle_filled(egui::pos2(sx, sy), dot_r, col);
                }
            }
            // Brush size label
            let label_sx = ((cx_t as f32 + r as f32 + 1.5 - cam_cx) * cam_zoom + cam_sw * 0.5)
                / self.render_scale
                / bp_ppp;
            let label_sy = ((cy_t as f32 + 0.5 - cam_cy) * cam_zoom + cam_sh * 0.5)
                / self.render_scale
                / bp_ppp;
            brush_painter.text(
                egui::pos2(label_sx, label_sy),
                egui::Align2::LEFT_CENTER,
                format!("r:{}", r),
                egui::FontId::monospace(10.0),
                egui::Color32::from_gray(180),
            );
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

        // Render footprints (subtle ground marks)
        if tile_px > 6.0 {
            let screen_r = ctx.content_rect();
            let (cam_cx, cam_cy, cam_zoom, cam_sw, cam_sh) = bp_cam;
            let fp_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("footprints"),
            ));
            let fp_lifetime = 60.0; // seconds before fully faded
            for &(fx, fy, _angle, age) in &self.footprints {
                if age < 0.0 || age > fp_lifetime {
                    continue;
                }
                let sx = ((fx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy = ((fy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                // Cull off-screen
                if sx < -10.0
                    || sy < -10.0
                    || sx > screen_r.max.x + 10.0
                    || sy > screen_r.max.y + 10.0
                {
                    continue;
                }
                let fade = 1.0 - (age / fp_lifetime);
                let r = tile_px * 0.04; // tiny mark
                let alpha = (fade * 40.0) as u8; // very subtle
                fp_painter.circle_filled(
                    egui::pos2(sx, sy),
                    r.max(0.8),
                    egui::Color32::from_rgba_unmultiplied(30, 25, 15, alpha),
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
                        egui::Tooltip::always_open(
                            ctx.clone(),
                            egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("item_tip")),
                            egui::Id::new("item_tip_inner"),
                            mp,
                        )
                        .at_pointer()
                        .show(|ui| {
                            ui.label(egui::RichText::new(tip).size(11.0));
                        });
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
                    physics::BodyType::Fragment => {
                        // Fragment: orange-red spark, shorter than bullet, brighter
                        let (gx, gy) = to_screen(body.x, body.y);
                        let speed = (body.vx * body.vx + body.vy * body.vy).sqrt().max(0.001);
                        let trail_len = 0.15 * tile_px;
                        let dx = -body.vx / speed * trail_len;
                        let dy = -body.vy / speed * trail_len;
                        // Bright orange-white spark fading to red
                        let intensity = (speed / 40.0).clamp(0.3, 1.0);
                        let r = 255;
                        let g = (180.0 * intensity) as u8;
                        let b = (80.0 * intensity) as u8;
                        painter.line_segment(
                            [
                                egui::pos2(gx, gy - z_offset),
                                egui::pos2(gx + dx, gy - z_offset + dy),
                            ],
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(r, g, b)),
                        );
                        // Hot core dot
                        painter.circle_filled(
                            egui::pos2(gx, gy - z_offset),
                            1.5,
                            egui::Color32::from_rgb(255, (220.0 * intensity) as u8, 100),
                        );
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

        // --- Group selection rings ---
        if self.selected_group.len() >= 2 {
            let grp_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("group_rings"),
            ));
            let to_scr = |wx: f32, wy: f32| -> egui::Pos2 {
                let sx = ((wx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy = ((wy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                egui::pos2(sx, sy)
            };
            let r = tile_px * 0.35;
            for &pi in &self.selected_group {
                if let Some(p) = self.plebs.get(pi) {
                    if p.is_dead {
                        continue;
                    }
                    let pos = to_scr(p.x, p.y);
                    // Cyan ring for group members
                    grp_painter.circle_stroke(
                        pos,
                        r,
                        egui::Stroke::new(
                            1.5,
                            egui::Color32::from_rgba_unmultiplied(80, 200, 220, 160),
                        ),
                    );
                    // Show group ID if assigned
                    if let Some(gid) = p.group_id {
                        grp_painter.text(
                            pos + egui::Vec2::new(0.0, -r - 2.0),
                            egui::Align2::CENTER_BOTTOM,
                            format!("G{}", gid),
                            egui::FontId::proportional(8.0),
                            egui::Color32::from_rgba_unmultiplied(80, 200, 220, 200),
                        );
                    }
                }
            }
        }

        // --- Move-to marker circle ---
        if let Some((mx, my, timer)) = self.move_marker {
            let marker_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("move_marker"),
            ));
            let to_scr = |wx: f32, wy: f32| -> egui::Pos2 {
                let sx = ((wx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy = ((wy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                egui::pos2(sx, sy)
            };
            let pos = to_scr(mx, my);
            let r = tile_px * 0.4;
            let fade = (timer / 2.0).min(1.0); // fade over 2 seconds
            let alpha = (fade * 80.0) as u8;
            marker_painter.circle_filled(
                pos,
                r,
                egui::Color32::from_rgba_unmultiplied(180, 180, 180, alpha),
            );
            marker_painter.circle_stroke(
                pos,
                r,
                egui::Stroke::new(
                    2.0,
                    egui::Color32::from_rgba_unmultiplied(220, 220, 220, alpha),
                ),
            );
        }

        // --- Debug: show cover positions as circles ---
        if self.debug_show_cover {
            let cover_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("cover_debug"),
            ));
            let to_screen = |wx: f32, wy: f32| -> egui::Pos2 {
                let sx = ((wx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy = ((wy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                egui::pos2(sx, sy)
            };
            let r = tile_px * 0.15;

            // Scan visible area for low walls and draw cover positions
            let half_w = cam_sw * 0.5 / cam_zoom;
            let half_h = cam_sh * 0.5 / cam_zoom;
            let min_x = ((cam_cx - half_w).floor() as i32).max(0);
            let max_x = ((cam_cx + half_w).ceil() as i32).min(GRID_W as i32 - 1);
            let min_y = ((cam_cy - half_h).floor() as i32).max(0);
            let max_y = ((cam_cy + half_h).ceil() as i32).min(GRID_H as i32 - 1);

            for wy in min_y..=max_y {
                for wx in min_x..=max_x {
                    let idx = (wy as u32 * GRID_W + wx as u32) as usize;
                    // Check wall_data for low wall edges
                    let wd = if idx < self.wall_data.len() {
                        self.wall_data[idx]
                    } else {
                        continue;
                    };
                    let is_low_wd = wd_edges(wd) != 0 && wd_height(wd) > 0 && wd_height(wd) < 3;
                    // Check grid_data
                    let bt = block_type_rs(self.grid_data[idx]);
                    let is_low_grid = bt == BT_LOW_WALL;

                    if !is_low_wd && !is_low_grid {
                        continue;
                    }

                    // Draw the wall tile center (yellow)
                    let wc = to_screen(wx as f32 + 0.5, wy as f32 + 0.5);
                    cover_painter.circle_filled(
                        wc,
                        r * 0.6,
                        egui::Color32::from_rgba_unmultiplied(200, 200, 50, 120),
                    );

                    // Draw cover positions on each side (the 4 adjacent tiles)
                    for &(adx, ady) in &[(0i32, -1), (0, 1), (-1, 0), (1, 0)] {
                        let cx = wx + adx;
                        let cy = wy + ady;
                        if cx < 0 || cy < 0 || cx >= GRID_W as i32 || cy >= GRID_H as i32 {
                            continue;
                        }
                        let cidx = (cy as u32 * GRID_W + cx as u32) as usize;
                        let cbt = block_type_rs(self.grid_data[cidx]);
                        let cbh = block_height_rs(self.grid_data[cidx]);
                        // Only walkable tiles are valid cover positions
                        if cbh > 0
                            && !bt_is!(
                                cbt,
                                BT_AIR,
                                BT_GROUND,
                                BT_DUG_GROUND,
                                BT_TREE,
                                BT_BERRY_BUSH,
                                BT_CROP,
                                BT_ROCK
                            )
                        {
                            continue; // not walkable
                        }

                        let pos = to_screen(cx as f32 + 0.5, cy as f32 + 0.5);
                        // Blue = "safe side" (away from typical threat), green = "fire side"
                        let col = egui::Color32::from_rgba_unmultiplied(60, 180, 255, 150);
                        cover_painter.circle_stroke(pos, r, egui::Stroke::new(1.5, col));
                    }
                }
            }
        }

        // --- Debug: show flock cohesion links between plebs ---
        if self.debug_show_flock {
            let flock_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("flock_debug"),
            ));
            let to_scr = |wx: f32, wy: f32| -> egui::Pos2 {
                let sx = ((wx - cam_cx) * cam_zoom + cam_sw * 0.5) / self.render_scale / bp_ppp;
                let sy = ((wy - cam_cy) * cam_zoom + cam_sh * 0.5) / self.render_scale / bp_ppp;
                egui::pos2(sx, sy)
            };

            let enemy_pos: Vec<(f32, f32)> = self
                .plebs
                .iter()
                .filter(|p| p.is_enemy && !p.is_dead)
                .map(|p| (p.x, p.y))
                .collect();
            let links =
                comms::compute_flock_links(&self.plebs, &enemy_pos, true, self.flock_spacing);

            for link in &links {
                let a = to_scr(link.ax, link.ay);
                let b = to_scr(link.bx, link.by);
                let alpha = (link.strength * 180.0) as u8;
                let (r, g, bb) = match link.force {
                    comms::FlockForce::Separation => (220, 60, 60), // red: too close
                    comms::FlockForce::Cohesion => (60, 100, 220),  // blue: pulling together
                    comms::FlockForce::Group => (140, 140, 140),    // gray: in range
                };
                let col = egui::Color32::from_rgba_unmultiplied(r, g, bb, alpha);
                let width = match link.force {
                    comms::FlockForce::Separation => 2.5,
                    comms::FlockForce::Cohesion => 1.5,
                    comms::FlockForce::Group => 0.8,
                };
                flock_painter.line_segment([a, b], egui::Stroke::new(width, col));
            }
        }
    }

    /// Draw text with a dark shadow for readability on bright backgrounds.
    pub(crate) fn shadow_text(
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

    pub(crate) fn world_label(
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

    pub(crate) fn draw_world_labels(
        &mut self,
        ctx: &egui::Context,
        bp_cam: (f32, f32, f32, f32, f32),
    ) {
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
                    // Dead plebs: no floating labels (info still on hover)
                    if pleb.is_dead {
                        continue;
                    }
                    let pos = to_screen(pleb.x, pleb.y + 0.7);
                    // Cull off-screen plebs
                    if pos.x < screen_rect.min.x - 60.0
                        || pos.x > screen_rect.max.x + 60.0
                        || pos.y < screen_rect.min.y - 60.0
                        || pos.y > screen_rect.max.y + 60.0
                    {
                        continue;
                    }

                    // Bubble (above pleb head)
                    if let Some((ref bubble, timer)) = pleb.bubble {
                        let fade = if timer < 0.3 { timer / 0.3 } else { 1.0 };
                        // Position above head
                        let bubble_pos = to_screen(pleb.x, pleb.y - 1.0);
                        let bg_alpha = (fade * 230.0) as u8;
                        let bg_col = egui::Color32::from_rgba_unmultiplied(245, 245, 245, bg_alpha);
                        let border_col =
                            egui::Color32::from_rgba_unmultiplied(180, 180, 180, bg_alpha);

                        let is_thought = matches!(bubble, pleb::BubbleKind::Thought(_));
                        let (text, font_size, text_col) = match bubble {
                            pleb::BubbleKind::Icon(ch, rgb) => (
                                format!("{}", ch),
                                14.0,
                                egui::Color32::from_rgba_unmultiplied(
                                    rgb[0],
                                    rgb[1],
                                    rgb[2],
                                    (fade * 255.0) as u8,
                                ),
                            ),
                            pleb::BubbleKind::Text(t) => (
                                t.clone(),
                                9.0,
                                egui::Color32::from_rgba_unmultiplied(
                                    40,
                                    40,
                                    45,
                                    (fade * 255.0) as u8,
                                ),
                            ),
                            pleb::BubbleKind::Thought(t) => (
                                t.clone(),
                                8.5,
                                egui::Color32::from_rgba_unmultiplied(
                                    80,
                                    75,
                                    70,
                                    (fade * 220.0) as u8,
                                ),
                            ),
                        };

                        let galley = label_painter.layout_no_wrap(
                            text.clone(),
                            egui::FontId::proportional(font_size),
                            text_col,
                        );
                        let text_rect =
                            egui::Align2::CENTER_BOTTOM.anchor_size(bubble_pos, galley.size());
                        let pad = egui::Vec2::new(5.0, 3.0);
                        let bg_rect = text_rect.expand2(pad);

                        if is_thought {
                            // Thought bubble: rounded cloud with small circles below
                            let thought_bg =
                                egui::Color32::from_rgba_unmultiplied(240, 240, 235, bg_alpha);
                            let thought_border =
                                egui::Color32::from_rgba_unmultiplied(170, 170, 165, bg_alpha);
                            label_painter.rect_filled(bg_rect, 10.0, thought_bg);
                            label_painter.rect_stroke(
                                bg_rect,
                                10.0,
                                egui::Stroke::new(0.5, thought_border),
                                egui::StrokeKind::Outside,
                            );
                            // Small trailing circles (thought cloud tail)
                            let cx = bg_rect.center().x;
                            let by_pos = bg_rect.max.y;
                            label_painter.circle_filled(
                                egui::pos2(cx + 2.0, by_pos + 3.0),
                                2.5,
                                thought_bg,
                            );
                            label_painter.circle_filled(
                                egui::pos2(cx + 5.0, by_pos + 7.0),
                                1.5,
                                thought_bg,
                            );
                        } else {
                            // Speech bubble: white rounded rect with triangle tail
                            label_painter.rect_filled(bg_rect, 6.0, bg_col);
                            label_painter.rect_stroke(
                                bg_rect,
                                6.0,
                                egui::Stroke::new(0.5, border_col),
                                egui::StrokeKind::Outside,
                            );
                            let tri_y = bg_rect.max.y;
                            let tri_cx = bg_rect.center().x;
                            label_painter.add(egui::Shape::convex_polygon(
                                vec![
                                    egui::pos2(tri_cx - 4.0, tri_y),
                                    egui::pos2(tri_cx + 4.0, tri_y),
                                    egui::pos2(tri_cx, tri_y + 5.0),
                                ],
                                bg_col,
                                egui::Stroke::NONE,
                            ));
                        }
                        // Text
                        label_painter.text(
                            bubble_pos,
                            egui::Align2::CENTER_BOTTOM,
                            text,
                            egui::FontId::proportional(font_size),
                            text_col,
                        );
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

                    // Status label (situational — only shown for notable states)
                    {
                        let px = pleb.x.floor() as i32;
                        let py = pleb.y.floor() as i32;
                        let widx = if px >= 0 && py >= 0 && px < GRID_W as i32 && py < GRID_H as i32
                        {
                            (py as u32 * GRID_W + px as u32) as usize
                        } else {
                            usize::MAX
                        };
                        // Water depth: use CPU mirror, OR seep formula as fallback
                        let water_d = if widx < self.water_depth_cpu.len()
                            && self.water_depth_cpu[widx] > 0.01
                        {
                            self.water_depth_cpu[widx]
                        } else if widx < self.water_table.len() {
                            // Fallback: check seep formula
                            let sub_e = crate::terrain::sample_elevation(
                                &self.sub_elevation,
                                pleb.x,
                                pleb.y,
                            );
                            (self.water_table[widx] + self.camera.water_table_offset - sub_e)
                                .max(0.0)
                        } else {
                            0.0
                        };

                        // Status label: show only the MOST critical status
                        // Priority: life-threatening > urgent needs > warning > info
                        // Color: red bg = critical, orange bg = danger, amber = warning, teal = info
                        let (status_text, text_col, bg_col): (
                            Option<&str>,
                            egui::Color32,
                            egui::Color32,
                        ) = if pleb.bleeding > 0.5 {
                            (
                                Some("Bleeding"),
                                egui::Color32::WHITE,
                                egui::Color32::from_rgb(170, 30, 30),
                            )
                        } else if pleb.needs.health < 0.10 {
                            (
                                Some("Dying"),
                                egui::Color32::WHITE,
                                egui::Color32::from_rgb(160, 20, 20),
                            )
                        } else if pleb.needs.hunger < 0.08 {
                            (
                                Some("Starving"),
                                egui::Color32::WHITE,
                                egui::Color32::from_rgb(170, 30, 30),
                            )
                        } else if pleb.needs.thirst < 0.08 {
                            (
                                Some("Dehydrated"),
                                egui::Color32::WHITE,
                                egui::Color32::from_rgb(170, 30, 30),
                            )
                        } else if pleb.needs.stress > 85.0 {
                            (
                                Some("Breaking!"),
                                egui::Color32::WHITE,
                                egui::Color32::from_rgb(170, 30, 30),
                            )
                        } else if pleb.needs.warmth < 0.15 {
                            (
                                Some("Freezing"),
                                egui::Color32::WHITE,
                                egui::Color32::from_rgb(40, 90, 160),
                            )
                        } else if pleb.needs.health < 0.25 {
                            (
                                Some("Wounded"),
                                egui::Color32::from_rgb(255, 240, 220),
                                egui::Color32::from_rgb(180, 80, 20),
                            )
                        } else if pleb.bleeding > 0.1 {
                            (
                                Some("Bleeding"),
                                egui::Color32::from_rgb(255, 240, 220),
                                egui::Color32::from_rgb(180, 60, 40),
                            )
                        } else if pleb.needs.hunger < 0.20 {
                            (
                                Some("Hungry"),
                                egui::Color32::from_rgb(255, 240, 220),
                                egui::Color32::from_rgb(170, 110, 20),
                            )
                        } else if pleb.needs.thirst < 0.20 {
                            (
                                Some("Thirsty"),
                                egui::Color32::from_rgb(255, 240, 220),
                                egui::Color32::from_rgb(30, 110, 170),
                            )
                        } else if pleb.needs.rest < 0.15 {
                            (
                                Some("Exhausted"),
                                egui::Color32::from_rgb(255, 240, 220),
                                egui::Color32::from_rgb(100, 80, 140),
                            )
                        } else if pleb.nauseous_timer > 0.0 {
                            (
                                Some("Nauseous"),
                                egui::Color32::from_rgb(255, 240, 220),
                                egui::Color32::from_rgb(120, 140, 50),
                            )
                        } else if pleb.needs.stress > 70.0 {
                            (
                                Some("Stressed"),
                                egui::Color32::from_rgb(40, 30, 20),
                                egui::Color32::from_rgb(200, 170, 50),
                            )
                        } else if water_d > 0.3 {
                            (
                                Some("Deep water"),
                                egui::Color32::WHITE,
                                egui::Color32::from_rgb(40, 90, 140),
                            )
                        } else if pleb.suppression > 0.5 {
                            (
                                Some("Suppressed"),
                                egui::Color32::from_rgb(255, 240, 220),
                                egui::Color32::from_rgb(150, 100, 30),
                            )
                        } else if pleb.smoke_exposure > 0.3 {
                            (
                                Some("Choking"),
                                egui::Color32::WHITE,
                                egui::Color32::from_rgb(120, 110, 90),
                            )
                        } else if pleb.needs.wetness > 0.5 {
                            (
                                Some(needs::wetness_label(pleb.needs.wetness)),
                                egui::Color32::WHITE,
                                egui::Color32::from_rgb(50, 100, 160),
                            )
                        } else {
                            (None, egui::Color32::TRANSPARENT, egui::Color32::TRANSPARENT)
                        };

                        if let Some(text) = status_text {
                            let status_pos = egui::pos2(pos.x, pos.y + 13.0);
                            // Background pill
                            let font = egui::FontId::proportional(8.0);
                            let galley = label_painter.layout_no_wrap(
                                text.to_string(),
                                font.clone(),
                                text_col,
                            );
                            let text_rect =
                                egui::Align2::CENTER_TOP.anchor_size(status_pos, galley.size());
                            let pill = text_rect.expand2(egui::Vec2::new(4.0, 1.5));
                            label_painter.rect_filled(pill, 3.0, bg_col);
                            label_painter.text(
                                status_pos,
                                egui::Align2::CENTER_TOP,
                                text,
                                font,
                                text_col,
                            );
                        }
                    }

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
                                        "Farming" => "Walking to farm",
                                        _ => "Walking to build",
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
                            PlebActivity::Digging => {
                                (Some("Digging"), egui::Color32::from_rgb(140, 100, 50))
                            }
                            PlebActivity::Filling => {
                                (Some("Building berm"), egui::Color32::from_rgb(180, 140, 60))
                            }
                            PlebActivity::Butchering(_) => {
                                (Some("Butchering"), egui::Color32::from_rgb(180, 80, 80))
                            }
                            PlebActivity::Cooking(_) => {
                                (Some("Cooking"), egui::Color32::from_rgb(220, 140, 50))
                            }
                            PlebActivity::Fishing(_) => {
                                (Some("Fishing"), egui::Color32::from_rgb(60, 140, 200))
                            }
                            PlebActivity::Mining(_) => {
                                (Some("Mining"), egui::Color32::from_rgb(160, 120, 80))
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

                        // Progress bar for all work activities
                        let progress = match inner {
                            PlebActivity::Farming(p)
                            | PlebActivity::Harvesting(p)
                            | PlebActivity::Butchering(p)
                            | PlebActivity::Cooking(p)
                            | PlebActivity::Fishing(p) => Some(*p),
                            PlebActivity::Building(p) => Some(*p),
                            PlebActivity::Crafting(_, p) => Some(*p),
                            PlebActivity::Drinking(p) => Some(*p),
                            PlebActivity::Digging => {
                                // Compute from elevation change
                                pleb.work_target.and_then(|(tx, ty)| {
                                    let cur = crate::terrain::sample_elevation(
                                        &self.sub_elevation,
                                        tx as f32 + 0.5,
                                        ty as f32 + 0.5,
                                    );
                                    let base = self
                                        .dig_zones
                                        .first()
                                        .and_then(|dz| dz.base_elevations.get(&(tx, ty)).copied())
                                        .unwrap_or(cur);
                                    let target = self
                                        .dig_zones
                                        .first()
                                        .map(|dz| dz.target_depth)
                                        .unwrap_or(0.8);
                                    if target > 0.01 {
                                        Some(((base - cur) / target).clamp(0.0, 1.0))
                                    } else {
                                        Some(0.0)
                                    }
                                })
                            }
                            PlebActivity::Filling => Some(0.5), // approximate
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
                            let bar_col = match inner {
                                PlebActivity::Building(_) => egui::Color32::from_rgb(100, 160, 220),
                                PlebActivity::Digging => egui::Color32::from_rgb(140, 100, 50),
                                PlebActivity::Filling => egui::Color32::from_rgb(180, 140, 60),
                                PlebActivity::Crafting(_, _) => {
                                    egui::Color32::from_rgb(200, 160, 60)
                                }
                                _ => egui::Color32::from_rgb(80, 200, 80),
                            };
                            label_painter.rect_filled(
                                egui::Rect::from_min_size(
                                    bar_pos,
                                    egui::Vec2::new(bar_w * prog, bar_h),
                                ),
                                1.0,
                                bar_col,
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

                // --- Ranged weapon targeting: LOS line + reticle ---
                // Shows during: attack mode (cursor preview) OR active aiming
                {
                    let aiming_plebs: Vec<usize> = if !self.selected_group.is_empty() {
                        self.selected_group.clone()
                    } else if let Some(idx) = self.selected_pleb {
                        vec![idx]
                    } else {
                        vec![]
                    };

                    // In attack mode: snap to enemy near cursor
                    let (hover_wx, hover_wy) = self.hover_world;
                    let hover_enemy = if self.attack_mode {
                        self.plebs
                            .iter()
                            .filter(|e| !e.is_dead && e.is_enemy)
                            .filter(|e| (e.x - hover_wx).powi(2) + (e.y - hover_wy).powi(2) < 2.25)
                            .min_by(|a, b| {
                                let da = (a.x - hover_wx).powi(2) + (a.y - hover_wy).powi(2);
                                let db = (b.x - hover_wx).powi(2) + (b.y - hover_wy).powi(2);
                                da.partial_cmp(&db).unwrap()
                            })
                            .map(|e| (e.x, e.y, true))
                    } else {
                        None
                    };

                    for &pi in &aiming_plebs {
                        let pleb = match self.plebs.get(pi) {
                            Some(p) if !p.is_dead && !p.is_enemy => p,
                            _ => continue,
                        };

                        // Target: attack mode cursor > active aim > aim_pos
                        let (tx, ty, is_enemy_target) = if self.attack_mode {
                            hover_enemy.unwrap_or((hover_wx, hover_wy, false))
                        } else if let Some(ti) = pleb.aim_target {
                            if let Some(target) = self.plebs.get(ti) {
                                (target.x, target.y, target.is_enemy)
                            } else {
                                continue;
                            }
                        } else if let Some((ax, ay)) = pleb.aim_pos {
                            (ax, ay, false)
                        } else {
                            continue;
                        };

                        if !self.attack_mode && pleb.aim_progress <= 0.0 {
                            continue;
                        }

                        let src = to_screen(pleb.x, pleb.y);
                        let dst = to_screen(tx, ty);
                        let sp = egui::Pos2::new(src.x, src.y);
                        let dp = egui::Pos2::new(dst.x, dst.y);

                        // Range check for color
                        let dist = ((tx - pleb.x).powi(2) + (ty - pleb.y).powi(2)).sqrt();
                        let combat_range = 25.0;
                        let in_range = dist <= combat_range;

                        // LOS line: green=clear, yellow=obstructed/water, red=out of range
                        let line_col = if !in_range {
                            egui::Color32::from_rgba_unmultiplied(200, 45, 35, 130)
                        } else if is_enemy_target {
                            egui::Color32::from_rgba_unmultiplied(50, 210, 50, 110)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(80, 180, 200, 90)
                        };
                        label_painter.line_segment([sp, dp], egui::Stroke::new(1.5, line_col));

                        // Pulse animation for enemies
                        let pulse = if is_enemy_target {
                            0.7 + 0.3 * (self.camera.time * 4.0).sin()
                        } else {
                            0.8
                        };

                        let r_col = if is_enemy_target {
                            egui::Color32::from_rgba_unmultiplied(
                                240,
                                50,
                                40,
                                (180.0 * pulse) as u8,
                            )
                        } else {
                            egui::Color32::from_rgba_unmultiplied(180, 200, 220, 120)
                        };
                        let stroke_w = if is_enemy_target { 1.5 } else { 1.0 };
                        let stroke = egui::Stroke::new(stroke_w, r_col);

                        // Target reticle: crosshair circle centered on target
                        let r_outer = tile_px * 0.3;
                        let r_inner = tile_px * 0.08;
                        let cross_len = tile_px * 0.15;

                        // Outer circle
                        label_painter.circle_stroke(dp, r_outer, stroke);
                        // Inner dot
                        label_painter.circle_filled(dp, r_inner, r_col);
                        // Crosshair lines (extending beyond outer circle)
                        let cx_ext = r_outer + cross_len;
                        let cx_gap = r_outer * 0.3;
                        // Top
                        label_painter.line_segment(
                            [
                                egui::Pos2::new(dp.x, dp.y - cx_gap),
                                egui::Pos2::new(dp.x, dp.y - cx_ext),
                            ],
                            stroke,
                        );
                        // Bottom
                        label_painter.line_segment(
                            [
                                egui::Pos2::new(dp.x, dp.y + cx_gap),
                                egui::Pos2::new(dp.x, dp.y + cx_ext),
                            ],
                            stroke,
                        );
                        // Left
                        label_painter.line_segment(
                            [
                                egui::Pos2::new(dp.x - cx_gap, dp.y),
                                egui::Pos2::new(dp.x - cx_ext, dp.y),
                            ],
                            stroke,
                        );
                        // Right
                        label_painter.line_segment(
                            [
                                egui::Pos2::new(dp.x + cx_gap, dp.y),
                                egui::Pos2::new(dp.x + cx_ext, dp.y),
                            ],
                            stroke,
                        );

                        // Center dot for enemies
                        if is_enemy_target {
                            label_painter.circle_filled(dp, 2.0, r_col);
                        }
                    }
                }

                // Move mode: path preview
                if self.move_mode {
                    if let Some(pleb) = self.selected_pleb.and_then(|i| self.plebs.get(i)) {
                        let (hx, hy) = self.hover_world;
                        let start = (pleb.x.floor() as i32, pleb.y.floor() as i32);
                        let goal = (hx.floor() as i32, hy.floor() as i32);
                        let path = pleb::astar_path_terrain_water_wd(
                            &self.grid_data,
                            &self.wall_data,
                            &self.terrain_data,
                            &self.water_depth_cpu,
                            start,
                            goal,
                        );
                        if path.len() >= 2 {
                            let pc = egui::Color32::from_rgba_unmultiplied(50, 200, 50, 100);
                            for i in 0..path.len() - 1 {
                                let sa = to_screen(path[i].0 as f32 + 0.5, path[i].1 as f32 + 0.5);
                                let sb = to_screen(
                                    path[i + 1].0 as f32 + 0.5,
                                    path[i + 1].1 as f32 + 0.5,
                                );
                                label_painter.line_segment([sa, sb], egui::Stroke::new(2.0, pc));
                            }
                            let d = path.last().unwrap();
                            label_painter.circle_filled(
                                to_screen(d.0 as f32 + 0.5, d.1 as f32 + 0.5),
                                4.0,
                                egui::Color32::from_rgba_unmultiplied(50, 220, 50, 150),
                            );
                        } else if goal != start {
                            let dp = to_screen(hx, hy);
                            let rc = egui::Color32::from_rgba_unmultiplied(220, 50, 40, 150);
                            label_painter.line_segment(
                                [
                                    egui::Pos2::new(dp.x - 5.0, dp.y - 5.0),
                                    egui::Pos2::new(dp.x + 5.0, dp.y + 5.0),
                                ],
                                egui::Stroke::new(2.0, rc),
                            );
                            label_painter.line_segment(
                                [
                                    egui::Pos2::new(dp.x + 5.0, dp.y - 5.0),
                                    egui::Pos2::new(dp.x - 5.0, dp.y + 5.0),
                                ],
                                egui::Stroke::new(2.0, rc),
                            );
                        }
                    }
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

                // Grenade targeting overlay: line, arc preview, impact circle
                if self.grenade_targeting
                    && let Some(pleb) = self.selected_pleb.and_then(|i| self.plebs.get(i))
                {
                    let (hx, hy) = self.hover_world;
                    let dx = hx - pleb.x;
                    let dy = hy - pleb.y;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let strength = pleb.skills[1].value;
                    let max_range = crate::grenade_max_range(strength, self.grenade_arc);
                    let in_range = dist <= max_range;

                    let pleb_screen = to_screen(pleb.x, pleb.y);
                    let target_screen = to_screen(hx, hy);
                    let half = tile_px * 0.5;

                    // Targeting line (green in range, red out)
                    let line_col = if in_range {
                        egui::Color32::from_rgba_premultiplied(60, 200, 60, 160)
                    } else {
                        egui::Color32::from_rgba_premultiplied(220, 60, 40, 160)
                    };
                    label_painter.line_segment(
                        [
                            egui::Pos2::new(pleb_screen.x + half, pleb_screen.y + half),
                            egui::Pos2::new(target_screen.x + half, target_screen.y + half),
                        ],
                        egui::Stroke::new(2.0, line_col),
                    );

                    // Ballistic arc preview + ground shadow
                    if in_range && dist > 0.5 {
                        let elev = match self.grenade_arc {
                            0 => 0.3f32,
                            2 => 1.0,
                            _ => 0.6,
                        };
                        let ndx = dx / dist;
                        let ndy = dy / dist;
                        let speed_mul = match self.grenade_arc {
                            0 => 1.0f32,
                            2 => 0.6,
                            _ => 0.85,
                        };
                        let max_v = (15.0 + strength * 3.0) * speed_mul;
                        let sin2a = (2.0 * elev).sin().max(0.1);
                        let v = (dist * 25.0 / sin2a).sqrt().min(max_v);
                        let flight_time = 2.0 * v * elev.sin() / 25.0;
                        let hvel = v * elev.cos();

                        let arc_steps = 20;
                        let mut prev_shadow =
                            egui::Pos2::new(pleb_screen.x + half, pleb_screen.y + half);
                        let mut prev_arc = egui::Pos2::new(
                            pleb_screen.x + half,
                            pleb_screen.y + half - 1.2 * tile_px * 0.3,
                        );

                        for step in 1..=arc_steps {
                            let t = step as f32 / arc_steps as f32;
                            let wt = flight_time * t;
                            let wx = pleb.x + ndx * hvel * wt;
                            let wy = pleb.y + ndy * hvel * wt;
                            let wz = (1.2 + v * elev.sin() * wt - 0.5 * 25.0 * wt * wt).max(0.0);

                            let sp_raw = to_screen(wx, wy);
                            let sp = egui::Pos2::new(sp_raw.x + half, sp_raw.y + half);

                            // Ground shadow (dashed)
                            if step % 2 == 0 {
                                label_painter.line_segment(
                                    [prev_shadow, sp],
                                    egui::Stroke::new(
                                        1.0,
                                        egui::Color32::from_rgba_premultiplied(0, 0, 0, 60),
                                    ),
                                );
                            }
                            prev_shadow = sp;

                            // Arc line (offset upward by z)
                            let z_px = wz * tile_px * 0.3;
                            let arc_pt = egui::Pos2::new(sp.x, sp.y - z_px);
                            label_painter.line_segment(
                                [prev_arc, arc_pt],
                                egui::Stroke::new(
                                    2.0,
                                    egui::Color32::from_rgba_premultiplied(255, 180, 60, 180),
                                ),
                            );
                            prev_arc = arc_pt;
                        }
                    }

                    // Impact probability circle
                    {
                        let base_spread = 0.5 + dist * 0.04;
                        let arc_spread = match self.grenade_arc {
                            0 => 0.0f32,
                            2 => 0.4,
                            _ => 0.15,
                        };
                        let skill_mod = 1.0 - strength * 0.03;
                        let radius = (base_spread + arc_spread) * skill_mod.max(0.3);
                        let radius_px = radius * tile_px;
                        let center =
                            egui::Pos2::new(target_screen.x + half, target_screen.y + half);
                        let alpha = if in_range { 25u8 } else { 12 };
                        let stroke_alpha = if in_range { 50u8 } else { 25 };
                        label_painter.circle_filled(
                            center,
                            radius_px,
                            egui::Color32::from_rgba_unmultiplied(0, 0, 0, alpha),
                        );
                        label_painter.circle_stroke(
                            center,
                            radius_px,
                            egui::Stroke::new(
                                1.0,
                                egui::Color32::from_rgba_unmultiplied(200, 200, 200, stroke_alpha),
                            ),
                        );
                    }
                }
            }
        }
    }

    pub(crate) fn draw_minimap(&mut self, ctx: &egui::Context) {
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
            let screen = ctx.content_rect();
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

        let mut minimap_click: Option<(f32, f32)> = None;
        egui::Area::new(egui::Id::new("minimap"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -180.0])
            .interactable(true)
            .show(ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(
                    egui::Vec2::splat(map_size),
                    egui::Sense::click_and_drag(),
                );

                // Click or drag: convert screen position to world coordinates
                if resp.clicked() || resp.dragged() {
                    if let Some(pos) = resp.interact_pointer_pos() {
                        let wx = ((pos.x - rect.min.x) / map_size * gw).clamp(0.0, gw - 1.0);
                        let wy = ((pos.y - rect.min.y) / map_size * gh).clamp(0.0, gh - 1.0);
                        minimap_click = Some((wx, wy));
                    }
                }
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

        // Apply camera move from minimap interaction
        if let Some((wx, wy)) = minimap_click {
            self.camera.center_x = wx;
            self.camera.center_y = wy;
        }
    }
}
