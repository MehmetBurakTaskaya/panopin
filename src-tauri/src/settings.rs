//! Typed settings stored as one JSON document in SQLite (D-011). Missing fields fall back to
//! defaults, so older stored documents keep working as settings are added.

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    error::{AppError, Result},
    store::retention,
};

const KEY: &str = "app";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    System,
    En,
    Tr,
    De,
    Es,
    Fr,
    It,
    #[serde(rename = "pt-BR")]
    PtBr,
    Ru,
    Ar,
    Ja,
    Ko,
    #[serde(rename = "zh-CN")]
    ZhCn,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CardSize {
    S,
    #[default]
    M,
    L,
}

/// The screen edge the shelf slides in from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShelfEdge {
    #[default]
    Top,
    Bottom,
}

/// Interface sounds; on and quiet by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Sounds {
    pub enabled: bool,
    /// 0 to 100.
    pub volume: u8,
}

impl Default for Sounds {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 30,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LibraryView {
    #[default]
    Grid,
    List,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortKey {
    #[default]
    LastCopied,
    Added,
    Title,
    Kind,
    Tags,
}

/// List column widths in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Columns {
    pub title: u32,
    pub kind: u32,
    pub tags: u32,
    pub added: u32,
}

impl Default for Columns {
    fn default() -> Self {
        Self {
            title: 360,
            kind: 110,
            tags: 200,
            added: 160,
        }
    }
}

/// How the Library looks; kept so it opens the way it was left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LibraryLayout {
    pub view: LibraryView,
    pub sort: SortKey,
    pub descending: bool,
    pub sidebar_width: u32,
    pub inspector_width: u32,
    pub columns: Columns,
}

impl Default for LibraryLayout {
    fn default() -> Self {
        Self {
            view: LibraryView::Grid,
            sort: SortKey::LastCopied,
            descending: true,
            sidebar_width: 230,
            inspector_width: 300,
            columns: Columns::default(),
        }
    }
}

impl LibraryLayout {
    fn widths_ok(&self) -> bool {
        let c = self.columns;
        [
            self.sidebar_width,
            self.inspector_width,
            c.title,
            c.kind,
            c.tags,
            c.added,
        ]
        .iter()
        .all(|w| (60..=1600).contains(w))
    }
}

/// Look of the windows (D-060).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Appearance {
    /// The Windows accent colour instead of Panopin's own blue.
    pub system_accent: bool,
    /// Mica behind the Library and Settings on Windows 11.
    pub material: bool,
    /// Off: no animations at all. On: they follow Windows' "Animation effects".
    pub animations: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            system_accent: true,
            material: true,
            animations: true,
        }
    }
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

/// What gets captured; each source is switched on or off on its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Sources {
    pub screenshots: bool,
    pub clipboard_images: bool,
    pub clipboard_text: bool,
    pub links: bool,
    pub screen_recordings: bool,
}

impl Default for Sources {
    fn default() -> Self {
        Self {
            screenshots: true,
            clipboard_images: true,
            clipboard_text: true,
            links: true,
            screen_recordings: false,
        }
    }
}

/// Days to keep unsaved items per kind, and days before Trash empties; `None` = keep forever.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Retention {
    pub image: Option<u32>,
    pub text: Option<u32>,
    pub link: Option<u32>,
    pub video: Option<u32>,
    pub trash: Option<u32>,
}

impl Default for Retention {
    fn default() -> Self {
        Self {
            image: Some(30),
            text: Some(30),
            link: Some(30),
            video: Some(30),
            trash: Some(30),
        }
    }
}

impl Retention {
    pub fn policy(&self) -> retention::Policy {
        retention::Policy {
            image: self.image,
            text: self.text,
            link: self.link,
            video: self.video,
            trash: self.trash,
        }
    }
}

