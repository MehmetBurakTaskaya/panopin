// Typed wrappers around the Rust commands and events. Types mirror src-tauri/src/settings.rs;
// they move to ts-rs generated bindings once the data model grows (D-013).
import { invoke } from '@tauri-apps/api/core';
import type { EditLayer } from './edit/layer';
import type { LanguageSetting, MessageKey } from './i18n';

export type Theme = 'system' | 'light' | 'dark';
export type CardSize = 's' | 'm' | 'l';
export type ShelfEdge = 'top' | 'bottom';
export type LibraryView = 'grid' | 'list';
export type SortKey = 'lastCopied' | 'added' | 'title' | 'kind' | 'tags';

/** How the Library opens: view, sorting, panel and column widths (pixels). */
export interface LibraryLayout {
  view: LibraryView;
  sort: SortKey;
  descending: boolean;
  sidebarWidth: number;
  inspectorWidth: number;
  columns: { title: number; kind: number; tags: number; added: number };
}

export interface Shortcuts {
  shelf: string | null;
  shelfSearch: string | null;
  snip: string | null;
  library: string | null;
  copyLastImage: string | null;
  pauseCapture: string | null;
}

export interface Sources {
  screenshots: boolean;
  clipboardImages: boolean;
  clipboardText: boolean;
  links: boolean;
  screenRecordings: boolean;
}

/** Days to keep unsaved items per kind and before Trash empties; `null` = forever. */
export interface Retention {
  image: number | null;
  text: number | null;
  link: number | null;
  video: number | null;
  trash: number | null;
}

/** Look of the windows; `animations` off means none, on follows Windows. */
export interface Appearance {
  systemAccent: boolean;
  material: boolean;
  animations: boolean;
}

export interface Updates {
  autoCheck: boolean;
}

export interface Settings {
  language: LanguageSetting;
  theme: Theme;
  shortcuts: Shortcuts;
  onboarded: boolean;
  sources: Sources;
  watchedFolders: string[];
  excludedApps: string[];
  retention: Retention;
  linkPreviews: boolean;
  pausedUntil: number | null;
  cardSize: CardSize;
  shelfEdge: ShelfEdge;
  sounds: Sounds;
  library: LibraryLayout;
  searchWithoutAccents: boolean;
  ocr: boolean;
  launchAtLogin: boolean;
  edgeOpen: boolean;
  appearance: Appearance;
  updates: Updates;
}

/** UI sounds; `volume` is 0–100. */
export interface Sounds {
  enabled: boolean;
  volume: number;
}

export type SettingsPatch = {
  language?: LanguageSetting;
  theme?: Theme;
  shortcuts?: Partial<Shortcuts>;
  onboarded?: boolean;
  sources?: Partial<Sources>;
  watchedFolders?: string[];
  excludedApps?: string[];
  retention?: Partial<Retention>;
  linkPreviews?: boolean;
  cardSize?: CardSize;
  shelfEdge?: ShelfEdge;
  sounds?: Partial<Sounds>;
  library?: Partial<Omit<LibraryLayout, 'columns'>> & { columns?: Partial<LibraryLayout['columns']> };
  searchWithoutAccents?: boolean;
  ocr?: boolean;
  launchAtLogin?: boolean;
  edgeOpen?: boolean;
  appearance?: Partial<Appearance>;
  updates?: Partial<Updates>;
};

export interface Counts {
  image: number;
  text: number;
  link: number;
  video: number;
}

export interface ShelfOpenEvent {
  generation: number;
  search: boolean;
  /** Sounds stay silent while a full-screen app, presentation or focus session is active. */
  muted: boolean;
  /** A message key to show on the board, e.g. why a paste did not happen. */
  notice: MessageKey | null;
  /** A card to put the cursor on and the editor to open on it (a new snip opens in crop). */
  focus: number | null;
  editor: 'crop' | null;
}

export interface ShelfCloseEvent {
  generation: number;
}

/** A row of the shelf. */
export type ShelfKind = 'image' | 'text' | 'link';
/** Screen recordings sit in the image row. */
export type ItemKind = ShelfKind | 'video';
export const rowOf = (kind: ItemKind): ShelfKind => (kind === 'video' ? 'image' : kind);

export interface ShelfItem {
  id: number;
  kind: ItemKind;
  title: string | null;
  /** First characters of a text; the full text stays in the backend. */
  preview: string | null;
  charCount: number | null;
  url: string | null;
  domain: string | null;
  width: number | null;
  height: number | null;
  hasThumb: boolean;
  pinned: boolean;
  lastCopiedAt: number;
  /** An image with an edit layer; width and height are then the edited size. */
  edited: boolean;
  /** Changes whenever the thumbnail is redrawn. */
  thumbVersion: number;
  /** A link's page title and site, once its preview has been fetched. */
  linkTitle: string | null;
  linkDescription: string | null;
  linkSite: string | null;
  linkImage: boolean;
  linkIcon: boolean;
  /** Changes whenever the link preview is fetched again. */
  linkVersion: number;
  durationMs: number | null;
}

