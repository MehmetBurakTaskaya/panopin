import { listen } from '@tauri-apps/api/event';
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { listShelf, searchShelf, type Shelf, type ShelfItem } from '../../ipc';
import { KINDS, type Kind, type Rows } from '../../keyboard/shelfModel';

/** Typing settles this long before the shelf asks Rust for matches. */
const SEARCH_DELAY_MS = 70;

export const EMPTY_SHELF: Shelf = { image: [], text: [], link: [] };

/** The ids the keyboard model works on: hidden rows are empty, F keeps pinned items only. */
export function rowsOf(shelf: Shelf, hidden: Record<Kind, boolean>, pinnedOnly: boolean): Rows {
  const ids = (kind: Kind) =>
    hidden[kind] ? [] : shelf[kind].filter((item) => !pinnedOnly || item.pinned).map((item) => item.id);
  return { image: ids('image'), text: ids('text'), link: ids('link') };
}

const sameItem = (a: ShelfItem, b: ShelfItem) =>
  (Object.keys(b) as (keyof ShelfItem)[]).every((key) => a[key] === b[key]);

/** The new rows with every unchanged item kept as the same object, so its memoised card does not
 * render again; the previous shelf itself when nothing changed (a note or a tag in the Library). */
export function reuseItems(previous: Shelf, next: Shelf): Shelf {
  const before = new Map(KINDS.flatMap((kind) => previous[kind]).map((item) => [item.id, item] as const));
  let same = true;
  const rows = { ...next };
  for (const kind of KINDS) {
    rows[kind] = next[kind].map((item) => {
      const old = before.get(item.id);
      return old && sameItem(old, item) ? old : item;
    });
    same &&= rows[kind].length === previous[kind].length && rows[kind].every((item, i) => item === previous[kind][i]);
  }
  return same ? previous : rows;
}

/** Shelf contents, kept current while the window is hidden so an opening shows them at once. */
export function useShelfData() {
  const [shelf, setShelf] = useState<Shelf>(EMPTY_SHELF);
  const latestRequest = useRef(0);

  const refresh = useCallback(async (): Promise<Shelf | null> => {
    const request = ++latestRequest.current;
    try {
      const next = await listShelf();
      // An older answer arriving late must not overwrite a newer one.
      if (request === latestRequest.current) setShelf((previous) => reuseItems(previous, next));
      return next;
    } catch {
      return null;
    }
  }, []);

  useEffect(() => {
    void refresh();
    const unlisten = listen('items:changed', () => void refresh());
    return () => void unlisten.then((stop) => stop());
  }, [refresh]);

  return { shelf, refresh };
}

/** Shift+S: the matching cards for what was typed; `null` while nothing searchable is typed, so
 * the shelf shows everything. Results stay current when items change. */
export function useShelfSearch(query: string, withoutAccents: boolean) {
  const [results, setResults] = useState<Shelf | null>(null);
  const latestRequest = useRef(0);

  const run = useCallback(async (text: string) => {
    const request = ++latestRequest.current;
    if (!text.trim()) {
      setResults(null);
      return;
    }
    try {
      const next = await searchShelf(text);
      if (request === latestRequest.current) setResults((previous) => (previous ? reuseItems(previous, next) : next));
    } catch {
      // The previous results stay until the next keystroke.
    }
  }, []);

  // The accent setting changes what matches, so it runs the search again.
  useEffect(() => {
    const timer = window.setTimeout(() => void run(query), query.trim() ? SEARCH_DELAY_MS : 0);
    return () => window.clearTimeout(timer);
  }, [query, withoutAccents, run]);

  // One listener while a search is typed, not one per keystroke; it reads the newest query.
  const latestQuery = useRef(query);
  useLayoutEffect(() => {
    latestQuery.current = query;
  });
  const searching = query.trim() !== '';
  useEffect(() => {
    if (!searching) return;
    const unlisten = listen('items:changed', () => void run(latestQuery.current));
    return () => void unlisten.then((stop) => stop());
  }, [searching, run]);

  return results;
}
