//! Tipos que se guardan en el archivo de configuración de Benteveo.
//!
//! Este módulo contiene únicamente datos serializables. La lógica de carga y
//! guardado vive en [`crate::services::persistence`], lo que permite usar los
//! modelos desde la UI y desde las pruebas sin tocar el sistema de archivos.

use serde::{Deserialize, Serialize};

/// Versión del formato de configuración actualmente soportada.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Preferencias y estado persistente de la aplicación.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    /// Versión del documento, no de la aplicación.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub preferences: Preferences,
    /// Estado relevante del temporizador que se debe recuperar al abrir.
    #[serde(default)]
    pub pomodoro: PomodoroState,
    /// Las tareas se mantienen como registros de almacenamiento estables.
    /// El dominio puede convertirlos a su tipo rico sin acoplarse a serde.
    #[serde(default)]
    pub tasks: Vec<TaskRecord>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            preferences: Preferences::default(),
            pomodoro: PomodoroState::default(),
            tasks: Vec::new(),
        }
    }
}

impl AppConfig {
    /// Completa el número de esquema de documentos antiguos y valida los
    /// formatos que esta versión de la aplicación entiende.
    pub fn migrate(mut self) -> Result<Self, UnsupportedSchemaVersion> {
        match self.schema_version {
            0 => self.schema_version = CURRENT_SCHEMA_VERSION,
            CURRENT_SCHEMA_VERSION => {}
            version => return Err(UnsupportedSchemaVersion { version }),
        }
        Ok(self)
    }
}

fn default_schema_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}

/// Preferencias agrupadas por funcionalidad.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default)]
    pub noise: NoisePreferences,
    #[serde(default)]
    pub pomodoro: PomodoroPreferences,
    #[serde(default)]
    pub tasks: TaskPreferences,
}

/// Preferencias del generador de ruido.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoisePreferences {
    #[serde(default)]
    pub kind: NoiseKind,
    /// Volumen porcentual entre 0 y 100, igual que el control de audio.
    #[serde(default = "default_volume")]
    pub volume: u8,
    /// Se recuerda el control, pero no se inicia audio automáticamente.
    #[serde(default)]
    pub playing: bool,
}

impl Default for NoisePreferences {
    fn default() -> Self {
        Self {
            kind: NoiseKind::White,
            volume: default_volume(),
            playing: false,
        }
    }
}

fn default_volume() -> u8 {
    50
}

/// Tipos de ruido soportados por el MVP.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NoiseKind {
    #[default]
    White,
    Pink,
    Brown,
}

/// Preferencias para crear nuevas sesiones Pomodoro.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PomodoroPreferences {
    #[serde(default)]
    pub profile: PomodoroProfile,
    /// Duración personalizada de trabajo, en minutos.
    #[serde(default = "default_work_minutes")]
    pub work_minutes: u16,
    /// Duración personalizada de descanso, en minutos.
    #[serde(default = "default_break_minutes")]
    pub break_minutes: u16,
    #[serde(default)]
    pub auto_start_break: bool,
    #[serde(default)]
    pub auto_start_work: bool,
}

impl Default for PomodoroPreferences {
    fn default() -> Self {
        Self {
            profile: PomodoroProfile::Classic,
            work_minutes: default_work_minutes(),
            break_minutes: default_break_minutes(),
            auto_start_break: false,
            auto_start_work: false,
        }
    }
}

fn default_work_minutes() -> u16 {
    25
}

fn default_break_minutes() -> u16 {
    5
}

/// Perfiles integrados o configuración editable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PomodoroProfile {
    #[default]
    Classic,
    Deep,
    Custom,
}

/// Preferencias visuales y de comportamiento de la lista de tareas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPreferences {
    #[serde(default = "default_true")]
    pub show_completed: bool,
}

impl Default for TaskPreferences {
    fn default() -> Self {
        Self {
            show_completed: true,
        }
    }
}

fn default_true() -> bool {
    true
}

/// Estado del temporizador que es seguro recuperar entre ejecuciones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PomodoroState {
    #[serde(default)]
    pub phase: PomodoroPhase,
    #[serde(default)]
    pub running: bool,
    #[serde(default)]
    pub remaining_seconds: u32,
    #[serde(default)]
    pub sessions_completed_today: u32,
    #[serde(default)]
    pub session_date: Option<String>,
}

impl Default for PomodoroState {
    fn default() -> Self {
        Self {
            phase: PomodoroPhase::Stopped,
            running: false,
            remaining_seconds: 0,
            sessions_completed_today: 0,
            session_date: None,
        }
    }
}

/// Fases persistibles del temporizador. Las variantes pausadas conservan la
/// distinción necesaria para que el dominio pueda reanudar correctamente.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PomodoroPhase {
    #[default]
    Stopped,
    Work,
    WorkPaused,
    Break,
    BreakPaused,
    Finished,
}

/// Representación de una tarea en el archivo JSON.
///
/// Se usan tipos primitivos deliberadamente: así el dominio puede envolver el
/// identificador y el instante en tipos propios sin acoplar el almacenamiento.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRecord {
    /// Valor numérico del `domain::task::TaskId`.
    pub id: u64,
    pub text: String,
    #[serde(default)]
    pub completed: bool,
    #[serde(default)]
    pub position: usize,
    /// Segundos desde Unix epoch, igual que el modelo de dominio actual.
    pub created_at: u64,
}

/// Error de validación para documentos de una versión futura.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedSchemaVersion {
    pub version: u32,
}

impl std::fmt::Display for UnsupportedSchemaVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "unsupported configuration schema version {}",
            self.version
        )
    }
}

impl std::error::Error for UnsupportedSchemaVersion {}
