//! Search (D-017): Rust folds the searchable fields of an item into the FTS5 table `search`.
//! Folding is for the index only; item identity never sees it (D-016).
//!
//! Every change to a searchable field calls [`index`]; a deleted item leaves the index through a
//! trigger. The index is rebuilt once when [`INDEX_VERSION`] changes.

use rusqlite::{params, Connection, OptionalExtension};
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

use crate::error::Result;

/// Bumped when folding or the indexed fields change; the index is then rebuilt once.
/// 2: accents of every language, not only the Turkish ones (D-082).
pub const INDEX_VERSION: i64 = 2;
const VERSION_KEY: &str = "search_index";
/// Characters of a text body that go into the index; longer texts are found by their start.
const BODY_CHARS: usize = 200_000;
/// Matches returned at most; the Library shows everything, the shelf only its rows.
pub const MAX_RESULTS: usize = 20_000;

/// The four Turkish i forms (İ, I, ı, i) are one letter, and case does not count.
pub fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.nfc() {
        match c {
            'İ' | 'I' | 'ı' => out.push('i'),
            c => out.extend(c.to_lowercase()),
        }
    }
    out
}

/// A folded text without accents, for searching in any language (D-082): "çiçek", "café" and
/// "ångström" become "cicek", "cafe" and "angstrom", Greek loses its tonos, Russian ё becomes е,
/// and Arabic loses its vowel marks and hamza seats. Marks that make a letter of their own stay:
/// й, kana with dakuten, Hangul.
pub fn bare(folded: &str) -> String {
    folded.chars().filter_map(bare_char).collect()
}

/// One character without its accent; `None` drops a vowel mark written on its own.
fn bare_char(c: char) -> Option<char> {
    match c {
        // Letters with a stroke or a ring that no decomposition takes apart.
        'ø' => return Some('o'),
        'ł' => return Some('l'),
        'đ' | 'ð' => return Some('d'),
        'ħ' => return Some('h'),
        'ё' => return Some('е'),
        // Latin and Greek accents and Arabic vowel marks written after their letter.
        '\u{0300}'..='\u{036F}' | '\u{064B}'..='\u{065F}' | '\u{0670}' => return None,
        // Accents come off in Latin, Greek and Arabic only.
        '\u{00C0}'..='\u{024F}'
        | '\u{1E00}'..='\u{1EFF}'
        | '\u{0370}'..='\u{03FF}'
        | '\u{1F00}'..='\u{1FFF}'
        | '\u{0620}'..='\u{06FF}' => {}
        _ => return Some(c),
    }
    let mut parts = std::iter::once(c).nfd();
    let base = parts.next().unwrap_or(c);
    Some(if parts.all(is_combining_mark) {
        base
    } else {
        c
    })
}

struct Document {
    title: String,
    body: String,
    extra: String,
}

fn join(parts: &[Option<String>]) -> String {
    parts
        .iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" \n ")
}

/// Title: the given name and a link's page title. Body: a text, or a link's address.
/// Extra: note, tags, a link's site and the text found in an image.
fn document(conn: &Connection, id: i64) -> Result<Option<Document>> {
    Ok(conn
        .query_row(
            "SELECT kind, title, link_title, text_body, url, note, link_site, ocr_text,
                    (SELECT group_concat(tags.name, ' ')
                     FROM item_tags JOIN tags ON tags.id = item_tags.tag_id
                     WHERE item_tags.item_id = items.id)
             FROM items WHERE id = ?1",
            [id],
            |row| {
                let kind: String = row.get(0)?;
                let body: Option<String> = if kind == "link" {
                    row.get(4)?
                } else {
                    row.get::<_, Option<String>>(3)?
                        .map(|text| text.chars().take(BODY_CHARS).collect())
                };
                let url: Option<String> = row.get(4)?;
                let domain = url.as_deref().and_then(super::shelf::domain_of);
                Ok(Document {
                    title: join(&[row.get(1)?, row.get(2)?]),
                    body: body.unwrap_or_default(),
                    extra: join(&[row.get(5)?, row.get(8)?, row.get(6)?, domain, row.get(7)?]),
                })
            },
        )
        .optional()?)
}

