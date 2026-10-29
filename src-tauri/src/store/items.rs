//! Item rows: insert or bump by identity, and the per-kind counts shown in the tray.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{error::Result, store::identity::Identity};

/// D-042: a saved item (pinned, named, in a folder or tagged) never expires, and taking it off
/// the shelf keeps it in the Library. A condition on the `items` table.
pub const SAVED: &str = "(items.pinned = 1 OR items.title IS NOT NULL \
     OR EXISTS (SELECT 1 FROM item_folders WHERE item_folders.item_id = items.id) \
     OR EXISTS (SELECT 1 FROM item_tags WHERE item_tags.item_id = items.id))";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Image,
    Text,
    Link,
    Video,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Image, Kind::Text, Kind::Link, Kind::Video];

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Image => "image",
            Kind::Text => "text",
            Kind::Link => "link",
            Kind::Video => "video",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Clipboard,
    Folder,
}

impl Source {
    fn as_str(self) -> &'static str {
        match self {
            Source::Clipboard => "clipboard",
            Source::Folder => "folder",
        }
    }
}

/// A copied text or link.
pub struct TextCapture<'a> {
    pub kind: Kind,
    pub body: &'a str,
    pub url: Option<&'a str>,
    pub identity: Identity,
    pub source_app: Option<&'a str>,
}

/// Inserts a text or link, or bumps the existing one with the same identity: the newest body
/// replaces the old one, it moves to the front and out of Trash, while title, pin and note stay.
pub fn upsert_text(conn: &Connection, capture: &TextCapture<'_>, now: i64) -> Result<i64> {
    Ok(conn.query_row(
        "INSERT INTO items (kind, identity, created_at, last_copied_at, source, source_app, text_body, url)
         VALUES (?1, ?2, ?3, ?3, 'clipboard', ?4, ?5, ?6)
         ON CONFLICT (kind, identity) DO UPDATE SET
             text_body = excluded.text_body,
             url = excluded.url,
             last_copied_at = excluded.last_copied_at,
             trashed_at = NULL,
             shelved = 1,
             source_app = COALESCE(excluded.source_app, items.source_app)
         RETURNING id",
        params![
            capture.kind.as_str(),
            &capture.identity[..],
            now,
            capture.source_app,
            capture.body,
            capture.url
        ],
        |row| row.get(0),
    )?)
}

pub fn find(conn: &Connection, kind: Kind, identity: &Identity) -> Result<Option<i64>> {
    Ok(conn
        .query_row(
            "SELECT id FROM items WHERE kind = ?1 AND identity = ?2",
            params![kind.as_str(), &identity[..]],
            |row| row.get(0),
        )
        .optional()?)
}

/// Moves an existing item to the front of the shelf (and out of Trash) without touching anything else.
pub fn bump(conn: &Connection, id: i64, now: i64) -> Result<()> {
    conn.execute(
        "UPDATE items SET last_copied_at = ?2, trashed_at = NULL, shelved = 1 WHERE id = ?1",
        params![id, now],
    )?;
    Ok(())
}

pub struct ImageRow<'a> {
    pub identity: Identity,
    pub width: u32,
    pub height: u32,
    pub byte_size: u64,
    pub media_ext: &'a str,
    pub thumb_ext: &'a str,
    pub source: Source,
    pub source_app: Option<&'a str>,
    pub source_path: Option<&'a str>,
}

pub fn insert_image(conn: &Connection, row: &ImageRow<'_>, now: i64) -> Result<i64> {
    conn.execute(
        "INSERT INTO items (kind, identity, created_at, last_copied_at, source, source_app,
                            source_path, width, height, byte_size, media_ext, thumb_ext)
         VALUES ('image', ?1, ?2, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            &row.identity[..],
            now,
            row.source.as_str(),
            row.source_app,
            row.source_path,
            row.width,
            row.height,
            i64::try_from(row.byte_size).unwrap_or(i64::MAX),
            row.media_ext,
            row.thumb_ext
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM items WHERE id = ?1", [id])?;
    Ok(())
}

/// Items on the shelf (not taken down, not in Trash), per kind.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Counts {
    pub image: u32,
    pub text: u32,
    pub link: u32,
    pub video: u32,
}

