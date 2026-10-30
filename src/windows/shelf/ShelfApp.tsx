import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
  type MouseEvent,
  type PointerEvent as ReactPointerEvent,
} from 'react';
import { t } from '../../i18n';
import { CommandPalette, type PaletteCommand } from '../../components/pickers/CommandPalette';
import { FolderPicker, NamePrompt, TagPicker } from '../../components/pickers/Pickers';
import {
  addToFolder,
  closeShelf,
  copyImageText,
  copyItems,
  createFolder,
  exportItems,
  libraryGroups,
  openLibrary,
  openLink,
  openSettings,
  pasteItems,
  refreshLink,
  rowOf,
  revertEdit,
  setPinned,
  setTitle,
  shelfHidden,
  startDrag,
  tagItems,
  takeDown,
  trashItems,
  undoTakeDown,
  type Folder,
  type ShelfCloseEvent,
  type ShelfItem,
  type ShelfOpenEvent,
  type Tag,
} from '../../ipc';
import { commandFor, type Command } from '../../keyboard/shelfKeys';
import { KINDS, openModel, reduce, type Action, type Kind, type Rows, type ShelfModel } from '../../keyboard/shelfModel';
import { displayTitle } from '../../library/query';
import { reducedMotion as reducedMotionSetting } from '../../look';
import { queryTerms } from '../../search/highlight';
import { DRAG_OUT, TRASH_SHORTCUT } from '../../shortcuts';
import { configure, play, setMuted } from '../../sound/sounds';
import { useSettings } from '../../state/settings';
import { cardDomId, type Effect } from './Card';
import cardStyles from './Card.module.css';
import { ConfirmTrash } from './ConfirmTrash';
import { ContextMenu, type MenuEntry, type MenuPlacement } from './ContextMenu';
import { EmptyState } from './EmptyState';
import { SearchBox } from './SearchBox';
import { Preview, type PreviewMode } from './preview/Preview';
import { trackSave, whenSaved } from './preview/saves';
import styles from './Shelf.module.css';
import { ShelfRows } from './ShelfRows';
import { rowsOf, useShelfData, useShelfSearch } from './useShelfData';

// Matches the CSS transitions: open ~420 ms ease-out, close ~200 ms.
const SETTLE_MS = 430;
const CLOSE_MS = 210;
const TOAST_MS = 1100;
/** Pixels a pressed card must move before it becomes a drag. */
const DRAG_THRESHOLD = 6;

type Phase = 'hidden' | 'open' | 'settled' | 'closing';
type ModelAction = Action | { type: 'reset'; rows: Rows };

const ALL_ROWS: Record<Kind, boolean> = { image: false, text: false, link: false };

// Off in Settings, or reduced motion in Windows (D-060).
const reducedMotion = reducedMotionSetting;
const wait = (ms: number) => new Promise<void>((resolve) => window.setTimeout(resolve, reducedMotion() ? 0 : ms));

function modelReducer(model: ShelfModel, action: ModelAction): ShelfModel {
  return action.type === 'reset' ? openModel(action.rows, model.pageSize) : reduce(model, action);
}

const cursorId = (model: ShelfModel): number | null =>
  model.cursor ? (model.rows[model.cursor.kind][model.cursor.index] ?? null) : null;

const isEditable = (target: EventTarget | null) =>
  target instanceof HTMLElement && (target.isContentEditable || target.tagName === 'TEXTAREA' || target.tagName === 'INPUT');

interface Toast {
  key: number;
  x: number;
  y: number;
  text: string;
}

/** Alt+P, Shift+P and Ctrl+Shift+T: the same pickers as in the Library. */
interface Picker {
  type: 'name' | 'folder' | 'tag';
  ids: number[];
  folders: Folder[];
  tags: Tag[];
}

