import { afterEach, describe, expect, it } from 'vitest';
import en from '../../locales/en.json';
import tr from '../../locales/tr.json';
import { LANGUAGES, formatLocale, getLanguage, inlineArrow, isolateShortcuts, isRtl, languageOf, lookup, needsReview, resolveLanguage, setLanguage, t } from './index';

const files = import.meta.glob<Record<string, string>>('../../locales/*.json', { eager: true, import: 'default' });
const dictionaries = Object.entries(files).map(
  ([path, dictionary]) => [path.slice(path.lastIndexOf('/') + 1, -'.json'.length), dictionary] as const,
);

describe('i18n', () => {
  afterEach(() => setLanguage('en'));

  it('has a file with the same keys as English for every language, and no other file', () => {
    expect(dictionaries.map(([code]) => code).sort()).toEqual(LANGUAGES.map(({ code }) => code).sort());
    for (const [, dictionary] of dictionaries) {
      expect(Object.keys(dictionary).sort()).toEqual(Object.keys(en).sort());
    }
  });

  it('maps language tags to the languages Panopin speaks', () => {
    expect(languageOf('tr-TR')).toBe('tr');
    expect(languageOf('de_AT')).toBe('de');
    expect(languageOf('pt-PT')).toBe('pt-BR');
    expect(languageOf('it-CH')).toBe('it');
    expect(languageOf('ar-EG')).toBe('ar');
    expect(languageOf('zh-Hans-CN')).toBe('zh-CN');
    expect(languageOf('zh-CN')).toBe('zh-CN');
    expect(languageOf('zh-TW')).toBeUndefined();
    expect(languageOf('zh-Hant-HK')).toBeUndefined();
    expect(languageOf('nl-NL')).toBeUndefined();
  });

  it('resolves the system language from the preferred languages, else English', () => {
    expect(resolveLanguage('system', ['tr-TR'])).toBe('tr');
    expect(resolveLanguage('system', ['nl-NL', 'ja-JP'])).toBe('ja');
    expect(resolveLanguage('system', ['zh-TW'])).toBe('en');
    expect(resolveLanguage('system', [])).toBe('en');
    expect(resolveLanguage('ko', ['en-US'])).toBe('ko');
  });

  it('translates and sets the document language', () => {
    setLanguage('tr');
    expect(getLanguage()).toBe('tr');
    expect(document.documentElement.lang).toBe('tr');
    expect(t('tray.quit')).toBe('Çıkış');
    setLanguage('zh-CN');
    expect(document.documentElement.lang).toBe('zh-CN');
  });

  it('mirrors the window for right-to-left languages', () => {
    setLanguage('ar');
    expect(isRtl()).toBe(true);
    expect(document.documentElement.dir).toBe('rtl');
    // The arrows keep pointing where they move: right goes back towards the start.
    expect(inlineArrow('ArrowRight')).toBe('ArrowLeft');
    expect(inlineArrow('ArrowLeft')).toBe('ArrowRight');
    expect(inlineArrow('ArrowUp')).toBe('ArrowUp');
    // Mac shortcuts keep their order inside right-to-left text; Windows ones need no help.
    expect(isolateShortcuts('لصق ⌘Enter · ⇧⌘Z · Ctrl+V')).toBe('لصق \u2066⌘Enter\u2069 · \u2066⇧⌘Z\u2069 · Ctrl+V');
    // Numbers keep Western digits, like the Arabic texts.
    expect(new Intl.NumberFormat(formatLocale()).format(1234)).toBe('1,234');
    setLanguage('it');
    expect(document.documentElement.dir).toBe('ltr');
    expect(inlineArrow('ArrowRight')).toBe('ArrowRight');
  });

  it('knows which translations wait for review', () => {
    expect(needsReview('en')).toBe(false);
    for (const [code, dictionary] of dictionaries) {
      expect(['source', 'needs review', 'reviewed']).toContain(dictionary['@status']);
      if (code !== 'en') expect(needsReview(code as (typeof LANGUAGES)[number]['code'])).toBe(dictionary['@status'] === 'needs review');
    }
  });

  it('fills placeholders and leaves unknown ones visible', () => {
    expect(t('settings.saveFailed', { error: 'disk full' })).toBe('Could not save the setting: disk full');
    expect(t('settings.saveFailed')).toBe('Could not save the setting: {error}');
  });

  it('uses the macOS variant of a string on a Mac only', () => {
    expect(lookup(en, 'settings.launchAtLogin', 'mac')).toBe('Open at login');
    expect(lookup(en, 'settings.launchAtLogin', 'windows')).toBe('Start with Windows');
    expect(lookup(tr, 'markup.undo', 'mac')).toBe('Geri al (⌘Z)');
    // Strings without a variant are the same everywhere.
    expect(lookup(en, 'tray.quit', 'mac')).toBe('Quit');
    expect(t('settings.launchAtLogin')).toBe('Start with Windows');
  });

  it('gives every macOS variant a general string to stand for', () => {
    const orphans = Object.keys(en).filter((key) => key.endsWith('.mac') && !(key.slice(0, -4) in en));
    expect(orphans).toEqual([]);
  });

  it('names no Windows keys or places in the macOS strings', () => {
    const windowsWords = /\b(Windows|Ctrl|Win|PC)\b/;
    for (const [, dictionary] of dictionaries) {
      const macStrings = Object.entries(dictionary).filter(([key]) => key.endsWith('.mac'));
      expect(macStrings.filter(([, text]) => windowsWords.test(text))).toEqual([]);
    }
  });
});
