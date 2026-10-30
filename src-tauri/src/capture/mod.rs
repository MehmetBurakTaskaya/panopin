//! Capture pipeline (ARCHITECTURE §5): platform listeners feed one worker thread, which applies
//! the admission rules and writes to the store. Clipboard content never reaches the logs.

// A few helpers and image formats only serve the Windows listeners.
#![cfg_attr(not(windows), allow(dead_code))]

pub mod admission;
#[cfg(target_os = "macos")]
pub(crate) mod clipboard_mac;
#[cfg(windows)]
pub(crate) mod clipboard_win;
#[cfg(windows)]
mod folder_watcher;
#[cfg(target_os = "macos")]
#[path = "folder_watcher_mac.rs"]
mod folder_watcher;

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Sender},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use image::RgbaImage;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    error::Result,
    media::{decode, thumbnails, video, MediaStore},
    settings::PAUSED_UNTIL_RESUMED,
    store::{
        identity,
        items::{self, Counts, ImageRow, Kind, Source, TextCapture, VideoRow},
        retention,
    },
    tray, AppState,
};
use admission::{Decision, Policy, RawImage, Snapshot};

pub enum Event {
    Clipboard(Snapshot),
    File(PathBuf),
}

/// Retention runs at start and then on this interval; the sleeping thread costs no CPU.
const SWEEP_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
const IMAGE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "bmp", "gif", "webp"];
const VIDEO_EXTENSIONS: [&str; 1] = ["mp4"];

