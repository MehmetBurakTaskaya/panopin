//! Text in images (D-056). Windows OCR reads every image once, newest first, on a background
//! thread with the lowest priority; the text goes into the search index, and Ctrl+Shift+C copies
//! it. Edited images are read as they look now, so a pixelated area stays unreadable.
//!
//! The thread sleeps until something wakes it (a new or edited image, a settings change); it never
//! polls.
#![cfg_attr(not(windows), allow(dead_code))]

#[cfg(windows)]
mod engine_win;

use std::sync::{Condvar, Mutex, PoisonError};

use image::RgbaImage;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::{
    error::{AppError, Result},
    store::search,
    transfer, AppState,
};

#[derive(Default)]
pub struct OcrQueue {
    woken: Mutex<bool>,
    signal: Condvar,
}

impl OcrQueue {
    fn wake(&self) {
        *self.woken.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.signal.notify_one();
    }

    fn wait(&self) {
        let mut woken = self.woken.lock().unwrap_or_else(PoisonError::into_inner);
        while !*woken {
            woken = self
                .signal
                .wait(woken)
                .unwrap_or_else(PoisonError::into_inner);
        }
        *woken = false;
    }
}

/// An OCR language installed in Windows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrLanguage {
    pub tag: String,
    pub name: String,
}

/// What Settings shows about OCR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrStatus {
    pub supported: bool,
    pub languages: Vec<OcrLanguage>,
    /// The language images are read in: Turkish, else English, else the user's language.
    pub reading: Option<String>,
}

pub fn status() -> OcrStatus {
    #[cfg(windows)]
    {
        engine_win::init_thread(false);
        let languages = engine_win::languages().unwrap_or_default();
        OcrStatus {
            supported: true,
            reading: engine_win::Engine::new().map(|engine| engine.language),
            languages,
        }
    }
    #[cfg(not(windows))]
    OcrStatus {
        supported: false,
        languages: Vec::new(),
        reading: None,
    }
}

pub fn start(app: &AppHandle) -> Result<()> {
    app.manage(OcrQueue::default());
    let worker = app.clone();
    std::thread::Builder::new()
        .name("ocr".into())
        .spawn(move || run(&worker))?;
    wake(app);
    Ok(())
}

/// Looks for work: called after captures, edits and settings changes.
pub fn wake(app: &AppHandle) {
    if let Some(queue) = app.try_state::<OcrQueue>() {
        queue.wake();
    }
}

/// The next image waiting to be read, newest first, with the edit time it was read at.
fn next_job(conn: &Connection) -> Result<Option<(i64, i64)>> {
    Ok(conn
        .query_row(
            "SELECT id, COALESCE(edited_at, 0) FROM items
             WHERE kind = 'image' AND ocr_state = 0 AND trashed_at IS NULL
             ORDER BY last_copied_at DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?)
}

/// Stores the text unless the image was edited meanwhile (it is then read again).
fn store(conn: &Connection, id: i64, edited_at: i64, text: Option<&str>) -> Result<()> {
    let state = if text.is_some() { 1 } else { 2 };
    let changed = conn.execute(
        "UPDATE items SET ocr_state = ?3, ocr_text = ?4
         WHERE id = ?1 AND COALESCE(edited_at, 0) = ?2",
        params![id, edited_at, state, text],
    )?;
    if changed == 1 && text.is_some_and(|t| !t.is_empty()) {
        search::index(conn, id)?;
    }
    Ok(())
}

fn pixels(app: &AppHandle, id: i64) -> Result<Option<RgbaImage>> {
    let media = crate::capture::media(app).ok_or(AppError::NotReady)?;
    let items = app
        .state::<AppState>()
        .with_db(|db| transfer::gather_items(db, &media, &[id]))?;
    match items.into_iter().next() {
        Some(transfer::Outgoing::Image(image)) => Ok(Some(image.pixels()?)),
        _ => Ok(None),
    }
}

#[cfg(windows)]
fn run(app: &AppHandle) {
    engine_win::init_thread(true);
    let queue = app.state::<OcrQueue>();
    let mut engine: Option<engine_win::Engine> = None;
    loop {
        queue.wait();
        loop {
            let state = app.state::<AppState>();
            if !state.settings().ocr {
                break;
            }
            let Ok(Some((id, edited_at))) = state.with_db(|db| next_job(db)) else {
                break;
            };
            if engine.is_none() {
                engine = engine_win::Engine::new();
            }
            // Without an OCR language the images wait; a later wake tries again.
            let Some(reader) = engine.as_ref() else {
                log::info!("no Windows OCR language is installed; images wait");
                break;
            };
            let text = match pixels(app, id) {
                Ok(Some(pixels)) => reader.recognize(&pixels).map_err(|err| {
                    log::warn!("OCR of item {id} failed: {err}");
                }),
                Ok(None) => Err(()),
                Err(err) => {
                    log::warn!("item {id} could not be read for OCR: {err}");
                    Err(())
                }
            };
            let text = text.ok();
            if let Err(err) = state.with_db(|db| store(db, id, edited_at, text.as_deref())) {
                log::error!("OCR result of item {id} could not be stored: {err}");
                break;
            }
        }
    }
}

#[cfg(not(windows))]
fn run(app: &AppHandle) {
    // Apple Vision arrives in Phase 8; until then the queue only sleeps.
    let queue = app.state::<OcrQueue>();
    loop {
        queue.wait();
    }
}

/// What Ctrl+Shift+C found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TextCopy {
    Copied,
    /// The image holds no readable text.
    NoText,
    /// Not an image, or OCR is not available here.
    Unavailable,
}

