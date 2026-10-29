//! Items leaving Panopin: copying to the clipboard, pasting into the previous app, dragging.
//!
//! - Texts and links are joined in selection order, one per line.
//! - One image goes on the clipboard as PNG and DIB at full resolution, edited images with their
//!   crop and marks drawn in (D-050).
//! - Several images become a file list (`CF_HDROP`) of neutrally named copies in the staging
//!   folder; the first one is also put on as PNG and DIB.
//! - A drag carries the text and the images as files the same way.
#![cfg_attr(not(windows), allow(dead_code))]

#[cfg(windows)]
mod clipboard_win;
#[cfg(windows)]
mod drag_win;
#[cfg(windows)]
mod folder_win;
#[cfg(windows)]
mod paste_win;
pub mod staging;

use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

use image::{ImageFormat, RgbaImage};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::{
    error::{AppError, Result},
    media::{annotations, decode, flatten, MediaStore},
    store::items::Kind,
};

pub use staging::{Staged, Staging};

/// A stored image on its way out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageSource {
    pub original: PathBuf,
    /// It has an edit layer next to the original.
    pub edited: bool,
}

impl ImageSource {
    fn dir(&self) -> &Path {
        self.original.parent().unwrap_or(Path::new("."))
    }

    /// The picture as it leaves Panopin: with its crop and marks when it was edited.
    pub(crate) fn pixels(&self) -> Result<RgbaImage> {
        let original = decode::decode(&fs::read(&self.original)?)?;
        if !self.edited {
            return Ok(original);
        }
        match annotations::load(self.dir())? {
            Some(layer) => flatten::flatten(&original, &layer),
            None => Ok(original),
        }
    }
}

/// What a selection puts on the clipboard, read from the store.
#[derive(Debug, Default, PartialEq)]
pub struct Selection {
    pub texts: Vec<String>,
    /// Images in selection order.
    pub images: Vec<ImageSource>,
    /// Recordings, handed out as files only (D-058).
    pub files: Vec<PathBuf>,
}

/// One selected item on its way out, in selection order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outgoing {
    Text(String),
    Link(String),
    Image(ImageSource),
    File(PathBuf),
}

pub fn gather(conn: &Connection, media: &MediaStore, ids: &[i64]) -> Result<Selection> {
    let mut selection = Selection::default();
    for item in gather_items(conn, media, ids)? {
        match item {
            Outgoing::Text(text) | Outgoing::Link(text) => selection.texts.push(text),
            Outgoing::Image(image) => selection.images.push(image),
            Outgoing::File(path) => selection.files.push(path),
        }
    }
    Ok(selection)
}

pub fn gather_items(conn: &Connection, media: &MediaStore, ids: &[i64]) -> Result<Vec<Outgoing>> {
    let mut items = Vec::with_capacity(ids.len());
    let mut stmt = conn.prepare(
        "SELECT kind, text_body, url, media_ext, edit_width IS NOT NULL FROM items WHERE id = ?1",
    )?;
    for &id in ids {
        let row = stmt
            .query_row([id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, bool>(4)?,
                ))
            })
            .optional()?;
        match row {
            Some((kind, _, _, Some(ext), edited)) if kind == Kind::Image.as_str() => {
                items.push(Outgoing::Image(ImageSource {
                    original: media.original(id, &ext),
                    edited,
                }))
            }
            Some((kind, _, _, Some(ext), _)) if kind == Kind::Video.as_str() => {
                items.push(Outgoing::File(media.original(id, &ext)))
            }
            Some((kind, _, _, None, _)) if kind == Kind::Video.as_str() => {}
            // A link goes back as its clean URL; a text exactly as it was last copied.
            Some((kind, body, url, _, _)) if kind == Kind::Link.as_str() => {
                items.extend(url.or(body).map(Outgoing::Link))
            }
            Some((_, body, _, _, _)) => items.extend(body.map(Outgoing::Text)),
            None => {}
        }
    }
    Ok(items)
}

