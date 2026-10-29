//! IPC commands. Thin by design: they validate input and delegate to the core modules.

use std::sync::{Mutex, PoisonError};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Manager, State};

use crate::{
    capture::{self, now_ms},
    editing,
    error::{AppError, Result},
    media::annotations::EditLayer,
    settings::Settings,
    shelf,
    store::{
        items::Kind,
        library::{self, LibraryData},
        retention,
        shelf::{self as shelf_store, Shelf, UndoStack},
    },
    transfer::{self, BoardRect, PasteOutcome},
    windows, AppState,
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

/// Everything the Library shows: items (Trash included), folders and tags.
#[tauri::command]
pub fn library_data(state: State<'_, AppState>) -> Result<LibraryData> {
    state.with_db(|db| library::load(db))
}

/// Folders and tags for the shelf's pickers.
#[tauri::command]
pub fn library_groups(state: State<'_, AppState>) -> Result<library::Groups> {
    state.with_db(|db| library::groups(db))
}

/// Runs a Library change and tells every window that items changed.
fn organise<T>(
    app: &AppHandle,
    f: impl FnOnce(&mut rusqlite::Connection) -> Result<T>,
) -> Result<T> {
    let result = app.state::<AppState>().with_db(f)?;
    capture::changed(app);
    Ok(result)
}

/// Alt+P and the inspector; an empty title removes the name.
#[tauri::command]
pub fn set_title(app: AppHandle, ids: Vec<i64>, title: Option<String>) -> Result<()> {
    organise(&app, |db| library::set_title(db, &ids, title.as_deref()))
}

#[tauri::command]
pub fn set_note(app: AppHandle, id: i64, note: Option<String>) -> Result<()> {
    organise(&app, |db| library::set_note(db, id, note.as_deref()))
}

#[tauri::command]
pub fn create_folder(app: AppHandle, name: String, parent: Option<i64>) -> Result<i64> {
    organise(&app, |db| {
        library::create_folder(db, &name, parent, now_ms())
    })
}

#[tauri::command]
pub fn rename_folder(app: AppHandle, id: i64, name: String) -> Result<()> {
    organise(&app, |db| library::rename_folder(db, id, &name))
}

/// Deletes a folder and its subfolders; the items themselves stay.
#[tauri::command]
pub fn delete_folder(app: AppHandle, id: i64) -> Result<()> {
    organise(&app, |db| library::delete_folder(db, id))
}

#[tauri::command]
pub fn add_to_folder(app: AppHandle, ids: Vec<i64>, folder: i64) -> Result<()> {
    organise(&app, |db| library::add_to_folder(db, &ids, folder))
}

#[tauri::command]
pub fn remove_from_folder(app: AppHandle, ids: Vec<i64>, folder: i64) -> Result<()> {
    organise(&app, |db| library::remove_from_folder(db, &ids, folder))
}

/// Ctrl+Shift+T: tags the items with an existing or a new tag; their other tags stay.
#[tauri::command]
pub fn tag_items(app: AppHandle, ids: Vec<i64>, name: String) -> Result<i64> {
    organise(&app, |db| {
        let tag = library::tag_named(db, &name)?;
        library::add_tag(db, &ids, tag)?;
        Ok(tag)
    })
}

#[tauri::command]
pub fn untag_items(app: AppHandle, ids: Vec<i64>, tag: i64) -> Result<()> {
    organise(&app, |db| library::remove_tag(db, &ids, tag))
}

#[tauri::command]
pub fn rename_tag(app: AppHandle, id: i64, name: String) -> Result<()> {
    organise(&app, |db| library::rename_tag(db, id, &name))
}

/// Deleting a tag removes it from every item.
#[tauri::command]
pub fn delete_tag(app: AppHandle, id: i64) -> Result<()> {
    organise(&app, |db| library::delete_tag(db, id))
}

#[tauri::command]
pub fn restore_items(app: AppHandle, ids: Vec<i64>) -> Result<()> {
    organise(&app, |db| library::restore(db, &ids))
}

fn remove_media(app: &AppHandle, ids: &[i64]) {
    if let Some(media) = capture::media(app) {
        for id in ids {
            media.remove(*id);
        }
    }
}

/// Deletes items in Trash for good (after the window's confirmation), with their media.
#[tauri::command]
pub fn delete_forever(app: AppHandle, ids: Vec<i64>) -> Result<usize> {
    let deleted = organise(&app, |db| library::delete_forever(db, &ids))?;
    remove_media(&app, &deleted);
    Ok(deleted.len())
}

#[tauri::command]
pub fn empty_trash(app: AppHandle) -> Result<usize> {
    let deleted = organise(&app, library::empty_trash)?;
    remove_media(&app, &deleted);
    Ok(deleted.len())
}

/// Ctrl+N: a new, pinned, empty text note.
#[tauri::command]
pub fn new_note(app: AppHandle) -> Result<i64> {
    organise(&app, |db| library::new_note(db, now_ms()))
}

/// Ctrl+Shift+E: asks for a folder and writes the items there as files.
#[tauri::command]
pub async fn export_items(
    app: AppHandle,
    window: tauri::WebviewWindow,
    ids: Vec<i64>,
) -> Result<Option<transfer::Exported>> {
    tauri::async_runtime::spawn_blocking(move || transfer::export(&app, &window, &ids))
        .await
        .map_err(|_| AppError::Interrupted)?
}

/// L on the shelf and the tray: the shelf steps aside and the Library comes forward.
#[tauri::command]
pub fn open_library(app: AppHandle) -> Result<()> {
    shelf::request_close(&app, false)?;
    windows::open_library(&app)
}

/// The whole text behind a text or link card; the shelf itself only holds a preview.
#[tauri::command]
pub fn get_text(app: AppHandle, id: i64) -> Result<String> {
    editing::full_text(&app, id)
}

/// Saves an edited text; the preview calls this about 300 ms after typing stops.
#[tauri::command]
pub fn update_text(app: AppHandle, id: i64, body: String) -> Result<()> {
    editing::update_text(&app, id, &body)
}

/// The crop and marks over an image, if it was edited.
#[tauri::command]
pub fn get_edit(app: AppHandle, id: i64) -> Result<Option<EditLayer>> {
    editing::load_layer(&app, id)
}

/// Saves an image's crop and marks and redraws its thumbnail (off the main thread).
#[tauri::command]
pub async fn save_edit(app: AppHandle, id: i64, layer: EditLayer) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || editing::save_layer(&app, id, &layer))
        .await
        .map_err(|_| AppError::Interrupted)?
}

/// "Revert to original": removes the crop and the marks.
#[tauri::command]
pub async fn revert_edit(app: AppHandle, id: i64) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || editing::revert(&app, id))
        .await
        .map_err(|_| AppError::Interrupted)?
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
pub async fn start_drag(
    app: AppHandle,
    window: tauri::WebviewWindow,
    ids: Vec<i64>,
    board: Option<BoardRect>,
    files: Option<bool>,
) -> Result<bool> {
    tauri::async_runtime::spawn_blocking(move || {
        transfer::drag(&app, &window, &ids, board, files.unwrap_or(false))
    })
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
