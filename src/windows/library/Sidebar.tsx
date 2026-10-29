import { useCallback, useEffect, useMemo, useRef, useState, type DragEvent, type KeyboardEvent, type Ref } from 'react';
import { t, type MessageKey } from '../../i18n';
import type { Folder, Tag } from '../../ipc';
import { sameSection, type Counts, type Section } from '../../library/query';
import styles from './Library.module.css';

/** The MIME type cards carry while they are dragged onto a folder. */
export const ITEMS_MIME = 'application/x-panopin-items';

export type Group = { type: 'folder' | 'tag'; id: number };

interface Entry {
  key: string;
  section: Section;
  label: string;
  count: number;
  depth: number;
  folder?: Folder;
  tag?: Tag;
  hasChildren?: boolean;
}

export interface SidebarProps {
  paneRef: Ref<HTMLDivElement>;
  section: Section;
  onSection: (section: Section) => void;
  counts: Counts;
  folders: Folder[];
  tags: Tag[];
  renaming: Group | null;
  onRenaming: (group: Group | null) => void;
  onRename: (group: Group, name: string) => void;
  onDelete: (group: Group) => void;
  onNewFolder: () => void;
  onDropItems: (folder: number, ids: number[]) => void;
  onEnter: () => void;
}

const FIXED: [Section, MessageKey, keyof Counts][] = [
  [{ type: 'all' }, 'library.section.all', 'all'],
  [{ type: 'recent' }, 'library.section.recent', 'recent'],
  [{ type: 'pinned' }, 'library.section.pinned', 'pinned'],
  [{ type: 'unsorted' }, 'library.section.unsorted', 'unsorted'],
];

/** Sections, the folder tree and tags. ↑/↓ move and select, ←/→ fold folders, F2 renames,
 * Delete removes a folder or tag (after asking). Cards dropped on a folder join it. */
