// What the Library shows: sidebar sections, filters and sorting. Pure, so the rules are tested;
// Rust owns the data and sends every item once (Trash included).
import type { Folder, LibraryItem, SortKey } from '../ipc';

export type Section =
  | { type: 'all' }
  | { type: 'recent' }
  | { type: 'pinned' }
  | { type: 'unsorted' }
  | { type: 'folder'; id: number }
  | { type: 'tag'; id: number }
  | { type: 'trash' };

export type KindFilter = 'all' | 'image' | 'text' | 'link';
export type DateFilter = 'any' | 'today' | 'week' | 'month' | 'older';

export interface Filters {
  kind: KindFilter;
  pinnedOnly: boolean;
  /** All of these tags (AND). */
  tags: number[];
  date: DateFilter;
  domain: string | null;
}

export const NO_FILTERS: Filters = { kind: 'all', pinnedOnly: false, tags: [], date: 'any', domain: null };

const DAY = 86_400_000;

export const sameSection = (a: Section, b: Section) =>
  a.type === b.type && ('id' in a ? a.id : null) === ('id' in b ? b.id : null);

export function inSection(item: LibraryItem, section: Section): boolean {
  if (section.type === 'trash') return item.trashedAt !== null;
  if (item.trashedAt !== null) return false;
  switch (section.type) {
    case 'all':
      return true;
    case 'recent':
      // Copies that are not saved yet and will expire.
      return !item.saved;
    case 'pinned':
      return item.pinned;
    case 'unsorted':
      // Saved, but in no folder and without tags.
      return item.saved && item.folders.length === 0 && item.tags.length === 0;
    case 'folder':
      return item.folders.includes(section.id);
    case 'tag':
      return item.tags.includes(section.id);
  }
}

function startOfDay(now: number): number {
  const day = new Date(now);
  day.setHours(0, 0, 0, 0);
  return day.getTime();
}

export function matches(item: LibraryItem, filters: Filters, now: number): boolean {
  // Screen recordings count as images, as they sit in the shelf's image row.
  const kind = item.kind === 'video' ? 'image' : item.kind;
  if (filters.kind !== 'all' && kind !== filters.kind) return false;
  if (filters.pinnedOnly && !item.pinned) return false;
  if (filters.tags.some((tag) => !item.tags.includes(tag))) return false;
  if (filters.domain !== null && item.domain !== filters.domain) return false;
  const age = now - item.lastCopiedAt;
  switch (filters.date) {
    case 'today':
      return item.lastCopiedAt >= startOfDay(now);
    case 'week':
      return age <= 7 * DAY;
    case 'month':
      return age <= 30 * DAY;
    case 'older':
      return age > 30 * DAY;
    default:
      return true;
  }
}

/** A title for lists and sorting: the given name, else the content's own first words. */
export function displayTitle(
  item: Pick<LibraryItem, 'title' | 'kind' | 'preview' | 'domain' | 'url' | 'width' | 'height' | 'linkTitle'>,
): string {
  if (item.title) return item.title;
  if (item.kind === 'text') return (item.preview ?? '').trim().split('\n')[0]?.slice(0, 120) ?? '';
  if (item.kind === 'link') return item.linkTitle ?? item.domain ?? item.url ?? '';
  return item.width && item.height ? `${item.width}×${item.height}` : '';
}

const KIND_ORDER: Record<string, number> = { image: 0, text: 1, link: 2, video: 3 };

export function sortItems(
  items: LibraryItem[],
  sort: SortKey,
  descending: boolean,
  tagName: (id: number) => string,
  lang: string,
): LibraryItem[] {
  const collator = new Intl.Collator(lang, { sensitivity: 'base', numeric: true });
  const firstTag = (item: LibraryItem) =>
    item.tags.map(tagName).sort((a, b) => collator.compare(a, b))[0] ?? null;
  const compare = (a: LibraryItem, b: LibraryItem): number => {
    switch (sort) {
      case 'added':
        return a.createdAt - b.createdAt;
      case 'title':
        return collator.compare(displayTitle(a), displayTitle(b));
      case 'kind':
        return (KIND_ORDER[a.kind] ?? 9) - (KIND_ORDER[b.kind] ?? 9);
      case 'tags': {
        const ta = firstTag(a);
        const tb = firstTag(b);
        // Untagged items always come last.
        if (ta === null || tb === null) return ta === tb ? 0 : ta === null ? (descending ? -1 : 1) : descending ? 1 : -1;
        return collator.compare(ta, tb);
      }
      default:
        return a.lastCopiedAt - b.lastCopiedAt;
    }
  };
  return [...items].sort((a, b) => {
    const order = compare(a, b);
    if (order !== 0) return descending ? -order : order;
    return b.lastCopiedAt - a.lastCopiedAt || b.id - a.id;
  });
}

export interface Counts {
  all: number;
  recent: number;
  pinned: number;
  unsorted: number;
  trash: number;
  folders: Map<number, number>;
  tags: Map<number, number>;
}

export function sectionCounts(items: LibraryItem[]): Counts {
  const counts: Counts = { all: 0, recent: 0, pinned: 0, unsorted: 0, trash: 0, folders: new Map(), tags: new Map() };
  for (const item of items) {
    if (item.trashedAt !== null) {
      counts.trash++;
      continue;
    }
    counts.all++;
    if (!item.saved) counts.recent++;
    if (item.pinned) counts.pinned++;
    if (item.saved && item.folders.length === 0 && item.tags.length === 0) counts.unsorted++;
    for (const id of item.folders) counts.folders.set(id, (counts.folders.get(id) ?? 0) + 1);
    for (const id of item.tags) counts.tags.set(id, (counts.tags.get(id) ?? 0) + 1);
  }
  return counts;
}

/**
 * Items in each folder or in any folder under it, each item once (D-083). A folded folder shows
 * this count, since its subfolders are out of sight; an open one shows its own.
 */
export function folderTotals(items: LibraryItem[], folders: Folder[]): Map<number, number> {
  const parent = new Map(folders.map((folder) => [folder.id, folder.parentId]));
  const totals = new Map<number, number>();
  for (const item of items) {
    if (item.trashedAt !== null) continue;
    const reached = new Set<number>();
    for (const id of item.folders) {
      // Up to the top; a folder reached before ends the climb, so a broken loop cannot hang.
      for (let at: number | null | undefined = id; at != null && !reached.has(at); at = parent.get(at)) reached.add(at);
    }
    for (const id of reached) totals.set(id, (totals.get(id) ?? 0) + 1);
  }
  return totals;
}

/** Domains of the links in view, for the domain filter. */
export function domainsOf(items: LibraryItem[]): string[] {
  return [...new Set(items.map((item) => item.domain).filter((d): d is string => !!d))].sort();
}
