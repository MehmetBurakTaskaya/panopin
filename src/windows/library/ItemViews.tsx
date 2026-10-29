import { useVirtualizer } from '@tanstack/react-virtual';
import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type DragEvent,
  type KeyboardEvent,
  type MouseEvent,
  type PointerEvent as ReactPointerEvent,
  type Ref,
} from 'react';
import { PinIcon } from '../../components/PinIcon';
import { getLanguage, t, type MessageKey } from '../../i18n';
import { mediaUrl, type LibraryItem, type LibraryLayout, type SortKey, type Tag } from '../../ipc';
import type { LibraryModel } from '../../keyboard/libraryModel';
import { displayTitle } from '../../library/query';
import { letterTile } from '../shelf/paper';
import styles from './Library.module.css';

const TILE_W = 172;
const TILE_H = 176;
const GAP = 12;
const ROW_H = 36;

export interface ItemViewProps {
  paneRef: Ref<HTMLDivElement>;
  items: LibraryItem[];
  model: LibraryModel;
  tags: Map<number, Tag>;
  emptyText: string;
  onKeyDown: (e: KeyboardEvent<HTMLDivElement>) => void;
  onClick: (id: number, e: MouseEvent) => void;
  onDoubleClick: (id: number) => void;
  onDragStart: (id: number, e: DragEvent) => void;
}

export const itemDomId = (id: number) => `lib-item-${id}`;

function useWidth(ref: { current: HTMLElement | null }) {
  const [width, setWidth] = useState(0);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const measure = () => setWidth(el.clientWidth);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, [ref]);
  return width;
}

function setRefs<T>(own: { current: T | null }, outer: Ref<T>) {
  return (el: T | null) => {
    own.current = el;
    if (typeof outer === 'function') outer(el);
    else if (outer) (outer as { current: T | null }).current = el;
  };
}

function Visual({ item }: { item: LibraryItem }) {
  if (item.kind === 'image' && item.hasThumb) {
    return <img src={mediaUrl('thumb', item.id, item.thumbVersion)} alt="" loading="lazy" decoding="async" draggable={false} />;
  }
  if (item.kind === 'text') return <div className={styles.thumbText}>{(item.preview ?? '').slice(0, 260)}</div>;
  const tile = letterTile(item.domain);
  return (
    <div className={styles.thumbLink}>
      <span className={styles.letter} style={{ background: tile.colour }} aria-hidden="true">
        {tile.letter}
      </span>
      <span>{item.domain}</span>
    </div>
  );
}

const KIND_LABEL: Record<string, MessageKey> = {
  image: 'library.kind.image',
  text: 'library.kind.text',
  link: 'library.kind.link',
  video: 'library.kind.video',
};

function itemLabel(item: LibraryItem): string {
  return `${t(KIND_LABEL[item.kind] ?? 'library.kind.text')}: ${displayTitle(item)}${item.pinned ? `, ${t('shelf.card.pinned')}` : ''}`;
}

/** Grid view: tiles in rows, only the visible rows are rendered (10 000 items stay smooth). */
export function ItemGrid(props: ItemViewProps & { onColumns: (columns: number) => void }) {
  const { items, model, onColumns } = props;
  const box = useRef<HTMLDivElement | null>(null);
  const width = useWidth(box);
  const columns = Math.max(1, Math.floor((width - GAP) / (TILE_W + GAP)));
  const rows = Math.ceil(items.length / columns);
  const virtualizer = useVirtualizer({
    count: rows,
    getScrollElement: () => box.current,
    estimateSize: () => TILE_H + GAP,
    overscan: 3,
  });
  useEffect(() => onColumns(columns), [columns, onColumns]);
  useEffect(() => {
    if (model.cursor !== null) virtualizer.scrollToIndex(Math.floor(model.cursor / columns), { align: 'auto' });
  }, [model.cursor, columns, virtualizer]);

  const selection = new Set(model.selection);
  const cursorId = model.cursor === null ? null : model.ids[model.cursor];
  return (
    <div
      ref={setRefs(box, props.paneRef)}
      className={`${styles.pane} ${styles.items}`}
      tabIndex={0}
      role="grid"
      aria-label={t('library.items')}
      aria-multiselectable="true"
      aria-activedescendant={cursorId == null ? undefined : itemDomId(cursorId)}
      onKeyDown={props.onKeyDown}
    >
      {items.length === 0 && <div className={styles.empty}>{props.emptyText}</div>}
      <div style={{ position: 'relative', height: virtualizer.getTotalSize() + GAP }}>
        {virtualizer.getVirtualItems().map((row) =>
          items.slice(row.index * columns, row.index * columns + columns).map((item, column) => (
            <div
              key={item.id}
              id={itemDomId(item.id)}
              role="gridcell"
              aria-selected={selection.has(item.id)}
              aria-label={itemLabel(item)}
              className={`${styles.tile} ${item.id === cursorId ? styles.cursor : ''}`}
              style={{ top: row.start + GAP, left: GAP + column * (TILE_W + GAP), width: TILE_W, height: TILE_H }}
              draggable
              onClick={(e) => props.onClick(item.id, e)}
              onDoubleClick={() => props.onDoubleClick(item.id)}
              onDragStart={(e) => props.onDragStart(item.id, e)}
            >
              <div className={styles.thumb}>
                <Visual item={item} />
              </div>
              <div className={styles.caption}>
                <span>{displayTitle(item) || '—'}</span>
                {item.pinned && <PinIcon className={styles.pin} />}
              </div>
            </div>
          )),
        )}
      </div>
    </div>
  );
}

