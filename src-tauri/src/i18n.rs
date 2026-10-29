//! UI strings for the Rust side (tray, window titles). The same `locales/*.json` files feed the
//! webview, so there is a single source of truth (D-027).

use std::{collections::HashMap, sync::OnceLock};

use crate::settings::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Tr,
}

type Dict = HashMap<String, String>;

fn dicts() -> &'static (Dict, Dict) {
    static DICTS: OnceLock<(Dict, Dict)> = OnceLock::new();
    DICTS.get_or_init(|| {
        (
            parse(include_str!("../../locales/en.json")),
            parse(include_str!("../../locales/tr.json")),
        )
    })
}

// CI validates the locale files; a broken file degrades to showing keys instead of crashing.
fn parse(json: &str) -> Dict {
    serde_json::from_str(json).unwrap_or_default()
}

/// Resolves the language setting; `System` follows the OS locale.
pub fn resolve(setting: Language) -> Lang {
    match setting {
        Language::En => Lang::En,
        Language::Tr => Lang::Tr,
        Language::System => from_locale(sys_locale::get_locale().as_deref()),
    }
}

fn from_locale(locale: Option<&str>) -> Lang {
    match locale {
        Some(tag) if tag.to_ascii_lowercase().starts_with("tr") => Lang::Tr,
        _ => Lang::En,
    }
}

/// Looks up a string, falling back to English and then to the key itself.
pub fn t(lang: Lang, key: &str) -> String {
    let (en, tr) = dicts();
    let primary = match lang {
        Lang::En => en,
        Lang::Tr => tr,
    };
    primary
        .get(key)
        .or_else(|| en.get(key))
        .cloned()
        .unwrap_or_else(|| key.to_owned())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn locale_files_parse_and_share_keys() {
        let (en, tr) = dicts();
        assert!(!en.is_empty());
        let en_keys: BTreeSet<_> = en.keys().collect();
        let tr_keys: BTreeSet<_> = tr.keys().collect();
        assert_eq!(en_keys, tr_keys);
    }

    #[test]
    fn system_locale_maps_to_supported_languages() {
        assert_eq!(from_locale(Some("tr-TR")), Lang::Tr);
        assert_eq!(from_locale(Some("TR")), Lang::Tr);
        assert_eq!(from_locale(Some("en-GB")), Lang::En);
        assert_eq!(from_locale(Some("de-DE")), Lang::En);
        assert_eq!(from_locale(None), Lang::En);
    }

    #[test]
    fn lookup_falls_back_to_the_key() {
        assert_eq!(t(Lang::Tr, "tray.quit"), "Çıkış");
        assert_eq!(t(Lang::En, "tray.quit"), "Quit");
        assert_eq!(t(Lang::Tr, "no.such.key"), "no.such.key");
    }
}
