// Turns a Tauri accelerator ("Super+Alt+V") into the label users expect on their platform.
export type Platform = 'windows' | 'mac';

export function detectPlatform(userAgent: string = navigator.userAgent): Platform {
  return /Mac OS X|Macintosh/.test(userAgent) ? 'mac' : 'windows';
}

const NAMES: Record<Platform, Record<string, string>> = {
  windows: { super: 'Win', alt: 'Alt', control: 'Ctrl', ctrl: 'Ctrl', shift: 'Shift', commandorcontrol: 'Ctrl' },
  mac: { super: '⌘', alt: '⌥', control: '⌃', ctrl: '⌃', shift: '⇧', commandorcontrol: '⌘' },
};

export function formatShortcut(accelerator: string, platform: Platform = detectPlatform()): string {
  const parts = accelerator.split('+').map((part) => {
    const name = NAMES[platform][part.toLowerCase()];
    // Letters read as capitals; named keys keep their name: Enter, Space, Delete.
    return name ?? (part.length === 1 ? part.toUpperCase() : part.charAt(0).toUpperCase() + part.slice(1));
  });
  return parts.join(platform === 'mac' ? '' : '+');
}

export interface RecordedKey {
  key: string;
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

const MODIFIER_KEYS = new Set(['Control', 'Alt', 'AltGraph', 'Shift', 'Meta', 'OS']);

/**
 * The accelerator a key press makes in the shortcut recorder, or null while only modifiers are
 * held or the combination cannot be a global shortcut: it needs Win, Ctrl or Alt (Shift alone
 * would take capital letters away). Keys are named by their physical position (`code`), so the
 * keyboard layout does not matter.
 */
export function acceleratorFrom(event: RecordedKey): string | null {
  if (MODIFIER_KEYS.has(event.key)) return null;
  if (!event.metaKey && !event.ctrlKey && !event.altKey) return null;
  const key = /^Key([A-Z])$/.exec(event.code)?.[1] ?? /^Digit([0-9])$/.exec(event.code)?.[1] ?? event.code;
  if (!key || key === 'Unidentified') return null;
  const parts = [event.metaKey && 'Super', event.ctrlKey && 'Ctrl', event.altKey && 'Alt', event.shiftKey && 'Shift'];
  return [...parts.filter(Boolean), key].join('+');
}

/** Ctrl+Alt without Win is what AltGr sends, so such a shortcut can block a typed character. */
export function clashesWithAltGr(accelerator: string): boolean {
  const parts = accelerator.split('+');
  return parts.includes('Ctrl') && parts.includes('Alt') && !parts.includes('Super');
}