pub struct CaptureState {
    tx: Sender<Event>,
    media: MediaStore,
    watchers: std::sync::Mutex<Vec<folder_watcher::Watcher>>,
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

pub fn start(app: &AppHandle, data_dir: &Path) -> Result<()> {
    let media = MediaStore::new(data_dir.join("media"));
    media.clean_temp();
    let (tx, rx) = mpsc::channel();
    let worker_app = app.clone();
    let worker_media = media.clone();
    thread::Builder::new()
        .name("capture".into())
        .spawn(move || {
            let mut imported = HashMap::new();
            for event in rx {
                handle(&worker_app, &worker_media, &mut imported, event);
            }
        })?;
    #[cfg(windows)]
    clipboard_win::spawn(tx.clone())?;
    #[cfg(target_os = "macos")]
    clipboard_mac::spawn(tx.clone())?;
    app.manage(CaptureState {
        tx,
        media,
        watchers: std::sync::Mutex::new(Vec::new()),
    });
    reconfigure(app);
    resume_timer(app);
    let sweep_app = app.clone();
    thread::Builder::new()
        .name("retention".into())
        .spawn(move || loop {
            sweep(&sweep_app);
            thread::sleep(SWEEP_EVERY);
        })?;
    Ok(())
}

/// (Re)starts the screenshot-folder watchers from the current settings.
pub fn reconfigure(app: &AppHandle) {
    let settings = app.state::<AppState>().settings();
    let state = app.state::<CaptureState>();
    let mut watchers = state
        .watchers
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    watchers.clear(); // dropping a watcher stops it
    let sources = &settings.sources;
    if !settings.onboarded || !(sources.screenshots || sources.screen_recordings) {
        return;
    }
    // Each folder, and whether only the pictures the system marked as screenshots count there.
    let mut folders = Vec::new();
    if sources.screenshots {
        match folder_watcher::screenshots_folder() {
            Some(dir) => folders.push((dir, true)),
            None => log::warn!("the screenshot folder could not be resolved"),
        }
    }
    // macOS saves recordings as .mov, which Panopin does not keep yet (D-073).
    #[cfg(windows)]
    if sources.screen_recordings {
        match folder_watcher::recordings_folder() {
            Some(dir) => folders.push((dir, false)),
            None => log::warn!("the Screen Recordings folder could not be resolved"),
        }
    }
    // Extra folders hold screenshots, recordings or both; each file goes by its source, and every
    // picture in them counts.
    for dir in settings.watched_folders.iter().map(PathBuf::from) {
        match folders.iter_mut().find(|(known, _)| *known == dir) {
            Some((_, screenshots_only)) => *screenshots_only = false,
            None => folders.push((dir, false)),
        }
    }
    for (dir, screenshots_only) in folders {
        match folder_watcher::watch(dir, screenshots_only, state.tx.clone()) {
            Ok(watcher) => watchers.push(watcher),
            Err(err) => log::warn!("cannot watch a screenshot folder: {err}"),
        }
    }
}

/// Pauses capture until `until` (Unix ms; `PAUSED_UNTIL_RESUMED` = until resumed) or resumes it.
pub fn set_pause(app: &AppHandle, until: Option<i64>) -> Result<()> {
    crate::apply_settings(app, json!({ "pausedUntil": until }))?;
    resume_timer(app);
    Ok(())
}

/// Resumes a timed pause when it runs out; a stale timer finds a different value and does nothing.
fn resume_timer(app: &AppHandle) {
    let Some(until) = app.state::<AppState>().settings().paused_until else {
        return;
    };
    if until == PAUSED_UNTIL_RESUMED {
        return;
    }
    let app = app.clone();
    thread::spawn(move || {
        let wait = u64::try_from(until.saturating_sub(now_ms())).unwrap_or(0);
        thread::sleep(Duration::from_millis(wait));
        if app.state::<AppState>().settings().paused_until == Some(until) {
            if let Err(err) = set_pause(&app, None) {
                log::warn!("could not resume capture: {err}");
            }
        }
    });
}

/// The managed media folder, once capture has started.
pub fn media(app: &AppHandle) -> Option<MediaStore> {
    app.try_state::<CaptureState>()
        .map(|state| state.media.clone())
}

pub fn counts(app: &AppHandle) -> Counts {
    app.state::<AppState>()
        .with_db(|db| items::counts(db))
        .unwrap_or_default()
}

pub(crate) fn changed(app: &AppHandle) {
    // New or edited images wait for their text to be read, new links for their preview.
    crate::ocr::wake(app);
    crate::links::wake(app);
    let counts = counts(app);
    tray::show_counts(app, counts);
    if let Err(err) = app.emit("items:changed", counts) {
        log::warn!("could not announce new items: {err}");
    }
}

fn handle(
    app: &AppHandle,
    media: &MediaStore,
    imported: &mut HashMap<PathBuf, (u64, SystemTime)>,
    event: Event,
) {
    let settings = app.state::<AppState>().settings();
    let now = now_ms();
    let result = match event {
        Event::Clipboard(snapshot) => {
            let source_app = snapshot
                .owner_exe
                .clone()
                .or_else(|| snapshot.foreground_exe.clone());
            let policy = Policy {
                onboarded: settings.onboarded,
                paused: settings.is_paused(now),
                sources: &settings.sources,
                excluded_apps: &settings.excluded_apps,
            };
            match admission::decide(snapshot, &policy) {
                Decision::Skip(reason) => {
                    log::debug!("clipboard change skipped: {reason:?}");
                    return;
                }
                Decision::Text(text) => {
                    save_text(app, Kind::Text, &text, None, source_app.as_deref(), now)
                }
                Decision::Link { text, url } => save_text(
                    app,
                    Kind::Link,
                    &text,
                    Some(&url),
                    source_app.as_deref(),
                    now,
                ),
                Decision::Image(raw) => {
                    save_clipboard_image(app, media, raw, source_app.as_deref(), now).map(|id| {
                        // The picture the snip shortcut was waiting for opens in the crop editor.
                        if crate::snip::take(app) {
                            if let Err(err) = crate::shelf::open_on(app, id, "crop") {
                                log::warn!("the snip could not be opened: {err}");
                            }
                        }
                    })
                }
            }
        }
        Event::File(path) => {
            if !settings.onboarded || settings.is_paused(now) {
                return;
            }
            if is_video_file(&path.to_string_lossy()) {
                if !settings.sources.screen_recordings
                    || written(&path, imported, VIDEO_WAIT_STEPS).is_none()
                {
                    return;
                }
                save_video(app, media, &path, Source::Folder, now)
            } else {
                if !settings.sources.screenshots {
                    return;
                }
                let Some(bytes) = ready_file(&path, imported) else {
                    return;
                };
                save_file_image(app, media, &path, &bytes, Source::Folder, now)
            }
        }
    };
    match result {
        Ok(()) => changed(app),
        Err(err) => log::warn!("capture failed: {err}"),
    }
}

fn save_text(
    app: &AppHandle,
    kind: Kind,
    body: &str,
    url: Option<&str>,
    source_app: Option<&str>,
    now: i64,
) -> Result<()> {
    // A link's identity is its URL, so outer whitespace never creates a second card.
    let identity = identity::text_identity(url.unwrap_or(body));
    let capture = TextCapture {
        kind,
        body,
        url,
        identity,
        source_app,
    };
    app.state::<AppState>()
        .with_db(|db| items::upsert_text(db, &capture, now))
        .map(drop)
}

struct NewImage<'a> {
    pixels: RgbaImage,
    original: &'a [u8],
    ext: &'a str,
    source: Source,
    source_app: Option<&'a str>,
    source_path: Option<&'a str>,
}