/// Ctrl+Shift+C: copies the text in an image, reading it first if the queue has not yet.
pub fn copy_text(app: &AppHandle, id: i64) -> Result<TextCopy> {
    let found: Option<(String, i64, Option<String>, i64)> =
        app.state::<AppState>().with_db(|db| {
            Ok(db
                .query_row(
                    "SELECT kind, ocr_state, ocr_text, COALESCE(edited_at, 0) FROM items WHERE id = ?1",
                    [id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .optional()?)
        })?;
    let Some((kind, state, text, edited_at)) = found else {
        return Ok(TextCopy::Unavailable);
    };
    if kind != "image" {
        return Ok(TextCopy::Unavailable);
    }
    let text = if state == 1 {
        text.unwrap_or_default()
    } else {
        match read_now(app, id)? {
            Some(text) => {
                app.state::<AppState>()
                    .with_db(|db| store(db, id, edited_at, Some(&text)))?;
                text
            }
            None => return Ok(TextCopy::Unavailable),
        }
    };
    if text.trim().is_empty() {
        return Ok(TextCopy::NoText);
    }
    transfer::copy_text(app, &text)?;
    Ok(TextCopy::Copied)
}

#[cfg(windows)]
fn read_now(app: &AppHandle, id: i64) -> Result<Option<String>> {
    engine_win::init_thread(false);
    let Some(engine) = engine_win::Engine::new() else {
        return Ok(None);
    };
    match pixels(app, id)? {
        Some(pixels) => Ok(Some(engine.recognize(&pixels)?)),
        None => Ok(None),
    }
}

#[cfg(not(windows))]
fn read_now(_app: &AppHandle, _id: i64) -> Result<Option<String>> {
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store;

    fn image(conn: &Connection, copied: i64) -> i64 {
        conn.execute(
            "INSERT INTO items (kind, identity, created_at, last_copied_at, source, media_ext)
             VALUES ('image', randomblob(32), ?1, ?1, 'clipboard', 'png')",
            [copied],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn the_newest_waiting_image_comes_first_and_edits_send_it_back() {
        let conn = store::open_in_memory().unwrap();
        let old = image(&conn, 1);
        let new = image(&conn, 2);
        assert_eq!(next_job(&conn).unwrap(), Some((new, 0)));
        store(&conn, new, 0, Some("Fiş toplamı")).unwrap();
        assert_eq!(next_job(&conn).unwrap(), Some((old, 0)));
        assert_eq!(search::find(&conn, "fiş", false).unwrap(), vec![new]);

        // An edit while the old image was being read: the stale text is not stored.
        conn.execute("UPDATE items SET edited_at = 5 WHERE id = ?1", [old])
            .unwrap();
        store(&conn, old, 0, Some("stale")).unwrap();
        assert_eq!(next_job(&conn).unwrap(), Some((old, 5)));
        store(&conn, old, 5, None).unwrap();
        assert_eq!(next_job(&conn).unwrap(), None, "a failure is not retried");
    }
}