pub const MAX_WATCHED_FOLDERS: usize = 3;
/// `paused_until` value meaning "until the user resumes".
pub const PAUSED_UNTIL_RESUMED: i64 = i64::MAX;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub language: Language,
    pub theme: Theme,
    pub shortcuts: Shortcuts,
    /// Capture starts only after the first-run screen has been confirmed.
    pub onboarded: bool,
    pub sources: Sources,
    /// Extra screenshot folders besides the Windows Screenshots folder.
    pub watched_folders: Vec<String>,
    /// Executable names whose copies are never captured (compared case-insensitively).
    pub excluded_apps: Vec<String>,
    pub retention: Retention,
    pub link_previews: bool,
    /// Capture is paused until this Unix time in milliseconds.
    pub paused_until: Option<i64>,
    pub card_size: CardSize,
    pub shelf_edge: ShelfEdge,
    pub sounds: Sounds,
    pub library: LibraryLayout,
    /// Search also ignores the Turkish accents: "cicek" finds "çiçek".
    pub search_without_accents: bool,
    /// Text in images is read in the background (Windows OCR).
    pub ocr: bool,
    pub launch_at_login: bool,
    /// Resting the pointer on the shelf's screen edge opens it (D-022).
    pub edge_open: bool,
    pub appearance: Appearance,
}

/// Password managers whose copies are never kept, named as each system lists its apps.
#[cfg(windows)]
const EXCLUDED_BY_DEFAULT: [&str; 4] = [
    "KeePass.exe",
    "KeePassXC.exe",
    "1Password.exe",
    "Bitwarden.exe",
];
#[cfg(not(windows))]
const EXCLUDED_BY_DEFAULT: [&str; 3] = ["KeePassXC", "1Password", "Bitwarden"];

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::default(),
            theme: Theme::default(),
            shortcuts: Shortcuts::default(),
            onboarded: false,
            sources: Sources::default(),
            watched_folders: Vec::new(),
            excluded_apps: EXCLUDED_BY_DEFAULT.map(String::from).to_vec(),
            retention: Retention::default(),
            link_previews: false,
            paused_until: None,
            card_size: CardSize::default(),
            shelf_edge: ShelfEdge::default(),
            sounds: Sounds::default(),
            library: LibraryLayout::default(),
            search_without_accents: false,
            ocr: true,
            launch_at_login: false,
            edge_open: false,
            appearance: Appearance::default(),
        }
    }
}

impl Settings {
    pub fn is_paused(&self, now: i64) -> bool {
        self.paused_until.is_some_and(|until| until > now)
    }

    fn validate(&self) -> Result<()> {
        if self.watched_folders.len() > MAX_WATCHED_FOLDERS {
            return Err(AppError::InvalidSetting("at most three extra folders"));
        }
        let r = &self.retention;
        if [r.image, r.text, r.link, r.video]
            .into_iter()
            .flatten()
            .any(|days| ![1, 3, 7, 15, 30].contains(&days))
        {
            return Err(AppError::InvalidSetting(
                "retention is 1, 3, 7, 15 or 30 days",
            ));
        }
        if r.trash.is_some_and(|days| ![7, 30].contains(&days)) {
            return Err(AppError::InvalidSetting("Trash empties after 7 or 30 days"));
        }
        if self.sounds.volume > 100 {
            return Err(AppError::InvalidSetting("volume is 0 to 100"));
        }
        crate::hotkeys::validate(&self.shortcuts)?;
        if !self.library.widths_ok() {
            return Err(AppError::InvalidSetting(
                "panel and column widths are 60 to 1600 px",
            ));
        }
        Ok(())
    }
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
    let next: Settings = serde_json::from_value(merged)?;
    next.validate()?;
    Ok(next)
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
    fn capture_defaults_are_privacy_first() {
        let s = Settings::default();
        assert!(
            !s.onboarded,
            "nothing is captured before the first-run screen"
        );
        assert!(s.sources.clipboard_text && s.sources.screenshots);
        assert!(!s.sources.screen_recordings);
        assert!(!s.link_previews);
        assert!(!s.launch_at_login && !s.edge_open && !s.search_without_accents);
        assert!(s.ocr && s.appearance.material && s.appearance.animations);
        assert_eq!(s.retention.text, Some(30));
        assert_eq!(s.retention.trash, Some(30));
        #[cfg(windows)]
        let managers = [
            "KeePass.exe",
            "KeePassXC.exe",
            "1Password.exe",
            "Bitwarden.exe",
        ];
        #[cfg(not(windows))]
        let managers = ["KeePassXC", "1Password", "Bitwarden"];
        for app in managers {
            assert!(s.excluded_apps.iter().any(|a| a == app), "{app}");
        }
    }

