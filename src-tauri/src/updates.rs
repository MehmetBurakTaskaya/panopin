//! Update checks and installs (D-065). Checking is off until the user turns it on; then Panopin
//! looks at start and every 6 hours, and "Check now" looks at once. Nothing is installed until
//! the user presses "Update and restart". The portable build only points to the download page.

use std::{
    sync::{Condvar, Mutex, PoisonError},
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::{capture::now_ms, error::Result, links, tray, AppState};

const EVERY: Duration = Duration::from_secs(6 * 60 * 60);
/// A failed check (often no network yet) is tried again sooner than the regular interval.
const AFTER_FAILURE: Duration = Duration::from_secs(60 * 60);
/// Gives the network time to come up after signing in before the first check.
const FIRST_DELAY: Duration = Duration::from_secs(30);
const TIMEOUT: Duration = Duration::from_secs(30);
/// Where the portable build sends the user for a new version.
pub const RELEASES: &str = "https://github.com/MehmetBurakTaskaya/panopin/releases/latest";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Status {
    #[default]
    Idle,
    Checking,
    UpToDate {
        checked_at: i64,
    },
    Available {
        version: String,
        notes: Option<String>,
        /// Release time in milliseconds, when the release states one.
        date: Option<i64>,
    },
    Downloading {
        received: u64,
        total: Option<u64>,
    },
    Failed {
        checked_at: i64,
    },
}

/// What Settings shows.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateView {
    pub current: String,
    /// False for the portable build and development runs, which never replace themselves.
    pub installable: bool,
    pub status: Status,
}