/// Brings one item's index row up to date (or removes it when the item is gone).
pub fn index(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM search WHERE rowid = ?1", [id])?;
    let Some(doc) = document(conn, id)? else {
        return Ok(());
    };
    let (title, body, extra) = (fold(&doc.title), fold(&doc.body), fold(&doc.extra));
    conn.execute(
        "INSERT INTO search (rowid, title, body, extra, title_bare, body_bare, extra_bare)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            title,
            body,
            extra,
            bare(&title),
            bare(&body),
            bare(&extra)
        ],
    )?;
    Ok(())
}

pub fn index_many(conn: &Connection, ids: &[i64]) -> Result<()> {
    ids.iter().try_for_each(|&id| index(conn, id))
}

/// Items carrying a tag, for when the tag is renamed or deleted.
pub fn tagged(conn: &Connection, tag: i64) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare("SELECT item_id FROM item_tags WHERE tag_id = ?1")?;
    let ids = stmt.query_map([tag], |row| row.get(0))?;
    Ok(ids.collect::<std::result::Result<Vec<_>, _>>()?)
}

/// The FTS5 expression for what the user typed: every word is a prefix, all must match. `None`
/// when nothing searchable was typed.
pub fn expression(query: &str, without_accents: bool) -> Option<String> {
    let folded = fold(query);
    let folded = if without_accents {
        bare(&folded)
    } else {
        folded
    };
    // Words split the way the unicode61 tokenizer splits them: letters and digits only, so a
    // term never holds a quote or an operator.
    let terms: Vec<String> = folded
        .split(|c: char| !c.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .take(16)
        .map(|term| format!("\"{term}\"*"))
        .collect();
    if terms.is_empty() {
        return None;
    }
    let columns = if without_accents {
        "{title_bare body_bare extra_bare}"
    } else {
        "{title body extra}"
    };
    Some(format!("{columns} : ({})", terms.join(" AND ")))
}

/// Ids of the matching items, best match first; Trash included, the caller decides.
pub fn find(conn: &Connection, query: &str, without_accents: bool) -> Result<Vec<i64>> {
    let Some(expression) = expression(query, without_accents) else {
        return Ok(Vec::new());
    };
    let mut stmt = conn.prepare_cached(
        "SELECT rowid FROM search WHERE search MATCH ?1
         ORDER BY bm25(search, 5.0, 1.0, 2.0, 5.0, 1.0, 2.0) LIMIT ?2",
    )?;
    let ids = stmt.query_map(params![expression, MAX_RESULTS as i64], |row| row.get(0))?;
    Ok(ids.collect::<std::result::Result<Vec<_>, _>>()?)
}

/// Whether the index was built by an older version and needs a rebuild.
pub fn outdated(conn: &Connection) -> Result<bool> {
    let built: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [VERSION_KEY],
            |row| row.get(0),
        )
        .optional()?;
    Ok(built.and_then(|v| v.parse::<i64>().ok()) != Some(INDEX_VERSION))
}

pub fn clear(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM search", [])?;
    Ok(())
}

/// Indexes up to `batch` items with ids above `after`; returns the last id done, `None` at the end.
pub fn rebuild_batch(conn: &mut Connection, after: i64, batch: usize) -> Result<Option<i64>> {
    let ids: Vec<i64> = {
        let mut stmt = conn.prepare("SELECT id FROM items WHERE id > ?1 ORDER BY id LIMIT ?2")?;
        let rows = stmt.query_map(params![after, batch as i64], |row| row.get(0))?;
        rows.collect::<std::result::Result<_, _>>()?
    };
    let Some(&last) = ids.last() else {
        return Ok(None);
    };
    let tx = conn.transaction()?;
    index_many(&tx, &ids)?;
    tx.commit()?;
    Ok(Some(last))
}

pub fn mark_built(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![VERSION_KEY, INDEX_VERSION.to_string()],
    )?;
    Ok(())
}

