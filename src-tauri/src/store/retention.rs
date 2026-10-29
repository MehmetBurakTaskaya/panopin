//! Expiry to Trash, Trash auto-empty and "Clear history" (D-018). Only managed rows and media are
//! touched; files in watched folders are never deleted or moved.

use rusqlite::{params, Connection};

use crate::{
    error::Result,
    store::items::{Kind, SAVED},
};

pub const DAY_MS: i64 = 86_400_000;

/// Retention in days per kind (`None` = keep forever) and for Trash (`None` = never empty).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    pub image: Option<u32>,
    pub text: Option<u32>,
    pub link: Option<u32>,
    pub video: Option<u32>,
    pub trash: Option<u32>,
}

impl Policy {
    fn days(&self, kind: Kind) -> Option<u32> {
        match kind {
            Kind::Image => self.image,
            Kind::Text => self.text,
            Kind::Link => self.link,
            Kind::Video => self.video,
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Sweep {
    pub trashed: usize,
    /// Rows deleted for good; the caller removes their media folders.
    pub purged: Vec<i64>,
}

pub fn sweep(conn: &mut Connection, policy: &Policy, now: i64) -> Result<Sweep> {
    let tx = conn.transaction()?;
    let mut trashed = 0;
    for kind in Kind::ALL {
        if let Some(days) = policy.days(kind) {
            trashed += tx.execute(
                &format!(
                    "UPDATE items SET trashed_at = ?1
                     WHERE kind = ?2 AND trashed_at IS NULL AND NOT {SAVED} AND last_copied_at < ?3"
                ),
                params![now, kind.as_str(), now - i64::from(days) * DAY_MS],
            )?;
        }
    }
    let mut purged = Vec::new();
    if let Some(days) = policy.trash {
        let mut stmt = tx.prepare(
            "DELETE FROM items WHERE trashed_at IS NOT NULL AND trashed_at < ?1 RETURNING id",
        )?;
        let rows = stmt.query_map([now - i64::from(days) * DAY_MS], |row| row.get(0))?;
        for row in rows {
            purged.push(row?);
        }
    }
    tx.commit()?;
    Ok(Sweep { trashed, purged })
}

fn kind_list(kinds: &[Kind]) -> String {
    kinds
        .iter()
        .map(|kind| format!("'{}'", kind.as_str()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// How many items "Clear history" would move now, plus the cutoff to hand back on confirm, so
/// anything copied while the confirmation is open stays untouched.
pub fn clearable(conn: &Connection, kinds: &[Kind], now: i64) -> Result<(usize, i64)> {
    if kinds.is_empty() {
        return Ok((0, now));
    }
    let count: i64 = conn.query_row(
        &format!(
            "SELECT count(*) FROM items
             WHERE kind IN ({}) AND trashed_at IS NULL AND NOT {SAVED} AND last_copied_at <= ?1",
            kind_list(kinds)
        ),
        [now],
        |row| row.get(0),
    )?;
    Ok((usize::try_from(count).unwrap_or(0), now))
}

/// Moves items that are still unsaved and were last copied at or before `cutoff` to Trash,
/// in one transaction: either everything moves or nothing does.
pub fn clear_history(
    conn: &mut Connection,
    kinds: &[Kind],
    cutoff: i64,
    now: i64,
) -> Result<usize> {
    if kinds.is_empty() {
        return Ok(0);
    }
    let tx = conn.transaction()?;
    let moved = tx.execute(
        &format!(
            "UPDATE items SET trashed_at = ?1
             WHERE kind IN ({}) AND trashed_at IS NULL AND NOT {SAVED} AND last_copied_at <= ?2",
            kind_list(kinds)
        ),
        params![now, cutoff],
    )?;
    tx.commit()?;
    Ok(moved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{
        self,
        identity::text_identity,
        items::{upsert_text, TextCapture},
    };

    const NOW: i64 = 100 * DAY_MS;

    fn add(conn: &Connection, kind: Kind, body: &str, copied_at: i64) -> i64 {
        let capture = TextCapture {
            kind,
            body,
            url: None,
            identity: text_identity(body),
            source_app: None,
        };
        upsert_text(conn, &capture, copied_at).unwrap()
    }

    fn trashed(conn: &Connection, id: i64) -> bool {
        conn.query_row(
            "SELECT trashed_at IS NOT NULL FROM items WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn exists(conn: &Connection, id: i64) -> bool {
        conn.query_row("SELECT count(*) FROM items WHERE id = ?1", [id], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap()
            == 1
    }

    fn policy(days: Option<u32>, trash: Option<u32>) -> Policy {
        Policy {
            image: days,
            text: days,
            link: days,
            video: days,
            trash,
        }
    }

    #[test]
    fn old_unsaved_items_go_to_trash() {
        let mut conn = store::open_in_memory().unwrap();
        let old = add(&conn, Kind::Text, "old", NOW - 31 * DAY_MS);
        let recent = add(&conn, Kind::Text, "recent", NOW - 29 * DAY_MS);
        let pinned = add(&conn, Kind::Text, "pinned", NOW - 90 * DAY_MS);
        let named = add(
            &conn,
            Kind::Link,
            "https://named.example",
            NOW - 90 * DAY_MS,
        );
        conn.execute("UPDATE items SET pinned = 1 WHERE id = ?1", [pinned])
            .unwrap();
        conn.execute("UPDATE items SET title = 'Docs' WHERE id = ?1", [named])
            .unwrap();

        let report = sweep(&mut conn, &policy(Some(30), None), NOW).unwrap();
        assert_eq!(report.trashed, 1);
        assert!(trashed(&conn, old));
        assert!(!trashed(&conn, recent) && !trashed(&conn, pinned) && !trashed(&conn, named));
    }

    #[test]
    fn keep_forever_never_expires() {
        let mut conn = store::open_in_memory().unwrap();
        let ancient = add(&conn, Kind::Text, "ancient", 0);
        sweep(&mut conn, &policy(None, None), NOW).unwrap();
        assert!(!trashed(&conn, ancient));
    }

    #[test]
    fn old_trash_is_purged() {
        let mut conn = store::open_in_memory().unwrap();
        let gone = add(&conn, Kind::Text, "gone", 0);
        let kept = add(&conn, Kind::Text, "kept", 0);
        conn.execute(
            "UPDATE items SET trashed_at = ?2 WHERE id = ?1",
            params![gone, NOW - 31 * DAY_MS],
        )
        .unwrap();
        conn.execute(
            "UPDATE items SET trashed_at = ?2 WHERE id = ?1",
            params![kept, NOW - DAY_MS],
        )
        .unwrap();
        let report = sweep(&mut conn, &policy(None, Some(30)), NOW).unwrap();
        assert_eq!(report.purged, vec![gone]);
        assert!(!exists(&conn, gone) && exists(&conn, kept));
    }

    #[test]
    fn clear_history_respects_the_confirmation_moment() {
        let mut conn = store::open_in_memory().unwrap();
        let a = add(&conn, Kind::Text, "a", NOW - 10);
        let b = add(&conn, Kind::Text, "b", NOW - 10);
        let saved_later = add(&conn, Kind::Text, "c", NOW - 10);
        let image_kind = add(&conn, Kind::Link, "https://keep.example", NOW - 10);

        let (count, cutoff) = clearable(&conn, &[Kind::Text], NOW).unwrap();
        assert_eq!(count, 3);
        // While the confirmation is open: a new copy arrives, `b` is copied again, `c` is pinned.
        let fresh = add(&conn, Kind::Text, "fresh", NOW + 5);
        add(&conn, Kind::Text, "b", NOW + 6);
        conn.execute("UPDATE items SET pinned = 1 WHERE id = ?1", [saved_later])
            .unwrap();

        let moved = clear_history(&mut conn, &[Kind::Text], cutoff, NOW + 10).unwrap();
        assert_eq!(moved, 1);
        assert!(trashed(&conn, a));
        assert!(!trashed(&conn, b) && !trashed(&conn, fresh) && !trashed(&conn, saved_later));
        assert!(!trashed(&conn, image_kind), "other kinds stay");
    }

    #[test]
    fn clear_history_is_all_or_nothing() {
        let mut conn = store::open_in_memory().unwrap();
        let a = add(&conn, Kind::Text, "a", 1);
        let b = add(&conn, Kind::Text, "b", 1);
        conn.execute_batch(&format!(
            "CREATE TRIGGER boom BEFORE UPDATE OF trashed_at ON items WHEN NEW.id = {b}
             BEGIN SELECT RAISE(ABORT, 'disk full'); END;"
        ))
        .unwrap();
        assert!(clear_history(&mut conn, &[Kind::Text], NOW, NOW).is_err());
        assert!(!trashed(&conn, a) && !trashed(&conn, b));
    }
}