#[derive(Default)]
pub struct Updates {
    status: Mutex<Status>,
    update: Mutex<Option<Update>>,
    /// Set when the setting changes, so the scheduler reads it again.
    nudged: Mutex<bool>,
    wake: Condvar,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn start(app: &AppHandle) {
    app.manage(Updates::default());
    let app = app.clone();
    if let Err(err) = std::thread::Builder::new()
        .name("updates".into())
        .spawn(move || schedule(&app))
    {
        log::error!("update checks could not start: {err}");
    }
}

/// The setting changed: the scheduler starts, stops or keeps its timing.
pub fn nudge(app: &AppHandle) {
    let state = app.state::<Updates>();
    *lock(&state.nudged) = true;
    state.wake.notify_all();
}

/// Sleeps until the next check is due or the setting changes; no polling.
fn schedule(app: &AppHandle) {
    let state = app.state::<Updates>();
    let mut next = Instant::now() + FIRST_DELAY;
    loop {
        let on = app.state::<AppState>().settings().updates.auto_check;
        if on && Instant::now() >= next {
            let ok = tauri::async_runtime::block_on(check(app));
            next = Instant::now() + if ok { EVERY } else { AFTER_FAILURE };
            continue;
        }
        let nudged = lock(&state.nudged);
        let mut nudged = if on {
            let wait = next.saturating_duration_since(Instant::now());
            state
                .wake
                .wait_timeout_while(nudged, wait, |nudged| !*nudged)
                .unwrap_or_else(PoisonError::into_inner)
                .0
        } else {
            state
                .wake
                .wait_while(nudged, |nudged| !*nudged)
                .unwrap_or_else(PoisonError::into_inner)
        };
        *nudged = false;
    }
}

/// One check; false when it failed. A check or download already running is left alone.
pub async fn check(app: &AppHandle) -> bool {
    let state = app.state::<Updates>();
    {
        let mut status = lock(&state.status);
        if matches!(*status, Status::Checking | Status::Downloading { .. }) {
            return true;
        }
        *status = Status::Checking;
    }
    announce(app);
    let exiting = app.clone();
    let result = match app
        .updater_builder()
        .timeout(TIMEOUT)
        // Installing ends the process at once; clean up as a normal exit would.
        .on_before_exit(move || before_exit(&exiting))
        .build()
    {
        Ok(updater) => updater.check().await,
        Err(err) => Err(err),
    };
    let ok = result.is_ok();
    let status = match result {
        Ok(Some(update)) => {
            let status = available(&update);
            log::info!("update {} is available", update.version);
            *lock(&state.update) = Some(update);
            status
        }
        Ok(None) => {
            *lock(&state.update) = None;
            Status::UpToDate {
                checked_at: now_ms(),
            }
        }
        Err(err) => {
            log::warn!("update check failed: {err}");
            // An update found earlier stays on offer.
            match lock(&state.update).as_ref() {
                Some(update) => available(update),
                None => Status::Failed {
                    checked_at: now_ms(),
                },
            }
        }
    };
    *lock(&state.status) = status;
    announce(app);
    ok
}

fn available(update: &Update) -> Status {
    Status::Available {
        version: update.version.clone(),
        notes: update.body.clone().filter(|notes| !notes.trim().is_empty()),
        date: update.date.map(|date| date.unix_timestamp() * 1000),
    }
}

/// "Update and restart": downloads, verifies and runs the installer, which ends this process
/// and starts the new version. The portable build opens the download page instead.
pub async fn install(app: &AppHandle) -> Result<()> {
    if !installable() {
        return links::open_url(RELEASES);
    }
    let state = app.state::<Updates>();
    let Some(update) = lock(&state.update).clone() else {
        return Ok(());
    };
    {
        // A second press while downloading changes nothing.
        let mut status = lock(&state.status);
        if matches!(*status, Status::Downloading { .. }) {
            return Ok(());
        }
        *status = Status::Downloading {
            received: 0,
            total: None,
        };
    }
    announce(app);
    let mut received = 0u64;
    let mut shown = Instant::now();
    let downloaded = update
        .download(
            |chunk, total| {
                received += chunk as u64;
                if shown.elapsed() >= Duration::from_millis(200) {
                    shown = Instant::now();
                    set(app, Status::Downloading { received, total });
                }
            },
            || {},
        )
        .await;
    // Both steps check the signature against the public key in tauri.conf.json.
    let result = downloaded.and_then(|bytes| {
        log::info!("installing update {}", update.version);
        update.install(bytes)
    });
    if let Err(err) = result {
        log::error!("update could not be installed: {err}");
        set(app, available(&update));
        return Err(err.into());
    }
    // Windows never gets here: the installer has taken over. Other systems restart now.
    app.restart()
}

fn set(app: &AppHandle, status: Status) {
    *lock(&app.state::<Updates>().status) = status;
    announce(app);
}

fn announce(app: &AppHandle) {
    if let Err(err) = app.emit("updates:changed", view(app)) {
        log::warn!("could not announce the update status: {err}");
    }
    if let Err(err) = tray::refresh(app) {
        log::warn!("could not update the tray: {err}");
    }
}

fn before_exit(app: &AppHandle) {
    if let Some(staging) = app.try_state::<crate::transfer::Staging>() {
        staging.clean();
    }
    app.cleanup_before_exit();
}

pub fn view(app: &AppHandle) -> UpdateView {
    UpdateView {
        current: app.package_info().version.to_string(),
        installable: installable(),
        status: lock(&app.state::<Updates>().status).clone(),
    }
}

/// The version on offer, for the tray.
pub fn offered(app: &AppHandle) -> Option<String> {
    let state = app.try_state::<Updates>()?;
    let status = lock(&state.status).clone();
    match status {
        Status::Available { version, .. } => Some(version),
        Status::Downloading { .. } => lock(&state.update)
            .as_ref()
            .map(|update| update.version.clone()),
        _ => None,
    }
}

/// Installed copies have Tauri's uninstaller next to them; the portable build and development
/// runs do not (D-066).
pub fn installable() -> bool {
    #[cfg(windows)]
    {
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join("uninstall.exe").is_file()))
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_matches_the_webview_types() {
        let json = |status: Status| serde_json::to_value(status).unwrap();
        assert_eq!(json(Status::Idle), serde_json::json!({ "state": "idle" }));
        assert_eq!(
            json(Status::UpToDate { checked_at: 5 }),
            serde_json::json!({ "state": "upToDate", "checkedAt": 5 })
        );
        assert_eq!(
            json(Status::Available {
                version: "1.1.0".into(),
                notes: None,
                date: Some(7)
            }),
            serde_json::json!({ "state": "available", "version": "1.1.0", "notes": null, "date": 7 })
        );
        assert_eq!(
            json(Status::Downloading {
                received: 3,
                total: None
            }),
            serde_json::json!({ "state": "downloading", "received": 3, "total": null })
        );
    }

    #[test]
    fn only_installed_copies_replace_themselves() {
        // Test binaries, like the portable build, have no uninstaller next to them.
        assert!(!installable());
    }
}
