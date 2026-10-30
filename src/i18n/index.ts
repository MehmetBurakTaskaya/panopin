// UI strings come from /locales, shared with the Rust side (D-027). English is the source; every
// other file names its language in "@language" and says in "@status" whether a native speaker has
// checked it ("needs review" until then; CONTRIBUTING.md).
import en from '../../locales/en.json';
import { detectPlatform, type Platform } from '../platform';

/** The languages Panopin speaks, by their own names, in the order the language lists show them. */
export const LANGUAGES = [
  { code: 'en', name: 'English' },
  { code: 'de', name: 'Deutsch' },
  { code: 'es', name: 'Español' },
  { code: 'fr', name: 'Français' },
  { code: 'pt-BR', name: 'Português (Brasil)' },
  { code: 'tr', name: 'Türkçe' },
  { code: 'ru', name: 'Русский' },
  { code: 'ja', name: '日本語' },
  { code: 'ko', name: '한국어' },
  { code: 'zh-CN', name: '简体中文' },
] as const;

export type Lang = (typeof LANGUAGES)[number]['code'];
export type LanguageSetting = 'system' | Lang;
export type MessageKey = keyof typeof en;
type Params = Record<string, string | number>;

// Every locale file comes with the window's code, so t() stays synchronous.
const files = import.meta.glob<Record<string, string>>('../../locales/*.json', { eager: true, import: 'default' });
const dictionaries: Record<string, Record<string, string>> = Object.fromEntries(
  Object.entries(files).map(([path, dictionary]) => [path.slice(path.lastIndexOf('/') + 1, -'.json'.length), dictionary]),
);
let current: Lang = 'en';
const platform = detectPlatform();

// The first part of a language tag, for the languages that need no more than that.
const BY_LANGUAGE: Record<string, Lang> = { en: 'en', de: 'de', es: 'es', fr: 'fr', tr: 'tr', ru: 'ru', ja: 'ja', ko: 'ko' };

/**
 * The language Panopin speaks for a BCP 47 tag, if any. Every Portuguese gets Brazilian
 * Portuguese; Chinese only in its simplified script.
 */
export function languageOf(tag: string): Lang | undefined {
  const [language = '', ...rest] = tag.toLowerCase().replace(/_/g, '-').split('-');
  if (language === 'pt') return 'pt-BR';
  if (language === 'zh') return rest.some((part) => ['hant', 'tw', 'hk', 'mo'].includes(part)) ? undefined : 'zh-CN';
  return BY_LANGUAGE[language];
}

/** Resolves the language setting; `system` takes the first of the system's languages Panopin speaks, else English. */
export function resolveLanguage(setting: LanguageSetting, preferred: readonly string[] = navigator.languages): Lang {
  if (setting !== 'system') return setting;
  for (const tag of preferred) {
    const lang = languageOf(tag);
    if (lang) return lang;
  }
  return 'en';
}

export function setLanguage(lang: Lang): void {
  current = lang;
  // `lang` drives casing (text-transform turns i into İ in Turkish) and the glyphs of Han characters.
  document.documentElement.lang = lang;
}

export function getLanguage(): Lang {
  return current;
}

/** Whether a language's texts still wait for a native speaker's review. */
export function needsReview(lang: Lang = current): boolean {
  return dictionaries[lang]?.['@status'] === 'needs review';
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
  const template = lookup(dictionaries[current] ?? en, key) ?? lookup(en, key) ?? key;
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    params && name in params ? String(params[name]) : match,
  );
}