export type Shelf = Record<ShelfKind, ShelfItem[]>;

/** A Library item: a shelf item plus everything the inspector shows. */
export interface LibraryItem extends ShelfItem {
  note: string | null;
  saved: boolean;
  createdAt: number;
  trashedAt: number | null;
  source: 'clipboard' | 'folder' | 'import' | 'manual';
  sourceApp: string | null;
  sourcePath: string | null;
  byteSize: number | null;
  folders: number[];
  tags: number[];
}

export interface Folder {
  id: number;
  parentId: number | null;
  name: string;
}

export interface Tag {
  id: number;
  name: string;
}

export interface LibraryData {
  items: LibraryItem[];
  folders: Folder[];
  tags: Tag[];
}

export interface Exported {
  count: number;
  folder: string;
}

export const getSettings = () => invoke<Settings>('get_settings');

/** A global shortcut's action, as Rust names it. */
export type ShortcutAction = 'toggleShelf' | 'openShelfSearch' | 'snip' | 'openLibrary' | 'copyLastImage' | 'pauseCapture';
/** The shortcut recorder releases Panopin's own shortcuts while it listens. */
export const suspendShortcuts = (suspended: boolean) => invoke<void>('suspend_shortcuts', { suspended });
/** Saves one shortcut (`null` = off); fails when another app holds the combination. */
export const setShortcut = (action: ShortcutAction, accelerator: string | null) =>
  invoke<Settings>('set_shortcut', { action, accelerator });
export const unavailableShortcuts = () => invoke<ShortcutAction[]>('unavailable_shortcuts');

/** Mica support and the Windows accent colour. */
export interface SystemLook {
  mica: boolean;
  accentLight: string | null;
  accentDark: string | null;
}
export const systemLook = () => invoke<SystemLook>('system_look');
export const pickWatchFolder = () => invoke<string | null>('pick_watch_folder');
export const openProjectLink = (which: 'website' | 'github' | 'licence' | 'languages') =>
  invoke<void>('open_project_link', { which });
export const appVersion = () => invoke<string>('app_version');
/** Opens the third-party licence texts installed next to Panopin. */
export const openLicenceTexts = () => invoke<void>('open_licence_texts');

/** Where the update check stands (D-065). Times are Unix milliseconds. */
export type UpdateStatus =
  | { state: 'idle' }
  | { state: 'checking' }
  | { state: 'upToDate'; checkedAt: number }
  | { state: 'available'; version: string; notes: string | null; date: number | null }
  | { state: 'downloading'; received: number; total: number | null }
  | { state: 'failed'; checkedAt: number };
export interface UpdateView {
  current: string;
  /** False for the portable build and development runs, which never replace themselves. */
  installable: boolean;
  status: UpdateStatus;
}
export const updateStatus = () => invoke<UpdateView>('update_status');
/** A check the user asked for; it runs whatever the automatic setting says. */
export const checkForUpdates = () => invoke<UpdateView>('check_for_updates');
/** Downloads and runs the installer, which restarts Panopin; the portable build opens the download page. */
export const installUpdate = () => invoke<void>('install_update');
/** The tab a new Settings window should open on, once (for example from the tray). */
export const takeSettingsTab = () => invoke<string | null>('take_settings_tab');
/** Pauses capture until the given Unix time (ms), `null` resumes. */
export const setPause = (until: number | null) => invoke<void>('set_pause', { until });

export type HistoryKind = 'image' | 'text' | 'link' | 'video';
/** "Clear history", step 1: how many unsaved items would go, and the moment that counts. */
export const countClearable = (kinds: HistoryKind[]) => invoke<{ count: number; cutoff: number }>('count_clearable', { kinds });
/** Step 2: moves what is still unsaved and older than the cutoff to Trash. */
export const clearHistory = (kinds: HistoryKind[], cutoff: number) => invoke<number>('clear_history', { kinds, cutoff });
export const updateSettings = (patch: SettingsPatch) => invoke<Settings>('update_settings', { patch });
export const closeShelf = () => invoke<void>('close_shelf');
export const shelfHidden = (generation: number) => invoke<void>('shelf_hidden', { generation });
export const listShelf = () => invoke<Shelf>('list_shelf');
export const setPinned = (ids: number[], pinned: boolean) => invoke<void>('set_pinned', { ids, pinned });
export const takeDown = (ids: number[]) => invoke<void>('take_down', { ids });
export const trashItems = (ids: number[]) => invoke<void>('trash_items', { ids });
/** Returns the ids that came back, empty when there was nothing to undo. */
export const undoTakeDown = () => invoke<number[]>('undo_take_down');
export const copyItems = (ids: number[]) => invoke<void>('copy_items', { ids });

