//! Tree sprite generation — procedural heightmap sprites for tree rendering.

pub const SPRITE_SIZE: u32 = 16;
pub const SPRITE_VARIANTS: u32 = 8;

pub fn generate_tree_sprites() -> Vec<u32> {
    let pixels_per = (SPRITE_SIZE * SPRITE_SIZE) as usize;
    let total = pixels_per * SPRITE_VARIANTS as usize;
    let mut data = vec![0u32; total];

    for variant in 0..SPRITE_VARIANTS {
        for y in 0..SPRITE_SIZE {
            for x in 0..SPRITE_SIZE {
                let cx = (x as f32 + 0.5) / SPRITE_SIZE as f32 - 0.5;
                let cy = (y as f32 + 0.5) / SPRITE_SIZE as f32 - 0.5;
                let dist = (cx * cx + cy * cy).sqrt();

                let (r, g, b, h) = match variant {
                    0 => {
                        // Round oak: large canopy
                        let canopy_r = 0.48;
                        let trunk_r = 0.08;
                        if dist < trunk_r {
                            (90, 58, 28, 60u8) // trunk: LOW height
                        } else if dist < canopy_r {
                            let shade = 1.0 - dist / canopy_r;
                            let g = (55.0 + shade * 90.0) as u8;
                            let h = (140.0 + shade * 80.0) as u8;
                            (30 + (shade * 25.0) as u8, g, 18, h)
                        } else {
                            (0, 0, 0, 0u8)
                        }
                    }
                    1 => {
                        // Pine/conifer: pointed, diamond-ish shape
                        let abs_cx = cx.abs();
                        let abs_cy = cy.abs();
                        let diamond = abs_cx + abs_cy;
                        let trunk_r = 0.05;
                        let canopy_r = 0.42 - (cy + 0.1).abs() * 0.25;
                        let canopy_r = canopy_r.max(0.06);
                        if dist < trunk_r {
                            (75, 48, 22, 55u8) // trunk: LOW height
                        } else if diamond < canopy_r + 0.12 && dist < 0.48 {
                            let shade = 1.0 - diamond / (canopy_r + 0.1);
                            let g = (40.0 + shade * 60.0) as u8;
                            let h = (160.0 + shade * 70.0) as u8;
                            (15 + (shade * 20.0) as u8, g, 22, h)
                        } else {
                            (0, 0, 0, 0u8)
                        }
                    }
                    2 => {
                        // Small bush: low, wide, lumpy
                        let canopy_r = 0.40;
                        let trunk_r = 0.05;
                        let angle = cy.atan2(cx);
                        let lump = 1.0 + 0.12 * (angle * 3.0).sin() + 0.08 * (angle * 7.0).sin();
                        let effective_r = canopy_r * lump;
                        if dist < trunk_r {
                            (80, 52, 25, 50u8) // trunk: LOW height
                        } else if dist < effective_r {
                            let shade = 1.0 - dist / effective_r;
                            let g = (65.0 + shade * 70.0) as u8;
                            let h = (130.0 + shade * 50.0) as u8;
                            (40 + (shade * 20.0) as u8, g, 25, h)
                        } else {
                            (0, 0, 0, 0u8)
                        }
                    }
                    3 => {
                        // Tall narrow tree: thin canopy
                        let canopy_rx = 0.26;
                        let canopy_ry = 0.44;
                        let trunk_r = 0.06;
                        let ellipse = (cx / canopy_rx).powi(2) + (cy / canopy_ry).powi(2);
                        if dist < trunk_r {
                            (85, 55, 25, 50u8) // trunk: LOW height
                        } else if ellipse < 1.0 {
                            let shade = 1.0 - ellipse;
                            let g = (50.0 + shade * 80.0) as u8;
                            let h = (170.0 + shade * 70.0) as u8;
                            (25 + (shade * 20.0) as u8, g, 20, h)
                        } else {
                            (0, 0, 0, 0u8)
                        }
                    }
                    4 => {
                        // Willow: drooping wide canopy, thin trunk
                        let trunk_r = 0.04;
                        let canopy_r = 0.46;
                        let droop = (cx.abs() * 3.0).sin() * 0.08; // wavy edge
                        if dist < trunk_r {
                            (85, 55, 28, 60u8) // trunk: LOW height
                        } else if dist < canopy_r + droop {
                            let shade = 1.0 - dist / canopy_r;
                            let g = (60.0 + shade * 70.0) as u8;
                            (
                                25 + (shade * 15.0) as u8,
                                g,
                                30,
                                (140.0 + shade * 60.0) as u8,
                            )
                        } else {
                            (0, 0, 0, 0)
                        }
                    }
                    5 => {
                        // Dense bush cluster: multiple lumps, no visible trunk
                        let lump1 = ((cx + 0.15).powi(2) + (cy - 0.1).powi(2)).sqrt();
                        let lump2 = ((cx - 0.12).powi(2) + (cy + 0.12).powi(2)).sqrt();
                        let lump3 = ((cx + 0.05).powi(2) + (cy + 0.15).powi(2)).sqrt();
                        let min_d = lump1.min(lump2).min(lump3);
                        if min_d < 0.22 {
                            let shade = 1.0 - min_d / 0.22;
                            let g = (55.0 + shade * 65.0) as u8;
                            (
                                35 + (shade * 15.0) as u8,
                                g,
                                28,
                                (100.0 + shade * 50.0) as u8,
                            )
                        } else {
                            (0, 0, 0, 0)
                        }
                    }
                    6 => {
                        // Tall cedar: narrow, very tall, prominent trunk
                        let trunk_r = 0.07;
                        let canopy_rx = 0.18;
                        let canopy_ry = 0.48;
                        let ellipse = (cx / canopy_rx).powi(2) + (cy / canopy_ry).powi(2);
                        if dist < trunk_r && cy > 0.1 {
                            (70, 45, 20, 50u8) // trunk visible below canopy
                        } else if ellipse < 1.0 {
                            let shade = 1.0 - ellipse;
                            let g = (35.0 + shade * 55.0) as u8;
                            (
                                12 + (shade * 15.0) as u8,
                                g,
                                18,
                                (170.0 + shade * 80.0) as u8,
                            )
                        } else {
                            (0, 0, 0, 0)
                        }
                    }
                    _ => {
                        // Birch: light bark, sparse canopy with gaps
                        let trunk_r = 0.06;
                        let canopy_r = 0.38;
                        let gap = ((cx * 8.0).sin() * (cy * 6.0).cos()).abs();
                        if dist < trunk_r {
                            (200, 195, 180, 55u8) // white bark trunk
                        } else if dist < canopy_r && gap < 0.6 {
                            let shade = 1.0 - dist / canopy_r;
                            let g = (70.0 + shade * 80.0) as u8;
                            (
                                40 + (shade * 20.0) as u8,
                                g,
                                15,
                                (130.0 + shade * 70.0) as u8,
                            )
                        } else {
                            (0, 0, 0, 0)
                        }
                    }
                };

                let packed =
                    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((h as u32) << 24);
                let idx = (variant * SPRITE_SIZE * SPRITE_SIZE + y * SPRITE_SIZE + x) as usize;
                data[idx] = packed;
            }
        }
    }

    data
}
