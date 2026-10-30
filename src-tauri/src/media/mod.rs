//! Managed media under `media/<id>/`: a copy of the original and a thumbnail (D-006, D-025).
//! Files in watched folders are only ever read, never moved or deleted.

pub mod annotations;
pub mod decode;
pub mod flatten;
pub mod thumbnails;
pub mod video;

use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use rusqlite::{Connection, OptionalExtension};
use tauri::http::Response;

use crate::error::Result;

fn content_type(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}

/// Serves `media://localhost/<thumb|original|link-image|link-icon>/<id>` (D-014). The webview only ever names an item
/// id; the file path comes from the database, so nothing outside the managed folder is reachable.
/// The webview asks for recordings a range at a time; at most this much is answered at once.
const RANGE_CHUNK: u64 = 2 * 1024 * 1024;

/// `bytes=start-end` (end optional) against a file of `len` bytes.
fn byte_range(header: &str, len: u64) -> Option<(u64, u64)> {
    let spec = header.trim().strip_prefix("bytes=")?;
    let (start, end) = spec.split_once('-')?;
    let start: u64 = start.trim().parse().ok()?;
    if start >= len {
        return None;
    }
    let end = match end.trim() {
        "" => len - 1,
        end => end.parse::<u64>().ok()?.min(len - 1),
    };
    (start <= end).then(|| (start, end.min(start + RANGE_CHUNK - 1)))
}

/// A recording, answered in ranges so it plays and seeks without being read whole.
fn serve_video(file: &Path, range: Option<&str>) -> Response<Vec<u8>> {
    let fail = |status: u16| {
        Response::builder()
            .status(status)
            .body(Vec::new())
            .unwrap_or_default()
    };
    let Ok(mut handle) = fs::File::open(file) else {
        return fail(404);
    };
    let Ok(len) = handle.metadata().map(|m| m.len()) else {
        return fail(404);
    };
    if len == 0 {
        return fail(404);
    }
    let (start, end) = match range {
        Some(header) => match byte_range(header, len) {
            Some(range) => range,
            None => {
                return Response::builder()
                    .status(416)
                    .header("Content-Range", format!("bytes */{len}"))
                    .body(Vec::new())
                    .unwrap_or_default()
            }
        },
        None => (0, (RANGE_CHUNK - 1).min(len - 1)),
    };
    let mut bytes = vec![0u8; (end - start + 1) as usize];
    if handle.seek(SeekFrom::Start(start)).is_err() || handle.read_exact(&mut bytes).is_err() {
        return fail(500);
    }
    Response::builder()
        .status(206)
        .header("Content-Type", "video/mp4")
        .header("Accept-Ranges", "bytes")
        .header("Content-Range", format!("bytes {start}-{end}/{len}"))
        .header("Cache-Control", NO_STORE)
        .body(bytes)
        .unwrap_or_default()
}

/// Media never goes into the webview's disk cache: a copy there would outlive the item when it
/// is deleted for good or the history is cleared (D-072). Reading the file again is cheap.
const NO_STORE: &str = "no-store";

pub fn serve(
    store: &MediaStore,
    db: &Connection,
    path: &str,
    range: Option<&str>,
) -> Response<Vec<u8>> {
    let empty = |status: u16| {
        Response::builder()
            .status(status)
            .body(Vec::new())
            .unwrap_or_default()
    };
    let Some((variant, id)) = path
        .trim_start_matches('/')
        .split_once('/')
        .and_then(|(variant, id)| Some((variant, id.parse::<i64>().ok()?)))
    else {
        return empty(400);
    };
    // Link pictures were re-encoded by Panopin (D-057); they live next to the item's media.
    let column = match variant {
        "thumb" => "thumb_ext",
        "original" => "media_ext",
        "link-image" => "link_image_ext",
        "link-icon" => "link_icon_ext",
        _ => return empty(404),
    };
    let ext: Option<String> = db
        .query_row(
            &format!("SELECT {column} FROM items WHERE id = ?1"),
            [id],
            |row| row.get(0),
        )
        .optional()
        .ok()
        .flatten()
        .flatten();
    let Some(ext) = ext else {
        return empty(404);
    };
    let file = store.dir(id).join(format!("{variant}.{ext}"));
    if ext == "mp4" {
        return serve_video(&file, range);
    }
    match fs::read(file) {
        Ok(bytes) => Response::builder()
            .header("Content-Type", content_type(&ext))
            .header("Cache-Control", NO_STORE)
            .body(bytes)
            .unwrap_or_default(),
        Err(_) => empty(404),
    }
}

#[derive(Clone)]
pub struct MediaStore {
    root: PathBuf,
}

