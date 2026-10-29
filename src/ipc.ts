// Typed wrappers around the Rust commands and events. Types mirror src-tauri/src/settings.rs;
// they move to ts-rs generated bindings once the data model grows (D-013).
import { invoke } from '@tauri-apps/api/core';
import type { LanguageSetting, MessageKey } from './i18n';

export type Theme = 'system' | 'light' | 'dark';
export type CardSize = 's' | 'm' | 'l';
export type ShelfEdge = 'top' | 'bottom';

export interface Shortcuts {
  shelf: string | null;
  shelfSearch: string | null;
  snip: string | null;
  library: string | null;
  copyLastImage: string | null;
  pauseCapture: string | null;
}

export interface Sources {
  screenshots: boolean;
  clipboardImages: boolean;
  clipboardText: boolean;
  links: boolean;
  screenRecordings: boolean;
}

/** Days to keep unsaved items per kind and before Trash empties; `null` = forever. */
export interface Retention {
  image: number | null;
  text: number | null;
  link: number | null;
  video: number | null;
  trash: number | null;
}

export interface Settings {
  language: LanguageSetting;
  theme: Theme;
  shortcuts: Shortcuts;
  onboarded: boolean;
  sources: Sources;
  watchedFolders: string[];
  excludedApps: string[];
  retention: Retention;
  linkPreviews: boolean;
  pausedUntil: number | null;
  cardSize: CardSize;
  shelfEdge: ShelfEdge;
  sounds: Sounds;
}

/** UI sounds; `volume` is 0–100. */
export interface Sounds {
  enabled: boolean;
  volume: number;
}

export type SettingsPatch = {
  language?: LanguageSetting;
  theme?: Theme;
  shortcuts?: Partial<Shortcuts>;
  onboarded?: boolean;
  sources?: Partial<Sources>;
  watchedFolders?: string[];
  excludedApps?: string[];
  retention?: Partial<Retention>;
  linkPreviews?: boolean;
  cardSize?: CardSize;
  shelfEdge?: ShelfEdge;
  sounds?: Partial<Sounds>;
};

export interface Counts {
  image: number;
  text: number;
  link: number;
  video: number;
}

export interface ShelfOpenEvent {
  generation: number;
  search: boolean;
  /** Sounds stay silent while a full-screen app, presentation or focus session is active. */
  muted: boolean;
  /** A message key to show on the board, e.g. why a paste did not happen. */
  notice: MessageKey | null;
}

export interface ShelfCloseEvent {
  generation: number;
}

export type ShelfKind = 'image' | 'text' | 'link';

export interface ShelfItem {
  id: number;
  kind: ShelfKind;
  title: string | null;
  /** First characters of a text; the full text stays in the backend. */
  preview: string | null;
  charCount: number | null;
  url: string | null;
  domain: string | null;
  width: number | null;
  height: number | null;
  hasThumb: boolean;
  pinned: boolean;
  lastCopiedAt: number;
}

export type Shelf = Record<ShelfKind, ShelfItem[]>;

export const getSettings = () => invoke<Settings>('get_settings');
export const updateSettings = (patch: SettingsPatch) => invoke<Settings>('update_settings', { patch });
export const closeShelf = () => invoke<void>('close_shelf');
export const shelfHidden = (generation: number) => invoke<void>('shelf_hidden', { generation });
export const listShelf = () => invoke<Shelf>('list_shelf');
export const setPinned = (ids: number[], pinned: boolean) => invoke<void>('set_pinned', { ids, pinned });
export const takeDown = (ids: number[]) => invoke<void>('take_down', { ids });
export const trashItems = (ids: number[]) => invoke<void>('trash_items', { ids });
/** Returns the ids that came back, empty when there was nothing to undo. */
export const undoTakeDown = () => invoke<number[]>('undo_take_down');
export const copyItems = (ids: number[]) => invoke<void>('copy_items', { ids });

/** Ctrl+Enter. `pasted`: the shelf closed and Ctrl+V was sent; otherwise the shelf stays open. */
export type PasteOutcome = 'pasted' | 'windowGone' | 'elevated' | 'refused';
export const pasteItems = (ids: number[]) => invoke<PasteOutcome>('paste_items', { ids });
export interface BoardRect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/** Runs a native drag of the items; letting go over `board` cancels. Resolves with whether they
 * were dropped somewhere. */
export const startDrag = (ids: number[], board: BoardRect) => invoke<boolean>('start_drag', { ids, board });

/** Managed media is served by item id only (D-014); Windows maps custom schemes to http://<scheme>.localhost. */
export function mediaUrl(variant: 'thumb' | 'original', id: number): string {
  const windows = /Windows/.test(navigator.userAgent);
  return windows ? `http://media.localhost/${variant}/${id}` : `media://localhost/${variant}/${id}`;
}
