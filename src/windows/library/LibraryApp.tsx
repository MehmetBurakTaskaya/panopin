import { listen } from '@tauri-apps/api/event';
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
  type DragEvent,
  type KeyboardEvent as ReactKeyboardEvent,
  type MouseEvent,
  type PointerEvent as ReactPointerEvent,
} from 'react';
import { CommandPalette, type PaletteCommand } from '../../components/pickers/CommandPalette';
import { ConfirmDialog, FolderPicker, NamePrompt, TagPicker } from '../../components/pickers/Pickers';
import { getLanguage, inlineArrow, isRtl, t, type MessageKey } from '../../i18n';
import {
  addToFolder,
  copyImageText,
  copyItems,
  createFolder,
  deleteFolder,
  deleteForever,
  deleteTag,
  emptyTrash,
  exportItems,
  importFiles,
  libraryData,
  newNote,
  openLink,
  openSettings,
  refreshLink,
  renameFolder,
  renameTag,
  restoreItems,
  searchLibrary,
  setPinned,
  setTitle,
  startDrag,
  tagItems,
  trashItems,
  undoTakeDown,
  type LibraryData,
  type LibraryLayout,
  type ShelfItem,
  type SortKey,
} from '../../ipc';
import { emptyModel, reduceLibrary, targets } from '../../keyboard/libraryModel';
import { letterOf } from '../../keyboard/shelfKeys';
import {
  NO_FILTERS,
  displayTitle,
  domainsOf,
  folderTotals,
  inSection,
  matches,
  sectionCounts,
  sortItems,
  type DateFilter,
  type Filters,
  type KindFilter,
  type Section,
} from '../../library/query';
import { queryTerms } from '../../search/highlight';
import { DRAG_OUT, TRASH_SHORTCUT, formatShortcut } from '../../shortcuts';
import { useSettings } from '../../state/settings';
import { Preview, type PreviewMode } from '../shelf/preview/Preview';
import { trackSave, whenSaved } from '../shelf/preview/saves';
import { Inspector } from './Inspector';
import { ItemGrid, ItemList } from './ItemViews';
import styles from './Library.module.css';
import { ITEMS_MIME, Sidebar, type Group } from './Sidebar';

type Dialog =
  | { type: 'folder' | 'tag' | 'name'; ids: number[] }
  | { type: 'confirm'; title: string; confirm: string; run: () => void }
  | { type: 'palette'; commands: PaletteCommand[] };

const DEFAULT_LAYOUT: LibraryLayout = {
  view: 'grid',
  sort: 'lastCopied',
  descending: true,
  sidebarWidth: 230,
  inspectorWidth: 300,
  columns: { title: 360, kind: 110, tags: 200, added: 160 },
};

const KINDS: [KindFilter, MessageKey][] = [
  ['all', 'library.kind.all'],
  ['image', 'library.kind.images'],
  ['text', 'library.kind.texts'],
  ['link', 'library.kind.links'],
];

const DATES: [DateFilter, MessageKey][] = [
  ['any', 'library.date.any'],
  ['today', 'library.date.today'],
  ['week', 'library.date.week'],
  ['month', 'library.date.month'],
  ['older', 'library.date.older'],
];

const SORTS: [string, MessageKey][] = [
  ['lastCopied:desc', 'library.sort.lastCopied'],
  ['added:desc', 'library.sort.added'],
  ['title:asc', 'library.sort.titleAsc'],
  ['title:desc', 'library.sort.titleDesc'],
  ['kind:asc', 'library.sort.kind'],
  ['tags:asc', 'library.sort.tags'],
];

const SECTION_NAME: Record<string, MessageKey> = {
  all: 'library.section.all',
  recent: 'library.section.recent',
  pinned: 'library.section.pinned',
  unsorted: 'library.section.unsorted',
  trash: 'library.section.trash',
};

const isEditable = (target: EventTarget | null) =>
  target instanceof HTMLElement && (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName));

