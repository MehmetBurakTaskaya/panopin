//! Updates (D-065, D-076). Panopin looks for a new version at start and every 6 hours and
//! installs it by itself once it is not in use: the installed Windows copy through Tauri's
//! installer, the portable one by swapping its files for the release ZIP's, a Mac by replacing its
//! app bundle. "Check now" looks at once and "Update and restart" installs at once. A copy that
//! cannot replace itself only points to the download page.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, OnceLock, PoisonError,
    },
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::{capture::now_ms, error::Result, links, shelf, tray};

const EVERY: Duration = Duration::from_secs(6 * 60 * 60);
/// A failed check (often no network yet) is tried again sooner than the regular interval.
const AFTER_FAILURE: Duration = Duration::from_secs(60 * 60);
/// Gives the network time to come up after signing in before the first check.
const FIRST_DELAY: Duration = Duration::from_secs(30);
const TIMEOUT: Duration = Duration::from_secs(30);
/// How often an update waiting to install looks whether Panopin is out of use.
const IDLE_POLL: Duration = Duration::from_secs(60);
/// Where a copy that cannot replace itself sends the user for a new version.
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
    /// False for copies that never replace themselves (development runs, a Mac app that macOS
    /// runs from a read-only translocation folder).
    pub installable: bool,
    /// Whether an update installs by itself; see [`automatic`].
    pub automatic: bool,
    pub status: Status,
}

