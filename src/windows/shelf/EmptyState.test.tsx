import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { setLanguage } from '../../i18n';
import { EmptyState } from './EmptyState';

describe('EmptyState', () => {
  afterEach(() => {
    cleanup();
    setLanguage('en');
  });

  it('shows the shelf shortcut from settings', () => {
    render(<EmptyState shelfShortcut="Super+Alt+V" />);
    expect(screen.getByText('The board is empty')).toBeTruthy();
    expect(screen.getByText('Win+Alt+V')).toBeTruthy();
  });

  it('hides the shelf hint when the shortcut is off', () => {
    render(<EmptyState shelfShortcut={null} />);
    expect(screen.queryByText('Open or close the shelf')).toBeNull();
  });

  it('speaks Turkish', () => {
    setLanguage('tr');
    render(<EmptyState shelfShortcut="Super+Alt+V" />);
    expect(screen.getByText('Pano şimdilik boş')).toBeTruthy();
    expect(screen.getByText('Rafı aç / kapat')).toBeTruthy();
  });
});
