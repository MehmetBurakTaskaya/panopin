import { describe, expect, it } from 'vitest';
import { detectPlatform, formatShortcut } from './shortcuts';

describe('formatShortcut', () => {
  it('uses Windows key names', () => {
    expect(formatShortcut('Super+Alt+V', 'windows')).toBe('Win+Alt+V');
    expect(formatShortcut('Control+Shift+t', 'windows')).toBe('Ctrl+Shift+T');
  });

  it('uses macOS symbols', () => {
    expect(formatShortcut('Super+Alt+V', 'mac')).toBe('⌘⌥V');
  });

  it('detects the platform from the user agent', () => {
    expect(detectPlatform('Mozilla/5.0 (Windows NT 10.0; Win64; x64)')).toBe('windows');
    expect(detectPlatform('Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0)')).toBe('mac');
  });
});