#[derive(Default)]
pub struct Updates {
    status: Mutex<Status>,
    update: Mutex<Option<Update>>,
    /// Set while an update on offer waits for Panopin to be out of use.
    waiting: Mutex<bool>,
    /// Set by "Update and restart" while the update downloads by itself: it then installs at once.
    now: AtomicBool,
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

/// Checks after start and then every 6 hours, or after an hour when a check failed.
fn schedule(app: &AppHandle) {
    std::thread::sleep(FIRST_DELAY);
    #[cfg(windows)]
    match kind() {
        Kind::Portable => portable::tidy(),
        Kind::Installer => installed::tidy(),
        Kind::Fixed => {}
    }
    loop {
        let ok = tauri::async_runtime::block_on(check(app));
        std::thread::sleep(if ok { EVERY } else { AFTER_FAILURE });
    }
}

/// One check; false when it failed. A check or download already running is left alone. A new
/// version found by any check installs by itself once Panopin is not in use.
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
    let builder = app
        .updater_builder()
        .timeout(TIMEOUT)
        // Installing ends the process at once; clean up as a normal exit would.
        .on_before_exit(move || before_exit(&exiting));
    // The portable copy has its own entry in latest.json: the ZIP it unpacks.
    #[cfg(windows)]
    let builder = if kind() == Kind::Portable {
        builder.target(portable::TARGET)
    } else {
        builder
    };
    let result = match builder.build() {
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
    let offered = matches!(status, Status::Available { .. });
    *lock(&state.status) = status;
    announce(app);
    if offered && automatic() {
        install_when_idle(app);
    }
    ok
}

fn available(update: &Update) -> Status {
    Status::Available {
        version: update.version.clone(),
        notes: update.body.clone().filter(|notes| !notes.trim().is_empty()),
        date: update.date.map(|date| date.unix_timestamp() * 1000),
    }
}

/// Installs the update on offer once the shelf is closed and no Panopin window is in front, so
/// the restart never cuts into the user's work. One waiter at a time; it stops when the update is
/// no longer on offer or installing failed, and the next check starts it again.
fn install_when_idle(app: &AppHandle) {
    let state = app.state::<Updates>();
    {
        let mut waiting = lock(&state.waiting);
        if *waiting {
            return;
        }
        *waiting = true;
    }
    let waiter = app.clone();
    let spawned = std::thread::Builder::new()
        .name("update-install".into())
        .spawn(move || {
            let state = waiter.state::<Updates>();
            loop {
                std::thread::sleep(IDLE_POLL);
                if in_use(&waiter) {
                    continue;
                }
                if !matches!(*lock(&state.status), Status::Available { .. }) {
                    break;
                }
                if let Err(err) = tauri::async_runtime::block_on(apply(&waiter, true)) {
                    log::error!("the update could not install by itself: {err}");
                    break;
                }
            }
            *lock(&state.waiting) = false;
        });
    if let Err(err) = spawned {
        log::error!("the update could not be scheduled: {err}");
        *lock(&state.waiting) = false;
    }
}

/// Panopin is in use while the shelf is open, one of its windows has the keyboard or items wait
/// to be pasted one by one.
fn in_use(app: &AppHandle) -> bool {
    shelf::is_open(app)
        || crate::transfer::queue::is_running()
        || app
            .webview_windows()
            .values()
            .any(|window| window.is_focused().unwrap_or(false))
}

/// "Update and restart": installs at once. A copy that cannot replace itself opens the download
/// page instead.
pub async fn install(app: &AppHandle) -> Result<()> {
    if !installable() {
        return links::open_url(RELEASES);
    }
    apply(app, false).await
}

/// Downloads, verifies and installs the update on offer. Windows runs the installer, which ends
/// this process and starts the new version; the portable copy and a Mac replace their files and
/// restart. An install that waited for Panopin to be out of use (`when_idle`) leaves the download
/// unused if Panopin came into use meanwhile; the waiter tries again later.
async fn apply(app: &AppHandle, when_idle: bool) -> Result<()> {
    let state = app.state::<Updates>();
    let Some(update) = lock(&state.update).clone() else {
        return Ok(());
    };
    {
        let mut status = lock(&state.status);
        if matches!(*status, Status::Downloading { .. }) {
            // A press while the update downloads by itself lets it install at once.
            if !when_idle {
                state.now.store(true, Ordering::Relaxed);
            }
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
    // Both download and install check the signature against the public key in tauri.conf.json.
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
    let result = match downloaded {
        Ok(_) if when_idle && !state.now.load(Ordering::Relaxed) && in_use(app) => {
            set(app, available(&update));
            return Ok(());
        }
        Ok(bytes) => {
            log::info!("installing update {}", update.version);
            put_in_place(&update, bytes)
        }
        Err(err) => Err(err.into()),
    };
    if let Err(err) = result {
        log::error!("update could not be installed: {err}");
        state.now.store(false, Ordering::Relaxed);
        set(app, available(&update));
        return Err(err);
    }
    // Windows never gets here: the installer has taken over. The portable copy and a Mac restart
    // into the new files.
    app.restart()
}

fn put_in_place(update: &Update, bytes: Vec<u8>) -> Result<()> {
    #[cfg(windows)]
    if kind() == Kind::Portable {
        update.verify(&bytes)?;
        return std::env::current_exe()
            .and_then(|exe| portable::replace(&exe, &bytes))
            .map_err(Into::into);
    }
    update.install(bytes).map_err(Into::into)
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
    crate::transfer::finish_queue();
    if let Some(staging) = app.try_state::<crate::transfer::Staging>() {
        staging.clean();
    }
    app.cleanup_before_exit();
}

pub fn view(app: &AppHandle) -> UpdateView {
    UpdateView {
        current: app.package_info().version.to_string(),
        installable: installable(),
        automatic: automatic(),
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

/// How this copy replaces itself (D-066, D-076).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Tauri's installer on Windows, the update archive on a Mac.
    Installer,
    /// The portable Windows copy swaps its own files.
    #[cfg(windows)]
    Portable,
    /// Development runs and a Mac app that macOS runs from a read-only translocation folder.
    Fixed,
}

fn kind() -> Kind {
    static KIND: OnceLock<Kind> = OnceLock::new();
    *KIND.get_or_init(|| {
        let Ok(exe) = std::env::current_exe() else {
            return Kind::Fixed;
        };
        #[cfg(windows)]
        {
            if exe
                .parent()
                .is_some_and(|dir| dir.join("uninstall.exe").is_file())
            {
                Kind::Installer
            } else if portable::is_portable(&exe) {
                Kind::Portable
            } else {
                Kind::Fixed
            }
        }
        #[cfg(target_os = "macos")]
        {
            let path = exe.to_string_lossy();
            if path.contains(".app/Contents/MacOS/") && !path.contains("/AppTranslocation/") {
                Kind::Installer
            } else {
                Kind::Fixed
            }
        }
        #[cfg(not(any(windows, target_os = "macos")))]
        {
            let _ = exe;
            Kind::Fixed
        }
    })
}

/// Whether this copy can replace itself.
pub fn installable() -> bool {
    kind() != Kind::Fixed
}

/// Whether an update installs by itself, without asking anything. A Mac app in a folder the user
/// may not change could only be replaced after an administrator's password, so its updates wait
/// for "Update and restart".
fn automatic() -> bool {
    #[cfg(target_os = "macos")]
    {
        installable() && mac::bundle_writable()
    }
    #[cfg(not(target_os = "macos"))]
    {
        installable()
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::{ffi::CString, os::unix::ffi::OsStrExt, path::Path};

    /// The updater renames the app bundle and the folder entry it sits in.
    pub fn bundle_writable() -> bool {
        let Ok(exe) = std::env::current_exe() else {
            return false;
        };
        // …/Panopin.app/Contents/MacOS/panopin
        let Some(bundle) = exe.ancestors().nth(3) else {
            return false;
        };
        writable(bundle) && bundle.parent().is_some_and(writable)
    }

    fn writable(path: &Path) -> bool {
        CString::new(path.as_os_str().as_bytes()).is_ok_and(|path| {
            // SAFETY: a NUL-terminated path; the call only checks permissions.
            unsafe { libc::access(path.as_ptr(), libc::W_OK) == 0 }
        })
    }
}

/// The portable Windows copy updates from the release's ZIP. Windows lets a running program be
/// renamed but not overwritten, so the new Panopin.exe is written in full first, the running one
/// steps aside as Panopin.old and the next start removes it.
#[cfg(windows)]
mod portable {
    use std::{
        fs,
        io::{self, Read, Seek},
        path::Path,
    };

    /// The portable copy's entry in latest.json.
    pub const TARGET: &str = "windows-x86_64-portable";
    /// The texts that come with the program in the ZIP.
    const TEXTS: [&str; 2] = ["LICENSE.txt", "THIRD_PARTY_NOTICES.txt"];

    /// A release build unpacked from the ZIP, with the notices next to it. The installed copy,
    /// which has them too, is recognised first by its uninstaller.
    pub fn is_portable(exe: &Path) -> bool {
        !cfg!(debug_assertions) && exe.parent().is_some_and(|dir| dir.join(TEXTS[1]).is_file())
    }

    /// Swaps the program at `exe` and the texts next to it for the ones in `zip`, whose signature
    /// has been checked.
    pub fn replace(exe: &Path, zip: &[u8]) -> io::Result<()> {
        let dir = exe
            .parent()
            .ok_or_else(|| io::Error::other("the program has no folder"))?;
        let mut archive = zip::ZipArchive::new(io::Cursor::new(zip))?;
        let fresh = exe.with_extension("new");
        if let Err(err) = unpack(&mut archive, "Panopin.exe", &fresh) {
            let _ = fs::remove_file(&fresh);
            return Err(err);
        }
        let old = exe.with_extension("old");
        // A leftover from an earlier update; if it cannot go, the rename below fails.
        let _ = fs::remove_file(&old);
        fs::rename(exe, &old)?;
        if let Err(err) = fs::rename(&fresh, exe) {
            let _ = fs::rename(&old, exe);
            let _ = fs::remove_file(&fresh);
            return Err(err);
        }
        // The program runs without them, so a text that cannot be replaced keeps its old version.
        for name in TEXTS {
            let target = dir.join(name);
            let fresh = target.with_extension("new");
            if unpack(&mut archive, name, &fresh)
                .and_then(|()| fs::rename(&fresh, &target))
                .is_err()
            {
                let _ = fs::remove_file(&fresh);
            }
        }
        Ok(())
    }

    /// Writes the ZIP's file called `name`, wherever it sits in the archive, to `to`.
    fn unpack<R: Read + Seek>(
        archive: &mut zip::ZipArchive<R>,
        name: &str,
        to: &Path,
    ) -> io::Result<()> {
        let path = archive
            .file_names()
            .filter_map(|path| path.ok())
            .find(|path| {
                path.rsplit(['/', '\\'])
                    .next()
                    .is_some_and(|file| file.eq_ignore_ascii_case(name))
            })
            .map(|path| path.into_owned())
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, format!("no {name} in the update"))
            })?;
        let mut entry = archive.by_name(&path)?;
        let mut file = fs::File::create(to)?;
        io::copy(&mut entry, &mut file)?;
        file.sync_all()
    }

    /// Removes the program an earlier update set aside, which has stopped by now.
    pub fn tidy() {
        let Ok(exe) = std::env::current_exe() else {
            return;
        };
        let old = exe.with_extension("old");
        if old.is_file() {
            if let Err(err) = fs::remove_file(&old) {
                log::warn!("the program an update set aside could not be removed: {err}");
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use std::io::Write;

        use super::*;

        fn zip_of(files: &[(&str, &str)]) -> Vec<u8> {
            let mut zip = zip::ZipWriter::new(io::Cursor::new(Vec::new()));
            // Deflated, as the release job packs it.
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            for (name, body) in files {
                zip.start_file(*name, options).unwrap();
                zip.write_all(body.as_bytes()).unwrap();
            }
            zip.finish().unwrap().into_inner()
        }

        #[test]
        fn swaps_the_program_and_its_texts() {
            let temp = tempfile::tempdir().unwrap();
            let dir = temp.path();
            let exe = dir.join("Panopin.exe");
            fs::write(&exe, "old program").unwrap();
            fs::write(dir.join("LICENSE.txt"), "licence").unwrap();
            fs::write(dir.join("THIRD_PARTY_NOTICES.txt"), "old notices").unwrap();

            // Nothing changes when the ZIP holds no program.
            let empty = zip_of(&[("THIRD_PARTY_NOTICES.txt", "new notices")]);
            assert!(replace(&exe, &empty).is_err());
            assert_eq!(fs::read_to_string(&exe).unwrap(), "old program");
            assert_eq!(
                fs::read_to_string(dir.join("THIRD_PARTY_NOTICES.txt")).unwrap(),
                "old notices"
            );
            assert!(!dir.join("Panopin.new").exists());

            let update = zip_of(&[
                ("Panopin.exe", "new program"),
                ("LICENSE.txt", "licence"),
                ("THIRD_PARTY_NOTICES.txt", "new notices"),
            ]);
            replace(&exe, &update).unwrap();
            assert_eq!(fs::read_to_string(&exe).unwrap(), "new program");
            assert_eq!(
                fs::read_to_string(dir.join("Panopin.old")).unwrap(),
                "old program"
            );
            assert_eq!(
                fs::read_to_string(dir.join("THIRD_PARTY_NOTICES.txt")).unwrap(),
                "new notices"
            );
            assert!(!dir.join("Panopin.new").exists());
            assert!(!dir.join("THIRD_PARTY_NOTICES.new").exists());

            // A second update replaces the leftover of the first.
            replace(&exe, &zip_of(&[("Panopin.exe", "newer program")])).unwrap();
            assert_eq!(fs::read_to_string(&exe).unwrap(), "newer program");
            assert_eq!(
                fs::read_to_string(dir.join("Panopin.old")).unwrap(),
                "new program"
            );
        }
    }
}

/// The installer sets the old program aside as `panopin.<n>.old` when antivirus software holds it,
/// and deletes it when it is done (D-081); one the antivirus still held then goes at this start.
#[cfg(windows)]
mod installed {
    use std::{fs, path::Path};

    pub fn tidy() {
        let Ok(exe) = std::env::current_exe() else {
            return;
        };
        if let (Some(dir), Some(stem)) = (exe.parent(), exe.file_stem().and_then(|s| s.to_str())) {
            tidy_in(dir, stem);
        }
    }

    fn tidy_in(dir: &Path, stem: &str) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        let prefix = format!("{}.", stem.to_ascii_lowercase());
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if name.starts_with(&prefix) && name.ends_with(".old") && name.len() > prefix.len() + 4
            {
                if let Err(err) = fs::remove_file(entry.path()) {
                    log::warn!("a program the installer set aside could not be removed: {err}");
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn removes_only_what_the_installer_set_aside() {
            let dir = tempfile::tempdir().unwrap();
            for name in [
                "panopin.exe",
                "panopin.204321625.old",
                "Panopin.7.OLD",
                "uninstall.exe",
                "notes.old",
            ] {
                fs::write(dir.path().join(name), "x").unwrap();
            }
            tidy_in(dir.path(), "panopin");
            let mut left: Vec<String> = fs::read_dir(dir.path())
                .unwrap()
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect();
            left.sort();
            assert_eq!(left, ["notes.old", "panopin.exe", "uninstall.exe"]);
        }
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
        // Test binaries are debug builds outside any installation or app bundle.
        assert!(!installable());
    }
}
