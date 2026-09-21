use egui::{Color32, Frame, Margin, RichText, Stroke};

pub const INK: Color32 = Color32::from_rgb(10, 11, 10);
pub const PANEL: Color32 = Color32::from_rgb(19, 21, 19);
pub const PANEL_RAISED: Color32 = Color32::from_rgb(29, 32, 29);
pub const GRID: Color32 = Color32::from_rgb(54, 59, 52);
pub const TEXT: Color32 = Color32::from_rgb(238, 239, 228);
pub const MUTED: Color32 = Color32::from_rgb(145, 151, 139);
pub const ACCENT: Color32 = Color32::from_rgb(234, 207, 93);
pub const SIGNAL: Color32 = Color32::from_rgb(107, 212, 142);

pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::monospace(28.0));
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::monospace(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::monospace(13.0));
    style.spacing.item_spacing = egui::vec2(9.0, 10.0);
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    ctx.set_style(style);

    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(TEXT);
    visuals.panel_fill = INK;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = Color32::BLACK;
    visuals.faint_bg_color = PANEL_RAISED;
    visuals.widgets.inactive.bg_fill = PANEL_RAISED;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, GRID);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(47, 51, 42);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    visuals.widgets.active.bg_fill = Color32::from_rgb(68, 65, 35);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    visuals.selection.bg_fill = Color32::from_rgb(93, 82, 31);
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    ctx.set_visuals(visuals);
}

pub fn terminal_panel(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui)) {
    Frame::group(ui.style())
        .fill(PANEL)
        .stroke(Stroke::new(1.0_f32, GRID))
        .inner_margin(Margin::same(18))
        .show(ui, contents);
}

pub fn title(ui: &mut egui::Ui, code: &str, text: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("[ {code} ]")).color(ACCENT).strong());
        ui.heading(text);
    });
}

pub fn label(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(RichText::new(text.into()).color(MUTED));
}

pub fn meter(ui: &mut egui::Ui, value: f32, segments: usize, color: Color32) {
    let filled = (value.clamp(0.0, 1.0) * segments as f32).round() as usize;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        for index in 0..segments {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(9.0, 16.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(rect, 0.0, if index < filled { color } else { GRID });
        }
    });
}
