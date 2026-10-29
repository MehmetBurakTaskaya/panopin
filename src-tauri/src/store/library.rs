//! The Library (D-053): every item with its folders and tags, and the ways to organise them.
//! The window filters and sorts on its side; this module owns the data and the rules.

use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::{
    error::{AppError, Result},
    store::{
        identity::note_identity,
        items::{Kind, SAVED},
        search,
        shelf::domain_of,
    },
};

/// Characters of a text sent to the Library; the preview fetches the whole text.
const PREVIEW_CHARS: usize = 400;
const MAX_NAME: usize = 200;
const MAX_NOTE: usize = 100_000;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItem {
    pub id: i64,
    pub kind: String,
    pub title: Option<String>,
    pub note: Option<String>,
    pub preview: Option<String>,
    pub char_count: Option<usize>,
    pub url: Option<String>,
    pub domain: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub has_thumb: bool,
    pub pinned: bool,
    pub saved: bool,
    pub edited: bool,
    pub thumb_version: i64,
    pub created_at: i64,
    pub last_copied_at: i64,
    pub trashed_at: Option<i64>,
    pub source: String,
    pub source_app: Option<String>,
    pub source_path: Option<String>,
    pub byte_size: Option<i64>,
    pub folders: Vec<i64>,
    pub tags: Vec<i64>,
    pub link_title: Option<String>,
    pub link_description: Option<String>,
    pub link_site: Option<String>,
    pub link_image: bool,
    pub link_icon: bool,
    pub link_version: i64,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Tag {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct LibraryData {
    pub items: Vec<LibraryItem>,
    pub folders: Vec<Folder>,
    pub tags: Vec<Tag>,
}

fn memberships(conn: &Connection, sql: &str) -> Result<HashMap<i64, Vec<i64>>> {
    let mut map: HashMap<i64, Vec<i64>> = HashMap::new();
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;
    for row in rows {
        let (item, group) = row?;
        map.entry(item).or_default().push(group);
    }
    Ok(map)
}

/// Everything the Library window shows, Trash included, newest copy first.
pub fn load(conn: &Connection) -> Result<LibraryData> {
    let mut folders_of = memberships(conn, "SELECT item_id, folder_id FROM item_folders")?;
    let mut tags_of = memberships(conn, "SELECT item_id, tag_id FROM item_tags")?;
    let mut stmt = conn.prepare(&format!(
        "SELECT id, kind, title, note, text_body, url, COALESCE(edit_width, width),
                COALESCE(edit_height, height), thumb_ext, pinned, {SAVED}, edit_width IS NOT NULL,
                COALESCE(edited_at, 0), created_at, last_copied_at, trashed_at, source, source_app,
                source_path, byte_size, link_title, link_description, link_site,
                link_image_ext IS NOT NULL, link_icon_ext IS NOT NULL, COALESCE(link_checked_at, 0),
                duration_ms
         FROM items ORDER BY last_copied_at DESC, id DESC"
    ))?;
    let rows = stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let body: Option<String> = row.get(4)?;
        let url: Option<String> = row.get(5)?;
        Ok(LibraryItem {
            id,
            kind: row.get(1)?,
            title: row.get(2)?,
            note: row.get(3)?,
            char_count: body.as_ref().map(|b| b.chars().count()),
            preview: body.map(|b| b.chars().take(PREVIEW_CHARS).collect()),
            domain: url.as_deref().and_then(domain_of),
            url,
            width: row.get(6)?,
            height: row.get(7)?,
            has_thumb: row.get::<_, Option<String>>(8)?.is_some(),
            pinned: row.get::<_, i64>(9)? == 1,
            saved: row.get(10)?,
            edited: row.get(11)?,
            thumb_version: row.get(12)?,
            created_at: row.get(13)?,
            last_copied_at: row.get(14)?,
            trashed_at: row.get(15)?,
            source: row.get(16)?,
            source_app: row.get(17)?,
            source_path: row.get(18)?,
            byte_size: row.get(19)?,
            folders: Vec::new(),
            tags: Vec::new(),
            link_title: row.get(20)?,
            link_description: row.get(21)?,
            link_site: row.get(22)?,
            link_image: row.get(23)?,
            link_icon: row.get(24)?,
            link_version: row.get(25)?,
            duration_ms: row.get(26)?,
        })
    })?;
    let mut items = Vec::new();
    for row in rows {
        let mut item = row?;
        item.folders = folders_of.remove(&item.id).unwrap_or_default();
        item.tags = tags_of.remove(&item.id).unwrap_or_default();
        items.push(item);
    }
    let Groups { folders, tags } = groups(conn)?;
    Ok(LibraryData {
        items,
        folders,
        tags,
    })
}

