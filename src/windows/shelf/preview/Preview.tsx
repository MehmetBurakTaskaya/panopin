import { t } from '../../../i18n';
import type { ShelfItem } from '../../../ipc';
import { letterTile, linkDetail } from '../paper';
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
}

function LinkPreview({ item, onClose }: { item: ShelfItem; onClose: () => void }) {
  const tile = letterTile(item.domain);
  return (
    <>
      <div className={styles.bar}>
        <span className={styles.title}>{t('preview.link')}</span>
        <button type="button" className={styles.button} title={t('preview.close')} aria-label={t('preview.close')} onClick={onClose}>
          {icons.close}
        </button>
      </div>
      <div className={styles.body}>
        <div className={styles.link}>
          <div className={styles.linkCard}>
            <span className={styles.linkTile} style={{ background: tile.colour }} aria-hidden="true">
              {tile.letter}
            </span>
            <div>
              <h2>{item.title ?? item.domain ?? item.url}</h2>
              <p>{item.title ? item.url : linkDetail(item.url) || item.url}</p>
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
export function Preview({ item, mode, onMode, onClose, onUndo }: PreviewProps) {
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
        {item?.kind === 'link' && <LinkPreview item={item} onClose={onClose} />}
      </div>
    </div>
  );
}