/** The Library window: sidebar, items and inspector, all usable from the keyboard (D-053). */
export function LibraryApp() {
  useSettings((s) => s.lang);
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  const layout = settings?.library ?? DEFAULT_LAYOUT;
  const [data, setData] = useState<LibraryData>({ items: [], folders: [], tags: [] });
  const [now, setNow] = useState(() => Date.now());
  const [section, setSection] = useState<Section>({ type: 'all' });
  const [filters, setFilters] = useState<Filters>(NO_FILTERS);
  const [model, dispatch] = useReducer(reduceLibrary, undefined, () => emptyModel(1));
  const [dialog, setDialog] = useState<Dialog | null>(null);
  const [renaming, setRenaming] = useState<Group | null>(null);
  const [preview, setPreview] = useState<PreviewMode | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [widths, setWidths] = useState({ sidebar: layout.sidebarWidth, inspector: layout.inspectorWidth });
  const sidebarPane = useRef<HTMLDivElement>(null);
  const itemsPane = useRef<HTMLDivElement>(null);
  const inspectorPane = useRef<HTMLDivElement>(null);
  const toastTimer = useRef<number | undefined>(undefined);

  // A dialog gives the focus back to where it was opened from: the items, the tree or a button.
  const opener = useRef<HTMLElement | null>(null);
  const openDialog = (next: Dialog) => {
    opener.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    setDialog(next);
  };
  const closeDialog = () => {
    setDialog(null);
    (opener.current?.isConnected ? opener.current : itemsPane.current)?.focus({ preventScroll: true });
  };

  const refresh = useCallback(async () => {
    try {
      const next = await libraryData();
      // SQLite sorts names by ASCII case only; the sidebar and pickers follow the UI language.
      const collator = new Intl.Collator(getLanguage(), { sensitivity: 'base', numeric: true });
      const byName = (a: { id: number; name: string }, b: { id: number; name: string }) => collator.compare(a.name, b.name) || a.id - b.id;
      setData({ items: next.items, folders: [...next.folders].sort(byName), tags: [...next.tags].sort(byName) });
      setNow(Date.now());
    } catch {
      // The next change event tries again.
    }
  }, []);

  useEffect(() => {
    void refresh();
    const unlisten = listen('items:changed', () => void refresh());
    return () => void unlisten.then((stop) => stop());
  }, [refresh]);

  useEffect(() => setWidths({ sidebar: layout.sidebarWidth, inspector: layout.inspectorWidth }), [layout.sidebarWidth, layout.inspectorWidth]);

  const say = useCallback((text: string) => {
    setToast(text);
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 1800);
  }, []);
  const fail = useCallback((err: unknown) => say(t('shelf.error', { error: String(err) })), [say]);

  const tags = useMemo(() => new Map(data.tags.map((tag) => [tag.id, tag])), [data.tags]);
  const byId = useMemo(() => new Map(data.items.map((item) => [item.id, item])), [data.items]);
  const counts = useMemo(() => sectionCounts(data.items), [data.items]);
  const totals = useMemo(() => folderTotals(data.items, data.folders), [data.items, data.folders]);
  const inSectionItems = useMemo(() => data.items.filter((item) => inSection(item, section)), [data.items, section]);
  // Ctrl+F: Rust finds the matches (best first); the section and filters still apply (D-059).
  const [query, setQuery] = useState('');
  const [found, setFound] = useState<number[] | null>(null);
  const [byRelevance, setByRelevance] = useState(true);
  const searchBox = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (!query.trim()) {
      setFound(null);
      return;
    }
    let stale = false;
    const timer = window.setTimeout(() => {
      searchLibrary(query).then(
        (ids) => !stale && setFound(ids),
        () => {},
      );
    }, 90);
    return () => {
      stale = true;
      window.clearTimeout(timer);
    };
  }, [query, data, settings?.searchWithoutAccents]);
  const visible = useMemo(() => {
    const rank = found ? new Map(found.map((id, index) => [id, index])) : null;
    const kept = inSectionItems.filter((item) => matches(item, filters, now) && (!rank || rank.has(item.id)));
    if (rank && byRelevance) return kept.sort((a, b) => rank.get(a.id)! - rank.get(b.id)!);
    return sortItems(kept, layout.sort, layout.descending, (id) => tags.get(id)?.name ?? '', getLanguage());
  }, [inSectionItems, filters, now, layout.sort, layout.descending, tags, found, byRelevance]);
  // The searched words, marked in the tiles and the list as on the shelf (D-083).
  const withoutAccents = settings?.searchWithoutAccents ?? false;
  const terms = useMemo(() => (found ? queryTerms(query, withoutAccents) : null), [found, query, withoutAccents]);
  // The grid reports how many tiles fit in a row; the list is one column.
  const setColumns = useCallback((columns: number) => dispatch({ type: 'setColumns', columns }), []);
  useEffect(() => {
    if (layout.view === 'list') setColumns(1);
  }, [layout.view, setColumns]);

  // An item made here (a note) gets the cursor, and maybe its editor, once it is in view.
  const reveal = useRef<{ id: number; preview: PreviewMode | null } | null>(null);
  useEffect(() => {
    dispatch({ type: 'setIds', ids: visible.map((item) => item.id) });
    const next = reveal.current;
    if (next && visible.some((item) => item.id === next.id)) {
      reveal.current = null;
      dispatch({ type: 'focus', id: next.id });
      if (next.preview) setPreview(next.preview);
    }
  }, [visible]);

  const ids = targets(model);
  const selected = ids.map((id) => byId.get(id)).filter((item) => item !== undefined);
  const inTrash = section.type === 'trash';
  const currentId = model.cursor === null ? null : (model.ids[model.cursor] ?? null);
  const current = currentId === null ? null : (byId.get(currentId) ?? null);

  const saveLayout = (patch: Partial<LibraryLayout>) => void update({ library: patch });

  // Actions. Every change comes back through `items:changed`.
  const copy = async (list: number[]) => {
    if (list.length === 0) return;
    try {
      await whenSaved();
      await copyItems(list);
      say(t('shelf.copied'));
    } catch (err) {
      fail(err);
    }
  };
  const openLinkOf = (id: number) => {
    if (byId.get(id)?.kind === 'link') openLink(id).catch(fail);
  };
  const refreshPreview = (id: number) => {
    refreshLink(id).then((on) => say(t(on ? 'library.link.refreshing' : 'shelf.link.previewsOff')), fail);
  };

  /** Ctrl+Shift+C: the text Windows OCR read in the focused image. */
  const copyText = () => {
    if (current?.kind !== 'image') return;
    copyImageText(current.id).then(
      (outcome) => say(t(outcome === 'copied' ? 'shelf.ocr.copied' : outcome === 'noText' ? 'shelf.ocr.none' : 'shelf.ocr.unavailable')),
      fail,
    );
  };
  const togglePin = (list: number[]) => {
    const items = list.map((id) => byId.get(id)).filter((item) => item !== undefined);
    if (items.length > 0) setPinned(list, !items.every((item) => item.pinned)).catch(fail);
  };
  const askTrash = (list: number[]) => {
    if (list.length === 0) return;
    openDialog({
      type: 'confirm',
      title: list.length === 1 ? t('shelf.trash.confirmOne') : t('shelf.trash.confirm', { count: list.length }),
      confirm: t('shelf.trash.ok'),
      run: () => trashItems(list).then(() => say(t('library.trashed')), fail),
    });
  };
  const askDeleteForever = (list: number[]) => {
    if (list.length === 0) return;
    openDialog({
      type: 'confirm',
      title: t('library.deleteForever.confirm', { count: list.length }),
      confirm: t('library.action.deleteForever'),
      run: () => deleteForever(list).catch(fail),
    });
  };
  const askEmptyTrash = () =>
    openDialog({
      type: 'confirm',
      title: t('library.emptyTrash.confirm', { count: counts.trash }),
      confirm: t('library.emptyTrash'),
      run: () => emptyTrash().catch(fail),
    });
  const restore = (list: number[]) => {
    if (list.length > 0) restoreItems(list).then(() => say(t('library.restored')), fail);
  };
  const undo = () => {
    undoTakeDown().then((back) => back.length > 0 && say(t('library.undone')), fail);
  };
  const exportSelection = (list: number[]) => {
    if (list.length === 0) return;
    whenSaved()
      .then(() => exportItems(list))
      .then((done) => done && say(t('library.exported', { count: done.count, folder: done.folder })), fail);
  };
  const addNote = () => {
    newNote()
      .then(async (id) => {
        if (section.type === 'folder') await addToFolder([id], section.id);
        // A note is a pinned text: leave the views that cannot show it.
        if (section.type === 'trash' || section.type === 'recent' || section.type === 'tag') setSection({ type: 'all' });
        setFilters((f) =>
          (f.kind === 'all' || f.kind === 'text') && f.tags.length === 0 && f.domain === null && f.date !== 'older' ? f : NO_FILTERS,
        );
        reveal.current = { id, preview: 'text' };
        await refresh();
      })
      .catch(fail);
  };
  /** Pictures and screen recordings from files (D-058). */
  const importSome = () => {
    importFiles().then((count) => count > 0 && say(t('library.imported', { count })), fail);
  };
  const addFolder = () => {
    const parent = section.type === 'folder' ? section.id : null;
    createFolder(t('library.folder.new'), parent)
      .then(async (id) => {
        await refresh();
        setSection({ type: 'folder', id });
        setRenaming({ type: 'folder', id });
      })
      .catch(fail);
  };
  const rename = (group: Group, name: string) => {
    setRenaming(null);
    const old = group.type === 'folder' ? data.folders.find((f) => f.id === group.id)?.name : tags.get(group.id)?.name;
    if (!name.trim() || name.trim() === old) return;
    (group.type === 'folder' ? renameFolder(group.id, name) : renameTag(group.id, name)).catch(fail);
  };
  const askDeleteGroup = (group: Group) => {
    const name = group.type === 'folder' ? data.folders.find((f) => f.id === group.id)?.name : tags.get(group.id)?.name;
    openDialog({
      type: 'confirm',
      title: t(group.type === 'folder' ? 'library.folder.deleteConfirm' : 'library.tag.deleteConfirm', { name: name ?? '' }),
      confirm: t('library.delete'),
      run: () => {
        (group.type === 'folder' ? deleteFolder(group.id) : deleteTag(group.id)).catch(fail);
        setSection({ type: 'all' });
      },
    });
  };
  const openEditor = (mode: PreviewMode) => {
    if (!current) return;
    if ((mode === 'text' && current.kind === 'text') || ((mode === 'markup' || mode === 'crop') && current.kind === 'image')) setPreview(mode);
  };

  const focusPane = (step: 1 | -1) => {
    const panes = [sidebarPane.current, itemsPane.current, inspectorPane.current];
    const at = panes.findIndex((pane) => pane?.contains(document.activeElement));
    const next = panes[(at + step + panes.length) % panes.length];
    const field = next === inspectorPane.current ? next?.querySelector<HTMLElement>('input, textarea, button') : null;
    (field ?? next)?.focus();
  };

  // With its item gone, an editor falls back to the plain preview, which Esc closes.
  const previewMode = preview && (current ? preview : 'view');
  const busy = dialog !== null || (previewMode !== null && previewMode !== 'view');

  /** Ctrl+K: the actions for the selection of this moment, then the Library's own (D-063). */
  const paletteCommands = (): PaletteCommand[] => {
    const list = ids;
    const one = list.length === 1 ? byId.get(list[0]!) : undefined;
    const allPinned = selected.length > 0 && selected.every((item) => item.pinned);
    const entries: [MessageKey, string, () => void, boolean][] = [
      ['shelf.menu.copy', 'CommandOrControl+C', () => void copy(list), list.length > 0],
      ['shelf.menu.preview', 'Space', () => setPreview('view'), one !== undefined],
      ['shelf.menu.edit', 'E', () => openEditor('text'), one?.kind === 'text'],
      ['shelf.menu.markup', 'M', () => openEditor('markup'), one?.kind === 'image'],
      ['shelf.menu.crop', 'C', () => openEditor('crop'), one?.kind === 'image'],
      ['shelf.menu.copyText', 'CommandOrControl+Shift+C', copyText, one?.kind === 'image'],
      ['shelf.menu.openLink', 'CommandOrControl+O', () => one && openLinkOf(one.id), one?.kind === 'link'],
      ['shelf.menu.refreshLink', '', () => one && refreshPreview(one.id), one?.kind === 'link'],
      [allPinned ? 'shelf.menu.unpin' : 'shelf.menu.pin', 'P', () => togglePin(list), list.length > 0 && !inTrash],
      ['library.action.name', 'Alt+P', () => openDialog({ type: 'name', ids: list }), list.length > 0 && !inTrash],
      ['library.action.folder', 'Shift+P', () => openDialog({ type: 'folder', ids: list }), list.length > 0 && !inTrash],
      ['library.action.tag', 'CommandOrControl+Shift+T', () => openDialog({ type: 'tag', ids: list }), list.length > 0 && !inTrash],
      ['library.action.export', 'CommandOrControl+Shift+E', () => exportSelection(list), list.length > 0],
      ['shelf.menu.trash', TRASH_SHORTCUT, () => askTrash(list), list.length > 0 && !inTrash],
      ['library.action.restore', 'R', () => restore(list), list.length > 0 && inTrash],
      ['library.action.deleteForever', TRASH_SHORTCUT, () => askDeleteForever(list), list.length > 0 && inTrash],
      ['shelf.menu.undo', 'CommandOrControl+Z', undo, true],
      ['library.newNote', 'CommandOrControl+N', addNote, true],
      ['library.folder.new', 'CommandOrControl+Shift+N', addFolder, true],
      ['library.import', '', importSome, true],
      ['palette.searchLibrary', 'CommandOrControl+F', () => searchBox.current?.focus(), true],
      ['palette.kindAll', 'CommandOrControl+0', () => setFilters((f) => ({ ...f, kind: 'all' })), true],
      ['palette.kindImages', 'CommandOrControl+1', () => setFilters((f) => ({ ...f, kind: 'image' })), true],
      ['palette.kindTexts', 'CommandOrControl+2', () => setFilters((f) => ({ ...f, kind: 'text' })), true],
      ['palette.kindLinks', 'CommandOrControl+3', () => setFilters((f) => ({ ...f, kind: 'link' })), true],
      ['library.filter.pinned', 'F', () => setFilters((f) => ({ ...f, pinnedOnly: !f.pinnedOnly })), true],
      ['palette.gridView', '', () => saveLayout({ view: 'grid' }), true],
      ['palette.listView', '', () => saveLayout({ view: 'list' }), true],
      ['library.emptyTrash', '', askEmptyTrash, counts.trash > 0],
      ['palette.goAll', '', () => setSection({ type: 'all' }), true],
      ['palette.goRecent', '', () => setSection({ type: 'recent' }), true],
      ['palette.goPinned', '', () => setSection({ type: 'pinned' }), true],
      ['palette.goUnsorted', '', () => setSection({ type: 'unsorted' }), true],
      ['palette.goTrash', '', () => setSection({ type: 'trash' }), true],
      ['palette.settings', '', () => void openSettings().catch(fail), true],
    ];
    return entries
      .filter(([, , , enabled]) => enabled)
      .map(([label, shortcut, run], i) => ({ id: `${i}-${label}`, label: t(label), shortcut: shortcut || undefined, run }));
  };

  const onItemsKey = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    if (busy) return;
    const ctrl = e.ctrlKey || e.metaKey;
    const { shiftKey: shift, altKey: alt } = e;
    const grid = layout.view === 'grid';
    const rows = Math.max(1, Math.floor((itemsPane.current?.clientHeight ?? 400) / (grid ? 188 : 36)) - 1);
    let handled = true;
    switch (e.key) {
      case 'ArrowLeft':
      case 'ArrowRight':
        if (grid) dispatch({ type: 'move', dx: inlineArrow(e.key) === 'ArrowLeft' ? -1 : 1, dy: 0, extend: shift });
        else handled = false;
        break;
      case 'ArrowUp':
      case 'ArrowDown':
        dispatch({ type: 'move', dx: 0, dy: e.key === 'ArrowUp' ? -1 : 1, extend: shift });
        break;
      case 'Home':
      case 'End':
        dispatch({ type: 'edge', end: e.key === 'Home' ? 'first' : 'last', extend: shift });
        break;
      case 'PageUp':
      case 'PageDown':
        dispatch({ type: 'page', dir: e.key === 'PageUp' ? -1 : 1, rows, extend: shift });
        break;
      case 'Enter':
        // Copying from the Library keeps the window open.
        void copy(ids);
        break;
      case ' ':
        setPreview((open) => (open ? null : 'view'));
        break;
      case 'Delete':
        if (inTrash) askDeleteForever(ids);
        else askTrash(ids);
        break;
      case 'Backspace':
        // ⌘⌫ does the same, as in Finder: a Mac's Delete key needs fn.
        if (e.metaKey && !e.ctrlKey && !alt) {
          if (inTrash) askDeleteForever(ids);
          else askTrash(ids);
        } else handled = false;
        break;
      case 'Escape':
        if (preview) setPreview(null);
        else handled = false;
        break;
      default:
        handled = false;
    }
    const letter = handled ? null : letterOf(e.nativeEvent);
    if (letter) {
      handled = true;
      if (ctrl && !alt && !shift && letter === 'a') dispatch({ type: 'selectAll' });
      else if (ctrl && !alt && !shift && letter === 'c') void copy(ids);
      else if (ctrl && shift && !alt && letter === 't') openDialog({ type: 'tag', ids });
      else if (ctrl && shift && !alt && letter === 'c') copyText();
      else if (ctrl && !shift && !alt && letter === 'o' && current) openLinkOf(current.id);
      else if (alt && !ctrl && !shift && letter === 'p') openDialog({ type: 'name', ids });
      else if (shift && !ctrl && !alt && letter === 'p') openDialog({ type: 'folder', ids });
      else if (!ctrl && !alt && !shift && letter === 'p') togglePin(ids);
      else if (!ctrl && !alt && !shift && letter === 'f') setFilters((f) => ({ ...f, pinnedOnly: !f.pinnedOnly }));
      else if (!ctrl && !alt && !shift && letter === 'e') openEditor('text');
      else if (!ctrl && !alt && !shift && letter === 'm') openEditor('markup');
      else if (!ctrl && !alt && !shift && letter === 'c') openEditor('crop');
      else if (!ctrl && !alt && !shift && letter === 'r' && inTrash) restore(ids);
      else handled = false;
    }
    if (handled && ids.length === 0 && ['Enter', 'Delete'].includes(e.key)) handled = true;
    if (handled) {
      e.preventDefault();
      e.stopPropagation();
    }
  };

  // Keys that work from every pane: panes, filters, new note or folder, export, undo.
  const onWindowKey = (e: KeyboardEvent) => {
    if (busy) return;
    if (e.key === 'F6') {
      e.preventDefault();
      focusPane(e.shiftKey ? -1 : 1);
      return;
    }
    if (e.key === 'Escape' && previewMode === 'view') {
      // The preview was clicked, so the items pane no longer has the focus.
      setPreview(null);
      return;
    }
    const ctrl = e.ctrlKey || e.metaKey;
    // Ctrl+K works from anywhere in the window, the search box included.
    if (ctrl && !e.shiftKey && !e.altKey && letterOf(e) === 'k') {
      e.preventDefault();
      openDialog({ type: 'palette', commands: paletteCommands() });
      return;
    }
    if (isEditable(e.target)) return;
    const letter = letterOf(e);
    const digit = /^Digit([0-3])$/.exec(e.code)?.[1];
    let handled = true;
    if (ctrl && !e.shiftKey && !e.altKey && letter === 'f') {
      searchBox.current?.focus();
      searchBox.current?.select();
    } else if (ctrl && !e.shiftKey && !e.altKey && digit !== undefined) setFilters((f) => ({ ...f, kind: KINDS[Number(digit)]![0] }));
    else if (ctrl && !e.altKey && letter === 'n') {
      if (e.shiftKey) addFolder();
      else addNote();
    } else if (ctrl && e.shiftKey && !e.altKey && letter === 'e') exportSelection(ids);
    else if ((ctrl && !e.shiftKey && letter === 'z') || (e.key === 'Backspace' && !ctrl && !e.altKey)) undo();
    else handled = false;
    if (handled) e.preventDefault();
  };
  const windowKey = useRef(onWindowKey);
  useLayoutEffect(() => {
    windowKey.current = onWindowKey;
  });
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => windowKey.current(e);
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  // The items take the focus when the window opens and whenever a dialog or editor closing
  // leaves nothing focused.
  useEffect(() => {
    if (!document.activeElement || document.activeElement === document.body) itemsPane.current?.focus({ preventScroll: true });
  }, [dialog, preview, renaming]);

  const onItemClick = (id: number, e: MouseEvent) => {
    dispatch({ type: 'click', id, ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey });
    itemsPane.current?.focus({ preventScroll: true });
  };
  const onDragStart = (id: number, e: DragEvent) => {
    const list = model.selection.includes(id) ? model.selection : [id];
    if (e.altKey && DRAG_OUT) {
      // Alt+drag hands the items out as files (.txt, .url, .png).
      e.preventDefault();
      whenSaved()
        .then(() => startDrag(list, null, true))
        .catch(fail);
      return;
    }
    e.dataTransfer.setData(ITEMS_MIME, JSON.stringify(list));
    e.dataTransfer.effectAllowed = 'copy';
  };
  const dropOnFolder = (folder: number, list: number[]) => {
    // Joining a folder leaves the pin as it is.
    addToFolder(list, folder).then(() => say(t('library.added')), fail);
  };

  const splitter = (side: 'sidebar' | 'inspector') => (e: ReactPointerEvent<HTMLDivElement>) => {
    e.preventDefault();
    const startX = e.clientX;
    const start = widths[side];
    // The sidebar sits at the start edge and the inspector at the end: left and right, or mirrored.
    const toEnd = (x: number) => (isRtl() ? startX - x : x - startX);
    const widthAt = (x: number) => Math.max(160, Math.min(560, start + (side === 'sidebar' ? toEnd(x) : -toEnd(x))));
    const onMove = (ev: PointerEvent) => setWidths((w) => ({ ...w, [side]: widthAt(ev.clientX) }));
    const onUp = (ev: PointerEvent) => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
      saveLayout(side === 'sidebar' ? { sidebarWidth: widthAt(ev.clientX) } : { inspectorWidth: widthAt(ev.clientX) });
    };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
  };

  const heading =
    section.type === 'folder'
      ? (data.folders.find((f) => f.id === section.id)?.name ?? '')
      : section.type === 'tag'
        ? `# ${tags.get(section.id)?.name ?? ''}`
        : t(SECTION_NAME[section.type]!);
  const domains = useMemo(() => domainsOf(inSectionItems), [inSectionItems]);
  const sortValue = found && byRelevance ? 'relevance' : `${layout.sort}:${layout.descending ? 'desc' : 'asc'}`;
  const emptyText = found
    ? t('library.empty.search')
    : inTrash
      ? t('library.empty.trash')
      : inSectionItems.length === 0
        ? t('library.empty.section')
        : t('library.empty.filters');

  const viewProps = {
    paneRef: itemsPane,
    items: visible,
    model,
    tags,
    emptyText,
    terms,
    withoutAccents,
    onKeyDown: onItemsKey,
    onClick: onItemClick,
    onDoubleClick: (id: number) => {
      dispatch({ type: 'focus', id });
      setPreview('view');
    },
    onDragStart,
  };

  return (
    <div className={styles.app} style={{ ['--sidebar' as string]: `${widths.sidebar}px`, ['--inspector' as string]: `${widths.inspector}px` }}>
      <Sidebar
        paneRef={sidebarPane}
        section={section}
        onSection={(next) => {
          setSection(next);
          setPreview(null);
        }}
        counts={counts}
        folderTotals={totals}
        folders={data.folders}
        tags={data.tags}
        renaming={renaming}
        onRenaming={setRenaming}
        onRename={rename}
        onDelete={askDeleteGroup}
        onNewFolder={addFolder}
        onDropItems={dropOnFolder}
        onEnter={() => itemsPane.current?.focus()}
      />
      <div className={styles.splitter} role="separator" aria-orientation="vertical" onPointerDown={splitter('sidebar')} />
      <main className={styles.pane}>
        <div className={styles.toolbar}>
          <h1 className={styles.heading}>
            {heading}
            <small>{visible.length === 1 ? t('shelf.row.countOne') : t('shelf.row.count', { count: visible.length })}</small>
          </h1>
          <input
            ref={searchBox}
            className={styles.search}
            type="search"
            value={query}
            placeholder={`${t('library.search')} (${formatShortcut('CommandOrControl+F')})`}
            aria-label={t('library.search')}
            spellCheck={false}
            onChange={(e) => {
              if (!query.trim()) setByRelevance(true);
              setQuery(e.target.value);
            }}
            onKeyDown={(e) => {
              if (e.key === 'Escape') {
                e.preventDefault();
                e.stopPropagation();
                // Esc clears first, then hands the keys back to the items.
                if (query) setQuery('');
                else itemsPane.current?.focus();
              } else if (e.key === 'Enter' || e.key === 'ArrowDown') {
                e.preventDefault();
                dispatch({ type: 'edge', end: 'first' });
                itemsPane.current?.focus();
              }
            }}
          />
          <div className={styles.segment} role="group" aria-label={t('library.filter.kind')}>
            {KINDS.map(([kind, label], i) => (
              <button
                key={kind}
                type="button"
                className={styles.tool}
                aria-pressed={filters.kind === kind}
                title={`${t(label)} (${formatShortcut(`CommandOrControl+${i}`)})`}
                onClick={() => setFilters((f) => ({ ...f, kind }))}
              >
                {t(label)}
              </button>
            ))}
          </div>
          <button
            type="button"
            className={styles.tool}
            aria-pressed={filters.pinnedOnly}
            title={`${t('library.filter.pinned')} (F)`}
            onClick={() => setFilters((f) => ({ ...f, pinnedOnly: !f.pinnedOnly }))}
          >
            {t('library.filter.pinned')}
          </button>
          {data.tags.length > 0 && (
            <select
              className={styles.select}
              aria-label={t('library.filter.tags')}
              value=""
              onChange={(e) => {
                const id = Number(e.target.value);
                if (id) setFilters((f) => ({ ...f, tags: f.tags.includes(id) ? f.tags : [...f.tags, id] }));
              }}
            >
              <option value="">{t('library.filter.tags')}</option>
              {data.tags
                .filter((tag) => !filters.tags.includes(tag.id))
                .map((tag) => (
                  <option key={tag.id} value={tag.id}>
                    # {tag.name}
                  </option>
                ))}
            </select>
          )}
          {filters.tags.map((id) => (
            <span key={id} className={styles.chip}>
              # {tags.get(id)?.name}
              <button
                type="button"
                aria-label={t('library.inspector.removeTag', { name: tags.get(id)?.name ?? '' })}
                onClick={() => setFilters((f) => ({ ...f, tags: f.tags.filter((tag) => tag !== id) }))}
              >
                ×
              </button>
            </span>
          ))}
          <select
            className={styles.select}
            aria-label={t('library.filter.date')}
            value={filters.date}
            onChange={(e) => setFilters((f) => ({ ...f, date: e.target.value as DateFilter }))}
          >
            {DATES.map(([value, label]) => (
              <option key={value} value={value}>
                {t(label)}
              </option>
            ))}
          </select>
          {domains.length > 0 && (
            <select
              className={styles.select}
              aria-label={t('library.filter.domain')}
              value={filters.domain ?? ''}
              onChange={(e) => setFilters((f) => ({ ...f, domain: e.target.value || null }))}
            >
              <option value="">{t('library.filter.anyDomain')}</option>
              {domains.map((domain) => (
                <option key={domain} value={domain}>
                  {domain}
                </option>
              ))}
            </select>
          )}
          <select
            className={styles.select}
            aria-label={t('library.sort')}
            value={sortValue}
            onChange={(e) => {
              if (e.target.value === 'relevance') {
                setByRelevance(true);
                return;
              }
              setByRelevance(false);
              const [sort, dir] = e.target.value.split(':');
              saveLayout({ sort: sort as SortKey, descending: dir === 'desc' });
            }}
          >
            {found && <option value="relevance">{t('library.sort.relevance')}</option>}
            {SORTS.map(([value, label]) => (
              <option key={value} value={value}>
                {t(label)}
              </option>
            ))}
          </select>
          <div className={styles.segment} role="group" aria-label={t('library.view')}>
            <button type="button" className={styles.tool} aria-pressed={layout.view === 'grid'} onClick={() => saveLayout({ view: 'grid' })}>
              {t('library.view.grid')}
            </button>
            <button type="button" className={styles.tool} aria-pressed={layout.view === 'list'} onClick={() => saveLayout({ view: 'list' })}>
              {t('library.view.list')}
            </button>
          </div>
          {inTrash ? (
            <button type="button" className={styles.tool} disabled={counts.trash === 0} onClick={askEmptyTrash}>
              {t('library.emptyTrash')}
            </button>
          ) : (
            <>
              <button type="button" className={styles.tool} title={`${t('library.newNote')} (${formatShortcut('CommandOrControl+N')})`} onClick={addNote}>
                + {t('library.newNote')}
              </button>
              <button type="button" className={styles.tool} onClick={importSome}>
                {t('library.import')}
              </button>
            </>
          )}
        </div>
        {layout.view === 'grid' ? (
          <ItemGrid {...viewProps} onColumns={setColumns} />
        ) : (
          <ItemList
            {...viewProps}
            layout={layout}
            onSort={(sort, descending) => saveLayout({ sort, descending })}
            onColumnWidth={(key, width) => saveLayout({ columns: { ...layout.columns, [key]: width } })}
          />
        )}
      </main>
      <div className={styles.splitter} role="separator" aria-orientation="vertical" onPointerDown={splitter('inspector')} />
      <Inspector
        paneRef={inspectorPane}
        items={selected}
        folders={data.folders}
        tags={tags}
        inTrash={inTrash}
        onName={() => openDialog({ type: 'name', ids })}
        onFolder={() => openDialog({ type: 'folder', ids })}
        onTag={() => openDialog({ type: 'tag', ids })}
        onPin={() => togglePin(ids)}
        onExport={() => exportSelection(ids)}
        onTrash={() => askTrash(ids)}
        onRestore={() => restore(ids)}
        onDeleteForever={() => askDeleteForever(ids)}
        onOpenLink={openLinkOf}
        onRefreshLink={refreshPreview}
      />
      {preview && (
        <Preview
          item={current && current.kind !== 'video' ? (current as unknown as ShelfItem) : null}
          mode={previewMode ?? 'view'}
          onMode={setPreview}
          onClose={() => {
            setPreview(null);
            itemsPane.current?.focus();
          }}
          onUndo={undo}
          onOpenLink={openLinkOf}
          onRefreshLink={refreshPreview}
        />
      )}
      {dialog?.type === 'palette' && <CommandPalette commands={dialog.commands} onClose={closeDialog} />}
      {dialog?.type === 'folder' && (
        <FolderPicker
          folders={data.folders}
          onCreate={(name) => createFolder(name, null)}
          onClose={closeDialog}
          onPick={(folder) => {
            const list = dialog.ids;
            closeDialog();
            trackSave(addToFolder(list, folder).then(() => setPinned(list, true))).catch(fail);
          }}
        />
      )}
      {dialog?.type === 'tag' && (
        <TagPicker
          tags={data.tags}
          onClose={closeDialog}
          onPick={(name) => {
            const list = dialog.ids;
            closeDialog();
            tagItems(list, name).catch(fail);
          }}
        />
      )}
      {dialog?.type === 'name' && (
        <NamePrompt
          items={dialog.ids.map((id) => {
            const item = byId.get(id);
            return { id, title: item?.title ?? null, label: item ? displayTitle(item) : '' };
          })}
          onSave={async (id, title) => {
            await setTitle([id], title.trim() ? title : null);
            await setPinned([id], true);
          }}
          onClose={() => {
            closeDialog();
          }}
        />
      )}
      {dialog?.type === 'confirm' && (
        <ConfirmDialog
          title={dialog.title}
          confirm={dialog.confirm}
          onClose={() => {
            closeDialog();
          }}
          onConfirm={() => {
            dialog.run();
            closeDialog();
          }}
        />
      )}
      {toast && (
        <div className={styles.toast} role="status">
          {toast}
        </div>
      )}
    </div>
  );
}
