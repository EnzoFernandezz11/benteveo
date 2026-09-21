use crate::services::audio::NoiseKind;
use crate::ui::theme;

pub struct NoiseResponse {
    pub changed: bool,
    pub play_toggle: bool,
}

pub fn show(
    ui: &mut egui::Ui,
    kind: &mut NoiseKind,
    volume: &mut u8,
    playing: bool,
) -> NoiseResponse {
    let mut changed = false;
    let mut play_toggle = false;
    theme::terminal_panel(ui, |ui| {
        theme::title(ui, "01", "NOISE ENGINE");
        theme::label(ui, "GENERADOR CONTINUO / SIN CONEXIÓN");
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            for option in NoiseKind::ALL {
                let active = *kind == option;
                let text = if active {
                    format!("● {}", option.label().to_uppercase())
                } else {
                    format!("○ {}", option.label().to_uppercase())
                };
                if ui.selectable_label(active, text).clicked() {
                    *kind = option;
                    changed = true;
                }
            }
        });
        ui.add_space(22.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                theme::label(ui, format!("VOLUMEN  /  {volume:03}%"));
                ui.add_sized(
                    [275.0, 18.0],
                    egui::Slider::new(volume, 0..=100).show_value(false),
                );
                theme::meter(ui, f32::from(*volume) / 100.0, 24, theme::ACCENT);
            });
            ui.add_space(22.0);
            let button = egui::Button::new(
                egui::RichText::new(if playing {
                    "■  DETENER"
                } else {
                    "▶  INICIAR"
                })
                .color(theme::INK),
            )
            .fill(if playing {
                theme::SIGNAL
            } else {
                theme::ACCENT
            });
            if ui.add_sized([156.0, 58.0], button).clicked() {
                play_toggle = true;
            }
        });
        ui.add_space(22.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(if playing { "● LIVE" } else { "○ STANDBY" })
                    .color(if playing { theme::SIGNAL } else { theme::MUTED }),
            );
            theme::label(ui, "El audio se mantiene activo al navegar.");
        });
    });
    NoiseResponse {
        changed,
        play_toggle,
    }
}