fn save_clipboard_image(
    app: &AppHandle,
    media: &MediaStore,
    raw: RawImage,
    source_app: Option<&str>,
    now: i64,
) -> Result<i64> {
    let (pixels, original) = match raw {
        RawImage::Png(bytes) => (decode::decode(&bytes)?, bytes),
        RawImage::Dib(bytes) => {
            let pixels = decode::decode_dib(&bytes)?;
            let png = decode::encode_png(&pixels)?;
            (pixels, png)
        }
        RawImage::Encoded(bytes) => {
            let pixels = decode::decode(&bytes)?;
            let png = decode::encode_png(&pixels)?;
            (pixels, png)
        }
    };
    let image = NewImage {
        pixels,
        original: &original,
        ext: "png",
        source: Source::Clipboard,
        source_app,
        source_path: None,
    };
    save_image(app, media, &image, now)
}

fn save_file_image(
    app: &AppHandle,
    media: &MediaStore,
    path: &Path,
    bytes: &[u8],
    source: Source,
    now: i64,
) -> Result<()> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_else(|| "png".into());
    let path_text = path.to_string_lossy();
    let image = NewImage {
        pixels: decode::decode(bytes)?,
        original: bytes,
        ext: &ext,
        source,
        source_app: None,
        source_path: Some(&path_text),
    };
    save_image(app, media, &image, now).map(drop)
}

/// Stores a picture, or brings the same one forward; returns its id.
fn save_image(app: &AppHandle, media: &MediaStore, image: &NewImage<'_>, now: i64) -> Result<i64> {
    let state = app.state::<AppState>();
    let (width, height) = image.pixels.dimensions();
    let identity = identity::image_identity(width, height, image.pixels.as_raw());
    if let Some(id) = state.with_db(|db| items::find(db, Kind::Image, &identity))? {
        state.with_db(|db| items::bump(db, id, now))?;
        return Ok(id);
    }
    let thumb = thumbnails::make(&image.pixels)?;
    let row = ImageRow {
        identity,
        width,
        height,
        byte_size: image.original.len() as u64,
        media_ext: image.ext,
        thumb_ext: thumb.ext,
        source: image.source,
        source_app: image.source_app,
        source_path: image.source_path,
    };
    let id = state.with_db(|db| items::insert_image(db, &row, now))?;
    if let Err(err) = media.write(id, image.original, image.ext, &thumb) {
        let _ = state.with_db(|db| items::delete(db, id));
        return Err(err);
    }
    Ok(id)
}