    #[test]
    fn out_of_range_values_are_rejected() {
        let s = Settings::default();
        assert!(apply_patch(&s, json!({ "retention": { "text": 10 } })).is_err());
        assert!(apply_patch(&s, json!({ "retention": { "trash": 15 } })).is_err());
        assert!(apply_patch(&s, json!({ "watchedFolders": ["a", "b", "c", "d"] })).is_err());
        let ok = apply_patch(&s, json!({ "retention": { "text": null, "trash": 7 } })).unwrap();
        assert_eq!((ok.retention.text, ok.retention.trash), (None, Some(7)));
    }

    #[test]
    fn shelf_settings_default_to_medium_cards_at_the_top_with_quiet_sounds() {
        let s = Settings::default();
        assert_eq!(s.card_size, CardSize::M);
        assert_eq!(s.shelf_edge, ShelfEdge::Top);
        assert_eq!(
            s.sounds,
            Sounds {
                enabled: true,
                volume: 30
            }
        );
        let next = apply_patch(
            &s,
            json!({ "cardSize": "l", "shelfEdge": "bottom", "sounds": { "volume": 0 } }),
        )
        .unwrap();
        assert_eq!(
            (
                next.card_size,
                next.shelf_edge,
                next.sounds.enabled,
                next.sounds.volume
            ),
            (CardSize::L, ShelfEdge::Bottom, true, 0)
        );
        assert!(apply_patch(&s, json!({ "sounds": { "volume": 101 } })).is_err());
        assert!(apply_patch(&s, json!({ "cardSize": "xl" })).is_err());
    }

    #[test]
    fn library_layout_keeps_its_widths_in_range() {
        let s = Settings::default();
        assert_eq!(s.library.view, LibraryView::Grid);
        let next = apply_patch(
            &s,
            json!({ "library": { "view": "list", "sort": "title", "descending": false, "columns": { "tags": 320 } } }),
        )
        .unwrap();
        assert_eq!(
            (
                next.library.view,
                next.library.sort,
                next.library.columns.tags
            ),
            (LibraryView::List, SortKey::Title, 320)
        );
        assert_eq!(next.library.columns.title, 360, "other widths stay");
        assert!(apply_patch(&s, json!({ "library": { "sidebarWidth": 5 } })).is_err());
    }

    #[test]
    fn pause_expires() {
        let s = Settings {
            paused_until: Some(1_000),
            ..Settings::default()
        };
        assert!(s.is_paused(999));
        assert!(!s.is_paused(1_000));
        let forever = Settings {
            paused_until: Some(PAUSED_UNTIL_RESUMED),
            ..Settings::default()
        };
        assert!(forever.is_paused(i64::MAX - 1));
        assert!(!Settings::default().is_paused(0));
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
        // Unknown fields are ignored, and so are retired ones such as the update switch (D-076).
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('app', ?1)",
            [r#"{"language":"tr","futureField":true,"updates":{"autoCheck":false}}"#],
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
        let s = apply_patch(&s, json!({ "language": "ar" })).unwrap();
        assert_eq!(s.language, Language::Ar);
        // Regional languages are stored as their tags.
        let s = apply_patch(&s, json!({ "language": "pt-BR" })).unwrap();
        assert_eq!(s.language, Language::PtBr);
        assert_eq!(
            serde_json::to_value(Language::ZhCn).unwrap(),
            json!("zh-CN")
        );
    }

    #[test]
    fn invalid_patch_is_rejected() {
        assert!(apply_patch(&Settings::default(), json!({ "language": "nl" })).is_err());
        assert!(apply_patch(&Settings::default(), json!({ "language": "pt" })).is_err());
        assert!(apply_patch(&Settings::default(), json!({ "theme": 3 })).is_err());
    }
}
