import { useLayoutEffect, useRef, type MouseEvent, type PointerEvent, type Ref, type WheelEvent } from 'react';
import { t, type MessageKey } from '../../i18n';
import type { Shelf, ShelfItem } from '../../ipc';
import { KINDS, pageStart, type Kind, type ShelfModel } from '../../keyboard/shelfModel';
import { Card, cardDomId, type Effect } from './Card';
import styles from './Shelf.module.css';

const ROW_NAME: Record<Kind, MessageKey> = { image: 'shelf.row.image', text: 'shelf.row.text', link: 'shelf.row.link' };
/** Space kept between the track edge and a card scrolled into view. */
const SCROLL_PAD = 28;

export interface ShelfRowsProps {
  /** Takes focus on opening, so screen readers follow the cursor through aria-activedescendant. */
  gridRef: Ref<HTMLDivElement>;
  shelf: Shelf;
  /** Folded words of the shelf search, marked on the cards; null while not searching. */
  terms: string[] | null;
  withoutAccents: boolean;
  model: ShelfModel;
  hidden: Record<Kind, boolean>;
  pinnedOnly: boolean;
  effects: ReadonlyMap<number, Effect>;
  now: number;
  reducedMotion: boolean;
  onPageSize: (size: number) => void;
  onCardClick: (id: number, event: MouseEvent) => void;
  onCardContextMenu: (id: number, event: MouseEvent) => void;
  onCardDoubleClick: (id: number) => void;
  onCardPointerDown: (id: number, event: PointerEvent) => void;
}

/** How many cards of a row fit side by side from its first card, at most 9 (the page size). */
function fittingCards(track: HTMLElement): number {
  const slots = [...track.querySelectorAll<HTMLElement>('[data-id]')];
  const first = slots[0];
  if (!first) return 9;
  const limit = first.offsetLeft + track.clientWidth - 2 * SCROLL_PAD;
  const fit = slots.filter((slot) => slot.offsetLeft + slot.offsetWidth <= limit).length;
  return Math.max(1, Math.min(9, fit));
}

export function ShelfRows(props: ShelfRowsProps) {
  const { shelf, model, hidden, pinnedOnly, effects, now, reducedMotion, onPageSize } = props;
  const tracks = useRef<Partial<Record<Kind, HTMLDivElement | null>>>({});
  const cursor = model.cursor;
  const activeKind = cursor?.kind ?? null;
  const start = pageStart(model);

  // The page size follows the active row's width; it changes with the card size and the monitor.
  useLayoutEffect(() => {
    const track = activeKind ? tracks.current[activeKind] : null;
    if (!track) return;
    const measure = () => onPageSize(fittingCards(track));
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(track);
    return () => observer.disconnect();
  }, [activeKind, model.rows, onPageSize]);

  // The page holding the cursor is aligned to the left edge, so the 1–9 badges are on screen.
  useLayoutEffect(() => {
    const track = cursor ? tracks.current[cursor.kind] : null;
    if (!track || !cursor) return;
    const slots = track.querySelectorAll<HTMLElement>('[data-id]');
    const first = slots[start];
    const current = slots[cursor.index];
    if (!first || !current) return;
    let left = first.offsetLeft - SCROLL_PAD;
    const right = current.offsetLeft + current.offsetWidth + SCROLL_PAD - track.clientWidth;
    if (left < right) left = right;
    track.scrollTo({ left: Math.max(0, left), behavior: reducedMotion ? 'auto' : 'smooth' });
  }, [cursor, start, reducedMotion]);

  const onWheel = (kind: Kind) => (event: WheelEvent) => {
    const track = tracks.current[kind];
    if (track && Math.abs(event.deltaY) > Math.abs(event.deltaX)) track.scrollLeft += event.deltaY;
  };

  const activeId = cursor ? (model.rows[cursor.kind][cursor.index] ?? null) : null;
  const selection = new Set(model.selection);

  return (
    <div
      ref={props.gridRef}
      className={styles.rows}
      role="grid"
      aria-label={t('shelf.label')}
      aria-multiselectable="true"
      aria-activedescendant={activeId === null ? undefined : cardDomId(activeId)}
      tabIndex={-1}
    >
      {KINDS.filter((kind) => !hidden[kind]).map((kind) => {
        const byId = new Map(shelf[kind].map((item) => [item.id, item] as const));
        const items = model.rows[kind].map((id) => byId.get(id)).filter((item): item is ShelfItem => !!item);
        const active = cursor?.kind === kind;
        return (
          <div key={kind} className={`${styles.row} ${active ? styles.activeRow : ''}`} data-kind={kind} role="row">
            <div className={styles.label} role="rowheader">
              {t(ROW_NAME[kind])}
              <small>{items.length === 1 ? t('shelf.row.countOne') : t('shelf.row.count', { count: items.length })}</small>
            </div>
            <div
              className={styles.track}
              ref={(el) => {
                tracks.current[kind] = el;
              }}
              onWheel={onWheel(kind)}
            >
              {items.map((item, index) => (
                <Card
                  key={item.id}
                  item={item}
                  cursor={item.id === activeId}
                  selected={selection.has(item.id)}
                  badge={active && index >= start && index < start + model.pageSize ? index - start + 1 : null}
                  effect={effects.get(item.id) ?? null}
                  order={Math.min(index, 12)}
                  now={now}
                  terms={props.terms}
                  withoutAccents={props.withoutAccents}
                  onClick={props.onCardClick}
                  onContextMenu={props.onCardContextMenu}
                  onDoubleClick={props.onCardDoubleClick}
                  onPointerDown={props.onCardPointerDown}
                />
              ))}
              {items.length === 0 && (
                <p className={styles.rowEmpty}>
                  {t(props.terms ? 'shelf.search.none' : pinnedOnly ? 'shelf.row.pinnedEmpty' : 'shelf.row.empty')}
                </p>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
