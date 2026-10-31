import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { SHELF_KEYS } from '../../keyboard/keyList';
import { KeyNote } from './KeyNote';

describe('KeyNote', () => {
  afterEach(cleanup);

  it('lists the shelf keys and goes on Esc, ? or F1 only', () => {
    const onClose = vi.fn();
    render(<KeyNote groups={SHELF_KEYS} onClose={onClose} />);
    expect(screen.getByRole('dialog', { name: 'Keys on the shelf' })).toBeTruthy();
    expect(screen.getByText('Show or hide the link row')).toBeTruthy();
    fireEvent.keyDown(window, { key: 'p' });
    expect(onClose).not.toHaveBeenCalled();
    fireEvent.keyDown(window, { key: 'Escape' });
    fireEvent.keyDown(window, { key: '?' });
    fireEvent.keyDown(window, { key: 'F1' });
    expect(onClose).toHaveBeenCalledTimes(3);
  });
});