/// Folders and tags only: what the shelf's pickers need, without loading every item.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Groups {
    pub folders: Vec<Folder>,
    pub tags: Vec<Tag>,
}

pub fn groups(conn: &Connection) -> Result<Groups> {
    let folders = conn
        .prepare("SELECT id, parent_id, name FROM folders ORDER BY name COLLATE NOCASE, id")?
        .query_map([], |row| {
            Ok(Folder {
                id: row.get(0)?,
                parent_id: row.get(1)?,
                name: row.get(2)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let tags = conn
        .prepare("SELECT id, name FROM tags ORDER BY name_key, id")?
        .query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(Groups { folders, tags })
}

/// A folder or tag name: trimmed, not empty, not too long.
fn clean_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME {
        return Err(AppError::InvalidName);
    }
    Ok(name.to_owned())
}

/// Alt+P and the inspector: an empty title removes the name.
pub fn set_title(conn: &mut Connection, ids: &[i64], title: Option<&str>) -> Result<()> {
    let title = title.map(str::trim).filter(|t| !t.is_empty());
    if title.is_some_and(|t| t.chars().count() > MAX_NAME) {
        return Err(AppError::InvalidName);
    }
    let tx = conn.transaction()?;
    for id in ids {
        tx.execute(
            "UPDATE items SET title = ?2 WHERE id = ?1",
            params![id, title],
        )?;
        search::index(&tx, *id)?;
    }
    tx.commit()?;
    Ok(())
}

pub fn set_note(conn: &Connection, id: i64, note: Option<&str>) -> Result<()> {
    let note = note.filter(|n| !n.trim().is_empty());
    if note.is_some_and(|n| n.len() > MAX_NOTE) {
        return Err(AppError::InvalidEdit("note too long"));
    }
    conn.execute(
        "UPDATE items SET note = ?2 WHERE id = ?1",
        params![id, note],
    )?;
    search::index(conn, id)
}

pub fn create_folder(conn: &Connection, name: &str, parent: Option<i64>, now: i64) -> Result<i64> {
    let name = clean_name(name)?;
    conn.execute(
        "INSERT INTO folders (parent_id, name, created_at) VALUES (?1, ?2, ?3)",
        params![parent, name, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn rename_folder(conn: &Connection, id: i64, name: &str) -> Result<()> {
    conn.execute(
        "UPDATE folders SET name = ?2 WHERE id = ?1",
        params![id, clean_name(name)?],
    )?;
    Ok(())
}

/// Deletes the folder and its subfolders; the items stay where else they are.
pub fn delete_folder(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM folders WHERE id = ?1", [id])?;
    Ok(())
}

pub fn add_to_folder(conn: &mut Connection, ids: &[i64], folder: i64) -> Result<()> {
    let tx = conn.transaction()?;
    for id in ids {
        tx.execute(
            "INSERT OR IGNORE INTO item_folders (item_id, folder_id) VALUES (?1, ?2)",
            params![id, folder],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn remove_from_folder(conn: &mut Connection, ids: &[i64], folder: i64) -> Result<()> {
    let tx = conn.transaction()?;
    for id in ids {
        tx.execute(
            "DELETE FROM item_folders WHERE item_id = ?1 AND folder_id = ?2",
            params![id, folder],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// The tag with this name (ignoring letter case), created when it does not exist yet.
/// What makes two tag names the same tag: the four Turkish i forms (İ, I, ı, i) are one letter,
/// and letter case does not count.
fn name_key(name: &str) -> String {
    name.chars()
        .flat_map(|c| match c {
            'İ' | 'I' | 'ı' => 'i'.to_lowercase(),
            c => c.to_lowercase(),
        })
        .collect()
}

pub fn tag_named(conn: &Connection, name: &str) -> Result<i64> {
    let name = clean_name(name)?;
    let key = name_key(&name);
    let existing: Option<i64> = conn
        .query_row("SELECT id FROM tags WHERE name_key = ?1", [&key], |row| {
            row.get(0)
        })
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }
    conn.execute(
        "INSERT INTO tags (name, name_key) VALUES (?1, ?2)",
        params![name, key],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn rename_tag(conn: &Connection, id: i64, name: &str) -> Result<()> {
    let name = clean_name(name)?;
    let key = name_key(&name);
    let other: Option<i64> = conn
        .query_row(
            "SELECT id FROM tags WHERE name_key = ?1 AND id <> ?2",
            params![key, id],
            |row| row.get(0),
        )
        .optional()?;
    if other.is_some() {
        return Err(AppError::NameTaken);
    }
    conn.execute(
        "UPDATE tags SET name = ?2, name_key = ?3 WHERE id = ?1",
        params![id, name, key],
    )?;
    search::index_many(conn, &search::tagged(conn, id)?)
}

/// Removes the tag from every item.
pub fn delete_tag(conn: &Connection, id: i64) -> Result<()> {
    let tagged = search::tagged(conn, id)?;
    conn.execute("DELETE FROM tags WHERE id = ?1", [id])?;
    search::index_many(conn, &tagged)
}

/// Adds a tag; the items keep the tags they already have.
pub fn add_tag(conn: &mut Connection, ids: &[i64], tag: i64) -> Result<()> {
    let tx = conn.transaction()?;
    for id in ids {
        tx.execute(
            "INSERT OR IGNORE INTO item_tags (item_id, tag_id) VALUES (?1, ?2)",
            params![id, tag],
        )?;
        search::index(&tx, *id)?;
    }
    tx.commit()?;
    Ok(())
}

pub fn remove_tag(conn: &mut Connection, ids: &[i64], tag: i64) -> Result<()> {
    let tx = conn.transaction()?;
    for id in ids {
        tx.execute(
            "DELETE FROM item_tags WHERE item_id = ?1 AND tag_id = ?2",
            params![id, tag],
        )?;
        search::index(&tx, *id)?;
    }
    tx.commit()?;
    Ok(())
}

/// Takes items out of Trash, back where they were.
pub fn restore(conn: &mut Connection, ids: &[i64]) -> Result<()> {
    let tx = conn.transaction()?;
    for id in ids {
        tx.execute("UPDATE items SET trashed_at = NULL WHERE id = ?1", [id])?;
    }
    tx.commit()?;
    Ok(())
}

/// Deletes items in Trash for good and returns their ids, so the caller removes their media.
/// Items outside Trash are never deleted here.
pub fn delete_forever(conn: &mut Connection, ids: &[i64]) -> Result<Vec<i64>> {
    let tx = conn.transaction()?;
    let mut deleted = Vec::new();
    for id in ids {
        if tx.execute(
            "DELETE FROM items WHERE id = ?1 AND trashed_at IS NOT NULL",
            [id],
        )? == 1
        {
            deleted.push(*id);
        }
    }
    tx.commit()?;
    Ok(deleted)
}

pub fn empty_trash(conn: &mut Connection) -> Result<Vec<i64>> {
    let tx = conn.transaction()?;
    let deleted = tx
        .prepare("DELETE FROM items WHERE trashed_at IS NOT NULL RETURNING id")?
        .query_map([], |row| row.get(0))?
        .collect::<std::result::Result<Vec<i64>, _>>()?;
    tx.commit()?;
    Ok(deleted)
}

/// Ctrl+N: an empty text note. It is pinned, because it was made on purpose and should not
/// expire like a copy.
pub fn new_note(conn: &Connection, now: i64) -> Result<i64> {
    conn.execute(
        "INSERT INTO items (kind, identity, created_at, last_copied_at, source, pinned, text_body)
         VALUES (?1, ?2, ?3, ?3, 'manual', 1, '')",
        params![Kind::Text.as_str(), &note_identity(now)[..], now],
    )?;
    let id = conn.last_insert_rowid();
    search::index(conn, id)?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{
        self,
        identity::text_identity,
        items::{upsert_text, TextCapture},
        retention::{self, Policy, DAY_MS},
    };

    fn add(conn: &Connection, body: &str, at: i64) -> i64 {
        let capture = TextCapture {
            kind: Kind::Text,
            body,
            url: None,
            identity: text_identity(body),
            source_app: None,
        };
        upsert_text(conn, &capture, at).unwrap()
    }

    fn item(conn: &Connection, id: i64) -> LibraryItem {
        load(conn)
            .unwrap()
            .items
            .into_iter()
            .find(|i| i.id == id)
            .unwrap()
    }

    #[test]
    fn folders_and_tags_attach_and_make_items_saved() {
        let mut conn = store::open_in_memory().unwrap();
        let a = add(&conn, "a", 1);
        let b = add(&conn, "b", 2);
        assert!(!item(&conn, a).saved);
        let work = create_folder(&conn, " Work ", None, 5).unwrap();
        let sub = create_folder(&conn, "Sub", Some(work), 6).unwrap();
        add_to_folder(&mut conn, &[a, b], sub).unwrap();
        add_to_folder(&mut conn, &[a], sub).unwrap(); // twice is fine
        let urgent = tag_named(&conn, "urgent").unwrap();
        assert_eq!(
            tag_named(&conn, "URGENT").unwrap(),
            urgent,
            "tags ignore letter case"
        );
        add_tag(&mut conn, &[a], urgent).unwrap();
        let data = load(&conn).unwrap();
        assert_eq!(
            data.folders
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>(),
            ["Sub", "Work"]
        );
        let a_item = item(&conn, a);
        assert_eq!(
            (a_item.folders.clone(), a_item.tags.clone(), a_item.saved),
            (vec![sub], vec![urgent], true)
        );

        // Saved items survive retention; deleting the groups makes them ordinary again.
        delete_folder(&conn, work).unwrap();
        assert!(
            load(&conn).unwrap().folders.is_empty(),
            "subfolders go with their parent"
        );
        assert!(item(&conn, a).saved, "still tagged");
        delete_tag(&conn, urgent).unwrap();
        assert!(!item(&conn, a).saved);
        let policy = Policy {
            image: Some(1),
            text: Some(1),
            link: Some(1),
            video: Some(1),
            trash: None,
        };
        retention::sweep(&mut conn, &policy, 10 * DAY_MS).unwrap();
        assert!(item(&conn, b).trashed_at.is_some());
    }

    #[test]
    fn names_titles_and_notes_are_cleaned() {
        let mut conn = store::open_in_memory().unwrap();
        let a = add(&conn, "a", 1);
        assert!(create_folder(&conn, "   ", None, 1).is_err());
        set_title(&mut conn, &[a], Some("  Invoice  ")).unwrap();
        set_note(&conn, a, Some("remember")).unwrap();
        let it = item(&conn, a);
        assert_eq!(
            (it.title.as_deref(), it.note.as_deref(), it.saved),
            (Some("Invoice"), Some("remember"), true)
        );
        set_title(&mut conn, &[a], Some("")).unwrap();
        assert_eq!(item(&conn, a).title, None);
        let x = tag_named(&conn, "x").unwrap();
        let y = tag_named(&conn, "y").unwrap();
        assert!(matches!(
            rename_tag(&conn, y, "X"),
            Err(AppError::NameTaken)
        ));
        rename_tag(&conn, x, "Xenon").unwrap();
        rename_tag(&conn, x, "XENON").unwrap(); // its own name in other letters
    }

    #[test]
    fn turkish_i_forms_name_one_tag() {
        let conn = store::open_in_memory().unwrap();
        let work = tag_named(&conn, "İş").unwrap();
        for same in ["iş", "IŞ", "ış"] {
            assert_eq!(tag_named(&conn, same).unwrap(), work, "{same}");
        }
        assert_ne!(tag_named(&conn, "eş").unwrap(), work);
        assert_eq!(
            groups(&conn).unwrap().tags[1].name,
            "İş",
            "the first spelling stays"
        );
    }

    #[test]
    fn trash_restores_and_deletes_only_trashed_items() {
        let mut conn = store::open_in_memory().unwrap();
        let a = add(&conn, "a", 1);
        let b = add(&conn, "b", 1);
        conn.execute(
            "UPDATE items SET trashed_at = 5 WHERE id IN (?1, ?2)",
            params![a, b],
        )
        .unwrap();
        restore(&mut conn, &[a]).unwrap();
        assert_eq!(item(&conn, a).trashed_at, None);
        assert_eq!(
            delete_forever(&mut conn, &[a, b]).unwrap(),
            vec![b],
            "only Trash is deleted"
        );
        conn.execute("UPDATE items SET trashed_at = 5 WHERE id = ?1", [a])
            .unwrap();
        assert_eq!(empty_trash(&mut conn).unwrap(), vec![a]);
        assert!(load(&conn).unwrap().items.is_empty());
    }

    #[test]
    fn new_notes_are_pinned_and_unique() {
        let conn = store::open_in_memory().unwrap();
        let first = new_note(&conn, 10).unwrap();
        let second = new_note(&conn, 10).unwrap();
        assert_ne!(first, second);
        let note = item(&conn, first);
        assert!(note.pinned && note.saved);
        assert_eq!(
            (note.source.as_str(), note.preview.as_deref()),
            ("manual", Some(""))
        );
    }
}
