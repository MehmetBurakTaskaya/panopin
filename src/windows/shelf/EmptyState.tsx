import { PinIcon } from '../../components/PinIcon';
import { t } from '../../i18n';
import { detectPlatform, formatShortcut } from '../../shortcuts';
import styles from './Shelf.module.css';

/** The board with nothing on it yet: a pinned note with the first shortcuts to try. */
export function EmptyState({ shelfShortcut }: { shelfShortcut: string | null }) {
  const snip = detectPlatform() === 'mac' ? 'Super+Shift+4' : 'Super+Shift+S';
  return (
    <div className={styles.empty}>
      <span className={styles.strayTape} aria-hidden="true" />
      <div className={styles.note}>
        <PinIcon className={styles.notePin} />
        <h2>{t('shelf.empty.title')}</h2>
        <p>{t('shelf.empty.body')}</p>
        <ul>
          <li>
            {t('shelf.empty.copy')} <kbd>{formatShortcut('CommandOrControl+C')}</kbd>
          </li>
          <li>
            {t('shelf.empty.snip')} <kbd>{formatShortcut(snip)}</kbd>
          </li>
          {shelfShortcut && (
            <li>
              {t('shelf.empty.toggle')} <kbd>{formatShortcut(shelfShortcut)}</kbd>
            </li>
          )}
        </ul>
      </div>
    </div>
  );
}
