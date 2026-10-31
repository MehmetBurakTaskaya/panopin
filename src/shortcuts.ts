// Turns a Tauri accelerator ("Super+Alt+V") into the label users expect on their platform.
import { isRtl, ltr, t, type MessageKey } from './i18n';
import { detectPlatform, type Platform } from './platform';

export { detectPlatform, type Platform };

// Windows names keys in the UI language (German writes Strg, French Maj); Win and Alt read the
// same everywhere. A Mac writes symbols.
const WINDOWS_KEYS: Record<string, MessageKey> = {
  control: 'key.ctrl',
  ctrl: 'key.ctrl',
  commandorcontrol: 'key.ctrl',
  shift: 'key.shift',
  enter: 'key.enter',
  space: 'key.space',
  esc: 'key.esc',
  escape: 'key.esc',
  backspace: 'key.backspace',
  delete: 'key.delete',
};

const NAMES: Record<Platform, Record<string, string>> = {
  windows: { super: 'Win', alt: 'Alt' },
  mac: {
    super: '⌘',
    alt: '⌥',
    control: '⌃',
    ctrl: '⌃',
    shift: '⇧',
    commandorcontrol: '⌘',
    backspace: '⌫',
    delete: '⌦',
  },
};

/** Dragging items out to other apps as text and files; macOS has no such drag yet (D-073). */
export const DRAG_OUT = detectPlatform() !== 'mac';

/** Moving to Trash: Delete, or ⌘⌫ on a Mac, as in Finder (the Delete key needs fn there). */
export const TRASH_SHORTCUT = detectPlatform() === 'mac' ? 'Super+Backspace' : 'Delete';

/** The note with every key: F1, the help key, on Windows; ? on a Mac, where F1 needs fn. Both
 * work everywhere. */
export const KEYS_SHORTCUT = detectPlatform() === 'mac' ? '?' : 'F1';

// macOS writes modifiers in one order: Control, Option, Shift, Command.
const MAC_ORDER = ['⌃', '⌥', '⇧', '⌘'];

export function formatShortcut(accelerator: string, platform: Platform = detectPlatform()): string {
  const parts = accelerator.split('+').map((part) => {
    const key = platform === 'windows' ? WINDOWS_KEYS[part.toLowerCase()] : undefined;
    const name = key ? t(key) : NAMES[platform][part.toLowerCase()];
    // Letters read as capitals; other keys keep their name: Tab, F9.
    return name ?? (part.length === 1 ? part.toUpperCase() : part.charAt(0).toUpperCase() + part.slice(1));
  });
  const rank = (part: string) => (MAC_ORDER.includes(part) ? MAC_ORDER.indexOf(part) : MAC_ORDER.length);
  const label = platform === 'mac' ? parts.sort((a, b) => rank(a) - rank(b)).join('') : parts.join('+');
  // Right to left, a shortcut is kept in its own left-to-right order.
  return isRtl() ? ltr(label) : label;
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

/**
 * Ctrl+Alt without Win is what AltGr sends on Windows, so such a shortcut can block a typed
 * character. Mac keyboards have no AltGr.
 */
export function clashesWithAltGr(accelerator: string, platform: Platform = detectPlatform()): boolean {
  if (platform === 'mac') return false;
  const parts = accelerator.split('+');
  return parts.includes('Ctrl') && parts.includes('Alt') && !parts.includes('Super');
}
