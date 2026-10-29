//! IPC commands. Thin by design: they validate input and delegate to the core modules.

use std::sync::{Mutex, PoisonError};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, State};

use crate::{
    capture::{self, now_ms},
    error::{AppError, Result},
    settings::Settings,
    shelf,
    store::{
        items::Kind,
        retention,
        shelf::{self as shelf_store, Shelf, UndoStack},
    },
    transfer::{self, BoardRect, PasteOutcome},
    AppState,
};

/// Session undo stack for take-downs.
#[derive(Default)]
pub struct ShelfUndo(Mutex<UndoStack>);

#[tauri::command]
pub fn list_shelf(state: State<'_, AppState>) -> Result<Shelf> {
    state.with_db(|db| shelf_store::list(db))
}

#[tauri::command]
pub fn set_pinned(
    app: AppHandle,
    state: State<'_, AppState>,
    ids: Vec<i64>,
    pinned: bool,
) -> Result<()> {
    state.with_db(|db| shelf_store::set_pinned(db, &ids, pinned))?;
    capture::changed(&app);
    Ok(())
}

/// "Take down": saved items leave the shelf but stay in the Library; the rest go to Trash.
#[tauri::command]
pub fn take_down(
    app: AppHandle,
    state: State<'_, AppState>,
    undo: State<'_, ShelfUndo>,
    ids: Vec<i64>,
) -> Result<()> {
    let batch = state.with_db(|db| shelf_store::take_down(db, &ids, now_ms()))?;
    undo.0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(batch);
    capture::changed(&app);
    Ok(())
}

/// Delete on the shelf, after the confirmation: moves the items to Trash. Backspace undoes it
/// like a take-down.
#[tauri::command]
pub fn trash_items(
    app: AppHandle,
    state: State<'_, AppState>,
    undo: State<'_, ShelfUndo>,
    ids: Vec<i64>,
) -> Result<()> {
    let batch = state.with_db(|db| shelf_store::trash(db, &ids, now_ms()))?;
    undo.0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(batch);
    capture::changed(&app);
    Ok(())
}

/// Puts the most recent take-down back and returns the restored ids (empty: nothing to undo).
#[tauri::command]
pub fn undo_take_down(
    app: AppHandle,
    state: State<'_, AppState>,
    undo: State<'_, ShelfUndo>,
) -> Result<Vec<i64>> {
    let Some(batch) = undo.0.lock().unwrap_or_else(PoisonError::into_inner).pop() else {
        return Ok(Vec::new());
    };
    let restored = state.with_db(|db| shelf_store::restore(db, &batch))?;
    capture::changed(&app);
    Ok(restored)
}

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

/// Esc, Enter and clicks beside the board: the shelf slides away and focus goes back.
#[tauri::command]
pub fn close_shelf(app: AppHandle) -> Result<()> {
    shelf::request_close(&app, true)
}

/// Click and Ctrl+C: copies the items in selection order; the shelf stays open. Images are
/// decoded and staged off the main thread.
#[tauri::command]
pub async fn copy_items(app: AppHandle, ids: Vec<i64>) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || transfer::copy(&app, &ids))
        .await
        .map_err(|_| AppError::Interrupted)?
}

/// Dragging cards out: text as text, images as files. Resolves when the drag ends, with whether
/// something was dropped.
#[tauri::command]
pub async fn start_drag(app: AppHandle, ids: Vec<i64>, board: BoardRect) -> Result<bool> {
    tauri::async_runtime::spawn_blocking(move || transfer::drag(&app, &ids, board))
        .await
        .map_err(|_| AppError::Interrupted)?
}

/// Ctrl+Enter: copies, returns to the previous window and pastes there, with the checks of
/// `transfer::paste`.
#[tauri::command]
pub async fn paste_items(app: AppHandle, ids: Vec<i64>) -> Result<PasteOutcome> {
    tauri::async_runtime::spawn_blocking(move || transfer::paste(&app, &ids))
        .await
        .map_err(|_| AppError::Interrupted)?
}

#[tauri::command]
pub fn shelf_hidden(app: AppHandle, generation: u64) -> Result<()> {
    shelf::finish_close(&app, generation)
}
