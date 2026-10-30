mod capture;
mod commands;
mod edge;
mod editing;
mod error;
mod focus;
mod hotkeys;
mod i18n;
mod links;
mod look;
mod media;
mod ocr;
mod settings;
mod shelf;
mod snip;
mod store;
mod system_state;
mod transfer;
mod tray;
mod updates;
mod windows;

use std::{
    path::PathBuf,
    sync::{Mutex, PoisonError},
};

use rusqlite::Connection;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::ShortcutState;

use crate::{error::Result, settings::Settings};

/// Shared state. One SQLite connection behind a mutex serves every writer; decoding, hashing
/// and thumbnails happen outside the lock, so it is held only for short statements (D-035).
pub struct AppState {
    db: Mutex<Connection>,
    settings: Mutex<Settings>,
}

impl AppState {
    pub(crate) fn settings(&self) -> Settings {
        self.settings
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn with_db<T>(&self, f: impl FnOnce(&mut Connection) -> Result<T>) -> Result<T> {
        let mut db = self.db.lock().unwrap_or_else(PoisonError::into_inner);
        f(&mut db)
    }

    /// Validates, persists and swaps in new settings. Returns `(old, new)`.
    fn update_settings(&self, patch: Value) -> Result<(Settings, Settings)> {
        let mut current = self.settings.lock().unwrap_or_else(PoisonError::into_inner);
        let next = settings::apply_patch(&current, patch)?;
        self.with_db(|db| settings::save(db, &next))?;
        let old = std::mem::replace(&mut *current, next.clone());
        Ok((old, next))
    }
}

pub(crate) fn current_lang(app: &AppHandle) -> i18n::Lang {
    let language = app.state::<AppState>().settings().language;
    i18n::resolve(language)
}

/// Applies a settings change and its side effects; every change takes effect immediately.
pub(crate) fn apply_settings(app: &AppHandle, patch: Value) -> Result<Settings> {
    let (old, new) = app.state::<AppState>().update_settings(patch)?;
    if old.language != new.language {
        windows::retitle(app, i18n::resolve(new.language))?;
    }
    if old.theme != new.theme {
        windows::apply_theme(app)?;
    }
    if old.appearance.material != new.appearance.material {
        windows::apply_material(app)?;
    }
    if old.launch_at_login != new.launch_at_login {
        set_launch_at_login(app, new.launch_at_login);
    }
    if old.edge_open != new.edge_open || old.shelf_edge != new.shelf_edge {
        edge::configure(app, new.edge_open, new.shelf_edge);
    }
    if old.language != new.language || old.paused_until != new.paused_until {
        tray::refresh(app)?;
    }
    if old.shortcuts != new.shortcuts {
        hotkeys::register_all(app, &new.shortcuts);
    }
    if !old.onboarded && new.onboarded {
        windows::close_onboarding(app)?;
    }
    if !old.ocr && new.ocr {
        ocr::wake(app);
    }
    if !old.link_previews && new.link_previews {
        links::wake(app);
    }
    if old.updates != new.updates {
        updates::nudge(app);
    }
    if old.onboarded != new.onboarded
        || old.sources != new.sources
        || old.watched_folders != new.watched_folders
    {
        capture::reconfigure(app);
    }
    app.emit("settings:changed", &new)?;
    Ok(new)
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
    let onboarded = settings.onboarded;
    let shortcuts = settings.shortcuts.clone();
    app.manage(AppState {
        db: Mutex::new(conn),
        settings: Mutex::new(settings),
    });
    let staging = transfer::Staging::new(dir.join("staging"));
    staging.clean();
    app.manage(staging);
    rebuild_search_index(app);
    capture::start(app, &dir)?;
    ocr::start(app)?;
    links::start(app);
    updates::start(app);
    look::watch(app);
    let settings = app.state::<AppState>().settings();
    edge::configure(app, settings.edge_open, settings.shelf_edge);
    // The setting is the truth; the registry entry follows it (it may point to an older copy).
    set_launch_at_login(app, settings.launch_at_login);
    tray::create(app)?;
    let unavailable = hotkeys::register_all(app, &shortcuts);
    if !unavailable.is_empty() {
        log::warn!("{} shortcut(s) could not be registered", unavailable.len());
    }
    if !onboarded {
        windows::open_onboarding(app)?;
    }
    log::info!("Panopin {} started", app.package_info().version);
    Ok(())
}

/// Fills the search index when it is new or was built by an older version, a few hundred items
/// per lock so captures never wait long. Items written meanwhile index themselves.
fn rebuild_search_index(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let run = || -> Result<bool> {
            if !state.with_db(|db| store::search::outdated(db))? {
                return Ok(false);
            }
            state.with_db(|db| store::search::clear(db))?;
            let mut after = 0;
            while let Some(last) =
                state.with_db(|db| store::search::rebuild_batch(db, after, 300))?
            {
                after = last;
            }
            state.with_db(|db| store::search::mark_built(db))?;
            Ok(true)
        };
        match run() {
            Ok(true) => log::info!("search index rebuilt"),
            Ok(false) => {}
            Err(err) => log::error!("search index rebuild failed: {err}"),
        }
    });
}

