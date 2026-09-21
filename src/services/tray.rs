//! Indicador de bandeja para mantener Benteveo accesible desde el panel.

use image::imageops::FilterType;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

pub struct TrayService {
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
    pub fn new() -> Result<Self, String> {
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

    pub fn update_status(&mut self, status: TrayStatus) {
        if self.last_status.as_ref() == Some(&status) {
            return;
        }
        self.pomodoro.set_text(&status.pomodoro);
        self.noise.set_text(&status.noise);
        self.tasks.set_text(&status.tasks);
        self.timer_action.set_text(&status.timer_action);
        self.last_status = Some(status);
    }

    pub fn next_action(&self) -> Option<TrayAction> {
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
