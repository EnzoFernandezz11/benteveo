pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::monospace(25.0));
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::monospace(15.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::monospace(14.0));
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    ctx.set_style(style);
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_rgb(222, 232, 222));
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(45, 102, 76);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(38, 75, 59);
    visuals.selection.bg_fill = egui::Color32::from_rgb(45, 102, 76);
    visuals.panel_fill = egui::Color32::from_rgb(18, 22, 20);
    ctx.set_visuals(visuals);
}
