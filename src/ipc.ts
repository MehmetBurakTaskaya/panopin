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

export interface Settings {
  language: LanguageSetting;
  theme: Theme;
  shortcuts: Shortcuts;
}

export type SettingsPatch = {
  language?: LanguageSetting;
  theme?: Theme;
  shortcuts?: Partial<Shortcuts>;
};

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
