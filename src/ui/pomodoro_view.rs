use crate::domain::pomodoro::{Pomodoro, Profile, Stage, TimerState};
use crate::ui::theme;

pub struct PomodoroResponse {
    pub profile_changed: Option<Profile>,
    pub action: Action,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    StartPause,
    Resume,
    Skip,
    Reset,
    Advance,
}

pub fn show(
    ui: &mut egui::Ui,
    timer: &Pomodoro,
    custom_work: &mut u16,
    custom_break: &mut u16,
) -> PomodoroResponse {
    let mut result = PomodoroResponse {
        profile_changed: None,
        action: Action::None,
    };
    theme::terminal_panel(ui, |ui| {
        theme::title(ui, "02", "FOCUS CYCLE");
        ui.horizontal(|ui| {
            for profile in [Profile::Classic, Profile::Deep] {
                if ui
                    .selectable_label(timer.profile() == profile, profile.name().to_uppercase())
                    .clicked()
                {
                    result.profile_changed = Some(profile);
                }
            }
            if ui
                .selectable_label(
                    matches!(timer.profile(), Profile::Custom { .. }),
                    "PERSONAL",
                )
                .clicked()
            {
                result.profile_changed = Profile::custom(*custom_work, *custom_break).ok();
            }
        });
        if matches!(timer.profile(), Profile::Custom { .. }) {
            ui.horizontal(|ui| {
                theme::label(ui, "TRABAJO");
                ui.add(
                    egui::DragValue::new(custom_work)
                        .range(1..=180)
                        .suffix(" min"),
                );
                theme::label(ui, "DESCANSO");
                ui.add(
                    egui::DragValue::new(custom_break)
                        .range(1..=180)
                        .suffix(" min"),
                );
                if ui.button("APLICAR").clicked() {
                    result.profile_changed = Profile::custom(*custom_work, *custom_break).ok();
                }
            });
        }
        ui.add_space(18.0);
        ui.separator();
        ui.add_space(12.0);
        let seconds = timer.remaining_seconds();
        let phase = match timer.stage() {
            Stage::Work => "TRABAJO ACTIVO",
            Stage::Break => "DESCANSO ACTIVO",
        };
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(phase).color(theme::ACCENT).strong());
                ui.label(
                    egui::RichText::new(format!("{:02}:{:02}", seconds / 60, seconds % 60))
                        .monospace()
                        .size(54.0)
                        .strong(),
                );
                theme::label(
                    ui,
                    format!(
                        "SESIONES HOY  //  {:02}",
                        timer.completed_work_sessions_today()
                    ),
                );
            });
            ui.add_space(28.0);
            ui.vertical(|ui| {
                let primary = match timer.state() {
                    TimerState::Stopped => "▶ INICIAR",
                    TimerState::WorkRunning | TimerState::BreakRunning => "Ⅱ PAUSAR",
                    TimerState::WorkPaused | TimerState::BreakPaused => "▶ REANUDAR",
                    TimerState::Finished => "→ SIGUIENTE",
                };
                let button = egui::Button::new(egui::RichText::new(primary).color(theme::INK))
                    .fill(theme::ACCENT);
                if ui.add_sized([160.0, 40.0], button).clicked() {
                    result.action = match timer.state() {
                        TimerState::Stopped
                        | TimerState::WorkRunning
                        | TimerState::BreakRunning => Action::StartPause,
                        TimerState::WorkPaused | TimerState::BreakPaused => Action::Resume,
                        TimerState::Finished => Action::Advance,
                    };
                }
                ui.horizontal(|ui| {
                    if ui.button("SKIP").clicked() {
                        result.action = Action::Skip;
                    }
                    if ui.button("RESET").clicked() {
                        result.action = Action::Reset;
                    }
                });
            });
        });
        ui.add_space(13.0);
        theme::meter(
            ui,
            1.0 - (seconds as f32
                / (timer.profile().duration(timer.stage()).as_secs().max(1) as f32)),
            32,
            theme::SIGNAL,
        );
    });
    result
}
