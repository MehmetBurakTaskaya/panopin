mod commands;
mod error;
mod hotkeys;
mod i18n;
mod settings;
mod shelf;
mod store;
mod tray;
mod windows;

use std::{
    path::PathBuf,
    sync::{Mutex, PoisonError},
};

use rusqlite::Connection;
use serde_json::Value;
use tauri::{AppHandle, Manager, WindowEvent};
use tauri_plugin_global_shortcut::ShortcutState;

use crate::{error::Result, settings::Settings};

/// Shared state. Phase 1 keeps a single connection; the dedicated writer thread from
/// ARCHITECTURE §4 arrives with capture in Phase 2.
pub struct AppState {
    db: Mutex<Connection>,
    settings: Mutex<Settings>,
}

impl AppState {
    fn settings(&self) -> Settings {
        self.settings
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Validates, persists and swaps in new settings. Returns `(old, new)`.
    fn update_settings(&self, patch: Value) -> Result<(Settings, Settings)> {
        let mut current = self.settings.lock().unwrap_or_else(PoisonError::into_inner);
        let next = settings::apply_patch(&current, patch)?;
        {
            let db = self.db.lock().unwrap_or_else(PoisonError::into_inner);
            settings::save(&db, &next)?;
        }
        let old = std::mem::replace(&mut *current, next.clone());
        Ok((old, next))
    }
}

pub(crate) fn current_lang(app: &AppHandle) -> i18n::Lang {
    let language = app.state::<AppState>().settings().language;
    i18n::resolve(language)
}

/// `%APPDATA%\Panopin` on Windows (D-006); the platform data directory elsewhere.
fn data_dir(app: &AppHandle) -> Result<PathBuf> {
    Ok(app.path().data_dir()?.join("Panopin"))
}

fn setup(app: &AppHandle) -> Result<()> {
    let dir = data_dir(app)?;
    std::fs::create_dir_all(&dir)?;
    let conn = store::open(&dir.join("data.db"))?;
    let settings = settings::load(&conn)?;
    let lang = i18n::resolve(settings.language);
    let shortcuts = settings.shortcuts.clone();
    app.manage(AppState {
        db: Mutex::new(conn),
        settings: Mutex::new(settings),
    });
    tray::create(app, lang)?;
    let failed = hotkeys::register_all(app, &shortcuts);
    if !failed.is_empty() {
        log::warn!("{} shortcut(s) could not be registered", failed.len());
    }
    log::info!("Panopin {} started", app.package_info().version);
    Ok(())
}

/// A second launch brings the running instance forward instead of starting a copy.
fn activate_existing(app: &AppHandle) {
    let result = if app.get_webview_window(windows::SETTINGS).is_some() {
        windows::open_settings(app)
    } else {
        shelf::open(app, false)
    };
    if let Err(err) = result {
        log::error!("could not activate the running instance: {err}");
    }
}

fn log_failure(result: Result<()>, what: &str) {
    if let Err(err) = result {
        log::error!("{what} failed: {err}");
    }
}

pub fn run() {
    let result = tauri::Builder::default()
        // Must be registered first so a second launch exits before doing anything else.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            activate_existing(app)
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    match hotkeys::action_for(app, shortcut) {
                        Some(hotkeys::Action::ToggleShelf) => {
                            log_failure(shelf::toggle(app), "toggling the shelf")
                        }
                        Some(hotkeys::Action::OpenShelfSearch) => {
                            log_failure(shelf::open(app, true), "opening the shelf")
                        }
                        None => {}
                    }
                })
                .build(),
        )
        .manage(shelf::ShelfState::default())
        .manage(hotkeys::HotkeyState::default())
        .setup(|app| {
            setup(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != shelf::LABEL {
                return;
            }
            match event {
                // Clicking anywhere outside Panopin closes the shelf.
                WindowEvent::Focused(false) => log_failure(
                    shelf::request_close(window.app_handle()),
                    "closing the shelf",
                ),
                // The overlay is reused: Alt+F4 slides it away instead of destroying it.
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    log_failure(
                        shelf::request_close(window.app_handle()),
                        "closing the shelf",
                    );
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::update_settings,
            commands::close_shelf,
            commands::shelf_hidden,
        ])
        .run(tauri::generate_context!());
    if let Err(err) = result {
        log::error!("Panopin stopped unexpectedly: {err}");
        eprintln!("Panopin stopped unexpectedly: {err}");
    }
}