/// Start at login (D-060), through the autostart plugin's entry in the user's Run key.
fn set_launch_at_login(app: &AppHandle, on: bool) {
    use tauri_plugin_autostart::ManagerExt;
    let launcher = app.autolaunch();
    let result = match (on, launcher.is_enabled()) {
        (true, Ok(false)) => launcher.enable(),
        (false, Ok(true)) => launcher.disable(),
        (_, Ok(_)) => Ok(()),
        (_, Err(err)) => Err(err),
    };
    if let Err(err) = result {
        log::warn!("start at login could not be changed: {err}");
    }
}

/// A second launch brings the running instance forward instead of starting a copy.
fn activate_existing(app: &AppHandle) {
    let result = if app.get_webview_window(windows::ONBOARDING).is_some() {
        windows::open_onboarding(app)
    } else if app.get_webview_window(windows::SETTINGS).is_some() {
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
    // A second launch hands its right to take the foreground to the running instance, so the
    // shelf it opens there gets focus (the first launch gives this up at its first input).
    #[cfg(windows)]
    // SAFETY: no pointers; failure only means focus stays where it is.
    unsafe {
        use ::windows::Win32::UI::WindowsAndMessaging::{AllowSetForegroundWindow, ASFW_ANY};
        let _ = AllowSetForegroundWindow(ASFW_ANY);
    }
    let app = tauri::Builder::default()
        // Must be registered first so a second launch exits before doing anything else.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            activate_existing(app)
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
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
                        Some(hotkeys::Action::Snip) => {
                            log_failure(snip::start(app), "starting a snip")
                        }
                        Some(hotkeys::Action::OpenLibrary) => {
                            log_failure(windows::open_library(app), "opening the Library")
                        }
                        Some(hotkeys::Action::CopyLastImage) => {
                            log_failure(capture::copy_last_image(app), "copying the last image")
                        }
                        Some(hotkeys::Action::PauseCapture) => {
                            log_failure(capture::toggle_pause(app), "pausing capture")
                        }
                        None => {}
                    }
                })
                .build(),
        )
        .manage(shelf::ShelfState::default())
        .manage(hotkeys::HotkeyState::default())
        .manage(snip::SnipState::default())
        .manage(commands::ShelfUndo::default())
        .manage(windows::SettingsTab::default())
        .register_asynchronous_uri_scheme_protocol("media", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            let path = request.uri().path().to_owned();
            let range = request
                .headers()
                .get("range")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            std::thread::spawn(move || {
                let unavailable = || {
                    tauri::http::Response::builder()
                        .status(503)
                        .body(Vec::new())
                        .unwrap_or_default()
                };
                let response = match (capture::media(&app), app.try_state::<AppState>()) {
                    (Some(store), Some(state)) => state
                        .with_db(|db| Ok(media::serve(&store, db, &path, range.as_deref())))
                        .unwrap_or_else(|_| unavailable()),
                    _ => unavailable(),
                };
                responder.respond(response);
            });
        })
        .setup(|app| {
            // A menu bar app on macOS: no Dock icon and no app menu (D-073).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            // updates::start reads the plugin's state, so it is registered first (D-065).
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            setup(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != shelf::LABEL {
                return;
            }
            match event {
                // Clicking anywhere outside Panopin closes the shelf.
                WindowEvent::Focused(focused) => log_failure(
                    shelf::focus_changed(window.app_handle(), *focused),
                    "closing the shelf",
                ),
                // The overlay is reused: Alt+F4 slides it away instead of destroying it.
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    log_failure(
                        shelf::request_close(window.app_handle(), true),
                        "closing the shelf",
                    );
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::update_settings,
            commands::count_clearable,
            commands::clear_history,
            commands::list_shelf,
            commands::search_shelf,
            commands::search_library,
            commands::copy_image_text,
            commands::ocr_status,
            commands::refresh_link,
            commands::open_link,
            commands::import_files,
            commands::suspend_shortcuts,
            commands::set_shortcut,
            commands::unavailable_shortcuts,
            commands::system_look,
            commands::set_pause,
            commands::pick_watch_folder,
            commands::open_project_link,
            commands::open_licence_texts,
            commands::app_version,
            commands::update_status,
            commands::check_for_updates,
            commands::install_update,
            commands::take_settings_tab,
            commands::set_pinned,
            commands::take_down,
            commands::undo_take_down,
            commands::trash_items,
            commands::copy_items,
            commands::paste_items,
            commands::start_drag,
            commands::get_text,
            commands::update_text,
            commands::get_edit,
            commands::save_edit,
            commands::revert_edit,
            commands::library_data,
            commands::library_groups,
            commands::set_title,
            commands::set_note,
            commands::create_folder,
            commands::rename_folder,
            commands::delete_folder,
            commands::add_to_folder,
            commands::remove_from_folder,
            commands::tag_items,
            commands::untag_items,
            commands::rename_tag,
            commands::delete_tag,
            commands::restore_items,
            commands::delete_forever,
            commands::empty_trash,
            commands::new_note,
            commands::export_items,
            commands::open_library,
            commands::open_settings,
            commands::close_shelf,
            commands::shelf_hidden,
        ])
        .build(tauri::generate_context!());
    match app {
        Ok(app) => app.run(|app, event| {
            // Staged files are only for the session that handed them out.
            if let tauri::RunEvent::Exit = event {
                if let Some(staging) = app.try_state::<transfer::Staging>() {
                    staging.clean();
                }
            }
        }),
        Err(err) => {
            log::error!("Panopin could not start: {err}");
            eprintln!("Panopin could not start: {err}");
        }
    }
}
