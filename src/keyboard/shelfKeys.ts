// Maps a key press on the shelf to a command. Pure, so the whole key table is covered by tests.
import type { Action, Kind } from './shelfModel';

export type Command =
  | { type: 'model'; action: Action }
  | { type: 'copy' }
  | { type: 'copyAndReturn' }
  | { type: 'paste' }
  | { type: 'pin' }
  | { type: 'takeDown' }
  | { type: 'undo' }
  | { type: 'trash' }
  | { type: 'pinnedOnly' }
  | { type: 'toggleRow'; kind: Kind }
  | { type: 'menu' }
  | { type: 'preview' }
  | { type: 'editText' }
  | { type: 'markup' }
  | { type: 'crop' }
  | { type: 'name' }
  | { type: 'folder' }
  | { type: 'tag' }
  | { type: 'export' }
  | { type: 'library' }
  | { type: 'search' }
  | { type: 'palette' }
  | { type: 'openLink' }
  | { type: 'copyText' }
  | { type: 'escape' };

export interface KeyInput {
  key: string;
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

const ROW_LETTERS: Record<string, Kind> = { i: 'image', t: 'text', l: 'link' };

/** Letters match the typed key with Turkish I-folding (İ, I, ı, i are one key) and fall back to
 * the physical key on layouts that type non-Latin letters (D-019). */
export function letterOf(event: KeyInput): string | null {
  const { key } = event;
  if (key === 'İ' || key === 'I' || key === 'ı' || key === 'i') return 'i';
  if (/^[a-z]$/i.test(key)) return key.toLowerCase();
  const physical = /^Key([A-Z])$/.exec(event.code);
  return physical ? physical[1]!.toLowerCase() : null;
}

/** 1–9 from the typed key, or from the digit row on layouts where digits need Shift. */
function digitOf(event: KeyInput): number | null {
  if (/^[1-9]$/.test(event.key)) return Number(event.key);
  const physical = /^Digit([1-9])$/.exec(event.code);
  return physical ? Number(physical[1]) : null;
}

const model = (action: Action): Command => ({ type: 'model', action });

export function commandFor(event: KeyInput): Command | null {
  const ctrl = event.ctrlKey || event.metaKey;
  const { altKey: alt, shiftKey: shift } = event;

  switch (event.key) {
    case 'Escape':
      return { type: 'escape' };
    case 'ArrowLeft':
    case 'ArrowRight': {
      const dx = event.key === 'ArrowLeft' ? -1 : 1;
      if (ctrl) return model({ type: 'edge', end: dx < 0 ? 'first' : 'last' });
      if (alt) return model({ type: 'page', dir: dx });
      return model({ type: 'move', dx, extend: shift });
    }
    case 'ArrowUp':
    case 'ArrowDown':
      return ctrl || alt ? null : model({ type: 'row', dy: event.key === 'ArrowUp' ? -1 : 1 });
    case 'Tab':
      return ctrl || alt ? null : model({ type: 'tab', back: shift });
    case 'Enter':
      if (alt) return null;
      return ctrl ? { type: 'paste' } : { type: 'copyAndReturn' };
    case 'Backspace':
      // ⌘⌫ moves to Trash, as in Finder: a Mac's Delete key needs fn.
      if (event.metaKey && !event.ctrlKey && !alt) return { type: 'trash' };
      return ctrl || alt ? null : { type: 'undo' };
    case 'Delete':
      return ctrl || alt ? null : { type: 'trash' };
    case 'ContextMenu':
      return { type: 'menu' };
    case 'F10':
      return shift && !ctrl && !alt ? { type: 'menu' } : null;
    case ' ':
      return ctrl || alt || shift ? null : { type: 'preview' };
  }

  const digit = digitOf(event);
  if (digit !== null && !ctrl && !alt) return model({ type: 'pick', n: digit });

  const letter = letterOf(event);
  if (letter === null) return null;
  if (alt) return !ctrl && !shift && letter === 'p' ? { type: 'name' } : null;
  if (ctrl) {
    if (shift) {
      if (letter === 't') return { type: 'tag' };
      if (letter === 'e') return { type: 'export' };
      if (letter === 'c') return { type: 'copyText' };
      return null;
    }
    if (letter === 'a') return model({ type: 'selectAll' });
    if (letter === 'c') return { type: 'copy' };
    if (letter === 'k') return { type: 'palette' };
    if (letter === 'o') return { type: 'openLink' };
    return null;
  }
  if (shift) {
    if (letter === 'p') return { type: 'folder' };
    if (letter === 's') return { type: 'search' };
    const kind = ROW_LETTERS[letter];
    return kind ? { type: 'toggleRow', kind } : null;
  }
  const plain: Record<string, Command> = {
    p: { type: 'pin' },
    s: { type: 'takeDown' },
    f: { type: 'pinnedOnly' },
    e: { type: 'editText' },
    m: { type: 'markup' },
    c: { type: 'crop' },
    l: { type: 'library' },
  };
  return plain[letter] ?? null;
}
