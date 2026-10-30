import { t } from '../../../i18n';
import { mediaUrl, type ShelfItem } from '../../../ipc';
import { formatShortcut } from '../../../shortcuts';
import { duration, isVideoLink, letterTile, linkHeading } from '../paper';
import { icons } from './icons';
import { ImagePreview } from './ImagePreview';
import styles from './Preview.module.css';
import { TextPreview } from './TextPreview';

/** view: looking; text: editing a text; markup and crop: editing an image. */
export type PreviewMode = 'view' | 'text' | 'markup' | 'crop';

export interface PreviewProps {
  /** The card under the shelf cursor; null when the last one was taken down. */
  item: ShelfItem | null;
  mode: PreviewMode;
  onMode: (mode: PreviewMode) => void;
  onClose: () => void;
  onUndo: () => void;
  /** Ctrl+O. */
  onOpenLink?: (id: number) => void;
  onRefreshLink?: (id: number) => void;
}

/** A screen recording: Windows plays it from Panopin's copy, a range at a time (D-058). */
function VideoPreview({ item, onClose }: { item: ShelfItem; onClose: () => void }) {
  return (
    <>
      <div className={styles.bar}>
        <span className={styles.title}>
          {t('preview.video')} · {duration(item.durationMs)}
        </span>
        <button type="button" className={styles.button} title={t('preview.close')} aria-label={t('preview.close')} onClick={onClose}>
          {icons.close}
        </button>
      </div>
      <div className={styles.body}>
        <video className={styles.video} src={mediaUrl('original', item.id)} controls preload="metadata" />
      </div>
    </>
  );
}

interface LinkPreviewProps {
  item: ShelfItem;
  onClose: () => void;
  onOpen?: (id: number) => void;
  onRefresh?: (id: number) => void;
}

/** A link as one large card: the page's picture, icon, title and description (D-057). */
function LinkPreview({ item, onClose, onOpen, onRefresh }: LinkPreviewProps) {
  const tile = letterTile(item.domain);
  return (
    <>
      <div className={styles.bar}>
        <span className={styles.title}>{t('preview.link')}</span>
        {onRefresh && (
          <button type="button" className={styles.button} onClick={() => onRefresh(item.id)}>
            {icons.revert}
            {t('preview.refreshLink')}
          </button>
        )}
        {onOpen && (
          <button type="button" className={`${styles.button} ${styles.primary}`} title={formatShortcut('CommandOrControl+O')} onClick={() => onOpen(item.id)}>
            {t('preview.openLink')}
          </button>
        )}
        <button type="button" className={styles.button} title={t('preview.close')} aria-label={t('preview.close')} onClick={onClose}>
          {icons.close}
        </button>
      </div>
      <div className={styles.body}>
        <div className={styles.link}>
          <div className={styles.linkCard}>
            {item.linkImage && (
              <div className={styles.linkPicture}>
                <img src={mediaUrl('link-image', item.id, item.linkVersion)} alt="" draggable={false} />
                {isVideoLink(item.url) && <span className={styles.linkPlay} aria-hidden="true" />}
              </div>
            )}
            <div className={styles.linkText}>
              {item.linkIcon ? (
                <img className={styles.linkTile} src={mediaUrl('link-icon', item.id, item.linkVersion)} alt="" draggable={false} />
              ) : (
                <span className={styles.linkTile} style={{ background: tile.colour }} aria-hidden="true">
                  {tile.letter}
                </span>
              )}
              <div>
                <h2 dir="auto">{linkHeading(item)}</h2>
                {item.linkSite && <p className={styles.linkSite}>{item.linkSite}</p>}
                {item.linkDescription && <p dir="auto">{item.linkDescription}</p>}
                <p className={styles.linkUrl}>{item.url}</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </>
  );
}

/**
 * Space opens and closes the preview; it shows the card under the shelf cursor, so the arrows
 * move through the cards and the panel stays open when the kind changes.
 */
export function Preview({ item, mode, onMode, onClose, onUndo, onOpenLink, onRefreshLink }: PreviewProps) {
  return (
    <div
      className={styles.layer}
      onPointerDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className={styles.panel} role="dialog" aria-modal="true" aria-label={t('preview.label')}>
        {!item && (
          <div className={styles.empty}>
            <span>{t('preview.empty')}</span>
            <button type="button" className={`${styles.button} ${styles.primary}`} onClick={onUndo}>
              {icons.undo}
              {t('preview.undo')}
            </button>
          </div>
        )}
        {item?.kind === 'image' && <ImagePreview key={item.id} item={item} mode={mode} onMode={onMode} onClose={onClose} />}
        {item?.kind === 'text' && <TextPreview key={item.id} item={item} editing={mode === 'text'} onMode={onMode} onClose={onClose} />}
        {item?.kind === 'video' && <VideoPreview key={item.id} item={item} onClose={onClose} />}
        {item?.kind === 'link' && <LinkPreview item={item} onClose={onClose} onOpen={onOpenLink} onRefresh={onRefreshLink} />}
      </div>
    </div>
  );
}
