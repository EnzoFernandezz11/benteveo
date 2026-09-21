//! Máquina de estados del temporizador Pomodoro.
//!
//! El reloj no depende de repintados de la interfaz: las operaciones que
//! cambian el estado tienen una variante `*_at(Instant)` para que la UI pueda
//! usar `Instant::now()` y las pruebas puedan usar un reloj determinista.

use std::fmt;
use std::time::{Duration, Instant};

/// Duración mínima permitida para cada etapa, en minutos.
pub const MIN_PROFILE_MINUTES: u16 = 1;
/// Duración máxima permitida para cada etapa, en minutos.
pub const MAX_PROFILE_MINUTES: u16 = 180;

/// Perfiles disponibles para el temporizador.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Profile {
    /// 25 minutos de trabajo y 5 de descanso.
    #[default]
    Classic,
    /// 90 minutos de trabajo y 10 de descanso.
    Deep,
    /// Duraciones definidas por el usuario, en minutos.
    Custom {
        work_minutes: u16,
        break_minutes: u16,
    },
}

impl Profile {
    /// Crea un perfil personalizado validando sus duraciones.
    pub fn custom(work_minutes: u16, break_minutes: u16) -> Result<Self, ProfileError> {
        let profile = Self::Custom {
            work_minutes,
            break_minutes,
        };
        profile.validate()?;
        Ok(profile)
    }

    /// Nombre estable para mostrar en la interfaz.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Classic => "Clásico",
            Self::Deep => "Profundo",
            Self::Custom { .. } => "Personalizado",
        }
    }

    /// Valida que las dos etapas estén dentro del rango del MVP.
    pub fn validate(self) -> Result<(), ProfileError> {
        let (work, break_minutes) = self.minutes();
        validate_minutes(work, Stage::Work)?;
        validate_minutes(break_minutes, Stage::Break)
    }

    /// Duración de trabajo del perfil.
    pub const fn work_minutes(self) -> u16 {
        match self {
            Self::Classic => 25,
            Self::Deep => 90,
            Self::Custom { work_minutes, .. } => work_minutes,
        }
    }

    /// Duración de descanso del perfil.
    pub const fn break_minutes(self) -> u16 {
        match self {
            Self::Classic => 5,
            Self::Deep => 10,
            Self::Custom { break_minutes, .. } => break_minutes,
        }
    }

    /// Duración de una etapa concreta.
    pub const fn duration(self, stage: Stage) -> Duration {
        Duration::from_secs(match stage {
            Stage::Work => self.work_minutes() as u64 * 60,
            Stage::Break => self.break_minutes() as u64 * 60,
        })
    }

    const fn minutes(self) -> (u16, u16) {
        (self.work_minutes(), self.break_minutes())
    }
}

fn validate_minutes(minutes: u16, stage: Stage) -> Result<(), ProfileError> {
    if !(MIN_PROFILE_MINUTES..=MAX_PROFILE_MINUTES).contains(&minutes) {
        return Err(match stage {
            Stage::Work => ProfileError::WorkDurationOutOfRange { minutes },
            Stage::Break => ProfileError::BreakDurationOutOfRange { minutes },
        });
    }
    Ok(())
}

/// Error producido al crear un perfil personalizado inválido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileError {
    WorkDurationOutOfRange { minutes: u16 },
    BreakDurationOutOfRange { minutes: u16 },
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkDurationOutOfRange { minutes } => write!(
                f,
                "la duración de trabajo ({minutes} min) debe estar entre {MIN_PROFILE_MINUTES} y {MAX_PROFILE_MINUTES} minutos"
            ),
            Self::BreakDurationOutOfRange { minutes } => write!(
                f,
                "la duración de descanso ({minutes} min) debe estar entre {MIN_PROFILE_MINUTES} y {MAX_PROFILE_MINUTES} minutos"
            ),
        }
    }
}

impl std::error::Error for ProfileError {}

/// Parte del ciclo que está transcurriendo o lista para comenzar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Work,
    Break,
}

impl Stage {
    const fn next(self) -> Self {
        match self {
            Self::Work => Self::Break,
            Self::Break => Self::Work,
        }
    }
}

/// Estados observables del temporizador.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    Stopped,
    WorkRunning,
    WorkPaused,
    BreakRunning,
    BreakPaused,
    Finished,
}

