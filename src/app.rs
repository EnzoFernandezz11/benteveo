use std::time::Duration;

use crate::config::{
    AppConfig, NoiseKind as StoredNoiseKind, PomodoroPhase, PomodoroProfile, TaskRecord,
};
use crate::domain::pomodoro::{Pomodoro, Profile, TimerEvent, TimerState};
use crate::domain::task::{Task, TaskId, TaskList};
use crate::services::{
    audio::{AudioService, NoiseKind},
    notifications,
    persistence::Persistence,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Noise,
    Pomodoro,
    Tasks,
}

pub struct FocusApp {
    tab: Tab,
    noise_kind: NoiseKind,
    volume: u8,
    audio: Option<AudioService>,
    timer: Pomodoro,
    custom_work: u16,
    custom_break: u16,
    tasks: TaskList,
    task_draft: String,
    persistence: Option<Persistence>,
    notice: Option<String>,
}

impl FocusApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::ui::theme::apply(&cc.egui_ctx);
        let (persistence, config, notice) = match Persistence::new() {
            Ok(service) => match service.load_with_report() {
                Ok(report) => (Some(service), report.config, report.recovered_from_corruption.then(|| "La configuración dañada fue respaldada; se restauraron valores predeterminados.".to_owned())),
                Err(error) => (Some(service), AppConfig::default(), Some(format!("No se pudo cargar la configuración: {error}"))),
            },
            Err(error) => (None, AppConfig::default(), Some(format!("Persistencia no disponible: {error}"))),
        };
        let custom_work = config.preferences.pomodoro.work_minutes.clamp(1, 180);
        let custom_break = config.preferences.pomodoro.break_minutes.clamp(1, 180);
        let profile = match config.preferences.pomodoro.profile {
            PomodoroProfile::Classic => Profile::Classic,
            PomodoroProfile::Deep => Profile::Deep,
            PomodoroProfile::Custom => {
                Profile::custom(custom_work, custom_break).unwrap_or(Profile::Classic)
            }
        };
        let mut timer = Pomodoro::new(profile).expect("perfil validado");
        timer.set_completed_work_sessions_today(config.pomodoro.sessions_completed_today);
        let tasks = TaskList::from_tasks(
            config
                .tasks
                .into_iter()
                .map(|record| Task {
                    id: TaskId::from_raw(record.id),
                    text: record.text,
                    completed: record.completed,
                    position: record.position,
                    created_at: record.created_at,
                })
                .collect(),
        );
        Self {
            tab: Tab::Noise,
            noise_kind: load_noise(config.preferences.noise.kind),
            volume: config.preferences.noise.volume.min(100),
            audio: None,
            timer,
            custom_work,
            custom_break,
            tasks,
            task_draft: String::new(),
            persistence,
            notice,
        }
    }

    fn save(&mut self) {
        let config = self.config();
        if let Some(persistence) = &self.persistence {
            if let Err(error) = persistence.save(&config) {
                self.notice = Some(format!("No se pudo guardar: {error}"));
            }
        }
    }

    fn config(&self) -> AppConfig {
        let profile = match self.timer.profile() {
            Profile::Classic => PomodoroProfile::Classic,
            Profile::Deep => PomodoroProfile::Deep,
            Profile::Custom { .. } => PomodoroProfile::Custom,
        };
        let phase = match self.timer.state() {
            TimerState::Stopped => PomodoroPhase::Stopped,
            TimerState::WorkRunning => PomodoroPhase::Work,
            TimerState::WorkPaused => PomodoroPhase::WorkPaused,
            TimerState::BreakRunning => PomodoroPhase::Break,
            TimerState::BreakPaused => PomodoroPhase::BreakPaused,
            TimerState::Finished => PomodoroPhase::Finished,
        };
        let mut config = AppConfig::default();
        config.preferences.noise.kind = store_noise(self.noise_kind);
        config.preferences.noise.volume = self.volume;
        config.preferences.noise.playing = false;
        config.preferences.pomodoro.profile = profile;
        config.preferences.pomodoro.work_minutes = self.custom_work;
        config.preferences.pomodoro.break_minutes = self.custom_break;
        config.pomodoro.phase = phase;
        config.pomodoro.remaining_seconds =
            self.timer.remaining_seconds().min(u64::from(u32::MAX)) as u32;
        config.pomodoro.sessions_completed_today = self.timer.completed_work_sessions_today();
        config.tasks = self
            .tasks
            .iter()
            .map(|task| TaskRecord {
                id: task.id.raw(),
                text: task.text.clone(),
                completed: task.completed,
                position: task.position,
                created_at: task.created_at,
            })
            .collect();
        config
    }

    fn toggle_audio(&mut self) {
        if let Some(audio) = &mut self.audio {
            if audio.is_playing() {
                audio.stop();
                self.save();
                return;
            }
        }
        if self.audio.is_none() {
            match AudioService::new(self.noise_kind, self.volume) {
                Ok(audio) => self.audio = Some(audio),
                Err(error) => {
                    self.notice = Some(format!("Audio no disponible: {error}"));
                    return;
                }
            }
        }
        if let Some(audio) = &mut self.audio {
            if let Err(error) = audio.play() {
                self.notice = Some(format!("No se pudo reproducir: {error}"));
            }
        }
        self.save();
    }

    fn update_audio_settings(&mut self) {
        if let Some(audio) = &mut self.audio {
            audio.set_volume(self.volume);
            if let Err(error) = audio.set_kind(self.noise_kind) {
                self.notice = Some(format!("No se pudo cambiar el ruido: {error}"));
            }
        }
        self.save();
    }

    fn handle_timer_event(&mut self, event: Option<TimerEvent>) {
        if let Some(TimerEvent::StageFinished(stage)) = event {
            let label = if matches!(stage, crate::domain::pomodoro::Stage::Work) {
                "Trabajo terminado"
            } else {
                "Descanso terminado"
            };
            if let Err(error) = notifications::stage_finished("Benteveo", label) {
                self.notice = Some(format!("{label}. Notificación no disponible: {error}"));
            } else {
                self.notice = Some(label.to_owned());
            }
            self.save();
        }
    }
}

