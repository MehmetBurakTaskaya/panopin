//! UI strings for the Rust side (tray, window titles). The same `locales/*.json` files feed the
//! webview, so there is a single source of truth (D-027).

use std::{collections::HashMap, sync::OnceLock};

use crate::settings::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    En,
    Tr,
    De,
    Es,
    Fr,
    PtBr,
    Ru,
    Ja,
    Ko,
    ZhCn,
}

impl Lang {
    const ALL: [Lang; 10] = [
        Lang::En,
        Lang::Tr,
        Lang::De,
        Lang::Es,
        Lang::Fr,
        Lang::PtBr,
        Lang::Ru,
        Lang::Ja,
        Lang::Ko,
        Lang::ZhCn,
    ];

    fn file(self) -> &'static str {
        match self {
            Lang::En => include_str!("../../locales/en.json"),
            Lang::Tr => include_str!("../../locales/tr.json"),
            Lang::De => include_str!("../../locales/de.json"),
            Lang::Es => include_str!("../../locales/es.json"),
            Lang::Fr => include_str!("../../locales/fr.json"),
            Lang::PtBr => include_str!("../../locales/pt-BR.json"),
            Lang::Ru => include_str!("../../locales/ru.json"),
            Lang::Ja => include_str!("../../locales/ja.json"),
            Lang::Ko => include_str!("../../locales/ko.json"),
            Lang::ZhCn => include_str!("../../locales/zh-CN.json"),
        }
    }
}

type Dict = HashMap<String, String>;

fn dicts() -> &'static HashMap<Lang, Dict> {
    static DICTS: OnceLock<HashMap<Lang, Dict>> = OnceLock::new();
    DICTS.get_or_init(|| {
        Lang::ALL
            .iter()
            .map(|&lang| (lang, parse(lang.file())))
            .collect()
    })
}

// CI validates the locale files; a broken file degrades to showing keys instead of crashing.
fn parse(json: &str) -> Dict {
    serde_json::from_str(json).unwrap_or_default()
}

/// Resolves the language setting; `System` takes the first of the system's languages Panopin
/// speaks, else English.
pub fn resolve(setting: Language) -> Lang {
    match setting {
        Language::System => from_locales(sys_locale::get_locales()),
        Language::En => Lang::En,
        Language::Tr => Lang::Tr,
        Language::De => Lang::De,
        Language::Es => Lang::Es,
        Language::Fr => Lang::Fr,
        Language::PtBr => Lang::PtBr,
        Language::Ru => Lang::Ru,
        Language::Ja => Lang::Ja,
        Language::Ko => Lang::Ko,
        Language::ZhCn => Lang::ZhCn,
    }
}

fn from_locales(tags: impl IntoIterator<Item = String>) -> Lang {
    tags.into_iter()
        .find_map(|tag| from_tag(&tag))
        .unwrap_or(Lang::En)
}

/// The language for a BCP 47 tag, if Panopin speaks it: every Portuguese gets Brazilian
/// Portuguese, Chinese only in its simplified script (as in `src/i18n`).
fn from_tag(tag: &str) -> Option<Lang> {
    let tag = tag.to_ascii_lowercase().replace('_', "-");
    let mut parts = tag.split('-');
    let language = parts.next()?;
    let traditional = parts.any(|part| matches!(part, "hant" | "tw" | "hk" | "mo"));
    match language {
        "en" => Some(Lang::En),
        "tr" => Some(Lang::Tr),
        "de" => Some(Lang::De),
        "es" => Some(Lang::Es),
        "fr" => Some(Lang::Fr),
        "pt" => Some(Lang::PtBr),
        "ru" => Some(Lang::Ru),
        "ja" => Some(Lang::Ja),
        "ko" => Some(Lang::Ko),
        "zh" if !traditional => Some(Lang::ZhCn),
        _ => None,
    }
}

/// Looks up a string, falling back to English and then to the key itself.
pub fn t(lang: Lang, key: &str) -> String {
    let dicts = dicts();
    dicts
        .get(&lang)
        .and_then(|dict| dict.get(key))
        .or_else(|| dicts.get(&Lang::En).and_then(|en| en.get(key)))
        .cloned()
        .unwrap_or_else(|| key.to_owned())
}

/// Like `t`, filling `{name}` placeholders.
pub fn t_with(lang: Lang, key: &str, args: &[(&str, u32)]) -> String {
    args.iter().fold(t(lang, key), |text, (name, value)| {
        text.replace(&format!("{{{name}}}"), &value.to_string())
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn locale_files_parse_and_share_keys() {
        let dicts = dicts();
        let en: BTreeSet<_> = dicts[&Lang::En].keys().collect();
        assert!(!en.is_empty());
        for lang in Lang::ALL {
            let keys: BTreeSet<_> = dicts[&lang].keys().collect();
            assert_eq!(keys, en, "{lang:?}");
        }
    }

    #[test]
    fn system_locales_map_to_supported_languages() {
        let pick = |tags: &[&str]| from_locales(tags.iter().map(|tag| tag.to_string()));
        assert_eq!(pick(&["tr-TR"]), Lang::Tr);
        assert_eq!(pick(&["TR"]), Lang::Tr);
        assert_eq!(pick(&["de_DE"]), Lang::De);
        assert_eq!(pick(&["pt-PT"]), Lang::PtBr);
        assert_eq!(pick(&["zh-Hans-CN"]), Lang::ZhCn);
        assert_eq!(pick(&["zh-TW"]), Lang::En);
        assert_eq!(pick(&["nl-NL", "ja-JP"]), Lang::Ja);
        assert_eq!(pick(&["en-GB"]), Lang::En);
        assert_eq!(pick(&[]), Lang::En);
    }

    #[test]
    fn lookup_falls_back_to_the_key() {
        assert_eq!(t(Lang::Tr, "tray.quit"), "Çıkış");
        assert_eq!(t(Lang::En, "tray.quit"), "Quit");
        assert_eq!(t(Lang::Tr, "no.such.key"), "no.such.key");
    }
}