pub fn counts(conn: &Connection) -> Result<Counts> {
    let mut stmt = conn.prepare(
        "SELECT kind, count(*) FROM items WHERE trashed_at IS NULL AND shelved = 1 GROUP BY kind",
    )?;
    let mut counts = Counts::default();
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
    })?;
    for row in rows {
        let (kind, n) = row?;
        match kind.as_str() {
            "image" => counts.image = n,
            "text" => counts.text = n,
            "link" => counts.link = n,
            _ => counts.video = n,
        }
    }
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{self, identity::text_identity};

    fn text<'a>(kind: Kind, body: &'a str) -> TextCapture<'a> {
        TextCapture {
            kind,
            body,
            url: None,
            identity: text_identity(body),
            source_app: Some("notepad.exe"),
        }
    }

    fn field<T: rusqlite::types::FromSql>(conn: &Connection, id: i64, column: &str) -> T {
        conn.query_row(
            &format!("SELECT {column} FROM items WHERE id = ?1"),
            [id],
            |row| row.get(0),
        )
        .unwrap()
    }

    #[test]
    fn copying_again_bumps_instead_of_duplicating() {
        let conn = store::open_in_memory().unwrap();
        let id = upsert_text(&conn, &text(Kind::Text, "hello\r\n"), 1_000).unwrap();
        conn.execute(
            "UPDATE items SET title = 'Kept', pinned = 1, trashed_at = 5 WHERE id = ?1",
            [id],
        )
        .unwrap();
        let again = upsert_text(&conn, &text(Kind::Text, "hello\n"), 2_000).unwrap();
        assert_eq!(again, id);
        assert_eq!(
            field::<String>(&conn, id, "text_body"),
            "hello\n",
            "newest bytes win"
        );
        assert_eq!(field::<i64>(&conn, id, "last_copied_at"), 2_000);
        assert_eq!(field::<i64>(&conn, id, "created_at"), 1_000);
        assert_eq!(field::<String>(&conn, id, "title"), "Kept");
        assert_eq!(field::<i64>(&conn, id, "pinned"), 1);
        assert_eq!(field::<Option<i64>>(&conn, id, "trashed_at"), None);
    }

    #[test]
    fn text_and_link_with_the_same_identity_are_separate() {
        let conn = store::open_in_memory().unwrap();
        let a = upsert_text(&conn, &text(Kind::Text, "https://example.com"), 1).unwrap();
        let b = upsert_text(&conn, &text(Kind::Link, "https://example.com"), 1).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn images_are_found_and_bumped() {
        let conn = store::open_in_memory().unwrap();
        let row = ImageRow {
            identity: [7; 32],
            width: 2,
            height: 2,
            byte_size: 64,
            media_ext: "png",
            thumb_ext: "png",
            source: Source::Folder,
            source_app: None,
            source_path: Some("C:/shots/a.png"),
        };
        let id = insert_image(&conn, &row, 10).unwrap();
        assert_eq!(find(&conn, Kind::Image, &[7; 32]).unwrap(), Some(id));
        assert_eq!(find(&conn, Kind::Image, &[8; 32]).unwrap(), None);
        bump(&conn, id, 20).unwrap();
        assert_eq!(field::<i64>(&conn, id, "last_copied_at"), 20);
    }

    #[test]
    fn counts_skip_trash() {
        let conn = store::open_in_memory().unwrap();
        upsert_text(&conn, &text(Kind::Text, "a"), 1).unwrap();
        let b = upsert_text(&conn, &text(Kind::Text, "b"), 1).unwrap();
        upsert_text(&conn, &text(Kind::Link, "https://a.example"), 1).unwrap();
        conn.execute("UPDATE items SET trashed_at = 2 WHERE id = ?1", [b])
            .unwrap();
        assert_eq!(
            counts(&conn).unwrap(),
            Counts {
                image: 0,
                text: 1,
                link: 1,
                video: 0
            }
        );
    }
}
