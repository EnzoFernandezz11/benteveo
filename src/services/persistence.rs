//! Persistencia local versionada y atómica.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use directories::ProjectDirs;

use crate::config::{AppConfig, UnsupportedSchemaVersion};

const APPLICATION_QUALIFIER: &str = "com";
const APPLICATION_ORGANIZATION: &str = "Benteveo";
const APPLICATION_NAME: &str = "Benteveo";
const CONFIG_FILE_NAME: &str = "config.json";

/// Resultado detallado de una carga. `load` expone sólo la configuración para
/// el camino normal, mientras que este tipo permite mostrar un aviso no
/// bloqueante en la UI cuando se recuperó un archivo corrupto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadReport {
    pub config: AppConfig,
    pub recovered_from_corruption: bool,
    pub backup_path: Option<PathBuf>,
}

/// Servicio que encapsula el archivo persistente de la aplicación.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Persistence {
    path: PathBuf,
}

impl Persistence {
    /// Usa el directorio de configuración estándar de Linux.
    pub fn new() -> Result<Self, PersistenceError> {
        let project_dirs = ProjectDirs::from(
            APPLICATION_QUALIFIER,
            APPLICATION_ORGANIZATION,
            APPLICATION_NAME,
        )
        .ok_or(PersistenceError::NoConfigDirectory)?;
        Ok(Self::from_path(
            project_dirs.config_dir().join(CONFIG_FILE_NAME),
        ))
    }

    /// Construye el servicio con una ruta explícita. Es útil para pruebas y
    /// permite a la aplicación inyectar una ubicación alternativa.
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    #[allow(dead_code)] // Útil para diagnósticos y pruebas de integración.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Carga la configuración; si no existe, devuelve valores por defecto.
    /// Un JSON corrupto se aparta como respaldo y también devuelve defaults.
    #[allow(dead_code)] // Atajo de API para consumidores sin reporte detallado.
    pub fn load(&self) -> Result<AppConfig, PersistenceError> {
        Ok(self.load_with_report()?.config)
    }

    pub fn load_with_report(&self) -> Result<LoadReport, PersistenceError> {
        if !self.path.exists() {
            return Ok(LoadReport {
                config: AppConfig::default(),
                recovered_from_corruption: false,
                backup_path: None,
            });
        }

        let bytes = fs::read(&self.path)?;
        match serde_json::from_slice::<AppConfig>(&bytes) {
            Ok(config) => match config.migrate() {
                Ok(config) => Ok(LoadReport {
                    config,
                    recovered_from_corruption: false,
                    backup_path: None,
                }),
                Err(error) => Err(PersistenceError::UnsupportedSchema(error)),
            },
            Err(_error) => {
                let backup = self.preserve_corrupt_file()?;
                Ok(LoadReport {
                    config: AppConfig::default(),
                    recovered_from_corruption: true,
                    backup_path: Some(backup),
                })
            }
        }
    }

    /// Escribe JSON con un archivo temporal en el mismo directorio y luego lo
    /// reemplaza mediante rename, que es atómico dentro del mismo filesystem.
    pub fn save(&self, config: &AppConfig) -> Result<(), PersistenceError> {
        let mut normalized = config.clone();
        normalized.schema_version = crate::config::CURRENT_SCHEMA_VERSION;
        let data = serde_json::to_vec_pretty(&normalized)?;

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temporary_path = self.temporary_path();
        let write_result = (|| -> Result<(), PersistenceError> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary_path)?;
            file.write_all(&data)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary_path, &self.path)?;
            sync_parent_directory(self.path.parent())?;
            Ok(())
        })();

        if write_result.is_err() {
            // Sólo se elimina nuestro archivo temporal concreto. Si la
            // escritura falló antes de crearlo, no hay nada que modificar.
            let _ = fs::remove_file(&temporary_path);
        }
        write_result
    }

    fn temporary_path(&self) -> PathBuf {
        let suffix = unique_suffix();
        let file_name = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(CONFIG_FILE_NAME);
        self.path
            .with_file_name(format!(".{file_name}.{suffix}.tmp"))
    }

    fn preserve_corrupt_file(&self) -> Result<PathBuf, PersistenceError> {
        let mut index = 0;
        loop {
            let backup = backup_path_for(&self.path, index);
            if !backup.exists() {
                // rename conserva los bytes sin una ventana en la que otro
                // proceso pueda observar un archivo parcialmente escrito.
                fs::rename(&self.path, &backup)?;
                return Ok(backup);
            }
            index += 1;
        }
    }
}

