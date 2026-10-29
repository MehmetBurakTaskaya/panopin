//! Typed settings stored as one JSON document in SQLite (D-011). Missing fields fall back to
//! defaults, so older stored documents keep working as settings are added.

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;

const KEY: &str = "app";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    System,
    En,
    Tr,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

/// Global shortcuts in Tauri accelerator syntax (`Super` is the Windows key); `None` means off.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Shortcuts {
    pub shelf: Option<String>,
    pub shelf_search: Option<String>,
    pub snip: Option<String>,
    pub library: Option<String>,
    pub copy_last_image: Option<String>,
    pub pause_capture: Option<String>,
}

impl Default for Shortcuts {
    // Approved defaults (D-020): no Ctrl, so AltGr can never trigger them.
    fn default() -> Self {
        Self {
            shelf: Some("Super+Alt+V".into()),
            shelf_search: Some("Super+Alt+C".into()),
            snip: Some("Super+Alt+S".into()),
            library: None,
            copy_last_image: None,
            pause_capture: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub language: Language,
    pub theme: Theme,
    pub shortcuts: Shortcuts,
}

pub fn load(conn: &Connection) -> Result<Settings> {
    let stored: Option<String> = conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [KEY], |row| {
            row.get(0)
        })
        .optional()?;
    Ok(match stored {
        None => Settings::default(),
        Some(json) => serde_json::from_str(&json).unwrap_or_else(|err| {
            log::warn!("stored settings are unreadable, using defaults: {err}");
            Settings::default()
        }),
    })
}

pub fn save(conn: &Connection, settings: &Settings) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        (KEY, serde_json::to_string(settings)?),
    )?;
    Ok(())
}

/// Applies a partial update: only the fields present in `patch` change, and the result must
/// still be a valid `Settings`.
pub fn apply_patch(current: &Settings, patch: Value) -> Result<Settings> {
    let mut merged = serde_json::to_value(current)?;
    merge(&mut merged, patch);
    Ok(serde_json::from_value(merged)?)
}

fn merge(target: &mut Value, patch: Value) {
    match (target, patch) {
        (Value::Object(target), Value::Object(patch)) => {
            for (key, value) in patch {
                merge(target.entry(key).or_insert(Value::Null), value);
            }
        }
        (target, patch) => *target = patch,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::store;

    #[test]
    fn defaults_match_the_approved_shortcuts() {
        let s = Settings::default();
        assert_eq!(s.language, Language::System);
        assert_eq!(s.shortcuts.shelf.as_deref(), Some("Super+Alt+V"));
        assert_eq!(s.shortcuts.shelf_search.as_deref(), Some("Super+Alt+C"));
        assert_eq!(s.shortcuts.snip.as_deref(), Some("Super+Alt+S"));
        assert_eq!(s.shortcuts.library, None);
    }

    #[test]
    fn save_and_load_round_trip() {
        let conn = store::open_in_memory().unwrap();
        assert_eq!(load(&conn).unwrap(), Settings::default());
        let s = Settings {
            language: Language::Tr,
            shortcuts: Shortcuts {
                library: Some("Super+Alt+L".into()),
                ..Shortcuts::default()
            },
            ..Settings::default()
        };
        save(&conn, &s).unwrap();
        save(&conn, &s).unwrap(); // upsert, not a second row
        assert_eq!(load(&conn).unwrap(), s);
    }

    #[test]
    fn partial_document_fills_in_defaults() {
        let conn = store::open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('app', ?1)",
            [r#"{"language":"tr","futureField":true}"#],
        )
        .unwrap();
        let s = load(&conn).unwrap();
        assert_eq!(s.language, Language::Tr);
        assert_eq!(s.shortcuts, Shortcuts::default());
    }

    #[test]
    fn unreadable_document_falls_back_to_defaults() {
        let conn = store::open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('app', ?1)",
            [r#"{"language":42}"#],
        )
        .unwrap();
        assert_eq!(load(&conn).unwrap(), Settings::default());
    }

    #[test]
    fn patch_changes_only_given_fields() {
        let s = apply_patch(
            &Settings::default(),
            json!({ "language": "en", "shortcuts": { "shelfSearch": null } }),
        )
        .unwrap();
        assert_eq!(s.language, Language::En);
        assert_eq!(s.shortcuts.shelf_search, None);
        assert_eq!(s.shortcuts.shelf.as_deref(), Some("Super+Alt+V"));
    }

    #[test]
    fn invalid_patch_is_rejected() {
        assert!(apply_patch(&Settings::default(), json!({ "language": "de" })).is_err());
        assert!(apply_patch(&Settings::default(), json!({ "theme": 3 })).is_err());
    }
}
