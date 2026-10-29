import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { setLanguage } from '../../i18n';
import type { Settings } from '../../ipc';
import { useSettings } from '../../state/settings';
import { testSettings } from '../../test/fixtures';
import { SettingsApp } from './SettingsApp';

describe('SettingsApp', () => {
  let stored: Settings;
  let patches: unknown[];
  let calls: { cmd: string; args: unknown }[];

  beforeEach(async () => {
    stored = testSettings();
    patches = [];
    calls = [];
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
      switch (cmd) {
        case 'get_settings':
          return stored;
        case 'update_settings': {
          const patch = (args as { patch: Partial<Settings> }).patch;
          patches.push(patch);
          stored = { ...stored, ...patch };
          return stored;
        }
        case 'system_look':
          return { mica: true, accentLight: '#005fb8', accentDark: '#60cdff' };
        case 'unavailable_shortcuts':
          return [];
        case 'suspend_shortcuts':
          return null;
        case 'set_shortcut': {
          const { accelerator } = args as { accelerator: string | null };
          stored = { ...stored, shortcuts: { ...stored.shortcuts, library: accelerator } };
          return stored;
        }
        case 'count_clearable':
          return { count: 4, cutoff: 1234 };
        case 'clear_history':
          return 4;
        case 'app_version':
          return '0.1.0';
        default:
          throw new Error(`unexpected command ${cmd}`);
      }
    });
    await useSettings.getState().load();
  });

  afterEach(() => {
    cleanup();
    clearMocks();
    setLanguage('en');
  });

  const openTab = async (name: string) => {
    await act(async () => {
      fireEvent.click(screen.getByRole('tab', { name }));
    });
  };

  it('moves between tabs with the arrow keys', async () => {
    render(<SettingsApp />);
    const general = screen.getByRole('tab', { name: 'General' });
    expect(general.getAttribute('aria-selected')).toBe('true');
    await act(async () => {
      fireEvent.keyDown(general, { key: 'ArrowDown' });
    });
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Sources');
    await act(async () => {
      fireEvent.keyDown(screen.getByRole('tab', { name: 'Sources' }), { key: 'End' });
    });
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('About');
  });

  it('switches the language immediately through the backend', async () => {
    render(<SettingsApp />);
    await act(async () => {
      fireEvent.change(screen.getByRole('combobox', { name: /^Language/ }), { target: { value: 'tr' } });
    });
    expect(patches).toEqual([{ language: 'tr' }]);
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Genel');
    expect(document.documentElement.lang).toBe('tr');
  });

  it('changes the shelf look and sounds', async () => {
    render(<SettingsApp />);
    await act(async () => {
      fireEvent.change(screen.getByRole('combobox', { name: /^Slides in from/ }), { target: { value: 'bottom' } });
    });
    await act(async () => {
      fireEvent.click(screen.getByRole('switch', { name: 'Sound effects' }));
    });
    expect(patches).toEqual([{ shelfEdge: 'bottom' }, { sounds: { enabled: false } }]);
    expect((screen.getByRole('slider', { name: 'Volume' }) as HTMLInputElement).disabled).toBe(true);
    await openTab('Appearance');
    await act(async () => {
      fireEvent.change(screen.getByRole('combobox', { name: /^Card size/ }), { target: { value: 'l' } });
    });
    expect(patches.at(-1)).toEqual({ cardSize: 'l' });
  });

  it('records a shortcut by the keys pressed, with Panopin shortcuts paused meanwhile', async () => {
    render(<SettingsApp />);
    await openTab('Shortcuts');
    expect(screen.getByText('Win+Alt+V')).toBeTruthy();
    const change = screen.getAllByRole('button', { name: 'Change' })[3]!; // Open the Library
    await act(async () => {
      fireEvent.click(change);
    });
    expect(calls.some((c) => c.cmd === 'suspend_shortcuts' && (c.args as { suspended: boolean }).suspended)).toBe(true);
    await act(async () => {
      fireEvent.keyDown(window, { key: 'l', code: 'KeyL', metaKey: true, altKey: true });
    });
    const saved = calls.find((c) => c.cmd === 'set_shortcut');
    expect(saved?.args).toEqual({ action: 'openLibrary', accelerator: 'Super+Alt+L' });
    expect(screen.getByText('Win+Alt+L')).toBeTruthy();
    expect(calls.filter((c) => c.cmd === 'suspend_shortcuts').at(-1)?.args).toEqual({ suspended: false });
  });

  it('counts before clearing history and clears only after confirming', async () => {
    render(<SettingsApp />);
    await openTab('History');
    await act(async () => {
      fireEvent.click(screen.getByRole('radio', { name: 'Texts' }));
    });
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Clear…' }));
    });
    expect(calls.find((c) => c.cmd === 'count_clearable')?.args).toEqual({ kinds: ['text'] });
    expect(calls.some((c) => c.cmd === 'clear_history')).toBe(false);
    expect(screen.getByText('Move 4 unsaved items to Trash?')).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Move to Trash' }));
    });
    expect(calls.find((c) => c.cmd === 'clear_history')?.args).toEqual({ kinds: ['text'], cutoff: 1234 });
  });
});
