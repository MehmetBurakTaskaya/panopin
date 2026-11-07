import { listen } from '@tauri-apps/api/event';
import { create } from 'zustand';
import { loadLanguage, resolveLanguage, setLanguage, type Lang } from '../i18n';
import { getSettings, updateSettings, type Settings, type SettingsPatch } from '../ipc';

interface SettingsState {
  settings: Settings | null;
  /** Resolved UI language; components select it so they re-render on change. */
  lang: Lang;
  error: string | null;
  load: () => Promise<void>;
  update: (patch: SettingsPatch) => Promise<void>;
}

/** The language the settings ask for, with its words loaded first. */
async function applyLanguage(settings: Settings): Promise<Lang> {
  const lang = resolveLanguage(settings.language);
  await loadLanguage(lang);
  setLanguage(lang);
  return lang;
}

export const useSettings = create<SettingsState>((set) => ({
  settings: null,
  lang: 'en',
  error: null,
  load: async () => {
    const settings = await getSettings();
    set({ settings, lang: await applyLanguage(settings), error: null });
  },
  update: async (patch) => {
    try {
      const settings = await updateSettings(patch);
      set({ settings, lang: await applyLanguage(settings), error: null });
    } catch (err) {
      set({ error: String(err) });
    }
  },
}));

/** Keeps this window in sync when settings change in another window. */
export function followSettingsChanges(): Promise<() => void> {
  let latest = 0;
  return listen<Settings>('settings:changed', async ({ payload }) => {
    // A language still loading must not overwrite a change that came after it.
    const change = ++latest;
    const lang = await applyLanguage(payload);
    if (change === latest) useSettings.setState({ settings: payload, lang, error: null });
  });
}
