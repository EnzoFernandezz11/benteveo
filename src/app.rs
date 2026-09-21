use std::time::Duration;

use crate::config::{
    AppConfig, NoiseKind as StoredNoiseKind, PomodoroPhase, PomodoroProfile, TaskRecord,
};
use crate::domain::pomodoro::{Pomodoro, Profile, TimerEvent, TimerState};
use crate::domain::task::{Task, TaskId, TaskList};
use crate::services::{
    audio::{AudioService, CompletionSound, NoiseKind},
    notifications,
    persistence::Persistence,
    tray::{TrayAction, TrayService, TrayStatus},
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
    completion_sound: Option<CompletionSound>,
    tray: Option<TrayService>,
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
        let tray = match TrayService::new() {
            Ok(tray) => Some(tray),
            Err(error) => {
                eprintln!("El indicador de Benteveo no está disponible: {error}");
                None
            }
        };
        Self {
            tab: Tab::Noise,
            noise_kind: load_noise(config.preferences.noise.kind),
            volume: config.preferences.noise.volume.min(100),
            audio: None,
            completion_sound: None,
            tray,
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
            let sound_error = match CompletionSound::play_benteveo_call() {
                Ok(sound) => {
                    self.completion_sound = Some(sound);
                    None
                }
                Err(error) => Some(error),
            };
            let notification_error = notifications::stage_finished("Benteveo", label).err();
            self.notice = match (sound_error, notification_error) {
                (None, None) => Some(label.to_owned()),
                (Some(error), None) => Some(format!("{label}. Sonido no disponible: {error}")),
                (None, Some(error)) => {
                    Some(format!("{label}. Notificación no disponible: {error}"))
                }
                (Some(sound), Some(notification)) => Some(format!(
                    "{label}. Sonido no disponible: {sound}. Notificación no disponible: {notification}"
                )),
            };
            self.save();
        }
    }

    fn handle_tray_actions(&mut self, ctx: &egui::Context) {
        let status = self.tray_status();
        if let Some(tray) = &mut self.tray {
            tray.update_status(status);
        }
        let action = self.tray.as_ref().and_then(TrayService::next_action);
        match action {
            Some(TrayAction::ToggleTimer) => self.toggle_timer_from_tray(),
            Some(TrayAction::Open) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            Some(TrayAction::Quit) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            None => {}
        }
    }

    fn tray_status(&self) -> TrayStatus {
        use crate::domain::pomodoro::Stage;
        let stage = match self.timer.stage() {
            Stage::Work => "Trabajo",
            Stage::Break => "Descanso",
        };
        let state = match self.timer.state() {
            TimerState::Stopped => "listo",
            TimerState::WorkRunning | TimerState::BreakRunning => "en curso",
            TimerState::WorkPaused | TimerState::BreakPaused => "pausado",
            TimerState::Finished => "terminado",
        };
        let remaining = self.timer.remaining_seconds();
        let noise = if self.audio.as_ref().is_some_and(AudioService::is_playing) {
            format!("Ruido: {} · {}%", self.noise_kind.label(), self.volume)
        } else {
            "Ruido: apagado".to_owned()
        };
        let timer_action = match self.timer.state() {
            TimerState::WorkRunning | TimerState::BreakRunning => "Pausar Pomodoro",
            TimerState::WorkPaused | TimerState::BreakPaused => "Reanudar Pomodoro",
            TimerState::Stopped => "Iniciar Pomodoro",
            TimerState::Finished => "Iniciar siguiente etapa",
        };
        TrayStatus {
            pomodoro: format!(
                "Pomodoro: {stage} · {:02}:{:02} · {state}",
                remaining / 60,
                remaining % 60
            ),
            noise,
            tasks: format!(
                "Tareas pendientes: {}",
                self.tasks.iter().filter(|task| !task.completed).count()
            ),
            timer_action: timer_action.to_owned(),
        }
    }

    fn toggle_timer_from_tray(&mut self) {
        let event = match self.timer.state() {
            TimerState::WorkRunning | TimerState::BreakRunning => self.timer.pause().ok(),
            TimerState::WorkPaused | TimerState::BreakPaused => self.timer.resume().ok(),
            TimerState::Stopped => self.timer.start().ok(),
            TimerState::Finished => self.timer.advance_stage().ok(),
        };
        if event.is_some() {
            self.save();
        }
    }
}

impl eframe::App for FocusApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let needs_fast_refresh = self.timer.is_running()
            || self
                .completion_sound
                .as_ref()
                .is_some_and(|sound| !sound.is_finished());
        ctx.request_repaint_after(Duration::from_millis(if needs_fast_refresh {
            250
        } else {
            1_000
        }));
        self.handle_tray_actions(ctx);
        if self
            .completion_sound
            .as_ref()
            .is_some_and(CompletionSound::is_finished)
        {
            self.completion_sound = None;
        }
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
                    egui::RichText::new(
                        r" ____  _____ _   _ _____ _______     _______ ___
| __ )| ____| \ | |_   _| ____\ \   / / ____/ _ \
|  _ \|  _| |  \| | | | |  _|  \ \ / /|  _|| | | |
| |_) | |___| |\  | | | | |___  \ V / | |__| |_| |
|____/|_____|_| \_| |_| |_____|  \_/  |_____\___/",
                    )
                    .color(crate::ui::theme::ACCENT)
                    .monospace()
                    .size(10.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let remaining = self.timer.remaining_seconds();
                    ui.label(
                        egui::RichText::new(format!("{:02}:{:02}", remaining / 60, remaining % 60))
                            .color(crate::ui::theme::SIGNAL)
                            .size(18.0)
                            .strong(),
                    );
                    ui.label(egui::RichText::new("●  WORK").color(crate::ui::theme::SIGNAL));
                });
            });
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("// FOCUS SYSTEM  ·  LOCAL FIRST  ·  v0.1")
                        .color(crate::ui::theme::MUTED)
                        .small(),
                );
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