/** Ctrl+Enter. `pasted`: the shelf closed and Ctrl+V was sent; otherwise the shelf stays open. */
export type PasteOutcome = 'pasted' | 'windowGone' | 'elevated' | 'refused';
export const pasteItems = (ids: number[]) => invoke<PasteOutcome>('paste_items', { ids });
export interface BoardRect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/** Runs a native drag of the items; letting go over `board` cancels. Resolves with whether they
 * were dropped somewhere. */
export const startDrag = (ids: number[], board: BoardRect | null, files = false) =>
  invoke<boolean>('start_drag', { ids, board, files });

export const searchShelf = (query: string) => invoke<Shelf>('search_shelf', { query });

/** Ctrl+Shift+C: what copying the text in an image did. */
export type TextCopy = 'copied' | 'noText' | 'unavailable';
export const copyImageText = (id: number) => invoke<TextCopy>('copy_image_text', { id });

export interface OcrStatus {
  supported: boolean;
  languages: { tag: string; name: string }[];
  /** The language images are read in, if any is installed. */
  reading: string | null;
}
export const ocrStatus = () => invoke<OcrStatus>('ocr_status');

/** "Refresh preview"; resolves false when link previews are off in Settings. */
export const refreshLink = (id: number) => invoke<boolean>('refresh_link', { id });
/** Ctrl+O: opens a link item in the default browser. */
export const openLink = (id: number) => invoke<void>('open_link', { id });
/** Ids of the matching items, best match first. */
export const searchLibrary = (query: string) => invoke<number[]>('search_library', { query });
export const libraryData = () => invoke<LibraryData>('library_data');
export const libraryGroups = () => invoke<Pick<LibraryData, 'folders' | 'tags'>>('library_groups');
export const setTitle = (ids: number[], title: string | null) => invoke<void>('set_title', { ids, title });
export const setNote = (id: number, note: string | null) => invoke<void>('set_note', { id, note });
export const createFolder = (name: string, parent: number | null) => invoke<number>('create_folder', { name, parent });
export const renameFolder = (id: number, name: string) => invoke<void>('rename_folder', { id, name });
export const deleteFolder = (id: number) => invoke<void>('delete_folder', { id });
export const addToFolder = (ids: number[], folder: number) => invoke<void>('add_to_folder', { ids, folder });
export const removeFromFolder = (ids: number[], folder: number) => invoke<void>('remove_from_folder', { ids, folder });
export const tagItems = (ids: number[], name: string) => invoke<number>('tag_items', { ids, name });
export const untagItems = (ids: number[], tag: number) => invoke<void>('untag_items', { ids, tag });
export const renameTag = (id: number, name: string) => invoke<void>('rename_tag', { id, name });
export const deleteTag = (id: number) => invoke<void>('delete_tag', { id });
export const restoreItems = (ids: number[]) => invoke<void>('restore_items', { ids });
export const deleteForever = (ids: number[]) => invoke<number>('delete_forever', { ids });
export const emptyTrash = () => invoke<number>('empty_trash');
export const newNote = () => invoke<number>('new_note');
/** Asks for a folder; null when the user cancels. */
export const exportItems = (ids: number[]) => invoke<Exported | null>('export_items', { ids });
export const openLibrary = () => invoke<void>('open_library');
export const openSettings = () => invoke<void>('open_settings');
/** "Import…": pictures and screen recordings; resolves with how many came in. */
export const importFiles = () => invoke<number>('import_files');

export const getText = (id: number) => invoke<string>('get_text', { id });
export const updateText = (id: number, body: string) => invoke<void>('update_text', { id, body });
export const getEdit = (id: number) => invoke<EditLayer | null>('get_edit', { id });
export const saveEdit = (id: number, layer: EditLayer) => invoke<void>('save_edit', { id, layer });
export const revertEdit = (id: number) => invoke<void>('revert_edit', { id });

/** Managed media is served by item id only (D-014); Windows maps custom schemes to http://<scheme>.localhost.
 * `version` changes when a thumbnail is redrawn, so a cached copy is not shown. */
export function mediaUrl(variant: 'thumb' | 'original' | 'link-image' | 'link-icon', id: number, version = 0): string {
  const windows = /Windows/.test(navigator.userAgent);
  const base = windows ? `http://media.localhost/${variant}/${id}` : `media://localhost/${variant}/${id}`;
  return version ? `${base}?v=${version}` : base;
}
