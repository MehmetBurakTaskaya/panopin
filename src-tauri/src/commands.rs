//! IPC commands. Thin by design: they validate input and delegate to the core modules.

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, State};

use crate::{
    capture::{self, now_ms},
    error::Result,
    settings::Settings,
    shelf,
    store::{items::Kind, retention},
    AppState,
};

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

/// Applies a partial settings update; every change takes effect immediately.
#[tauri::command]
pub fn update_settings(app: AppHandle, patch: Value) -> Result<Settings> {
    crate::apply_settings(&app, patch)
}

#[derive(Serialize)]
pub struct Clearable {
    count: usize,
    cutoff: i64,
}

/// "Clear history", step 1: how many unsaved items would move to Trash, and the cutoff to confirm with.
#[tauri::command]
pub fn count_clearable(state: State<'_, AppState>, kinds: Vec<Kind>) -> Result<Clearable> {
    let (count, cutoff) = state.with_db(|db| retention::clearable(db, &kinds, now_ms()))?;
    Ok(Clearable { count, cutoff })
}

/// "Clear history", step 2: moves the items that are still unsaved and were copied before the
/// cutoff to Trash, in one transaction. Copies made while confirming stay.
#[tauri::command]
pub fn clear_history(
    app: AppHandle,
    state: State<'_, AppState>,
    kinds: Vec<Kind>,
    cutoff: i64,
) -> Result<usize> {
    let moved = state.with_db(|db| retention::clear_history(db, &kinds, cutoff, now_ms()))?;
    capture::changed(&app);
    Ok(moved)
}

#[tauri::command]
pub fn close_shelf(app: AppHandle) -> Result<()> {
    shelf::request_close(&app)
}

#[tauri::command]
pub fn shelf_hidden(app: AppHandle, generation: u64) -> Result<()> {
    shelf::finish_close(&app, generation)
}
