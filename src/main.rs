mod app;
mod config;
mod domain;
mod services;
mod ui;

fn main() -> eframe::Result<()> {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../resources/benteveo-pixel.png"))
        .expect("el ícono integrado de Benteveo debe ser un PNG válido");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("benteveo")
            .with_inner_size([760.0, 480.0])
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native(
        "Benteveo",
        options,
        Box::new(|cc| Ok(Box::new(app::FocusApp::new(cc)))),
    )
}