impl MediaStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn dir(&self, id: i64) -> PathBuf {
        self.root.join(id.to_string())
    }

    /// Writes both files into a temporary folder that is renamed into place, so a crash never
    /// leaves a half-written item behind.
    pub fn write(
        &self,
        id: i64,
        original: &[u8],
        original_ext: &str,
        thumb: &thumbnails::Thumbnail,
    ) -> Result<()> {
        let tmp = self.root.join(format!(".tmp-{id}"));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp)?;
        fs::write(tmp.join(format!("original.{original_ext}")), original)?;
        fs::write(tmp.join(format!("thumb.{}", thumb.ext)), &thumb.bytes)?;
        let dest = self.dir(id);
        let _ = fs::remove_dir_all(&dest);
        fs::rename(&tmp, &dest)?;
        Ok(())
    }

    /// Copies a file (a recording) into place with its thumbnail, through a temporary folder.
    pub fn write_file(
        &self,
        id: i64,
        source: &Path,
        ext: &str,
        thumb: Option<&thumbnails::Thumbnail>,
    ) -> Result<()> {
        let tmp = self.root.join(format!(".tmp-{id}"));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp)?;
        fs::copy(source, tmp.join(format!("original.{ext}")))?;
        if let Some(thumb) = thumb {
            fs::write(tmp.join(format!("thumb.{}", thumb.ext)), &thumb.bytes)?;
        }
        let dest = self.dir(id);
        let _ = fs::remove_dir_all(&dest);
        fs::rename(&tmp, &dest)?;
        Ok(())
    }

    pub fn remove(&self, id: i64) {
        if let Err(err) = fs::remove_dir_all(self.dir(id)) {
            if err.kind() != std::io::ErrorKind::NotFound {
                log::warn!("could not remove media of item {id}: {err}");
            }
        }
    }

    pub fn original(&self, id: i64, ext: &str) -> PathBuf {
        self.dir(id).join(format!("original.{ext}"))
    }

    /// Swaps in a new thumbnail after an edit; an old one with another extension is removed.
    pub fn replace_thumb(
        &self,
        id: i64,
        thumb: &thumbnails::Thumbnail,
        old_ext: Option<&str>,
    ) -> Result<()> {
        let dir = self.dir(id);
        let tmp = dir.join(format!("thumb.{}.tmp", thumb.ext));
        fs::write(&tmp, &thumb.bytes)?;
        fs::rename(&tmp, dir.join(format!("thumb.{}", thumb.ext)))?;
        if let Some(old) = old_ext.filter(|old| *old != thumb.ext) {
            let _ = fs::remove_file(dir.join(format!("thumb.{old}")));
        }
        Ok(())
    }

    /// Removes leftovers of writes interrupted by a crash.
    pub fn clean_temp(&self) {
        let Ok(entries) = fs::read_dir(&self.root) else {
            return;
        };
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(".tmp-") {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_replace_and_remove_item_folders() {
        let root = tempfile::tempdir().unwrap();
        let store = MediaStore::new(root.path().to_path_buf());
        let thumb = thumbnails::Thumbnail {
            bytes: vec![1, 2],
            ext: "jpg",
        };
        store.write(7, b"png-bytes", "png", &thumb).unwrap();
        assert_eq!(
            fs::read(store.dir(7).join("original.png")).unwrap(),
            b"png-bytes"
        );
        assert_eq!(
            fs::read(store.dir(7).join("thumb.jpg")).unwrap(),
            vec![1, 2]
        );

        fs::create_dir_all(root.path().join(".tmp-9")).unwrap();
        store.clean_temp();
        assert!(!root.path().join(".tmp-9").exists());

        store.remove(7);
        store.remove(7); // already gone: no error
        assert!(!store.dir(7).exists());
    }

    #[test]
    fn recordings_are_served_a_range_at_a_time() {
        assert_eq!(byte_range("bytes=0-", 10), Some((0, 9)));
        assert_eq!(byte_range("bytes=4-6", 10), Some((4, 6)));
        assert_eq!(byte_range("bytes=4-99", 10), Some((4, 9)));
        assert_eq!(byte_range("bytes=10-", 10), None);
        assert_eq!(byte_range("bytes=7-3", 10), None);
        assert_eq!(byte_range("items=0-1", 10), None);
        let big = 10 * RANGE_CHUNK;
        assert_eq!(byte_range("bytes=0-", big), Some((0, RANGE_CHUNK - 1)));

        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("original.mp4");
        fs::write(&file, b"0123456789").unwrap();
        let partial = serve_video(&file, Some("bytes=2-5"));
        assert_eq!(partial.status(), 206);
        assert_eq!(partial.body(), b"2345");
        assert_eq!(partial.headers()["Content-Range"], "bytes 2-5/10");
        assert_eq!(serve_video(&file, Some("bytes=20-")).status(), 416);
        assert_eq!(serve_video(&file, None).body().len(), 10);
    }
}
