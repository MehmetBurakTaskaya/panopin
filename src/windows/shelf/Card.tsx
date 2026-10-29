import { memo, type CSSProperties, type MouseEvent, type PointerEvent } from 'react';
import { PinIcon } from '../../components/PinIcon';
import { getLanguage, t } from '../../i18n';
import { mediaUrl, type ShelfItem } from '../../ipc';
import styles from './Card.module.css';
import { age, aspectRatio, letterTile, linkDetail, tilt } from './paper';

/** A short animation on one card; it ends by itself. */
export type Effect = 'copied' | 'pinning' | 'unpinning' | 'peeling' | 'restoring' | 'settle' | 'gone';

export interface CardProps {
  item: ShelfItem;
  cursor: boolean;
  selected: boolean;
  /** 1–9 quick-pick number, shown on the visible page of the active row. */
  badge: number | null;
  effect: Effect | null;
  /** Position in the row, for the staggered settle after opening (capped by the caller). */
  order: number;
  now: number;
  onClick: (id: number, event: MouseEvent) => void;
  onContextMenu: (id: number, event: MouseEvent) => void;
  onPointerDown: (id: number, event: PointerEvent) => void;
}

export const cardDomId = (id: number) => `card-${id}`;

function label(item: ShelfItem): string {
  let text: string;
  if (item.kind === 'image') {
    text = item.title ?? t('shelf.card.image', { size: `${item.width ?? '?'}×${item.height ?? '?'}` });
  } else if (item.kind === 'text') {
    text = t('shelf.card.text', { text: item.title ?? (item.preview ?? '').trim().split('\n')[0] ?? '' });
  } else {
    text = t('shelf.card.link', { domain: item.title ?? item.domain ?? item.url ?? '' });
  }
  return item.pinned ? `${text}, ${t('shelf.card.pinned')}` : text;
}

function charCount(count: number): string {
  const formatted = new Intl.NumberFormat(getLanguage()).format(count);
  return count === 1 ? t('shelf.card.charsOne') : t('shelf.card.chars', { count: formatted });
}

function Body({ item, now }: { item: ShelfItem; now: number }) {
  if (item.kind === 'image') {
    return (
      <div className={styles.ph}>
        {item.hasThumb && <img src={mediaUrl('thumb', item.id)} alt="" draggable={false} loading="lazy" decoding="async" />}
      </div>
    );
  }
  if (item.kind === 'text') {
    return (
      <>
        <div className={styles.txt}>{item.preview}</div>
        <div className={styles.meta}>
          <span>{charCount(item.charCount ?? 0)}</span>
          <span>{age(item.lastCopiedAt, now, getLanguage())}</span>
        </div>
      </>
    );
  }
  const tile = letterTile(item.domain);
  return (
    <div className={styles.lkBody}>
      <span className={styles.fav} style={{ background: tile.colour }} aria-hidden="true">
        {tile.letter}
      </span>
      <div className={styles.lkTitle}>{item.title ?? item.domain ?? item.url}</div>
      <span className={styles.lkDetail}>{item.title ? item.domain : linkDetail(item.url)}</span>
    </div>
  );
}

function CardView(props: CardProps) {
  const { item, cursor, selected, badge, effect, order, now, onClick, onContextMenu, onPointerDown } = props;
  const style = {
    '--r': `${tilt(item.id, 'r', 3).toFixed(2)}deg`,
    '--ta': `${tilt(item.id, 't', 6).toFixed(2)}deg`,
    '--tx': `${tilt(item.id, 'x', 8).toFixed(1)}px`,
    '--ar': aspectRatio(item.width, item.height).toFixed(3),
    '--i': order,
  } as CSSProperties;
  const slotClass = [styles.slot, cursor && styles.cursor, selected && styles.selected].filter(Boolean).join(' ');
  const cardClass = [styles.card, item.pinned && styles.pinned, effect && styles[effect]].filter(Boolean).join(' ');

  return (
    <div className={slotClass} data-id={item.id}>
      <div className={styles.lift}>
        <article
          id={cardDomId(item.id)}
          className={cardClass}
          data-kind={item.kind}
          style={style}
          role="gridcell"
          aria-selected={selected}
          aria-label={label(item)}
          onClick={(event) => onClick(item.id, event)}
          onContextMenu={(event) => onContextMenu(item.id, event)}
          onPointerDown={(event) => onPointerDown(item.id, event)}
        >
          <span className={styles.tape} aria-hidden="true">
            <i />
          </span>
          <PinIcon className={styles.pin} />
          <Body item={item} now={now} />
          {badge !== null && (
            <span className={styles.num} aria-hidden="true">
              {badge}
            </span>
          )}
        </article>
      </div>
    </div>
  );
}

export const Card = memo(CardView);
