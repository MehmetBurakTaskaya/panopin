import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { setLanguage } from '../../i18n';
import type { Settings } from '../../ipc';
import { useSettings } from '../../state/settings';
import { SettingsApp } from './SettingsApp';

const defaults: Settings = {
  language: 'en',
  theme: 'system',
  shortcuts: {
    shelf: 'Super+Alt+V',
    shelfSearch: 'Super+Alt+C',
    snip: 'Super+Alt+S',
    library: null,
    copyLastImage: null,
    pauseCapture: null,
  },
};

describe('SettingsApp', () => {
  let stored: Settings;
  let patches: unknown[];

  beforeEach(async () => {
    stored = structuredClone(defaults);
    patches = [];
    mockIPC((cmd, args) => {
      if (cmd === 'get_settings') return stored;
      if (cmd === 'update_settings') {
        const patch = (args as { patch: Partial<Settings> }).patch;
        patches.push(patch);
        stored = { ...stored, ...patch };
        return stored;
      }
      throw new Error(`unexpected command ${cmd}`);
    });
    await useSettings.getState().load();
  });

  afterEach(() => {
    cleanup();
    clearMocks();
    setLanguage('en');
  });

  it('shows the current shortcuts', () => {
    render(<SettingsApp />);
    expect(screen.getByText('Win+Alt+V')).toBeTruthy();
    expect(screen.getByText('Win+Alt+C')).toBeTruthy();
  });

  it('switches the language immediately through the backend', async () => {
    render(<SettingsApp />);
    await act(async () => {
      fireEvent.change(screen.getByRole('combobox'), { target: { value: 'tr' } });
    });
    expect(patches).toEqual([{ language: 'tr' }]);
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Genel');
    expect(document.documentElement.lang).toBe('tr');
  });
});