/// Rebuilds the whole index at once; tests and small databases.
#[cfg(test)]
pub fn rebuild(conn: &mut Connection) -> Result<()> {
    clear(conn)?;
    let mut after = 0;
    while let Some(last) = rebuild_batch(conn, after, 500)? {
        after = last;
    }
    mark_built(conn)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::store;

    fn add_text(conn: &Connection, body: &str) -> i64 {
        conn.execute(
            "INSERT INTO items (kind, identity, created_at, last_copied_at, source, text_body)
             VALUES ('text', randomblob(32), 1, 1, 'clipboard', ?1)",
            [body],
        )
        .unwrap();
        let id = conn.last_insert_rowid();
        index(conn, id).unwrap();
        id
    }

    #[test]
    fn folding_unifies_the_turkish_i_and_case() {
        assert_eq!(fold("İSTANBUL Işık ılık"), "istanbul işik ilik");
        assert_eq!(fold("ÇİÇEK Şeker"), "çiçek şeker");
        assert_eq!(bare(&fold("ÇİÇEK Ğüöşâ")), "cicek guosa");
        // A decomposed İ (I + combining dot) folds like the composed one.
        assert_eq!(fold("I\u{307}stanbul"), "istanbul");
    }

    #[test]
    fn bare_drops_the_accents_of_every_language() {
        let bare_of = |text: &str| bare(&fold(text));
        assert_eq!(
            bare_of("Über Café Naïve Señor Ångström Øre Łódź Việt"),
            "uber cafe naive senor angstrom ore lodz viet"
        );
        assert_eq!(bare_of("Ελλάδα"), "ελλαδα");
        assert_eq!(bare_of("Ёлка йод"), "елка йод", "й is a letter of its own");
        assert_eq!(bare_of("أحمد إسلام آمن"), "احمد اسلام امن");
        assert_eq!(bare_of("كَتَبَ"), "كتب", "vowel marks go");
        assert_eq!(
            bare_of("がっこう 한국 中文"),
            "がっこう 한국 中文",
            "no change outside them"
        );
    }

    #[test]
    fn expressions_use_prefixes_and_never_pass_operators() {
        assert_eq!(
            expression("İst  ank", false).as_deref(),
            Some("{title body extra} : (\"ist\"* AND \"ank\"*)")
        );
        assert_eq!(
            expression("ÇİÇ", true).as_deref(),
            Some("{title_bare body_bare extra_bare} : (\"cic\"*)")
        );
        assert_eq!(
            expression("a\" OR b NEAR(c", false).as_deref(),
            Some("{title body extra} : (\"a\"* AND \"or\"* AND \"b\"* AND \"near\"* AND \"c\"*)")
        );
        assert_eq!(expression(" -*\"() ", false), None);
    }

    #[test]
    fn turkish_text_is_found_from_any_spelling() {
        let conn = store::open_in_memory().unwrap();
        let istanbul = add_text(&conn, "Toplantı İstanbul ofisinde");
        let light = add_text(&conn, "IŞIK AÇIK KALSIN");
        let flower = add_text(&conn, "Bahçede çiçek açtı");
        let ids = |q: &str, bare: bool| find(&conn, q, bare).unwrap();
        assert_eq!(ids("istanbul", false), vec![istanbul]);
        assert_eq!(ids("İSTAN", false), vec![istanbul]);
        assert_eq!(ids("ışık", false), vec![light]);
        assert_eq!(
            ids("isik", false),
            Vec::<i64>::new(),
            "accents count by default"
        );
        assert_eq!(ids("isik", true), vec![light]);
        assert_eq!(ids("cicek", true), vec![flower]);
        assert_eq!(ids("çiçek bahç", false), vec![flower]);
        let cafe = add_text(&conn, "Café crème à Paris");
        assert_eq!(ids("cafe creme", true), vec![cafe]);
        assert_eq!(ids("cafe", false), Vec::<i64>::new());
        assert_eq!(
            ids("çiçek ofis", false),
            Vec::<i64>::new(),
            "every word must match"
        );
    }

    #[test]
    fn titles_notes_tags_links_and_ocr_are_searchable() {
        let conn = store::open_in_memory().unwrap();
        let text = add_text(&conn, "plain words");
        conn.execute(
            "UPDATE items SET title = 'Fatura', note = 'Muhasebeye gönder' WHERE id = ?1",
            [text],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tags (name, name_key) VALUES ('Önemli', 'önemli')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO item_tags (item_id, tag_id) VALUES (?1, ?2)",
            params![text, conn.last_insert_rowid()],
        )
        .unwrap();
        index(&conn, text).unwrap();
        conn.execute(
            "INSERT INTO items (kind, identity, created_at, last_copied_at, source, url, link_title)
             VALUES ('link', x'01', 1, 1, 'clipboard', 'https://www.example.org/a', 'Örnek Sayfa')",
            [],
        )
        .unwrap();
        let link = conn.last_insert_rowid();
        index(&conn, link).unwrap();
        conn.execute(
            "INSERT INTO items (kind, identity, created_at, last_copied_at, source, ocr_state, ocr_text)
             VALUES ('image', x'02', 1, 1, 'clipboard', 1, 'Fiş toplamı 245 TL')",
            [],
        )
        .unwrap();
        let image = conn.last_insert_rowid();
        index(&conn, image).unwrap();
        for (query, expected) in [
            ("fatura", text),
            ("muhaseb", text),
            ("önem", text),
            ("örnek", link),
            ("example", link),
            ("toplam", image),
            ("245", image),
        ] {
            assert_eq!(
                find(&conn, query, false).unwrap(),
                vec![expected],
                "{query}"
            );
        }
        // Deleting an item takes it out of the index.
        conn.execute("DELETE FROM items WHERE id = ?1", [image])
            .unwrap();
        assert!(find(&conn, "toplam", false).unwrap().is_empty());
    }

    #[test]
    fn a_rebuild_restores_every_row() {
        let mut conn = store::open_in_memory().unwrap();
        let id = add_text(&conn, "kalem defter");
        assert!(outdated(&conn).unwrap());
        clear(&conn).unwrap();
        assert!(find(&conn, "kalem", false).unwrap().is_empty());
        rebuild(&mut conn).unwrap();
        assert_eq!(find(&conn, "kalem", false).unwrap(), vec![id]);
        assert!(!outdated(&conn).unwrap());
    }

    /// The target (D-059): below 50 ms with more than 10 000 items, measured.
    #[test]
    fn ten_thousand_items_answer_in_under_50_ms() {
        const WORDS: &[&str] = &[
            "toplantı",
            "İstanbul",
            "ankara",
            "fatura",
            "ödeme",
            "çiçek",
            "ışık",
            "şeker",
            "güncelleme",
            "rapor",
            "proje",
            "müşteri",
            "teklif",
            "sözleşme",
            "invoice",
            "meeting",
            "release",
            "build",
            "error",
            "warning",
            "kalem",
            "defter",
            "kitap",
            "masa",
            "kapı",
            "pencere",
            "github",
            "https",
            "example",
            "notes",
            "yarın",
            "bugün",
            "akşam",
            "sabah",
            "kod",
            "test",
            "server",
            "client",
            "database",
            "query",
        ];
        let mut conn = store::open_in_memory().unwrap();
        let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        // A third of the words are made up, so the vocabulary is large like real clipboard text.
        const LETTERS: &[char] = &[
            'a', 'b', 'c', 'ç', 'd', 'e', 'f', 'g', 'ğ', 'h', 'ı', 'i', 'k', 'l', 'm', 'n', 'o',
            'ö', 'p', 'r', 's', 'ş', 't', 'u', 'ü', 'v', 'y', 'z',
        ];
        {
            let tx = conn.transaction().unwrap();
            for n in 0..10_500 {
                let words = 8 + (next() % 40) as usize;
                let body: Vec<String> = (0..words)
                    .map(|_| {
                        if next() % 3 == 0 {
                            let len = 3 + (next() % 7) as usize;
                            (0..len)
                                .map(|_| LETTERS[(next() % LETTERS.len() as u64) as usize])
                                .collect()
                        } else {
                            WORDS[(next() % WORDS.len() as u64) as usize].to_owned()
                        }
                    })
                    .collect();
                tx.execute(
                    "INSERT INTO items (kind, identity, created_at, last_copied_at, source, text_body)
                     VALUES ('text', ?1, ?2, ?2, 'clipboard', ?3)",
                    params![n.to_string().into_bytes(), n, body.join(" ")],
                )
                .unwrap();
            }
            tx.commit().unwrap();
        }
        rebuild(&mut conn).unwrap();
        for (query, bare) in [
            ("i", false),
            ("is", false),
            ("ista", false),
            ("istanbul", false),
            ("ışık", false),
            ("isik", true),
            ("cicek", true),
            ("toplantı fatura", false),
            ("ser cli", false),
        ] {
            let best = (0..3)
                .map(|_| {
                    let start = Instant::now();
                    let ids = find(&conn, query, bare).unwrap();
                    assert!(!ids.is_empty(), "{query}");
                    start.elapsed()
                })
                .min()
                .unwrap();
            println!("search {query:?}: {best:?}");
            assert!(best < Duration::from_millis(50), "{query}: {best:?}");
        }
    }
}
