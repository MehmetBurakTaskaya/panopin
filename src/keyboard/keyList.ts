// The keys of the shelf and the Library, listed for people: Settings → Shortcuts shows both, ? on
// the shelf shows its own. Keys are written as accelerators ("Shift+S") or as the key itself
// ("←", "1–9"), so `formatShortcut` names them for the platform; the tests check every shelf key
// against the key model.
import type { MessageKey } from '../i18n';
import { TRASH_SHORTCUT } from '../shortcuts';

export interface KeyEntry {
  label: MessageKey;
  /** Each a way to do it. */
  keys: string[];
}

export interface KeyGroup {
  title: MessageKey;
  entries: KeyEntry[];
}

export const SHELF_KEYS: KeyGroup[] = [
  {
    title: 'keys.group.move',
    entries: [
      { label: 'keys.cards', keys: ['←', '→'] },
      { label: 'keys.rows', keys: ['↑', '↓', 'Tab'] },
      { label: 'keys.pick', keys: ['1–9'] },
      { label: 'keys.ends', keys: ['CommandOrControl+←', 'CommandOrControl+→'] },
      { label: 'keys.page', keys: ['Alt+←', 'Alt+→'] },
      { label: 'keys.select', keys: ['Shift+←', 'Shift+→'] },
      { label: 'keys.selectAll', keys: ['CommandOrControl+A'] },
    ],
  },
  {
    title: 'keys.group.copy',
    entries: [
      { label: 'keys.copyAndReturn', keys: ['Enter'] },
      { label: 'shelf.menu.paste', keys: ['CommandOrControl+Enter'] },
      { label: 'shelf.menu.pasteInTurn', keys: ['Shift+Enter'] },
      { label: 'keys.copyStay', keys: ['CommandOrControl+C'] },
      { label: 'shelf.menu.copyText', keys: ['CommandOrControl+Shift+C'] },
      { label: 'shelf.menu.openLink', keys: ['CommandOrControl+O'] },
    ],
  },
  {
    title: 'keys.group.organise',
    entries: [
      { label: 'shelf.menu.preview', keys: ['Space'] },
      { label: 'keys.pin', keys: ['P'] },
      { label: 'library.action.name', keys: ['Alt+P'] },
      { label: 'library.action.folder', keys: ['Shift+P'] },
      { label: 'library.action.tag', keys: ['CommandOrControl+Shift+T'] },
      { label: 'library.action.export', keys: ['CommandOrControl+Shift+E'] },
      { label: 'shelf.menu.edit', keys: ['E'] },
      { label: 'shelf.menu.markup', keys: ['M'] },
      { label: 'shelf.menu.crop', keys: ['C'] },
      { label: 'shelf.menu.takeDown', keys: ['S'] },
      { label: 'shelf.menu.trash', keys: [TRASH_SHORTCUT] },
      { label: 'shelf.menu.undo', keys: ['Backspace'] },
    ],
  },
  {
    title: 'keys.group.view',
    entries: [
      { label: 'shelf.search.label', keys: ['Shift+S'] },
      { label: 'palette.rowImages', keys: ['Shift+I'] },
      { label: 'palette.rowTexts', keys: ['Shift+T'] },
      { label: 'palette.rowLinks', keys: ['Shift+L'] },
      { label: 'library.filter.pinned', keys: ['F'] },
      { label: 'keys.palette', keys: ['CommandOrControl+K'] },
      { label: 'keys.menu', keys: ['Shift+F10'] },
      { label: 'tray.library', keys: ['L'] },
      { label: 'keys.list', keys: ['?', 'F1'] },
      { label: 'keys.close', keys: ['Esc'] },
    ],
  },
];

export const LIBRARY_KEYS: KeyGroup[] = [
  {
    title: 'keys.group.move',
    entries: [
      { label: 'keys.items', keys: ['←', '→', '↑', '↓'] },
      { label: 'keys.ends', keys: ['Home', 'End'] },
      { label: 'keys.page', keys: ['PgUp', 'PgDn'] },
      { label: 'keys.select', keys: ['Shift+↑', 'Shift+↓'] },
      { label: 'keys.selectAll', keys: ['CommandOrControl+A'] },
      { label: 'keys.panes', keys: ['F6', 'Shift+F6'] },
    ],
  },
  {
    title: 'keys.group.copy',
    entries: [
      { label: 'shelf.menu.copy', keys: ['Enter', 'CommandOrControl+C'] },
      { label: 'shelf.menu.pasteInTurn', keys: ['Shift+Enter'] },
      { label: 'shelf.menu.copyText', keys: ['CommandOrControl+Shift+C'] },
      { label: 'shelf.menu.openLink', keys: ['CommandOrControl+O'] },
    ],
  },
  {
    title: 'keys.group.organise',
    entries: [
      { label: 'shelf.menu.preview', keys: ['Space'] },
      { label: 'keys.pin', keys: ['P'] },
      { label: 'library.action.name', keys: ['Alt+P'] },
      { label: 'library.action.folder', keys: ['Shift+P'] },
      { label: 'library.action.tag', keys: ['CommandOrControl+Shift+T'] },
      { label: 'library.action.export', keys: ['CommandOrControl+Shift+E'] },
      { label: 'shelf.menu.edit', keys: ['E'] },
      { label: 'shelf.menu.markup', keys: ['M'] },
      { label: 'shelf.menu.crop', keys: ['C'] },
      { label: 'shelf.menu.trash', keys: [TRASH_SHORTCUT] },
      { label: 'library.action.restore', keys: ['R'] },
      { label: 'keys.undo', keys: ['CommandOrControl+Z', 'Backspace'] },
      { label: 'library.newNote', keys: ['CommandOrControl+N'] },
      { label: 'library.folder.new', keys: ['CommandOrControl+Shift+N'] },
    ],
  },
  {
    title: 'keys.group.view',
    entries: [
      { label: 'palette.searchLibrary', keys: ['CommandOrControl+F'] },
      { label: 'palette.kindAll', keys: ['CommandOrControl+0'] },
      { label: 'palette.kindImages', keys: ['CommandOrControl+1'] },
      { label: 'palette.kindTexts', keys: ['CommandOrControl+2'] },
      { label: 'palette.kindLinks', keys: ['CommandOrControl+3'] },
      { label: 'library.filter.pinned', keys: ['F'] },
      { label: 'keys.palette', keys: ['CommandOrControl+K'] },
      { label: 'keys.closePreview', keys: ['Esc'] },
    ],
  },
];
