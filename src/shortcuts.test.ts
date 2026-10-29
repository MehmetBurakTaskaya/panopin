import { describe, expect, it } from 'vitest';
import { acceleratorFrom, clashesWithAltGr, detectPlatform, formatShortcut, type RecordedKey } from './shortcuts';

describe('formatShortcut', () => {
  it('uses Windows key names', () => {
    expect(formatShortcut('Super+Alt+V', 'windows')).toBe('Win+Alt+V');
    expect(formatShortcut('Control+Shift+t', 'windows')).toBe('Ctrl+Shift+T');
    expect(formatShortcut('Ctrl+Enter', 'windows')).toBe('Ctrl+Enter');
    expect(formatShortcut('Delete', 'windows')).toBe('Delete');
  });

  it('uses macOS symbols', () => {
    expect(formatShortcut('Super+Alt+V', 'mac')).toBe('⌘⌥V');
  });

  it('detects the platform from the user agent', () => {
    expect(detectPlatform('Mozilla/5.0 (Windows NT 10.0; Win64; x64)')).toBe('windows');
    expect(detectPlatform('Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0)')).toBe('mac');
  });
});

const press = (key: string, code: string, mods: Partial<RecordedKey> = {}): RecordedKey => ({
  key,
  code,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
});

describe('shortcut recorder', () => {
  it('names keys by position, so the layout does not matter', () => {
    expect(acceleratorFrom(press('v', 'KeyV', { metaKey: true, altKey: true }))).toBe('Super+Alt+V');
    // Turkish F types "ğ" on the physical W key; Turkish Q types "ı" on I.
    expect(acceleratorFrom(press('ğ', 'KeyW', { metaKey: true, altKey: true }))).toBe('Super+Alt+W');
    expect(acceleratorFrom(press('ı', 'KeyI', { metaKey: true, shiftKey: true }))).toBe('Super+Shift+I');
    expect(acceleratorFrom(press('5', 'Digit5', { ctrlKey: true, shiftKey: true }))).toBe('Ctrl+Shift+5');
    expect(acceleratorFrom(press('F9', 'F9', { altKey: true }))).toBe('Alt+F9');
  });

  it('waits for a real key and wants Win, Ctrl or Alt', () => {
    expect(acceleratorFrom(press('Alt', 'AltLeft', { altKey: true }))).toBeNull();
    expect(acceleratorFrom(press('Meta', 'MetaLeft', { metaKey: true }))).toBeNull();
    expect(acceleratorFrom(press('a', 'KeyA'))).toBeNull();
    expect(acceleratorFrom(press('A', 'KeyA', { shiftKey: true }))).toBeNull();
  });

  it('warns about combinations AltGr can produce', () => {
    expect(clashesWithAltGr('Ctrl+Alt+Q')).toBe(true);
    expect(clashesWithAltGr('Super+Ctrl+Alt+Q')).toBe(false);
    expect(clashesWithAltGr('Super+Alt+V')).toBe(false);
  });
});