/// The file an item becomes when it leaves as a file (export, Alt+drag): UTF-8 text, an
/// Internet shortcut, or a PNG with any edits drawn in.
pub fn as_file(item: &Outgoing) -> Result<Staged> {
    Ok(match item {
        Outgoing::Text(text) => Staged::Bytes(text.as_bytes().to_vec(), "txt"),
        Outgoing::Link(url) => Staged::Bytes(
            format!("[InternetShortcut]\r\nURL={url}\r\n").into_bytes(),
            "url",
        ),
        Outgoing::Image(image)
            if !image.edited && image.original.extension().is_some_and(|e| e == "png") =>
        {
            Staged::Copy(image.original.clone())
        }
        Outgoing::Image(image) => Staged::Bytes(encode_png(&image.pixels()?)?, "png"),
        Outgoing::File(path) => Staged::Copy(path.clone()),
    })
}

/// The clipboard formats for one copy, ready to be written.
#[derive(Debug, Default)]
pub struct Formats {
    /// UTF-16, NUL-terminated (`CF_UNICODETEXT`).
    pub text: Option<Vec<u16>>,
    pub png: Option<Vec<u8>>,
    /// A packed DIB (`CF_DIB`): header and bottom-up 32-bit pixels.
    pub dib: Option<Vec<u8>>,
    /// `DROPFILES` and the paths (`CF_HDROP`).
    pub files: Option<Vec<u8>>,
}

impl Formats {
    pub fn is_empty(&self) -> bool {
        self.text.is_none() && self.png.is_none() && self.files.is_none()
    }
}

pub fn prepare(selection: &Selection, staging: &Staging) -> Result<Formats> {
    let mut formats = Formats {
        text: (!selection.texts.is_empty()).then(|| utf16z(&selection.texts.join("\r\n"))),
        ..Formats::default()
    };
    if let Some(first) = selection.images.first() {
        let pixels = first.pixels()?;
        formats.dib = Some(dib(&pixels));
        let stored_png =
            !first.edited && first.original.extension().is_some_and(|ext| ext == "png");
        formats.png = Some(if stored_png {
            fs::read(&first.original)?
        } else {
            encode_png(&pixels)?
        });
    }
    // Several images, or any recording: a file list as well.
    if selection.images.len() > 1 || !selection.files.is_empty() {
        let mut paths = if selection.images.len() > 1 {
            stage_images(&selection.images, staging)?
        } else {
            Vec::new()
        };
        paths.extend(stage_files(&selection.files, staging)?);
        formats.files = Some(dropfiles(&paths));
    }
    Ok(formats)
}

/// Neutrally named files for a file list or a drag: a copy of the stored file, or a PNG of the
/// edited picture.
pub fn stage_images(images: &[ImageSource], staging: &Staging) -> Result<Vec<PathBuf>> {
    let mut items = Vec::with_capacity(images.len());
    for image in images {
        items.push(if image.edited {
            Staged::Bytes(encode_png(&image.pixels()?)?, "png")
        } else {
            Staged::Copy(image.original.clone())
        });
    }
    staging.stage(&items)
}

/// Neutrally named copies of recordings.
pub fn stage_files(files: &[PathBuf], staging: &Staging) -> Result<Vec<PathBuf>> {
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let items: Vec<Staged> = files.iter().cloned().map(Staged::Copy).collect();
    staging.stage(&items)
}

fn encode_png(pixels: &RgbaImage) -> Result<Vec<u8>> {
    let mut out = Cursor::new(Vec::new());
    pixels.write_to(&mut out, ImageFormat::Png)?;
    Ok(out.into_inner())
}