impl TimerState {
    pub const fn is_running(self) -> bool {
        matches!(self, Self::WorkRunning | Self::BreakRunning)
    }

    pub const fn is_paused(self) -> bool {
        matches!(self, Self::WorkPaused | Self::BreakPaused)
    }
}

/// Fallo de una transición solicitada por la UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerError {
    InvalidProfile(ProfileError),
    AlreadyRunning,
    NotRunning,
    NotPaused,
    CannotStartFromFinished,
}

impl fmt::Display for TimerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProfile(error) => fmt::Display::fmt(error, f),
            Self::AlreadyRunning => f.write_str("el temporizador ya está en curso"),
            Self::NotRunning => f.write_str("el temporizador no está en curso"),
            Self::NotPaused => f.write_str("el temporizador no está pausado"),
            Self::CannotStartFromFinished => {
                f.write_str("una etapa terminada debe avanzar a la siguiente antes de iniciar")
            }
        }
    }
}

impl std::error::Error for TimerError {}

impl From<ProfileError> for TimerError {
    fn from(error: ProfileError) -> Self {
        Self::InvalidProfile(error)
    }
}

/// Hechos que pueden interesar a la UI, notificaciones o persistencia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerEvent {
    Started(Stage),
    Paused(Stage),
    Resumed(Stage),
    StageFinished(Stage),
    StageAdvanced(Stage),
    Reset,
}

/// Máquina de estados Pomodoro sin dependencias externas.
#[derive(Debug, Clone)]
pub struct Pomodoro {
    profile: Profile,
    state: TimerState,
    stage: Stage,
    remaining: Duration,
    started_at: Option<Instant>,
    completed_work_sessions_today: u32,
}

impl Default for Pomodoro {
    fn default() -> Self {
        Self::new(Profile::default()).expect("el perfil clásico siempre es válido")
    }
}

impl Pomodoro {
    /// Crea un temporizador detenido y listo para iniciar trabajo.
    pub fn new(profile: Profile) -> Result<Self, ProfileError> {
        profile.validate()?;
        let stage = Stage::Work;
        Ok(Self {
            profile,
            state: TimerState::Stopped,
            stage,
            remaining: profile.duration(stage),
            started_at: None,
            completed_work_sessions_today: 0,
        })
    }

    pub fn profile(&self) -> Profile {
        self.profile
    }

    /// Cambia el perfil. Para evitar alterar una etapa en curso, sólo se
    /// permite hacerlo cuando el temporizador está detenido.
    pub fn set_profile(&mut self, profile: Profile) -> Result<(), TimerError> {
        profile.validate()?;
        if self.state != TimerState::Stopped {
            return Err(TimerError::AlreadyRunning);
        }
        self.profile = profile;
        self.stage = Stage::Work;
        self.remaining = profile.duration(self.stage);
        Ok(())
    }

