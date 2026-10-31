import { describe, expect, it } from 'vitest';
import { commandFor, type KeyInput } from './shelfKeys';

const press = (key: string, extra: Partial<KeyInput> = {}): KeyInput => ({
  key,
  code: '',
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...extra,
});

describe('shelf keys', () => {
  it('maps arrows to moves, edges, pages and range selection', () => {
    expect(commandFor(press('ArrowRight'))).toEqual({ type: 'model', action: { type: 'move', dx: 1, extend: false } });
    expect(commandFor(press('ArrowLeft', { shiftKey: true }))).toEqual({
      type: 'model',
      action: { type: 'move', dx: -1, extend: true },
    });
    expect(commandFor(press('ArrowLeft', { ctrlKey: true }))).toEqual({
      type: 'model',
      action: { type: 'edge', end: 'first' },
    });
    expect(commandFor(press('ArrowRight', { altKey: true }))).toEqual({ type: 'model', action: { type: 'page', dir: 1 } });
    expect(commandFor(press('ArrowDown'))).toEqual({ type: 'model', action: { type: 'row', dy: 1 } });
    expect(commandFor(press('Tab', { shiftKey: true }))).toEqual({ type: 'model', action: { type: 'tab', back: true } });
  });

  it('lists every key on ? and F1, however the layout types ?', () => {
    expect(commandFor(press('?', { shiftKey: true, code: 'Slash' }))).toEqual({ type: 'keys' });
    expect(commandFor(press('?', { shiftKey: true, code: 'Minus' }))).toEqual({ type: 'keys' });
    expect(commandFor(press('?', { ctrlKey: true, altKey: true }))).toEqual({ type: 'keys' });
    expect(commandFor(press('F1'))).toEqual({ type: 'keys' });
    expect(commandFor(press('F1', { ctrlKey: true }))).toBeNull();
  });

  it('separates copy, copy-and-return and paste', () => {
    expect(commandFor(press('Enter'))).toEqual({ type: 'copyAndReturn' });
    expect(commandFor(press('Enter', { ctrlKey: true }))).toEqual({ type: 'paste' });
    expect(commandFor(press('c', { ctrlKey: true, code: 'KeyC' }))).toEqual({ type: 'copy' });
    expect(commandFor(press('a', { ctrlKey: true, code: 'KeyA' }))).toEqual({
      type: 'model',
      action: { type: 'selectAll' },
    });
  });

  it('picks numbered cards, also where digits need Shift', () => {
    expect(commandFor(press('7', { code: 'Digit7' }))).toEqual({ type: 'model', action: { type: 'pick', n: 7 } });
    expect(commandFor(press('&', { code: 'Digit1' }))).toEqual({ type: 'model', action: { type: 'pick', n: 1 } });
    expect(commandFor(press('0', { code: 'Digit0' }))).toBeNull();
    expect(commandFor(press('3', { code: 'Digit3', ctrlKey: true }))).toBeNull();
  });

  it('treats the Turkish I forms as one key for the row toggles', () => {
    for (const key of ['I', 'İ', 'ı', 'i']) {
      expect(commandFor(press(key, { shiftKey: true, code: 'KeyI' }))).toEqual({ type: 'toggleRow', kind: 'image' });
    }
    expect(commandFor(press('T', { shiftKey: true, code: 'KeyT' }))).toEqual({ type: 'toggleRow', kind: 'text' });
    expect(commandFor(press('L', { shiftKey: true, code: 'KeyL' }))).toEqual({ type: 'toggleRow', kind: 'link' });
  });

  it('uses the physical key on layouts that type non-Latin letters', () => {
    expect(commandFor(press('з', { code: 'KeyP' }))).toEqual({ type: 'pin' });
    expect(commandFor(press('ы', { code: 'KeyS' }))).toEqual({ type: 'takeDown' });
  });

  it('keeps letters without modifiers for pin, take down and the pinned filter', () => {
    expect(commandFor(press('p', { code: 'KeyP' }))).toEqual({ type: 'pin' });
    expect(commandFor(press('s', { code: 'KeyS' }))).toEqual({ type: 'takeDown' });
    expect(commandFor(press('f', { code: 'KeyF' }))).toEqual({ type: 'pinnedOnly' });
  });

  it('opens the search, the palette and the link, and copies the text in an image', () => {
    expect(commandFor(press('S', { code: 'KeyS', shiftKey: true }))).toEqual({ type: 'search' });
    expect(commandFor(press('k', { code: 'KeyK', ctrlKey: true }))).toEqual({ type: 'palette' });
    expect(commandFor(press('o', { code: 'KeyO', ctrlKey: true }))).toEqual({ type: 'openLink' });
    expect(commandFor(press('C', { code: 'KeyC', ctrlKey: true, shiftKey: true }))).toEqual({ type: 'copyText' });
  });

  it('names, files, tags and exports items and opens the Library', () => {
    expect(commandFor(press('p', { code: 'KeyP', altKey: true }))).toEqual({ type: 'name' });
    expect(commandFor(press('P', { code: 'KeyP', shiftKey: true }))).toEqual({ type: 'folder' });
    expect(commandFor(press('T', { code: 'KeyT', ctrlKey: true, shiftKey: true }))).toEqual({ type: 'tag' });
    expect(commandFor(press('E', { code: 'KeyE', ctrlKey: true, shiftKey: true }))).toEqual({ type: 'export' });
    expect(commandFor(press('l', { code: 'KeyL' }))).toEqual({ type: 'library' });
    // The row toggles keep Shift+T and Shift+L.
    expect(commandFor(press('T', { code: 'KeyT', shiftKey: true }))).toEqual({ type: 'toggleRow', kind: 'text' });
    expect(commandFor(press('p', { code: 'KeyP', altKey: true, ctrlKey: true }))).toBeNull();
  });

  it('opens the preview and its editors', () => {
    expect(commandFor(press(' ', { code: 'Space' }))).toEqual({ type: 'preview' });
    expect(commandFor(press(' ', { code: 'Space', ctrlKey: true }))).toBeNull();
    expect(commandFor(press('e', { code: 'KeyE' }))).toEqual({ type: 'editText' });
    expect(commandFor(press('m', { code: 'KeyM' }))).toEqual({ type: 'markup' });
    expect(commandFor(press('c', { code: 'KeyC' }))).toEqual({ type: 'crop' });
    expect(commandFor(press('c', { code: 'KeyC', ctrlKey: true }))).toEqual({ type: 'copy' });
  });

  it('maps undo, trash, the menu keys and Escape', () => {
    expect(commandFor(press('Backspace'))).toEqual({ type: 'undo' });
    expect(commandFor(press('Delete'))).toEqual({ type: 'trash' });
    expect(commandFor(press('Backspace', { metaKey: true }))).toEqual({ type: 'trash' });
    expect(commandFor(press('Backspace', { ctrlKey: true }))).toBeNull();
    expect(commandFor(press('ContextMenu'))).toEqual({ type: 'menu' });
    expect(commandFor(press('F10', { shiftKey: true }))).toEqual({ type: 'menu' });
    expect(commandFor(press('F10'))).toBeNull();
    expect(commandFor(press('Escape'))).toEqual({ type: 'escape' });
  });
});
