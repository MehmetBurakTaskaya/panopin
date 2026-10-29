// UI strings come from /locales, shared with the Rust side (D-027).
import en from '../../locales/en.json';
import tr from '../../locales/tr.json';

export type Lang = 'en' | 'tr';
export type LanguageSetting = 'system' | Lang;
export type MessageKey = keyof typeof en;
type Params = Record<string, string | number>;

const dictionaries: Record<Lang, Record<string, string>> = { en, tr };
let current: Lang = 'en';

/** Resolves the language setting; `system` follows the OS/browser locale. */
export function resolveLanguage(setting: LanguageSetting, locale: string = navigator.language): Lang {
  if (setting !== 'system') return setting;
  return locale.toLowerCase().startsWith('tr') ? 'tr' : 'en';
}

export function setLanguage(lang: Lang): void {
  current = lang;
  // `lang` drives Turkish-aware casing (text-transform turns i into İ).
  document.documentElement.lang = lang;
}

export function getLanguage(): Lang {
  return current;
}

/** Looks up a string with `{name}` placeholders, falling back to English, then to the key. */
export function t(key: MessageKey, params?: Params): string {
  const template = dictionaries[current][key] ?? en[key] ?? key;
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    params && name in params ? String(params[name]) : match,
  );
}
