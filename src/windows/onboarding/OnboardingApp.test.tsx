import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { setLanguage } from '../../i18n';
import type { Settings, SettingsPatch } from '../../ipc';
import { useSettings } from '../../state/settings';
import { testSettings } from '../../test/fixtures';
import { OnboardingApp } from './OnboardingApp';

describe('OnboardingApp', () => {
  let patches: SettingsPatch[];

  beforeEach(async () => {
    patches = [];
    let stored: Settings = { ...testSettings(), onboarded: false };
    mockIPC((cmd, args) => {
      if (cmd === 'get_settings') return stored;
      if (cmd === 'update_settings') {
        const patch = (args as { patch: SettingsPatch }).patch;
        patches.push(patch);
        stored = { ...stored, onboarded: patch.onboarded ?? stored.onboarded };
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

  it('says plainly that text is collected and nothing leaves the PC', () => {
    render(<OnboardingApp />);
    expect(screen.getByText(/Text is collected too/)).toBeTruthy();
    expect(screen.getByText(/Everything stays on this PC/)).toBeTruthy();
    expect(screen.getByText(/not a password vault/)).toBeTruthy();
  });

  it('starts capture with the chosen sources and link previews off', async () => {
    render(<OnboardingApp />);
    fireEvent.click(screen.getByLabelText('Text you copy'));
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Start' }));
    });
    expect(patches).toEqual([
      {
        onboarded: true,
        linkPreviews: false,
        sources: {
          screenshots: true,
          clipboardImages: true,
          clipboardText: false,
          links: true,
          screenRecordings: false,
        },
      },
    ]);
  });
});
