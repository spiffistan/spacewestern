//! Reusable card UI widget — frontier-styled cards for characters, events, items, discoveries.
//! Used across the game wherever information is presented as a "card" (Manifest, events, trade).

use crate::theme::palette;

/// Card visual category — determines border color, ribbon color, and overall tone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CardType {
    Person,    // amber/warm — recruitment, character info
    Event,     // deep red — random events, raids, crises
    Item,      // steel blue — item inspection, loot
    Discovery, // gold — eureka, aptitude reveal, hidden trait
    Lore,      // dark brown — artifacts, notes, books
    Trade,     // green — caravan offers, trade deals
}

impl CardType {
    /// Border color for the card frame
    pub fn border_color(&self) -> egui::Color32 {
        match self {
            CardType::Person => egui::Color32::from_rgb(170, 140, 80),
            CardType::Event => egui::Color32::from_rgb(150, 55, 45),
            CardType::Item => egui::Color32::from_rgb(90, 120, 150),
            CardType::Discovery => egui::Color32::from_rgb(180, 160, 70),
            CardType::Lore => egui::Color32::from_rgb(120, 90, 55),
            CardType::Trade => egui::Color32::from_rgb(70, 130, 60),
        }
    }

    /// Ribbon/accent color (top strip)
    pub fn ribbon_color(&self) -> egui::Color32 {
        match self {
            CardType::Person => egui::Color32::from_rgb(195, 165, 65),
            CardType::Event => egui::Color32::from_rgb(185, 45, 35),
            CardType::Item => egui::Color32::from_rgb(70, 115, 165),
            CardType::Discovery => egui::Color32::from_rgb(215, 185, 55),
            CardType::Lore => egui::Color32::from_rgb(155, 115, 65),
            CardType::Trade => egui::Color32::from_rgb(55, 155, 45),
        }
    }
}

// Card colors from the centralized palette
const CARD_BG: egui::Color32 = palette::PARCHMENT;
const CARD_TEXT: egui::Color32 = palette::INK;
const CARD_TEXT_DIM: egui::Color32 = palette::INK_DIM;
const CARD_TEXT_FAINT: egui::Color32 = palette::INK_FAINT;

/// Draw a card frame. Returns the inner response for click handling.
/// `width` is the card width. Height is determined by content.
///
/// The card has:
/// - A thin colored ribbon at the top
/// - Parchment background
/// - Border matching the card type
/// - All content drawn by the `add_contents` closure
pub fn draw_card(
    ui: &mut egui::Ui,
    card_type: CardType,
    width: f32,
    add_contents: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    let border = card_type.border_color();
    let ribbon = card_type.ribbon_color();

    let resp = egui::Frame::NONE
        .fill(CARD_BG)
        .stroke(egui::Stroke::new(1.5, border))
        .corner_radius(3.0)
        .inner_margin(egui::Margin {
            left: 8,
            right: 8,
            top: 0,
            bottom: 8,
        })
        .show(ui, |ui| {
            ui.set_width(width);

            // Ribbon at top
            let (ribbon_rect, _) =
                ui.allocate_exact_size(egui::Vec2::new(width, 3.0), egui::Sense::hover());
            ui.painter_at(ribbon_rect).rect_filled(
                ribbon_rect,
                egui::CornerRadius {
                    nw: 2,
                    ne: 2,
                    sw: 0,
                    se: 0,
                },
                ribbon,
            );

            ui.add_space(4.0);

            // Content
            add_contents(ui);
        });

    resp.response
}

/// Draw a card portrait area (dark recessed rectangle for face/image).
pub fn card_portrait(
    ui: &mut egui::Ui,
    width: f32,
    height: f32,
    draw: impl FnOnce(&egui::Painter, egui::Rect),
) {
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(width, height), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    // Dark recessed background
    painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(28, 26, 32));
    painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(0.5, egui::Color32::from_rgb(60, 55, 45)),
        egui::StrokeKind::Inside,
    );
    draw(&painter, rect);
}