impl eframe::App for FocusApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(250));
        let timer_event = self.timer.tick();
        self.handle_timer_event(timer_event);
        if ctx.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::Num1)) {
            self.tab = Tab::Noise;
        }
        if ctx.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::Num2)) {
            self.tab = Tab::Pomodoro;
        }
        if ctx.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::Num3)) {
            self.tab = Tab::Tasks;
        }
        if ctx.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::Q)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("BENTEVEO")
                        .color(crate::ui::theme::ACCENT)
                        .size(20.0)
                        .strong(),
                );
                ui.label(egui::RichText::new("/ FOCUS SYSTEM").color(crate::ui::theme::MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let remaining = self.timer.remaining_seconds();
                    ui.label(
                        egui::RichText::new(format!("{:02}:{:02}", remaining / 60, remaining % 60))
                            .color(crate::ui::theme::SIGNAL)
                            .strong(),
                    );
                    ui.label(egui::RichText::new("●").color(crate::ui::theme::SIGNAL));
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Noise, "01  NOISE");
                ui.selectable_value(&mut self.tab, Tab::Pomodoro, "02  FOCUS");
                ui.selectable_value(&mut self.tab, Tab::Tasks, "03  TASKS");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("CTRL+1 / 2 / 3")
                            .color(crate::ui::theme::MUTED)
                            .small(),
                    );
                });
            });
            ui.add_space(3.0);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(12.0);
            match self.tab {
                Tab::Noise => {
                    let response = crate::ui::noise_view::show(
                        ui,
                        &mut self.noise_kind,
                        &mut self.volume,
                        self.audio.as_ref().is_some_and(AudioService::is_playing),
                    );
                    if response.changed {
                        self.update_audio_settings();
                    }
                    if response.play_toggle {
                        self.toggle_audio();
                    }
                }
                Tab::Pomodoro => {
                    let response = crate::ui::pomodoro_view::show(
                        ui,
                        &self.timer,
                        &mut self.custom_work,
                        &mut self.custom_break,
                    );
                    if let Some(profile) = response.profile_changed {
                        if let Err(error) = self.timer.set_profile(profile) {
                            self.notice = Some(format!(
                                "Reiniciá el temporizador antes de cambiar el perfil: {error}"
                            ));
                        } else {
                            self.save();
                        }
                    }
                    use crate::ui::pomodoro_view::Action;
                    let event = match response.action {
                        Action::None => None,
                        Action::StartPause => {
                            if self.timer.is_running() {
                                self.timer.pause().ok()
                            } else {
                                self.timer.start().ok()
                            }
                        }
                        Action::Resume => self.timer.resume().ok(),
                        Action::Skip => Some(self.timer.skip_stage()),
                        Action::Reset => Some(self.timer.reset()),
                        Action::Advance => self.timer.advance_stage().ok(),
                    };
                    if event.is_some() {
                        self.save();
                    }
                }
                Tab::Tasks => {
                    let response =
                        crate::ui::tasks_view::show(ui, &mut self.tasks, &mut self.task_draft);
                    if response.changed {
                        self.save();
                    }
                }
            }
        });
        if let Some(message) = &self.notice {
            egui::TopBottomPanel::bottom("notice").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("// STATUS").color(crate::ui::theme::ACCENT));
                    ui.label(egui::RichText::new(message).color(crate::ui::theme::MUTED));
                });
            });
        }
    }
}

fn load_noise(kind: StoredNoiseKind) -> NoiseKind {
    match kind {
        StoredNoiseKind::White => NoiseKind::White,
        StoredNoiseKind::Pink => NoiseKind::Pink,
        StoredNoiseKind::Brown => NoiseKind::Brown,
    }
}
fn store_noise(kind: NoiseKind) -> StoredNoiseKind {
    match kind {
        NoiseKind::White => StoredNoiseKind::White,
        NoiseKind::Pink => StoredNoiseKind::Pink,
        NoiseKind::Brown => StoredNoiseKind::Brown,
    }
}
