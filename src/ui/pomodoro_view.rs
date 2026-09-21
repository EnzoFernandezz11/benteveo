use crate::domain::pomodoro::{Pomodoro, Profile, Stage, TimerState};

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
    ui.heading("// pomodoro");
    ui.horizontal(|ui| {
        for profile in [Profile::Classic, Profile::Deep] {
            if ui
                .selectable_label(timer.profile() == profile, profile.name())
                .clicked()
            {
                result.profile_changed = Some(profile);
            }
        }
        if ui
            .selectable_label(
                matches!(timer.profile(), Profile::Custom { .. }),
                "Personalizado",
            )
            .clicked()
        {
            result.profile_changed = Profile::custom(*custom_work, *custom_break).ok();
        }
    });
    if matches!(timer.profile(), Profile::Custom { .. }) {
        ui.horizontal(|ui| {
            ui.label("trabajo");
            ui.add(
                egui::DragValue::new(custom_work)
                    .range(1..=180)
                    .suffix(" min"),
            );
            ui.label("descanso");
            ui.add(
                egui::DragValue::new(custom_break)
                    .range(1..=180)
                    .suffix(" min"),
            );
            if ui.button("aplicar").clicked() {
                result.profile_changed = Profile::custom(*custom_work, *custom_break).ok();
            }
        });
    }
    ui.add_space(20.0);
    let seconds = timer.remaining_seconds();
    let phase = match timer.stage() {
        Stage::Work => "TRABAJO",
        Stage::Break => "DESCANSO",
    };
    ui.label(egui::RichText::new(phase).strong());
    ui.heading(format!("{:02}:{:02}", seconds / 60, seconds % 60));
    ui.label(format!(
        "sesiones de hoy: {}",
        timer.completed_work_sessions_today()
    ));
    ui.add_space(12.0);
    ui.horizontal(|ui| match timer.state() {
        TimerState::Stopped => {
            if ui.button("▶ iniciar").clicked() {
                result.action = Action::StartPause;
            }
        }
        TimerState::WorkRunning | TimerState::BreakRunning => {
            if ui.button("Ⅱ pausar").clicked() {
                result.action = Action::StartPause;
            }
        }
        TimerState::WorkPaused | TimerState::BreakPaused => {
            if ui.button("▶ reanudar").clicked() {
                result.action = Action::Resume;
            }
        }
        TimerState::Finished => {
            if ui.button("→ siguiente etapa").clicked() {
                result.action = Action::Advance;
            }
        }
    });
    ui.horizontal(|ui| {
        if ui.button("saltar etapa").clicked() {
            result.action = Action::Skip;
        }
        if ui.button("reiniciar").clicked() {
            result.action = Action::Reset;
        }
    });
    result
}