/// Draw a card section title (small, bold, dimmed).
pub fn card_section(ui: &mut egui::Ui, text: &str) {
    ui.add_space(3.0);
    ui.label(
        egui::RichText::new(text)
            .size(8.0)
            .strong()
            .color(CARD_TEXT_DIM),
    );
}

/// Draw a card title (large, bold).
pub fn card_title(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(12.0)
            .strong()
            .color(CARD_TEXT),
    );
}

/// Draw a card subtitle (smaller, dimmed).
pub fn card_subtitle(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(9.5).color(CARD_TEXT_DIM));
}

/// Draw a thin horizontal divider line.
pub fn card_divider(ui: &mut egui::Ui) {
    ui.add_space(2.0);
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(width, 1.0), egui::Sense::hover());
    ui.painter_at(rect)
        .rect_filled(rect, 0.0, egui::Color32::from_rgb(200, 185, 160));
    ui.add_space(2.0);
}

/// Draw a skill bar inside a card (compact, themed).
pub fn card_skill_bar(ui: &mut egui::Ui, label: &str, value: f32, max: f32, color: egui::Color32) {
    let bar_w = 60.0;
    let bar_h = 6.0;
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .size(8.0)
                .monospace()
                .color(CARD_TEXT_DIM),
        );
        let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(bar_w, bar_h), egui::Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(50, 45, 38));
        let fill = (value / max).clamp(0.0, 1.0);
        let fill_w = bar_w * fill;
        if fill_w > 0.5 {
            painter.rect_filled(
                egui::Rect::from_min_size(rect.min, egui::Vec2::new(fill_w, bar_h)),
                2.0,
                color,
            );
        }
        // Level number
        ui.label(
            egui::RichText::new(format!("{:.1}", value))
                .size(7.5)
                .color(CARD_TEXT_FAINT),
        );
    });
}

/// Draw a trait tag (small colored pill).
pub fn card_trait_tag(ui: &mut egui::Ui, name: &str, positive: bool) {
    let (bg, fg) = if positive {
        (
            egui::Color32::from_rgb(60, 120, 60),
            egui::Color32::from_rgb(220, 240, 210),
        )
    } else {
        (
            egui::Color32::from_rgb(140, 55, 45),
            egui::Color32::from_rgb(240, 220, 210),
        )
    };
    let text_width = name.len() as f32 * 5.0 + 10.0;
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(text_width, 14.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, bg);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        name,
        egui::FontId::proportional(8.0),
        fg,
    );
}

/// Draw the hidden trait "?" tag.
pub fn card_hidden_trait(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(50.0, 14.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, egui::Color32::from_rgb(80, 75, 65));
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "? ? ? ? ?",
        egui::FontId::proportional(8.0),
        egui::Color32::from_rgb(160, 150, 130),
    );
}

/// Draw a flavor quote at the bottom of a card (italic-style, faint).
pub fn card_quote(ui: &mut egui::Ui, text: &str) {
    ui.add_space(3.0);
    ui.label(
        egui::RichText::new(format!("\"{}\"", text))
            .size(8.0)
            .italics()
            .color(CARD_TEXT_FAINT),
    );
}

/// Draw a card action button (recruit, dismiss, accept, etc.)
pub fn card_button(ui: &mut egui::Ui, label: &str) -> bool {
    let btn_color = egui::Color32::from_rgb(170, 145, 85);
    let btn_text = egui::Color32::from_rgb(40, 35, 20);

    let btn = egui::Button::new(
        egui::RichText::new(label)
            .size(10.0)
            .strong()
            .color(btn_text),
    )
    .fill(btn_color)
    .corner_radius(2.0);

    let resp = ui.add(btn);
    resp.clicked()
}

/// Draw gear icons as a compact row.
pub fn card_gear_row(ui: &mut egui::Ui, items: &[(&str, &str)]) {
    // items: [(icon, name), ...]
    ui.horizontal(|ui| {
        for (icon, _name) in items {
            ui.label(egui::RichText::new(*icon).size(13.0).color(CARD_TEXT));
        }
    });
}