pub fn utf16z(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// A packed 32-bit DIB with a `BITMAPINFOHEADER`. Apps that read DIBs ignore alpha, so
/// transparent pixels are laid on white; apps that understand alpha read the PNG instead.
pub fn dib(pixels: &RgbaImage) -> Vec<u8> {
    let (width, height) = pixels.dimensions();
    let size = width as usize * height as usize * 4;
    let mut out = Vec::with_capacity(40 + size);
    out.extend_from_slice(&40u32.to_le_bytes()); // biSize
    out.extend_from_slice(&(width as i32).to_le_bytes());
    out.extend_from_slice(&(height as i32).to_le_bytes()); // positive: bottom-up rows
    out.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
    out.extend_from_slice(&32u16.to_le_bytes()); // biBitCount
    out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    out.extend_from_slice(&(size as u32).to_le_bytes());
    out.extend_from_slice(&[0; 16]); // resolution and palette fields
    for row in pixels.rows().rev() {
        for pixel in row {
            let [r, g, b, a] = pixel.0;
            let on_white =
                |c: u8| ((u32::from(c) * u32::from(a) + 255 * (255 - u32::from(a))) / 255) as u8;
            out.extend_from_slice(&[on_white(b), on_white(g), on_white(r), 255]);
        }
    }
    out
}

/// `DROPFILES` (20 bytes, wide paths) followed by NUL-terminated paths and a final NUL.
pub fn dropfiles(paths: &[PathBuf]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&20u32.to_le_bytes()); // pFiles: offset of the path list
    out.extend_from_slice(&[0; 8]); // pt
    out.extend_from_slice(&0u32.to_le_bytes()); // fNC
    out.extend_from_slice(&1u32.to_le_bytes()); // fWide
    for path in paths {
        for unit in utf16z(&path.to_string_lossy()) {
            out.extend_from_slice(&unit.to_le_bytes());
        }
    }
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// What Ctrl+Enter did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PasteOutcome {
    /// The shelf closed and Ctrl+V went to the previous window.
    Pasted,
    /// The previous window is gone; nothing was copied or pasted.
    WindowGone,
    /// The previous window runs elevated, so input cannot reach it; the item is on the clipboard.
    Elevated,
    /// Pasting was refused after the shelf closed (a password field, or input was blocked); the
    /// shelf reopened with the reason.
    Refused,
}

fn formats_for(app: &AppHandle, ids: &[i64]) -> Result<Formats> {
    let media = crate::capture::media(app).ok_or(AppError::NotReady)?;
    let selection = app
        .state::<crate::AppState>()
        .with_db(|db| gather(db, &media, ids))?;
    prepare(&selection, &app.state::<Staging>())
}

/// Where the board is inside the shelf window, in CSS pixels (from the webview).
#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct BoardRect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

/// Starts a drag of the items (Windows: an OLE drag on the main thread). From the shelf, `board`
/// is where letting go cancels; `as_files` (Alt+drag in the Library) hands texts and links out as
/// files too. Returns whether something was dropped; the shelf then slides away without taking
/// focus back.
pub fn drag(
    app: &AppHandle,
    window: &WebviewWindow,
    ids: &[i64],
    board: Option<BoardRect>,
    as_files: bool,
) -> Result<bool> {
    let media = crate::capture::media(app).ok_or(AppError::NotReady)?;
    let staging = app.state::<Staging>();
    if as_files {
        let items = app
            .state::<crate::AppState>()
            .with_db(|db| gather_items(db, &media, ids))?;
        let files = items.iter().map(as_file).collect::<Result<Vec<_>>>()?;
        let paths = staging.stage(&files)?;
        return drag_now(app, window, board, None, paths);
    }
    let selection = app
        .state::<crate::AppState>()
        .with_db(|db| gather(db, &media, ids))?;
    let text = (!selection.texts.is_empty()).then(|| utf16z(&selection.texts.join("\r\n")));
    let mut files = stage_images(&selection.images, &staging)?;
    files.extend(stage_files(&selection.files, &staging)?);
    if text.is_none() && files.is_empty() {
        return Ok(false);
    }
    let dropped = drag_now(app, window, board, text, files)?;
    if dropped && window.label() == crate::shelf::LABEL {
        crate::shelf::request_close(app, false)?;
    }
    Ok(dropped)
}

