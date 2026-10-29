import { listen } from '@tauri-apps/api/event';
import { create } from 'zustand';
import { resolveLanguage, setLanguage, type Lang } from '../i18n';
import { getSettings, updateSettings, type Settings, type SettingsPatch } from '../ipc';

interface SettingsState {
  settings: Settings | null;
  /** Resolved UI language; components select it so they re-render on change. */
  lang: Lang;
  error: string | null;
  load: () => Promise<void>;
  update: (patch: SettingsPatch) => Promise<void>;
}

function applyLanguage(settings: Settings): Lang {
  const lang = resolveLanguage(settings.language);
  setLanguage(lang);
  return lang;
}

export const useSettings = create<SettingsState>((set) => ({
  settings: null,
  lang: 'en',
  error: null,
  load: async () => {
    const settings = await getSettings();
    set({ settings, lang: applyLanguage(settings), error: null });
  },
  update: async (patch) => {
    try {
      const settings = await updateSettings(patch);
      set({ settings, lang: applyLanguage(settings), error: null });
    } catch (err) {
      set({ error: String(err) });
    }
  },
}));

/** Keeps this window in sync when settings change in another window. */
export function followSettingsChanges(): Promise<() => void> {
  return listen<Settings>('settings:changed', ({ payload }) => {
    useSettings.setState({ settings: payload, lang: applyLanguage(payload), error: null });
  });
}
