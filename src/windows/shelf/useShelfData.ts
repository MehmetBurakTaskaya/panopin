import { listen } from '@tauri-apps/api/event';
import { useCallback, useEffect, useRef, useState } from 'react';
import { listShelf, type Shelf } from '../../ipc';
import type { Kind, Rows } from '../../keyboard/shelfModel';

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