function CheckIcon() {
  return (
    <svg viewBox="0 0 16 16" aria-hidden="true">
      <path d="M3 8.5l3 3 7-7" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function ShelfApp() {
  useSettings((s) => s.lang);
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  const edge = settings?.shelfEdge ?? 'top';
  const [phase, setPhase] = useState<Phase>('hidden');
  const { shelf, refresh } = useShelfData();
  // Shift+S: while something is typed the rows show only the matches (D-059).
  const withoutAccents = settings?.searchWithoutAccents ?? false;
  const [searchOpen, setSearchOpen] = useState(false);
  const [query, setQuery] = useState('');
  const results = useShelfSearch(searchOpen ? query : '', withoutAccents);
  const shown = searchOpen && results ? results : shelf;
  const terms = useMemo(
    () => (searchOpen && results ? queryTerms(query, withoutAccents) : null),
    [searchOpen, results, query, withoutAccents],
  );
  const [searchTop, setSearchTop] = useState(0);
  const searchInput = useRef<HTMLInputElement>(null);
  // Row toggles (Shift+I/T/L) and the pinned filter (F) last for the session, like the board itself.
  const [hidden, setHidden] = useState(ALL_ROWS);
  const [pinnedOnly, setPinnedOnly] = useState(false);
  const rows = useMemo(() => rowsOf(shown, hidden, pinnedOnly), [shown, hidden, pinnedOnly]);
  const [model, dispatch] = useReducer(modelReducer, rows, (initial: Rows) => openModel(initial));
  const [effects, setEffects] = useState<ReadonlyMap<number, Effect>>(new Map());
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [notice, setNotice] = useState<string | null>(null);
  const [menu, setMenu] = useState<{ at: MenuPlacement; ids: number[] } | null>(null);
  const [confirm, setConfirm] = useState<number[] | null>(null);
  const [picker, setPicker] = useState<Picker | null>(null);
  // Ctrl+K: built when it opens, so its commands act on that moment's selection.
  const [palette, setPalette] = useState<PaletteCommand[] | null>(null);
  // Space opens the preview of the card under the cursor; E, M and C open its editors.
  const [preview, setPreview] = useState<PreviewMode | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const fallLayer = useRef<HTMLDivElement>(null);
  const grid = useRef<HTMLDivElement>(null);
  const board = useRef<HTMLElement>(null);
  const pressed = useRef<{ id: number; x: number; y: number } | null>(null);

  useEffect(() => {
    document.documentElement.dataset.size = settings?.cardSize ?? 'm';
  }, [settings?.cardSize]);

  useEffect(() => {
    if (settings) configure(settings.sounds.enabled, settings.sounds.volume);
  }, [settings]);

  useEffect(() => dispatch({ type: 'setRows', rows }), [rows]);

  useLayoutEffect(() => {
    if (!searchOpen) return;
    const place = () => {
      const rect = board.current?.getBoundingClientRect();
      const height = window.innerHeight;
      if (!rect) return setSearchTop(height / 2);
      setSearchTop(
        edge === 'top'
          ? Math.min(height - 64, Math.max(height / 2, rect.bottom + 72))
          : Math.max(64, Math.min(height / 2, rect.top - 72)),
      );
    };
    place();
    window.addEventListener('resize', place);
    return () => window.removeEventListener('resize', place);
  }, [searchOpen, edge, rows]);

  /** Starts a short card animation; `idleOnly` leaves cards that are already animating alone. */
  const flash = useCallback((ids: number[], effect: Effect, ms: number, idleOnly = false) => {
    setEffects((prev) => {
      const next = new Map(prev);
      for (const id of ids) if (!idleOnly || !next.has(id)) next.set(id, effect);
      return next;
    });
    window.setTimeout(
      () =>
        setEffects((prev) => {
          const next = new Map(prev);
          for (const id of ids) if (next.get(id) === effect) next.delete(id);
          return next;
        }),
      reducedMotion() ? 0 : ms,
    );
  }, []);

  // Event handlers read the newest state through this ref, so they can stay stable for memoised cards.
  const latest = useRef({ model, shown, rows, hidden, pinnedOnly, phase, menu, confirm, picker, palette, preview, searchOpen });
  useLayoutEffect(() => {
    latest.current = { model, shown, rows, hidden, pinnedOnly, phase, menu, confirm, picker, palette, preview, searchOpen };
  });

  const current = useMemo(() => {
    const id = cursorId(model);
    return id === null ? null : (KINDS.flatMap((kind) => shown[kind]).find((item) => item.id === id) ?? null);
  }, [model, shown]);

  const itemsOf = (ids: number[]): ShelfItem[] => {
    const all = new Map(KINDS.flatMap((kind) => latest.current.shown[kind]).map((item) => [item.id, item] as const));
    return ids.map((id) => all.get(id)).filter((item): item is ShelfItem => !!item);
  };

  /** The selection, or the card under the cursor when nothing is selected. */
  const targetIds = (): number[] => {
    const current = latest.current.model;
    if (current.selection.length > 0) return current.selection;
    const id = cursorId(current);
    return id === null ? [] : [id];
  };

  const fail = (err: unknown) => setNotice(t('shelf.error', { error: String(err) }));

  const toastAt = (id: number, text = t('shelf.copied')) => {
    const rect = document.getElementById(cardDomId(id))?.getBoundingClientRect();
    if (!rect) return;
    const key = performance.now();
    setToasts((list) => [...list, { key, x: rect.left + rect.width / 2, y: rect.top - 6, text }]);
    window.setTimeout(() => setToasts((list) => list.filter((toast) => toast.key !== key)), TOAST_MS);
  };

  /** Taken-down cards fall off the board as paper; the clones live on a layer React does not manage. */
  const dropPapers = (ids: number[]) => {
    const layer = fallLayer.current;
    if (!layer || reducedMotion()) return;
    for (const id of ids) {
      const card = document.getElementById(cardDomId(id));
      const rect = card?.parentElement?.getBoundingClientRect();
      if (!card || !rect) continue;
      const ghost = card.cloneNode(true) as HTMLElement;
      ghost.removeAttribute('id');
      ghost.removeAttribute('role');
      ghost.className = `${cardStyles.card} ${cardStyles.falling}`;
      const holder = document.createElement('div');
      holder.className = styles.faller ?? '';
      Object.assign(holder.style, {
        left: `${rect.left}px`,
        top: `${rect.top}px`,
        width: `${rect.width}px`,
        height: `${rect.height}px`,
      });
      holder.append(ghost);
      layer.append(holder);
      window.setTimeout(() => holder.remove(), 820);
    }
  };

  const copy = async (ids: number[], toastOn: number | null): Promise<boolean> => {
    if (ids.length === 0) return false;
    try {
      // A copy always carries the latest edits.
      await whenSaved();
      await copyItems(ids);
    } catch (err) {
      fail(err);
      return false;
    }
    play('copy');
    flash(ids, 'copied', 450);
    if (toastOn !== null) toastAt(toastOn);
    return true;
  };

  const copyAndReturn = async (ids: number[]) => {
    // The card jiggles briefly before the board slides away and focus goes back.
    if (await copy(ids, null)) {
      await wait(260);
      void closeShelf();
    }
  };

  const paste = async (ids: number[]) => {
    if (ids.length === 0) return;
    play('copy');
    flash(ids, 'copied', 450);
    await wait(160);
    try {
      await whenSaved();
      const outcome = await pasteItems(ids);
      if (outcome === 'windowGone') setNotice(t('shelf.paste.gone'));
      if (outcome === 'elevated') setNotice(t('shelf.paste.elevated'));
      if (outcome === 'noPermission') setNotice(t('shelf.paste.permission'));
    } catch (err) {
      fail(err);
    }
  };

  const togglePin = async (ids: number[]) => {
    const items = itemsOf(ids);
    if (items.length === 0) return;
    const pin = !items.every((item) => item.pinned);
    if (pin) {
      // The knock lands on the pin's impact frame.
      play('pin', reducedMotion() ? 0 : 0.21);
      flash(ids, 'pinning', 560);
    } else {
      play('unpin');
      flash(ids, 'unpinning', 400);
    }
    try {
      await setPinned(ids, pin);
    } catch (err) {
      fail(err);
    }
  };

  /** S takes cards down; Delete (after the confirmation) moves them to Trash. Both can be undone. */
  const remove = async (ids: number[], trash: boolean) => {
    const items = itemsOf(ids);
    if (items.length === 0) return;
    for (const item of items) flash([item.id], item.pinned ? 'unpinning' : 'peeling', 200);
    play('drop');
    await wait(150);
    dropPapers(ids);
    flash(ids, 'gone', 2000);
    try {
      await (trash ? trashItems(ids) : takeDown(ids));
    } catch (err) {
      fail(err);
    }
  };

  const openSearch = () => {
    setSearchOpen(true);
    window.requestAnimationFrame(() => {
      searchInput.current?.focus();
      searchInput.current?.select();
    });
  };

  /** Esc in the box, or on the results: the query goes and every card comes back. */
  const closeSearch = () => {
    setSearchOpen(false);
    setQuery('');
    grid.current?.focus({ preventScroll: true });
  };

  const undo = async () => {
    // A card coming back must be visible, so the search makes way.
    if (latest.current.searchOpen) closeSearch();
    let ids: number[];
    try {
      ids = await undoTakeDown();
    } catch (err) {
      fail(err);
      return;
    }
    if (ids.length === 0) return;
    const next = await refresh();
    if (!next) return;
    const restored = KINDS.flatMap((kind) => next[kind]).filter((item) => ids.includes(item.id));
    // A restored card must be visible: show its row and leave the pinned filter if needed.
    const nextHidden = { ...latest.current.hidden };
    for (const item of restored) nextHidden[rowOf(item.kind)] = false;
    const nextPinnedOnly = latest.current.pinnedOnly && restored.every((item) => item.pinned);
    setHidden(nextHidden);
    setPinnedOnly(nextPinnedOnly);
    dispatch({ type: 'setRows', rows: rowsOf(next, nextHidden, nextPinnedOnly) });
    const first = restored[0];
    if (first) dispatch({ type: 'click', id: first.id });
    play('back');
    flash(ids, 'restoring', 560);
  };

  /** Hiding the last visible row closes the shelf; the rows come back for the next opening. */
  const toggleRow = (kind: Kind) => {
    const next = { ...latest.current.hidden, [kind]: !latest.current.hidden[kind] };
    if (KINDS.every((k) => next[k])) {
      setHidden(ALL_ROWS);
      void closeShelf();
      return;
    }
    setHidden(next);
  };

  const openMenuAtCursor = () => {
    const id = cursorId(latest.current.model);
    if (id === null) return;
    const rect = document.getElementById(cardDomId(id))?.getBoundingClientRect();
    setMenu({ ids: targetIds(), at: { x: rect ? rect.left + 8 : 40, y: rect ? rect.bottom + 6 : 40 } });
  };

  /** E, M and C open an editor only for the kind they belong to. */
  const openEditor = (mode: PreviewMode) => {
    const kind = latest.current.model.cursor?.kind;
    if ((mode === 'text' && kind === 'text') || ((mode === 'markup' || mode === 'crop') && kind === 'image')) setPreview(mode);
  };

  const revert = (id: number) => {
    trackSave(revertEdit(id)).catch(fail);
  };

  /** Ctrl+Shift+C: the text Windows OCR read in the image under the cursor. */
  const copyText = async () => {
    const id = cursorId(latest.current.model);
    const item = id === null ? undefined : itemsOf([id])[0];
    if (!item || item.kind !== 'image') return;
    try {
      const outcome = await copyImageText(item.id);
      if (outcome === 'copied') {
        play('copy');
        flash([item.id], 'copied', 450);
        toastAt(item.id, t('shelf.ocr.copied'));
      } else {
        setNotice(t(outcome === 'noText' ? 'shelf.ocr.none' : 'shelf.ocr.unavailable'));
      }
    } catch (err) {
      fail(err);
    }
  };

  /** Ctrl+O: the browser comes forward, so the shelf closes on its own. */
  const openLinkOf = (id: number) => {
    if (itemsOf([id])[0]?.kind === 'link') openLink(id).catch(fail);
  };

  const refreshPreview = (id: number) => {
    refreshLink(id).then((on) => !on && setNotice(t('shelf.link.previewsOff')), fail);
  };

  /** Naming or filing pins the items; the ones not pinned yet get the pin's knock. */
  const pinEffect = (ids: number[]) => {
    const fresh = itemsOf(ids)
      .filter((item) => !item.pinned)
      .map((item) => item.id);
    if (fresh.length === 0) return;
    play('pin', reducedMotion() ? 0 : 0.21);
    flash(fresh, 'pinning', 560);
  };

  const openPicker = async (type: Picker['type'], ids: number[]) => {
    if (ids.length === 0) return;
    try {
      const groups = type === 'name' ? { folders: [], tags: [] } : await libraryGroups();
      setPicker({ type, ids, ...groups });
    } catch (err) {
      fail(err);
    }
  };

  const closePicker = () => {
    setPicker(null);
    grid.current?.focus({ preventScroll: true });
  };

  /** Ctrl+Shift+E: asks for a folder and writes the items there as files; the shelf stays open. */
  const exportSelection = async (ids: number[]) => {
    if (ids.length === 0) return;
    try {
      await whenSaved();
      const done = await exportItems(ids);
      if (done) setNotice(t('library.exported', { count: done.count, folder: done.folder }));
    } catch (err) {
      fail(err);
    }
  };

  const run = (command: Command) => {
    setNotice(null);
    const ids = targetIds();
    switch (command.type) {
      case 'preview':
        setPreview((open) => (open ? null : 'view'));
        return;
      case 'editText':
        openEditor('text');
        return;
      case 'markup':
        openEditor('markup');
        return;
      case 'crop':
        openEditor('crop');
        return;
      case 'model':
        dispatch(command.action);
        return;
      case 'copy':
        void copy(ids, cursorId(latest.current.model));
        return;
      case 'copyAndReturn':
        void copyAndReturn(ids);
        return;
      case 'paste':
        void paste(ids);
        return;
      case 'pin':
        void togglePin(ids);
        return;
      case 'takeDown':
        void remove(ids, false);
        return;
      case 'trash':
        if (ids.length > 0) setConfirm(ids);
        return;
      case 'undo':
        void undo();
        return;
      case 'pinnedOnly':
        setPinnedOnly((on) => !on);
        return;
      case 'toggleRow':
        toggleRow(command.kind);
        return;
      case 'menu':
        openMenuAtCursor();
        return;
      case 'palette':
        setPalette(paletteCommands());
        return;
      case 'name':
      case 'folder':
      case 'tag':
        void openPicker(command.type, ids);
        return;
      case 'export':
        void exportSelection(ids);
        return;
      case 'library':
        openLibrary().catch(fail);
        return;
      case 'search':
        openSearch();
        return;
      case 'copyText':
        void copyText();
        return;
      case 'openLink': {
        const id = cursorId(latest.current.model);
        if (id !== null) openLinkOf(id);
        return;
      }
      case 'escape':
        // Esc closes the top layer first: the preview, then the search, then the board.
        if (latest.current.preview) setPreview(null);
        else if (latest.current.searchOpen) closeSearch();
        else void closeShelf();
        return;
    }
  };

  const menuEntries = (ids: number[]): MenuEntry[] => {
    const items = itemsOf(ids);
    const allPinned = items.every((item) => item.pinned);
    const single = items.length === 1 ? items[0] : undefined;
    const kindEntries: MenuEntry[] = [];
    if (single?.kind === 'text') kindEntries.push({ label: 'shelf.menu.edit', shortcut: 'E', run: () => setPreview('text') });
    if (single?.kind === 'link') {
      kindEntries.push(
        { label: 'shelf.menu.openLink', shortcut: 'CommandOrControl+O', run: () => openLinkOf(single.id) },
        { label: 'shelf.menu.refreshLink', shortcut: '', run: () => refreshPreview(single.id) },
      );
    }
    if (single?.kind === 'image') {
      kindEntries.push(
        { label: 'shelf.menu.markup', shortcut: 'M', run: () => setPreview('markup') },
        { label: 'shelf.menu.crop', shortcut: 'C', run: () => setPreview('crop') },
        { label: 'shelf.menu.copyText', shortcut: 'CommandOrControl+Shift+C', run: () => void copyText() },
      );
      if (single.edited) kindEntries.push({ label: 'shelf.menu.revert', shortcut: '', run: () => revert(single.id) });
    }
    return [
      { label: 'shelf.menu.copy', shortcut: 'CommandOrControl+C', run: () => void copy(ids, ids[0] ?? null) },
      { label: 'shelf.menu.paste', shortcut: 'CommandOrControl+Enter', run: () => void paste(ids) },
      { label: 'shelf.menu.preview', shortcut: 'Space', run: () => setPreview('view') },
      ...kindEntries,
      { label: allPinned ? 'shelf.menu.unpin' : 'shelf.menu.pin', shortcut: 'P', run: () => void togglePin(ids) },
      { label: 'library.action.name', shortcut: 'Alt+P', run: () => void openPicker('name', ids) },
      { label: 'library.action.folder', shortcut: 'Shift+P', run: () => void openPicker('folder', ids) },
      { label: 'library.action.tag', shortcut: 'CommandOrControl+Shift+T', run: () => void openPicker('tag', ids) },
      { label: 'library.action.export', shortcut: 'CommandOrControl+Shift+E', run: () => void exportSelection(ids) },
      { label: 'shelf.menu.takeDown', shortcut: 'S', run: () => void remove(ids, false) },
      { label: 'shelf.menu.trash', shortcut: TRASH_SHORTCUT, run: () => setConfirm(ids) },
      { label: 'shelf.menu.undo', shortcut: 'Backspace', run: () => void undo() },
    ];
  };

  /** The palette: the card actions for the selection, then the shelf's own (D-063). */
  const paletteCommands = (): PaletteCommand[] => {
    const ids = targetIds();
    const fromMenu = ids.length > 0 ? menuEntries(ids) : [];
    const own: MenuEntry[] = [
      { label: 'shelf.search.label', shortcut: 'Shift+S', run: openSearch },
      { label: 'library.filter.pinned', shortcut: 'F', run: () => setPinnedOnly((on) => !on) },
      { label: 'palette.rowImages', shortcut: 'Shift+I', run: () => toggleRow('image') },
      { label: 'palette.rowTexts', shortcut: 'Shift+T', run: () => toggleRow('text') },
      { label: 'palette.rowLinks', shortcut: 'Shift+L', run: () => toggleRow('link') },
      { label: 'tray.library', shortcut: 'L', run: () => void openLibrary().catch(fail) },
      { label: 'palette.settings', shortcut: '', run: () => void openSettings().catch(fail) },
      { label: 'palette.closeShelf', shortcut: 'Esc', run: () => void closeShelf() },
    ];
    return [...fromMenu, ...own].map((entry, i) => ({
      id: `${i}-${entry.label}`,
      label: t(entry.label),
      shortcut: entry.shortcut || undefined,
      run: entry.run,
    }));
  };

  const handlers = useRef({ run, copy });
  useLayoutEffect(() => {
    handlers.current = { run, copy };
  });

  useEffect(() => {
    const win = getCurrentWebviewWindow();
    let settleTimer: number | undefined;
    const opened = win.listen<ShelfOpenEvent>('shelf:open', ({ payload }) => {
      window.clearTimeout(settleTimer);
      setMuted(payload.muted);
      setNow(Date.now());
      setMenu(null);
      setConfirm(null);
      setPicker(null);
      setPalette(null);
      setPreview(null);
      setNotice(payload.notice ? t(payload.notice) : null);
      setQuery('');
      setSearchOpen(payload.search);
      dispatch({ type: 'reset', rows: latest.current.rows });
      setPhase('open');
      play('open');
      grid.current?.focus({ preventScroll: true });
      if (payload.search) window.requestAnimationFrame(() => searchInput.current?.focus());
      // A new snip: its card may be newer than the rows, so they are read first.
      const focus = payload.focus;
      if (focus !== null) {
        void refresh().then((next) => {
          if (!next) return;
          const { hidden: rowsHidden, pinnedOnly: onlyPinned } = latest.current;
          dispatch({ type: 'reset', rows: rowsOf(next, rowsHidden, onlyPinned) });
          dispatch({ type: 'click', id: focus });
          if (payload.editor) setPreview(payload.editor);
        });
      }
      // Keys work at once; the mouse waits until the board has landed, then the cards settle.
      settleTimer = window.setTimeout(() => {
        setPhase('settled');
        if (!reducedMotion()) {
          const ids = KINDS.flatMap((kind) => latest.current.model.rows[kind].slice(0, 13));
          flash(ids, 'settle', 800, true);
        }
      }, reducedMotion() ? 0 : SETTLE_MS);
    });
    const closed = win.listen<ShelfCloseEvent>('shelf:close', ({ payload }) => {
      window.clearTimeout(settleTimer);
      setPhase('closing');
      setMenu(null);
      setConfirm(null);
      setPicker(null);
      setPalette(null);
      setPreview(null);
      setSearchOpen(false);
      setQuery('');
      play('close');
      // Rust hides the window only if no newer open happened in the meantime.
      window.setTimeout(() => void shelfHidden(payload.generation), reducedMotion() ? 0 : CLOSE_MS);
    });
    return () => {
      window.clearTimeout(settleTimer);
      void opened.then((unlisten) => unlisten());
      void closed.then((unlisten) => unlisten());
    };
  }, [flash, refresh]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const state = latest.current;
      // Closed boards, the menu, the confirmation, the pickers and the palette handle their own
      // keys; typing in an IME too.
      if (state.phase === 'hidden' || state.phase === 'closing' || state.menu || state.confirm || state.picker || state.palette) {
        return;
      }
      // The text, markup and crop editors handle their own keys; typing never reaches the shelf.
      if ((state.preview && state.preview !== 'view') || isEditable(event.target)) return;
      if (event.isComposing) return;
      const command = commandFor(event);
      if (!command) return;
      event.preventDefault();
      handlers.current.run(command);
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, []);

  const onCardClick = useCallback((id: number, event: MouseEvent) => {
    if (latest.current.phase !== 'settled') return;
    setNotice(null);
    if (event.ctrlKey || event.metaKey) {
      dispatch({ type: 'click', id, ctrl: true });
    } else if (event.shiftKey) {
      dispatch({ type: 'click', id, shift: true });
    } else {
      // A single click copies and the board stays open.
      dispatch({ type: 'click', id });
      void handlers.current.copy([id], id);
    }
  }, []);

  const onCardContextMenu = useCallback((id: number, event: MouseEvent) => {
    event.preventDefault();
    if (latest.current.phase !== 'settled') return;
    const selection = latest.current.model.selection;
    const ids = selection.includes(id) ? selection : [id];
    if (!selection.includes(id)) dispatch({ type: 'click', id });
    setMenu({ ids, at: { x: event.clientX, y: event.clientY } });
  }, []);

  // A double click opens the preview (the first click has already copied).
  const onCardDoubleClick = useCallback((id: number) => {
    if (latest.current.phase !== 'settled') return;
    dispatch({ type: 'click', id });
    setPreview('view');
  }, []);

  const onCardPointerDown = useCallback((id: number, event: ReactPointerEvent) => {
    const ready = DRAG_OUT && event.button === 0 && latest.current.phase === 'settled';
    pressed.current = ready ? { id, x: event.clientX, y: event.clientY } : null;
  }, []);

  // A pressed card that moves a few pixels becomes a native drag: text as text, images as files.
  // The selection goes along when the card is part of it.
  useEffect(() => {
    const onMove = (event: PointerEvent) => {
      const press = pressed.current;
      if (!press || (event.buttons & 1) === 0) return;
      if (Math.hypot(event.clientX - press.x, event.clientY - press.y) < DRAG_THRESHOLD) return;
      pressed.current = null;
      const rect = board.current?.getBoundingClientRect();
      if (!rect) return;
      const selection = latest.current.model.selection;
      const ids = selection.includes(press.id) ? selection : [press.id];
      const area = { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom };
      whenSaved()
        .then(() => startDrag(ids, area))
        .catch((err: unknown) => setNotice(t('shelf.error', { error: String(err) })));
    };
    const onUp = () => {
      pressed.current = null;
    };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    return () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
    };
  }, []);
  const onPageSize = useCallback((pageSize: number) => dispatch({ type: 'setPageSize', pageSize }), []);

  const isOpen = phase === 'open' || phase === 'settled';
  const empty = KINDS.every((kind) => shelf[kind].length === 0);
  const wrapClass = [styles.boardWrap, isOpen && styles.open, phase === 'settled' && styles.settled]
    .filter(Boolean)
    .join(' ');
  const hints = <span className={styles.engrave}>{t('shelf.hints')}</span>;

  return (
    <div
      className={`${styles.layer} ${searchOpen ? styles.searching : ''}`}
      data-edge={edge}
      onContextMenu={(event) => event.preventDefault()}
    >
      <div className={styles.backdrop} onClick={() => phase === 'settled' && void closeShelf()} />
      <div className={wrapClass}>
        <section ref={board} className={styles.board} aria-label={t('shelf.label')}>
          <div className={`${styles.frame} ${styles.frameTop}`}>{edge === 'bottom' && hints}</div>
          <div className={`${styles.frame} ${styles.frameLeft}`} />
          <div className={`${styles.frame} ${styles.frameRight}`} />
          <div className={`${styles.frame} ${styles.frameBottom}`}>{edge === 'top' && hints}</div>
          <div className={styles.cork}>
            {notice && (
              <div className={styles.notice} role="alert">
                {notice}
              </div>
            )}
            {pinnedOnly && !empty && <div className={styles.filterTag}>{t('shelf.pinnedOnly')}</div>}
            {empty ? (
              <EmptyState shelfShortcut={settings?.shortcuts.shelf ?? null} />
            ) : (
              <ShelfRows
                gridRef={grid}
                shelf={shown}
                terms={terms}
                withoutAccents={withoutAccents}
                model={model}
                hidden={hidden}
                pinnedOnly={pinnedOnly}
                effects={effects}
                now={now}
                reducedMotion={reducedMotion()}
                onPageSize={onPageSize}
                onCardClick={onCardClick}
                onCardContextMenu={onCardContextMenu}
                onCardDoubleClick={onCardDoubleClick}
                onCardPointerDown={onCardPointerDown}
              />
            )}
            {confirm && (
              <ConfirmTrash
                count={confirm.length}
                onConfirm={() => {
                  setConfirm(null);
                  void remove(confirm, true);
                }}
                onCancel={() => setConfirm(null)}
              />
            )}
          </div>
        </section>
      </div>
      <div ref={fallLayer} className={styles.fallLayer} aria-hidden="true" />
      {toasts.map((toast) => (
        <div key={toast.key} className={styles.toast} role="status" style={{ left: toast.x, top: toast.y }}>
          <CheckIcon />
          {toast.text}
        </div>
      ))}
      {preview && (phase === 'open' || phase === 'settled') && (
        <Preview
          item={current}
          mode={preview}
          onMode={setPreview}
          onClose={() => setPreview(null)}
          onUndo={() => void undo()}
          onOpenLink={openLinkOf}
          onRefreshLink={refreshPreview}
        />
      )}
      {menu && <ContextMenu at={menu.at} entries={menuEntries(menu.ids)} onClose={() => setMenu(null)} />}
      {searchOpen && isOpen && (
        <SearchBox
          inputRef={searchInput}
          query={query}
          counts={results ? { image: rows.image.length, text: rows.text.length, link: rows.link.length } : null}
          hidden={hidden}
          withoutAccents={withoutAccents}
          top={searchTop}
          onQuery={setQuery}
          onAccents={() => {
            void update({ searchWithoutAccents: !withoutAccents });
            searchInput.current?.focus();
          }}
          onResults={() => {
            dispatch({ type: 'reset', rows: latest.current.rows });
            grid.current?.focus({ preventScroll: true });
          }}
          onClose={closeSearch}
        />
      )}
      {palette && (
        <CommandPalette
          commands={palette}
          onClose={() => {
            setPalette(null);
            grid.current?.focus({ preventScroll: true });
          }}
        />
      )}
      {picker?.type === 'name' && (
        <NamePrompt
          items={itemsOf(picker.ids).map((item) => ({ id: item.id, title: item.title, label: displayTitle(item) }))}
          onSave={async (id, title) => {
            await setTitle([id], title.trim() ? title : null);
            pinEffect([id]);
            await setPinned([id], true);
          }}
          onClose={closePicker}
        />
      )}
      {picker?.type === 'folder' && (
        <FolderPicker
          folders={picker.folders}
          onCreate={(name) => createFolder(name, null)}
          onClose={closePicker}
          onPick={(folder) => {
            const ids = picker.ids;
            closePicker();
            pinEffect(ids);
            addToFolder(ids, folder)
              .then(() => setPinned(ids, true))
              .catch(fail);
          }}
        />
      )}
      {picker?.type === 'tag' && (
        <TagPicker
          tags={picker.tags}
          onClose={closePicker}
          onPick={(name) => {
            const ids = picker.ids;
            closePicker();
            tagItems(ids, name).catch(fail);
          }}
        />
      )}
    </div>
  );
}
