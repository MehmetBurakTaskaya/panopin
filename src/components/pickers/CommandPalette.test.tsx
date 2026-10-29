import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { setLanguage } from '../../i18n';
import { CommandPalette, filterCommands, type PaletteCommand } from './CommandPalette';

const commands = (runs: string[]): PaletteCommand[] =>
  ['Kopyala', 'İsim ver ve sabitle', 'Görseldeki yazıyı kopyala', 'Ayarları aç'].map((label, i) => ({
    id: `c${i}`,
    label,
    shortcut: i === 2 ? 'Ctrl+Shift+C' : undefined,
    run: () => runs.push(label),
  }));

describe('command palette', () => {
  afterEach(() => {
    cleanup();
    setLanguage('en');
  });

  it('matches every typed word anywhere in a label, Turkish-aware', () => {
    setLanguage('tr');
    const list = commands([]);
    expect(filterCommands(list, 'kopya').map((c) => c.label)).toEqual(['Kopyala', 'Görseldeki yazıyı kopyala']);
    expect(filterCommands(list, 'ISIM sabit').map((c) => c.label)).toEqual(['İsim ver ve sabitle']);
    expect(filterCommands(list, '  ')).toHaveLength(4);
  });

  it('runs the chosen command with Enter and closes first', async () => {
    const runs: string[] = [];
    const onClose = vi.fn();
    render(<CommandPalette commands={commands(runs)} onClose={onClose} />);
    const input = screen.getByRole('textbox');
    await act(async () => {
      fireEvent.change(input, { target: { value: 'kopyala' } });
    });
    expect(screen.getByText('Ctrl+Shift+C')).toBeTruthy();
    await act(async () => {
      fireEvent.keyDown(input, { key: 'ArrowDown' });
    });
    await act(async () => {
      fireEvent.keyDown(input, { key: 'Enter' });
    });
    expect(onClose).toHaveBeenCalled();
    expect(runs).toEqual(['Görseldeki yazıyı kopyala']);
  });
});
