//! Changes the user makes to stored items: the body of a text, the edit state of an image.

use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    error::{AppError, Result},
    store::{
        identity::{edited_identity, text_identity},
        items::Kind,
    },
};

/// An edited text may not grow beyond this (bytes of UTF-8).
pub const MAX_TEXT_BYTES: usize = 10 * 1024 * 1024;

/// What an image edit needs to know about the stored image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageFacts {
    pub width: u32,
    pub height: u32,
    pub media_ext: String,
    pub thumb_ext: Option<String>,
}

pub fn image_facts(conn: &Connection, id: i64) -> Result<ImageFacts> {
    conn.query_row(
        "SELECT width, height, media_ext, thumb_ext FROM items WHERE id = ?1 AND kind = ?2",
        params![id, Kind::Image.as_str()],
        |row| {
            Ok(ImageFacts {
                width: row.get(0)?,
                height: row.get(1)?,
                media_ext: row.get(2)?,
                thumb_ext: row.get(3)?,
            })
        },
    )
    .optional()?
    .ok_or(AppError::NotEditable)
}

/// Records an image's edit state: the edited size (`None` after "revert to original") and the
/// new thumbnail, whose URL version moves on.
pub fn set_image_edit(
    conn: &Connection,
    id: i64,
    size: Option<(u32, u32)>,
    thumb_ext: &str,
    now: i64,
) -> Result<()> {
    conn.execute(
        "UPDATE items SET edit_width = ?2, edit_height = ?3, thumb_ext = ?4, edited_at = ?5
         WHERE id = ?1",
        params![id, size.map(|s| s.0), size.map(|s| s.1), thumb_ext, now],
    )?;
    Ok(())
}

/// The whole text of a text or link card (the shelf only receives a preview).
pub fn full_text(conn: &Connection, id: i64) -> Result<String> {
    conn.query_row(
        "SELECT COALESCE(text_body, url) FROM items WHERE id = ?1 AND kind IN ('text', 'link')",
        [id],
        |row| row.get::<_, Option<String>>(0),
    )
    .optional()?
    .flatten()
    .ok_or(AppError::NotEditable)
}

/// Saves an edited text. Its identity follows the new text, so copying that text again finds this
/// card; when another card already holds that text, this one gets an identity of its own and both
/// stay (merging would lose a name or pin).
pub fn update_text(conn: &Connection, id: i64, body: &str) -> Result<()> {
    if body.len() > MAX_TEXT_BYTES {
        return Err(AppError::InvalidEdit("text too long"));
    }
    let identity = text_identity(body);
    let taken: Option<i64> = conn
        .query_row(
            "SELECT id FROM items WHERE kind = 'text' AND identity = ?1 AND id <> ?2",
            params![&identity[..], id],
            |row| row.get(0),
        )
        .optional()?;
    let identity = if taken.is_some() {
        edited_identity(id, body)
    } else {
        identity
    };
    let changed = conn.execute(
        "UPDATE items SET text_body = ?2, identity = ?3 WHERE id = ?1 AND kind = 'text'",
        params![id, body, &identity[..]],
    )?;
    if changed == 0 {
        return Err(AppError::NotEditable);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{
        self,
        items::{insert_image, upsert_text, ImageRow, Source, TextCapture},
    };

    fn add_text(conn: &Connection, body: &str) -> i64 {
        let capture = TextCapture {
            kind: Kind::Text,
            body,
            url: None,
            identity: text_identity(body),
            source_app: None,
        };
        upsert_text(conn, &capture, 1).unwrap()
    }

    #[test]
    fn an_edited_text_is_found_by_its_new_content() {
        let conn = store::open_in_memory().unwrap();
        let id = add_text(&conn, "draft");
        update_text(&conn, id, "final text").unwrap();
        assert_eq!(full_text(&conn, id).unwrap(), "final text");
        // Copying the new text again bumps the same card instead of adding one.
        assert_eq!(add_text(&conn, "final text"), id);
        // The old text is a new card now.
        assert_ne!(add_text(&conn, "draft"), id);
    }

    #[test]
    fn editing_into_another_cards_text_keeps_both_cards() {
        let conn = store::open_in_memory().unwrap();
        let a = add_text(&conn, "first");
        let b = add_text(&conn, "second");
        update_text(&conn, a, "second").unwrap();
        assert_eq!(full_text(&conn, a).unwrap(), "second");
        assert_eq!(
            add_text(&conn, "second"),
            b,
            "the existing card keeps its identity"
        );
    }

    #[test]
    fn only_texts_can_be_edited_and_images_record_their_edit() {
        let conn = store::open_in_memory().unwrap();
        let row = ImageRow {
            identity: [7; 32],
            source: Source::Clipboard,
            source_app: None,
            source_path: None,
            width: 640,
            height: 480,
            byte_size: 10,
            media_ext: "png",
            thumb_ext: "jpg",
        };
        let image = insert_image(&conn, &row, 1).unwrap();
        assert!(update_text(&conn, image, "x").is_err());
        assert!(full_text(&conn, image).is_err());
        let facts = image_facts(&conn, image).unwrap();
        assert_eq!(
            (facts.width, facts.height, facts.media_ext.as_str()),
            (640, 480, "png")
        );

        set_image_edit(&conn, image, Some((200, 100)), "png", 99).unwrap();
        let shelf = store::shelf::list(&conn).unwrap();
        let item = &shelf.image[0];
        assert!(item.edited);
        assert_eq!(
            (item.width, item.height, item.thumb_version),
            (Some(200), Some(100), 99)
        );

        set_image_edit(&conn, image, None, "jpg", 120).unwrap();
        let item = &store::shelf::list(&conn).unwrap().image[0];
        assert!(!item.edited);
        assert_eq!(
            (item.width, item.height, item.thumb_version),
            (Some(640), Some(480), 120)
        );
    }
}
