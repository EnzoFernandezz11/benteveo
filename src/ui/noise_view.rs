use crate::services::audio::NoiseKind;

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
    ui.heading("// ruido de enfoque");
    ui.add_space(16.0);
    ui.horizontal(|ui| {
        for option in NoiseKind::ALL {
            if ui
                .selectable_label(*kind == option, option.label())
                .clicked()
            {
                *kind = option;
                changed = true;
            }
        }
    });
    ui.add_space(18.0);
    ui.label("volumen");
    changed |= ui
        .add(egui::Slider::new(volume, 0..=100).suffix("%"))
        .changed();
    ui.add_space(18.0);
    if ui
        .button(if playing {
            "■ detener"
        } else {
            "▶ reproducir"
        })
        .clicked()
    {
        play_toggle = true;
    }
    ui.add_space(24.0);
    ui.weak("El sonido continúa al cambiar de pestaña. No se inicia automáticamente al abrir.");
    NoiseResponse {
        changed,
        play_toggle,
    }
}
