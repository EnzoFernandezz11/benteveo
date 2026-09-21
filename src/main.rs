mod app;
mod config;
mod domain;
mod services;
mod ui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([760.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Benteveo",
        options,
        Box::new(|cc| Ok(Box::new(app::FocusApp::new(cc)))),
    )
}
