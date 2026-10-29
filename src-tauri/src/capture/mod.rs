//! Capture pipeline (ARCHITECTURE §5): platform listeners feed one worker thread, which applies
//! the admission rules and writes to the store. Clipboard content never reaches the logs.

// Only Windows has listeners so far; the macOS ones arrive in Phase 8 and feed the same pipeline.
#![cfg_attr(not(windows), allow(dead_code))]

pub mod admission;
#[cfg(windows)]
mod clipboard_win;
#[cfg(windows)]
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
    media::{decode, thumbnails, MediaStore},
    settings::PAUSED_UNTIL_RESUMED,
    store::{
        identity,
        items::{self, Counts, ImageRow, Kind, Source, TextCapture},
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

pub struct CaptureState {
    tx: Sender<Event>,
    media: MediaStore,
    #[cfg(windows)]
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
    app.manage(CaptureState {
        tx,
        media,
        #[cfg(windows)]
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
    #[cfg(windows)]
    {
        let settings = app.state::<AppState>().settings();
        let state = app.state::<CaptureState>();
        let mut watchers = state
            .watchers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        watchers.clear(); // dropping a watcher stops it
        if !settings.onboarded || !settings.sources.screenshots {
            return;
        }
        let mut folders = Vec::new();
        match folder_watcher::screenshots_folder() {
            Some(dir) => folders.push(dir),
            None => log::warn!("the Windows Screenshots folder could not be resolved"),
        }
        folders.extend(settings.watched_folders.iter().map(PathBuf::from));
        for dir in folders {
            match folder_watcher::watch(dir, state.tx.clone()) {
                Ok(watcher) => watchers.push(watcher),
                Err(err) => log::warn!("cannot watch a screenshot folder: {err}"),
            }
        }
    }
    #[cfg(not(windows))]
    let _ = app;
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

pub fn counts(app: &AppHandle) -> Counts {
    app.state::<AppState>()
        .with_db(|db| items::counts(db))
        .unwrap_or_default()
}

pub(crate) fn changed(app: &AppHandle) {
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
                    save_clipboard_image(app, media, raw, source_app.as_deref(), now)
                }
            }
        }
        Event::File(path) => {
            if !settings.onboarded || !settings.sources.screenshots || settings.is_paused(now) {
                return;
            }
            let Some(bytes) = ready_file(&path, imported) else {
                return;
            };
            save_file_image(app, media, &path, &bytes, now)
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
) -> Result<()> {
    let (pixels, original) = match raw {
        RawImage::Png(bytes) => (decode::decode(&bytes)?, bytes),
        RawImage::Dib(bytes) => {
            let pixels = decode::decode_dib(&bytes)?;
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
        source: Source::Folder,
        source_app: None,
        source_path: Some(&path_text),
    };
    save_image(app, media, &image, now)
}

fn save_image(app: &AppHandle, media: &MediaStore, image: &NewImage<'_>, now: i64) -> Result<()> {
    let state = app.state::<AppState>();
    let (width, height) = image.pixels.dimensions();
    let identity = identity::image_identity(width, height, image.pixels.as_raw());
    if let Some(id) = state.with_db(|db| items::find(db, Kind::Image, &identity))? {
        return state.with_db(|db| items::bump(db, id, now));
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
    Ok(())
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

/// Waits until a new screenshot file is completely written: its size holds still and nothing
/// else has it open. This bounded wait only runs after a folder event. Files already imported
/// with the same size and time are skipped.
fn ready_file(path: &Path, imported: &mut HashMap<PathBuf, (u64, SystemTime)>) -> Option<Vec<u8>> {
    let mut last_len = None;
    for _ in 0..50 {
        let meta = fs::metadata(path).ok()?;
        let stamp = (meta.len(), meta.modified().ok()?);
        if meta.len() > 0 && last_len == Some(meta.len()) && unlocked(path) {
            if imported.get(path) == Some(&stamp) {
                return None;
            }
            let bytes = fs::read(path).ok()?;
            if imported.len() > 256 {
                imported.clear();
            }
            imported.insert(path.to_path_buf(), stamp);
            return Some(bytes);
        }
        last_len = Some(meta.len());
        thread::sleep(Duration::from_millis(200));
    }
    None
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