/// Ctrl+Shift+E: asks for a folder, then writes one neutrally named file per item there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Exported {
    pub count: usize,
    pub folder: String,
}

pub fn export(app: &AppHandle, window: &WebviewWindow, ids: &[i64]) -> Result<Option<Exported>> {
    let media = crate::capture::media(app).ok_or(AppError::NotReady)?;
    let items = app
        .state::<crate::AppState>()
        .with_db(|db| gather_items(db, &media, ids))?;
    if items.is_empty() {
        return Ok(None);
    }
    let title = crate::i18n::t(crate::current_lang(app), "library.export.pick");
    // The folder dialog takes the focus from its window; the shelf must not close under it.
    let hold = (window.label() == crate::shelf::LABEL).then(|| crate::shelf::hold_open(app));
    let picked = pick_folder(app, window, title);
    drop(hold);
    let Some(folder) = picked? else {
        return Ok(None);
    };
    let files = items.iter().map(as_file).collect::<Result<Vec<_>>>()?;
    let written = staging::write_neutral(&folder, &files)?;
    Ok(Some(Exported {
        count: written.len(),
        folder: folder.display().to_string(),
    }))
}

#[cfg(windows)]
fn pick_folder(app: &AppHandle, window: &WebviewWindow, title: String) -> Result<Option<PathBuf>> {
    let (tx, rx) = std::sync::mpsc::channel();
    let owner = window.clone();
    // The dialog runs on the UI thread, owned by the window that asked for it.
    app.run_on_main_thread(move || {
        let result = owner
            .hwnd()
            .map_err(AppError::from)
            .and_then(|hwnd| folder_win::pick(hwnd.0 as isize, &title));
        let _ = tx.send(result);
    })?;
    rx.recv().map_err(|_| AppError::Interrupted)?
}

/// A folder chosen in the Windows dialog, with a title from the strings (`None` = cancelled).
pub fn pick_folder_titled(
    app: &AppHandle,
    window: &WebviewWindow,
    title_key: &str,
) -> Result<Option<PathBuf>> {
    let title = crate::i18n::t(crate::current_lang(app), title_key);
    pick_folder(app, window, title)
}

/// "Import…": pictures and recordings, chosen in the Windows file dialog.
#[cfg(windows)]
pub fn pick_import(app: &AppHandle, window: &WebviewWindow) -> Result<Vec<PathBuf>> {
    let lang = crate::current_lang(app);
    let title = crate::i18n::t(lang, "library.import.pick");
    let kind = crate::i18n::t(lang, "library.import.kind");
    let (tx, rx) = std::sync::mpsc::channel();
    let owner = window.clone();
    app.run_on_main_thread(move || {
        let result = owner.hwnd().map_err(AppError::from).and_then(|hwnd| {
            folder_win::pick_files(
                hwnd.0 as isize,
                &title,
                &kind,
                "*.png;*.jpg;*.jpeg;*.gif;*.bmp;*.webp;*.mp4",
            )
        });
        let _ = tx.send(result);
    })?;
    rx.recv().map_err(|_| AppError::Interrupted)?
}

#[cfg(not(windows))]
pub fn pick_import(_app: &AppHandle, _window: &WebviewWindow) -> Result<Vec<PathBuf>> {
    Err(AppError::Unsupported)
}

#[cfg(not(windows))]
fn pick_folder(
    _app: &AppHandle,
    _window: &WebviewWindow,
    _title: String,
) -> Result<Option<PathBuf>> {
    Err(AppError::Unsupported)
}