    pub fn state(&self) -> TimerState {
        self.state
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn completed_work_sessions_today(&self) -> u32 {
        self.completed_work_sessions_today
    }

    /// Permite restaurar el contador diario cargado desde persistencia.
    pub fn set_completed_work_sessions_today(&mut self, count: u32) {
        self.completed_work_sessions_today = count;
    }

    pub fn is_running(&self) -> bool {
        self.state.is_running()
    }

    /// Tiempo restante según el instante dado, sin mutar el temporizador.
    pub fn remaining_at(&self, now: Instant) -> Duration {
        match self.started_at {
            Some(start) if self.state.is_running() => self
                .remaining
                .saturating_sub(now.saturating_duration_since(start)),
            _ => self.remaining,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn remaining(&self) -> Duration {
        self.remaining_at(Instant::now())
    }

    pub fn remaining_seconds_at(&self, now: Instant) -> u64 {
        self.remaining_at(now).as_secs()
    }

    pub fn remaining_seconds(&self) -> u64 {
        self.remaining_seconds_at(Instant::now())
    }

    /// Inicia la etapa actual. En una etapa terminada se debe llamar antes a
    /// `advance_stage_at` para no repetir accidentalmente la etapa anterior.
    pub fn start(&mut self) -> Result<TimerEvent, TimerError> {
        self.start_at(Instant::now())
    }

    pub fn start_at(&mut self, now: Instant) -> Result<TimerEvent, TimerError> {
        match self.state {
            TimerState::Stopped => {
                self.started_at = Some(now);
                self.state = running_state(self.stage);
                Ok(TimerEvent::Started(self.stage))
            }
            TimerState::WorkRunning | TimerState::BreakRunning => Err(TimerError::AlreadyRunning),
            TimerState::WorkPaused | TimerState::BreakPaused => Err(TimerError::NotPaused),
            TimerState::Finished => Err(TimerError::CannotStartFromFinished),
        }
    }

    pub fn pause(&mut self) -> Result<TimerEvent, TimerError> {
        self.pause_at(Instant::now())
    }

    pub fn pause_at(&mut self, now: Instant) -> Result<TimerEvent, TimerError> {
        if !self.state.is_running() {
            return Err(TimerError::NotRunning);
        }
        self.remaining = self.remaining_at(now);
        if self.remaining.is_zero() {
            let stage = self.stage;
            self.finish_current_stage();
            return Ok(TimerEvent::StageFinished(stage));
        }
        let stage = self.stage;
        self.state = paused_state(stage);
        self.started_at = None;
        Ok(TimerEvent::Paused(stage))
    }

    pub fn resume(&mut self) -> Result<TimerEvent, TimerError> {
        self.resume_at(Instant::now())
    }

    pub fn resume_at(&mut self, now: Instant) -> Result<TimerEvent, TimerError> {
        if !self.state.is_paused() {
            return Err(TimerError::NotPaused);
        }
        self.started_at = Some(now);
        self.state = running_state(self.stage);
        Ok(TimerEvent::Resumed(self.stage))
    }

    /// Actualiza el estado y devuelve un evento cuando una etapa termina.
    pub fn tick(&mut self) -> Option<TimerEvent> {
        self.tick_at(Instant::now())
    }

    pub fn tick_at(&mut self, now: Instant) -> Option<TimerEvent> {
        if !self.state.is_running() || !self.remaining_at(now).is_zero() {
            return None;
        }
        self.remaining = Duration::ZERO;
        self.started_at = None;
        self.state = TimerState::Finished;
        if self.stage == Stage::Work {
            self.completed_work_sessions_today =
                self.completed_work_sessions_today.saturating_add(1);
        }
        Some(TimerEvent::StageFinished(self.stage))
    }

    /// Cambia a la siguiente etapa y la deja detenida, lista para iniciar.
    pub fn skip_stage(&mut self) -> TimerEvent {
        self.skip_stage_at(Instant::now())
    }

    pub fn skip_stage_at(&mut self, _now: Instant) -> TimerEvent {
        self.started_at = None;
        self.stage = self.stage.next();
        self.remaining = self.profile.duration(self.stage);
        self.state = TimerState::Stopped;
        TimerEvent::StageAdvanced(self.stage)
    }

    /// Avanza a la siguiente etapa y la inicia inmediatamente.
    pub fn advance_stage(&mut self) -> Result<TimerEvent, TimerError> {
        self.advance_stage_at(Instant::now())
    }

    pub fn advance_stage_at(&mut self, now: Instant) -> Result<TimerEvent, TimerError> {
        self.skip_stage_at(now);
        self.start_at(now)
    }

    /// Reinicia el ciclo desde una etapa de trabajo detenida.
    pub fn reset(&mut self) -> TimerEvent {
        self.reset_at(Instant::now())
    }

    pub fn reset_at(&mut self, _now: Instant) -> TimerEvent {
        self.started_at = None;
        self.stage = Stage::Work;
        self.remaining = self.profile.duration(self.stage);
        self.state = TimerState::Stopped;
        TimerEvent::Reset
    }

    fn finish_current_stage(&mut self) {
        self.remaining = Duration::ZERO;
        self.started_at = None;
        self.state = TimerState::Finished;
        if self.stage == Stage::Work {
            self.completed_work_sessions_today =
                self.completed_work_sessions_today.saturating_add(1);
        }
    }
}

const fn running_state(stage: Stage) -> TimerState {
    match stage {
        Stage::Work => TimerState::WorkRunning,
        Stage::Break => TimerState::BreakRunning,
    }
}

const fn paused_state(stage: Stage) -> TimerState {
    match stage {
        Stage::Work => TimerState::WorkPaused,
        Stage::Break => TimerState::BreakPaused,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Profile {
        Profile::custom(1, 2).expect("valid test profile")
    }

    #[test]
    fn default_profiles_have_plan_durations() {
        assert_eq!(Profile::Classic.work_minutes(), 25);
        assert_eq!(Profile::Classic.break_minutes(), 5);
        assert_eq!(Profile::Deep.work_minutes(), 90);
        assert_eq!(Profile::Deep.break_minutes(), 10);
    }

    #[test]
    fn custom_profile_validates_both_bounds() {
        assert!(Profile::custom(1, 180).is_ok());
        assert_eq!(
            Profile::custom(0, 5),
            Err(ProfileError::WorkDurationOutOfRange { minutes: 0 })
        );
        assert_eq!(
            Profile::custom(5, 181),
            Err(ProfileError::BreakDurationOutOfRange { minutes: 181 })
        );
    }

    #[test]
    fn start_and_finish_work_use_simulated_instants() {
        let origin = Instant::now();
        let mut timer = Pomodoro::new(profile()).unwrap();
        assert_eq!(timer.state(), TimerState::Stopped);
        assert_eq!(timer.start_at(origin), Ok(TimerEvent::Started(Stage::Work)));
        assert_eq!(
            timer.remaining_at(origin + Duration::from_secs(59)),
            Duration::from_secs(1)
        );
        assert_eq!(
            timer.tick_at(origin + Duration::from_secs(60)),
            Some(TimerEvent::StageFinished(Stage::Work))
        );
        assert_eq!(timer.state(), TimerState::Finished);
        assert_eq!(timer.completed_work_sessions_today(), 1);
        assert_eq!(timer.tick_at(origin + Duration::from_secs(61)), None);
    }

    #[test]
    fn pause_and_resume_do_not_count_paused_time() {
        let origin = Instant::now();
        let mut timer = Pomodoro::new(profile()).unwrap();
        timer.start_at(origin).unwrap();
        timer.pause_at(origin + Duration::from_secs(20)).unwrap();
        assert_eq!(timer.state(), TimerState::WorkPaused);
        assert_eq!(timer.remaining(), Duration::from_secs(40));
        timer.resume_at(origin + Duration::from_secs(120)).unwrap();
        assert_eq!(
            timer.remaining_at(origin + Duration::from_secs(150)),
            Duration::from_secs(10)
        );
        assert_eq!(
            timer.tick_at(origin + Duration::from_secs(160)),
            Some(TimerEvent::StageFinished(Stage::Work))
        );
    }

    #[test]
    fn advancing_stage_starts_break_and_reset_returns_to_work() {
        let origin = Instant::now();
        let mut timer = Pomodoro::new(profile()).unwrap();
        timer.start_at(origin).unwrap();
        timer.tick_at(origin + Duration::from_secs(60));
        assert_eq!(
            timer.advance_stage_at(origin + Duration::from_secs(60)),
            Ok(TimerEvent::Started(Stage::Break))
        );
        assert_eq!(timer.state(), TimerState::BreakRunning);
        assert_eq!(
            timer.remaining_at(origin + Duration::from_secs(61)),
            Duration::from_secs(119)
        );
        assert_eq!(
            timer.reset_at(origin + Duration::from_secs(61)),
            TimerEvent::Reset
        );
        assert_eq!(timer.state(), TimerState::Stopped);
        assert_eq!(timer.stage(), Stage::Work);
        assert_eq!(timer.remaining(), Duration::from_secs(60));
    }

    #[test]
    fn skip_from_running_does_not_complete_a_session() {
        let origin = Instant::now();
        let mut timer = Pomodoro::new(profile()).unwrap();
        timer.start_at(origin).unwrap();
        assert_eq!(
            timer.skip_stage_at(origin + Duration::from_secs(10)),
            TimerEvent::StageAdvanced(Stage::Break)
        );
        assert_eq!(timer.state(), TimerState::Stopped);
        assert_eq!(timer.completed_work_sessions_today(), 0);
    }

    #[test]
    fn profile_change_is_allowed_only_when_stopped() {
        let origin = Instant::now();
        let mut timer = Pomodoro::new(Profile::Classic).unwrap();
        timer.set_profile(Profile::Deep).unwrap();
        assert_eq!(timer.profile(), Profile::Deep);
        timer.start_at(origin).unwrap();
        assert_eq!(
            timer.set_profile(Profile::Classic),
            Err(TimerError::AlreadyRunning)
        );
    }
}