const COLUMNS: { key: keyof LibraryLayout['columns']; sort: SortKey; label: MessageKey }[] = [
  { key: 'title', sort: 'title', label: 'library.column.title' },
  { key: 'kind', sort: 'kind', label: 'library.column.kind' },
  { key: 'tags', sort: 'tags', label: 'library.column.tags' },
  { key: 'added', sort: 'added', label: 'library.column.added' },
];

/** List view: sortable, resizable columns (kept in settings), virtualised rows. */
export function ItemList(
  props: ItemViewProps & {
    layout: LibraryLayout;
    onSort: (sort: SortKey, descending: boolean) => void;
    onColumnWidth: (key: keyof LibraryLayout['columns'], width: number) => void;
  },
) {
  const { items, model, layout, tags } = props;
  const box = useRef<HTMLDivElement | null>(null);
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => box.current,
    estimateSize: () => ROW_H,
    overscan: 8,
  });
  useEffect(() => {
    if (model.cursor !== null) virtualizer.scrollToIndex(model.cursor, { align: 'auto' });
  }, [model.cursor, virtualizer]);
  const [widths, setWidths] = useState(layout.columns);
  useEffect(() => setWidths(layout.columns), [layout.columns]);

  const resize = (key: keyof LibraryLayout['columns']) => (e: ReactPointerEvent<HTMLSpanElement>) => {
    e.preventDefault();
    e.stopPropagation();
    const startX = e.clientX;
    const start = widths[key];
    const onMove = (ev: PointerEvent) => setWidths((w) => ({ ...w, [key]: Math.max(60, Math.min(1600, start + ev.clientX - startX)) }));
    const onUp = (ev: PointerEvent) => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
      props.onColumnWidth(key, Math.max(60, Math.min(1600, start + ev.clientX - startX)));
    };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
  };

  const date = new Intl.DateTimeFormat(getLanguage(), { dateStyle: 'medium', timeStyle: 'short' });
  const selection = new Set(model.selection);
  const cursorId = model.cursor === null ? null : model.ids[model.cursor];
  const totalWidth = COLUMNS.reduce((sum, c) => sum + widths[c.key], 0) + 40;

  return (
    <div
      ref={setRefs(box, props.paneRef)}
      className={`${styles.pane} ${styles.items}`}
      tabIndex={0}
      role="grid"
      aria-label={t('library.items')}
      aria-multiselectable="true"
      aria-activedescendant={cursorId == null ? undefined : itemDomId(cursorId)}
      onKeyDown={props.onKeyDown}
    >
      <div className={styles.listHead} role="row" style={{ width: totalWidth }}>
        <span className={styles.col} style={{ width: 40 }} />
        {COLUMNS.map((column) => (
          <span
            key={column.key}
            role="columnheader"
            aria-sort={layout.sort === column.sort ? (layout.descending ? 'descending' : 'ascending') : 'none'}
            className={styles.col}
            style={{ width: widths[column.key] }}
            onClick={() => props.onSort(column.sort, layout.sort === column.sort ? !layout.descending : column.sort === 'added')}
          >
            {t(column.label)}
            {layout.sort === column.sort ? (layout.descending ? ' ↓' : ' ↑') : ''}
            <span className={styles.resizer} onPointerDown={resize(column.key)} onClick={(e) => e.stopPropagation()} />
          </span>
        ))}
      </div>
      {items.length === 0 && <div className={styles.empty}>{props.emptyText}</div>}
      <div style={{ position: 'relative', height: virtualizer.getTotalSize(), width: totalWidth }}>
        {virtualizer.getVirtualItems().map((row) => {
          const item = items[row.index]!;
          const tile = letterTile(item.domain);
          return (
            <div
              key={item.id}
              id={itemDomId(item.id)}
              role="row"
              aria-selected={selection.has(item.id)}
              aria-label={itemLabel(item)}
              className={`${styles.row} ${item.id === cursorId ? styles.cursor : ''}`}
              style={{ top: row.start }}
              draggable
              onClick={(e) => props.onClick(item.id, e)}
              onDoubleClick={() => props.onDoubleClick(item.id)}
              onDragStart={(e) => props.onDragStart(item.id, e)}
            >
              <span className={styles.col} style={{ width: 40 }}>
                {item.kind === 'image' && item.hasThumb ? (
                  <img className={styles.rowThumb} src={mediaUrl('thumb', item.id, item.thumbVersion)} alt="" loading="lazy" draggable={false} />
                ) : item.kind === 'link' ? (
                  <span className={styles.letter} style={{ background: tile.colour }} aria-hidden="true">
                    {tile.letter}
                  </span>
                ) : null}
              </span>
              <span className={styles.col} role="gridcell" style={{ width: widths.title }}>
                {item.pinned && <PinIcon className={styles.pin} />}
                {displayTitle(item) || '—'}
              </span>
              <span className={styles.col} role="gridcell" style={{ width: widths.kind }}>
                {t(KIND_LABEL[item.kind] ?? 'library.kind.text')}
              </span>
              <span className={styles.col} role="gridcell" style={{ width: widths.tags }}>
                {item.tags.map((id) => tags.get(id)?.name).filter(Boolean).join(', ')}
              </span>
              <span className={styles.col} role="gridcell" style={{ width: widths.added }}>
                {date.format(item.createdAt)}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