#[cfg(windows)]
fn drag_now(
    app: &AppHandle,
    window: &WebviewWindow,
    board: Option<BoardRect>,
    text: Option<Vec<u16>>,
    files: Vec<PathBuf>,
) -> Result<bool> {
    let origin = window.inner_position()?;
    let scale = window.scale_factor()?;
    let screen = |x: f64, base: i32| base + (x * scale).round() as i32;
    let overlay = board.is_some();
    let board = board.map_or([0; 4], |board| {
        [
            screen(board.left, origin.x),
            screen(board.top, origin.y),
            screen(board.right, origin.x),
            screen(board.bottom, origin.y),
        ]
    });
    let window = window.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        // The shelf covers the screen; while dragging it lets the pointer through, so the drop
        // reaches the app underneath.
        if overlay {
            let _ = window.set_ignore_cursor_events(true);
        }
        let result = window
            .hwnd()
            .map_err(AppError::from)
            .and_then(|hwnd| drag_win::run(hwnd.0 as isize, board, text.as_deref(), &files));
        if overlay {
            let _ = window.set_ignore_cursor_events(false);
        }
        let _ = tx.send(result);
    })?;
    rx.recv().map_err(|_| AppError::Interrupted)?
}

#[cfg(not(windows))]
fn drag_now(
    _app: &AppHandle,
    _window: &WebviewWindow,
    _board: Option<BoardRect>,
    _text: Option<Vec<u16>>,
    _files: Vec<PathBuf>,
) -> Result<bool> {
    Err(AppError::Unsupported)
}

/// Puts a plain text on the clipboard, e.g. the text read from an image (Ctrl+Shift+C).
pub fn copy_text(app: &AppHandle, text: &str) -> Result<()> {
    let formats = Formats {
        text: Some(utf16z(text)),
        ..Formats::default()
    };
    write(app, &formats)
}

/// Copies the items; the shelf stays open.
pub fn copy(app: &AppHandle, ids: &[i64]) -> Result<()> {
    let formats = formats_for(app, ids)?;
    if formats.is_empty() {
        return Ok(());
    }
    write(app, &formats)
}

#[cfg(windows)]
fn write(app: &AppHandle, formats: &Formats) -> Result<()> {
    let window = app
        .get_webview_window(crate::shelf::LABEL)
        .ok_or(AppError::NotReady)?;
    clipboard_win::write(window.hwnd()?.0 as isize, formats)
}

#[cfg(not(windows))]
fn write(_app: &AppHandle, _formats: &Formats) -> Result<()> {
    Err(AppError::Unsupported)
}

/// Ctrl+Enter: copies, goes back to the previous window and sends Ctrl+V there.
pub fn paste(app: &AppHandle, ids: &[i64]) -> Result<PasteOutcome> {
    let formats = formats_for(app, ids)?;
    let Some(target) = crate::shelf::previous_window(app).filter(|t| crate::focus::is_alive(*t))
    else {
        return Ok(PasteOutcome::WindowGone);
    };
    if formats.is_empty() {
        return Ok(PasteOutcome::WindowGone);
    }
    write(app, &formats)?;
    paste_into(app, target)
}

#[cfg(windows)]
fn paste_into(app: &AppHandle, target: crate::focus::Target) -> Result<PasteOutcome> {
    if paste_win::is_elevated_above_us(target.pid) {
        return Ok(PasteOutcome::Elevated);
    }
    crate::shelf::request_close(app, true)?;
    match paste_win::send_paste(target) {
        paste_win::Sent::Pasted => Ok(PasteOutcome::Pasted),
        paste_win::Sent::PasswordField => {
            crate::shelf::open_with_notice(app, "shelf.paste.password")?;
            Ok(PasteOutcome::Refused)
        }
        paste_win::Sent::Blocked => {
            crate::shelf::open_with_notice(app, "shelf.paste.failed")?;
            Ok(PasteOutcome::Refused)
        }
    }
}

#[cfg(not(windows))]
fn paste_into(_app: &AppHandle, _target: crate::focus::Target) -> Result<PasteOutcome> {
    Err(AppError::Unsupported)
}

#[cfg(test)]
mod tests {
    use image::Rgba;

