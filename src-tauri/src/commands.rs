//! IPC commands. Thin by design: they validate input and delegate to the core modules.

use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

use crate::{error::Result, hotkeys, i18n, settings::Settings, shelf, tray, windows, AppState};

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

/// Applies a partial settings update; every change takes effect immediately.
#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: Value,
) -> Result<Settings> {
    let (old, new) = state.update_settings(patch)?;
    if old.language != new.language {
        let lang = i18n::resolve(new.language);
        tray::relabel(&app, lang)?;
        windows::retitle(&app, lang)?;
    }
    if old.shortcuts != new.shortcuts {
        hotkeys::register_all(&app, &new.shortcuts);
    }
    app.emit("settings:changed", &new)?;
    Ok(new)
}

#[tauri::command]
pub fn close_shelf(app: AppHandle) -> Result<()> {
    shelf::request_close(&app)
}

#[tauri::command]
pub fn shelf_hidden(app: AppHandle, generation: u64) -> Result<()> {
    shelf::finish_close(&app, generation)
}
