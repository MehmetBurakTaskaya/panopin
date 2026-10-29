import { afterEach, describe, expect, it } from 'vitest';
import en from '../../locales/en.json';
import tr from '../../locales/tr.json';
import { getLanguage, resolveLanguage, setLanguage, t } from './index';

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
});