    use super::*;
    use crate::store::{
        self,
        identity::text_identity,
        items::{upsert_text, TextCapture},
    };

    #[test]
    fn texts_and_links_keep_selection_order() {
        let conn = store::open_in_memory().unwrap();
        let add = |kind, body: &str, url: Option<&str>| {
            let capture = TextCapture {
                kind,
                body,
                url,
                identity: text_identity(url.unwrap_or(body)),
                source_app: None,
            };
            upsert_text(&conn, &capture, 1).unwrap()
        };
        let first = add(Kind::Text, "first\r\nline", None);
        let link = add(
            Kind::Link,
            " https://example.com/a \n",
            Some("https://example.com/a"),
        );
        let media = MediaStore::new(PathBuf::from("unused"));
        let selection = gather(&conn, &media, &[link, first, 999]).unwrap();
        assert_eq!(selection.texts, ["https://example.com/a", "first\r\nline"]);
        assert!(selection.images.is_empty());
        let staging = Staging::new(tempfile::tempdir().unwrap().path().to_path_buf());
        let formats = prepare(&selection, &staging).unwrap();
        assert_eq!(
            formats.text.unwrap(),
            utf16z("https://example.com/a\r\nfirst\r\nline")
        );
        assert!(formats.png.is_none() && formats.files.is_none());
    }

    #[test]
    fn dib_is_bottom_up_bgra_on_white() {
        let mut pixels = RgbaImage::new(2, 2);
        pixels.put_pixel(0, 0, Rgba([255, 0, 0, 255])); // top-left red
        pixels.put_pixel(1, 1, Rgba([0, 0, 0, 0])); // bottom-right transparent
        let bytes = dib(&pixels);
        assert_eq!(bytes.len(), 40 + 16);
        assert_eq!(&bytes[4..8], &2i32.to_le_bytes());
        assert_eq!(&bytes[14..16], &32u16.to_le_bytes());
        // Rows are stored bottom-up: the top row comes last.
        assert_eq!(&bytes[40 + 8..40 + 12], &[0, 0, 255, 255]);
        assert_eq!(&bytes[40 + 4..40 + 8], &[255, 255, 255, 255]);
    }

    #[test]
    fn several_images_become_staged_files_and_the_first_a_bitmap() {
        let dir = tempfile::tempdir().unwrap();
        let mut images = Vec::new();
        for (index, ext) in ["png", "bmp"].into_iter().enumerate() {
            let path = dir.path().join(format!("original-{index}.{ext}"));
            let pixels = RgbaImage::from_pixel(3, 2, Rgba([10, 20, 30, 255]));
            let format = if ext == "png" {
                ImageFormat::Png
            } else {
                ImageFormat::Bmp
            };
            pixels.save_with_format(&path, format).unwrap();
            images.push(path);
        }
        let staging = Staging::new(dir.path().join("staging"));
        let selection = Selection {
            texts: Vec::new(),
            images: images
                .iter()
                .map(|original| ImageSource {
                    original: original.clone(),
                    edited: false,
                })
                .collect(),
            files: Vec::new(),
        };
        let formats = prepare(&selection, &staging).unwrap();
        let png = formats.png.unwrap();
        assert_eq!(
            png,
            fs::read(&images[0]).unwrap(),
            "a PNG original is used as is"
        );
        assert!(formats.dib.is_some());
        let files = formats.files.unwrap();
        assert_eq!(&files[16..20], &1u32.to_le_bytes(), "wide paths");
        let mut staged: Vec<_> = fs::read_dir(dir.path().join("staging"))
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .collect();
        staged.sort();
        assert_eq!(staged.len(), 2);
        for (copy, original) in staged.iter().zip(&images) {
            let name = copy.file_name().unwrap().to_string_lossy().into_owned();
            assert!(name.starts_with("Panopin-"), "{name}");
            assert_eq!(copy.extension(), original.extension());
            assert_eq!(fs::read(copy).unwrap(), fs::read(original).unwrap());
        }
    }
}
