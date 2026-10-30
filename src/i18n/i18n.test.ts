import { afterEach, describe, expect, it } from 'vitest';
import en from '../../locales/en.json';
import tr from '../../locales/tr.json';
import { getLanguage, lookup, resolveLanguage, setLanguage, t } from './index';

describe('i18n', () => {
  afterEach(() => setLanguage('en'));

  it('keeps English and Turkish keys in sync', () => {
    expect(Object.keys(tr).sort()).toEqual(Object.keys(en).sort());
  });

  it('resolves the system language from the locale', () => {
    expect(resolveLanguage('system', 'tr-TR')).toBe('tr');
    expect(resolveLanguage('system', 'en-US')).toBe('en');
    expect(resolveLanguage('system', 'de-DE')).toBe('en');
    expect(resolveLanguage('tr', 'en-US')).toBe('tr');
  });

  it('translates and sets the document language', () => {
    setLanguage('tr');
    expect(getLanguage()).toBe('tr');
    expect(document.documentElement.lang).toBe('tr');
    expect(t('tray.quit')).toBe('Çıkış');
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
    for (const dictionary of [en, tr] as Record<string, string>[]) {
      const macStrings = Object.entries(dictionary).filter(([key]) => key.endsWith('.mac'));
      expect(macStrings.filter(([, text]) => windowsWords.test(text))).toEqual([]);
    }
  });
});
