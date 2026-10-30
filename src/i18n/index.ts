// UI strings come from /locales, shared with the Rust side (D-027).
import en from '../../locales/en.json';
import tr from '../../locales/tr.json';
import { detectPlatform, type Platform } from '../shortcuts';

export type Lang = 'en' | 'tr';
export type LanguageSetting = 'system' | Lang;
export type MessageKey = keyof typeof en;
type Params = Record<string, string | number>;

const dictionaries: Record<Lang, Record<string, string>> = { en, tr };
let current: Lang = 'en';
const platform = detectPlatform();

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

/**
 * A string as this platform needs it: on macOS a `key.mac` variant replaces texts that name
 * Windows, its keys or "this PC".
 */
export function lookup(dictionary: Record<string, string>, key: string, on: Platform = platform): string | undefined {
  return (on === 'mac' ? dictionary[`${key}.mac`] : undefined) ?? dictionary[key];
}

/** Looks up a string with `{name}` placeholders, falling back to English, then to the key. */
export function t(key: MessageKey, params?: Params): string {
  const template = lookup(dictionaries[current], key) ?? lookup(en, key) ?? key;
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    params && name in params ? String(params[name]) : match,
  );
}
