//! Indicador de bandeja para mantener Benteveo accesible desde el panel.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use image::imageops::FilterType;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

pub struct TrayService {
    status_tx: Sender<TrayStatus>,
    action_rx: Receiver<TrayAction>,
}

/// Vive por completo en el hilo de la bandeja porque `tray-icon` usa tipos
/// ligados al hilo donde fueron creados.
struct TrayBackend {
    _icon: TrayIcon,
    pomodoro: MenuItem,
    noise: MenuItem,
    tasks: MenuItem,
    timer_action: MenuItem,
    open: MenuItem,
    quit: MenuItem,
    last_status: Option<TrayStatus>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    ToggleTimer,
    Open,
    Quit,
}

#[derive(Clone, PartialEq, Eq)]
pub struct TrayStatus {
    pub pomodoro: String,
    pub noise: String,
    pub tasks: String,
    pub timer_action: String,
}

impl TrayService {
    /// Inicia la bandeja sin bloquear el hilo que dibuja la ventana.
    pub fn spawn() -> Result<Self, String> {
        let (status_tx, status_rx) = mpsc::channel();
        let (action_tx, action_rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("benteveo-tray".to_owned())
            .spawn(move || {
                if let Err(error) = run_tray(status_rx, action_tx) {
                    eprintln!("El indicador de Benteveo no está disponible: {error}");
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            status_tx,
            action_rx,
        })
    }

    pub fn update_status(&self, status: TrayStatus) {
        let _ = self.status_tx.send(status);
    }

    pub fn next_action(&self) -> Option<TrayAction> {
        self.action_rx.try_recv().ok()
    }
}

impl TrayBackend {
    fn new() -> Result<Self, String> {
        let icon = pixel_icon()?;
        let menu = Menu::new();
        let pomodoro = MenuItem::new("Pomodoro: cargando…", false, None);
        let noise = MenuItem::new("Ruido: cargando…", false, None);
        let tasks = MenuItem::new("Tareas pendientes: cargando…", false, None);
        let timer_action = MenuItem::new("Pomodoro", true, None);
        let open = MenuItem::new("Abrir Benteveo", true, None);
        let quit = MenuItem::new("Salir", true, None);
        menu.append(&pomodoro).map_err(|error| error.to_string())?;
        menu.append(&noise).map_err(|error| error.to_string())?;
        menu.append(&tasks).map_err(|error| error.to_string())?;
        menu.append(&PredefinedMenuItem::separator())
            .map_err(|error| error.to_string())?;
        menu.append(&timer_action)
            .map_err(|error| error.to_string())?;
        menu.append(&PredefinedMenuItem::separator())
            .map_err(|error| error.to_string())?;
        menu.append(&open).map_err(|error| error.to_string())?;
        menu.append(&PredefinedMenuItem::separator())
            .map_err(|error| error.to_string())?;
        menu.append(&quit).map_err(|error| error.to_string())?;
        let icon = TrayIconBuilder::new()
            .with_id("benteveo")
            .with_menu(Box::new(menu))
            .with_tooltip("Benteveo · Focus app")
            .with_icon(icon)
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            _icon: icon,
            pomodoro,
            noise,
            tasks,
            timer_action,
            open,
            quit,
            last_status: None,
        })
    }

    fn update_status(&mut self, status: TrayStatus) {
        if self.last_status.as_ref() == Some(&status) {
            return;
        }
        self.pomodoro.set_text(&status.pomodoro);
        self.noise.set_text(&status.noise);
        self.tasks.set_text(&status.tasks);
        self.timer_action.set_text(&status.timer_action);
        self.last_status = Some(status);
    }

    fn next_action(&self) -> Option<TrayAction> {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.timer_action.id() {
                return Some(TrayAction::ToggleTimer);
            }
            if event.id == self.open.id() {
                return Some(TrayAction::Open);
            }
            if event.id == self.quit.id() {
                return Some(TrayAction::Quit);
            }
        }
        None
    }
}

fn run_tray(status_rx: Receiver<TrayStatus>, action_tx: Sender<TrayAction>) -> Result<(), String> {
    // Esta creación puede esperar a D-Bus/StatusNotifier en algunos
    // escritorios. Al ocurrir aquí, nunca congela la ventana ni su cursor.
    let mut tray = TrayBackend::new()?;
    loop {
        match status_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(mut status) => {
                // Si la inicialización tardó, solo interesa el estado más
                // reciente; evita repintar estados viejos uno por uno.
                for newer in status_rx.try_iter() {
                    status = newer;
                }
                tray.update_status(status);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        while let Some(action) = tray.next_action() {
            if action_tx.send(action).is_err() {
                return Ok(());
            }
        }
    }
    Ok(())
}

fn pixel_icon() -> Result<Icon, String> {
    const TRAY_ICON_SIZE: u32 = 32;
    let image = image::load_from_memory(include_bytes!("../../resources/benteveo-pixel.png"))
        .map_err(|error| error.to_string())?
        .resize_exact(TRAY_ICON_SIZE, TRAY_ICON_SIZE, FilterType::Nearest)
        .into_rgba8();
    Icon::from_rgba(image.into_raw(), TRAY_ICON_SIZE, TRAY_ICON_SIZE)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_pixel_art_builds_a_small_tray_icon() {
        assert!(pixel_icon().is_ok());
    }
}
