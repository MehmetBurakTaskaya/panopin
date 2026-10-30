//! IPC commands. Thin by design: they validate input and delegate to the core modules.

use std::sync::{Mutex, PoisonError};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Manager, State};

use crate::{
    capture::{self, now_ms},
    editing,
    error::{AppError, Result},
    hotkeys, links,
    media::annotations::EditLayer,
    ocr,
    settings::Settings,
    shelf,
    store::{
        items::Kind,
        library::{self, LibraryData},
        retention, search,
        shelf::{self as shelf_store, Shelf, UndoStack},
    },
    transfer::{self, BoardRect, PasteOutcome},
    updates, windows, AppState,
};

/// Session undo stack for take-downs.
#[derive(Default)]
pub struct ShelfUndo(Mutex<UndoStack>);

#[tauri::command]
pub fn list_shelf(state: State<'_, AppState>) -> Result<Shelf> {
    state.with_db(|db| shelf_store::list(db))
}

/// Shift+S on the shelf: the matching cards of each row, newest first (D-059).
#[tauri::command]
pub fn search_shelf(state: State<'_, AppState>, query: String) -> Result<Shelf> {
    let without_accents = state.settings().search_without_accents;
    state.with_db(|db| shelf_store::search(db, &query, without_accents))
}

/// Ctrl+Shift+C: copies the text in an image, reading it first when needed.
#[tauri::command]
pub async fn copy_image_text(app: AppHandle, id: i64) -> Result<ocr::TextCopy> {
    tauri::async_runtime::spawn_blocking(move || ocr::copy_text(&app, id))
        .await
        .map_err(|_| AppError::Interrupted)?
}

/// "Refresh preview": the link is visited again. Returns whether previews are on at all.
#[tauri::command]
pub fn refresh_link(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<bool> {
    state.with_db(|db| links::refresh(db, id))?;
    links::wake(&app);
    Ok(state.settings().link_previews)
}

/// Ctrl+O: opens a link in the default browser.
#[tauri::command]
pub fn open_link(state: State<'_, AppState>, id: i64) -> Result<()> {
    state.with_db(|db| links::open(db, id))
}

/// Installed OCR languages for Settings.
#[tauri::command]
pub async fn ocr_status() -> Result<ocr::OcrStatus> {
    tauri::async_runtime::spawn_blocking(ocr::status)
        .await
        .map_err(|_| AppError::Interrupted)
}

/// Ctrl+F in the Library: ids of the matching items, best match first.
#[tauri::command]
pub fn search_library(state: State<'_, AppState>, query: String) -> Result<Vec<i64>> {
    let without_accents = state.settings().search_without_accents;
    state.with_db(|db| search::find(db, &query, without_accents))
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

/// The shortcut recorder releases Panopin's shortcuts while it listens (D-061).
#[tauri::command]
pub fn suspend_shortcuts(app: AppHandle, suspended: bool) {
    hotkeys::suspend(&app, suspended);
}

/// Saves one shortcut (`None` = off). A combination another app holds is refused and the old
/// one stays.
#[tauri::command]
pub fn set_shortcut(
    app: AppHandle,
    action: hotkeys::Action,
    accelerator: Option<String>,
) -> Result<Settings> {
    let key = match action {
        hotkeys::Action::ToggleShelf => "shelf",
        hotkeys::Action::OpenShelfSearch => "shelfSearch",
        hotkeys::Action::Snip => "snip",
        hotkeys::Action::OpenLibrary => "library",
        hotkeys::Action::CopyLastImage => "copyLastImage",
        hotkeys::Action::PauseCapture => "pauseCapture",
    };
    let previous = app.state::<AppState>().settings().shortcuts;
    let next = crate::apply_settings(
        &app,
        serde_json::json!({ "shortcuts": { key: accelerator } }),
    )?;
    if accelerator.is_some() && hotkeys::unavailable(&app).contains(&action) {
        crate::apply_settings(&app, serde_json::json!({ "shortcuts": previous }))?;
        return Err(AppError::ShortcutUnavailable);
    }
    Ok(next)
}

/// Settings → Privacy: pause until a time (Unix ms), or resume with `None`.
#[tauri::command]
pub fn set_pause(app: AppHandle, until: Option<i64>) -> Result<()> {
    // JavaScript numbers stop at 2^53; anything that far away means "until resumed".
    let until = until.map(|at| {
        if at >= (1 << 53) - 1 {
            crate::settings::PAUSED_UNTIL_RESUMED
        } else {
            at
        }
    });
    capture::set_pause(&app, until)
}

/// Mica support and the Windows accent colour (D-060).
#[tauri::command]
pub fn system_look() -> crate::look::SystemLook {
    crate::look::current()
}

/// Settings → Sources: a folder to watch for screenshots and recordings.
#[tauri::command]
pub async fn pick_watch_folder(
    app: AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Option<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        transfer::pick_folder_titled(&app, &window, "settings.sources.pick")
            .map(|folder| folder.map(|f| f.display().to_string()))
    })
    .await
    .map_err(|_| AppError::Interrupted)?
}