export function Sidebar(props: SidebarProps) {
  const { section, counts, folders, tags, renaming, paneRef } = props;
  const [collapsed, setCollapsed] = useState<Set<number>>(new Set());
  const [dropping, setDropping] = useState<number | null>(null);
  const pane = useRef<HTMLDivElement | null>(null);
  const setPane = useCallback(
    (el: HTMLDivElement | null) => {
      pane.current = el;
      if (typeof paneRef === 'function') paneRef(el);
      else if (paneRef) paneRef.current = el;
    },
    [paneRef],
  );

  // Enter or Esc in the rename box hands the focus back to the tree (a click elsewhere keeps it).
  const refocus = useRef(false);
  useEffect(() => {
    if (!renaming && refocus.current) {
      refocus.current = false;
      pane.current?.focus();
    }
  }, [renaming]);

  const entries = useMemo(() => {
    const list: Entry[] = FIXED.map(([sec, key, count]) => ({
      key: sec.type,
      section: sec,
      label: t(key),
      count: counts[count] as number,
      depth: 0,
    }));
    const children = new Map<number | null, Folder[]>();
    for (const folder of folders) children.set(folder.parentId, [...(children.get(folder.parentId) ?? []), folder]);
    const walk = (parent: number | null, depth: number) => {
      for (const folder of children.get(parent) ?? []) {
        const hasChildren = (children.get(folder.id) ?? []).length > 0;
        list.push({
          key: `f${folder.id}`,
          section: { type: 'folder', id: folder.id },
          label: folder.name,
          count: counts.folders.get(folder.id) ?? 0,
          depth,
          folder,
          hasChildren,
        });
        if (hasChildren && !collapsed.has(folder.id) && depth < 32) walk(folder.id, depth + 1);
      }
    };
    walk(null, 0);
    for (const tag of tags) {
      list.push({ key: `t${tag.id}`, section: { type: 'tag', id: tag.id }, label: tag.name, count: counts.tags.get(tag.id) ?? 0, depth: 0, tag });
    }
    list.push({ key: 'trash', section: { type: 'trash' }, label: t('library.section.trash'), count: counts.trash, depth: 0 });
    return list;
  }, [counts, folders, tags, collapsed]);

  const current = Math.max(0, entries.findIndex((entry) => sameSection(entry.section, section)));
  const groupOf = (entry: Entry | undefined): Group | null =>
    entry?.folder ? { type: 'folder', id: entry.folder.id } : entry?.tag ? { type: 'tag', id: entry.tag.id } : null;

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (renaming) return;
    const entry = entries[current];
    const go = (index: number) => {
      const target = entries[Math.max(0, Math.min(entries.length - 1, index))];
      if (target) props.onSection(target.section);
    };
    const handled = (() => {
      switch (e.key) {
        case 'ArrowDown':
          go(current + 1);
          return true;
        case 'ArrowUp':
          go(current - 1);
          return true;
        case 'Home':
          go(0);
          return true;
        case 'End':
          go(entries.length - 1);
          return true;
        case 'ArrowRight':
          if (entry?.folder && entry.hasChildren) {
            if (collapsed.has(entry.folder.id)) setCollapsed((s) => new Set([...s].filter((id) => id !== entry.folder!.id)));
            else go(current + 1);
          }
          return true;
        case 'ArrowLeft':
          if (entry?.folder) {
            if (entry.hasChildren && !collapsed.has(entry.folder.id)) setCollapsed((s) => new Set(s).add(entry.folder!.id));
            else if (entry.folder.parentId !== null) props.onSection({ type: 'folder', id: entry.folder.parentId });
          }
          return true;
        case 'F2': {
          const group = groupOf(entry);
          if (group) props.onRenaming(group);
          return true;
        }
        case 'Delete': {
          const group = groupOf(entry);
          if (group) props.onDelete(group);
          return true;
        }
        case 'Enter':
          props.onEnter();
          return true;
        default:
          return false;
      }
    })();
    if (handled) {
      e.preventDefault();
      e.stopPropagation();
    }
  };

  const dropProps = (folder: Folder) => ({
    onDragOver: (e: DragEvent) => {
      if (!e.dataTransfer.types.includes(ITEMS_MIME)) return;
      e.preventDefault();
      e.dataTransfer.dropEffect = 'copy';
      setDropping(folder.id);
    },
    onDragLeave: () => setDropping((id) => (id === folder.id ? null : id)),
    onDrop: (e: DragEvent) => {
      e.preventDefault();
      setDropping(null);
      const ids = JSON.parse(e.dataTransfer.getData(ITEMS_MIME) || '[]') as number[];
      if (ids.length > 0) props.onDropItems(folder.id, ids);
    },
  });

  const row = (entry: Entry, index: number) => {
    const group = groupOf(entry);
    const editing = group && renaming && renaming.type === group.type && renaming.id === group.id;
    return (
      <li
        key={entry.key}
        id={`nav-${entry.key}`}
        role="treeitem"
        aria-selected={index === current}
        aria-level={entry.depth + 1}
        aria-expanded={entry.hasChildren ? !collapsed.has(entry.folder!.id) : undefined}
        className={`${styles.navItem} ${dropping !== null && entry.folder?.id === dropping ? styles.dropping : ''}`}
        style={{ ['--depth' as string]: entry.depth }}
        onClick={() => props.onSection(entry.section)}
        onDoubleClick={() => group && props.onRenaming(group)}
        {...(entry.folder ? dropProps(entry.folder) : {})}
      >
        <span className={styles.twisty} aria-hidden="true">
          {entry.hasChildren ? (collapsed.has(entry.folder!.id) ? '▸' : '▾') : entry.tag ? '#' : ''}
        </span>
        {editing ? (
          <input
            autoFocus
            defaultValue={entry.label}
            aria-label={t('library.rename')}
            onFocus={(e) => e.currentTarget.select()}
            onKeyDown={(e) => {
              e.stopPropagation();
              if (e.key === 'Enter') {
                refocus.current = true;
                props.onRename(group, e.currentTarget.value);
              } else if (e.key === 'Escape') {
                refocus.current = true;
                props.onRenaming(null);
              }
            }}
            onBlur={(e) => props.onRename(group, e.currentTarget.value)}
          />
        ) : (
          <span className={styles.label}>{entry.label}</span>
        )}
        <span className={styles.count}>{entry.count}</span>
      </li>
    );
  };

  const firstFolder = FIXED.length;
  const firstTag = entries.findIndex((entry) => entry.tag);
  const trashIndex = entries.length - 1;
  return (
    <div
      ref={setPane}
      className={`${styles.pane} ${styles.sidebar}`}
      tabIndex={0}
      role="tree"
      aria-label={t('library.sidebar')}
      aria-activedescendant={entries[current] ? `nav-${entries[current].key}` : undefined}
      onKeyDown={onKeyDown}
    >
      <ul className={styles.nav} role="group">
        {entries.slice(0, firstFolder).map((entry, i) => row(entry, i))}
      </ul>
      <div className={styles.navHead}>
        <span>{t('library.folders')}</span>
        <button type="button" title={`${t('library.folder.new')} (Ctrl+Shift+N)`} aria-label={t('library.folder.new')} onClick={props.onNewFolder}>
          +
        </button>
      </div>
      <ul className={styles.nav} role="group">
        {entries.slice(firstFolder, firstTag < 0 ? trashIndex : firstTag).map((entry, i) => row(entry, firstFolder + i))}
      </ul>
      {firstTag >= 0 && (
        <>
          <div className={styles.navHead}>
            <span>{t('library.tags')}</span>
          </div>
          <ul className={styles.nav} role="group">
            {entries.slice(firstTag, trashIndex).map((entry, i) => row(entry, firstTag + i))}
          </ul>
        </>
      )}
      <ul className={styles.nav} role="group" style={{ marginTop: 14 }}>
        {row(entries[trashIndex]!, trashIndex)}
      </ul>
    </div>
  );
}
