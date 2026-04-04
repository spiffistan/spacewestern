//! Design system — centralized palette, typography, and spacing constants.
//! All UI code should reference these instead of hardcoding colors.

/// The frontier palette: warm, weathered, organic.
pub mod palette {
    use egui::Color32;

    // --- Parchment (backgrounds) ---
    pub const PARCHMENT: Color32 = Color32::from_rgb(240, 232, 215);
    pub const PARCHMENT_DARK: Color32 = Color32::from_rgb(225, 215, 195);
    pub const PARCHMENT_HOVER: Color32 = Color32::from_rgb(248, 240, 225);

    // --- Ink (text) ---
    pub const INK: Color32 = Color32::from_rgb(40, 35, 25);
    pub const INK_DIM: Color32 = Color32::from_rgb(100, 90, 70);
    pub const INK_FAINT: Color32 = Color32::from_rgb(140, 125, 100);
    pub const INK_GHOST: Color32 = Color32::from_rgb(175, 160, 135);

    // --- Panel (dark UI backgrounds) ---
    pub const PANEL: Color32 = Color32::from_rgb(30, 32, 38);
    pub const PANEL_LIGHT: Color32 = Color32::from_rgb(42, 45, 52);
    pub const PANEL_HOVER: Color32 = Color32::from_rgb(52, 56, 65);
    pub const PANEL_BORDER: Color32 = Color32::from_rgb(55, 58, 65);

    // --- Accent (category colors for cards, alerts, UI highlights) ---
    pub const AMBER: Color32 = Color32::from_rgb(195, 165, 65); // person, gold, warmth
    pub const AMBER_DIM: Color32 = Color32::from_rgb(170, 140, 80);
    pub const RED: Color32 = Color32::from_rgb(185, 45, 35); // threat, damage, fire
    pub const RED_DIM: Color32 = Color32::from_rgb(150, 55, 45);
    pub const GREEN: Color32 = Color32::from_rgb(55, 155, 45); // positive, growth, health
    pub const GREEN_DIM: Color32 = Color32::from_rgb(70, 130, 60);
    pub const BLUE: Color32 = Color32::from_rgb(70, 115, 165); // item, water, cold
    pub const BLUE_DIM: Color32 = Color32::from_rgb(90, 120, 150);
    pub const BROWN: Color32 = Color32::from_rgb(155, 115, 65); // lore, earth, wood
    pub const BROWN_DIM: Color32 = Color32::from_rgb(120, 90, 55);

    // --- Status (need/health bars, pills) ---
    pub const HP_RED: Color32 = Color32::from_rgb(190, 60, 60);
    pub const HUNGER_AMBER: Color32 = Color32::from_rgb(190, 150, 40);
    pub const THIRST_BLUE: Color32 = Color32::from_rgb(50, 130, 210);
    pub const REST_INDIGO: Color32 = Color32::from_rgb(70, 110, 190);
    pub const WARMTH_ORANGE: Color32 = Color32::from_rgb(190, 95, 35);
    pub const O2_CYAN: Color32 = Color32::from_rgb(90, 190, 210);
    pub const STRESS_LOW: Color32 = Color32::from_rgb(70, 170, 70);
    pub const STRESS_MID: Color32 = Color32::from_rgb(190, 170, 50);
    pub const STRESS_HIGH: Color32 = Color32::from_rgb(190, 55, 55);

    // --- Skill colors ---
    pub const SKILL_SHOOTING: Color32 = Color32::from_rgb(200, 120, 80);
    pub const SKILL_MELEE: Color32 = Color32::from_rgb(200, 80, 80);
    pub const SKILL_CRAFTING: Color32 = Color32::from_rgb(140, 180, 100);
    pub const SKILL_FARMING: Color32 = Color32::from_rgb(100, 170, 60);
    pub const SKILL_MEDICAL: Color32 = Color32::from_rgb(120, 160, 220);
    pub const SKILL_CONSTRUCTION: Color32 = Color32::from_rgb(180, 150, 100);

    /// Get skill color by index
    pub fn skill_color(idx: usize) -> Color32 {
        match idx {
            0 => SKILL_SHOOTING,
            1 => SKILL_MELEE,
            2 => SKILL_CRAFTING,
            3 => SKILL_FARMING,
            4 => SKILL_MEDICAL,
            5 => SKILL_CONSTRUCTION,
            _ => INK_DIM,
        }
    }

    // --- Seal/stamp colors (for notifications, card marks) ---
    pub const SEAL_RED: Color32 = Color32::from_rgb(180, 40, 40);
    pub const SEAL_AMBER: Color32 = Color32::from_rgb(190, 160, 50);
    pub const SEAL_GREEN: Color32 = Color32::from_rgb(50, 150, 70);
    pub const SEAL_GRAY: Color32 = Color32::from_rgb(120, 120, 130);

    // --- White / light ---
    pub const WHITE: Color32 = Color32::from_rgb(245, 240, 230); // warm white
    pub const WHITE_PURE: Color32 = Color32::WHITE;
}

/// Standard text sizes.
pub mod text {
    pub const TITLE: f32 = 14.0;
    pub const HEADING: f32 = 12.0;
    pub const BODY: f32 = 10.0;
    pub const SMALL: f32 = 9.0;
    pub const TINY: f32 = 8.0;
    pub const MICRO: f32 = 7.0;
}

/// Standard spacing values.
pub mod spacing {
    pub const XS: f32 = 2.0;
    pub const SM: f32 = 4.0;
    pub const MD: f32 = 8.0;
    pub const LG: f32 = 12.0;
    pub const XL: f32 = 16.0;
}

/// Standard card/widget widths.
pub mod size {
    pub const CARD_NARROW: f32 = 150.0;
    pub const CARD_NORMAL: f32 = 180.0;
    pub const CARD_WIDE: f32 = 220.0;
    pub const PORTRAIT_H: f32 = 80.0;
}
