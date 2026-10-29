import { listen } from '@tauri-apps/api/event';
import { useCallback, useEffect, useRef, useState } from 'react';
import { listShelf, searchShelf, type Shelf } from '../../ipc';
import type { Kind, Rows } from '../../keyboard/shelfModel';

/** Typing settles this long before the shelf asks Rust for matches. */
const SEARCH_DELAY_MS = 70;

export const EMPTY_SHELF: Shelf = { image: [], text: [], link: [] };

/** The ids the keyboard model works on: hidden rows are empty, F keeps pinned items only. */
export function rowsOf(shelf: Shelf, hidden: Record<Kind, boolean>, pinnedOnly: boolean): Rows {
  const ids = (kind: Kind) =>
    hidden[kind] ? [] : shelf[kind].filter((item) => !pinnedOnly || item.pinned).map((item) => item.id);
  return { image: ids('image'), text: ids('text'), link: ids('link') };
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
      if (request === latestRequest.current) setShelf(next);
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
      if (request === latestRequest.current) setResults(next);
    } catch {
      // The previous results stay until the next keystroke.
    }
  }, []);

  // The accent setting changes what matches, so it runs the search again.
  useEffect(() => {
    const timer = window.setTimeout(() => void run(query), query.trim() ? SEARCH_DELAY_MS : 0);
    return () => window.clearTimeout(timer);
  }, [query, withoutAccents, run]);

  useEffect(() => {
    if (!query.trim()) return;
    const unlisten = listen('items:changed', () => void run(query));
    return () => void unlisten.then((stop) => stop());
  }, [query, run]);

  return results;
}
