// Typed wrappers around the Rust commands and events. Types mirror src-tauri/src/settings.rs;
// they move to ts-rs generated bindings once the data model grows (D-013).
import { invoke } from '@tauri-apps/api/core';
import type { LanguageSetting } from './i18n';

export type Theme = 'system' | 'light' | 'dark';

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
}

export interface ShelfCloseEvent {
  generation: number;
}

export const getSettings = () => invoke<Settings>('get_settings');
export const updateSettings = (patch: SettingsPatch) => invoke<Settings>('update_settings', { patch });
export const closeShelf = () => invoke<void>('close_shelf');
export const shelfHidden = (generation: number) => invoke<void>('shelf_hidden', { generation });