/// Settings → About: only these fixed pages open in the browser.
#[tauri::command]
pub fn open_project_link(which: String) -> Result<()> {
    let url = match which.as_str() {
        "website" => "https://mehmetburaktaskaya.com",
        "github" => "https://github.com/MehmetBurakTaskaya/panopin",
        "licence" => "https://github.com/MehmetBurakTaskaya/panopin/blob/main/LICENSE",
        // Where Windows adds languages, OCR included.
        "languages" => "ms-settings:regionlanguage",
        _ => return Err(AppError::InvalidSetting("unknown link")),
    };
    links::open_url(url)
}

/// Settings → About: the licence texts installed next to Panopin, else their copy on GitHub.
#[tauri::command]
pub fn open_licence_texts(app: AppHandle) -> Result<()> {
    let installed = app
        .path()
        .resolve(
            "THIRD_PARTY_NOTICES.txt",
            tauri::path::BaseDirectory::Resource,
        )
        .ok()
        .filter(|path| path.is_file());
    match installed {
        // The shell does not open verbatim (`\\?\`) paths.
        Some(path) => links::open_url(path.display().to_string().trim_start_matches(r"\\?\")),
        None => links::open_url(
            "https://github.com/MehmetBurakTaskaya/panopin/blob/main/THIRD_PARTY_NOTICES.txt",
        ),
    }
}

#[tauri::command]
pub fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
pub fn update_status(app: AppHandle) -> updates::UpdateView {
    updates::view(&app)
}

/// "Check now" in Settings: the user asked, so it runs whatever the automatic setting says.
#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> updates::UpdateView {
    updates::check(&app).await;
    updates::view(&app)
}

/// "Update and restart"; the portable build opens the download page instead.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<()> {
    updates::install(&app).await
}

/// The tab a new Settings window opens on, once.
#[tauri::command]
pub fn take_settings_tab(state: State<'_, windows::SettingsTab>) -> Option<&'static str> {
    state.take()
}

/// Shortcuts whose combination another app holds; Settings warns about them.
#[tauri::command]
pub fn unavailable_shortcuts(app: AppHandle) -> Vec<hotkeys::Action> {
    hotkeys::unavailable(&app)
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

/// "Import…" in the Library: pictures and recordings from files; returns how many came in.
#[tauri::command]
pub async fn import_files(app: AppHandle, window: tauri::WebviewWindow) -> Result<usize> {
    tauri::async_runtime::spawn_blocking(move || {
        let paths = transfer::pick_import(&app, &window)?;
        capture::import(&app, &paths)
    })
    .await
    .map_err(|_| AppError::Interrupted)?
}

/// The palette's "Settings": the shelf steps aside and Settings comes forward.
#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<()> {
    shelf::request_close(&app, false)?;
    windows::open_settings(&app)
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