/// The shortcut "Copy the last image": the newest picture goes back on the clipboard.
pub fn copy_last_image(app: &AppHandle) -> Result<()> {
    let newest: Option<i64> = app.state::<AppState>().with_db(|db| {
        use rusqlite::OptionalExtension;
        Ok(db
            .query_row(
                "SELECT id FROM items WHERE kind = 'image' AND trashed_at IS NULL
                 ORDER BY last_copied_at DESC, id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?)
    })?;
    match newest {
        Some(id) => crate::transfer::copy(app, &[id]),
        None => Ok(()),
    }
}

/// The shortcut "Pause or resume capture": pauses until resumed, or resumes.
pub fn toggle_pause(app: &AppHandle) -> Result<()> {
    let paused = app.state::<AppState>().settings().is_paused(now_ms());
    set_pause(app, (!paused).then_some(PAUSED_UNTIL_RESUMED))
}

/// A recording: hashed and probed where it lies, then copied into managed storage (D-058).
fn save_video(
    app: &AppHandle,
    media: &MediaStore,
    path: &Path,
    source: Source,
    now: i64,
) -> Result<()> {
    let state = app.state::<AppState>();
    let identity = identity::file_identity(path)?;
    if let Some(id) = state.with_db(|db| items::find(db, Kind::Video, &identity))? {
        return state.with_db(|db| items::bump(db, id, now));
    }
    let facts = video::probe(path);
    let thumb = facts.poster.as_ref().map(thumbnails::make).transpose()?;
    // Without frame-size properties the poster still gives the card its shape.
    let poster_size = facts.poster.as_ref().map(RgbaImage::dimensions);
    let path_text = path.to_string_lossy();
    let row = VideoRow {
        identity,
        width: facts.width.or(poster_size.map(|s| s.0)),
        height: facts.height.or(poster_size.map(|s| s.1)),
        byte_size: fs::metadata(path)?.len(),
        duration_ms: facts.duration_ms,
        thumb_ext: thumb.as_ref().map(|t| t.ext),
        source,
        source_path: Some(&path_text),
    };
    let id = state.with_db(|db| items::insert_video(db, &row, now))?;
    if let Err(err) = media.write_file(id, path, "mp4", thumb.as_ref()) {
        let _ = state.with_db(|db| items::delete(db, id));
        return Err(err);
    }
    Ok(())
}

/// "Import…" in the Library: pictures and recordings the user chose. The files are only read.
/// Returns how many were added (or brought forward when already there).
pub fn import(app: &AppHandle, paths: &[PathBuf]) -> Result<usize> {
    let media = media(app).ok_or(crate::error::AppError::NotReady)?;
    let now = now_ms();
    let mut count = 0;
    for path in paths {
        let name = path.to_string_lossy();
        let result = if is_video_file(&name) {
            save_video(app, &media, path, Source::Import, now)
        } else if is_image_file(&name) {
            fs::read(path)
                .map_err(Into::into)
                .and_then(|bytes| save_file_image(app, &media, path, &bytes, Source::Import, now))
        } else {
            continue;
        };
        match result {
            Ok(()) => count += 1,
            Err(err) => log::warn!("an imported file was skipped: {err}"),
        }
    }
    if count > 0 {
        changed(app);
    }
    Ok(count)
}

fn sweep(app: &AppHandle) {
    let state = app.state::<AppState>();
    let policy = state.settings().retention.policy();
    match state.with_db(|db| retention::sweep(db, &policy, now_ms())) {
        Ok(report) => {
            let media = &app.state::<CaptureState>().media;
            for id in &report.purged {
                media.remove(*id);
            }
            if report.trashed > 0 || !report.purged.is_empty() {
                log::info!(
                    "retention: {} to Trash, {} deleted",
                    report.trashed,
                    report.purged.len()
                );
                changed(app);
            }
        }
        Err(err) => log::warn!("retention failed: {err}"),
    }
}

pub(crate) fn is_video_file(name: &str) -> bool {
    Path::new(name).extension().is_some_and(|ext| {
        VIDEO_EXTENSIONS
            .iter()
            .any(|known| ext.eq_ignore_ascii_case(known))
    })
}

/// Files a watched folder reports: pictures and recordings.
pub(crate) fn is_capture_file(name: &str) -> bool {
    is_image_file(name) || is_video_file(name)
}

pub(crate) fn is_image_file(name: &str) -> bool {
    Path::new(name).extension().is_some_and(|ext| {
        IMAGE_EXTENSIONS
            .iter()
            .any(|known| ext.eq_ignore_ascii_case(known))
    })
}

/// Text from a `CF_UNICODETEXT` buffer: UTF-16LE up to the first NUL.
pub(crate) fn utf16_until_nul(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    let end = units
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}

/// Steps of 200 ms to wait for a file to be written: about 10 s for a screenshot, a minute for a
/// recording, which the Snipping Tool writes when the recording stops.
const IMAGE_WAIT_STEPS: u32 = 50;
const VIDEO_WAIT_STEPS: u32 = 300;

/// Waits until a new file is completely written: its size holds still and nothing else has it
/// open. This bounded wait only runs after a folder event. Files already imported with the same
/// size and time are skipped (`None`).
fn written(
    path: &Path,
    imported: &mut HashMap<PathBuf, (u64, SystemTime)>,
    steps: u32,
) -> Option<(u64, SystemTime)> {
    let mut last_len = None;
    for _ in 0..steps {
        let meta = fs::metadata(path).ok()?;
        let stamp = (meta.len(), meta.modified().ok()?);
        if meta.len() > 0 && last_len == Some(meta.len()) && unlocked(path) {
            if imported.get(path) == Some(&stamp) {
                return None;
            }
            if imported.len() > 256 {
                imported.clear();
            }
            imported.insert(path.to_path_buf(), stamp);
            return Some(stamp);
        }
        last_len = Some(meta.len());
        thread::sleep(Duration::from_millis(200));
    }
    None
}

fn ready_file(path: &Path, imported: &mut HashMap<PathBuf, (u64, SystemTime)>) -> Option<Vec<u8>> {
    written(path, imported, IMAGE_WAIT_STEPS)?;
    fs::read(path).ok()
}

#[cfg(windows)]
fn unlocked(path: &Path) -> bool {
    use std::os::windows::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(path)
        .is_ok()
}

#[cfg(not(windows))]
fn unlocked(_: &Path) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_image_files_are_watched() {
        for name in [
            "Screenshot 2026-10-29.png",
            "shot.JPG",
            "a.jpeg",
            "b.webp",
            "c.gif",
            "d.bmp",
        ] {
            assert!(is_image_file(name), "{name}");
        }
        for name in ["notes.txt", "clip.mp4", "png", "archive.png.zip"] {
            assert!(!is_image_file(name), "{name}");
        }
    }

    #[test]
    fn unicode_text_stops_at_nul() {
        let mut bytes: Vec<u8> = "İstanbul ₺\r\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        bytes.extend_from_slice(&[0, 0, 0x41, 0]);
        assert_eq!(utf16_until_nul(&bytes), "İstanbul ₺\r\n");
        assert_eq!(utf16_until_nul(&[0x41, 0]), "A");
    }

    #[test]
    fn finished_files_are_read_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("shot.png");
        fs::write(&path, b"pixels").unwrap();
        let mut imported = HashMap::new();
        assert_eq!(
            ready_file(&path, &mut imported).as_deref(),
            Some(&b"pixels"[..])
        );
        assert_eq!(ready_file(&path, &mut imported), None, "same file again");
        assert_eq!(
            ready_file(&dir.path().join("missing.png"), &mut imported),
            None
        );
    }
}
