//! Managed media under `media/<id>/`: a copy of the original and a thumbnail (D-006, D-025).
//! Files in watched folders are only ever read, never moved or deleted.

pub mod decode;
pub mod thumbnails;

use std::{fs, path::PathBuf};

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
        _ => "application/octet-stream",
    }
}

/// Serves `media://localhost/<thumb|original>/<id>` (D-014). The webview only ever names an item
/// id; the file path comes from the database, so nothing outside the managed folder is reachable.
pub fn serve(store: &MediaStore, db: &Connection, path: &str) -> Response<Vec<u8>> {
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
    let column = match variant {
        "thumb" => "thumb_ext",
        "original" => "media_ext",
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
    match fs::read(file) {
        Ok(bytes) => Response::builder()
            .header("Content-Type", content_type(&ext))
            .header("Cache-Control", "max-age=31536000, immutable")
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

    pub fn remove(&self, id: i64) {
        if let Err(err) = fs::remove_dir_all(self.dir(id)) {
            if err.kind() != std::io::ErrorKind::NotFound {
                log::warn!("could not remove media of item {id}: {err}");
            }
        }
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
}
