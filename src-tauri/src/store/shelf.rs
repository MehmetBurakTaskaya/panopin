//! What the shelf shows, and the shelf actions that change items: pin, take down, undo.

use std::collections::VecDeque;

use rusqlite::{params, Connection};
use serde::Serialize;

use crate::{error::Result, store::items::Kind};

/// Characters of a text sent to the shelf; the full text stays in the store.
const PREVIEW_CHARS: usize = 400;
/// Cards per row; the shelf shows the most recent ones.
const ROW_LIMIT: usize = 300;
/// Take-down steps that Backspace can undo in one session.
pub const UNDO_DEPTH: usize = 32;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShelfItem {
    pub id: i64,
    pub kind: Kind,
    pub title: Option<String>,
    pub preview: Option<String>,
    pub char_count: Option<usize>,
    pub url: Option<String>,
    pub domain: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub has_thumb: bool,
    pub pinned: bool,
    pub last_copied_at: i64,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct Shelf {
    pub image: Vec<ShelfItem>,
    pub text: Vec<ShelfItem>,
    pub link: Vec<ShelfItem>,
}

fn domain_of(url: &str) -> Option<String> {
    let host = url::Url::parse(url).ok()?.host_str()?.to_owned();
    Some(host.strip_prefix("www.").map(str::to_owned).unwrap_or(host))
}

pub fn list(conn: &Connection) -> Result<Shelf> {
    let mut stmt = conn.prepare(
        "SELECT id, title, text_body, url, width, height, thumb_ext, pinned, last_copied_at
         FROM items
         WHERE kind = ?1 AND trashed_at IS NULL AND shelved = 1
         ORDER BY last_copied_at DESC, id DESC
         LIMIT ?2",
    )?;
    let mut row_of = |kind: Kind| -> Result<Vec<ShelfItem>> {
        let rows = stmt.query_map(params![kind.as_str(), ROW_LIMIT as i64], |row| {
            let body: Option<String> = row.get(2)?;
            let url: Option<String> = row.get(3)?;
            Ok(ShelfItem {
                id: row.get(0)?,
                kind,
                title: row.get(1)?,
                char_count: body.as_ref().map(|b| b.chars().count()),
                preview: body.map(|b| b.chars().take(PREVIEW_CHARS).collect()),
                domain: url.as_deref().and_then(domain_of),
                url,
                width: row.get(4)?,
                height: row.get(5)?,
                has_thumb: row.get::<_, Option<String>>(6)?.is_some(),
                pinned: row.get::<_, i64>(7)? == 1,
                last_copied_at: row.get(8)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    };
    Ok(Shelf {
        image: row_of(Kind::Image)?,
        text: row_of(Kind::Text)?,
        link: row_of(Kind::Link)?,
    })
}

pub fn set_pinned(conn: &mut Connection, ids: &[i64], pinned: bool) -> Result<()> {
    let tx = conn.transaction()?;
    for id in ids {
        tx.execute(
            "UPDATE items SET pinned = ?2 WHERE id = ?1",
            params![id, pinned],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// One take-down batch, enough to put every item back exactly as it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TakenDown {
    items: Vec<(i64, i64, Option<i64>)>,
}

/// Saved items (pinned or named) leave the shelf but stay in the Library; the rest go to Trash.
pub fn take_down(conn: &mut Connection, ids: &[i64], now: i64) -> Result<TakenDown> {
    let tx = conn.transaction()?;
    let mut items = Vec::new();
    for id in ids {
        let (shelved, trashed_at, saved): (i64, Option<i64>, bool) = tx.query_row(
            "SELECT shelved, trashed_at, pinned = 1 OR title IS NOT NULL FROM items WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        items.push((*id, shelved, trashed_at));
        if saved {
            tx.execute("UPDATE items SET shelved = 0 WHERE id = ?1", [id])?;
        } else {
            tx.execute(
                "UPDATE items SET trashed_at = ?2 WHERE id = ?1",
                params![id, now],
            )?;
        }
    }
    tx.commit()?;
    Ok(TakenDown { items })
}

/// Delete on the shelf (after confirming): every item goes to Trash, saved or not.
pub fn trash(conn: &mut Connection, ids: &[i64], now: i64) -> Result<TakenDown> {
    let tx = conn.transaction()?;
    let mut items = Vec::new();
    for id in ids {
        let (shelved, trashed_at): (i64, Option<i64>) = tx.query_row(
            "SELECT shelved, trashed_at FROM items WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        items.push((*id, shelved, trashed_at));
        tx.execute(
            "UPDATE items SET trashed_at = ?2 WHERE id = ?1",
            params![id, now],
        )?;
    }
    tx.commit()?;
    Ok(TakenDown { items })
}

/// Puts a batch back exactly as it was and returns the ids of the items that still exist.
pub fn restore(conn: &mut Connection, batch: &TakenDown) -> Result<Vec<i64>> {
    let tx = conn.transaction()?;
    let mut restored = Vec::new();
    for (id, shelved, trashed_at) in &batch.items {
        let changed = tx.execute(
            "UPDATE items SET shelved = ?2, trashed_at = ?3 WHERE id = ?1",
            params![id, shelved, trashed_at],
        )?;
        if changed == 1 {
            restored.push(*id);
        }
    }
    tx.commit()?;
    Ok(restored)
}

/// Session undo for take-downs: the newest batch comes back first, at most `UNDO_DEPTH` kept.
#[derive(Debug, Default)]
pub struct UndoStack(VecDeque<TakenDown>);

impl UndoStack {
    pub fn push(&mut self, batch: TakenDown) {
        if self.0.len() == UNDO_DEPTH {
            self.0.pop_front();
        }
        self.0.push_back(batch);
    }

    pub fn pop(&mut self) -> Option<TakenDown> {
        self.0.pop_back()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{
        self,
        identity::text_identity,
        items::{upsert_text, TextCapture},
    };

    fn add(conn: &Connection, kind: Kind, body: &str, url: Option<&str>, at: i64) -> i64 {
        let capture = TextCapture {
            kind,
            body,
            url,
            identity: text_identity(body),
            source_app: None,
        };
        upsert_text(conn, &capture, at).unwrap()
    }

    #[test]
    fn rows_are_newest_first_with_previews() {
        let conn = store::open_in_memory().unwrap();
        let long = "ş".repeat(1000);
        add(&conn, Kind::Text, "older", None, 1);
        let newer = add(&conn, Kind::Text, &long, None, 2);
        add(
            &conn,
            Kind::Link,
            "https://www.example.com/a",
            Some("https://www.example.com/a"),
            3,
        );
        let shelf = list(&conn).unwrap();
        assert_eq!(shelf.text.len(), 2);
        assert_eq!(shelf.text[0].id, newer);
        assert_eq!(shelf.text[0].char_count, Some(1000));
        assert_eq!(
            shelf.text[0].preview.as_ref().unwrap().chars().count(),
            PREVIEW_CHARS
        );
        assert_eq!(shelf.link[0].domain.as_deref(), Some("example.com"));
        assert!(shelf.image.is_empty());
    }

    #[test]
    fn take_down_trashes_temporary_and_unshelves_saved_items() {
        let mut conn = store::open_in_memory().unwrap();
        let temp = add(&conn, Kind::Text, "temp", None, 1);
        let saved = add(&conn, Kind::Text, "saved", None, 1);
        set_pinned(&mut conn, &[saved], true).unwrap();

        let batch = take_down(&mut conn, &[temp, saved], 50).unwrap();
        assert!(list(&conn).unwrap().text.is_empty());
        let (trashed, shelved): (Option<i64>, i64) = conn
            .query_row(
                "SELECT trashed_at, shelved FROM items WHERE id = ?1",
                [temp],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((trashed, shelved), (Some(50), 1));
        let (trashed, shelved): (Option<i64>, i64) = conn
            .query_row(
                "SELECT trashed_at, shelved FROM items WHERE id = ?1",
                [saved],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            (trashed, shelved),
            (None, 0),
            "saved items stay in the Library"
        );

        assert_eq!(restore(&mut conn, &batch).unwrap(), vec![temp, saved]);
        assert_eq!(list(&conn).unwrap().text.len(), 2);
    }

    #[test]
    fn trash_takes_saved_items_too_and_undo_restores_them() {
        let mut conn = store::open_in_memory().unwrap();
        let saved = add(&conn, Kind::Text, "saved", None, 1);
        set_pinned(&mut conn, &[saved], true).unwrap();
        let batch = trash(&mut conn, &[saved], 9).unwrap();
        let trashed: Option<i64> = conn
            .query_row("SELECT trashed_at FROM items WHERE id = ?1", [saved], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(trashed, Some(9));
        assert_eq!(restore(&mut conn, &batch).unwrap(), vec![saved]);
        let item = &list(&conn).unwrap().text[0];
        assert!(item.pinned, "the pin survives a trip through Trash");
    }

    #[test]
    fn copying_again_brings_a_taken_down_item_back() {
        let mut conn = store::open_in_memory().unwrap();
        let id = add(&conn, Kind::Text, "again", None, 1);
        set_pinned(&mut conn, &[id], true).unwrap();
        take_down(&mut conn, &[id], 2).unwrap();
        add(&conn, Kind::Text, "again", None, 3);
        assert_eq!(list(&conn).unwrap().text[0].id, id);
    }

    #[test]
    fn undo_keeps_the_newest_32_batches() {
        let mut stack = UndoStack::default();
        for id in 0..40 {
            stack.push(TakenDown {
                items: vec![(id, 1, None)],
            });
        }
        assert_eq!(stack.pop().unwrap().items[0].0, 39);
        let mut left = 1;
        while stack.pop().is_some() {
            left += 1;
        }
        assert_eq!(left, UNDO_DEPTH);
    }
}
