import { describe, expect, it } from 'vitest';
import type { LibraryItem } from '../ipc';
import { NO_FILTERS, displayTitle, inSection, matches, sectionCounts, sortItems } from './query';
import { emptyModel, reduceLibrary, targets } from '../keyboard/libraryModel';

const DAY = 86_400_000;
const NOW = Date.UTC(2026, 9, 29, 12);

let nextId = 1;
function item(extra: Partial<LibraryItem>): LibraryItem {
  return {
    id: nextId++,
    kind: 'text',
    title: null,
    note: null,
    preview: 'text',
    charCount: 4,
    url: null,
    domain: null,
    width: null,
    height: null,
    hasThumb: false,
    pinned: false,
    saved: false,
    edited: false,
    thumbVersion: 0,
    createdAt: NOW - DAY,
    lastCopiedAt: NOW - DAY,
    trashedAt: null,
    source: 'clipboard',
    sourceApp: null,
    sourcePath: null,
    byteSize: null,
    folders: [],
    tags: [],
    ...extra,
  };
}

describe('library sections and filters', () => {
  const copy = item({});
  const pinned = item({ pinned: true, saved: true });
  const filed = item({ saved: true, folders: [7], tags: [1, 2] });
  const trashed = item({ trashedAt: NOW });
  const all = [copy, pinned, filed, trashed];

  it('sorts items into the sidebar sections', () => {
    const ids = (type: Parameters<typeof inSection>[1]) => all.filter((i) => inSection(i, type)).map((i) => i.id);
    expect(ids({ type: 'all' })).toEqual([copy.id, pinned.id, filed.id]);
    expect(ids({ type: 'recent' })).toEqual([copy.id]);
    expect(ids({ type: 'pinned' })).toEqual([pinned.id]);
    expect(ids({ type: 'unsorted' })).toEqual([pinned.id]);
    expect(ids({ type: 'folder', id: 7 })).toEqual([filed.id]);
    expect(ids({ type: 'trash' })).toEqual([trashed.id]);
    const counts = sectionCounts(all);
    expect([counts.all, counts.recent, counts.pinned, counts.unsorted, counts.trash]).toEqual([3, 1, 1, 1, 1]);
    expect(counts.tags.get(2)).toBe(1);
  });

  it('needs every chosen tag and honours kind, pin, domain and date', () => {
    expect(matches(filed, { ...NO_FILTERS, tags: [1, 2] }, NOW)).toBe(true);
    expect(matches(filed, { ...NO_FILTERS, tags: [1, 3] }, NOW)).toBe(false);
    expect(matches(copy, { ...NO_FILTERS, kind: 'image' }, NOW)).toBe(false);
    expect(matches(copy, { ...NO_FILTERS, pinnedOnly: true }, NOW)).toBe(false);
    const link = item({ kind: 'link', url: 'https://a.example/', domain: 'a.example' });
    expect(matches(link, { ...NO_FILTERS, domain: 'a.example' }, NOW)).toBe(true);
    expect(matches(copy, { ...NO_FILTERS, domain: 'a.example' }, NOW)).toBe(false);
    const old = item({ lastCopiedAt: NOW - 40 * DAY });
    expect(matches(old, { ...NO_FILTERS, date: 'month' }, NOW)).toBe(false);
    expect(matches(old, { ...NO_FILTERS, date: 'older' }, NOW)).toBe(true);
    expect(matches(item({ lastCopiedAt: NOW }), { ...NO_FILTERS, date: 'today' }, NOW)).toBe(true);
  });

  it('sorts by title with Turkish letters and keeps untagged items last', () => {
    const a = item({ title: 'çay' });
    const b = item({ title: 'İstanbul' });
    const c = item({ title: 'armut', tags: [5] });
    const d = item({ title: 'zeytin', tags: [6] });
    const names = (id: number) => ({ 5: 'zz', 6: 'aa' })[id] ?? '';
    expect(sortItems([a, b, c], 'title', false, names, 'tr').map(displayTitle)).toEqual(['armut', 'çay', 'İstanbul']);
    expect(sortItems([a, c, d], 'tags', false, names, 'tr').map((i) => i.id)).toEqual([d.id, c.id, a.id]);
    expect(sortItems([a, c, d], 'tags', true, names, 'tr').map((i) => i.id)).toEqual([c.id, d.id, a.id]);
    const older = item({ lastCopiedAt: NOW - 5 * DAY });
    expect(sortItems([older, a], 'lastCopied', true, names, 'en')[0]).toBe(a);
  });
});

describe('library keyboard model', () => {
  const ids = [10, 11, 12, 13, 14, 15, 16];

  it('moves through a grid by columns and clamps at the edges', () => {
    let m = reduceLibrary(reduceLibrary(emptyModel(3), { type: 'setIds', ids }), { type: 'move', dx: 0, dy: 0 });
    expect(m.cursor).toBe(0);
    m = reduceLibrary(m, { type: 'move', dx: 0, dy: 1 });
    expect(m.ids[m.cursor!]).toBe(13);
    m = reduceLibrary(m, { type: 'move', dx: 0, dy: 5 });
    expect(m.ids[m.cursor!]).toBe(16);
    m = reduceLibrary(m, { type: 'edge', end: 'first' });
    expect(targets(m)).toEqual([10]);
  });

  it('extends ranges with Shift and toggles with Ctrl', () => {
    let m = reduceLibrary(emptyModel(1), { type: 'setIds', ids });
    m = reduceLibrary(m, { type: 'click', id: 11 });
    m = reduceLibrary(m, { type: 'move', dx: 2, dy: 0, extend: true });
    expect(m.selection).toEqual([11, 12, 13]);
    m = reduceLibrary(m, { type: 'click', id: 15, ctrl: true });
    expect(m.selection).toEqual([11, 12, 13, 15]);
    m = reduceLibrary(m, { type: 'click', id: 12, ctrl: true });
    expect(m.selection).toEqual([11, 13, 15]);
  });

  it('stays on its item when the list changes and steps to the next one when it goes', () => {
    let m = reduceLibrary(emptyModel(1), { type: 'setIds', ids });
    m = reduceLibrary(m, { type: 'click', id: 13 });
    m = reduceLibrary(m, { type: 'setIds', ids: [9, ...ids] });
    expect(m.ids[m.cursor!]).toBe(13);
    m = reduceLibrary(m, { type: 'setIds', ids: [9, 10, 11, 12, 14, 15, 16] });
    expect(targets(m)).toEqual([14]);
    m = reduceLibrary(m, { type: 'setIds', ids: [] });
    expect(m.cursor).toBeNull();
  });

  it('moves by rows of the current width and keeps the model when the width does not change', () => {
    let m = reduceLibrary(reduceLibrary(emptyModel(3), { type: 'setIds', ids }), { type: 'click', id: 10 });
    // Switching to the list makes it one column wide: Down is the next item.
    m = reduceLibrary(m, { type: 'setColumns', columns: 1 });
    m = reduceLibrary(m, { type: 'move', dx: 0, dy: 1, extend: true });
    expect(m.selection).toEqual([10, 11]);
    expect(reduceLibrary(m, { type: 'setColumns', columns: 1 })).toBe(m);
  });
});
