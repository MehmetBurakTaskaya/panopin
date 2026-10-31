import { describe, expect, it } from 'vitest';
import { SHELF_KEYS } from './keyList';
import { commandFor, type Command, type KeyInput } from './shelfKeys';

const NAMES: Record<string, string> = { '←': 'ArrowLeft', '→': 'ArrowRight', '↑': 'ArrowUp', '↓': 'ArrowDown', Space: ' ', Esc: 'Escape' };

/** The key presses a listed key stands for; "1–9" is each digit. */
function pressesOf(listed: string): KeyInput[] {
  const parts = listed.split('+');
  const main = parts.pop()!;
  const held = new Set(parts.map((part) => part.toLowerCase()));
  const modifiers = {
    ctrlKey: held.has('commandorcontrol') || held.has('ctrl'),
    altKey: held.has('alt'),
    shiftKey: held.has('shift'),
    metaKey: held.has('super'),
  };
  const keys = main === '1–9' ? ['1', '5', '9'] : [NAMES[main] ?? (main.length === 1 ? main.toLowerCase() : main)];
  return keys.map((key) => ({ ...modifiers, key, code: /^[a-z]$/.test(key) ? `Key${key.toUpperCase()}` : '' }));
}

const commandsOf = (label: string): (Command | null)[] =>
  SHELF_KEYS.flatMap((group) => group.entries)
    .filter((entry) => entry.label === label)
    .flatMap((entry) => entry.keys.flatMap(pressesOf))
    .map(commandFor);

describe('key list', () => {
  it('lists only keys the shelf answers', () => {
    for (const group of SHELF_KEYS) {
      for (const entry of group.entries) {
        for (const listed of entry.keys) {
          for (const press of pressesOf(listed)) expect(commandFor(press), `${entry.label}: ${listed}`).not.toBeNull();
        }
      }
    }
  });

  it('puts each key next to what it does', () => {
    expect(commandsOf('palette.rowImages')).toEqual([{ type: 'toggleRow', kind: 'image' }]);
    expect(commandsOf('palette.rowLinks')).toEqual([{ type: 'toggleRow', kind: 'link' }]);
    expect(commandsOf('keys.copyAndReturn')).toEqual([{ type: 'copyAndReturn' }]);
    expect(commandsOf('shelf.menu.paste')).toEqual([{ type: 'paste' }]);
    expect(commandsOf('keys.list')).toEqual([{ type: 'keys' }, { type: 'keys' }]);
    expect(commandsOf('shelf.search.label')).toEqual([{ type: 'search' }]);
    expect(commandsOf('shelf.menu.trash')).toEqual([{ type: 'trash' }]);
    expect(commandsOf('keys.menu')).toEqual([{ type: 'menu' }]);
  });
});