fn backup_path_for(path: &Path, index: u32) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(CONFIG_FILE_NAME);
    if index == 0 {
        path.with_file_name(format!("{file_name}.bak"))
    } else {
        path.with_file_name(format!("{file_name}.bak.{index}"))
    }
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{}-{}", std::process::id(), nanos)
}

fn sync_parent_directory(parent: Option<&Path>) -> io::Result<()> {
    if let Some(parent) = parent {
        // Linux permite abrir un directorio y sincronizar su entrada. En
        // plataformas donde no aplica, la escritura del archivo sigue siendo
        // correcta y se ignora sólo el error de operación no soportada.
        match File::open(parent).and_then(|directory| directory.sync_all()) {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::Unsupported | io::ErrorKind::PermissionDenied
                ) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

#[derive(Debug)]
pub enum PersistenceError {
    Io(io::Error),
    Json(serde_json::Error),
    UnsupportedSchema(UnsupportedSchemaVersion),
    NoConfigDirectory,
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "persistence I/O error: {error}"),
            Self::Json(error) => write!(formatter, "invalid configuration JSON: {error}"),
            Self::UnsupportedSchema(error) => error.fmt(formatter),
            Self::NoConfigDirectory => {
                formatter.write_str("no standard configuration directory available")
            }
        }
    }
}

impl std::error::Error for PersistenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::UnsupportedSchema(error) => Some(error),
            Self::NoConfigDirectory => None,
        }
    }
}

impl From<io::Error> for PersistenceError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for PersistenceError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{NoiseKind, PomodoroProfile};

    fn temp_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!("benteveo-persistence-{}", unique_suffix()));
        fs::create_dir_all(&path).expect("create temporary directory");
        path
    }

    #[test]
    fn missing_file_returns_defaults() {
        let directory = temp_dir();
        let persistence = Persistence::from_path(directory.join(CONFIG_FILE_NAME));
        let config = persistence.load().expect("defaults should load");
        assert_eq!(config, AppConfig::default());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn save_and_load_round_trip_is_versioned() {
        let directory = temp_dir();
        let persistence = Persistence::from_path(directory.join(CONFIG_FILE_NAME));
        let mut config = AppConfig::default();
        config.preferences.noise.kind = NoiseKind::Pink;
        config.preferences.pomodoro.profile = PomodoroProfile::Deep;
        config.tasks.push(crate::config::TaskRecord {
            id: 1,
            text: "Leer PLAN.md".into(),
            completed: false,
            position: 0,
            created_at: 1_789_862_400,
        });
        persistence.save(&config).expect("save config");
        let loaded = persistence.load().expect("load config");
        assert_eq!(loaded, config);
        let text = fs::read_to_string(persistence.path()).expect("read config");
        assert!(text.contains("\"schema_version\": 1"));
        assert!(!fs::read_dir(&directory)
            .expect("read temporary directory")
            .any(|entry| entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn corrupt_file_is_backed_up_and_defaults_are_returned() {
        let directory = temp_dir();
        let path = directory.join(CONFIG_FILE_NAME);
        fs::write(&path, b"{ definitely not json").expect("write corrupt config");
        let persistence = Persistence::from_path(&path);
        let report = persistence
            .load_with_report()
            .expect("recover corrupt config");
        assert!(report.recovered_from_corruption);
        let backup = report.backup_path.expect("backup path");
        assert!(backup.exists());
        assert_eq!(
            fs::read(backup).expect("read backup"),
            b"{ definitely not json"
        );
        assert_eq!(report.config, AppConfig::default());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn old_schema_is_migrated_and_future_schema_is_rejected() {
        let directory = temp_dir();
        let path = directory.join(CONFIG_FILE_NAME);
        fs::write(&path, br#"{"schema_version":0,"tasks":[]}"#).expect("write old config");
        let persistence = Persistence::from_path(&path);
        assert_eq!(
            persistence
                .load()
                .expect("migrate old config")
                .schema_version,
            1
        );
        fs::write(&path, br#"{"schema_version":99}"#).expect("write future config");
        assert!(matches!(
            persistence.load(),
            Err(PersistenceError::UnsupportedSchema(
                UnsupportedSchemaVersion { version: 99 }
            ))
        ));
        let _ = fs::remove_dir_all(directory);
    }
}
