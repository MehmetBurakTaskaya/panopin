import { memo, type CSSProperties, type MouseEvent, type PointerEvent } from 'react';
import { Marked } from '../../components/Marked';
import { PinIcon } from '../../components/PinIcon';
import { formatLocale, ltr, t } from '../../i18n';
import { mediaUrl, type ShelfItem } from '../../ipc';
import { looksLikeCode } from '../../text/code';
import styles from './Card.module.css';
import { age, aspectRatio, duration, isVideoLink, letterTile, linkDetail, linkHeading, tilt } from './paper';

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
  /** Words of the shelf search to mark; null while not searching. */
  terms: string[] | null;
  withoutAccents: boolean;
  onClick: (id: number, event: MouseEvent) => void;
  onContextMenu: (id: number, event: MouseEvent) => void;
  onDoubleClick: (id: number) => void;
  onPointerDown: (id: number, event: PointerEvent) => void;
}

export const cardDomId = (id: number) => `card-${id}`;

function label(item: ShelfItem): string {
  let text: string;
  if (item.kind === 'image') {
    text = item.title ?? t('shelf.card.image', { size: ltr(`${item.width ?? '?'}×${item.height ?? '?'}`) });
  } else if (item.kind === 'video') {
    text = t('shelf.card.video', { title: item.title ?? '', length: duration(item.durationMs) });
  } else if (item.kind === 'text') {
    text = t('shelf.card.text', { text: item.title ?? (item.preview ?? '').trim().split('\n')[0] ?? '' });
  } else {
    text = t('shelf.card.link', { domain: linkHeading(item) });
  }
  return item.pinned ? `${text}, ${t('shelf.card.pinned')}` : text;
}

function charCount(count: number): string {
  const formatted = new Intl.NumberFormat(formatLocale()).format(count);
  return count === 1 ? t('shelf.card.charsOne') : t('shelf.card.chars', { count: formatted });
}

interface BodyProps {
  item: ShelfItem;
  now: number;
  terms: string[] | null;
  withoutAccents: boolean;
}

function Body({ item, now, terms, withoutAccents }: BodyProps) {
  if (item.kind === 'image' || item.kind === 'video') {
    return (
      <div className={styles.ph}>
        {item.hasThumb && (
          <img src={mediaUrl('thumb', item.id, item.thumbVersion)} alt="" draggable={false} loading="lazy" decoding="async" />
        )}
        {item.kind === 'video' && (
          <>
            <span className={styles.play} aria-hidden="true" />
            <span className={styles.duration}>{duration(item.durationMs)}</span>
          </>
        )}
      </div>
    );
  }
  if (item.kind === 'text') {
    return (
      <>
        <div className={styles.txt} dir="auto" data-code={looksLikeCode(item.preview ?? '') || undefined}>
          <Marked text={item.preview ?? ''} terms={terms} withoutAccents={withoutAccents} />
        </div>
        <div className={styles.meta}>
          <span>{charCount(item.charCount ?? 0)}</span>
          <span>{age(item.lastCopiedAt, now, formatLocale())}</span>
        </div>
      </>
    );
  }
  const tile = letterTile(item.domain);
  const heading = linkHeading(item);
  return (
    <>
      {item.linkImage && (
        <div className={styles.lkThumb}>
          <img src={mediaUrl('link-image', item.id, item.linkVersion)} alt="" draggable={false} loading="lazy" decoding="async" />
          {isVideoLink(item.url) && <span className={styles.play} aria-hidden="true" />}
        </div>
      )}
      <div className={styles.lkBody}>
        {item.linkIcon ? (
          <img className={styles.fav} src={mediaUrl('link-icon', item.id, item.linkVersion)} alt="" draggable={false} />
        ) : (
          <span className={styles.fav} style={{ background: tile.colour }} aria-hidden="true">
            {tile.letter}
          </span>
        )}
        <div className={styles.lkTitle} dir="auto">
          <Marked text={heading} terms={terms} withoutAccents={withoutAccents} />
        </div>
        <span className={styles.lkDetail} dir="auto">
          <Marked
            text={(heading === item.domain ? linkDetail(item.url) : item.domain) ?? ''}
            terms={terms}
            withoutAccents={withoutAccents}
          />
        </span>
      </div>
    </>
  );
}

function CardView(props: CardProps) {
  const { item, cursor, selected, badge, effect, order, now, terms, withoutAccents } = props;
  const { onClick, onContextMenu, onDoubleClick, onPointerDown } = props;
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
          onDoubleClick={() => onDoubleClick(item.id)}
          onPointerDown={(event) => onPointerDown(item.id, event)}
        >
          <span className={styles.tape} aria-hidden="true">
            <i />
          </span>
          <PinIcon className={styles.pin} />
          <Body item={item} now={now} terms={terms} withoutAccents={withoutAccents} />
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
